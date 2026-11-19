#!/usr/bin/env bash
# Give the read-only reference its own automation identity. Reusing Vela's ID
# lets macOS resolve /Applications/Vela.app instead of the isolated dev process.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=scripts/build-env.sh
source scripts/build-env.sh
python3 - <<'PY'
import hashlib, os, plistlib, shutil, subprocess
from pathlib import Path

reference_root = Path(os.environ.get('VELA_REFERENCE_ROOT', 'archive/vela-electron')).resolve()
source = reference_root / 'dev/Vela.app'
if not source.is_dir():
    raise SystemExit(f'Reference app not found: {source}. Restore it from a backup or use a separate writable pre-merge checkout via VELA_REFERENCE_ROOT; do not build inside archive/vela-electron.')
bundle = Path(os.environ['VELA_TARGET_DIR']) / 'parity/Vela Reference QA.app'
name = 'Vela Reference QA'
processes = subprocess.check_output(['ps', '-axo', 'comm='], text=True)
if any(Path(line.strip()).name == name for line in processes.splitlines()):
    raise SystemExit('Close Vela Reference QA before replacing its bundle.')
original = source / 'Contents/MacOS/Electron'
binary = bundle / f'Contents/MacOS/{name}'
if not bundle.exists():
    bundle.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(['cp', '-cR', str(source), str(bundle)], check=True)
    (bundle / 'Contents/MacOS/Electron').rename(binary)
elif not binary.is_file():
    raise SystemExit('Refusing to replace an unrecognized reference bundle.')
if hashlib.sha256(original.read_bytes()).digest() != hashlib.sha256(binary.read_bytes()).digest():
    shutil.copy2(original, binary)
info = bundle / 'Contents/Info.plist'
data = plistlib.loads((source / 'Contents/Info.plist').read_bytes())
data.update(CFBundleIdentifier='dev.vela.reference.parity', CFBundleName=name,
            CFBundleDisplayName=name, CFBundleExecutable=name)
info.write_bytes(plistlib.dumps(data))
subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(bundle)], check=True)
print(bundle.resolve())
PY
