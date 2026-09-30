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

function evalExpression(expr: string, isPullRequest: boolean): unknown {
  // Only the shape used by the workflow: `cond && X || Y`
  const match = /^\$\{\{\s*github\.event_name\s*(!=|==)\s*'pull_request'\s*&&\s*(\S+)\s*\|\|\s*(\S+)\s*\}\}$/.exec(
    expr.trim(),
  );
  if (!match) throw new Error(`unrecognised expression: ${expr}`);

  const [, operator, whenTrue, whenFalse] = match;
  const cond = operator === "!=" ? !isPullRequest : isPullRequest;
  const truthy = whenTrue.includes("secrets.") ? SECRET : whenTrue.replace(/^'|'$/g, "");
  const falsy = whenFalse.includes("secrets.") ? SECRET : whenFalse.replace(/^'|'$/g, "");

  return or(and(cond, truthy), falsy);
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
  // must never reach a pull_request build, which executes PR-controlled code
  // (astro.config.mjs, package.json, npm ci install scripts). The first version
  // of this line was inverted and leaked the key on *every* event, because an
  // empty string is falsy so `'' || secret` resolves to the secret.
  it("withholds the personal API key on pull_request events", () => {
    expect(evalExpression(keyExpression(), true)).toBe("");
  });

  it("provides the personal API key on trusted events", () => {
    expect(evalExpression(keyExpression(), false)).toBe(SECRET);
  });

  // The project token is public and write-only by design, so PR builds may
  // have it; that is what makes analytics testable on a fork PR.
  it("keeps the public project token available to pull requests", () => {
    expect(readFileSync(WORKFLOW, "utf8")).toContain(
      "PUBLIC_POSTHOG_KEY: ${{ secrets.POSTHOG_PROJECT_KEY }}",
    );
  });
});
