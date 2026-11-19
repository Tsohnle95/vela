#!/usr/bin/env bash
# Launch the native app with a persistent, isolated development profile.
# Usage: scripts/dev-app.sh [workspace]
set -euo pipefail
cd "$(dirname "$0")/.."
repo_root="$(pwd -P)"
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh

workspace="${1:-}"
if [[ -z "$workspace" && "$PWD" != "/" ]]; then
  workspace="$PWD"
fi
if [[ -n "$workspace" ]]; then
  if [[ ! -d "$workspace" ]]; then
    echo "Workspace is not a directory: $workspace" >&2
    exit 2
  fi
  workspace="$(cd "$workspace" && pwd -P)"
fi

dev_root="${VELA_DEV_HOME:-$HOME/Library/Application Support/Vela Dev}"
config_root="$dev_root/opencode-config"
data_root="$dev_root/opencode-data"
state_root="$dev_root/opencode-state"
profile_root="$dev_root/profile"
mkdir -p "$config_root" "$data_root" "$state_root" "$profile_root"

app="$VELA_TARGET_DIR/debug/Vela Native.app"
binary="$app/Contents/MacOS/Vela Native"
if pgrep -f "$binary" >/dev/null 2>&1; then
  echo "Vela Dev is already running. Quit it first (Cmd+Q), then run this again." >&2
  exit 0
fi

cargo_bin="${VELA_CARGO:-$(command -v cargo || true)}"
if [[ -z "$cargo_bin" && -x "$HOME/.cargo/bin/cargo" ]]; then
  cargo_bin="$HOME/.cargo/bin/cargo"
fi
if [[ -z "$cargo_bin" || ! -x "$cargo_bin" ]]; then
  echo "Cargo was not found. Set VELA_CARGO=/path/to/cargo." >&2
  exit 1
fi
echo "Building Vela…"
"$cargo_bin" build -p vela
echo "Packaging Vela Dev…"
bash scripts/package-macos.sh debug

opencode_bin="${VELA_OPENCODE_BINARY:-$(command -v opencode || true)}"
if [[ -z "$opencode_bin" && -x "$HOME/.opencode/bin/opencode" ]]; then
  opencode_bin="$HOME/.opencode/bin/opencode"
fi
if [[ -z "$opencode_bin" || ! -x "$opencode_bin" ]]; then
  echo "OpenCode was not found. Set VELA_OPENCODE_BINARY=/path/to/opencode." >&2
  exit 1
fi

# The launcher uses its own OpenCode service so it cannot attach to the
# production service or reuse its credentials.
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
service_env=(
  env
  "XDG_CONFIG_HOME=$config_root"
  "XDG_DATA_HOME=$data_root"
  "XDG_STATE_HOME=$state_root"
  "$opencode_bin"
)
"${service_env[@]}" service stop >/dev/null 2>&1 || true
"${service_env[@]}" service set port "$port" >/dev/null

log="$dev_root/app.log"
: > "$log"
launch_env=(
  env
  "HOME=$HOME"
  "VELA_DATA_DIR=$profile_root"
  "VELA_SKIP_LEGACY_IMPORT=1"
  "VELA_OPENCODE_BINARY=$opencode_bin"
  "XDG_CONFIG_HOME=$config_root"
  "XDG_DATA_HOME=$data_root"
  "XDG_STATE_HOME=$state_root"
  "VELA_QUIT_ON_CLOSE=1"
)
args=()
if [[ -n "$workspace" ]]; then
  args+=(--workspace "$workspace")
fi
nohup "${launch_env[@]}" "$binary" "${args[@]}" >>"$log" 2>&1 &
pid=$!
sleep 2
if ! kill -0 "$pid" 2>/dev/null; then
  echo "Vela Dev exited during startup. Last log lines:" >&2
  tail -20 "$log" >&2
  exit 1
fi
echo "Vela Dev is running (pid $pid)."
echo "Profile: $dev_root"
echo "Log: $log"
