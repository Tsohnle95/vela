#!/usr/bin/env bash
# Launch a separately profiled native build for the reference comparison.
# Build/package first: cargo build -p vela && bash scripts/package-macos.sh debug
set -euo pipefail
cd "$(dirname "$0")/.."
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
  reference_binary="../vela/dev/Vela.app/Contents/MacOS/Electron"
  if "$reference_comparison"; then
    reference_binary="target/parity/Vela Reference QA.app/Contents/MacOS/Vela Reference QA"
  fi
  exec "$reference_binary" ../vela \
    "--user-data-dir=${VELA_PARITY_PROFILE:-/private/tmp/vela-parity-original}"
fi
if [ -n "${1:-}" ]; then export VELA_UI_ACTION="$1"; else unset VELA_UI_ACTION; fi
native_bundle="Vela Native"
if [[ "${VELA_PARITY_COMPARISON:-}" == 1 ]]; then native_bundle="Vela Native QA"; fi
exec "target/debug/$native_bundle.app/Contents/MacOS/$native_bundle" --workspace "${VELA_PARITY_WORKSPACE:-/private/tmp/vela-test-workspace}"
