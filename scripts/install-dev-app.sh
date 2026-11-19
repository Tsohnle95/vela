#!/usr/bin/env bash
# Install a self-contained Vela Dev app in ~/Applications. All runtime state
# lives in one folder: ~/Library/Application Support/Vela Dev.
set -euo pipefail
cd "$(dirname "$0")/.."
repo_root="$(pwd -P)"
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh
target="$HOME/Applications/Vela Dev.app"
state="$HOME/Library/Application Support/Vela Dev"
if [[ -e "$target" && "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$target/Contents/Info.plist" 2>/dev/null || true)" != "dev.vela.native.dev" ]]; then
  echo "Refusing to replace an unrelated app at $target" >&2
  exit 2
fi
if pgrep -f "$target/Contents/MacOS/Vela Native" >/dev/null 2>&1; then
  echo "Quit Vela Dev before reinstalling it." >&2
  exit 1
fi

cargo_bin="${VELA_CARGO:-$(command -v cargo || true)}"
if [[ -z "$cargo_bin" && -x "$HOME/.cargo/bin/cargo" ]]; then
  cargo_bin="$HOME/.cargo/bin/cargo"
fi
if [[ -z "$cargo_bin" || ! -x "$cargo_bin" ]]; then
  echo "Cargo was not found. Set VELA_CARGO=/path/to/cargo." >&2
  exit 1
fi
"$cargo_bin" build -p vela
bash scripts/package-macos.sh debug

source_app="$VELA_TARGET_DIR/debug/Vela Native.app"
if [[ ! -x "$source_app/Contents/MacOS/Vela Native" ]]; then
  echo "Packaged app not found at $source_app" >&2
  exit 1
fi
rm -rf "$target"
mkdir -p "$(dirname "$target")"
ditto "$source_app" "$target"

/usr/libexec/PlistBuddy -c 'Set :CFBundleExecutable Vela Native' "$target/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleName Vela Dev' "$target/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier dev.vela.native.dev' "$target/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Delete :LSEnvironment' "$target/Contents/Info.plist" >/dev/null 2>&1 || true
/usr/libexec/PlistBuddy -c 'Add :LSEnvironment dict' "$target/Contents/Info.plist"
mkdir -p "$state/profile" "$state/opencode-config" "$state/opencode-data" "$state/opencode-state" "$state/logs"

opencode_bin="${VELA_OPENCODE_BINARY:-$(command -v opencode || true)}"
if [[ -z "$opencode_bin" && -x "$HOME/.opencode/bin/opencode" ]]; then
  opencode_bin="$HOME/.opencode/bin/opencode"
fi
if [[ -z "$opencode_bin" || ! -x "$opencode_bin" ]]; then
  echo "OpenCode was not found. Set VELA_OPENCODE_BINARY=/path/to/opencode." >&2
  exit 1
fi
port="$(python3 - <<'PY'
import socket
for candidate in range(49380, 49400):
    sock = socket.socket()
    try:
        sock.bind(("127.0.0.1", candidate))
    except OSError:
        continue
    finally:
        sock.close()
    print(candidate)
    break
else:
    raise SystemExit("no free loopback port in 49380-49399")
PY
)"
env "XDG_CONFIG_HOME=$state/opencode-config" "XDG_DATA_HOME=$state/opencode-data" "XDG_STATE_HOME=$state/opencode-state" "$opencode_bin" service stop >/dev/null 2>&1 || true
env "XDG_CONFIG_HOME=$state/opencode-config" "XDG_DATA_HOME=$state/opencode-data" "XDG_STATE_HOME=$state/opencode-state" "$opencode_bin" service set port "$port" >/dev/null

set_env() {
  /usr/libexec/PlistBuddy -c "Add :LSEnvironment:$1 string $2" "$target/Contents/Info.plist" >/dev/null 2>&1 || \
    /usr/libexec/PlistBuddy -c "Set :LSEnvironment:$1 $2" "$target/Contents/Info.plist" >/dev/null
}
set_env VELA_DATA_DIR "$state/profile"
set_env VELA_SKIP_LEGACY_IMPORT "1"
set_env VELA_OPENCODE_BINARY "$opencode_bin"
set_env XDG_CONFIG_HOME "$state/opencode-config"
set_env XDG_DATA_HOME "$state/opencode-data"
set_env XDG_STATE_HOME "$state/opencode-state"
set_env VELA_QUIT_ON_CLOSE "1"

icon_source="$repo_root/archive/vela-electron/resources/icon.png"
if [[ -f "$icon_source" ]]; then
  icon_tmp="$(mktemp -d)"
  iconset="$icon_tmp/VelaDev.iconset"
  mkdir -p "$iconset"
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$icon_source" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$icon_source" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$target/Contents/Resources/icon.icns"
  rm -rf "$icon_tmp"
  /usr/libexec/PlistBuddy -c 'Add :CFBundleIconFile string icon.icns' "$target/Contents/Info.plist" >/dev/null 2>&1 || \
    /usr/libexec/PlistBuddy -c 'Set :CFBundleIconFile icon.icns' "$target/Contents/Info.plist" >/dev/null
fi
codesign --force --deep --sign - "$target" >/dev/null
echo "Installed: $target"
echo "All app data: $state"
echo "Open: open \"$target\""
