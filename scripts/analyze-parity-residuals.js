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

// ─── CLI ─────────────────────────────────────────────────────────────────

function parseArgs(argv) {
  const opts = { json: false, byType: null, input: null, list: null };
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === '--json') opts.json = true;
    else if (argv[i] === '--by-type') opts.byType = path.resolve(argv[(i += 1)]);
    else if (argv[i] === '--list') opts.list = argv[(i += 1)];
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

function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (!opts.input) {
    console.error(
      'usage: analyze-parity-residuals.js <report.json> [--json] [--by-type <refs.json>] ' +
        '[--list "<label>"]'
    );
    return 2;
  }
  const report = JSON.parse(fs.readFileSync(opts.input, 'utf8'));
  const styles = report.styles || [];

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
};
