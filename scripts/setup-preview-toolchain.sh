#!/usr/bin/env bash
# Install optional Node/Vite/Sass tooling under build-junk, outside the source tree.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh
version=v22.23.2
case "$(uname -m)" in arm64) architecture=arm64 ;; x86_64) architecture=x64 ;; *) echo 'Unsupported preview toolchain architecture' >&2; exit 1 ;; esac
case "$(uname -s)" in Darwin) platform=darwin ;; Linux) platform=linux ;; *) echo 'Unsupported preview toolchain platform' >&2; exit 1 ;; esac
cache="$VELA_PREVIEW_ROOT/toolchain"
deps="$VELA_PREVIEW_ROOT/deps"
archive="node-$version-$platform-$architecture.tar.gz"
distribution="node-$version-$platform-$architecture"
mkdir -p "$cache/downloads" "$deps"
if [[ ! -x "$cache/$distribution/bin/node" ]]; then
  curl -fsSL "https://nodejs.org/dist/$version/SHASUMS256.txt" -o "$cache/downloads/SHASUMS256.txt"
  curl -fsSL "https://nodejs.org/dist/$version/$archive" -o "$cache/downloads/$archive"
  python3 - "$cache/downloads/$archive" "$cache/downloads/SHASUMS256.txt" "$archive" <<'PY'
import hashlib,sys
from pathlib import Path
archive,listing,name=sys.argv[1:]
expected=dict(line.split(maxsplit=1)[::-1] for line in Path(listing).read_text().splitlines()).get(name)
actual=hashlib.sha256(Path(archive).read_bytes()).hexdigest()
if not expected or actual!=expected:raise SystemExit('Official Node archive checksum mismatch; nothing was extracted.')
print('Verified official Node archive:',name)
PY
  tar -xzf "$cache/downloads/$archive" -C "$cache"
fi
if [[ -e "$cache/node" || -L "$cache/node" ]]; then
  if [[ "$(readlink "$cache/node" || true)" != "$distribution" ]]; then
    mv "$cache/node" "$cache/node.previous.$$"
    ln -s "$distribution" "$cache/node"
  fi
else
  ln -s "$distribution" "$cache/node"
fi
cp tools/preview/package.json tools/preview/package-lock.json "$deps/"
export npm_config_cache="$VELA_PREVIEW_ROOT/npm-cache"
"$cache/node/bin/node" "$cache/node/lib/node_modules/npm/bin/npm-cli.js" ci \
  --prefix "$deps" --ignore-scripts --no-audit --no-fund
if [[ -e tools/preview/node_modules && ! -L tools/preview/node_modules ]]; then
  rm -rf tools/preview/node_modules
fi
ln -sfn "$deps/node_modules" tools/preview/node_modules
"$cache/node/bin/node" "$deps/node_modules/vite/bin/vite.js" --version
