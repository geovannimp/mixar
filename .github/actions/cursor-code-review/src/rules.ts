import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { minimatch } from 'minimatch';

export interface PathRule {
  path: string;
  rule: string;
}

export interface ReviewConfig {
  exclude: string[];
  rules: PathRule[];
  intent_model?: string;
}

const DEFAULT_CONFIG: ReviewConfig = {
  exclude: [],
  rules: [],
};

export function loadReviewConfig(cwd: string): ReviewConfig {
  const path = join(cwd, '.cursor-review', 'config.json');
  if (!existsSync(path)) return DEFAULT_CONFIG;
  const raw = JSON.parse(readFileSync(path, 'utf8')) as Partial<ReviewConfig>;
  return {
    exclude: Array.isArray(raw.exclude) ? raw.exclude : [],
    rules: Array.isArray(raw.rules) ? raw.rules : [],
    intent_model:
      typeof raw.intent_model === 'string' ? raw.intent_model : undefined,
  };
}

export function isExcluded(filePath: string, exclude: string[]): boolean {
  return exclude.some((pattern) =>
    minimatch(filePath, pattern, { dot: true, matchBase: true }),
  );
}

/** Collect path rules that match any of the given files (OCR-style injection). */
export function matchingRules(files: string[], rules: PathRule[]): PathRule[] {
  const matched: PathRule[] = [];
  const seen = new Set<string>();
  for (const rule of rules) {
    const hits = files.some((f) =>
      minimatch(f, rule.path, { dot: true, matchBase: true }),
    );
    if (hits) {
      const key = `${rule.path}::${rule.rule.slice(0, 80)}`;
      if (!seen.has(key)) {
        seen.add(key);
        matched.push(rule);
      }
    }
  }
  return matched;
}

export function formatSystemRules(rules: PathRule[]): string {
  if (rules.length === 0) return '';
  return rules
    .map(
      (r, i) =>
        `### Path rule ${i + 1}: \`${r.path}\`\n${r.rule}`,
    )
    .join('\n\n');
}
