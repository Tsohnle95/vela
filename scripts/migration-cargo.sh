#!/usr/bin/env bash
# Offline migration toolchain; survives loss of the old /private/tmp wrapper.
set -euo pipefail
vela_repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$vela_repo_root"
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh
vela_cargo_subcommand="${1:?Usage: bash scripts/migration-cargo.sh <check|test|clippy|build|fmt> [args]}"
shift
vela_cache_root="$VELA_MIGRATION_CACHE/rust"
if [[ -x "$vela_cache_root/cargo/bin/cargo" && -d "$vela_cache_root/rustup/toolchains/stable-aarch64-apple-darwin" ]]; then
  export CARGO_HOME="$vela_cache_root/cargo"
  export RUSTUP_HOME="$vela_cache_root/rustup"
  export RUSTUP_TOOLCHAIN="$vela_cache_root/rustup/toolchains/stable-aarch64-apple-darwin"
  vela_cargo_binary="$vela_cache_root/cargo/bin/cargo"
elif [[ -n "${VELA_CARGO:-}" && -x "$VELA_CARGO" ]]; then
  vela_cargo_binary="$VELA_CARGO"
elif command -v cargo >/dev/null 2>&1; then
  vela_cargo_binary="$(command -v cargo)"
elif [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  vela_cargo_binary="$HOME/.cargo/bin/cargo"
else
  echo "cargo not found; install the toolchain, set VELA_CARGO, or restore $VELA_MIGRATION_CACHE/rust" >&2
  exit 127
fi
if [[ "$vela_cargo_subcommand" == fmt ]]; then
  exec "$vela_cargo_binary" fmt "$@"
fi
exec "$vela_cargo_binary" "$vela_cargo_subcommand" --offline "$@"
