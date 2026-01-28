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

// Test items - same as used in csln_processor
const testItems = {
  "ITEM-1": {
    "id": "ITEM-1",
    "type": "article-journal",
    "title": "The Structure of Scientific Revolutions",
    "author": [{ "family": "Kuhn", "given": "Thomas S." }],
    "issued": { "date-parts": [[1962]] },
    "container-title": "International Encyclopedia of Unified Science",
    "volume": "2",
    "issue": "2",
    "publisher": "University of Chicago Press",
    "publisher-place": "Chicago",
    "DOI": "10.1234/example"
  },
  "ITEM-2": {
    "id": "ITEM-2",
    "type": "book",
    "title": "A Brief History of Time",
    "author": [{ "family": "Hawking", "given": "Stephen" }],
    "issued": { "date-parts": [[1988]] },
    "publisher": "Bantam Dell Publishing Group",
    "publisher-place": "New York"
  },
  "ITEM-3": {
    "id": "ITEM-3",
    "type": "article-journal",
    "title": "Deep Learning",
    "author": [
      { "family": "LeCun", "given": "Yann" },
      { "family": "Bengio", "given": "Yoshua" },
      { "family": "Hinton", "given": "Geoffrey" }
    ],
    "issued": { "date-parts": [[2015]] },
    "container-title": "Nature",
    "volume": "521",
    "page": "436-444",
    "DOI": "10.1038/nature14539"
  },
  "ITEM-4": {
    "id": "ITEM-4",
    "type": "chapter",
    "title": "The Role of Deliberate Practice",
    "author": [
      { "family": "Ericsson", "given": "K. Anders" }
    ],
    "editor": [
      { "family": "Ericsson", "given": "K. Anders" },
      { "family": "Charness", "given": "Neil" },
      { "family": "Feltovich", "given": "Paul J." },
      { "family": "Hoffman", "given": "Robert R." }
    ],
    "issued": { "date-parts": [[2006]] },
    "container-title": "The Cambridge Handbook of Expertise and Expert Performance",
    "publisher": "Cambridge University Press",
    "page": "683-703"
  },
  "ITEM-5": {
    "id": "ITEM-5",
    "type": "report",
    "title": "World Development Report 2023",
    "author": [{ "literal": "World Bank" }],
    "issued": { "date-parts": [[2023]] },
    "publisher": "World Bank Group",
    "publisher-place": "Washington, DC"
  }
};

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
      `cargo run -q --bin csln_migrate -- "${absStylePath}"`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
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
      `cargo run -q --bin csln_processor -- .migrated-temp.yaml`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    console.error('Processor failed:', e.stderr || e.message);
    fs.unlinkSync(tempFile);
    return null;
  }
  
  fs.unlinkSync(tempFile);
  
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
    .replace(/<[^>]+>/g, '')   // Strip HTML tags
    .replace(/&#38;/g, '&')    // HTML entity for &
    .replace(/_([^_]+)_/g, '$1') // Strip markdown italics    // HTML entity for &
    .replace(/\s+/g, ' ')      // Normalize whitespace
    .trim();
}

function compare(oracle, csln, label) {
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
// Sort both by first author for comparison
const oracleBibNorm = oracle.bibliography.map(b => normalizeText(b)).sort();
const cslnBibNorm = csln.bibliography.map(b => normalizeText(b)).sort();
const bibTotal = Math.max(oracleBibNorm.length, cslnBibNorm.length);

for (let i = 0; i < bibTotal; i++) {
  const oEntry = oracleBibNorm[i] || '(missing)';
  const cEntry = cslnBibNorm[i] || '(missing)';
  if (compare(oEntry, cEntry, `Entry ${i + 1}`)) {
    bibMatch++;
  }
}

console.log(`\n=== SUMMARY ===`);
console.log(`Citations: ${citationsMatch}/${citationsTotal} match`);
console.log(`Bibliography: ${bibMatch}/${bibTotal} match`);
console.log();

process.exit(citationsMatch === citationsTotal && bibMatch === bibTotal ? 0 : 1);
