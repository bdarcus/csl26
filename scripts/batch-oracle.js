#!/usr/bin/env node
/**
 * Batch oracle test runner - evaluates rendering fidelity across all styles.
 * 
 * Usage: node batch-oracle.js [options]
 * 
 * Options:
 *   --limit N      Test only first N styles (for quick checks)
 *   --sample N     Random sample of N styles
 *   --pattern P    Only test styles matching pattern (e.g., "apa")
 *   --output FILE  Write JSON report to file (default: styles-report.json)
 *   --verbose      Show per-style progress
 */

const CSL = require('citeproc');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const stylesDir = path.join(__dirname, '..', 'styles');
const projectRoot = path.resolve(__dirname, '..');

// Load locale
function loadLocale(lang) {
    const localePath = path.join(__dirname, `locales-${lang}.xml`);
    if (fs.existsSync(localePath)) {
        return fs.readFileSync(localePath, 'utf8');
    }
    const fallback = path.join(__dirname, 'locales-en-US.xml');
    if (fs.existsSync(fallback)) {
        return fs.readFileSync(fallback, 'utf8');
    }
    throw new Error(`Locale not found: ${lang}`);
}

// Test items
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
    "author": [{ "family": "Ericsson", "given": "K. Anders" }],
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

function normalizeText(text) {
  if (!text) return '';
  return text
    .replace(/<[^>]+>/g, '')
    .replace(/&#38;/g, '&')
    .replace(/_([^_]+)_/g, '$1')
    .replace(/\s+/g, ' ')
    .trim();
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
  const absStylePath = path.resolve(stylePath);
  
  // Migrate CSL to CSLN
  let migratedYaml;
  try {
    migratedYaml = execSync(
      `cargo run -q --bin csln_migrate -- "${absStylePath}"`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    return { error: 'migration', message: (e.stderr || e.message).slice(0, 200) };
  }
  
  const tempFile = path.join(projectRoot, '.migrated-temp.yaml');
  fs.writeFileSync(tempFile, migratedYaml);
  
  let output;
  try {
    output = execSync(
      `cargo run -q --bin csln_processor -- .migrated-temp.yaml`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    fs.rmSync(tempFile, { force: true });
    return { error: 'processor', message: (e.stderr || e.message).slice(0, 200) };
  }

  fs.rmSync(tempFile, { force: true });
  
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

function testStyle(stylePath) {
  const styleName = path.basename(stylePath, '.csl');
  
  // Render with oracle
  let oracle;
  try {
    oracle = renderWithCiteprocJs(stylePath);
  } catch (e) {
    return { 
      style: styleName, 
      status: 'oracle_error', 
      message: e.message.slice(0, 200) 
    };
  }
  
  // Render with CSLN
  const csln = renderWithCslnProcessor(stylePath);
  
  if (csln.error) {
    return {
      style: styleName,
      status: csln.error + '_error',
      message: csln.message
    };
  }
  
  // Compare
  let citationMatches = 0;
  let citationMismatches = [];
  Object.keys(testItems).forEach(id => {
    const oNorm = normalizeText(oracle.citations[id]);
    const cNorm = normalizeText(csln.citations[id]);
    if (oNorm === cNorm) {
      citationMatches++;
    } else {
      citationMismatches.push({ id, oracle: oNorm, csln: cNorm });
    }
  });
  
  let bibMatches = 0;
  let bibMismatches = [];
  const oracleBib = oracle.bibliography.map(b => normalizeText(b)).sort();
  const cslnBib = csln.bibliography.map(b => normalizeText(b)).sort();
  const bibTotal = Math.max(oracleBib.length, cslnBib.length);
  
  for (let i = 0; i < bibTotal; i++) {
    const o = oracleBib[i] || '(missing)';
    const c = cslnBib[i] || '(missing)';
    if (o === c) {
      bibMatches++;
    } else {
      if (bibMismatches.length < 2) { // Only keep first 2 samples
        bibMismatches.push({ oracle: o, csln: c });
      }
    }
  }
  
  const citationTotal = Object.keys(testItems).length;
  const passed = citationMatches === citationTotal && bibMatches === bibTotal;
  
  return {
    style: styleName,
    status: passed ? 'pass' : 'mismatch',
    citations: { match: citationMatches, total: citationTotal },
    bibliography: { match: bibMatches, total: bibTotal },
    samples: passed ? undefined : {
      citations: citationMismatches.slice(0, 2),
      bibliography: bibMismatches
    }
  };
}

// Parse CLI args
const args = process.argv.slice(2);
let limit = null;
let sample = null;
let pattern = null;
let outputFile = 'styles-report.json';
let verbose = false;

for (let i = 0; i < args.length; i++) {
  if (args[i] === '--limit' && args[i+1]) limit = parseInt(args[++i]);
  if (args[i] === '--sample' && args[i+1]) sample = parseInt(args[++i]);
  if (args[i] === '--pattern' && args[i+1]) pattern = args[++i];
  if (args[i] === '--output' && args[i+1]) outputFile = args[++i];
  if (args[i] === '--verbose') verbose = true;
}

// Get style files
let styleFiles = fs.readdirSync(stylesDir)
  .filter(f => f.endsWith('.csl'))
  .map(f => path.join(stylesDir, f));

if (pattern) {
  styleFiles = styleFiles.filter(f => path.basename(f).includes(pattern));
}

if (sample) {
  // Random sample
  for (let i = styleFiles.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [styleFiles[i], styleFiles[j]] = [styleFiles[j], styleFiles[i]];
  }
  styleFiles = styleFiles.slice(0, sample);
} else if (limit) {
  styleFiles = styleFiles.slice(0, limit);
}

console.log(`Testing ${styleFiles.length} styles...`);

// Pre-build Rust binaries
console.log('Building Rust binaries...');
try {
  execSync('cargo build --release --bin csln_migrate --bin csln_processor', {
    cwd: projectRoot,
    stdio: 'inherit'
  });
} catch (e) {
  console.error('Failed to build Rust binaries');
  process.exit(1);
}

// Update exec commands to use release builds
const origExec = execSync;

// Run tests
const results = [];
const stats = { pass: 0, mismatch: 0, migration_error: 0, processor_error: 0, oracle_error: 0 };

for (let i = 0; i < styleFiles.length; i++) {
  const stylePath = styleFiles[i];
  const styleName = path.basename(stylePath, '.csl');
  
  if (verbose) {
    process.stdout.write(`[${i+1}/${styleFiles.length}] ${styleName}... `);
  } else if (i % 50 === 0) {
    process.stdout.write(`\rProgress: ${i}/${styleFiles.length}`);
  }
  
  const result = testStyle(stylePath);
  results.push(result);
  stats[result.status] = (stats[result.status] || 0) + 1;
  
  if (verbose) {
    console.log(result.status === 'pass' ? '✅' : '❌ ' + result.status);
  }
}

console.log(`\rProgress: ${styleFiles.length}/${styleFiles.length} - Done!`);

// Generate report
const report = {
  timestamp: new Date().toISOString(),
  totalStyles: styleFiles.length,
  summary: stats,
  passRate: ((stats.pass / styleFiles.length) * 100).toFixed(1) + '%',
  results: results
};

// Write report
fs.writeFileSync(outputFile, JSON.stringify(report, null, 2));
console.log(`\nReport written to ${outputFile}`);

// Print summary
console.log('\n=== SUMMARY ===');
console.log(`Total: ${report.totalStyles}`);
console.log(`Pass:  ${stats.pass} (${report.passRate})`);
console.log(`Mismatch: ${stats.mismatch}`);
console.log(`Migration errors: ${stats.migration_error}`);
console.log(`Processor errors: ${stats.processor_error}`);
console.log(`Oracle errors: ${stats.oracle_error}`);

// Show sample failures if any
const mismatches = results.filter(r => r.status === 'mismatch').slice(0, 3);
if (mismatches.length > 0) {
  console.log('\n=== SAMPLE MISMATCHES ===');
  for (const m of mismatches) {
    console.log(`\n${m.style}:`);
    if (m.samples?.citations?.[0]) {
      const s = m.samples.citations[0];
      console.log(`  Citation ${s.id}:`);
      console.log(`    Oracle: ${s.oracle}`);
      console.log(`    CSLN:   ${s.csln}`);
    }
  }
}

const migrationErrors = results.filter(r => r.status === 'migration_error').slice(0, 3);
if (migrationErrors.length > 0) {
  console.log('\n=== SAMPLE MIGRATION ERRORS ===');
  for (const e of migrationErrors) {
    console.log(`${e.style}: ${e.message}`);
  }
}
