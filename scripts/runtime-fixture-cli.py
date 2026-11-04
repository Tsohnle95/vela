#!/usr/bin/env python3
"""Disposable OpenCode CLI double, copied into a runtime fixture's bin/ folder.

Only changes its own fixture files. Never invokes a real CLI, package manager,
signal, shell command or network request.
"""
import json
import os
from pathlib import Path
import sys
import time


def main():
    root = Path(__file__).resolve().parent.parent
    config = root / "runtime-fixture.json"
    if not config.is_file():
        raise SystemExit("The disposable runtime fixture marker is missing.")
    state = json.loads(config.read_text())
    args = sys.argv[1:]
    if args == ["--version"]:
        print(f"opencode v{state['cli_version']}")
        return
    if Path(os.environ.get("XDG_STATE_HOME", "")).resolve() != root:
        raise SystemExit("Refusing a command outside the disposable fixture state profile.")
    if args not in (["upgrade"], ["service", "restart"], ["service", "start"], ["serve", "--service"]):
        raise SystemExit("Unsupported disposable fixture command.")
    with (root / "runtime-commands.jsonl").open("a") as log:
        log.write(json.dumps({"command": args, "state_home": str(root), "time": time.time()}) + "\n")
    time.sleep(min(max(float(state.get("delay", 0)), 0), 10))
    if state.get("failure") == args[0]:
        print("Deliberate fixture runtime failure", file=sys.stderr)
        raise SystemExit(7)
    if args == ["upgrade"]:
        if state.get("package_manager_prompt"):
            print("OpenCode may be managed by a package manager. Install anyways?")
            return
        state["cli_version"] = state["next_version"]
    else:
        state["service_version"] = state["cli_version"]
        registration = json.dumps({"url": state["url"], "pid": state["pid"], "version": state["service_version"]}) + "\n"
        for file in (root / "service.json", root / "opencode/service.json"):
            temporary = file.with_suffix(".fixture-tmp")
            temporary.write_text(registration)
            temporary.replace(file)
    temporary = config.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, indent=2) + "\n")
    temporary.replace(config)


if __name__ == "__main__":
    main()
