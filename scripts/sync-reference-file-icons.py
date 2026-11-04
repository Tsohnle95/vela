#!/usr/bin/env python3
"""Snapshot Vela's file glyphs and filename rules; no reference runtime dependency."""
import hashlib
import json
import re
from pathlib import Path
import argparse
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("reference", type=Path)
args = parser.parse_args()
base = args.reference / "src/renderer/src"
source = (base / "components/file-glyphs.ts").read_text()
foundation = (base / "styles/_foundation.scss").read_text()

def record(name, text=source):
    body = re.search(r"(?:export )?const " + name + r"[^=]*= \{(.*?)\n\};", text, re.S).group(1)
    return {quoted or plain: value for quoted, plain, value in
            re.findall(r'(?:"([^"]+)"|(\w+))\s*:\s*"([^"]+)"', body)}

glyphs = dict(re.findall(r'^  "([^"]+)": `([^`]+)`', source, re.M))
assert glyphs and "default" in glyphs
contains = re.search(r"const BY_NAME_CONTAINS[^=]*= \[(.*?)\n\];", source, re.S).group(1)
data = {
    "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
    "tab_source_sha256": hashlib.sha256((base / "components/FileIcons.tsx").read_bytes()).hexdigest(),
    "glyphs": {name: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">' + body + '</svg>'
               for name, body in glyphs.items()},
    "exact": record("BY_FILE_NAME"),
    "contains": re.findall(r'\["([^"]+)", "([^"]+)"\]', contains),
    "extensions": record("BY_FILE_EXTENSION"),
    "colors": record("TREE_ICON_COLOR"),
    "palette": dict(re.findall(r'--trees-icon-([\w]+):\s*(#[0-9a-fA-F]+);', foundation)),
    "tab_rules": record("FILE_GLYPHS", (base / "components/FileIcons.tsx").read_text()),
    "tab_glyphs": json.loads(subprocess.check_output([
        "node", str(Path(__file__).with_name("reference-tab-icons.cjs")), str(args.reference)
    ])),
}
assigned = set(data["exact"].values()) | set(data["extensions"].values()) | {v for _, v in data["contains"]}
assert assigned <= glyphs.keys(), assigned - glyphs.keys()
destination = Path(__file__).resolve().parent.parent / "crates/vela-ui/assets/tree-icons.json"
destination.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
print(f"Captured {len(glyphs)} original glyphs and all filename rules.")
