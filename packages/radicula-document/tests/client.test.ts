import assert from "node:assert/strict";
import test from "node:test";
import { connect } from "../dist/index.js";

const alice = { key: "preferences", identity: "alice" };
const bob = { key: "preferences", identity: "bob" };
const held = (revision, tone) => ({ revision, document: { tone } });
const tick = () => new Promise((resolve) => setImmediate(resolve));

function task(queue, request) {
  return new Promise((resolve, reject) => queue.push({ request, resolve, reject }));
}

function fixture() {
  const reads = [];
  const writes = [];
  const client = connect({
    base: "/folio",
    transport: {
      read: (request) => task(reads, request),
      write: (request) => task(writes, request),
    },
    project: (body, tone) => ({ ...body, tone }),
  });
  return { client, reads, writes };
}

async function ready() {
  const setup = fixture();
  const loaded = setup.client.select(alice);
  setup.reads[0].resolve(held("r1", "light"));
  await loaded;
  return setup;
}

test("immediate display, first rejection, queued intent and canonical success", async () => {
  const { client, reads, writes } = await ready();
  let notices = 0;
  const stop = client.subscribe(() => { notices += 1; });
  client.update("dark");
  assert.equal(client.string("/tone").read().value, "dark");
  assert.equal(client.state.confirmed.revision, "r1");
  client.update("light");
  assert.equal(client.state.pending, 2);
  assert.equal(writes.length, 1);
  writes[0].resolve({ state: "rejected", error: "write denied" });
  await tick();
  assert.equal(client.state.error, "write denied");
  assert.equal(client.string("/tone").read().value, "light");
  assert.equal(writes[1].request.revision, "r1");
  assert.equal(writes[1].request.intent, "light");
  writes[1].resolve({ state: "saved", snapshot: held("r2", "dark") });
  await tick();
  stop();
  assert.ok(notices > 0);
  assert.equal(client.string("/tone").read().value, "dark");
  assert.equal(client.state.confirmed.revision, "r2");
  assert.equal(client.state.pending, 0);
  assert.equal(reads.length, 1);
});

test("conflict reads fresh confirmation before sending the next intention", async () => {
  const { client, reads, writes } = await ready();
  client.update("dark");
  client.update("system");
  writes[0].resolve({ state: "conflict" });
  await tick();
  assert.equal(client.state.phase, "loading");
  assert.equal(writes.length, 1);
  reads[1].resolve(held("r4", "light"));
  await tick();
  assert.equal(writes[1].request.revision, "r4");
  assert.equal(writes[1].request.intent, "system");
  writes[1].resolve({ state: "saved", snapshot: held("r5", "system") });
  await tick();
  assert.equal(client.state.confirmed.revision, "r5");
});

test("uncertain write freezes the queue and preserves confirmation", async () => {
  const { client, reads, writes } = await ready();
  client.update("dark");
  client.update("light");
  writes[0].reject(new Error("timeout"));
  await tick();
  assert.equal(client.state.phase, "unknown");
  assert.equal(client.state.confirmed.revision, "r1");
  assert.equal(client.state.pending, 2);
  await client.refresh();
  await client.select(alice);
  assert.equal(reads.length, 1);
  assert.equal(writes.length, 1);
  assert.throws(() => client.update("dark"));
  assert.equal(client.string("/tone", () => assert.fail()).read().state, "unknown");
});

test("identity changes discard old pending writes and late snapshots", async () => {
  const { client, reads, writes } = await ready();
  client.update("dark");
  client.update("system");
  const next = client.select(bob);
  reads[1].resolve(held("bob1", "light"));
  await next;
  writes[0].resolve({ state: "saved", snapshot: held("alice2", "dark") });
  await tick();
  assert.equal(client.state.confirmed.revision, "bob1");
  assert.equal(client.state.pending, 0);
  assert.equal(client.string("/tone").read().value, "light");
  assert.equal(writes.length, 1);
  await client.select(null);
  assert.equal(client.state.phase, "signed-out");
  assert.equal(client.state.confirmed, undefined);
});

test("refresh replaces a failed read without resolving defaults against an error", async () => {
  const { client, reads } = fixture();
  const loaded = client.select(alice);
  reads[0].reject(new Error("offline"));
  await loaded;
  const tone = client.string("/tone", () => assert.fail("unexpected default"));
  assert.equal(tone.read().state, "failed");
  const retried = client.refresh();
  reads[1].resolve(held("r1", "dark"));
  await retried;
  assert.equal(tone.read().value, "dark");
  assert.equal(client.state.error, undefined);
});
