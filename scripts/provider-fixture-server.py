#!/usr/bin/env python3
"""Loopback-only provider fixture for real native GUI acceptance checks.

Never forwards requests or writes credentials. Keys beginning reject- return an
error that deliberately echoes the disposable key; delay- postpones success so
cancel/reopen ownership can be exercised. The status file records hashes only.
This is functional evidence, not a substitute for paired original screenshots.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import re
import shutil
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=49557)
    parser.add_argument("--directory", default="/private/tmp/vela-test-workspace")
    parser.add_argument("--state", type=Path, default=Path("/private/tmp/vela-provider-fixture"))
    parser.add_argument("--resume", action="store_true", help="Retain only previously recorded fixture metadata")
    parser.add_argument("--oauth", action="store_true", help="Expose synthetic OAuth methods on OpenCode Go")
    parser.add_argument("--forms", action="store_true", help="Expose typed fixture fields on CommandCode")
    parser.add_argument("--models", action="store_true", help="Expose selectable and deliberately rejected fixture models")
    parser.add_argument("--variants", action="store_true", help="Expose ordered response-strength choices on the fixture model")
    parser.add_argument("--model-count", type=int, default=0, help="Expand the disposable model catalog to this count")
    parser.add_argument("--agent-count", type=int, default=0, help="Expand the disposable agent catalog to this count")
    parser.add_argument("--permissions", action="store_true", help="Allow injecting harmless fixture permission events; never executes a command")
    parser.add_argument("--prompts", action="store_true", help="Record disposable prompt metadata without reading files or running a model")
    parser.add_argument("--history", type=Path, help="Serve an explicit disposable canonical message history for restart comparisons")
    parser.add_argument("--inbox-count", type=int, default=0, help="Seed disposable runtime inbox entries for queue controls and scrolling")
    parser.add_argument("--runtime", action="store_true", help="Create a disposable CLI double for runtime Sync/Update checks")
    parser.add_argument("--session-actions", action="store_true", help="Allow deletion of this synthetic session only")
    parser.add_argument("--session-count", type=int, default=1, help="Expose up to 64 synthetic conversations in the same directory for ownership checks")
    parser.add_argument("--delete-failures", type=int, default=0, help="Reject this many synthetic session deletion attempts")
    args = parser.parse_args()
    history = json.loads(args.history.read_text()) if args.history else []
    if not isinstance(history, list):
        parser.error("--history must contain a JSON array of disposable messages")
    args.state.mkdir(parents=True, exist_ok=True)
    lock = threading.Lock()
    credentials = {}
    requests = []
    attempts = {}
    counter = 0
    prompt_failures = 0
    lifecycle_assistant = "fixture-lifecycle-assistant"
    selection = {}
    permissions = {}
    inbox = [{"id": f"fixture-inbox-{index:03d}", "sessionID": "ses_provider_fixture", "type": "user", "delivery": "queue",
              "payload": {"text": f"Queued fixture {index:03d}\nSecond line 🌍", "files": [{"name": "image 🌍.png"}, {"name": "notes.txt"}] if index % 3 == 0 else []},
              "timeCreated": index + 1} for index in range(max(0, args.inbox_count))]
    subscribers = set()
    session_lifecycle = {"deleted": False, "failures": max(0, args.delete_failures)}
    if args.resume and (args.state / "status.json").exists():
        previous = json.loads((args.state / "status.json").read_text())
        credentials = previous["credentials"]
        requests = previous["requests"]
        attempts = previous.get("attempts", {})
        counter = len(requests)
        selection = previous.get("selection", {})
        permissions = previous.get("permissions", {})
        inbox = previous.get("inbox", inbox)
        session_lifecycle = previous.get("session_lifecycle", session_lifecycle)

    def status():
        (args.state / "status.json").write_text(json.dumps({
            "requests": requests,
            "credentials": credentials,
            "attempts": attempts,
            "selection": selection,
            "permissions": permissions,
            "inbox": inbox,
            "session_lifecycle": session_lifecycle,
        }, indent=2) + "\n")

    session = {
        "id": "ses_provider_fixture", "title": "Provider fixture", "projectID": "fixture",
        "cost": 0, "tokens": {"input": 0, "output": 0, "reasoning": 0,
                                "cache": {"read": 0, "write": 0}},
        "time": {"created": 1, "updated": 2}, "location": {"directory": args.directory},
    }
    sessions = [session] + [dict(session, id=f"ses_provider_fixture_{index}", title=f"Provider fixture {index}") for index in range(2, min(64, max(1, args.session_count)) + 1)]
    model_catalog = [
        {"id": "fixture", "providerID": "mock", "name": "Fixture", "limit": {"context": 32000}},
        {"id": "wide", "providerID": "mock", "name": "Long Unicode fixture model 🌍 for selection width", "limit": {"context": 128000}},
        {"id": "reject-selection", "providerID": "mock", "name": "Rejected fixture choice"},
    ]
    if args.variants:
        model_catalog[0]["variants"] = [{"id": variant} for variant in ("low", "high", "extra_high", "reject-variant")]
    model_catalog.extend({"id": f"catalog-{index:03d}", "providerID": "mock", "name": f"Catalog model {index:03d}"}
                         for index in range(len(model_catalog), args.model_count))
    agent_catalog = [{"id": "build", "name": "Build"}]
    agent_catalog.extend({"id": f"catalog-agent-{index:03d}", "name": f"Catalog agent {index:03d}"}
                         for index in range(len(agent_catalog), args.agent_count))

    def publish(event):
        # Caller holds lock. This models external runtime events; it never
        # changes app UI state, selection, approval preferences or screenshots.
        for subscriber in subscribers:
            subscriber.put(event)

    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def handle(self):
            try:
                super().handle()
            except (BrokenPipeError, ConnectionResetError):
                pass  # Expected when a client cancels or replaces a stream.

        def log_message(self, *_):
            pass  # Never log request bodies, query values or credentials.

        def respond(self, payload=None, code=200):
            data = b"" if payload is None else json.dumps(payload).encode()
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            try:
                self.wfile.write(data)
            except (BrokenPipeError, ConnectionResetError):
                pass

        def do_GET(self):
            path = urlsplit(self.path).path
            if path == "/api/info":
                version = json.loads((args.state / "runtime-fixture.json").read_text())["service_version"] if args.runtime else "2.1.4"
                return self.respond({"pid": os.getpid(), "version": version})
            if path == "/api/event":
                subscriber = queue.Queue()
                with lock:
                    subscribers.add(subscriber)
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Cache-Control", "no-cache")
                self.end_headers()
                try:
                    while True:
                        try:
                            event = subscriber.get(timeout=1)
                            self.wfile.write(b"data: " + json.dumps(event).encode() + b"\n\n")
                        except queue.Empty:
                            self.wfile.write(b": provider fixture heartbeat\n\n")
                        self.wfile.flush()
                except (BrokenPipeError, ConnectionResetError):
                    return
                finally:
                    with lock:
                        subscribers.discard(subscriber)
            if path == "/api/session":
                listed = sessions[1:] if args.session_actions and session_lifecycle["deleted"] else sessions
                return self.respond({"data": listed, "cursor": {}})
            if re.fullmatch(r"/api/session/[^/]+", path):
                with lock:
                    if path == "/api/session/ses_provider_fixture" and args.session_actions and session_lifecycle["deleted"]:
                        return self.respond({"message": "Synthetic session was deleted"}, 404)
                    requested = path.rsplit("/", 1)[1]
                    selected = next((item for item in sessions if item["id"] == requested), dict(session, id=requested))
                    return self.respond({"data": dict(selected, **selection)})
            authorization = re.fullmatch(r"/authorize/(oauth-[0-9]+)", path)
            if authorization:
                return self.respond({"fixture": "Local authorization page", "attempt": authorization.group(1)})
            oauth = re.fullmatch(r"/api/integration/[^/]+/connect/oauth/(oauth-[0-9]+)", path)
            if oauth:
                with lock:
                    attempt = attempts.get(oauth.group(1))
                    if not attempt:
                        return self.respond({"message": "Unknown fixture attempt"}, 404)
                    attempt["polls"] += 1
                    if attempt["status"] == "pending":
                        if attempt["method"] == "expired":
                            attempt["status"] = "expired"
                        elif attempt["method"] == "auto" and attempt["polls"] >= 2:
                            attempt["status"] = "complete"
                            credentials.setdefault(attempt["provider"], []).append({"type": "credential", "id": oauth.group(1), "label": "OAuth fixture"})
                    status()
                    return self.respond({"data": {"status": attempt["status"]}})
            if path == "/api/integration":
                with lock:
                    providers = []
                    for provider, name, env in [
                        ("opencode-go", "OpenCode Go", "OPENCODE_API_KEY"),
                        ("commandcode", "CommandCode", "CMD_API_KEY"),
                        ("openai", "OpenAI", "OPENAI_API_KEY"),
                    ]:
                        methods = [{"type": "key", "form": []}, {"type": "env", "names": [env]}]
                        if args.forms and provider == "commandcode":
                            methods[0]["form"] = [
                                {"key": "profile", "type": "string", "title": "Profile", "required": True,
                                 "minLength": 3, "maxLength": 6, "pattern": "[[a-z]&&[^aeiou]]+", "placeholder": "bcdf"},
                                {"key": "region", "type": "string", "title": "Region", "required": True,
                                 "options": [{"value": "us", "label": "United States"}, {"value": "eu", "label": "Europe"},
                                             {"value": "ap", "label": "Asia Pacific"}]},
                                {"key": "capacity", "type": "integer", "title": "Capacity", "minimum": 1, "maximum": 4, "default": 2},
                                {"key": "threshold", "type": "number", "title": "Threshold", "minimum": 0.5, "maximum": 4.5, "default": 1.5},
                                {"key": "advanced", "type": "boolean", "title": "Advanced", "default": False},
                                {"key": "team", "type": "string", "title": "Team", "required": True,
                                 "when": [{"key": "advanced", "op": "eq", "value": True}]},
                                {"key": "scopes", "type": "multiselect", "title": "Scopes", "default": ["read"], "minItems": 2,
                                 "options": [{"value": "read", "label": "Read"}, {"value": "write", "label": "Write"},
                                             {"value": "admin", "label": "Admin"}]},
                                {"key": "setup", "type": "external", "title": "Setup", "url": f"http://127.0.0.1:{args.port}/authorize/setup"},
                            ]
                        if args.oauth and provider == "opencode-go":
                            methods = [{"type": "oauth", "id": method, "label": label} for method, label in [
                                ("auto", "Automatic fixture"), ("code", "Code fixture"),
                                ("expired", "Expired fixture"), ("slow", "Delayed fixture")]]
                        providers.append({"id": provider, "name": name, "methods": methods,
                            "connections": list(credentials.get(provider, []))})
                return self.respond({"data": providers})
            if path == "/api/model/default":
                return self.respond({"data": {"id": "fixture", "providerID": "mock", "name": "Fixture"}})
            if path == "/api/model" and args.models:
                return self.respond({"data": model_catalog})
            if path == "/api/permission/request" and args.permissions:
                with lock:
                    return self.respond({"data": list(permissions.values())})
            if path == "/api/agent":
                return self.respond({"data": agent_catalog})
            if path == "/api/session/ses_provider_fixture/inbox":
                with lock:
                    return self.respond({"data": list(inbox)})
            if path == "/api/session/ses_provider_fixture/message":
                if args.session_actions and session_lifecycle["deleted"]:
                    return self.respond({"message": "Synthetic session was deleted"}, 404)
                return self.respond({"data": history, "cursor": {}})
            if path in ("/api/model", "/api/permission/request", "/api/form") or re.fullmatch(
                    r"/api/session/[^/]+/(message|inbox|form)", path):
                return self.respond({"data": [], "cursor": {}})
            return self.respond({"message": "Unknown fixture endpoint"}, 404)

        def do_POST(self):
            nonlocal counter, prompt_failures, lifecycle_assistant
            url = urlsplit(self.path)
            match = re.fullmatch(r"/api/integration/(opencode-go|commandcode|openai)/connect/key", url.path)
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length <= 65536:
                return self.respond({"message": "Invalid fixture body"}, 400)
            try:
                body = json.loads(self.rfile.read(length))
            except (ValueError, UnicodeDecodeError):
                return self.respond({"message": "Invalid fixture body"}, 400)
            if url.path == "/api/session":
                return self.respond({"data": session})
            if url.path == "/api/session/ses_provider_fixture/prompt" and args.prompts:
                text, files = body.get("text"), body.get("files", [])
                if not isinstance(text, str) or not isinstance(files, list) or any(
                        not isinstance(file, dict) or not isinstance(file.get("uri"), str)
                        or not file["uri"].startswith("file://") or not isinstance(file.get("name"), str)
                        or "path" in file for file in files):
                    return self.respond({"message": "Invalid fixture prompt"}, 400)
                with lock:
                    requests.append({"operation": "prompt", "text": text, "files": files,
                                     "delivery": body.get("delivery"), "session": "ses_provider_fixture"})
                    if prompt_failures > 0:
                        prompt_failures -= 1
                        requests[-1]["rejected"] = True
                        status()
                        return self.respond({"message": "Disposable prompt rejection"}, 503)
                    if body.get("delivery") == "queue":
                        counter += 1
                        identifier = f"fixture-inbox-submitted-{counter}"
                        item = {"id": identifier, "sessionID": "ses_provider_fixture", "type": "user", "delivery": "queue",
                                "payload": {"text": text, "files": files}, "timeCreated": counter}
                        inbox.append(item)
                        publish({"type": "session.inbox.enqueued", "data": {"sessionID": "ses_provider_fixture", "inboxID": identifier, "item": item}})
                    status()
                return self.respond({"data": {"id": "fixture-prompt-ack"}})
            if url.path == "/fixture/prompt-failures" and args.prompts:
                count = body.get("count")
                if not isinstance(count, int) or not 0 <= count <= 20:
                    return self.respond({"message": "Invalid fixture failure count"}, 400)
                with lock:
                    prompt_failures = count
                return self.respond(None, 204)
            if url.path == "/fixture/status" and args.prompts:
                phase = body.get("phase")
                if phase not in ("busy", "idle"):
                    return self.respond({"message": "Invalid fixture phase"}, 400)
                with lock:
                    requests.append({"operation": "fixture-phase", "phase": phase})
                    publish({"type": "session.execution.started" if phase == "busy" else "session.execution.succeeded",
                             "data": {"sessionID": "ses_provider_fixture"}})
                    if phase == "busy" and isinstance(body.get("assistant"), str):
                        publish({"type": "session.step.started", "data": {
                            "sessionID": "ses_provider_fixture", "assistantMessageID": body["assistant"]}})
                    status()
                return self.respond(None, 204)
            if url.path == "/fixture/transcript" and args.prompts:
                scenario = body.get("scenario")
                if scenario not in ("text", "thinking", "answer", "retry", "failed", "interrupted", "idle-incomplete", "silent"):
                    return self.respond({"message": "Invalid transcript scenario"}, 400)
                with lock:
                    requests.append({"operation": "fixture-transcript", "scenario": scenario})
                    if scenario in ("text", "thinking", "idle-incomplete"):
                        counter += 1
                        lifecycle_assistant = f"fixture-lifecycle-{counter}"
                    session_id, assistant_id = "ses_provider_fixture", lifecycle_assistant
                    def emit(kind, **data):
                        publish({"type": kind, "created": int(time.time() * 1000),
                                 "data": {"sessionID": session_id, **data}})
                    if scenario in ("text", "idle-incomplete"):
                        emit("session.step.started", assistantMessageID=assistant_id)
                        emit("session.text.started", assistantMessageID=assistant_id, ordinal=0)
                        emit("session.text.delta", assistantMessageID=assistant_id, ordinal=0,
                             delta="Streaming response 🌍\n\nA second paragraph.")
                    if scenario == "thinking":
                        emit("session.step.started", assistantMessageID=assistant_id)
                        emit("session.reasoning.started", assistantMessageID=assistant_id, ordinal=0)
                        emit("session.reasoning.delta", assistantMessageID=assistant_id, ordinal=0,
                             delta="Inspect the existing behavior.\n\nThen preserve every detail.")
                    if scenario == "answer":
                        emit("session.reasoning.ended", assistantMessageID=assistant_id, ordinal=0,
                             text="Inspect the existing behavior.\n\nThen preserve every detail.")
                        emit("session.text.started", assistantMessageID=assistant_id, ordinal=1)
                        emit("session.text.delta", assistantMessageID=assistant_id, ordinal=1,
                             delta="Final response 🌍\n\nAll fixture details are retained.")
                        emit("session.text.ended", assistantMessageID=assistant_id, ordinal=1,
                             text="Final response 🌍\n\nAll fixture details are retained.")
                        emit("session.step.ended", assistantMessageID=assistant_id, finish="stop")
                        emit("session.execution.succeeded")
                    if scenario == "retry":
                        emit("session.retry.scheduled", assistantMessageID=assistant_id, attempt=2,
                             at=int(time.time() * 1000) + 45000,
                             error={"code": "fixture.limit", "message": "Retrying the disposable request"})
                    elif scenario == "failed":
                        emit("session.step.failed", assistantMessageID=assistant_id,
                             error={"code": "fixture.failed", "message": "Disposable request failed"})
                    elif scenario == "interrupted":
                        emit("session.execution.interrupted")
                    elif scenario == "idle-incomplete":
                        emit("session.status", status={"type": "idle"})
                    elif scenario == "silent":
                        emit("session.execution.started")
                    status()
                return self.respond(None, 204)
            if url.path == "/api/session/ses_provider_fixture/interrupt" and args.prompts:
                with lock:
                    requests.append({"operation": "interrupt", "session": "ses_provider_fixture"})
                    publish({"type": "session.execution.interrupted", "data": {"sessionID": "ses_provider_fixture"}})
                    status()
                return self.respond(None, 204)
            if url.path == "/fixture/permission" and args.permissions:
                owner = body.get("sessionID", "ses_provider_fixture")
                if owner not in ("ses_provider_fixture", "ses_fixture_background"):
                    return self.respond({"message": "Unknown fixture session"}, 400)
                with lock:
                    counter += 1
                    identifier = body.get("id", f"req_fixture_{counter}")
                    if not isinstance(identifier, str) or not re.fullmatch(r"req_fixture_[a-zA-Z0-9_]{1,48}", identifier):
                        return self.respond({"message": "Invalid fixture permission ID"}, 400)
                    permission = {"id": identifier, "sessionID": owner, "action": "fixture",
                                  "resources": ["Disposable fixture action. No command is executed."]}
                    permissions[identifier] = permission
                    requests.append({"operation": "permission-injected", "id": identifier, "session": owner})
                    publish({"type": "permission.asked", "data": permission})
                    status()
                return self.respond({"data": {"id": identifier}}, 201)
            permission_reply = re.fullmatch(r"/api/session/(ses_provider_fixture|ses_fixture_background)/permission/(req_fixture_[a-zA-Z0-9_]+)/reply", url.path)
            if permission_reply and args.permissions:
                decision = body.get("decision")
                if decision not in ("allow", "always", "deny"):
                    return self.respond({"message": "Invalid fixture decision"}, 400)
                with lock:
                    identifier = permission_reply.group(2)
                    permission = permissions.get(identifier)
                    if not permission or permission["sessionID"] != permission_reply.group(1):
                        return self.respond({"message": "Unknown fixture permission"}, 404)
                    del permissions[identifier]
                    requests.append({"operation": "permission-replied", "id": identifier, "session": permission_reply.group(1), "decision": decision})
                    publish({"type": "permission.replied", "data": {"sessionID": permission_reply.group(1), "requestID": identifier, "reply": decision}})
                    status()
                return self.respond(None, 204)
            setting = re.fullmatch(r"/api/session/(ses_provider_fixture)/(model|agent)", url.path)
            if setting and args.models:
                kind = setting.group(2)
                value = body.get(kind)
                if kind == "model":
                    if not isinstance(value, dict) or not any(value.get("id") == model["id"] and value.get("providerID") == model["providerID"] for model in model_catalog):
                        return self.respond({"message": "Unknown fixture model"}, 400)
                    if value["id"] == "reject-selection" or value.get("variant") == "reject-variant":
                        with lock:
                            requests.append({"operation": "model-rejected", "session": setting.group(1), "model": value})
                            status()
                        return self.respond({"message": "Deliberate fixture model rejection"}, 400)
                elif not any(value == agent["id"] for agent in agent_catalog):
                    return self.respond({"message": "Unknown fixture agent"}, 400)
                with lock:
                    selection[kind] = value
                    requests.append({"operation": f"select-{kind}", "session": setting.group(1), kind: value})
                    status()
                return self.respond(None, 204)
            oauth = re.fullmatch(r"/api/integration/(opencode-go|commandcode|openai)/connect/oauth", url.path)
            if oauth and args.oauth:
                method = body.get("methodID")
                if method not in ("auto", "code", "expired", "slow"):
                    return self.respond({"message": "Unknown fixture method"}, 400)
                with lock:
                    counter += 1
                    identifier = f"oauth-{counter}"
                    attempts[identifier] = {"method": method, "provider": oauth.group(1), "status": "pending", "polls": 0}
                    requests.append({"operation": "oauth-start", "id": identifier, "method": method})
                    status()
                if method == "slow":
                    time.sleep(8)
                return self.respond({"data": {"attemptID": identifier, "mode": "code" if method == "code" else "auto",
                    "instructions": "Use the disposable fixture authorization flow.", "url": f"http://127.0.0.1:{args.port}/authorize/{identifier}"}})
            complete = re.fullmatch(r"/api/integration/[^/]+/connect/oauth/(oauth-[0-9]+)/complete", url.path)
            if complete:
                code = body.get("code")
                if not isinstance(code, str):
                    return self.respond({"message": "Missing fixture code"}, 400)
                with lock:
                    attempt = attempts.get(complete.group(1))
                    if not attempt:
                        return self.respond({"message": "Unknown fixture attempt"}, 404)
                    requests.append({"operation": "oauth-code", "id": complete.group(1), "code_sha256": hashlib.sha256(code.encode()).hexdigest()})
                    if not code.startswith("reject-"):
                        attempt["status"] = "complete"
                        credentials.setdefault(attempt["provider"], []).append({"type": "credential", "id": complete.group(1), "label": "Code fixture"})
                    status()
                if code.startswith("reject-"):
                    return self.respond({"message": f"Deliberately echoed {code}"}, 400)
                return self.respond(code=204)
            if not match:
                return self.respond({"message": "Unknown fixture endpoint"}, 404)
            key = body.get("key")
            if not isinstance(key, str) or not key:
                return self.respond({"message": "Missing fixture key"}, 400)
            provider = match.group(1)
            directory = parse_qs(url.query).get("location[directory]", [None])[0]
            reject = key.startswith("reject-") or directory != args.directory
            delay = 8 if key.startswith("delay-") else 0
            with lock:
                counter += 1
                credential_id = f"fixture-{counter}"
                record = {"id": credential_id, "provider": provider,
                          "key_sha256": hashlib.sha256(key.encode()).hexdigest(),
                          "key_utf16_length": len(key.encode("utf-16-le")) // 2,
                          "label": body.get("label"), "answer": body.get("answer", {}),
                          "directory_matches": directory == args.directory,
                          "result": "pending"}
                requests.append(record)
                status()
            time.sleep(delay)
            with lock:
                record["result"] = "rejected" if reject else "connected"
                if not reject:
                    credentials.setdefault(provider, []).append({"type": "credential",
                        "id": credential_id, "label": body.get("label") or "API key"})
                status()
            if reject:
                return self.respond({"message": f"Deliberately echoed {key}"}, 400)
            return self.respond(code=204)

        def do_PATCH(self):
            entry = re.fullmatch(r"/api/session/ses_provider_fixture/inbox/(fixture-inbox-[a-zA-Z0-9-]+)", urlsplit(self.path).path)
            if not entry:
                return self.respond({"message": "Unknown fixture endpoint"}, 404)
            length = int(self.headers.get("Content-Length", "0"))
            try:
                body = json.loads(self.rfile.read(length)) if 0 < length <= 65536 else None
            except (ValueError, UnicodeDecodeError):
                body = None
            if not isinstance(body, dict) or body.get("delivery") != "steer":
                return self.respond({"message": "Invalid fixture delivery"}, 400)
            with lock:
                item = next((item for item in inbox if item["id"] == entry.group(1)), None)
                if item is None:
                    return self.respond({"message": "Unknown fixture inbox item"}, 404)
                item["delivery"] = "steer"
                requests.append({"operation": "inbox-steer", "id": entry.group(1)})
                publish({"type": "session.inbox.updated", "data": {"sessionID": "ses_provider_fixture", "inboxID": entry.group(1), "item": item}})
                status()
            return self.respond(None, 204)

        def do_DELETE(self):
            if args.session_actions and urlsplit(self.path).path == "/api/session/ses_provider_fixture":
                with lock:
                    if session_lifecycle["failures"] > 0:
                        session_lifecycle["failures"] -= 1
                        requests.append({"operation": "session-delete", "result": "rejected"})
                        status()
                        return self.respond({"message": "Deliberate synthetic session deletion rejection"}, 503)
                    session_lifecycle["deleted"] = True
                    requests.append({"operation": "session-delete", "result": "deleted"})
                    publish({"type": "session.deleted", "data": {"sessionID": "ses_provider_fixture"}})
                    status()
                return self.respond(None, 204)
            entry = re.fullmatch(r"/api/session/ses_provider_fixture/inbox/(fixture-inbox-[a-zA-Z0-9-]+)", urlsplit(self.path).path)
            if entry:
                with lock:
                    if not any(item["id"] == entry.group(1) for item in inbox):
                        return self.respond({"message": "Unknown fixture inbox item"}, 404)
                    inbox[:] = [item for item in inbox if item["id"] != entry.group(1)]
                    requests.append({"operation": "inbox-cancel", "id": entry.group(1)})
                    publish({"type": "session.inbox.cancelled", "data": {"sessionID": "ses_provider_fixture", "inboxID": entry.group(1)}})
                    status()
                return self.respond(None, 204)
            oauth = re.fullmatch(r"/api/integration/[^/]+/connect/oauth/(oauth-[0-9]+)", urlsplit(self.path).path)
            if oauth:
                with lock:
                    attempt = attempts.get(oauth.group(1))
                    if not attempt:
                        return self.respond({"message": "Unknown fixture attempt"}, 404)
                    if attempt["status"] != "complete":
                        attempt["status"] = "cancelled"
                    requests.append({"operation": "oauth-cancel", "id": oauth.group(1)})
                    status()
                return self.respond(code=204)
            match = re.fullmatch(r"/api/credential/((?:fixture|oauth)-[0-9]+)", urlsplit(self.path).path)
            if not match:
                return self.respond({"message": "Unknown fixture credential"}, 404)
            identifier = match.group(1)
            with lock:
                for provider, items in credentials.items():
                    if any(item["id"] == identifier for item in items):
                        credentials[provider] = [item for item in items if item["id"] != identifier]
                        requests.append({"operation": "remove", "id": identifier, "result": "removed"})
                        status()
                        return self.respond(code=204)
            return self.respond({"message": "Unknown fixture credential"}, 404)

    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    server.daemon_threads = True
    version = "2.1.4"
    if args.runtime:
        runtime = args.state / "runtime-fixture.json"
        config = json.loads(runtime.read_text()) if args.resume and runtime.exists() else {
            "cli_version": "2.1.4", "service_version": "2.1.4", "next_version": "2.1.5", "delay": 2,
        }
        config.update({"url": f"http://127.0.0.1:{args.port}", "pid": os.getpid()})
        runtime.write_text(json.dumps(config, indent=2) + "\n")
        version = config["service_version"]
        (args.state / "bin").mkdir(exist_ok=True)
        executable = args.state / "bin/opencode"
        shutil.copyfile(Path(__file__).with_name("runtime-fixture-cli.py"), executable)
        executable.chmod(0o700)
    registration = json.dumps({
        "url": f"http://127.0.0.1:{args.port}", "pid": os.getpid(), "version": version,
    }) + "\n"
    (args.state / "service.json").write_text(registration)
    (args.state / "opencode").mkdir(exist_ok=True)
    (args.state / "opencode" / "service.json").write_text(registration)
    status()
    print(f"Provider fixture listening on 127.0.0.1:{args.port}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
