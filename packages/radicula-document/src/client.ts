import { handle } from "./handle.js";
import type {
  Address, Binding, Handle, Json, Options, Phase, Reply, Snapshot, State,
} from "./types.js";
import { document, snapshot } from "./value.js";

type Turn<I> = {
  address: Address;
  phase: Exclude<Phase, "signed-out">;
  queue: I[];
  busy: boolean;
  confirmed?: Snapshot;
  document?: Json;
  error?: unknown;
};

export class Client<I> {
  readonly #options: Options<I>;
  readonly #listeners = new Set<() => void>();
  #turn?: Turn<I>;

  constructor(options: Options<I>) {
    this.#options = options;
  }

  get state(): State {
    const turn = this.#turn;
    if (turn === undefined) return { phase: "signed-out", pending: 0 };
    return {
      phase: turn.phase,
      pending: turn.queue.length,
      confirmed: turn.confirmed,
      document: turn.document,
      error: turn.error,
    };
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => { this.#listeners.delete(listener); };
  }

  async select(binding: Binding | null): Promise<void> {
    const current = this.#turn?.address.binding;
    if (binding === null) {
      this.#turn = undefined;
      this.#emit();
      return;
    }
    if (current?.key === binding.key && current.identity === binding.identity) return;
    const address = Object.freeze({
      base: this.#options.base ?? "/",
      binding: Object.freeze({ ...binding }),
    });
    const turn: Turn<I> = { address, phase: "loading", queue: [], busy: false };
    this.#turn = turn;
    await this.#load(turn);
  }

  async refresh(): Promise<void> {
    const turn = this.#turn;
    if (turn === undefined || turn.busy || turn.phase === "unknown") return;
    turn.error = undefined;
    await this.#load(turn);
  }

  string(path: string, fallback?: () => string): Handle<string> {
    return handle(() => this.state, path, (held): held is string => typeof held === "string", fallback);
  }

  integer(path: string, fallback?: () => number): Handle<number> {
    return handle(() => this.state, path, (held): held is number => Number.isSafeInteger(held), fallback);
  }

  number(path: string, fallback?: () => number): Handle<number> {
    return handle(() => this.state, path, (held): held is number => typeof held === "number", fallback);
  }

  boolean(path: string, fallback?: () => boolean): Handle<boolean> {
    return handle(() => this.state, path, (held): held is boolean => typeof held === "boolean", fallback);
  }

  value(path: string, fallback?: () => Json): Handle<Json> {
    return handle(() => this.state, path, (_held): _held is Json => true, fallback);
  }

  update(intent: I): void {
    const turn = this.#turn;
    if (turn?.phase !== "ready" || turn.document === undefined) {
      throw new Error("document is not ready");
    }
    const body = this.#project(turn.document, intent);
    turn.queue.push(intent);
    turn.document = body;
    this.#emit();
    void this.#send(turn);
  }

  #project(body: Json, intent: I): Json {
    return document(this.#options.project(body, intent));
  }

  #emit(): void {
    for (const listener of this.#listeners) {
      queueMicrotask(() => {
        if (this.#listeners.has(listener)) listener();
      });
    }
  }

  #rebuild(turn: Turn<I>, held: Snapshot): void {
    let body = held.document;
    const queue: I[] = [];
    for (const intent of turn.queue) {
      try {
        body = this.#project(body, intent);
        queue.push(intent);
      } catch (error) {
        turn.error = error;
      }
    }
    turn.queue = queue;
    turn.confirmed = held;
    turn.document = body;
  }

  async #load(turn: Turn<I>): Promise<void> {
    turn.busy = true;
    turn.phase = "loading";
    this.#emit();
    try {
      const held = await this.#options.transport.read(turn.address);
      if (this.#turn !== turn) return;
      this.#rebuild(turn, snapshot(held));
      turn.phase = "ready";
    } catch (error) {
      if (this.#turn !== turn) return;
      turn.error = error;
      turn.phase = "failed";
    }
    turn.busy = false;
    this.#emit();
    void this.#send(turn);
  }

  async #request(turn: Turn<I>, held: Snapshot, intent: I): Promise<Reply> {
    try {
      const reply = await this.#options.transport.write({
        ...turn.address, revision: held.revision, intent,
      });
      if (reply.state === "saved") return { state: "saved", snapshot: snapshot(reply.snapshot) };
      if (reply.state === "rejected" || reply.state === "conflict" || reply.state === "unknown") {
        return reply;
      }
      return { state: "unknown", error: new Error("invalid write response") };
    } catch (error) {
      return { state: "unknown", error };
    }
  }

  async #send(turn: Turn<I>): Promise<void> {
    if (this.#turn !== turn || turn.busy || turn.phase !== "ready") return;
    const held = turn.confirmed;
    if (held === undefined || turn.queue.length === 0) return;
    turn.busy = true;
    const reply = await this.#request(turn, held, turn.queue[0]);
    if (this.#turn !== turn) return;
    turn.busy = false;
    if (reply.state === "unknown") {
      turn.phase = "unknown";
      turn.error = reply.error ?? new Error("write outcome is unknown");
      this.#emit();
      return;
    }
    turn.queue.shift();
    if (reply.state === "conflict") {
      turn.error = reply.error ?? new Error("document revision conflict");
      await this.#load(turn);
      return;
    }
    turn.error = reply.state === "saved" ? undefined : reply.error ?? new Error("write rejected");
    this.#rebuild(turn, reply.state === "saved" ? reply.snapshot : held);
    this.#emit();
    void this.#send(turn);
  }
}
