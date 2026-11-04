#!/usr/bin/env bash
# Warm-session navigation harness. Requires six imported sessions; pass their
# ids explicitly so the result is tied to a known fixture.
set -euo pipefail
: "${VELA_NAV_IDS:?export VELA_NAV_IDS=id1,id2,id3,id4,id5,id6}"
VELA_NAV_OUT="${VELA_NAV_OUT:-/tmp/vela-nav.json}"
VELA_NAV_WARM="${VELA_NAV_WARM:-3}"
cargo build --release -p vela
VELA_NAV_IDS="$VELA_NAV_IDS" \
VELA_NAV_OUT="$VELA_NAV_OUT" \
VELA_NAV_WARM="$VELA_NAV_WARM" \
  ./target/release/vela --workspace "${VELA_WORKSPACE:-$PWD}"
echo "wrote $VELA_NAV_OUT"
cat "$VELA_NAV_OUT"
