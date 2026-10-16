import {
  createReviewAgent,
  disposeAgent,
  ensureCodegraphIndex,
  runAgentPrompt,
} from './agent.js';
import { bundleFiles, selectReviewFiles, type ChangedFile } from './files.js';
import {
  filterFindings,
  mergeReviewResults,
  parseReviewResult,
  type ReviewFinding,
  type ReviewResult,
} from './findings.js';
import {
  createOctokit,
  createReviewBatched,
  findingsToReviewComments,
  getDiffAnchors,
  getPull,
  listChangedFiles,
  upsertStickySummary,
  type RepoContext,
} from './github.js';
import {
  factCheckSystemPrompt,
  reviewSystemPrompt,
  reviewUserPrompt,
} from './prompts.js';
import { loadReviewConfig, matchingRules } from './rules.js';
import { FINDING_MARKER } from './markers.js';
import { extractLastJsonFence } from './findings.js';
import { z } from 'zod';

export interface ReviewOptions {
  token: string;
  apiKey: string;
  model: string;
  cwd: string;
  repo: RepoContext;
  prNumber: number;
  baseRef?: string;
  headSha?: string;
  maxFiles: number;
  batchSize: number;
  dryRun?: boolean;
}

const factCheckSchema = z.object({
  keep: z.array(z.number().int().nonnegative()),
  drop: z
    .array(z.object({ index: z.number().int().nonnegative(), reason: z.string() }))
    .optional(),
});

export async function runReview(opts: ReviewOptions): Promise<void> {
  const octokit = createOctokit(opts.token);
  const pull = await getPull(octokit, opts.repo, opts.prNumber);
  const headSha = opts.headSha || pull.headSha;
  const baseRef = opts.baseRef || pull.baseRef;

  const index = ensureCodegraphIndex(opts.cwd);
  const config = loadReviewConfig(opts.cwd);
  const allFiles = await listChangedFiles(octokit, opts.repo, opts.prNumber);
  const selected = selectReviewFiles(allFiles, config, opts.maxFiles);
  const anchors = getDiffAnchors(allFiles);

  if (selected.selected.length === 0) {
    const summary = [
      `${FINDING_MARKER}`,
      `## Cursor Code Review`,
      ``,
      `No reviewable files after excludes (removed: ${selected.skippedRemoved.length}, excluded: ${selected.skippedExcluded.length}).`,
      index.degraded ? `\n_Note: ${index.message}_` : '',
    ].join('\n');
    if (!opts.dryRun) {
      await upsertStickySummary(octokit, opts.repo, opts.prNumber, summary);
    } else {
      console.log('[dry-run] sticky summary:\n', summary);
    }
    return;
  }

  const pathRules = matchingRules(
    selected.selected.map((f) => f.path),
    config.rules,
  );
  const systemPrompt = reviewSystemPrompt(pathRules);
  const agent = await createReviewAgent({
    apiKey: opts.apiKey,
    model: opts.model,
    cwd: opts.cwd,
    name: 'cursor-code-review',
    systemPrompt,
    enableCodegraphMcp: !index.degraded,
  });

  let merged: ReviewResult = { summary_markdown: '', findings: [] };
  try {
    const bundles = bundleFiles(selected.selected);
    const parts: ReviewResult[] = [];
    for (const bundle of bundles) {
      const user = reviewUserPrompt({
        prNumber: opts.prNumber,
        title: pull.title,
        baseRef,
        headSha,
        files: bundle,
      });
      const { text, requestId } = await runAgentPrompt(agent, user);
      console.log(`review bundle requestId=${requestId ?? 'n/a'} files=${bundle.length}`);
      try {
        const parsed = parseReviewResult(text);
        const paths = new Set(bundle.map((f) => f.path));
        parts.push(filterFindings(parsed, paths));
      } catch (err) {
        console.warn(
          `failed to parse review JSON: ${err instanceof Error ? err.message : String(err)}`,
        );
        parts.push({
          summary_markdown: `_Bundle parse failure for ${bundle.map((f) => f.path).join(', ')}_`,
          findings: [],
        });
      }
    }
    merged = mergeReviewResults(parts);
    merged = await optionalFactCheck(opts, merged, selected.selected);
  } finally {
    await disposeAgent(agent);
  }

  const { comments, outside } = findingsToReviewComments(merged.findings, anchors);

  const stickyParts: string[] = [
    `## Cursor Code Review`,
    ``,
    merged.summary_markdown.trim() ||
      (merged.findings.length === 0
        ? 'No issues found.'
        : `Posted **${merged.findings.length}** finding(s).`),
    ``,
    `Reviewed ${selected.selected.length} file(s)` +
      (selected.overflow.length
        ? ` (${selected.overflow.length} over max_files=${opts.maxFiles} skipped: ${selected.overflow.slice(0, 10).join(', ')}${selected.overflow.length > 10 ? '…' : ''})`
        : '') +
      `.`,
  ];
  if (index.degraded) {
    stickyParts.push(``, `_Degraded mode: ${index.message}_`);
  }
  if (outside.length > 0) {
    stickyParts.push(``, `### Findings outside diff anchors`);
    for (const f of outside) {
      stickyParts.push(
        `- **[${f.severity}/${f.category}] ${f.path}:${f.start_line}** — ${f.title}: ${f.body}`,
      );
    }
  }

  const sticky = stickyParts.join('\n');

  if (opts.dryRun) {
    console.log('[dry-run] sticky:\n', sticky);
    console.log('[dry-run] comments:', comments.length);
    for (const c of comments) {
      console.log(`  ${c.path}:${c.line}`);
    }
    return;
  }

  await createReviewBatched(octokit, opts.repo, {
    prNumber: opts.prNumber,
    commitId: headSha,
    body: `${FINDING_MARKER}\nCursor Code Review completed.`,
    comments,
    batchSize: opts.batchSize,
  });
  await upsertStickySummary(octokit, opts.repo, opts.prNumber, sticky);
}

async function optionalFactCheck(
  opts: ReviewOptions,
  result: ReviewResult,
  files: ChangedFile[],
): Promise<ReviewResult> {
  if (result.findings.length === 0) return result;
  let agent = null;
  try {
    agent = await createReviewAgent({
      apiKey: opts.apiKey,
      model: opts.model,
      cwd: opts.cwd,
      name: 'cursor-code-review-filter',
      systemPrompt: factCheckSystemPrompt(),
      tools: [],
      enableCodegraphMcp: false,
    });
    const prompt = [
      '## Findings',
      '```json',
      JSON.stringify(result.findings, null, 2),
      '```',
      '',
      '## Diffs (truncated)',
      ...files.slice(0, 30).map((f) => `### ${f.path}\n\`\`\`diff\n${(f.patch ?? '').slice(0, 4000)}\n\`\`\``),
    ].join('\n');
    const { text } = await runAgentPrompt(agent, prompt);
    const raw = extractLastJsonFence(text);
    if (!raw) return result;
    const parsed = factCheckSchema.parse(JSON.parse(raw));
    const keepSet = new Set(parsed.keep);
    // If keep is empty but drop lists some, keep everything not dropped
    let kept: ReviewFinding[];
    if (keepSet.size > 0) {
      kept = result.findings.filter((_, i) => keepSet.has(i));
    } else if (parsed.drop && parsed.drop.length > 0) {
      const dropSet = new Set(parsed.drop.map((d) => d.index));
      kept = result.findings.filter((_, i) => !dropSet.has(i));
    } else {
      return result;
    }
    for (const d of parsed.drop ?? []) {
      console.log(`fact-check dropped [${d.index}]: ${d.reason}`);
    }
    return { summary_markdown: result.summary_markdown, findings: kept };
  } catch (err) {
    console.warn(
      `fact-check skipped: ${err instanceof Error ? err.message : String(err)}`,
    );
    return result;
  } finally {
    await disposeAgent(agent);
  }
}
