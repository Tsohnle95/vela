#!/usr/bin/env python3
"""Check routed documentation links and canonical agent entry points.

This is a bounded surface-alignment check, not an architecture or behavior test.
It checks links/paths in the agent-facing documents plus the CI -> verify ->
docs-check command chain. It deliberately excludes historical parity evidence
and does not validate Markdown anchors, reference-style link usages, or nested
parentheses inside link destinations.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parent.parent

CANONICAL_DOCS = [
    Path("README.md"),
    Path("AGENTS.md"),
    Path("docs/agent-execution.md"),
    Path("docs/architecture.md"),
    Path("docs/architecture/core.md"),
    Path("docs/architecture/opencode.md"),
    Path("docs/architecture/engine.md"),
    Path("docs/architecture/ui.md"),
    Path("docs/architecture/editor.md"),
    Path("docs/development.md"),
    Path("docs/migration/START-HERE.md"),
    Path("docs/migration/PAUSE-HANDOFF.md"),
    Path("docs/migration/feature-parity-matrix.md"),
    Path("docs/migration/decisions.md"),
    Path(".agent/tasks/README.md"),
    Path("archive/AGENTS.md"),
]

REQUIRED_PATHS = [
    Path("README.md"),
    Path("AGENTS.md"),
    Path("Cargo.lock"),
    Path("rust-toolchain.toml"),
    Path(".cargo/config.toml"),
    Path("scripts/build-env.sh"),
    Path("docs/agent-execution.md"),
    Path("docs/architecture.md"),
    Path("docs/architecture/core.md"),
    Path("docs/architecture/opencode.md"),
    Path("docs/architecture/engine.md"),
    Path("docs/architecture/ui.md"),
    Path("docs/architecture/editor.md"),
    Path("docs/development.md"),
    Path("docs/migration/feature-parity-matrix.md"),
    Path("docs/migration/decisions.md"),
    Path(".agent/tasks/README.md"),
    Path(".agent/tasks/_TEMPLATE.md"),
    Path("archive/AGENTS.md"),
    Path("scripts/verify.sh"),
    Path("scripts/check-docs.py"),
]

# Stable anchors that agent routing depends on. Keep this list small; it is a
# routing contract, not an inventory of source files.
REQUIRED_SYMBOLS = [
    (Path("apps/vela/src/main.rs"), "fn main()"),
    (Path("crates/vela-core/src/lib.rs"), "pub mod transcript;"),
    (Path("crates/vela-opencode/src/lib.rs"), "pub use stream::spawn_event_stream;"),
    (Path("crates/vela-engine/src/lib.rs"), "pub use engine::"),
    (Path("crates/vela-terminal/src/pty.rs"), "pub fn spawn("),
    (Path("crates/vela-browser/src/lib.rs"), "pub struct BrowserHost"),
    (Path("crates/vela-ui/src/lib.rs"), "pub use app::run;"),
    (Path(".github/workflows/ci.yml"), "run: bash scripts/verify.sh --release"),
]

LINK_RE = re.compile(
    r"!?\[[^\]]*\]\(\s*(<[^>]+>|[^)\s]+)(?:\s+(?:\"[^\"]*\"|'[^']*'))?\s*\)"
)
REF_RE = re.compile(r"^\s*\[[^\]]+\]:\s*(<[^>]+>|\S+)")
FENCE_RE = re.compile(r"^\s*(```|~~~)")
KNOWN_URL_SCHEMES = {"http", "https", "mailto", "tel", "data", "file", "ftp"}


def strip_code(text: str) -> str:
    lines: list[str] = []
    in_fence = False
    for line in text.splitlines():
        if FENCE_RE.match(line):
            in_fence = not in_fence
            lines.append("")
            continue
        if in_fence or line.startswith("    ") or line.startswith("\t"):
            lines.append("")
            continue
        lines.append(line)
    joined = "\n".join(lines)
    joined = re.sub(r"<!--.*?-->", "", joined, flags=re.S)
    return re.sub(r"(`+)(.*?)\1", "", joined, flags=re.S)


def local_target(raw: str) -> str | None:
    target = raw.strip().strip("<>").strip()
    if not target or target.startswith("#") or target.startswith("//"):
        return None
    try:
        scheme = urlsplit(target).scheme.lower()
    except ValueError:
        # Malformed URL: report it as a missing local target rather than a crash.
        return target
    if scheme in KNOWN_URL_SCHEMES:
        return None
    target = target.split("#", 1)[0].split("?", 1)[0]
    return unquote(target) if target else None


def check_links(path: Path) -> list[str]:
    errors: list[str] = []
    text = strip_code(path.read_text(encoding="utf-8"))
    targets = [match.group(1) for match in LINK_RE.finditer(text)]
    for line in text.splitlines():
        match = REF_RE.match(line)
        if match:
            targets.append(match.group(1))

    for raw in targets:
        target = local_target(raw)
        if target is None:
            continue
        resolved = (path.parent / target).resolve()
        try:
            resolved.relative_to(ROOT)
        except ValueError:
            errors.append(f"{path}: link escapes repository: {raw}")
            continue
        if not resolved.exists():
            errors.append(f"{path}: missing link target: {raw}")
        elif resolved.is_dir() and not target.endswith("/"):
            errors.append(f"{path}: link target is a directory: {raw}")
    return errors


def active_lines(path: Path) -> list[str]:
    lines: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            lines.append(stripped)
    return lines


def check_verification_chain() -> list[str]:
    errors: list[str] = []
    ci = ROOT / ".github/workflows/ci.yml"
    verify = ROOT / "scripts/verify.sh"
    if ci.exists() and "run: bash scripts/verify.sh --release" not in active_lines(ci):
        errors.append("CI lacks an active step: run: bash scripts/verify.sh --release")
    if verify.exists():
        commands = active_lines(verify)
        for command in (
            '"$python_bin" scripts/check-docs.py',
            '"$cargo_bin" clippy --locked --workspace --all-targets -- -D warnings',
            '"$cargo_bin" test --locked --workspace',
            '"$cargo_bin" build --locked --release -p vela',
        ):
            if command not in commands:
                errors.append(f"scripts/verify.sh lacks active command: {command}")
    return errors


def main() -> int:
    errors: list[str] = []

    for path in REQUIRED_PATHS:
        if not (ROOT / path).exists():
            errors.append(f"missing required path: {path}")

    checked = 0
    for path in CANONICAL_DOCS:
        absolute = ROOT / path
        if not absolute.exists():
            errors.append(f"missing canonical document: {path}")
            continue
        checked += 1
        errors.extend(check_links(absolute))

    for path, symbol in REQUIRED_SYMBOLS:
        absolute = ROOT / path
        if not absolute.exists():
            errors.append(f"missing routed source file: {path}")
            continue
        if not any(symbol in line for line in active_lines(absolute)):
            errors.append(f"{path}: missing active routed symbol: {symbol}")

    errors.extend(check_verification_chain())

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(
        f"docs: {checked} canonical documents, "
        f"{len(REQUIRED_PATHS)} required paths, "
        f"{len(REQUIRED_SYMBOLS)} routed symbols checked"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
