#!/usr/bin/env python3
"""Capture short-lived app feedback after an isolated fixture operation.

Only observes the named fixture's metadata. All app actions remain manual or
CUA-driven. Each capture uses the explicit window ID and unchanged lossless
capture helper; this does not synthesize feedback or alter either image.
"""
import argparse
import json
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("status", type=Path)
parser.add_argument("name")
parser.add_argument("--operation", default="model-rejected")
parser.add_argument("--native-window", required=True, type=int)
parser.add_argument("--original-window", required=True, type=int)
parser.add_argument("--evidence", default="docs/parity/2026-10-25")
parser.add_argument("--order", default="native,original")
args = parser.parse_args()
order = args.order.split(",")
if sorted(order) != ["native", "original"]:
    parser.error("order must name native and original exactly once")

def count():
    try:
        requests = json.loads(args.status.read_text())["requests"]
        return sum(request.get("operation") == args.operation for request in requests)
    except (OSError, ValueError, KeyError):
        return None  # Ignore a fixture's in-progress metadata write.

initial = count()
if initial is None:
    parser.error("fixture metadata must exist before capture starts")
deadline = time.monotonic() + 120
captured = 0
while captured < 2 and time.monotonic() < deadline:
    observed = count()
    if observed is not None and observed > initial + captured:
        if observed != initial + captured + 1:
            raise SystemExit("Unexpected fixture operation count; feedback capture aborted")
        app = order[captured]
        window = args.native_window if app == "native" else args.original_window
        # Permit the server response to reach the renderer, without stretching
        # or pausing either app's actual four-second feedback lifetime.
        time.sleep(0.25)
        subprocess.run(["bash", "scripts/capture-parity.sh", app, args.name,
                        args.evidence, str(window)], check=True)
        captured += 1
    time.sleep(0.025)
if captured != 2:
    raise SystemExit(f"Captured {captured} of 2 feedback states; no match is claimed")
