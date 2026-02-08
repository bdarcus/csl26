#!/usr/bin/env node
/**
 * End-to-end oracle test for CSLN migration.
 * 
 * This script:
 * 1. Takes a CSL 1.0 file
 * 2. Renders citations/bibliography with citeproc-js (the oracle)
 * 3. Migrates the CSL file to CSLN format
 * 4. Renders with csln_processor
 * 5. Compares the outputs
 * 
 * Usage: node oracle-e2e.js ../styles/apa.csl
 */

const CSL = require('citeproc');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Load locale from file (same as oracle.js)
function loadLocale(lang) {
  const localePath = path.join(__dirname, `locales-${lang}.xml`);
  if (fs.existsSync(localePath)) {
    return fs.readFileSync(localePath, 'utf8');
  }
  // Fallback to en-US
  const fallback = path.join(__dirname, 'locales-en-US.xml');
  if (fs.existsSync(fallback)) {
    return fs.readFileSync(fallback, 'utf8');
  }
  throw new Error(`Locale not found: ${lang}`);
}

// Load test items from JSON fixture
const fixturesPath = path.join(__dirname, '..', 'tests', 'fixtures', 'references-expanded.json');
const fixturesData = JSON.parse(fs.readFileSync(fixturesPath, 'utf8'));
const testItems = Object.fromEntries(
  Object.entries(fixturesData).filter(([key]) => key !== 'comment')
);

function renderWithCiteprocJs(stylePath) {
  const styleXml = fs.readFileSync(stylePath, 'utf8');

  const sys = {
    retrieveLocale: (lang) => loadLocale(lang),
    retrieveItem: (id) => testItems[id]
  };

  const citeproc = new CSL.Engine(sys, styleXml);
  citeproc.updateItems(Object.keys(testItems));

  // Generate citations using makeCitationCluster (simpler API)
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

  // Migrate CSL to CSLN
  let migratedYaml;
  try {
    migratedYaml = execSync(
      `cargo run -q --bin csln-migrate -- "${absStylePath}"`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'inherit'] }
    );
  } catch (e) {
    console.error('Migration failed:', e.stderr || e.message);
    return null;
  }

  // Write to temp file in project root
  const tempFile = path.join(projectRoot, '.migrated-temp.yaml');
  fs.writeFileSync(tempFile, migratedYaml);

  // Run csln_processor
  let output;
  try {
    output = execSync(
      `cargo run -q --bin csln-processor -- .migrated-temp.yaml`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    console.error('Processor failed:', e.stderr || e.message);
    try { fs.unlinkSync(tempFile); } catch { } // Ignore if already deleted
    return null;
  }

  try { fs.unlinkSync(tempFile); } catch { } // Ignore if already deleted

  // Parse output
  const lines = output.split('\n');
  const citations = {};
  const bibliography = [];

  let section = null;
  for (const line of lines) {
    if (line.includes('CITATIONS:')) {
      section = 'citations';
    } else if (line.includes('BIBLIOGRAPHY:')) {
      section = 'bibliography';
    } else if (section === 'citations' && line.match(/\[ITEM-\d+\]/)) {
      const match = line.match(/\[(ITEM-\d+)\]\s*(.+)/);
      if (match) {
        citations[match[1]] = match[2].trim();
      }
    } else if (section === 'bibliography' && line.trim()) {
      bibliography.push(line.trim());
    }
  }

  return { citations, bibliography };
}

function normalizeText(text) {
  return text
    .replace(/<[^>]+>/g, '')       // Strip HTML tags
    .replace(/&#38;/g, '&')        // HTML entity for &
    .replace(/_([^_]+)_/g, '$1')   // Strip markdown italics
    .replace(/\*\*([^*]+)\*\*/g, '$1') // Strip markdown bold
    .replace(/\s+/g, ' ')          // Normalize whitespace
    .trim();
}

function compare(oracle, csln, label) {
  if (!oracle) {
    console.log(`  ⚠️  ${label} - Oracle returned undefined`);
    return false;
  }
  if (!csln) {
    console.log(`  ⚠️  ${label} - CSLN returned undefined`);
    return false;
  }

  const oracleNorm = normalizeText(oracle);
  const cslnNorm = normalizeText(csln);

  if (oracleNorm === cslnNorm) {
    console.log(`  ✅ ${label}`);
    return true;
  } else {
    console.log(`  ❌ ${label}`);
    console.log(`     Oracle: ${oracleNorm}`);
    console.log(`     CSLN:   ${cslnNorm}`);
    return false;
  }
}

// Compute similarity between two strings using longest common subsequence ratio.
function similarity(a, b) {
  if (a === b) return 1;
  if (!a || !b) return 0;
  // Use word-level overlap for speed
  const wordsA = a.toLowerCase().split(/\s+/);
  const wordsB = new Set(b.toLowerCase().split(/\s+/));
  let common = 0;
  for (const w of wordsA) {
    if (wordsB.has(w)) common++;
  }
  return common / Math.max(wordsA.length, wordsB.size);
}

// Main
const stylePath = process.argv[2] || path.join(__dirname, '..', 'styles', 'apa.csl');
const styleName = path.basename(stylePath, '.csl');

console.log(`\n=== End-to-End Oracle Test: ${styleName} ===\n`);

console.log('Rendering with citeproc-js (oracle)...');
const oracle = renderWithCiteprocJs(stylePath);

console.log('Migrating and rendering with CSLN...');
const csln = renderWithCslnProcessor(stylePath);

if (!csln) {
  console.log('\n❌ CSLN rendering failed\n');
  process.exit(1);
}

console.log('\n--- CITATIONS ---');
let citationsMatch = 0;
let citationsTotal = 0;
Object.keys(testItems).forEach(id => {
  citationsTotal++;
  if (compare(oracle.citations[id], csln.citations[id], id)) {
    citationsMatch++;
  }
});

console.log('\n--- BIBLIOGRAPHY ---');
let bibMatch = 0;
const oracleBibNorm = oracle.bibliography.filter(b => b).map(b => normalizeText(b));
const cslnBibNorm = csln.bibliography.filter(b => b).map(b => normalizeText(b));

// Match entries by best similarity rather than positional comparison.
// When CSLN produces fewer entries (unsupported reference types), positional
// comparison after sorting mismatches unrelated entries against each other.
const usedOracle = new Set();
const matched = [];

for (let ci = 0; ci < cslnBibNorm.length; ci++) {
  const cEntry = cslnBibNorm[ci];
  let bestIdx = -1;
  let bestScore = 0;
  for (let oi = 0; oi < oracleBibNorm.length; oi++) {
    if (usedOracle.has(oi)) continue;
    const score = similarity(cEntry, oracleBibNorm[oi]);
    if (score > bestScore) {
      bestScore = score;
      bestIdx = oi;
    }
  }
  if (bestIdx >= 0) {
    usedOracle.add(bestIdx);
    matched.push({ oracle: oracleBibNorm[bestIdx], csln: cEntry });
  }
}

// Add unmatched oracle entries
for (let oi = 0; oi < oracleBibNorm.length; oi++) {
  if (!usedOracle.has(oi)) {
    matched.push({ oracle: oracleBibNorm[oi], csln: null });
  }
}

const bibTotal = matched.length;
for (let i = 0; i < bibTotal; i++) {
  const { oracle: oEntry, csln: cEntry } = matched[i];
  // Truncate label from oracle entry for readability
  const label = oEntry.substring(0, 40).replace(/\s+$/, '') + (oEntry.length > 40 ? '...' : '');
  if (compare(oEntry, cEntry, label)) {
    bibMatch++;
  }
}

console.log(`\n=== SUMMARY ===`);
console.log(`Citations: ${citationsMatch}/${citationsTotal} match`);
console.log(`Bibliography: ${bibMatch}/${bibTotal} match`);
console.log();

process.exit(citationsMatch === citationsTotal && bibMatch === bibTotal ? 0 : 1);
