"""Validate the delivered gallery, not just the art generator's inputs."""

from hashlib import sha256
from html.parser import HTMLParser
import json
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent
manifest = json.loads((ROOT / "concepts.json").read_text())
baseline = json.loads((ROOT / "expansion/original-art-baseline.json").read_text())
reservations = json.loads((ROOT / "expansion/registry.json").read_text())
assert len(manifest) == 154, "Deliver all 154 concepts"
assert [int(c["id"]) for c in manifest] == list(range(1, 155)), "IDs must be complete and ordered"
assert sum(c["batch"] == "new" for c in manifest) == 100
assert sum(c["batch"] == "compact" for c in manifest) == 12
assert sum(c["batch"] == "slick" for c in manifest) == 12
assert len({c["slug"] for c in manifest}) == 154


class Page(HTMLParser):
    def __init__(self, filename):
        super().__init__()
        self.filename = filename
        self.links = {}
        self.ids = set()
        self.cards = 0
        self.feed(filename.read_text())

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get("id"):
            assert attrs["id"] not in self.ids, f"Duplicate HTML id: {self.filename}/{attrs['id']}"
            self.ids.add(attrs["id"])
        if "card" in attrs.get("class", "").split():
            self.cards += 1
        if tag == "a" and attrs.get("id"):
            self.links[attrs["id"]] = attrs.get("href", "")
        for key in ("src", "href"):
            value = attrs.get(key)
            if not value or value.startswith(("#", "data:")):
                continue
            parsed = urlsplit(value)
            assert not parsed.scheme, f"External dependency in {self.filename}: {value}"
            target = (self.filename.parent / unquote(parsed.path)).resolve()
            assert target.is_relative_to(ROOT), f"Link escapes deliverable: {self.filename}/{value}"
            assert target.is_file(), f"Missing local link: {self.filename}/{value}"


hashes = set()
svg_ns = "{http://www.w3.org/2000/svg}"
for index, concept in enumerate(manifest):
    artwork = ROOT / "concepts" / f"{concept['slug']}.svg"
    content = artwork.read_bytes()
    digest = sha256(content).hexdigest()
    assert digest not in hashes, f"Identical artwork: {concept['id']}"
    hashes.add(digest)
    assert digest[:10] == concept["version"], f"Stale asset version: {concept['id']}"
    tree = ET.fromstring(content)
    assert tree.tag == svg_ns + "svg"
    assert tree.attrib["viewBox"] == f"0 0 {concept.get('width', 1440)} {concept.get('height', 900)}"
    assert not re.search(r"\borbit\b", " ".join(tree.itertext()), re.I), f"Old brand in artwork: {concept['id']}"
    ids = [node.attrib["id"] for node in tree.iter() if "id" in node.attrib]
    assert len(ids) == len(set(ids)), f"Duplicate SVG IDs: {concept['id']}"
    references = re.findall(r"url\(#([^\)]+)\)", content.decode())
    assert set(references).issubset(set(ids)), f"Missing SVG definition: {concept['id']}"
    for node in tree.iter():
        assert node.tag != svg_ns + "script", f"Artwork includes script: {concept['id']}"
        for key in ("href", "{http://www.w3.org/1999/xlink}href"):
            value = node.attrib.get(key, "")
            assert not value or value.startswith(("#", "data:")), f"Non-embedded SVG dependency: {concept['id']}"
    if concept["batch"] == "original":
        assert baseline[concept["id"]] == digest[:10], "Original artwork changed"
    elif concept["batch"] == "new":
        reserved = next(r for r in reservations if r["id"] == concept["id"])
        assert concept["name"] == reserved["name"]
        assert concept["tone"] in ("light", "dark")
        for key in ("composition", "silhouette", "typography", "material", "nearest"):
            assert len(concept[key]) > 15, f"Missing design reasoning: {concept['id']}/{key}"
        for source in concept["source"]:
            assert (ROOT.parent / source).is_file(), f"Missing source: {source}"
    else:
        assert concept["presentation"] == "compact"
        assert concept["splashWidth"] <= 420 and concept["splashHeight"] <= 360
        if concept["batch"] == "compact":
            assert concept["source"] == ["ChatGPT Image Oct 13, 2026, 03_23_53 PM.png"]
        else:
            assert concept["source"] == []
            assert "pixelated" not in content.decode()
            assert "prefers-reduced-motion" in content.decode()
    page = Page(ROOT / "concepts" / f"{concept['slug']}.html")
    previous = manifest[(index - 1) % len(manifest)]["slug"]
    following = manifest[(index + 1) % len(manifest)]["slug"]
    assert page.links["previous"] == f"./{previous}.html"
    assert page.links["next"] == f"./{following}.html"

gallery = Page(ROOT / "index.html")
assert gallery.cards == 154, "Gallery must expose all 154 designs"
for filename in ("process.html", "contact-sheet.html", "verify.html"):
    Page(ROOT / filename)
print("154/154 SVGs valid, distinct, self-contained, correctly linked, with no old Orbit wordmarks.")
print("12 slick and 12 pixel Mira studies; 130 earlier compositions; branding baseline matches.")
print("Browser decode, visual review and timing are separate checks: open verify.html.")
