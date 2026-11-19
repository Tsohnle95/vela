#!/usr/bin/env bash
# Shared generated-output root. Keep build junk outside the repository.
#
# Source this from repository scripts, after changing into the repository root:
#   source scripts/build-env.sh
#
# Override VELA_BUILD_JUNK or CARGO_TARGET_DIR when a different location is
# needed. The directory is disposable; nothing in it is tracked or required.

vela_source="${BASH_SOURCE:-}"
if [[ -z "$vela_source" ]]; then
  # zsh sets $0 to the sourced file; bash uses BASH_SOURCE.
  vela_source="$0"
fi
vela_repo_root="$(cd -- "$(dirname -- "$vela_source")/.." && pwd -P)"
VELA_BUILD_JUNK="${VELA_BUILD_JUNK:-$(cd -- "$vela_repo_root/.." && pwd -P)/build-junk}"
VELA_TARGET_DIR="${CARGO_TARGET_DIR:-$VELA_BUILD_JUNK/vela-target}"
VELA_PREVIEW_ROOT="${VELA_PREVIEW_ROOT:-$VELA_BUILD_JUNK/vela-preview}"
VELA_MIGRATION_CACHE="${VELA_MIGRATION_CACHE:-$VELA_BUILD_JUNK/vela-migration-cache}"

export VELA_BUILD_JUNK VELA_TARGET_DIR VELA_PREVIEW_ROOT VELA_MIGRATION_CACHE
export CARGO_TARGET_DIR="$VELA_TARGET_DIR"
