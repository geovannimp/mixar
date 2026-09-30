import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const WORKFLOW = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
  "..",
  "..",
  ".github",
  "workflows",
  "pages.yml",
);

const SECRET = "phx_pretend_this_is_the_personal_key";

/**
 * Minimal model of GitHub Actions `&&` / `||`: they return *values*, not
 * booleans, and an empty string is falsy. `a && b` yields `a` when `a` is
 * falsy, otherwise `b`; `a || b` yields `a` when truthy, otherwise `b`.
 */
function and(a: unknown, b: unknown): unknown {
  return a ? b : a;
}

function or(a: unknown, b: unknown): unknown {
  return a ? a : b;
}

type Context = { event: string; ref: string };

/**
 * Evaluates the workflow's gate. Supports the shape it actually uses:
 * one or more `github.*` comparisons, `&&`-chained, then `|| <fallback>`.
 */
function evalExpression(expr: string, ctx: Context): unknown {
  const body = /^\$\{\{(.*)\}\}$/.exec(expr.trim())?.[1];
  if (!body) throw new Error(`unrecognised expression: ${expr}`);

  const [chain, fallback] = body.split("||").map((part) => part.trim());
  if (fallback === undefined) throw new Error(`no fallback branch: ${expr}`);

  const falsy = fallback.includes("secrets.")
    ? SECRET
    : fallback.replace(/^'|'$/g, "");

  let value: unknown = true;
  for (const clause of chain.split("&&").map((c) => c.trim())) {
    const cmp = /^github\.(\w+)\s*(!=|==)\s*'([^']*)'$/.exec(clause);
    if (cmp) {
      const [, field, operator, expected] = cmp;
      const actual = ctx[field as keyof Context];
      value = and(value, operator === "!=" ? actual !== expected : actual === expected);
      continue;
    }
    // A non-comparison operand is the value yielded when the conditions hold.
    value = and(value, clause.includes("secrets.") ? SECRET : clause.replace(/^'|'$/g, ""));
  }

  return or(value, falsy);
}

function keyExpression(): string {
  const line = readFileSync(WORKFLOW, "utf8")
    .split("\n")
    .find((l) => l.includes("POSTHOG_API_KEY:"));
  if (!line) throw new Error("POSTHOG_API_KEY not found in pages.yml");
  return line.slice(line.indexOf("${{"));
}

describe("pages.yml PostHog credentials", () => {
  // This is the security-critical invariant: the write-capable personal token
  // must never reach a build that executes untrusted code. The first version of
  // this line was inverted and leaked the key on *every* event, because an
  // empty string is falsy so `'' || secret` resolves to the secret.
  it("withholds the personal API key on pull_request events", () => {
    expect(evalExpression(keyExpression(), { event: "pull_request", ref: "refs/pull/1/merge" })).toBe("");
  });

  // workflow_dispatch can target any ref. Dispatching against an unmerged
  // branch runs that branch's astro.config.mjs / package.json / npm ci scripts
  // with the secret in its environment, which is the same exfiltration vector
  // the pull_request guard exists to close.
  it("withholds the personal API key on a workflow_dispatch against a branch", () => {
    expect(
      evalExpression(keyExpression(), {
        event: "workflow_dispatch",
        ref: "refs/heads/some-feature-branch",
      }),
    ).toBe("");
  });

  it("provides the personal API key when pushing to main", () => {
    expect(
      evalExpression(keyExpression(), { event: "push", ref: "refs/heads/main" }),
    ).toBe(SECRET);
  });

  it("provides the personal API key when dispatching on main", () => {
    expect(
      evalExpression(keyExpression(), {
        event: "workflow_dispatch",
        ref: "refs/heads/main",
      }),
    ).toBe(SECRET);
  });

  it("withholds the personal API key when pushing to a branch", () => {
    expect(
      evalExpression(keyExpression(), {
        event: "push",
        ref: "refs/heads/some-feature-branch",
      }),
    ).toBe("");
  });

  // The project token is public and write-only by design, so PR builds may
  // have it; that is what makes analytics testable on a fork PR.
  it("keeps the public project token available to pull requests", () => {
    expect(readFileSync(WORKFLOW, "utf8")).toContain(
      "PUBLIC_POSTHOG_KEY: ${{ secrets.POSTHOG_PROJECT_KEY }}",
    );
  });
});
