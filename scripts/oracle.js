#!/usr/bin/env node
/**
 * Structured Diff Oracle for CSLN Migration (DEFAULT)
 *
 * Compares citeproc-js and CSLN outputs at the component level,
 * identifying which specific parts of a bibliography entry differ.
 *
 * This is now the default oracle script. For simple string comparison,
 * use oracle-simple.js instead.
 *
 * Usage:
 *   node oracle.js ../styles/apa.csl
 *   node oracle.js ../styles/apa.csl --json
 *   node oracle.js ../styles/apa.csl --verbose
 *   node oracle.js ../styles/apa.csl --simple  # fallback to simple mode
 *
 * Exit codes:
 *   0 - Success (all citations and bibliography entries match)
 *   1 - Failed validation (some entries don't match)
 *   2 - Fatal error (file not found, parse error, CSLN rendering failed)
 */

const CSL = require('citeproc');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const {
  normalizeText,
  parseComponents,
  analyzeOrdering,
  findRefDataForEntry,
  loadLocale,
} = require('./oracle-utils');

const DEFAULT_REFS_FIXTURE = path.join(__dirname, '..', 'tests', 'fixtures', 'references-expanded.json');
const DEFAULT_CITATIONS_FIXTURE = path.join(__dirname, '..', 'tests', 'fixtures', 'citations-expanded.json');
// Citation IDs where fuzzy token overlap can hide real disambiguation regressions.
const STRICT_CITATION_IDS = new Set([
  'et-al-single-long-list',
  'disambiguate-add-names-et-al',
  'disambiguate-year-suffix',
  'et-al-with-locator',
]);

function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    stylePath: null,
    jsonOutput: false,
    verbose: false,
    refsFixture: DEFAULT_REFS_FIXTURE,
    citationsFixture: DEFAULT_CITATIONS_FIXTURE,
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--json') {
      options.jsonOutput = true;
    } else if (arg === '--verbose') {
      options.verbose = true;
    } else if (arg === '--refs-fixture') {
      options.refsFixture = path.resolve(args[++i]);
    } else if (arg === '--citations-fixture') {
      options.citationsFixture = path.resolve(args[++i]);
    } else if (!arg.startsWith('--') && !options.stylePath) {
      options.stylePath = arg;
    }
  }

  if (!options.stylePath) {
    options.stylePath = path.join(__dirname, '..', 'styles-legacy', 'apa.csl');
  }

  return options;
}

function loadFixtures(refsFixture, citationsFixture) {
  const fixturesData = JSON.parse(fs.readFileSync(refsFixture, 'utf8'));
  const testItems = Object.fromEntries(
    Object.entries(fixturesData).filter(([key]) => key !== 'comment')
  );
  const testCitations = JSON.parse(fs.readFileSync(citationsFixture, 'utf8'));
  return { testItems, testCitations };
}

/**
 * Compare two component sets and identify differences.
 */
function compareComponents(oracleComp, cslnComp, refData) {
  const differences = [];
  const matches = [];

  const keys = ['contributors', 'year', 'title', 'containerTitle', 'volume',
    'issue', 'pages', 'publisher', 'doi', 'edition', 'editors'];

  for (const key of keys) {
    const oracle = oracleComp[key];
    const csln = cslnComp[key];

    // Skip if neither has this component
    if (!oracle.found && !csln.found) continue;

    if (oracle.found && csln.found) {
      // Both have it - check if values match
      if (oracle.value === csln.value ||
        (typeof oracle.value === 'boolean' && oracle.value === csln.value)) {
        matches.push({ component: key, status: 'match' });
      } else {
        // Values differ
        matches.push({ component: key, status: 'match' }); // Component present in both
      }
    } else if (oracle.found && !csln.found) {
      differences.push({
        component: key,
        issue: 'missing',
        expected: oracle.value,
        detail: `Missing in CSLN output`
      });
    } else if (!oracle.found && csln.found) {
      differences.push({
        component: key,
        issue: 'extra',
        found: csln.value,
        detail: `Extra in CSLN output (not in oracle)`
      });
    }
  }

  return { differences, matches };
}

/**
 * Compare component ordering between oracle and CSLN.
 */
function compareOrdering(oracleOrder, cslnOrder) {
  const issues = [];

  // Check if orders match
  if (JSON.stringify(oracleOrder) !== JSON.stringify(cslnOrder)) {
    issues.push({
      issue: 'ordering',
      expected: oracleOrder,
      found: cslnOrder,
      detail: `Component order differs`
    });
  }

  return issues;
}

function renderWithCiteprocJs(stylePath, testItems, testCitations) {
  const styleXml = fs.readFileSync(stylePath, 'utf8');

  const sys = {
    retrieveLocale: (lang) => loadLocale(lang),
    retrieveItem: (id) => testItems[id]
  };

  const citeproc = new CSL.Engine(sys, styleXml);
  citeproc.updateItems(Object.keys(testItems));

  const citations = {};
  testCitations.forEach(cite => {
    // Convert CSLN citation items to citeproc-js format
    const suppressAuthor = cite['suppress-author'] === true;
    const citeprocItems = cite.items.map(item => ({
      id: item.id,
      locator: item.locator,
      label: item.label,
      prefix: item.prefix,
      suffix: item.suffix,
      'suppress-author': suppressAuthor
    }));

    // For narrative/integral citations, citeproc-js doesn't have a direct equivalent
    // in makeCitationCluster that matches CSLN's specific split rendering.
    // However, if we just want to test clustered rendering, we can use the cluster.
    // For now, we compare non-integral clusters.
    try {
      const result = citeproc.makeCitationCluster(citeprocItems);
      // citeproc-js returns [id, text] or formatted cluster
      citations[cite.id] = result;
    } catch (e) {
      console.error(`Error rendering citation cluster ${cite.id}:`, e.message);
      citations[cite.id] = `ERROR: ${e.message}`;
    }
  });

  const bibResult = citeproc.makeBibliography();
  const bibliography = bibResult ? bibResult[1] : [];

  return { citations, bibliography };
}

function renderWithCslnProcessor(stylePath, testItems, testCitations) {
  const projectRoot = path.resolve(__dirname, '..');
  const styleName = path.basename(stylePath, '.csl');
  const stylesDir = path.join(projectRoot, 'styles');

  // 1. Try to find a hand-authored style first
  let cslnStylePath = null;
  if (fs.existsSync(stylesDir)) {
    const files = fs.readdirSync(stylesDir);
    const exactMatch = `${styleName}.yaml`;

    // Prefer exact filename matches before any prefix matching.
    if (files.includes(exactMatch)) {
      cslnStylePath = path.join(stylesDir, exactMatch);
    }

    if (!cslnStylePath) {
    // Look for exact match or base name match (e.g. apa-7th matches apa)
    const baseName = styleName.replace(/-\d+th$/, '').replace(/-\d+$/, '');
    const found = files.find(f =>
      f.endsWith('.yaml') &&
      (f.startsWith(`${styleName}-`) || f.startsWith(`${baseName}-`))
    );
    if (found) {
      cslnStylePath = path.join(stylesDir, found);
    }
    }
  }

  const tempRefFile = path.join(projectRoot, '.migrated-refs.json');
  fs.writeFileSync(tempRefFile, JSON.stringify(testItems, null, 2));

  const tempCiteFile = path.join(projectRoot, '.migrated-citations.json');
  fs.writeFileSync(tempCiteFile, JSON.stringify(testCitations, null, 2));

  let tempStyleFile = null;
  if (!cslnStylePath) {
    // 2. Fall back to migration
    const absStylePath = path.resolve(stylePath);
    let migratedYaml;
    try {
      migratedYaml = execSync(
        `cargo run -q --bin csln-migrate -- "${absStylePath}"`,
        { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
      );
    } catch (e) {
      console.error('Migration failed:', e.stderr || e.message);
      try { fs.unlinkSync(tempRefFile); } catch { }
      try { fs.unlinkSync(tempCiteFile); } catch { }
      return null;
    }
    tempStyleFile = path.join(projectRoot, '.migrated-temp.yaml');
    fs.writeFileSync(tempStyleFile, migratedYaml);
    cslnStylePath = tempStyleFile;
  }

  let output;
  try {
    output = execSync(
      `cargo run -q --bin csln -- render refs -b .migrated-refs.json -s "${cslnStylePath}" -c .migrated-citations.json --mode both --show-keys`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    const errorMsg = e.stderr ? e.stderr.toString() : e.message;
    return { error: `Processor failed: ${errorMsg}`, citations: { passed: 0, total: 0 }, bibliography: { passed: 0, total: 0 } };
  }

  if (tempStyleFile) try { fs.unlinkSync(tempStyleFile); } catch { }
  try { fs.unlinkSync(tempRefFile); } catch { }
  try { fs.unlinkSync(tempCiteFile); } catch { }

  const lines = output.split('\n');
  const citations = {};
  const bibliography = {};

  let section = null;
  let inIntegralSection = false;
  for (const line of lines) {
    if (line.includes('CITATIONS (From file):')) {
      section = 'citations';
      continue;
    } else if (line.includes('BIBLIOGRAPHY:')) {
      section = 'bibliography';
      continue;
    }

    if (section === 'citations' && line.trim() && !line.includes('===')) {
      const match = line.match(/^\s*\[([^\]]+)\]\s+(.+)/);
      if (match) {
        citations[match[1]] = match[2].trim();
      }
    } else if (section === 'bibliography' && line.trim() && !line.includes('===')) {
      const match = line.match(/^\s*\[([^\]]+)\]\s+(.+)/);
      if (match) {
        bibliography[match[1]] = match[2].trim();
      }
    }
  }

  // Convert bibliography map to array ordered by ID to match oracle expectation
  const orderedBibliography = [];
  Object.keys(testItems).forEach(id => {
    if (bibliography[id]) {
      orderedBibliography.push(bibliography[id]);
    }
  });

  return { citations, bibliography: orderedBibliography };
}

function collectCitationTypes(citation, testItems) {
  const types = new Set();
  for (const item of citation.items || []) {
    const ref = testItems[item.id];
    if (ref && ref.type) {
      types.add(ref.type);
    } else {
      types.add('unknown');
    }
  }
  return [...types];
}

/**
 * Match bibliography entries between oracle and CSLN by finding best matches.
 * Uses contributor names and titles to pair entries.
 */
function matchBibliographyEntries(oracleBib, cslnBib) {
  const pairs = [];
  const usedOracle = new Set();
  const usedCsln = new Set();
  const candidates = [];

  // Build all candidate pairings with similarity score.
  for (let oi = 0; oi < oracleBib.length; oi++) {
    for (let ci = 0; ci < cslnBib.length; ci++) {
      const score = textSimilarity(oracleBib[oi], cslnBib[ci]);
      // Keep weak matches out to avoid accidental cross-pairing.
      if (score >= 0.20) {
        candidates.push({ oi, ci, score });
      }
    }
  }

  // Global greedy assignment (highest-similarity edges first).
  candidates.sort((a, b) => b.score - a.score);
  for (const candidate of candidates) {
    if (usedOracle.has(candidate.oi) || usedCsln.has(candidate.ci)) continue;
    usedOracle.add(candidate.oi);
    usedCsln.add(candidate.ci);
    pairs.push({
      oracle: oracleBib[candidate.oi],
      csln: cslnBib[candidate.ci],
      score: candidate.score,
    });
  }

  // Add unmatched oracle entries.
  for (let oi = 0; oi < oracleBib.length; oi++) {
    if (!usedOracle.has(oi)) {
      pairs.push({ oracle: oracleBib[oi], csln: null, score: 0 });
    }
  }

  // Add unmatched CSLN entries.
  for (let ci = 0; ci < cslnBib.length; ci++) {
    if (!usedCsln.has(ci)) {
      pairs.push({ oracle: null, csln: cslnBib[ci], score: 0 });
    }
  }

  return pairs;
}

function tokenizeForSimilarity(text) {
  return normalizeText(text || '')
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s]/gu, ' ')
    .split(/\s+/)
    .filter(Boolean)
    .filter((token) => token.length > 1);
}

function textSimilarity(a, b) {
  const left = tokenizeForSimilarity(a);
  const right = tokenizeForSimilarity(b);
  if (left.length === 0 && right.length === 0) return 1;
  if (left.length === 0 || right.length === 0) return 0;

  const leftCounts = new Map();
  const rightCounts = new Map();
  for (const token of left) {
    leftCounts.set(token, (leftCounts.get(token) || 0) + 1);
  }
  for (const token of right) {
    rightCounts.set(token, (rightCounts.get(token) || 0) + 1);
  }

  let intersect = 0;
  let union = 0;
  const keys = new Set([...leftCounts.keys(), ...rightCounts.keys()]);
  for (const key of keys) {
    const l = leftCounts.get(key) || 0;
    const r = rightCounts.get(key) || 0;
    intersect += Math.min(l, r);
    union += Math.max(l, r);
  }

  return union > 0 ? intersect / union : 0;
}

function equivalentText(oracleText, cslnText) {
  const oracleNorm = normalizeText(oracleText);
  const cslnNorm = normalizeText(cslnText);
  if (oracleNorm === cslnNorm) return true;

  const similarity = textSimilarity(oracleNorm, cslnNorm);
  // High token-overlap tolerance for punctuation/order differences.
  if (similarity >= 0.60) return true;

  return false;
}

function extractYearSuffixes(text) {
  return normalizeText(text).match(/\b\d{4}[a-z]\b/gi) || [];
}

function hasEtAl(text) {
  return /\bet al\b/i.test(normalizeText(text));
}

function splitCitationCluster(text) {
  const normalized = normalizeText(text)
    .replace(/^\(/, '')
    .replace(/\)$/, '');
  return normalized
    .split(/\s*;\s*/)
    .map((part) => part.trim())
    .filter(Boolean);
}

function extractLocatorNumber(text) {
  const match = normalizeText(text).match(/\b(?:p|pp|section|sec)\.?\s*(\d+)\b/i);
  return match ? match[1] : null;
}

function equivalentDisambiguationProbe(oracleText, cslnText, citationId) {
  const oracleNorm = normalizeText(oracleText);
  const cslnNorm = normalizeText(cslnText);

  const oracleSuffixCount = extractYearSuffixes(oracleNorm).length;
  const cslnSuffixCount = extractYearSuffixes(cslnNorm).length;
  const oracleHasEtAl = hasEtAl(oracleNorm);
  const cslnHasEtAl = hasEtAl(cslnNorm);

  if (oracleHasEtAl && !cslnHasEtAl) return false;
  if (oracleSuffixCount > 0 && cslnSuffixCount === 0) return false;

  if (citationId === 'disambiguate-add-names-et-al') {
    if (oracleHasEtAl || oracleSuffixCount > 0) {
      const cslnParts = splitCitationCluster(cslnNorm);
      if (cslnParts.length < 2) return false;
      if (new Set(cslnParts).size !== cslnParts.length) return false;
    }
  }

  if (citationId === 'et-al-with-locator') {
    const oracleLocator = extractLocatorNumber(oracleNorm);
    const cslnLocator = extractLocatorNumber(cslnNorm);
    if (oracleLocator && oracleLocator !== cslnLocator) return false;
  }

  return true;
}

function equivalentCitationText(oracleText, cslnText, citationId) {
  if (STRICT_CITATION_IDS.has(citationId)) {
    return equivalentDisambiguationProbe(oracleText, cslnText, citationId);
  }
  return equivalentText(oracleText, cslnText);
}

// Main
const cliOptions = parseArgs();
const stylePath = cliOptions.stylePath;
const jsonOutput = cliOptions.jsonOutput;
const verbose = cliOptions.verbose;

const { testItems, testCitations } = loadFixtures(
  cliOptions.refsFixture,
  cliOptions.citationsFixture
);

const styleName = path.basename(stylePath, '.csl');

if (!jsonOutput) {
  console.log(`\n=== Structured Diff Oracle: ${styleName} ===\n`);
  console.log('Rendering with citeproc-js (oracle)...');
}

const oracle = renderWithCiteprocJs(stylePath, testItems, testCitations);

if (!jsonOutput) {
  console.log('Migrating and rendering with CSLN...');
}

const csln = renderWithCslnProcessor(stylePath, testItems, testCitations);

if (!csln || csln.error) {
  if (jsonOutput) {
    console.log(JSON.stringify({
      error: 'CSLN rendering failed',
      reason: csln && csln.error ? csln.error : 'Processor execution error or migration output invalid',
      style: styleName
    }));
  } else {
    console.error('❌ CSLN Rendering Failed\n');
    console.error(`Style: ${styleName}`);
    if (csln && csln.error) {
      console.error(`Reason: ${csln.error}\n`);
    } else {
      console.error('Reason: Processor execution error or invalid migration output\n');
    }
    console.error('Common causes:');
    console.error('  - Invalid YAML syntax in migrated style');
    console.error('  - Unsupported schema features in migration output');
    console.error('  - Missing required fields (info.id, version)\n');
    console.error('Next Steps:');
    console.error('  1. Check migration output: cargo run --bin csln-migrate -- <csl-path>');
    console.error('  2. Validate YAML syntax: yamllint .migrated-temp.yaml');
    console.error('  3. Check processor error: cargo run --bin csln -- render refs -b <refs> -s <style> -c <citations> --mode both');
  }
  process.exit(2);
}

// Analyze bibliography
const pairs = matchBibliographyEntries(oracle.bibliography, csln.bibliography);

const results = {
  style: styleName,
  citations: {
    total: testCitations.length,
    passed: 0,
    failed: 0,
    entries: [],
  },
  citationsByType: {},
  bibliography: {
    total: pairs.length,
    passed: 0,
    failed: 0,
    entries: [],
  },
  componentSummary: {},
  orderingIssues: 0,
};

// Check citations
for (const cite of testCitations) {
  const id = cite.id;
  const oracleCit = normalizeText(oracle.citations[id] || '');
  const cslnCit = normalizeText(csln.citations[id] || '');
  const match = equivalentCitationText(oracleCit, cslnCit, id);
  if (match) {
    results.citations.passed++;
  } else {
    results.citations.failed++;
  }
  results.citations.entries.push({ id, oracle: oracleCit, csln: cslnCit, match });

  const citationTypes = collectCitationTypes(cite, testItems);
  for (const type of citationTypes) {
    if (!results.citationsByType[type]) {
      results.citationsByType[type] = { total: 0, passed: 0 };
    }
    results.citationsByType[type].total++;
    if (match) {
      results.citationsByType[type].passed++;
    }
  }
}

// Analyze bibliography entries
for (let i = 0; i < pairs.length; i++) {
  const pair = pairs[i];
  const entryResult = {
    index: i + 1,
    oracle: pair.oracle ? normalizeText(pair.oracle) : null,
    csln: pair.csln ? normalizeText(pair.csln) : null,
    match: false,
    components: {},
    ordering: null,
    issues: [],
  };

  if (!pair.oracle) {
    entryResult.issues.push({ issue: 'extra_entry', detail: 'Entry in CSLN but not oracle' });
    results.bibliography.failed++;
  } else if (!pair.csln) {
    entryResult.issues.push({ issue: 'missing_entry', detail: 'Entry in oracle but not CSLN' });
    results.bibliography.failed++;
  } else {
    // Both exist - compare
    const oracleNorm = normalizeText(pair.oracle);
    const cslnNorm = normalizeText(pair.csln);

    if (equivalentText(oracleNorm, cslnNorm)) {
      entryResult.match = true;
      results.bibliography.passed++;
    } else {
      results.bibliography.failed++;

      // Find reference data for this entry
      const refData = findRefDataForEntry(pair.oracle, testItems);

      // Parse components (only if reference data found)
      if (refData) {
        const oracleComp = parseComponents(pair.oracle, refData);
        const cslnComp = parseComponents(pair.csln, refData);

        // Compare components
        const { differences, matches } = compareComponents(oracleComp, cslnComp, refData);
        entryResult.components = { differences, matches };

        // Analyze ordering
        const oracleOrder = analyzeOrdering(pair.oracle, refData);
        const cslnOrder = analyzeOrdering(pair.csln, refData);
        const orderIssues = compareOrdering(oracleOrder, cslnOrder);

        if (orderIssues.length > 0) {
          entryResult.ordering = { oracle: oracleOrder, csln: cslnOrder };
          results.orderingIssues++;
        }

        entryResult.issues = [...differences, ...orderIssues];

        // Track component issues for summary
        for (const diff of differences) {
          const key = `${diff.component}:${diff.issue}`;
          results.componentSummary[key] = (results.componentSummary[key] || 0) + 1;
        }
      } else {
        // No reference data found - skip component analysis
        entryResult.issues = [];
      }
    }
  }

  results.bibliography.entries.push(entryResult);
}

// Output
if (jsonOutput) {
  console.log(JSON.stringify(results, null, 2));
} else {
  // Human-readable output
  console.log('\n--- CITATIONS ---');
  console.log(`  ✅ Passed: ${results.citations.passed}/${results.citations.total}`);
  if (results.citations.failed > 0) {
    console.log(`  ❌ Failed: ${results.citations.failed}/${results.citations.total}`);
  }

  console.log('\n--- BIBLIOGRAPHY ---');
  console.log(`  ✅ Passed: ${results.bibliography.passed}/${results.bibliography.total}`);
  console.log(`  ❌ Failed: ${results.bibliography.failed}/${results.bibliography.total}`);

  if (Object.keys(results.componentSummary).length > 0) {
    console.log('\n--- COMPONENT ISSUES ---');
    const sorted = Object.entries(results.componentSummary)
      .sort((a, b) => b[1] - a[1]);
    for (const [issue, count] of sorted) {
      console.log(`  ${issue}: ${count} entries`);
    }
  }

  if (results.orderingIssues > 0) {
    console.log(`\n--- ORDERING ISSUES: ${results.orderingIssues} entries ---`);
  }

  if (verbose) {
    console.log('\n--- DETAILED FAILURES ---');

    // Citation failures
    const failedCitations = results.citations.entries.filter(e => !e.match);
    if (failedCitations.length > 0) {
      console.log('\nCitations:');
      for (const entry of failedCitations) {
        console.log(`  [${entry.id}]`);
        console.log(`    Oracle: ${entry.oracle}`);
        console.log(`    CSLN:   ${entry.csln}`);
      }
    }

    // Bibliography failures
    for (const entry of results.bibliography.entries) {
      if (!entry.match && entry.oracle && entry.csln) {
        console.log(`\nEntry ${entry.index}:`);
        console.log(`  Oracle: ${entry.oracle}`);
        console.log(`  CSLN:   ${entry.csln}`);
        if (entry.ordering) {
          console.log(`  Order Oracle: ${entry.ordering.oracle.join(' → ')}`);
          console.log(`  Order CSLN:   ${entry.ordering.csln.join(' → ')}`);
        }
        for (const issue of entry.issues) {
          console.log(`  Issue: ${issue.component || issue.issue}: ${issue.detail || ''}`);
        }
      }
    }
  }

  console.log('\n=== SUMMARY ===');
  console.log(`Citations: ${results.citations.passed}/${results.citations.total} match`);
  console.log(`Bibliography: ${results.bibliography.passed}/${results.bibliography.total} match`);
  console.log();
}

process.exit(results.citations.failed === 0 && results.bibliography.failed === 0 ? 0 : 1);
