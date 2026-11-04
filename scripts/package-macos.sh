#!/usr/bin/env bash
# Create a distinct local app bundle so accessibility and screenshot tooling can
# address the native app without confusing it with the original Vela process.
set -euo pipefail
cd "$(dirname "$0")/.."
profile="${1:-release}"
case "$profile" in debug|release) ;; *) echo "usage: $0 [debug|release] [--comparison]" >&2; exit 2 ;; esac
name="Vela Native"
identifier="dev.vela.native"
if [[ "${2:-}" == --comparison ]]; then name="Vela Native QA"; identifier="dev.vela.native.parity"; fi
bundle="target/$profile/$name.app"
python3 - "$name" <<'PY'
import subprocess,sys
from pathlib import Path
name=sys.argv[1]
processes=subprocess.run(['ps','-axo','comm='],check=True,capture_output=True,text=True).stdout
if any(Path(line.strip()).name==name for line in processes.splitlines()):
 raise SystemExit(f'Close {name} before replacing its app bundle.')
PY
if [[ ! -x .tools/preview/node/bin/node || ! -f tools/preview/node_modules/vite/bin/vite.js ]]; then
  echo 'Preview tools are missing. Run bash scripts/setup-preview-toolchain.sh first.' >&2
  exit 1
fi
mkdir -p "$bundle/Contents/MacOS"
cp "target/$profile/vela" "$bundle/Contents/MacOS/.$name.$$"
mv "$bundle/Contents/MacOS/.$name.$$" "$bundle/Contents/MacOS/$name"
python3 - "$bundle" <<'PY'
import hashlib,os,shutil,sys,tempfile
from pathlib import Path
bundle=Path(sys.argv[1]);resources=bundle/'Contents/Resources';resources.mkdir(exist_ok=True)
destination=resources/'preview';node=Path('.tools/preview/node');packages=Path('tools/preview')
digest=hashlib.sha256()
inputs=[node/'bin/node',node/'LICENSE',packages/'package.json',packages/'package-lock.json']
helpers=['sass-plan.mjs','sass-compile.mjs']
inputs += [packages/name for name in helpers]
inputs+=sorted((packages/'node_modules').rglob('*'))
for path in inputs:
 if path.is_symlink():content=b'link:'+os.readlink(path).encode()
 elif path.is_file():content=path.read_bytes()
 else:continue
 for value in (str(path).encode(),content):digest.update(len(value).to_bytes(8,'big'));digest.update(value)
stamp=digest.hexdigest()
marker=destination/'.vela-preview-stamp'
if marker.exists() and marker.read_text().strip()==stamp:
 print('Preview resources are current.')
 raise SystemExit(0)
if destination.exists() and not marker.is_file():raise SystemExit('Refusing to replace unrecognized preview resources.')
stage=Path(tempfile.mkdtemp(prefix='.preview-stage-',dir=resources))
try:
 (stage/'node/bin').mkdir(parents=True)
 shutil.copy2(node/'bin/node',stage/'node/bin/node')
 shutil.copy2(node/'LICENSE',stage/'node/LICENSE')
 shutil.copytree(packages/'node_modules',stage/'node_modules',symlinks=True)
 for name in ('package.json','package-lock.json',*helpers):shutil.copy2(packages/name,stage/name)
 (stage/'.vela-preview-stamp').write_text(stamp+'\n')
 backup=resources/('.preview-previous-'+str(os.getpid()))
 if destination.exists():destination.rename(backup)
 stage.rename(destination)
 if backup.exists():shutil.rmtree(backup)
 print('Packaged pinned Node/Vite/Sass preview resources.')
finally:
 if stage.exists():shutil.rmtree(stage)
PY
mkdir -p "$bundle/Contents/Resources/third-party/chromium"
cp crates/vela-ui/assets/chromium/LICENSE "$bundle/Contents/Resources/third-party/chromium/LICENSE"
cp crates/vela-ui/assets/chromium/source.json "$bundle/Contents/Resources/third-party/chromium/source.json"
cat > "$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>$identifier</string>
  <key>CFBundleName</key><string>$name</string>
  <key>CFBundleDisplayName</key><string>$name</string>
  <key>CFBundleExecutable</key><string>$name</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --deep --sign - "$bundle"
printf '%s\n' "$PWD/$bundle"
