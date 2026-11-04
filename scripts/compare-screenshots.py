#!/usr/bin/env python3
"""Compare real app captures without resizing, cropping or masking differences.

Requires Pillow. Example:
  python3 scripts/compare-screenshots.py docs/parity/2026-10-24
The report deliberately fails unmatched sizes and missing pages. A visual match
does not establish feature completeness; keep interaction results alongside it.
"""
import argparse
import hashlib
import html
import json
import math
from pathlib import Path

from PIL import Image, ImageChops, ImageStat


def compare(root, threshold):
    output = root / "comparison"
    output.mkdir(parents=True, exist_ok=True)
    names = sorted({p.name for app in ("original", "native") for p in (root / app).glob("*.png")})
    reviews_path = root / "capture-reviews.json"
    reviews = json.loads(reviews_path.read_text()) if reviews_path.exists() else {}
    if not isinstance(reviews, dict):
        raise ValueError("capture-reviews.json must map case names to exclusion reasons")
    results = []
    for name in names:
        result = {"page": name.removesuffix(".png")}
        images = []
        exclusions = reviews.get(result["page"], [])
        if not isinstance(exclusions, list) or any(not isinstance(reason, str) or not reason.strip() for reason in exclusions):
            raise ValueError(f"invalid capture review for {result['page']}")
        # Reviews may only disqualify comparisons. They never mask pixels,
        # relax metrics, alter captures, or grant acceptance.
        issues = [f"review: {reason}" for reason in exclusions]
        for app in ("original", "native"):
            path = root / app / name
            if not path.exists():
                result[app] = {"missing": True}
                continue
            source = Image.open(path)
            encoding = source.format
            image = source.convert("RGBA")
            images.append(image)
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            result[app] = {"size": image.size, "encoding": encoding, "lossless": encoding == "PNG", "sha256": digest,
                "icc_profile_sha256": hashlib.sha256(source.info.get("icc_profile", b"")).hexdigest()}
            try:
                manifest = json.loads(path.with_suffix(".json").read_text())
                points = manifest.get("point_size")
                aspect_valid = (isinstance(points, list) and len(points) == 2
                    and all(isinstance(value, (int, float)) and math.isfinite(value) and value > 0 for value in points)
                    and abs(image.width / points[0] - image.height / points[1]) <= 0.01)
                capture_surface = manifest.get("capture_surface", "window")
                stable = manifest.get("capture_stable") is True
                before_inventory = manifest.get("capture_window_inventory", [])
                after_inventory = manifest.get("capture_window_inventory_after", [])
                for captured in (manifest.get("window"), manifest.get("requested_window")):
                    if not isinstance(captured, dict) or captured not in before_inventory:
                        stable = False
                        continue
                    after = next((row for row in after_inventory if row.get("id") == captured.get("id")), None)
                    if not after or any(after.get(key) != captured.get(key)
                        for key in ("pid", "owner", "bounds", "layer", "active")):
                        stable = False
                group_valid = True
                if capture_surface == "window_group":
                    requested, context = manifest.get("requested_window", {}), manifest.get("window", {})
                    inventory = manifest.get("capture_window_inventory", [])
                    inner, outer = requested.get("bounds", {}), context.get("bounds", {})
                    try:
                        group_valid = (requested in inventory and context in inventory
                            and requested["id"] != context["id"]
                            and requested["pid"] == context["pid"] and requested["owner"] == context["owner"]
                            and outer["X"] <= inner["X"] and outer["Y"] <= inner["Y"]
                            and outer["X"] + outer["Width"] >= inner["X"] + inner["Width"]
                            and outer["Y"] + outer["Height"] >= inner["Y"] + inner["Height"])
                    except (KeyError, TypeError):
                        group_valid = False
                valid = (encoding == "PNG" and manifest.get("sha256") == digest and manifest.get("pixel_size") == list(image.size)
                    and manifest.get("surface") == app and manifest.get("encoding") == "PNG"
                    and manifest.get("lossless") is True
                    and aspect_valid and group_valid and stable and capture_surface in ("window", "window_group")
                    and all(manifest.get(key) is False for key in ("resized", "cropped", "masked")))
                result[app]["capture_manifest_valid"] = valid
                result[app]["capture_stable"] = stable
                result[app]["capture_surface"] = capture_surface
                result[app]["point_size"] = manifest.get("point_size")
                result[app]["window_active"] = manifest.get("window", {}).get("active")
                if not valid:
                    issues.append(f"{app}: invalid capture manifest")
            except (OSError, ValueError):
                result[app]["capture_manifest_valid"] = False
                issues.append(f"{app}: missing or unreadable capture manifest")
        if len(images) != 2:
            result["status"] = "missing capture"
        elif images[0].size != images[1].size:
            result["status"] = "viewport mismatch"
        else:
            if result["original"]["icc_profile_sha256"] != result["native"]["icc_profile_sha256"]:
                issues.append("color profile mismatch")
            if result["original"].get("point_size") != result["native"].get("point_size"):
                issues.append("point viewport mismatch")
            if result["original"].get("capture_surface") != result["native"].get("capture_surface"):
                issues.append("capture surface mismatch")
            if result["original"].get("window_active") != result["native"].get("window_active"):
                issues.append("window activation mismatch")
            if any(not isinstance(result[app].get("window_active"), bool) for app in ("original", "native")):
                issues.append("window activation not recorded")
            if any(not result[app].get("point_size") for app in ("original", "native")):
                issues.append("point viewport not recorded")
            original, native = images
            diff = ImageChops.difference(original, native)
            channels = diff.split()
            maximum = channels[0]
            for channel in channels[1:]:
                maximum = ImageChops.lighter(maximum, channel)
            histogram = maximum.histogram()
            count = original.width * original.height
            result.update({
                "status": "identical" if histogram[0] == count else "different",
                "exact_changed_pixels": count - histogram[0],
                "changed_pixels_above_threshold": sum(histogram[threshold + 1:]),
                "changed_fraction_above_threshold": sum(histogram[threshold + 1:]) / count,
                "mean_absolute_channel_error": sum(ImageStat.Stat(diff).mean) / 4,
                "lossless_pixel_match": histogram[0] == count and not issues and all(result[app]["lossless"] for app in ("original", "native")),
            })
            Image.blend(original, native, .5).save(output / (result["page"] + "-overlay.png"))
            # Amplification is for inspection only; metrics use the raw difference.
            # Keep the viewable difference opaque; its RGB includes alpha-only
            # discrepancies, while all metrics above use unmodified RGBA data.
            Image.merge("RGB", (ImageChops.lighter(channels[0], channels[3]), channels[1], channels[2])).point(lambda value: min(255, value * 4)).save(output / (result["page"] + "-diff.png"))
        result["comparison_issues"] = issues
        result["eligible_for_visual_review"] = len(images) == 2 and images[0].size == images[1].size and not issues
        results.append(result)
    report = {"threshold": threshold, "channels": "RGBA", "resized": False, "masked": False, "pages": results}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    rows = []
    for result in results:
        page = html.escape(result["page"])
        cells = "".join(f'<a href="../{app}/{page}.png"><img src="../{app}/{page}.png" alt="{app} {page}"></a>' for app in ("original", "native") if not result.get(app, {}).get("missing"))
        if result["status"] in ("identical", "different"):
            cells += f'<a href="{page}-overlay.png"><img src="{page}-overlay.png" alt="Overlay"></a><a href="{page}-diff.png"><img src="{page}-diff.png" alt="Difference amplified four times"></a>'
        rows.append(f'<section><h2>{page}: {html.escape(result["status"])}</h2><pre>{html.escape(json.dumps(result, indent=2))}</pre><div>{cells}</div></section>')
    (output / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Vela parity</title><style>body{font:14px system-ui;margin:24px;background:#eee;color:#111}section{border-top:1px solid #aaa;padding:20px 0}section>div{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}img{width:100%;display:block}pre{white-space:pre-wrap;font-size:11px}</style><h1>Vela / Vela Native</h1><p>Original, native, 50% overlay, amplified difference. Captures are never resized or masked for comparison. Click to inspect full pixels. Feature verification is separate.</p>' + "".join(rows) + '</html>')
    print(json.dumps({"pages": len(results), "lossless_pixel_matches": sum(p.get("lossless_pixel_match", False) for p in results), "report": str(output / "report.json")}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--threshold", type=int, default=8)
    args = parser.parse_args()
    if not 0 <= args.threshold <= 255:
        parser.error("threshold must be between 0 and 255")
    compare(args.root, args.threshold)
