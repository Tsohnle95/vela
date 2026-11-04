#!/usr/bin/env bash
# Give the read-only reference its own automation identity. Reusing Vela's ID
# lets macOS resolve /Applications/Vela.app instead of the isolated dev process.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
import hashlib, plistlib, shutil, subprocess
from pathlib import Path

source = Path('../vela/dev/Vela.app').resolve()
bundle = Path('target/parity/Vela Reference QA.app')
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
