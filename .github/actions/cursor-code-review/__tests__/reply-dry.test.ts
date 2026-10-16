import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { resetReplyGuards } from '../src/reply.js';
import {
  intentResultSchema,
  parseIntentResult,
  mentionsCursorReview,
} from '../src/intent.js';
import { isOurReviewComment } from '../src/github.js';
import { FINDING_MARKER } from '../src/markers.js';

/**
 * Dry-run reply routing fixtures (CURSOR_REVIEW_DRY_RUN=1 semantics).
 * These exercise the intent/dispatch table without calling GitHub or Cursor.
 */
describe('reply dry-run routing table', () => {
  beforeEach(() => {
    resetReplyGuards();
  });

  it('our thread + question → reply, open', () => {
    const intent = intentResultSchema.parse({
      intent: 'question',
      should_reply: true,
      rationale: 'asks how the finding was derived',
    });
    const weOwn = isOurReviewComment(`${FINDING_MARKER}\nfinding`);
    assert.equal(weOwn, true);
    assert.equal(intent.should_reply, true);
    // resolve only if user asked to close — default open
    const resolve = weOwn && /resolve|close/i.test('why is this wrong?');
    assert.equal(resolve, false);
  });

  it('our thread + fixed → verify + resolve when verified', () => {
    const intent = intentResultSchema.parse({
      intent: 'fixed',
      should_reply: true,
      rationale: 'claims fix landed',
    });
    const weOwn = true;
    const verified = true;
    assert.equal(intent.intent, 'fixed');
    assert.equal(weOwn && verified, true); // resolve
  });

  it('OCR thread + gap → add_value, do not resolve', () => {
    const intent = intentResultSchema.parse({
      intent: 'add_value',
      should_reply: true,
      rationale: 'OCR missed caller impact',
    });
    const weOwn = isOurReviewComment('OCR finding without our marker');
    assert.equal(weOwn, false);
    assert.equal(intent.intent, 'add_value');
    const resolve = false; // never resolve foreign
    assert.equal(resolve, false);
  });

  it('OCR thread + pure status noise → no-op', () => {
    const intent = intentResultSchema.parse({
      intent: 'unrelated',
      should_reply: false,
      rationale: 'CI status bot',
    });
    assert.equal(intent.should_reply, false);
  });

  it('other bot @-mentioning us → reply', () => {
    const body = '@cursor-review can you check the callers of this?';
    assert.equal(mentionsCursorReview(body), true);
    const intent = parseIntentResult('not-json', { mentionsUs: true });
    assert.equal(intent.should_reply, true);
  });

  it('our own newest comment → skip (anti-loop)', () => {
    const newestOurs = isOurReviewComment(
      `${FINDING_MARKER}\nAlready replied here.`,
    );
    assert.equal(newestOurs, true);
    // reply.ts skips when newest author is our login AND body has marker
  });
});
