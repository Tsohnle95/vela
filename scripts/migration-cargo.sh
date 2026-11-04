#!/usr/bin/env bash
# Offline migration toolchain; survives loss of the old /private/tmp wrapper.
set -euo pipefail
vela_repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
vela_cargo_subcommand="${1:?Usage: bash scripts/migration-cargo.sh <check|test|clippy|build|fmt> [args]}"
shift
vela_cache_root="$vela_repo_root/target/migration-cache/rust"
if [[ -x "$vela_cache_root/cargo/bin/cargo" && -d "$vela_cache_root/rustup/toolchains/stable-aarch64-apple-darwin" ]]; then
  export CARGO_HOME="$vela_cache_root/cargo"
  export RUSTUP_HOME="$vela_cache_root/rustup"
  export RUSTUP_TOOLCHAIN="$vela_cache_root/rustup/toolchains/stable-aarch64-apple-darwin"
  vela_cargo_binary="$vela_cache_root/cargo/bin/cargo"
else
  vela_cargo_binary="$(command -v cargo)"
fi
if [[ "$vela_cargo_subcommand" == fmt ]]; then
  exec "$vela_cargo_binary" fmt "$@"
fi
exec "$vela_cargo_binary" "$vela_cargo_subcommand" --offline "$@"
