import type { CATEGORIES, SEVERITIES } from './constants.ts';

export type Severity = (typeof SEVERITIES)[number];
export type Category = (typeof CATEGORIES)[number];

export interface ReviewFinding {
  path: string;
  start_line: number;
  end_line?: number;
  severity: Severity;
  category: Category;
  title: string;
  body: string;
  confidence: number;
}

export interface PathRule {
  path: string;
  rule: string;
}

export interface ReviewConfig {
  exclude: string[];
  rules: PathRule[];
}

export interface ReviewCommentInput {
  path: string;
  line: number;
  side: 'RIGHT';
  body: string;
  start_line?: number;
  start_side?: 'RIGHT';
}

/** Map of changed path to the set of new-file line numbers that appear in the diff. */
export type DiffAnchors = Map<string, Set<number>>;

export interface ThreadMessage {
  id: number;
  author: string;
  body: string;
  createdAt: string;
  isBot: boolean;
}

export interface PostReviewParams {
  summary_markdown: string;
  findings: ReviewFinding[];
}

export interface GitHubClient {
  gh<T = unknown>(method: string, path: string, body?: unknown): Promise<T>;
  list<T>(path: string): Promise<T[]>;
  graphql<T>(query: string, variables: Record<string, unknown>): Promise<T>;
}

export type EventPayload = {
  pull_request?: { number?: number };
  issue?: { number?: number };
};

export type Env = Record<string, string | undefined>;

export interface PullPayload {
  title: string;
  base: { ref: string };
  head: { sha: string };
}

export interface FilePayload {
  filename: string;
  status: string;
  patch?: string | null;
}

export interface ReviewCommentPayload {
  id: number;
  node_id: string;
  in_reply_to_id?: number;
  body?: string | null;
  created_at: string;
  user?: { login?: string; type?: string } | null;
}
