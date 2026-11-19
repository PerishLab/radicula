export type Json =
  | null
  | boolean
  | number
  | string
  | readonly Json[]
  | { readonly [key: string]: Json };

export type Presence =
  | { readonly state: "missing" }
  | { readonly state: "present"; readonly value: Json };

export type Snapshot = {
  readonly revision: string;
  readonly document: Json;
};

export type Binding = {
  readonly key: string;
  readonly identity: string;
};

export type Address = {
  readonly base_rel_path: string;
  readonly binding: Binding;
};

export type Request<I> = Address & {
  readonly revision: string;
  readonly intent: I;
};

export type Reply =
  | { readonly state: "saved"; readonly snapshot: Snapshot }
  | { readonly state: "rejected" | "conflict" | "unknown"; readonly error?: unknown };

export type Transport<I> = {
  read(address: Address): Promise<Snapshot>;
  write(request: Request<I>): Promise<Reply>;
};

export type Options<I> = {
  readonly base_rel_path?: string;
  readonly transport: Transport<I>;
  readonly project: (document: Json, intent: I) => Json;
};

export type Phase = "signed-out" | "loading" | "ready" | "failed" | "unknown";

export type State = {
  readonly phase: Phase;
  readonly pending: number;
  readonly confirmed?: Snapshot;
  readonly document?: Json;
  readonly error?: unknown;
};

export type Result<T> =
  | { readonly state: "value"; readonly value: T; readonly source: "document" | "default" }
  | { readonly state: "missing" | "loading" }
  | { readonly state: "failed" | "unknown"; readonly error: unknown };

export type Handle<T> = {
  read(): Result<T>;
};
