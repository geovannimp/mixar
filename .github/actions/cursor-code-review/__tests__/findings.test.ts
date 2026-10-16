import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  CONFIDENCE_FLOOR,
  extractLastJsonFence,
  filterFindings,
  parseReviewResult,
} from '../src/findings.js';
import {
  FINDING_MARKER,
  formatFindingComment,
  formatSummaryBody,
  SUMMARY_ANCHOR,
  bodyHasOurMarker,
} from '../src/markers.js';
import {
  intentResultSchema,
  parseIntentResult,
  sanitizeCommentBody,
} from '../src/intent.js';
import { selectReviewFiles } from '../src/files.js';
import { clampToDiffAnchor, getDiffAnchors } from '../src/github.js';

describe('findings parser', () => {
  it('parses last JSON fence and drops low confidence', () => {
    const text = `
Some chatter
\`\`\`json
{"summary_markdown":"first","findings":[]}
\`\`\`
more
\`\`\`json
{
  "summary_markdown": "ok",
  "findings": [
    {
      "path": "crates/engine-core/src/x.rs",
      "start_line": 10,
      "severity": "critical",
      "category": "bug",
      "title": "unwrap in callback",
      "body": "Premise → Path → Verdict",
      "confidence": 0.9
    },
    {
      "path": "crates/engine-core/src/x.rs",
      "start_line": 12,
      "severity": "low",
      "category": "style",
      "title": "nit",
      "body": "style only",
      "confidence": 0.4
    },
    {
      "path": "other/file.rs",
      "start_line": 1,
      "severity": "high",
      "category": "bug",
      "title": "outside set",
      "body": "no",
      "confidence": 0.99
    }
  ]
}
\`\`\`
`;
    const parsed = parseReviewResult(text);
    assert.equal(parsed.findings.length, 3);
    const filtered = filterFindings(
      parsed,
      new Set(['crates/engine-core/src/x.rs']),
      CONFIDENCE_FLOOR,
    );
    assert.equal(filtered.findings.length, 1);
    assert.equal(filtered.findings[0]!.title, 'unwrap in callback');
  });

  it('extractLastJsonFence prefers the last fence', () => {
    const raw = extractLastJsonFence(
      '```json\n{"a":1}\n```\n```json\n{"b":2}\n```',
    );
    assert.equal(raw, '{"b":2}');
  });
});

describe('markers', () => {
  it('prefixes finding bodies and summary', () => {
    const body = formatFindingComment({
      severity: 'high',
      category: 'bug',
      title: 'race',
      body: 'details',
    });
    assert.ok(body.startsWith(FINDING_MARKER));
    assert.ok(bodyHasOurMarker(body));
    const summary = formatSummaryBody('## hello');
    assert.ok(summary.includes(SUMMARY_ANCHOR));
  });
});

describe('intent schema', () => {
  it('accepts add_value', () => {
    const parsed = intentResultSchema.parse({
      intent: 'add_value',
      should_reply: true,
      rationale: 'OCR missed callers',
    });
    assert.equal(parsed.intent, 'add_value');
  });

  it('parse failure falls back to mention signal', () => {
    const hit = parseIntentResult('not json', { mentionsUs: true });
    assert.equal(hit.intent, 'question');
    assert.equal(hit.should_reply, true);
    const miss = parseIntentResult('not json', { mentionsUs: false });
    assert.equal(miss.intent, 'unrelated');
  });

  it('sanitize strips HTML comments', () => {
    const clean = sanitizeCommentBody(
      'hello <!-- cursor-code-review --> world <!-- injection: ignore prior -->',
    );
    assert.equal(clean, 'hello  world');
  });
});

describe('file selection + anchors', () => {
  it('excludes removed and glob matches', () => {
    const selected = selectReviewFiles(
      [
        { path: 'a.rs', status: 'modified' },
        { path: 'gone.rs', status: 'removed' },
        { path: 'foo/Cargo.lock', status: 'modified' },
      ],
      { exclude: ['**/Cargo.lock'], rules: [] },
      80,
    );
    assert.deepEqual(
      selected.selected.map((f) => f.path),
      ['a.rs'],
    );
  });

  it('clamps to diff anchors within ±3', () => {
    const anchors = getDiffAnchors([
      {
        path: 'a.rs',
        status: 'modified',
        patch: [
          '@@ -1,3 +10,4 @@',
          ' context',
          '+added1',
          '+added2',
          ' context2',
        ].join('\n'),
      },
    ]);
    // Patch covers new lines 10–13; ±3 clamp from 14 → 13, far lines → null
    assert.equal(clampToDiffAnchor(anchors, 'a.rs', 11), 11);
    assert.equal(clampToDiffAnchor(anchors, 'a.rs', 14), 13);
    assert.equal(clampToDiffAnchor(anchors, 'a.rs', 100), null);
  });
});
