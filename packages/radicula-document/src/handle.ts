import { locate, parts } from "./pointer.js";
import type { Handle, Json, Presence, Result, State } from "./types.js";
import { document, Fault } from "./value.js";

type Check<T extends Json> = (value: Json) => value is T;

function chosen<T extends Json>(
  presence: Presence,
  check: Check<T>,
  fallback?: () => T,
): Result<T> {
  if (presence.state === "present") {
    if (!check(presence.value)) throw new Fault("type");
    return { state: "value", source: "document", value: presence.value };
  }
  if (fallback === undefined) return { state: "missing" };
  const value = document(fallback());
  if (!check(value)) throw new Fault("type");
  return { state: "value", source: "default", value };
}

function value<T extends Json>(
  state: State,
  path: readonly string[],
  check: Check<T>,
  fallback?: () => T,
): Result<T> {
  if (state.phase === "loading") return { state: "loading" };
  if (state.phase === "unknown" || state.phase === "failed") {
    return { state: state.phase, error: state.error };
  }
  try {
    const presence: Presence = state.document === undefined
      ? { state: "missing" }
      : locate(state.document, path);
    return chosen(presence, check, fallback);
  } catch (error) {
    return { state: "failed", error };
  }
}

export function handle<T extends Json>(
  state: () => State,
  path: string,
  check: Check<T>,
  fallback?: () => T,
): Handle<T> {
  const keys = parts(path);
  return { read: () => value(state(), keys, check, fallback) };
}
