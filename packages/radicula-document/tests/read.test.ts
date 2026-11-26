import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { connect, document, parse, read } from "../dist/index.js";

const cases = JSON.parse(readFileSync(
  "../../crates/document/spec/conformance.json",
  "utf8",
)).cases;

test("shared Rust reading and value cases", () => {
  for (const row of cases) {
    if (row.op.read === undefined && Object.keys(row.op).length > 0) continue;
    const run = () => {
      const held = parse(row.document);
      return row.op.read === undefined ? held : read(held, row.op.read);
    };
    if (row.expect.error !== undefined) {
      assert.throws(run, { code: row.expect.error }, row.name);
      continue;
    }
    const held = run();
    if (row.op.read === undefined) continue;
    assert.equal(held.state, row.expect.presence, row.name);
    assert.deepEqual(held.value, row.expect.value, row.name);
  }
});

test("own keys, finite numbers and cyclic data", () => {
  const held = parse('{"__proto__":{"letter":"a"},"constructor":false}');
  assert.deepEqual(read(held, "/__proto__/letter"), { state: "present", value: "a" });
  assert.deepEqual(read(held, "/constructor"), { state: "present", value: false });
  assert.deepEqual(read(held, "/toString"), { state: "missing" });
  assert.throws(() => read([1], "/0\n"), { code: "index" });
  assert.throws(() => parse("1e400"), { code: "syntax" });
  assert.throws(() => parse("9007199254740992"), { code: "number" });
  const circle = {};
  circle.self = circle;
  assert.throws(() => document(circle), { code: "syntax" });
});

test("lazy defaults, explicit empty and null, initial loading, and missing values", async () => {
  let calls = 0;
  let tone = "light";
  const client = connect({
    base: "/folio",
    transport: {
      async read(address) {
        assert.equal(address.base, "/folio");
        return { revision: "r1", document: { empty: "", null: null, count: 4 } };
      },
      async write() { assert.fail("unexpected write"); },
    },
    project: (held) => held,
  });
  const fallback = () => { calls += 1; return tone; };
  const missing = client.string("/absent", fallback);
  const empty = client.string("/empty", fallback);
  const nil = client.value("/null", fallback);
  assert.equal(calls, 0);
  assert.deepEqual(missing.read(), { state: "value", value: "light", source: "default" });
  const load = client.select({ key: "preferences", identity: "a" });
  assert.deepEqual(missing.read(), { state: "loading" });
  await load;
  tone = "dark";
  assert.deepEqual(missing.read(), { state: "value", value: "dark", source: "default" });
  assert.deepEqual(empty.read(), { state: "value", value: "", source: "document" });
  assert.deepEqual(nil.read(), { state: "value", value: null, source: "document" });
  assert.equal(client.string("/null", fallback).read().error.code, "type");
  assert.equal(client.integer("/count").read().value, 4);
  assert.equal(client.number("/absent", () => Infinity).read().error.code, "syntax");
  assert.equal(calls, 2);
});
