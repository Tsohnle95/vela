#!/usr/bin/env python3
"""Copy Blink's original fallback glyph from the installed reference runtime.

Data-pack format: Chromium tools/grit/grit/format/data_pack.py (version 5).
This copies asset bytes; it does not derive product graphics from screenshots.
"""
import hashlib
import html
import json
from pathlib import Path
import re
import struct

reference = Path(__file__).resolve().parents[2] / 'vela'
dist = reference / 'node_modules/electron/dist'
resources = dist / 'Electron.app/Contents/Frameworks/Electron Framework.framework/Versions/A/Resources'
output = Path(__file__).resolve().parents[1] / 'crates/vela-ui/assets/chromium'
output.mkdir(parents=True, exist_ok=True)
manifest = {'electron_version': json.loads((reference / 'node_modules/electron/package.json').read_text())['version'], 'resource_id': 51431, 'assets': []}
for scale in (1, 2):
    package = f'chrome_{scale * 100}_percent.pak'
    data = (resources / package).read_bytes()
    assert struct.unpack_from('<I', data)[0] == 5
    _, count, _ = struct.unpack_from('<BxxxHH', data, 4)
    entries = [struct.unpack_from('<HI', data, 12 + i * 6) for i in range(count + 1)]
    image = next(data[start:end] for (identifier, start), (_, end) in zip(entries, entries[1:]) if identifier == manifest['resource_id'])
    assert image.startswith(b'\x89PNG\r\n\x1a\n')
    dimensions = struct.unpack_from('>II', image, 16)
    assert dimensions == (14 * scale, 16 * scale)
    name = f'broken-image@{scale}x.png'
    (output / name).write_bytes(image)
    manifest['assets'].append({'file': name, 'source_package': package, 'dimensions': dimensions, 'sha256': hashlib.sha256(image).hexdigest()})
notices = (dist / 'LICENSES.chromium.html').read_text()
section = next(section for section in notices.split('<div class="product">') if '<span class="title">The Chromium Project</span>' in section)
license_text = html.unescape(re.search(r'<pre>(.*?)</pre>', section, re.S).group(1))
(output / 'LICENSE').write_text(license_text + '\n')
(output / 'source.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(manifest))
