#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test -p vela-core --release --test perf -- --ignored --nocapture
