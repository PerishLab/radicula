import type { Json, Snapshot } from "./types.js";

export type Code = "escape" | "index" | "kind" | "number" | "pointer" | "syntax" | "type";

export class Fault extends Error {
  readonly code: Code;

  constructor(code: Code) {
    super(code);
    this.code = code;
  }
}

function number(held: number): number {
  if (!Number.isFinite(held)) throw new Fault("syntax");
  if (Number.isInteger(held) && !Number.isSafeInteger(held)) throw new Fault("number");
  return held;
}

function object(held: object, seen: Set<object>): Json {
  if (seen.has(held)) throw new Fault("syntax");
  seen.add(held);
  const body = Array.isArray(held)
    ? Object.freeze(Array.from(held, (item) => tree(item, seen)))
    : record(held, seen);
  seen.delete(held);
  return body;
}

function record(held: object, seen: Set<object>): Json {
  const kind = Object.getPrototypeOf(held);
  if (kind !== null && kind !== Object.prototype) throw new Fault("syntax");
  const pairs = Object.entries(held).map(([key, item]) => [key, tree(item, seen)] as const);
  return Object.freeze(Object.fromEntries(pairs));
}

function tree(held: unknown, seen: Set<object>): Json {
  if (held === null || typeof held === "string" || typeof held === "boolean") return held;
  if (typeof held === "number") return number(held);
  if (typeof held === "object") return object(held, seen);
  throw new Fault("syntax");
}

export function document(held: unknown): Json {
  return tree(held, new Set());
}

export function parse(text: string): Json {
  let held: unknown;
  try {
    held = JSON.parse(text);
  } catch {
    throw new Fault("syntax");
  }
  return document(held);
}

export function snapshot(held: Snapshot): Snapshot {
  if (typeof held.revision !== "string" || held.revision.length === 0) throw new Fault("type");
  return Object.freeze({ revision: held.revision, document: document(held.document) });
}

export function list(held: Json): held is readonly Json[] {
  return Array.isArray(held);
}
