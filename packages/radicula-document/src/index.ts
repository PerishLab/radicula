import { Client } from "./client.js";
import type { Options } from "./types.js";

export { Client } from "./client.js";
export { read } from "./pointer.js";
export type {
  Address, Binding, Handle, Json, Options, Phase, Presence, Reply, Request, Result, Snapshot, State, Transport,
} from "./types.js";
export { document, Fault, parse, type Code } from "./value.js";

export function connect<I>(options: Options<I>): Client<I> {
  return new Client(options);
}
