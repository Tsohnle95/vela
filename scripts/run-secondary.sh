#!/usr/bin/env bash
# Launch the native app on the largest non-main display. Used for visual
# verification so test windows do not flash on the primary monitor.
set -euo pipefail
cd "$(dirname "$0")/.."
export VELA_WINDOW_SCREEN=non-main
exec ./target/release/vela "$@"
