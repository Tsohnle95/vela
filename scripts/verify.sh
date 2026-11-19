#!/usr/bin/env bash
# Canonical Vela verification gate. Keep CI and local work on this owner.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh
export PYTHONPYCACHEPREFIX="${PYTHONPYCACHEPREFIX:-$VELA_BUILD_JUNK/pycache}"

release=0
check_format=0
show_help=0
for argument in "$@"; do
  case "$argument" in
    --release)
      release=1
      ;;
    --check-format)
      check_format=1
      ;;
    -h|--help)
      show_help=1
      ;;
    *)
      echo "unknown argument: $argument" >&2
      exit 2
      ;;
  esac
done

if [[ "$show_help" -eq 1 ]]; then
  cat <<'EOF'
Usage: bash scripts/verify.sh [--release] [--check-format]

Runs documentation-link checks, Clippy, the workspace tests, and an app build.
--release selects the release build used by CI and packaging.
--check-format adds rustfmt verification to the full gate.

Formatting is opt-in because the imported tree has pre-existing rustfmt drift;
do not reformat unrelated files as part of a normal behavior change. For a
formatting-only check, run `cargo fmt --all --check` directly.
EOF
  exit 0
fi

if [[ -n "${VELA_CARGO:-}" ]]; then
  if [[ -f "$VELA_CARGO" && -x "$VELA_CARGO" ]]; then
    cargo_bin="$VELA_CARGO"
  elif command -v "$VELA_CARGO" >/dev/null 2>&1; then
    cargo_bin="$(command -v "$VELA_CARGO")"
  else
    echo "VELA_CARGO is not executable: $VELA_CARGO" >&2
    exit 127
  fi
elif command -v cargo >/dev/null 2>&1; then
  cargo_bin="$(command -v cargo)"
elif [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  cargo_bin="$HOME/.cargo/bin/cargo"
else
  echo "cargo not found; install the rust-toolchain.toml toolchain or set VELA_CARGO" >&2
  exit 127
fi

python_bin="${VELA_PYTHON:-python3}"
if ! command -v "$python_bin" >/dev/null 2>&1; then
  echo "python3 not found; set VELA_PYTHON" >&2
  exit 127
fi
if ! "$python_bin" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 7) else 1)' \
  >/dev/null 2>&1; then
  echo "VELA_PYTHON must be Python 3.7+: $python_bin" >&2
  exit 127
fi

echo "== documentation links =="
"$python_bin" scripts/check-docs.py

if [[ "$check_format" -eq 1 ]]; then
  echo "== formatting =="
  "$cargo_bin" fmt --all --check
fi

echo "== clippy =="
"$cargo_bin" clippy --locked --workspace --all-targets -- -D warnings

echo "== tests =="
"$cargo_bin" test --locked --workspace

if [[ "$release" -eq 1 ]]; then
  echo "== app build (release) =="
  "$cargo_bin" build --locked --release -p vela
else
  echo "== app build =="
  "$cargo_bin" build --locked -p vela
fi
