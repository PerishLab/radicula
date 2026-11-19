import type { Json, Presence } from "./types.js";
import { Fault, list } from "./value.js";

export function parts(path: string): readonly string[] {
  if (path === "") return [];
  if (!path.startsWith("/")) throw new Fault("pointer");
  return path.slice(1).split("/").map(segment);
}

function segment(text: string): string {
  if (/~(?![01])/u.test(text)) throw new Fault("escape");
  return text.replace(/~1/gu, "/").replace(/~0/gu, "~");
}

function child(held: Json, key: string): Json | undefined {
  if (list(held)) {
    if (!/^(0|[1-9][0-9]*)$/u.test(key)) throw new Fault("index");
    const offset = Number(key);
    return offset < held.length ? held[offset] : undefined;
  }
  if (held === null || typeof held !== "object") throw new Fault("kind");
  return Object.hasOwn(held, key) ? held[key] : undefined;
}

export function locate(held: Json, path: readonly string[]): Presence {
  let value = held;
  for (const key of path) {
    const next = child(value, key);
    if (next === undefined) return { state: "missing" };
    value = next;
  }
  return { state: "present", value };
}

export function read(held: Json, path: string): Presence {
  return locate(held, parts(path));
}
