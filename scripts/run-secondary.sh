#!/usr/bin/env bash
# Launch the native app on the largest non-main display. Used for visual
# verification so test windows do not flash on the primary monitor.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh
export VELA_WINDOW_SCREEN=non-main
exec "$VELA_TARGET_DIR/release/vela" "$@"
