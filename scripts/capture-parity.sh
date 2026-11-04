#!/usr/bin/env bash
# Lossless window-only captures. Does not activate, move, resize or mask a window.
set -euo pipefail
cd "$(dirname "$0")/.."
surface="${1:?Usage: capture-parity.sh original|native capture-name [evidence-directory] [window-id] [context-window-id] [--single-window]}"
name="${2:?A capture name is required}"
evidence="${3:-docs/parity/2026-10-25}"
case "$surface" in original|native) ;; *) echo 'surface must be original or native' >&2; exit 2;; esac
if [[ ! "$name" =~ ^[a-z0-9][a-z0-9-]*$ ]]; then echo 'Use a lowercase hyphenated capture name' >&2; exit 2; fi
mkdir -p "$evidence/$surface"
output="$evidence/$surface/$name.png"
if [[ -e "$output" || -e "$evidence/$surface/$name.json" ]]; then
  echo 'This capture already exists. Use a new name so earlier evidence is preserved.' >&2
  exit 2
fi
if [[ -n "${4:-}" ]]; then
  windows="$(env CLANG_MODULE_CACHE_PATH=/private/tmp/vela-capture-swift swift scripts/parity-window-info.swift --all-layers)"
else
  windows="$(env CLANG_MODULE_CACHE_PATH=/private/tmp/vela-capture-swift swift scripts/parity-window-info.swift)"
fi
selected="$(python3 -c 'import json,sys
rows=[json.loads(line) for line in sys.argv[2].splitlines() if line]
rows=[row for row in rows if (row["owner"] in ("Vela Native", "Vela Native QA")) == (sys.argv[1]=="native")]
if sys.argv[3]: rows=[row for row in rows if str(row["id"])==sys.argv[3]]
else:
 preferred="Vela Native QA" if sys.argv[1]=="native" else "Vela Reference QA"
 qa=[row for row in rows if row["owner"]==preferred]
 if qa: rows=qa
if len(rows)!=1:
 print(json.dumps(rows),file=sys.stderr)
 raise SystemExit(f"Expected exactly one {sys.argv[1]} Vela window; found {len(rows)}")
print(json.dumps(rows[0]))' "$surface" "$windows" "${4:-}")"
context="$selected"
if [[ -n "${5:-}" ]]; then
  context="$(python3 -c 'import json,sys
requested=json.loads(sys.argv[1]);rows=[json.loads(line) for line in sys.argv[2].splitlines() if line]
rows=[row for row in rows if str(row["id"])==sys.argv[3] and row["pid"]==requested["pid"] and row["owner"]==requested["owner"]]
if len(rows)!=1:raise SystemExit("Expected one context window from the same app process")
outer=rows[0]["bounds"];inner=requested["bounds"]
if not (outer["X"]<=inner["X"] and outer["Y"]<=inner["Y"] and outer["X"]+outer["Width"]>=inner["X"]+inner["Width"] and outer["Y"]+outer["Height"]>=inner["Y"]+inner["Height"]):raise SystemExit("Requested sheet must be inside its context window")
print(json.dumps(rows[0]))' "$selected" "$windows" "$5")"
fi
window_id="$(python3 -c 'import json,sys;print(int(json.loads(sys.argv[1])["id"]))' "$selected")"
capture_flags=(-x -o)
if [[ "${6:-}" == --single-window ]]; then
  if [[ -n "${5:-}" ]]; then echo 'A single-window capture cannot use a parent context.' >&2; exit 2; fi
  capture_flags+=(-a)
elif [[ -n "${6:-}" ]]; then
  echo 'Unknown capture mode; use --single-window or omit it.' >&2; exit 2
fi
/usr/sbin/screencapture "${capture_flags[@]}" -l "$window_id" "$output"
after="$(env CLANG_MODULE_CACHE_PATH=/private/tmp/vela-capture-swift swift scripts/parity-window-info.swift --all-layers)"
python3 -c 'import datetime,hashlib,json,pathlib,struct,subprocess,sys
path=pathlib.Path(sys.argv[1]);data=path.read_bytes()
if data[:8]!=b"\x89PNG\r\n\x1a\n":raise SystemExit("Capture must be a lossless PNG")
width,height=struct.unpack(">II",data[16:24]);requested=json.loads(sys.argv[2]);surface=sys.argv[3];window=json.loads(sys.argv[4])
after={row["id"]:row for row in (json.loads(line) for line in sys.argv[6].splitlines() if line)}
for before in (requested,window):
 current=after.get(before["id"])
 if not current or any(current.get(key)!=before.get(key) for key in ("pid","owner","bounds","layer","active")):
  raise SystemExit("Window identity, bounds or activation changed during capture. Recapture with a new name.")
point_width,point_height=window["bounds"]["Width"],window["bounds"]["Height"]
if point_width<=0 or point_height<=0 or abs(width/point_width-height/point_height)>0.01:
 raise SystemExit("Captured pixels do not match the window aspect ratio. macOS can include a sheets parent; specify its context-window-id explicitly.")
repo=pathlib.Path.cwd() if surface=="native" else pathlib.Path.cwd().parent/"vela"
def git(*args):return subprocess.check_output(["git","-C",str(repo),*args])
record={"captured_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),"surface":surface,"window":window,"requested_window":requested,"capture_surface":"window_group" if requested["id"]!=window["id"] else "window","capture_window_inventory":[json.loads(line) for line in sys.argv[5].splitlines() if line],"encoding":"PNG","lossless":True,"pixel_size":[width,height],"point_size":[point_width,point_height],"sha256":hashlib.sha256(data).hexdigest(),"head":git("rev-parse","HEAD").decode().strip(),"tracked_diff_sha256":hashlib.sha256(git("diff","--binary","HEAD")).hexdigest(),"capture_command":["screencapture","-x","-o","-l",str(requested["id"])],"resized":False,"cropped":False,"masked":False}
record["capture_stable"]=True;record["capture_window_inventory_after"]=list(after.values())
record["attached_windows_included"]=sys.argv[7]!="--single-window"
if not record["attached_windows_included"]:record["capture_command"].insert(3,"-a")
roots=["crates","apps","Cargo.toml","Cargo.lock"] if surface=="native" else ["src","resources","package.json","package-lock.json","electron-vite.config.ts"]
paths=sorted(set(git("ls-files","-z","--cached","--others","--exclude-standard","--",*roots).split(b"\0"))-{b""})
def tree_hash(items):
 digest=hashlib.sha256();count=0
 for relative in items:
  path=repo/relative
  if not path.is_file():continue
  name=relative.encode();content=path.read_bytes()
  for value in [name,content]:digest.update(len(value).to_bytes(8,"big"));digest.update(value)
  count+=1
 return digest.hexdigest(),count
record["source_tree_sha256"],record["source_tree_files"]=tree_hash([p.decode() for p in paths])
record["source_tree_roots"]=roots;record["source_tree_includes_untracked"]=True
if surface=="original" and (repo/"out").is_dir():
 record["reference_bundle_sha256"],record["reference_bundle_files"]=tree_hash(sorted(str(p.relative_to(repo)) for p in (repo/"out").rglob("*") if p.is_file()))
if surface=="native":
 name=window["owner"];binary=pathlib.Path(f"target/debug/{name}.app/Contents/MacOS/{name}")
 record["binary_path"]=str(binary)
 record["binary_sha256"]=hashlib.sha256(binary.read_bytes()).hexdigest()
path.with_suffix(".json").write_text(json.dumps(record,indent=2)+"\n")
print(json.dumps({"capture":str(path),"pixel_size":[width,height],"window_id":window["id"],"capture_surface":record["capture_surface"]}))' "$output" "$selected" "$surface" "$context" "$windows" "$after" "${6:-}"
