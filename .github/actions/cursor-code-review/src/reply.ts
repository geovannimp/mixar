import {
  createIntentAgent,
  createReviewAgent,
  disposeAgent,
  ensureCodegraphIndex,
  runAgentPrompt,
} from './agent.js';
import {
  extractLastJsonFence,
} from './findings.js';
import {
  createGraphql,
  createOctokit,
  createReply,
  getAuthenticatedLogin,
  getPull,
  getThreadContextForPr,
  isOurReviewComment,
  resolveThreadForComment,
  type RepoContext,
  type ThreadMessage,
} from './github.js';
import {
  mentionsCursorReview,
  parseIntentResult,
  sanitizeCommentBody,
  disputeResultSchema,
  fixedResultSchema,
  type IntentResult,
} from './intent.js';
import {
  addValueSystemPrompt,
  disputeSystemPrompt,
  fixedVerifySystemPrompt,
  intentSystemPrompt,
  replyAnswerSystemPrompt,
} from './prompts.js';
import { loadReviewConfig } from './rules.js';
import { FINDING_MARKER } from './markers.js';
import { runReview } from './review.js';

export interface ReplyOptions {
  token: string;
  apiKey: string;
  model: string;
  cwd: string;
  repo: RepoContext;
  prNumber: number;
  commentId: number;
  headSha?: string;
  dryRun?: boolean;
  /** Prefer from env CURSOR_REVIEW_DRY_RUN fixtures */
  fixtureIntent?: IntentResult;
}

/** In-memory anti-loop ledger for this process (one Actions job). */
const repliedKeys = new Set<string>();
const addValueKeys = new Set<string>();

function threadKey(pr: number, rootId: number, headSha: string): string {
  return `${pr}:${rootId}:${headSha}`;
}

export async function runReply(opts: ReplyOptions): Promise<void> {
  const octokit = createOctokit(opts.token);
  const gql = createGraphql(opts.token);
  const ourLogin = await getAuthenticatedLogin(octokit);
  const pull = await getPull(octokit, opts.repo, opts.prNumber);
  const headSha = opts.headSha || pull.headSha;

  const { root, replies } = await getThreadContextForPr(
    octokit,
    opts.repo,
    opts.prNumber,
    opts.commentId,
  );

  const thread: ThreadMessage[] = [root, ...replies];
  const newest = thread[thread.length - 1]!;

  // Anti-loop: skip if newest is from our bot
  if (
    ourLogin &&
    (newest.authorLogin === ourLogin ||
      newest.authorLogin === 'github-actions[bot]') &&
    isOurReviewComment(newest.body)
  ) {
    console.log('skip: newest comment is ours (anti-loop)');
    return;
  }

  const key = threadKey(opts.prNumber, root.id, headSha);
  if (repliedKeys.has(key) && isOurReviewComment(root.body)) {
    // Already replied on our thread for this head — still allow if newest is new human
    const weRepliedAfter = thread.some(
      (m) =>
        isOurReviewComment(m.body) &&
        m.id !== root.id &&
        m.createdAt >= newest.createdAt,
    );
    if (weRepliedAfter) {
      console.log('skip: already replied in this thread for this head_sha');
      return;
    }
  }

  const sanitized = thread.map((m) => ({
    ...m,
    body: sanitizeCommentBody(m.body),
  }));

  const config = loadReviewConfig(opts.cwd);
  const intentModel = config.intent_model ?? opts.model;

  let intent: IntentResult;
  if (opts.fixtureIntent) {
    intent = opts.fixtureIntent;
  } else {
    intent = await classifyIntent({
      apiKey: opts.apiKey,
      model: intentModel,
      cwd: opts.cwd,
      messages: sanitized,
      newestBody: newest.body,
    });
  }

  console.log(
    `intent=${intent.intent} should_reply=${intent.should_reply} rationale=${intent.rationale}`,
  );

  if (!intent.should_reply || intent.intent === 'unrelated') {
    console.log('no-op: nothing useful to add');
    return;
  }

  const weOwnThread = isOurReviewComment(root.body);
  const dry = opts.dryRun || process.env.CURSOR_REVIEW_DRY_RUN === '1';

  const replyOnce = async (body: string, resolve = false) => {
    if (dry) {
      console.log(`[dry-run] reply (resolve=${resolve}):\n${body}`);
      return;
    }
    await createReply(octokit, opts.repo, opts.prNumber, root.id, body);
    repliedKeys.add(key);
    if (resolve && weOwnThread && root.nodeId) {
      await resolveThreadForComment(
        gql,
        opts.repo,
        opts.prNumber,
        root.nodeId,
      );
    }
  };

  switch (intent.intent) {
    case 'question': {
      ensureCodegraphIndex(opts.cwd);
      const text = await withToolsAgent(
        opts,
        replyAnswerSystemPrompt(),
        renderThreadPrompt(sanitized, 'Answer the newest question.'),
      );
      const askClose = /resolve|close|done|lgtm/i.test(newest.body);
      await replyOnce(text, askClose && weOwnThread);
      break;
    }
    case 'dispute': {
      ensureCodegraphIndex(opts.cwd);
      const text = await withToolsAgent(
        opts,
        disputeSystemPrompt(),
        renderThreadPrompt(sanitized, 'Re-investigate the disputed finding.'),
      );
      const raw = extractLastJsonFence(text) ?? text;
      const parsed = disputeResultSchema.parse(JSON.parse(raw));
      await replyOnce(
        parsed.reply_markdown,
        parsed.verdict === 'withdraw' && weOwnThread,
      );
      break;
    }
    case 'fixed': {
      ensureCodegraphIndex(opts.cwd);
      const text = await withToolsAgent(
        opts,
        fixedVerifySystemPrompt(),
        renderThreadPrompt(
          sanitized,
          `Verify the claimed fix on head ${headSha}.`,
        ),
      );
      const raw = extractLastJsonFence(text) ?? text;
      const parsed = fixedResultSchema.parse(JSON.parse(raw));
      await replyOnce(
        parsed.reply_markdown,
        parsed.verified && weOwnThread,
      );
      break;
    }
    case 'ack': {
      await replyOnce('Noted — thanks.');
      break;
    }
    case 're_review': {
      await replyOnce('Starting a fresh review of the PR head.');
      await runReview({
        token: opts.token,
        apiKey: opts.apiKey,
        model: opts.model,
        cwd: opts.cwd,
        repo: opts.repo,
        prNumber: opts.prNumber,
        headSha,
        maxFiles: Number(process.env.INPUT_MAX_FILES ?? 80),
        batchSize: Number(process.env.INPUT_REVIEW_COMMENT_BATCH_SIZE ?? 50),
        dryRun: dry,
      });
      break;
    }
    case 'add_value': {
      if (!weOwnThread) {
        if (addValueKeys.has(key)) {
          console.log('skip: max 1 add_value reply per foreign thread per head');
          return;
        }
        addValueKeys.add(key);
      }
      ensureCodegraphIndex(opts.cwd);
      const text = await withToolsAgent(
        opts,
        addValueSystemPrompt(),
        renderThreadPrompt(
          sanitized,
          'Add concrete evidence or blast-radius context if warranted.',
        ),
      );
      // Never resolve foreign threads
      await replyOnce(text, false);
      break;
    }
    default:
      console.log('no-op: unhandled intent');
  }
}

async function classifyIntent(opts: {
  apiKey: string;
  model: string;
  cwd: string;
  messages: ThreadMessage[];
  newestBody: string;
}): Promise<IntentResult> {
  let agent = null;
  try {
    agent = await createIntentAgent({
      apiKey: opts.apiKey,
      model: opts.model,
      cwd: opts.cwd,
      name: 'cursor-code-review-intent',
      systemPrompt: intentSystemPrompt(),
    });
    const rendered = opts.messages
      .map((m, i) => {
        const tag = i === opts.messages.length - 1 ? 'NEWEST MESSAGE' : 'message';
        const role = `${m.authorLogin}${m.isBot ? ' (bot)' : ''}${
          isOurReviewComment(m.body) ? ' [cursor-code-review]' : ''
        }`;
        return `<${tag} author="${role}">\n${m.body.slice(0, 2000)}\n</${tag}>`;
      })
      .join('\n\n');
    const { text } = await runAgentPrompt(
      agent,
      `Thread, oldest first:\n\n${rendered}`,
    );
    return parseIntentResult(text, {
      mentionsUs: mentionsCursorReview(opts.newestBody),
    });
  } catch (err) {
    console.warn(
      `intent agent failed: ${err instanceof Error ? err.message : String(err)}`,
    );
    return parseIntentResult('', {
      mentionsUs: mentionsCursorReview(opts.newestBody),
    });
  } finally {
    await disposeAgent(agent);
  }
}

async function withToolsAgent(
  opts: ReplyOptions,
  systemPrompt: string,
  userPrompt: string,
): Promise<string> {
  let agent = null;
  try {
    agent = await createReviewAgent({
      apiKey: opts.apiKey,
      model: opts.model,
      cwd: opts.cwd,
      name: 'cursor-code-review-reply',
      systemPrompt,
      enableCodegraphMcp: true,
    });
    const { text } = await runAgentPrompt(agent, userPrompt);
    return text.trim() || '_No reply generated._';
  } finally {
    await disposeAgent(agent);
  }
}

function renderThreadPrompt(
  messages: ThreadMessage[],
  instruction: string,
): string {
  const rendered = messages
    .map((m) => {
      const ours = isOurReviewComment(m.body) ? ' [ours]' : '';
      return `**${m.authorLogin}${m.isBot ? ' (bot)' : ''}${ours}**:\n${m.body.slice(0, 3000)}`;
    })
    .join('\n\n');
  return `${instruction}\n\nInclude ${FINDING_MARKER} is added by the poster — do not strip context.\n\n## Thread\n${rendered}`;
}

/** Test/dry-run helper: clear anti-loop ledgers. */
export function resetReplyGuards(): void {
  repliedKeys.clear();
  addValueKeys.clear();
}
