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

// Load test items from JSON fixture
const fixturesPath = path.join(__dirname, '..', 'tests', 'fixtures', 'references-expanded.json');
const fixturesData = JSON.parse(fs.readFileSync(fixturesPath, 'utf8'));
const testItems = Object.fromEntries(
  Object.entries(fixturesData).filter(([key]) => key !== 'comment')
);

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

function renderWithCiteprocJs(stylePath) {
  const styleXml = fs.readFileSync(stylePath, 'utf8');

  const sys = {
    retrieveLocale: (lang) => loadLocale(lang),
    retrieveItem: (id) => testItems[id]
  };

  const citeproc = new CSL.Engine(sys, styleXml);
  citeproc.updateItems(Object.keys(testItems));

  const citations = {};
  Object.keys(testItems).forEach(id => {
    citations[id] = citeproc.makeCitationCluster([{ id }]);
  });

  const bibResult = citeproc.makeBibliography();
  const bibliography = bibResult ? bibResult[1] : [];

  return { citations, bibliography };
}

function renderWithCslnProcessor(stylePath) {
  const projectRoot = path.resolve(__dirname, '..');
  const absStylePath = path.resolve(stylePath);

  let migratedYaml;
  try {
    migratedYaml = execSync(
      `cargo run -q --bin csln-migrate -- "${absStylePath}"`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    console.error('Migration failed:', e.stderr || e.message);
    return null;
  }

  const tempStyleFile = path.join(projectRoot, '.migrated-temp.yaml');
  const tempRefFile = path.join(projectRoot, '.migrated-refs.json');
  fs.writeFileSync(tempStyleFile, migratedYaml);
  fs.writeFileSync(tempRefFile, JSON.stringify(testItems, null, 2));

  let output;
  try {
    output = execSync(
      `cargo run -q --bin csln -- process .migrated-refs.json .migrated-temp.yaml --show-keys`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    console.error('Processor failed:', e.stderr || e.message);
    try { fs.unlinkSync(tempStyleFile); } catch {}
    try { fs.unlinkSync(tempRefFile); } catch {}
    return null;
  }

  try { fs.unlinkSync(tempStyleFile); } catch {}
  try { fs.unlinkSync(tempRefFile); } catch {}

  const lines = output.split('\n');
  const citations = {};
  const bibliography = [];

  let section = null;
  for (const line of lines) {
    if (line.includes('CITATIONS (Non-Integral)') || line.includes('CITATIONS (Integral)')) {
      section = 'citations';
      continue;
    } else if (line.includes('BIBLIOGRAPHY:')) {
      section = 'bibliography';
      continue;
    } else if (section === 'citations' && line.trim() && !line.includes('===')) {
      const match = line.match(/^\s*\[([^\]]+)\]\s+(.+)/);
      if (match) {
        citations[match[1]] = match[2].trim();
      }
    } else if (section === 'bibliography' && line.trim() && !line.includes('===')) {
      const match = line.match(/^\s*\[([^\]]+)\]\s+(.+)/);
      if (match) {
        bibliography.push(match[2].trim());
      } else {
        bibliography.push(line.trim());
      }
    }
  }

  return { citations, bibliography };
}

/**
 * Match bibliography entries between oracle and CSLN by finding best matches.
 * Uses contributor names and titles to pair entries.
 */
function matchBibliographyEntries(oracleBib, cslnBib) {
  const pairs = [];
  const usedCsln = new Set();

  for (const oracleEntry of oracleBib) {
    const oracleNorm = normalizeText(oracleEntry).toLowerCase();
    let bestMatch = null;
    let bestScore = 0;

    for (let i = 0; i < cslnBib.length; i++) {
      if (usedCsln.has(i)) continue;

      const cslnNorm = normalizeText(cslnBib[i]).toLowerCase();

      // Score based on shared words
      const oracleWords = new Set(oracleNorm.split(/\s+/).filter(w => w.length > 3));
      const cslnWords = new Set(cslnNorm.split(/\s+/).filter(w => w.length > 3));
      let score = 0;
      for (const word of oracleWords) {
        if (cslnWords.has(word)) score++;
      }

      if (score > bestScore) {
        bestScore = score;
        bestMatch = i;
      }
    }

    if (bestMatch !== null && bestScore > 2) {
      pairs.push({ oracle: oracleEntry, csln: cslnBib[bestMatch], score: bestScore });
      usedCsln.add(bestMatch);
    } else {
      pairs.push({ oracle: oracleEntry, csln: null, score: 0 });
    }
  }

  // Add unmatched CSLN entries
  for (let i = 0; i < cslnBib.length; i++) {
    if (!usedCsln.has(i)) {
      pairs.push({ oracle: null, csln: cslnBib[i], score: 0 });
    }
  }

  return pairs;
}

// Main
const args = process.argv.slice(2);
const stylePath = args.find(a => !a.startsWith('--')) || path.join(__dirname, '..', 'styles-legacy', 'apa.csl');
const jsonOutput = args.includes('--json');
const verbose = args.includes('--verbose');

const styleName = path.basename(stylePath, '.csl');

if (!jsonOutput) {
  console.log(`\n=== Structured Diff Oracle: ${styleName} ===\n`);
  console.log('Rendering with citeproc-js (oracle)...');
}

const oracle = renderWithCiteprocJs(stylePath);

if (!jsonOutput) {
  console.log('Migrating and rendering with CSLN...');
}

const csln = renderWithCslnProcessor(stylePath);

if (!csln) {
  if (jsonOutput) {
    console.log(JSON.stringify({
      error: 'CSLN rendering failed',
      reason: 'Processor execution error or migration output invalid',
      style: styleName
    }));
  } else {
    console.error('❌ CSLN Rendering Failed\n');
    console.error(`Style: ${styleName}`);
    console.error('Reason: Processor execution error or invalid migration output\n');
    console.error('Common causes:');
    console.error('  - Invalid YAML syntax in migrated style');
    console.error('  - Unsupported schema features in migration output');
    console.error('  - Missing required fields (info.id, version)\n');
    console.error('Next Steps:');
    console.error('  1. Check migration output: cargo run --bin csln-migrate -- <csl-path>');
    console.error('  2. Validate YAML syntax: yamllint .migrated-temp.yaml');
    console.error('  3. Check processor error: cargo run --bin csln -- process <refs> <style>');
  }
  process.exit(2);
}

// Analyze bibliography
const pairs = matchBibliographyEntries(oracle.bibliography, csln.bibliography);

const results = {
  style: styleName,
  citations: {
    total: Object.keys(testItems).length,
    passed: 0,
    failed: 0,
    entries: [],
  },
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
for (const id of Object.keys(testItems)) {
  const oracleCit = normalizeText(oracle.citations[id] || '');
  const cslnCit = normalizeText(csln.citations[id] || '');
  const match = oracleCit === cslnCit;
  if (match) {
    results.citations.passed++;
  } else {
    results.citations.failed++;
  }
  results.citations.entries.push({ id, oracle: oracleCit, csln: cslnCit, match });
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

    if (oracleNorm === cslnNorm) {
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
