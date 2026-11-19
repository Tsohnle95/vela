#!/usr/bin/env bash
# Launch a separately profiled native build for the reference comparison.
# Build/package first: cargo build -p vela && bash scripts/package-macos.sh debug
set -euo pipefail
cd "$(dirname "$0")/.."
repo_root="$(pwd -P)"
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh
reference=false
reference_comparison=false
while [[ "${1:-}" == --* ]]; do
  case "$1" in
    --reference) reference=true ;;
    --reference-comparison) reference=true; reference_comparison=true ;;
    --comparison) export VELA_PARITY_COMPARISON=1 ;;
    --log=*) export RUST_LOG="${1#--log=}" ;;
    --origin=*) export VELA_PARITY_ORIGIN="${1#--origin=}" ;;
    --size=*) export VELA_PARITY_SIZE="${1#--size=}" ;;
    --profile=*) export VELA_PARITY_PROFILE="${1#--profile=}" ;;
    --service-file=*) export VELA_OPENCODE_SERVICE_FILE="${1#--service-file=}" ;;
    --opencode=*) export VELA_OPENCODE_BINARY="${1#--opencode=}" ;;
    --workspace=*) export VELA_PARITY_WORKSPACE="${1#--workspace=}" ;;
    --state=*) export VELA_PARITY_STATE="${1#--state=}" ;;
    --config=*) export VELA_PARITY_CONFIG="${1#--config=}" ;;
    --data=*) export VELA_PARITY_DATA="${1#--data=}" ;;
    *) echo "Unknown parity option: $1" >&2; exit 2 ;;
  esac
  shift
done
export VELA_DATA_DIR="${VELA_PARITY_PROFILE:-/private/tmp/vela-parity-native}"
export VELA_SKIP_LEGACY_IMPORT=1
if [[ "${VELA_PARITY_SIZE:-}" == saved ]]; then
  unset VELA_WINDOW_SIZE
else
  export VELA_WINDOW_SIZE="${VELA_PARITY_SIZE:-1280x800}"
fi
export VELA_WINDOW_SCREEN=non-main
if [ -n "${VELA_PARITY_ORIGIN:-}" ]; then export VELA_WINDOW_ORIGIN="$VELA_PARITY_ORIGIN"; fi
export VELA_QUIT_ON_CLOSE=1
export XDG_STATE_HOME="${VELA_PARITY_STATE:-/private/tmp/vela-native-state}"
export XDG_CONFIG_HOME="${VELA_PARITY_CONFIG:-/private/tmp/vela-native-occonfig}"
export XDG_DATA_HOME="${VELA_PARITY_DATA:-/private/tmp/vela-native-ocdata}"
if "$reference"; then
  if [ -n "${VELA_OPENCODE_BINARY:-}" ]; then
    export PATH="$(dirname "$VELA_OPENCODE_BINARY"):$PATH"
  fi
  reference_root="${VELA_REFERENCE_ROOT:-$repo_root/archive/vela-electron}"
  reference_binary="$reference_root/dev/Vela.app/Contents/MacOS/Electron"
  if "$reference_comparison"; then
    reference_binary="$VELA_TARGET_DIR/parity/Vela Reference QA.app/Contents/MacOS/Vela Reference QA"
  fi
  if [ ! -x "$reference_binary" ]; then
    echo "Reference Electron binary not found: $reference_binary" >&2
    echo "Restore it from a backup or use a separate writable pre-merge checkout via VELA_REFERENCE_ROOT; do not build inside archive/vela-electron." >&2
    exit 1
  fi
  exec "$reference_binary" "$reference_root" \
    "--user-data-dir=${VELA_PARITY_PROFILE:-/private/tmp/vela-parity-original}"
fi
if [ -n "${1:-}" ]; then export VELA_UI_ACTION="$1"; else unset VELA_UI_ACTION; fi
native_bundle="Vela Native"
if [[ "${VELA_PARITY_COMPARISON:-}" == 1 ]]; then native_bundle="Vela Native QA"; fi
exec "$VELA_TARGET_DIR/debug/$native_bundle.app/Contents/MacOS/$native_bundle" --workspace "${VELA_PARITY_WORKSPACE:-/private/tmp/vela-test-workspace}"
