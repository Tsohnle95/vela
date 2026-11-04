#!/usr/bin/env node
/**
 * Isolated OpenCode V2 service smoke.
 *
 * Verifies, against the installed `opencode` CLI, that Vela's V2 contract
 * works end to end without touching the user's OpenCode data and without
 * sending a model prompt or using provider credentials:
 *
 *   1. `Service.ensure` starts a private V2 service under a temporary HOME/XDG
 *      state directory.
 *   2. `Service.discover` finds that registered service and reports the same
 *      endpoint.
 *   3. `OpenCode.make` produces a working client; `session.create`,
 *      `session.get`, and `session.remove` round-trip a session in a temporary
 *      workspace. A fixture import exercises revert stage/commit and verifies
 *      that the selected prompt disappears while earlier prompts remain.
 *   4. `event.subscribe` delivers at least one streamed event.
 *   5. `Service.stop` terminates the private service; temporary directories are
 *      removed.
 *
 * Run with: npm run smoke:opencode
 */
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import net from "node:net";
import { OpenCode } from "@opencode/client";
import { Service } from "@opencode/client/service";

const step = (message) => console.log(`[smoke] ${message}`);

/** Reserve a free loopback port so the isolated service never fights the
 *  user's managed service, which binds its own fixed default port. */
const freePort = await new Promise((resolve, reject) => {
  const server = net.createServer();
  server.on("error", reject);
  server.listen(0, "127.0.0.1", () => {
    const { port } = server.address();
    server.close(() => resolve(port));
  });
});

const home = await mkdtemp(path.join(tmpdir(), "vela-opencode-smoke-"));
const workspace = await mkdtemp(path.join(tmpdir(), "vela-opencode-workspace-"));
const env = {
  HOME: home,
  XDG_CONFIG_HOME: path.join(home, ".config"),
  XDG_DATA_HOME: path.join(home, ".local", "share"),
  XDG_STATE_HOME: path.join(home, ".local", "state"),
  XDG_CACHE_HOME: path.join(home, ".cache")
};
const file = path.join(env.XDG_STATE_HOME, "opencode", "service.json");
const abort = new AbortController();
const watchdog = setTimeout(() => {
  console.error("[smoke] FAIL: timed out after 120s");
  abort.abort();
  process.exitCode = 1;
}, 120_000);

let ensured = false;

try {
  step(`starting an isolated V2 service on port ${freePort} (state: ${file})`);
  const endpoint = await Service.ensure({
    file,
    command: ["opencode", "serve", "--service", "--port", String(freePort)],
    env,
    onStart: (reason, previousVersion) =>
      step(`service starting (${reason}${previousVersion ? `, previous ${previousVersion}` : ""})`)
  });
  ensured = true;
  step(`service ready at ${endpoint.url}`);

  step("discovering the registered service");
  const found = await Service.discover({ file });
  if (!found || found.url !== endpoint.url) {
    throw new Error(`Service.discover returned ${found?.url ?? "nothing"}, expected ${endpoint.url}`);
  }

  const client = OpenCode.make({ baseUrl: endpoint.url, headers: Service.headers(endpoint) });

  step("subscribing to the SSE event stream");
  const iterator = client.event.subscribe({ signal: abort.signal })[Symbol.asyncIterator]();

  step("creating a session in the temporary workspace");
  const created = await client.session.create({ location: { directory: workspace } });
  const sessionID = created?.id ?? created?.data?.id;
  if (!sessionID) throw new Error("session.create returned no id");
  step(`session created: ${sessionID}`);

  step("reading the session back");
  const fetched = await client.session.get({ sessionID });
  const fetchedID = fetched?.id ?? fetched?.data?.id;
  if (fetchedID !== sessionID) {
    throw new Error(`session.get returned ${fetchedID ?? "no id"}, expected ${sessionID}`);
  }

  step("importing two fixture prompts without calling a model");
  const transfer = await client.session.export({ sessionID });
  const firstMessageID = "msg_fixture_first";
  const revertedMessageID = "msg_fixture_second";
  transfer.messages = [
    { id: firstMessageID, type: "user", text: "Keep this prompt", time: { created: Date.now() - 100 } },
    { id: revertedMessageID, type: "user", text: "Undo this prompt", time: { created: Date.now() } }
  ];
  await client.session.remove({ sessionID });
  const imported = await client.session.import(transfer);
  if (imported.id !== sessionID) throw new Error("session.import changed the fixture session id");
  const beforeRevert = await client.message.list({ sessionID });
  if (![firstMessageID, revertedMessageID].every((id) => beforeRevert.data.some((message) => message.id === id))) {
    throw new Error("fixture prompts were not imported");
  }
  step("staging and committing a runtime prompt undo");
  const staged = await client.session.revert.stage({ sessionID, messageID: revertedMessageID, files: true });
  if (staged.messageID !== revertedMessageID) throw new Error("session.revert.stage returned the wrong boundary");
  await client.session.revert.commit({ sessionID });
  const afterRevert = await client.message.list({ sessionID });
  if (afterRevert.data.some((message) => message.id === revertedMessageID)) {
    throw new Error("committed undo retained the reverted prompt");
  }
  if (!afterRevert.data.some((message) => message.id === firstMessageID)) {
    throw new Error("committed undo unexpectedly removed the earlier prompt");
  }
  step("runtime undo removed only the selected prompt and its following history");

  step("waiting for a streamed event");
  let eventType = null;
  const deadline = Date.now() + 20_000;
  while (!eventType && Date.now() < deadline) {
    const next = await Promise.race([
      iterator.next(),
      new Promise((resolve) => setTimeout(() => resolve({ timeout: true }), 2_000))
    ]);
    if (next.timeout) continue;
    if (next.done) break;
    const type = next.value?.type;
    if (typeof type === "string" && type.length > 0) eventType = type;
  }
  if (!eventType) throw new Error("no event arrived on the subscription");
  step(`event received: ${eventType}`);

  step("deleting the session");
  await client.session.remove({ sessionID });

  step("PASS");
} catch (error) {
  console.error(`[smoke] FAIL: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
} finally {
  clearTimeout(watchdog);
  abort.abort();
  if (ensured) {
    await Service.stop({ file }).catch((error) =>
      console.error(`[smoke] warn: failed to stop the isolated service: ${error.message}`)
    );
  }
  await rm(home, { recursive: true, force: true }).catch(() => {});
  await rm(workspace, { recursive: true, force: true }).catch(() => {});
  step("cleaned up");
}
