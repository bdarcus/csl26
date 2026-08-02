#!/usr/bin/env node
/**
 * scripts/oracle-snapshot.js
 *
 * Pre-computes citeproc-js rendered output for CSL styles and stores the
 * results as static JSON snapshots. The diff oracle (oracle-fast.js) loads
 * these instead of re-running citeproc-js on every report run.
 *
 * Usage:
 *   node scripts/oracle-snapshot.js <style.csl>          # single style
 *   node scripts/oracle-snapshot.js --all                # all styles-legacy/*.csl
 *   node scripts/oracle-snapshot.js --all --force        # regenerate even if current
 *   node scripts/oracle-snapshot.js --all --concurrency 8
 *
 * Snapshot location: tests/snapshots/csl/<stylename>.json
 *
 * Each snapshot is keyed on fixture_hash + csl_hash. If either changes the
 * snapshot is considered stale and oracle-fast.js will refuse to use it.
 * Version 2 adds bibliography_ids aligned by index with bibliography rows;
 * version 1 remains readable through oracle-fast.js's similarity fallback.
 *
 * Exit codes:
 *   0 — all snapshots written (or already current)
 *   1 — one or more styles failed to render
 */

'use strict';

const CSL = require('citeproc');
const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { loadLocale } = require('./oracle-utils');
const { bibliographyIdsForRenderedRows } = require('./lib/citeproc-bibliography');
const { toCiteprocItem } = require('./lib/citeproc-locators');

const CITEPROC_VERSION = '2.4.63';
const SNAPSHOT_VERSION = 2;
const PROJECT_ROOT = path.resolve(__dirname, '..');
const SNAPSHOT_DIR = path.join(PROJECT_ROOT, 'tests', 'snapshots', 'csl');
const DEFAULT_REFS_FIXTURE = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'references-expanded.json');
const DEFAULT_CITATIONS_FIXTURE = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'citations-expanded.json');
const NOTE_CITATIONS_FIXTURE = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'citations-note-expanded.json');
const STYLES_LEGACY_DIR = path.join(PROJECT_ROOT, 'styles-legacy');

/** Return true if the CSL file is a note-class style (class="note" on <style>). */
function isNoteStyle(stylePath) {
  try {
    const xml = fs.readFileSync(stylePath, 'utf8');
    return /<style[^>]+class=["']note["']/.test(xml);
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {
    stylePath: null,
    all: false,
    force: false,
    concurrency: os.cpus().length,
    refsFixture: DEFAULT_REFS_FIXTURE,
    citationsFixture: DEFAULT_CITATIONS_FIXTURE,
  };

  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === '--all') {
      opts.all = true;
    } else if (a === '--force') {
      opts.force = true;
    } else if (a === '--concurrency') {
      opts.concurrency = parseInt(args[++i], 10);
    } else if (a === '--refs-fixture') {
      opts.refsFixture = path.resolve(args[++i]);
    } else if (a === '--citations-fixture') {
      opts.citationsFixture = path.resolve(args[++i]);
    } else if (!a.startsWith('--')) {
      opts.stylePath = path.resolve(a);
    }
  }

  return opts;
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/** SHA-256 of file contents, hex, first 16 chars. */
function fileHash(filePath) {
  const content = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(content).digest('hex').slice(0, 16);
}

/** Combined hash for both fixture files. */
function fixtureHash(refsFixture, citationsFixture) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(refsFixture));
  h.update(fs.readFileSync(citationsFixture));
  return h.digest('hex').slice(0, 16);
}

// ---------------------------------------------------------------------------
// Snapshot I/O
// ---------------------------------------------------------------------------

/** Return the snapshot path for a given CSL style path. */
function snapshotPath(stylePath) {
  const name = path.basename(stylePath, '.csl');
  return path.join(SNAPSHOT_DIR, `${name}.json`);
}

/**
 * Return true if an up-to-date snapshot already exists.
 * "Up-to-date" means fixture_hash and csl_hash both match.
 */
function isSnapshotCurrent(snapPath, fxHash, cslHash) {
  if (!fs.existsSync(snapPath)) return false;
  try {
    const snap = JSON.parse(fs.readFileSync(snapPath, 'utf8'));
    return snap.version === SNAPSHOT_VERSION &&
      snap.fixture_hash === fxHash &&
      snap.csl_hash === cslHash &&
      Array.isArray(snap.bibliography_ids) &&
      snap.bibliography_ids.length === snap.bibliography.length &&
      snap.bibliography_ids.every((id) => id !== null && id !== undefined && id !== '') &&
      new Set(snap.bibliography_ids).size === snap.bibliography_ids.length;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// citeproc-js rendering
// ---------------------------------------------------------------------------

function loadFixtures(refsFixture, citationsFixture) {
  const refs = JSON.parse(fs.readFileSync(refsFixture, 'utf8'));
  const testItems = Object.fromEntries(
    Object.entries(refs).filter(([k]) => k !== 'comment')
  );
  const testCitations = JSON.parse(fs.readFileSync(citationsFixture, 'utf8'));
  return { testItems, testCitations };
}

/**
 * Render citations and bibliography for one CSL style using citeproc-js.
 * Returns bibliography text and its source item IDs alongside citation text.
 */
function renderWithCiteprocJs(stylePath, testItems, testCitations) {
  const styleXml = fs.readFileSync(stylePath, 'utf8');

  const sys = {
    retrieveLocale: (lang) => loadLocale(lang),
    retrieveItem: (id) => testItems[id],
  };

  const engine = new CSL.Engine(sys, styleXml);
  engine.updateItems(Object.keys(testItems));

  const citations = {};
  for (const cite of testCitations) {
    const suppressAuthor = cite['suppress-author'] === true;
    const items = cite.items.map((item) => toCiteprocItem(item, suppressAuthor));
    try {
      citations[cite.id] = engine.makeCitationCluster(items);
    } catch (err) {
      citations[cite.id] = `ERROR: ${err.message}`;
    }
  }

  const bibResult = engine.makeBibliography();
  const bibliography = bibResult ? bibResult[1] : [];
  const bibliographyIds = bibliographyIdsForRenderedRows(bibResult?.[0], bibliography);

  return { citations, bibliography, bibliographyIds };
}

// ---------------------------------------------------------------------------
// Single-style snapshot generation
// ---------------------------------------------------------------------------

function buildSnapshot(styleName, fxHash, cslHash, rendered) {
  return {
    version: SNAPSHOT_VERSION,
    generated_by: `citeproc-js@${CITEPROC_VERSION}`,
    generated_at: new Date().toISOString(),
    style: styleName,
    fixture_hash: fxHash,
    csl_hash: cslHash,
    citations: rendered.citations,
    bibliography: rendered.bibliography,
    bibliography_ids: rendered.bibliographyIds,
  };
}

/**
 * Generate (or skip if current) the snapshot for one CSL style.
 * Returns 'written' | 'skipped' | 'error:<message>'.
 */
function generateSnapshot(stylePath, { testItems, testCitations }, fxHash, opts) {
  const cslHash = fileHash(stylePath);
  const snapPath = snapshotPath(stylePath);
  const styleName = path.basename(stylePath, '.csl');

  if (!opts.force && isSnapshotCurrent(snapPath, fxHash, cslHash)) {
    return 'skipped';
  }

  let rendered;
  try {
    rendered = renderWithCiteprocJs(stylePath, testItems, testCitations);
  } catch (err) {
    return `error:${err.message}`;
  }

  const snapshot = buildSnapshot(styleName, fxHash, cslHash, rendered);

  fs.mkdirSync(path.dirname(snapPath), { recursive: true });
  fs.writeFileSync(snapPath, JSON.stringify(snapshot, null, 2) + '\n', 'utf8');
  return 'written';
}

// ---------------------------------------------------------------------------
// Batch: process all CSL styles with a concurrency limit
// ---------------------------------------------------------------------------

async function generateAll(opts) {
  if (!fs.existsSync(STYLES_LEGACY_DIR)) {
    process.stderr.write(
      `styles-legacy/ not found at ${STYLES_LEGACY_DIR}\n` +
      'Run: git submodule update --init --depth=1\n'
    );
    process.exit(1);
  }

  const allStyles = fs.readdirSync(STYLES_LEGACY_DIR)
    .filter((f) => f.endsWith('.csl'))
    .map((f) => path.join(STYLES_LEGACY_DIR, f));

  // Pre-compute fixtures for both style classes
  const stdFixtures = loadFixtures(opts.refsFixture, opts.citationsFixture);
  const stdHash = fixtureHash(opts.refsFixture, opts.citationsFixture);
  const noteFixtures = loadFixtures(opts.refsFixture, NOTE_CITATIONS_FIXTURE);
  const noteHash = fixtureHash(opts.refsFixture, NOTE_CITATIONS_FIXTURE);

  let written = 0;
  let skipped = 0;
  let failed = 0;
  let processed = 0;
  const total = allStyles.length;
  const concurrency = Math.max(1, opts.concurrency);

  // Process in chunks (citeproc-js is sync; chunking gives us progress logs)
  for (let i = 0; i < allStyles.length; i += concurrency) {
    const chunk = allStyles.slice(i, i + concurrency);
    for (const stylePath of chunk) {
      // Auto-select note fixture unless caller explicitly overrode --citations-fixture
      const useNote = opts.citationsFixture === DEFAULT_CITATIONS_FIXTURE && isNoteStyle(stylePath);
      const fixtures = useNote ? noteFixtures : stdFixtures;
      const fxHash = useNote ? noteHash : stdHash;
      const result = generateSnapshot(stylePath, fixtures, fxHash, opts);
      processed++;
      if (result === 'written') {
        written++;
        process.stderr.write(`[${processed}/${total}] ✓ ${path.basename(stylePath)}\n`);
      } else if (result === 'skipped') {
        skipped++;
      } else {
        failed++;
        process.stderr.write(
          `[${processed}/${total}] ✗ ${path.basename(stylePath)}: ${result.slice(6)}\n`
        );
      }
    }
  }

  process.stderr.write(
    `\nDone. Written: ${written}, Skipped: ${skipped}, Failed: ${failed} (of ${total})\n`
  );
  return failed === 0 ? 0 : 1;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const opts = parseArgs();

  if (!opts.all && !opts.stylePath) {
    process.stderr.write(
      'Usage:\n' +
      '  oracle-snapshot.js <style.csl>\n' +
      '  oracle-snapshot.js --all [--force] [--concurrency N]\n'
    );
    process.exit(1);
  }

  if (opts.all) {
    process.exit(await generateAll(opts));
  }

  // Single style — auto-detect note format unless caller overrode --citations-fixture
  if (!fs.existsSync(opts.stylePath)) {
    process.stderr.write(`Style not found: ${opts.stylePath}\n`);
    process.exit(1);
  }

  if (opts.citationsFixture === DEFAULT_CITATIONS_FIXTURE && isNoteStyle(opts.stylePath)) {
    opts.citationsFixture = NOTE_CITATIONS_FIXTURE;
  }

  const fixtures = loadFixtures(opts.refsFixture, opts.citationsFixture);
  const fxHash = fixtureHash(opts.refsFixture, opts.citationsFixture);

  const result = generateSnapshot(opts.stylePath, fixtures, fxHash, opts);
  const styleName = path.basename(opts.stylePath, '.csl');

  if (result === 'written') {
    process.stderr.write(`✓ Snapshot written: tests/snapshots/csl/${styleName}.json\n`);
    process.exit(0);
  } else if (result === 'skipped') {
    process.stderr.write(`— Already current: tests/snapshots/csl/${styleName}.json\n`);
    process.exit(0);
  } else {
    process.stderr.write(`✗ Failed: ${result.slice(6)}\n`);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch((err) => {
    process.stderr.write(`Fatal: ${err.message}\n`);
    process.exit(1);
  });
}

module.exports = {
  SNAPSHOT_VERSION,
  buildSnapshot,
  generateSnapshot,
  isSnapshotCurrent,
  renderWithCiteprocJs,
};
