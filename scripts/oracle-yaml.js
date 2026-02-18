#!/usr/bin/env node
/**
 * Oracle test for hand-authored CSLN styles.
 *
 * Compares a CSLN YAML style against citeproc-js output from the corresponding CSL file.
 * Useful for verifying hand-authored styles during styleauthor workflow.
 *
 * Usage:
 *   node oracle-yaml.js styles/elsevier-harvard.yaml styles-legacy/elsevier-harvard.csl
 *   node oracle-yaml.js styles/apa-7th.yaml --json
 *   node oracle-yaml.js styles/chicago.yaml --verbose
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
  loadLocale
} = require('./oracle-utils');

// Load test items from JSON fixture
const fixturesPath = path.join(__dirname, '..', 'tests', 'fixtures', 'references-expanded.json');
const fixturesData = JSON.parse(fs.readFileSync(fixturesPath, 'utf8'));
const testItems = Object.fromEntries(
  Object.entries(fixturesData).filter(([key]) => key !== 'comment')
);

/**
 * Render CSLN YAML style with csln process.
 * @param {string} yamlPath - Path to CSLN YAML file
 * @returns {{ citations: Object, bibliography: Array }|null}
 */
function renderWithCslnYaml(yamlPath) {
  const projectRoot = path.resolve(__dirname, '..');
  const absYamlPath = path.resolve(yamlPath);

  let output;
  try {
    output = execSync(
      `cargo run -q --bin csln -- process tests/fixtures/references-expanded.json "${absYamlPath}" --show-keys`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    console.error('Processor failed:', e.stderr || e.message);
    return null;
  }

  const lines = output.split('\n');
  const citations = {};
  const bibliography = [];

  let section = null;
  for (const line of lines) {
    if (line.includes('CITATIONS (Non-Integral)')) {
      section = 'citations';
    } else if (line.includes('CITATIONS (Integral)')) {
      // For now, we compare against non-integral which matches citeproc-js default
      section = 'citations-integral';
    } else if (line.includes('BIBLIOGRAPHY:')) {
      section = 'bibliography';
    } else if (section === 'citations' && line.match(/\[ITEM-\d+\]/)) {
      const match = line.match(/\[(ITEM-\d+)\]\s*(.+)/);
      if (match && !citations[match[1]]) {
        citations[match[1]] = match[2].trim();
      }
    } else if (section === 'bibliography' && line.trim() && !line.includes('===')) {
      const match = line.match(/\[([^\]]+)\]\s+(.+)/);
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
 * Render reference style with citeproc-js for comparison.
 * @param {string} cslPath - Path to CSL file
 * @returns {{ citations: Object, bibliography: Array }|null}
 */
function renderWithCiteprocJs(cslPath) {
  if (!fs.existsSync(cslPath)) {
    console.warn(`Reference CSL not found: ${cslPath}`);
    return null;
  }

  const styleXml = fs.readFileSync(cslPath, 'utf8');

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

/**
 * Match bibliography entries between oracle and CSLN by finding best matches.
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

/**
 * Format output as JSON.
 */
function jsonOutput(results) {
  const summary = {
    style: results.styleName,
    citations: {
      total: results.citationResults.length,
      matches: results.citationResults.filter(r => r.match).length,
      mismatches: results.citationResults.filter(r => !r.match).length
    },
    bibliography: {
      total: results.bibResults.length,
      matches: results.bibResults.filter(r => r.match).length,
      mismatches: results.bibResults.filter(r => !r.match).length
    },
    entries: {
      citations: results.citationResults,
      bibliography: results.bibResults
    }
  };
  return JSON.stringify(summary, null, 2);
}

/**
 * Format output as human-readable text.
 */
function textOutput(results, verbose) {
  let output = `\nOracle Test: ${results.styleName}\n`;
  output += '='.repeat(50) + '\n\n';

  // Citations
  const citMatches = results.citationResults.filter(r => r.match).length;
  const citTotal = results.citationResults.length;
  output += `CITATIONS: ${citMatches}/${citTotal} match\n`;
  if (verbose && results.citationResults.length > 0) {
    for (const result of results.citationResults.slice(0, 3)) {
      const status = result.match ? '✓' : '✗';
      output += `  ${status} ${result.itemId}\n`;
      if (!result.match) {
        output += `    Expected: ${result.oracle.substring(0, 60)}...\n`;
        output += `    Got:      ${result.csln.substring(0, 60)}...\n`;
      }
    }
  }

  // Bibliography
  output += '\n';
  const bibMatches = results.bibResults.filter(r => r.match).length;
  const bibTotal = results.bibResults.length;
  output += `BIBLIOGRAPHY: ${bibMatches}/${bibTotal} match\n`;
  if (verbose && results.bibResults.length > 0) {
    for (const result of results.bibResults.slice(0, 3)) {
      if (result.oracle) {
        const status = result.match ? '✓' : '✗';
        output += `  ${status} Entry\n`;
        if (!result.match) {
          output += `    Expected: ${result.oracle.substring(0, 60)}...\n`;
          output += `    Got:      ${result.csln ? result.csln.substring(0, 60) + '...' : '(missing)'}\n`;
        }
      }
    }
  }

  output += '\n' + '='.repeat(50) + '\n';
  return output;
}

// Main
const args = process.argv.slice(2);
const yamlPath = args.find(a => !a.startsWith('--'));
const jsonMode = args.includes('--json');
const verbose = args.includes('--verbose');

if (!yamlPath) {
  console.error('Usage: node oracle-yaml.js <yaml-path> [reference.csl] [--json] [--verbose]');
  process.exit(1);
}

const styleName = path.basename(yamlPath, '.yaml');

// Find reference CSL
let cslPath = args.find((a, i) => !a.startsWith('--') && i !== args.indexOf(yamlPath));
if (!cslPath) {
  cslPath = path.join(path.dirname(yamlPath), '..', 'styles-legacy', `${styleName}.csl`);
}

// Render with CSLN YAML
const cslnResult = renderWithCslnYaml(yamlPath);
if (!cslnResult) {
  console.error('Failed to render CSLN YAML');
  process.exit(1);
}

// Render with citeproc-js if reference exists
const citeprocResult = renderWithCiteprocJs(cslPath);

// Compare
const citationResults = [];
const bibResults = [];

if (citeprocResult) {
  // Compare citations
  for (const [itemId, cslnCit] of Object.entries(cslnResult.citations)) {
    const oracleCit = citeprocResult.citations[itemId];
    if (oracleCit) {
      const match = normalizeText(oracleCit) === normalizeText(cslnCit);
      citationResults.push({
        itemId,
        oracle: oracleCit,
        csln: cslnCit,
        match
      });
    }
  }

  // Compare bibliography
  const bibPairs = matchBibliographyEntries(citeprocResult.bibliography, cslnResult.bibliography);
  for (const pair of bibPairs) {
    const match = pair.oracle && pair.csln && normalizeText(pair.oracle) === normalizeText(pair.csln);
    bibResults.push({
      oracle: pair.oracle,
      csln: pair.csln,
      match,
      score: pair.score
    });
  }
} else {
  // No reference, just report what we got
  console.warn('No reference CSL found; comparing CSLN output only.');
}

const results = { styleName, citationResults, bibResults };

if (jsonMode) {
  console.log(jsonOutput(results));
} else {
  console.log(textOutput(results, verbose));
}
