export type EventPayload = {
  pull_request?: { number?: number };
  issue?: { number?: number };
};

export type Env = Record<string, string | undefined>;
