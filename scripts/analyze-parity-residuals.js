#!/usr/bin/env node
/**
 * Rank exact-parity residuals by defect-class leverage.
 *
 * Exact-parity analogue of scripts/analyze-oracle-clusters.py, which
 * clusters *fidelity* (lenient) failures. This clusters the strict
 * exact-parity failures a style still carries, across both bibliography
 * (`oracleDetail`) and citation (`citationEntries`) rows, and answers the
 * question that drives the `style-tune` exact-parity loop: which classes
 * of defect, fixed in full, flip the most rows to an exact match?
 *
 * Each failing row is multi-labeled against a fixed set of pattern rules
 * (title-case transitions, quote boundaries, date-detail tokens,
 * contributor-role phrases, genre/medium vocabulary, volume/issue
 * grammar, legal-citation tokens, URL/DOI presence, punctuation-only
 * regions, and more — see LABEL_RULES below). A row commonly carries more
 * than one label. A greedy set cover then orders labels by how many
 * additional rows they fully explain once already-chosen labels are
 * subtracted — fixing labels in that order, in full, flips the most rows
 * per unit of work.
 *
 * See docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md
 * for the method's first application and worked results.
 *
 * Usage:
 *   node scripts/report-core.js --style <name> --all-features > /tmp/r.json
 *   node scripts/analyze-parity-residuals.js /tmp/r.json
 *   node scripts/analyze-parity-residuals.js /tmp/r.json --json
 *   node scripts/analyze-parity-residuals.js /tmp/r.json --by-type <refs.json>
 *   node scripts/analyze-parity-residuals.js /tmp/r.json --list "A1 title-case not applied"
 *   node scripts/analyze-parity-residuals.js /tmp/after.json --diff /tmp/before.json
 *   node scripts/analyze-parity-residuals.js /tmp/after.json --diff /tmp/before.json --json
 *
 * --diff <before.json> compares this report (the "after") against an
 * earlier one, per style plus a summed "ALL STYLES" aggregate: exactMatch
 * delta (newly-passing / newly-failing rows -- the regression signal
 * docs/guides/STYLE_WORKFLOW_EXECUTION.md's exact-parity loop, step 3c,
 * requires before treating any aggregate `passed` increase as a clean win),
 * per-label instance-count delta, a rows-by-label-count histogram, and a
 * near-miss queue (rows carrying exactly one remaining label -- the next
 * wave's ready-to-convert target list). Rows commonly carry 2+ overlapping
 * labels and don't flip to exactMatch until every one clears, so a wave can
 * do real, measurable work while the aggregate `passed` count stays flat;
 * --diff surfaces that progress directly instead of requiring it to be
 * reconstructed by hand, which is how the 2026-08-23 audit's wave 1/2
 * postscript first found it -- see
 * docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md.
 *
 * --by-type additionally joins each failing bibliography row's reference
 * `type` from a CSL-JSON-shaped fixture file (an object keyed by id, or an
 * object with an `items` array/object, or a bare array) and reports
 * per-type parity. This is how the audit's per-type tables were produced,
 * e.g. against tests/fixtures/test-items-library/chicago-18th.json.
 *
 * --list <label> drills from a ranked class down to its actual entries
 * (id, oracle text, citum text) so a fix can be written against real
 * examples instead of just a count. Added after the 2026-08-23 wave-1 pass
 * needed exactly this and had to improvise it ad hoc — see
 * docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md.
 */

'use strict';

const fs = require('fs');
const path = require('path');

// ─── Label rules ──────────────────────────────────────────────────────────
// Each rule receives (oracle, citum, diffOps) and returns true if the row
// carries that label. diffOps is an array of { tag, a, b } non-equal
// SequenceMatcher-style opcodes (see diffOps() below).

const STOP_WORDS = new Set([
  'a', 'an', 'the', 'and', 'but', 'or', 'nor', 'for', 'yet', 'so',
  'at', 'by', 'in', 'of', 'on', 'to', 'up', 'via', 'with', 'from',
  'into', 'onto', 'over', 'as',
]);

function wordAt(s, i) {
  let a = i;
  while (a > 0 && /[A-Za-z’'-]/.test(s[a - 1])) a -= 1;
  let b = i;
  while (b < s.length && /[A-Za-z’'-]/.test(s[b])) b += 1;
  return s.slice(a, b);
}

const LABEL_RULES = [
  [
    'A1 title-case not applied',
    (o, c, ops) =>
      ops.some(({ tag, a, b, oi, ci }) => {
        if (tag !== 'replace' || a === b || a.toLowerCase() !== b.toLowerCase()) return false;
        const wo = wordAt(o, oi);
        const wc = wordAt(c, ci);
        if (wo.toUpperCase() === wo && wo.length > 1) return false; // acronym, not case-only
        if (wc.toUpperCase() === wc && wc.length > 1) return false;
        return /^[A-Z]/.test(wo) && /^[a-z]/.test(wc);
      }),
  ],
  [
    'A2 title-case over-applied / stop-word',
    (o, c, ops) =>
      ops.some(({ tag, a, b, oi, ci }) => {
        if (tag !== 'replace' || a === b || a.toLowerCase() !== b.toLowerCase()) return false;
        const wo = wordAt(o, oi);
        const wc = wordAt(c, ci);
        return /^[a-z]/.test(wo) && /^[A-Z]/.test(wc);
      }),
  ],
  [
    // Internal-capitalization case flips (PhD -> Phd, NIPS -> Nips): the
    // word's *first* letter case agrees between oracle and citum but an
    // internal letter's case differs. Distinct from A1/A2, which are
    // whole-word leading-capitalization flips (title case vs. sentence
    // case) — see docs/policies/TEXT_CASE_PROTECTION.md, the recorded
    // policy this class must not be fixed by reverting.
    'A3 acronym/mixed-case',
    (o, c, ops) =>
      ops.some(({ tag, a, b, oi, ci }) => {
        if (tag !== 'replace' || a === b || a.toLowerCase() !== b.toLowerCase()) return false;
        const wo = wordAt(o, oi);
        const wc = wordAt(c, ci);
        return wo.length > 1 && wc.length > 1 && wo[0] === wc[0] && wo !== wc;
      }),
  ],
  ['B title quote boundary', (o, c, ops) => ops.some(({ a, b }) => /[“”]/.test(a + b))],
  [
    'C year-suffix letter',
    (o, c, ops) =>
      ops.some(({ a, b }) => {
        const at = a.trim();
        const bt = b.trim();
        const single = (x) => /^[a-z]$/.test(x);
        return (single(at) && bt === '') || (single(bt) && at === '') || (single(at) && single(bt) && at !== bt);
      }),
  ],
  [
    'D date detail (month/day)',
    (o, c, ops) =>
      ops.some(({ a, b }) =>
        /\b(january|february|march|april|may|june|july|august|september|october|november|december)\b/i.test(
          a + b
        )
      ),
  ],
  [
    'E contributor role & ordering',
    (o, c, ops) =>
      ops.some(({ a, b }) => /\b(edited|translated|compiled|illustrated|narrated|directed|performed) by\b|\beds?\.\b|\btrans\.\b|\bcomp\.\b/i.test(a + b)),
  ],
  [
    'F genre/medium label',
    (o, c, ops) =>
      ops.some(({ tag, a }) =>
        tag === 'delete' &&
        /\b(thesis|diss|kindle|epub|pdf|dvd|blu-ray|microfilm|typescript|film|audio|video|map|memorandum|apparatus|model|manuscript|holograph|scroll|report)\b/i.test(
          a
        )
      ),
  ],
  ['G accessed-date policy', (o, c, ops) => ops.some(({ a, b }) => /Accessed/.test(a + b))],
  [
    'H volume/issue/series grammar',
    (o, c, ops) => ops.some(({ a, b }) => /\b(no\.|special issue|supplement|pt\.|ser\.|vol\.|vols)/i.test(a + b)),
  ],
  [
    'I legal/statute grammar',
    (o, c, ops) => ops.some(({ a, b }) => /§|U\.S\.C|Stat\.|Cong\.|C\.F\.R|Reports/.test(a + b)),
  ],
  ['J URL/DOI policy', (o, c, ops) => ops.some(({ a, b }) => /http/.test(a + b))],
  [
    'K editorial phrase / status',
    (o, c, ops) =>
      ops.some(({ a, b }) =>
        /review of|foreword to|introduction to|preface to|afterword|originally published|in press|forthcoming|accepted/i.test(
          a + b
        )
      ),
  ],
  [
    'L in-container / works-within-works',
    (o, c, ops) => ops.some(({ a, b }) => ['in', 'in,', 'in.'].includes((a + b).trim().toLowerCase()) && (a + b).trim() !== ''),
  ],
  [
    'M edition placement',
    (o, c, ops) =>
      ops.some(({ a, b }) => /\b\d(st|nd|rd|th) ed\.|annotated ed|anniversary edition|\bed\.$/i.test(a + b)),
  ],
  [
    'N punctuation/delimiter only',
    (o, c, ops) =>
      ops.some(({ a, b }) => {
        const seg = (a + b).trim();
        return seg !== '' && /^[.,;:()[\]—–"'\- ]+$/.test(seg);
      }),
  ],
];

// ─── Diff ──────────────────────────────────────────────────────────────────
// Minimal LCS-based SequenceMatcher-equivalent opcode extraction: good
// enough for citation-length strings and dependency-free.

function diffOps(o, c) {
  const n = o.length;
  const m = c.length;
  // Myers-ish DP is O(n*m); citation strings are short (<400 chars), fine.
  const dp = Array.from({ length: n + 1 }, () => new Int32Array(m + 1));
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      dp[i][j] = o[i] === c[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const ops = [];
  let i = 0;
  let j = 0;
  let eqStart = null;
  const flushReplace = (oi0, oi1, ci0, ci1) => {
    if (oi1 > oi0 || ci1 > ci0) {
      ops.push({
        tag: oi1 > oi0 && ci1 > ci0 ? 'replace' : oi1 > oi0 ? 'delete' : 'insert',
        a: o.slice(oi0, oi1),
        b: c.slice(ci0, ci1),
        oi: oi0,
        ci: ci0,
      });
    }
  };
  let pendO0 = 0;
  let pendC0 = 0;
  while (i < n && j < m) {
    if (o[i] === c[j]) {
      flushReplace(pendO0, i, pendC0, j);
      i += 1;
      j += 1;
      pendO0 = i;
      pendC0 = j;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      i += 1;
    } else {
      j += 1;
    }
  }
  flushReplace(pendO0, n, pendC0, m);
  return ops;
}

function labelsFor(o, c) {
  const ops = diffOps(o || '', c || '');
  const found = [];
  for (const [name, fn] of LABEL_RULES) {
    if (fn(o || '', c || '', ops)) found.push(name);
  }
  if (found.length === 0) found.push('Z unclassified');
  return found;
}

// ─── Row extraction ─────────────────────────────────────────────────────────

function failingRows(styleReport) {
  const rows = [];
  for (const e of styleReport.oracleDetail || []) {
    if (e.exactParityEligible && !e.exactMatch) {
      rows.push({ kind: 'bib', id: e.id, oracle: e.exactOracle, citum: e.exactCitum });
    }
  }
  for (const e of styleReport.citationEntries || []) {
    if (!e.exactMatch) {
      rows.push({ kind: 'cite', id: e.id, oracle: e.exactOracle, citum: e.exactCitum });
    }
  }
  return rows;
}

/**
 * Every row (pass and fail) from one style report, with the same source
 * arrays and gating as `failingRows` (`oracleDetail` gated on
 * `exactParityEligible`, `citationEntries` ungated) but keeping
 * `exactMatch` on each row instead of filtering to failures. `failingRows`
 * alone can't see newly-passing rows, so a diff needs this separately.
 */
function allRows(styleReport) {
  const rows = [];
  for (const e of styleReport.oracleDetail || []) {
    if (e.exactParityEligible) {
      rows.push({ kind: 'bib', id: e.id, oracle: e.exactOracle, citum: e.exactCitum, exactMatch: e.exactMatch });
    }
  }
  for (const e of styleReport.citationEntries || []) {
    rows.push({ kind: 'cite', id: e.id, oracle: e.exactOracle, citum: e.exactCitum, exactMatch: e.exactMatch });
  }
  return rows;
}

function rowKey(row) {
  return `${row.kind} ${row.id}`;
}

/**
 * Per-row exactMatch map for one style report, keyed by `(kind, id)` (see
 * `listLabel`'s note below: a report can carry the same id twice across
 * merged benchmark runs). Duplicates fold with AND -- any failing instance
 * marks the key failing. Conservative default for regression detection: a
 * row that fails in even one duplicate should not be reported as cleanly
 * passing.
 */
function exactMatchMap(styleReport) {
  const map = new Map();
  for (const row of allRows(styleReport)) {
    const key = rowKey(row);
    const existing = map.get(key);
    map.set(key, existing === undefined ? row.exactMatch : existing && row.exactMatch);
  }
  return map;
}

function loadTypeMap(fixturePath) {
  const raw = JSON.parse(fs.readFileSync(fixturePath, 'utf8'));
  const items = Array.isArray(raw) ? raw : raw.items || raw;
  const map = new Map();
  const entries = Array.isArray(items) ? items.map((v) => [v.id, v]) : Object.entries(items);
  for (const [key, v] of entries) {
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      map.set(v.id || key, v.type || v.class || null);
    }
  }
  return map;
}

function greedySetCover(rowLabels) {
  const counts = new Map();
  for (const labels of rowLabels) {
    for (const l of labels) counts.set(l, (counts.get(l) || 0) + 1);
  }
  const chosen = [];
  const chosenSet = new Set();
  const steps = [];
  const total = rowLabels.length;
  for (let step = 0; step < counts.size; step += 1) {
    let best = null;
    let bestN = 0;
    for (const label of counts.keys()) {
      if (chosenSet.has(label) || label === 'Z unclassified') continue;
      const candidate = new Set(chosenSet);
      candidate.add(label);
      const n = rowLabels.filter((labels) => labels.length > 0 && labels.every((l) => candidate.has(l))).length;
      if (n > bestN) {
        bestN = n;
        best = label;
      }
    }
    if (!best) break;
    chosen.push(best);
    chosenSet.add(best);
    steps.push({ label: best, cumulativeFlipped: bestN, cumulativePct: total ? bestN / total : 0 });
  }
  return steps;
}

function analyzeStyle(styleReport, typeMap) {
  const rows = failingRows(styleReport);
  const rowLabels = rows.map((r) => labelsFor(r.oracle, r.citum));

  const perLabel = new Map();
  const soleCause = new Map();
  rowLabels.forEach((labels) => {
    for (const l of labels) perLabel.set(l, (perLabel.get(l) || 0) + 1);
    if (labels.length === 1) soleCause.set(labels[0], (soleCause.get(labels[0]) || 0) + 1);
  });

  const byType = typeMap
    ? (() => {
        const tot = new Map();
        const ok = new Map();
        for (const e of styleReport.oracleDetail || []) {
          if (!e.exactParityEligible) continue;
          const t = typeMap.get(e.id) || 'UNKNOWN';
          tot.set(t, (tot.get(t) || 0) + 1);
          if (e.exactMatch) ok.set(t, (ok.get(t) || 0) + 1);
        }
        return [...tot.entries()]
          .map(([type, total]) => ({ type, passed: ok.get(type) || 0, total }))
          .sort((a, b) => b.total - a.total);
      })()
    : null;

  return {
    name: styleReport.name,
    exactParity: styleReport.exactParity,
    fidelityScore: styleReport.fidelityScore,
    failingRows: rows.length,
    labelCounts: [...perLabel.entries()]
      .map(([label, rowsWithLabel]) => ({ label, rows: rowsWithLabel, soleCause: soleCause.get(label) || 0 }))
      .sort((a, b) => b.rows - a.rows),
    setCover: greedySetCover(rowLabels),
    byType,
  };
}

// ─── Diff (wave before/after comparison) ───────────────────────────────────
//
// Rows in this corpus commonly carry 2-4 overlapping defect labels and
// don't flip to exactMatch until every one of them clears. That means a
// leverage-ordered wave can do real, measurable work -- clearing labels off
// rows -- while the aggregate `passed` count stays flat, because the rows
// it touched still have other labels outstanding. The functions below
// surface that underlying signal directly instead of requiring it to be
// reconstructed by hand (as the 2026-08-23 audit's wave 1/2 postscript was),
// and give the `newlyFailing` regression check that
// docs/guides/STYLE_WORKFLOW_EXECUTION.md's exact-parity loop (step 3c)
// already mandates in prose but has never had a named tool for.

/**
 * Diff two style reports' per-row exactMatch state. `newlyFailing` is the
 * regression signal: a fix routed through a shared category (e.g. a
 * `titles.type-mapping` entry) can flip several previously-passing rows to
 * failing while flipping more failing rows to passing, and a rising
 * aggregate `passed` count alone cannot distinguish that from a clean win.
 */
function diffExactMatch(beforeStyle, afterStyle) {
  const beforeMap = exactMatchMap(beforeStyle);
  const afterMap = exactMatchMap(afterStyle);
  const afterRowsByKey = new Map(allRows(afterStyle).map((r) => [rowKey(r), r]));

  const newlyPassing = [];
  const newlyFailing = [];
  for (const [key, wasPassing] of beforeMap) {
    if (!afterMap.has(key)) continue;
    const isPassing = afterMap.get(key);
    if (wasPassing === isPassing) continue;
    const [kind, id] = key.split(' ');
    if (isPassing) {
      newlyPassing.push({ kind, id });
    } else {
      const row = afterRowsByKey.get(key);
      newlyFailing.push({ kind, id, oracle: row ? row.oracle : undefined, citum: row ? row.citum : undefined });
    }
  }

  return {
    newlyPassing,
    newlyFailing,
    before: { passed: beforeStyle.exactParity.passed, total: beforeStyle.exactParity.total },
    after: { passed: afterStyle.exactParity.passed, total: afterStyle.exactParity.total },
  };
}

/**
 * Diff two `analyzeStyle(...).labelCounts` arrays. Pure diff of already-
 * computed output -- no new labeling logic. Sorted by |delta| descending
 * (biggest moves first), label name ascending as a tiebreaker.
 */
function diffLabelCounts(beforeAnalysis, afterAnalysis) {
  const beforeMap = new Map(beforeAnalysis.labelCounts.map((l) => [l.label, l.rows]));
  const afterMap = new Map(afterAnalysis.labelCounts.map((l) => [l.label, l.rows]));
  const labels = new Set([...beforeMap.keys(), ...afterMap.keys()]);
  const result = [...labels].map((label) => {
    const before = beforeMap.get(label) || 0;
    const after = afterMap.get(label) || 0;
    return { label, before, after, delta: after - before };
  });
  result.sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta) || a.label.localeCompare(b.label));
  return result;
}

/** Sum `rows` per label across several `analyzeStyle(...).labelCounts` arrays. */
function mergeLabelCounts(analyses) {
  const totals = new Map();
  for (const a of analyses) {
    for (const { label, rows } of a.labelCounts) {
      totals.set(label, (totals.get(label) || 0) + rows);
    }
  }
  return [...totals.entries()].map(([label, rows]) => ({ label, rows }));
}

/**
 * Bucket failing rows by how many defect labels they carry. A row with N
 * labels needs all N cleared before it flips to exactMatch -- this
 * histogram is the leading indicator that a wave is converging the
 * residual population even when the aggregate `passed` count hasn't moved:
 * rows migrate from higher buckets to lower ones before any of them reach
 * zero.
 */
function labelCountHistogram(styleReport) {
  const histogram = {};
  for (const row of failingRows(styleReport)) {
    const n = labelsFor(row.oracle, row.citum).length;
    histogram[n] = (histogram[n] || 0) + 1;
  }
  return histogram;
}

/**
 * Rows carrying exactly one remaining defect label -- the row-level
 * complement to `analyzeStyle`'s `soleCause` counts. This is the next
 * wave's ready-to-convert target list: fixing that one label flips the row
 * to exactMatch.
 */
function nearMissQueue(styleReport) {
  const seen = new Set();
  const out = [];
  for (const row of failingRows(styleReport)) {
    const key = rowKey(row);
    if (seen.has(key)) continue;
    const labels = labelsFor(row.oracle, row.citum);
    if (labels.length !== 1) continue;
    seen.add(key);
    out.push({ kind: row.kind, id: row.id, label: labels[0], oracle: row.oracle, citum: row.citum });
  }
  out.sort((a, b) => a.label.localeCompare(b.label) || a.id.localeCompare(b.id));
  return out;
}

function diffStyles(beforeStyle, afterStyle, beforeAnalysis, afterAnalysis) {
  const ba = beforeAnalysis || analyzeStyle(beforeStyle);
  const aa = afterAnalysis || analyzeStyle(afterStyle);
  return {
    name: afterStyle.name,
    exactMatch: diffExactMatch(beforeStyle, afterStyle),
    labelDeltas: diffLabelCounts(ba, aa),
    histogramBefore: labelCountHistogram(beforeStyle),
    histogramAfter: labelCountHistogram(afterStyle),
    nearMiss: nearMissQueue(afterStyle),
  };
}

function sumHistograms(histograms) {
  const out = {};
  for (const h of histograms) {
    for (const [k, v] of Object.entries(h)) out[k] = (out[k] || 0) + v;
  }
  return out;
}

/**
 * Diff two full reports (as produced by `report-core.js --all-features`,
 * typically without `--style` so multiple styles are present -- that is
 * the normal input shape, not an edge case, since a cross-style regression
 * check needs it). Styles present in only one report are listed under
 * `addedStyles`/`removedStyles` rather than silently dropped from the diff.
 */
function diffReports(beforeReport, afterReport) {
  const beforeByName = new Map((beforeReport.styles || []).map((s) => [s.name, s]));
  const afterByName = new Map((afterReport.styles || []).map((s) => [s.name, s]));
  const common = [...afterByName.keys()].filter((name) => beforeByName.has(name));

  const beforeAnalyses = common.map((name) => analyzeStyle(beforeByName.get(name)));
  const afterAnalyses = common.map((name) => analyzeStyle(afterByName.get(name)));

  const styles = common.map((name, i) =>
    diffStyles(beforeByName.get(name), afterByName.get(name), beforeAnalyses[i], afterAnalyses[i])
  );

  const aggregate = {
    exactMatch: {
      newlyPassing: styles.flatMap((s) => s.exactMatch.newlyPassing.map((r) => ({ style: s.name, ...r }))),
      newlyFailing: styles.flatMap((s) => s.exactMatch.newlyFailing.map((r) => ({ style: s.name, ...r }))),
      before: {
        passed: styles.reduce((n, s) => n + s.exactMatch.before.passed, 0),
        total: styles.reduce((n, s) => n + s.exactMatch.before.total, 0),
      },
      after: {
        passed: styles.reduce((n, s) => n + s.exactMatch.after.passed, 0),
        total: styles.reduce((n, s) => n + s.exactMatch.after.total, 0),
      },
    },
    labelDeltas: diffLabelCounts({ labelCounts: mergeLabelCounts(beforeAnalyses) }, { labelCounts: mergeLabelCounts(afterAnalyses) }),
    histogramBefore: sumHistograms(styles.map((s) => s.histogramBefore)),
    histogramAfter: sumHistograms(styles.map((s) => s.histogramAfter)),
    nearMiss: styles.flatMap((s) => s.nearMiss.map((r) => ({ style: s.name, ...r }))),
  };

  return {
    styles,
    aggregate,
    addedStyles: [...afterByName.keys()].filter((name) => !beforeByName.has(name)),
    removedStyles: [...beforeByName.keys()].filter((name) => !afterByName.has(name)),
  };
}

// ─── CLI ─────────────────────────────────────────────────────────────────

function parseArgs(argv) {
  const opts = { json: false, byType: null, input: null, list: null, diff: null };
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === '--json') opts.json = true;
    else if (argv[i] === '--by-type') opts.byType = path.resolve(argv[(i += 1)]);
    else if (argv[i] === '--list') opts.list = argv[(i += 1)];
    else if (argv[i] === '--diff') opts.diff = path.resolve(argv[(i += 1)]);
    else if (!opts.input) opts.input = argv[i];
  }
  return opts;
}

/**
 * List the individual failing rows carrying `label` for one style, deduped
 * by id (a report can carry the same id twice across merged benchmark
 * runs). This is the drill-down the aggregate label counts can't provide:
 * once a class has been ranked, fixing it means looking at its actual
 * entries, not just its count.
 */
function listLabel(styleReport, label) {
  const rows = failingRows(styleReport);
  const seen = new Set();
  const out = [];
  for (const r of rows) {
    if (seen.has(r.id)) continue;
    const labels = labelsFor(r.oracle, r.citum);
    if (!labels.includes(label)) continue;
    seen.add(r.id);
    out.push({ ...r, labels });
  }
  return out;
}

function printHuman(result) {
  const { name, exactParity, fidelityScore, failingRows: n, labelCounts, setCover, byType } = result;
  console.log(
    `\n### ${name}: ${exactParity.passed}/${exactParity.total} exact parity ` +
      `(fidelity ${fidelityScore}), ${n} failing rows`
  );
  console.log(`${'class'.padEnd(46)}${'rows'.padStart(7)}${'%'.padStart(7)}${'sole cause'.padStart(12)}`);
  for (const { label, rows, soleCause } of labelCounts) {
    const pct = n ? ((rows / n) * 100).toFixed(1) : '0.0';
    console.log(`  ${label.padEnd(44)}${String(rows).padStart(7)}${pct.padStart(6)}%${String(soleCause).padStart(12)}`);
  }
  console.log('\n  greedy set cover (fix in this order for max leverage):');
  for (const [i, step] of setCover.entries()) {
    console.log(
      `    ${i + 1}. + ${step.label.padEnd(44)} cumulative flipped: ${step.cumulativeFlipped} ` +
        `(${(step.cumulativePct * 100).toFixed(1)}%)`
    );
  }
  if (byType) {
    console.log('\n  per reference type:');
    for (const { type, passed, total } of byType) {
      const pct = total ? ((passed / total) * 100).toFixed(1) : '0.0';
      console.log(`    ${type.padEnd(32)}${String(passed).padStart(4)}/${String(total).padEnd(4)}  ${pct}%`);
    }
  }
}

/** Print one diff section (a per-style diff, or the aggregate) in the shared column style. */
function printDiffSection(label, diff) {
  const { exactMatch, labelDeltas, histogramBefore, histogramAfter, nearMiss } = diff;
  console.log(`\n### ${label}`);
  console.log(
    `  exactMatch: ${exactMatch.before.passed}/${exactMatch.before.total} -> ` +
      `${exactMatch.after.passed}/${exactMatch.after.total} ` +
      `(+${exactMatch.newlyPassing.length} newly passing, ${exactMatch.newlyFailing.length} newly failing)`
  );
  if (exactMatch.newlyFailing.length) {
    console.log('  REGRESSIONS:');
    for (const r of exactMatch.newlyFailing) {
      const where = r.style ? `${r.style} ` : '';
      console.log(`    [${where}${r.kind} ${r.id}]`);
      console.log(`      O: ${r.oracle}`);
      console.log(`      C: ${r.citum}`);
    }
  }
  const nonzeroDeltas = labelDeltas.filter((d) => d.delta !== 0);
  if (nonzeroDeltas.length) {
    console.log('  label-instance deltas (nonzero only):');
    for (const { label: l, before, after, delta } of nonzeroDeltas) {
      const sign = delta > 0 ? '+' : '';
      console.log(`    ${l.padEnd(44)} ${String(before).padStart(4)} -> ${String(after).padStart(4)} (${sign}${delta})`);
    }
  }
  const keys = new Set([...Object.keys(histogramBefore), ...Object.keys(histogramAfter)]);
  if (keys.size) {
    console.log('  rows-by-label-count histogram:');
    console.log(`    ${'n-labels'.padEnd(10)}${'before'.padEnd(8)}after`);
    for (const k of [...keys].sort((a, b) => Number(a) - Number(b))) {
      console.log(`    ${k.padEnd(10)}${String(histogramBefore[k] || 0).padEnd(8)}${histogramAfter[k] || 0}`);
    }
  }
  console.log(`  near-miss queue (rows 1 label from passing): ${nearMiss.length}`);
}

function printDiffHuman(result) {
  for (const style of result.styles) {
    printDiffSection(style.name, style);
  }
  printDiffSection('ALL STYLES (aggregate)', result.aggregate);
  if (result.addedStyles.length) {
    console.log(`\n(styles only in the after report, not diffed: ${result.addedStyles.join(', ')})`);
  }
  if (result.removedStyles.length) {
    console.log(`\n(styles only in the before report, not diffed: ${result.removedStyles.join(', ')})`);
  }
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (!opts.input) {
    console.error(
      'usage: analyze-parity-residuals.js <report.json> [--json] [--by-type <refs.json>] ' +
        '[--list "<label>"] [--diff <before-report.json>]'
    );
    return 2;
  }
  const report = JSON.parse(fs.readFileSync(opts.input, 'utf8'));
  const styles = report.styles || [];

  if (opts.diff) {
    const beforeReport = JSON.parse(fs.readFileSync(opts.diff, 'utf8'));
    const result = diffReports(beforeReport, report);
    if (opts.json) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      printDiffHuman(result);
    }
    return 0;
  }

  if (opts.list) {
    for (const style of styles) {
      const entries = listLabel(style, opts.list);
      if (opts.json) {
        console.log(JSON.stringify({ name: style.name, label: opts.list, entries }, null, 2));
        continue;
      }
      console.log(`\n### ${style.name}: ${entries.length} entries carrying "${opts.list}"`);
      for (const e of entries) {
        console.log(`[${e.kind} ${e.id}] labels: ${e.labels.join(', ')}`);
        console.log(`  O: ${e.oracle}`);
        console.log(`  C: ${e.citum}`);
      }
    }
    return 0;
  }

  const typeMap = opts.byType ? loadTypeMap(opts.byType) : null;
  const results = styles.map((s) => analyzeStyle(s, typeMap));
  if (opts.json) {
    console.log(JSON.stringify(results, null, 2));
  } else {
    for (const r of results) printHuman(r);
  }
  return 0;
}

if (require.main === module) {
  process.exitCode = main();
}

module.exports = {
  diffOps,
  labelsFor,
  failingRows,
  greedySetCover,
  analyzeStyle,
  loadTypeMap,
  listLabel,
  LABEL_RULES,
  allRows,
  rowKey,
  exactMatchMap,
  diffExactMatch,
  diffLabelCounts,
  mergeLabelCounts,
  labelCountHistogram,
  nearMissQueue,
  diffStyles,
  diffReports,
};
