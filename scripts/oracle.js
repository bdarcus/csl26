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
 */

const CSL = require('citeproc');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Load locale from file
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

// Load test items from JSON fixture
const fixturesPath = path.join(__dirname, '..', 'tests', 'fixtures', 'references-expanded.json');
const fixturesData = JSON.parse(fs.readFileSync(fixturesPath, 'utf8'));
const testItems = Object.fromEntries(
  Object.entries(fixturesData).filter(([key]) => key !== 'comment')
);

/**
 * Component patterns for parsing bibliography entries.
 * These patterns identify common bibliographic components.
 */
const COMPONENT_PATTERNS = {
  // Year in parentheses: (2020) or (2020).
  yearParens: /\((\d{4})\)\.?/,
  // Year standalone: 2020. or 2020,
  yearStandalone: /(?:^|\s)(\d{4})[.,]/,
  // DOI patterns
  doi: /(?:https?:\/\/doi\.org\/|doi:\s*|DOI:\s*)(10\.\d+\/[^\s]+)/i,
  // URL patterns
  url: /(?:URL\s*|Available at\s*)?https?:\/\/[^\s]+/i,
  // Page ranges: pp. 123-456, p. 123, 123-456, 123–456
  pages: /(?:pp?\.\s*)?(\d+)[\-–](\d+)/,
  // Single page
  page: /(?:p\.\s*)?(\d+)(?![0-9\-–])/,
  // Volume/issue: 15(3), vol. 15, no. 3
  volumeIssue: /(?:vol(?:ume)?\.?\s*)?(\d+)\s*\((\d+)\)/i,
  // Volume only: 521, vol. 15
  volume: /(?:vol(?:ume)?\.?\s*)?(\d+)(?!\s*\()/,
  // Edition: 2nd ed., Silver Anniversary Edition
  edition: /(\d+(?:st|nd|rd|th)\s+ed\.|[A-Za-z\s]+Edition)/i,
  // Editor markers: (Ed.), (Eds.), edited by
  editors: /\(Eds?\.\)|edited by|Ed\.|Eds\./i,
  // "In" prefix for chapters
  inPrefix: /\bIn[:\s]/,
  // Publisher-place pattern: City: Publisher or Publisher, City
  publisherPlace: /([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\s*:\s*([^,.]+)|([^,.]+),\s*([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)/,
};

/**
 * Extract structured components from a bibliography entry string.
 */
function parseComponents(entry, refData) {
  const components = {
    raw: entry,
    contributors: { found: false, value: null },
    year: { found: false, value: null },
    title: { found: false, value: null },
    containerTitle: { found: false, value: null },
    volume: { found: false, value: null },
    issue: { found: false, value: null },
    pages: { found: false, value: null },
    publisher: { found: false, value: null },
    place: { found: false, value: null },
    doi: { found: false, value: null },
    url: { found: false, value: null },
    edition: { found: false, value: null },
    editors: { found: false, value: null },
  };
  
  const normalized = normalizeText(entry);
  
  // Extract year
  const yearMatch = normalized.match(COMPONENT_PATTERNS.yearParens) || 
                    normalized.match(COMPONENT_PATTERNS.yearStandalone);
  if (yearMatch) {
    components.year = { found: true, value: yearMatch[1] };
  }
  
  // Extract DOI
  const doiMatch = normalized.match(COMPONENT_PATTERNS.doi);
  if (doiMatch) {
    components.doi = { found: true, value: doiMatch[1] };
  }
  
  // Extract URL (if no DOI)
  if (!components.doi.found) {
    const urlMatch = normalized.match(COMPONENT_PATTERNS.url);
    if (urlMatch) {
      components.url = { found: true, value: urlMatch[0] };
    }
  }
  
  // Extract pages
  const pagesMatch = normalized.match(COMPONENT_PATTERNS.pages);
  if (pagesMatch) {
    components.pages = { found: true, value: `${pagesMatch[1]}-${pagesMatch[2]}` };
  }
  
  // Extract volume/issue
  const volIssueMatch = normalized.match(COMPONENT_PATTERNS.volumeIssue);
  if (volIssueMatch) {
    components.volume = { found: true, value: volIssueMatch[1] };
    components.issue = { found: true, value: volIssueMatch[2] };
  } else {
    // Try volume only if we have reference data
    if (refData && refData.volume) {
      if (normalized.includes(refData.volume)) {
        components.volume = { found: true, value: refData.volume };
      }
    }
    if (refData && refData.issue) {
      if (normalized.includes(refData.issue)) {
        components.issue = { found: true, value: refData.issue };
      }
    }
  }
  
  // Extract edition
  const editionMatch = normalized.match(COMPONENT_PATTERNS.edition);
  if (editionMatch) {
    components.edition = { found: true, value: editionMatch[1] };
  }
  
  // Check for editor markers
  if (COMPONENT_PATTERNS.editors.test(normalized)) {
    components.editors = { found: true, value: true };
  }
  
  // Use reference data to verify presence of specific fields
  if (refData) {
    // Check title presence (normalize both for comparison)
    if (refData.title) {
      const titleNorm = normalizeText(refData.title).toLowerCase();
      const entryNorm = normalized.toLowerCase();
      components.title = { 
        found: entryNorm.includes(titleNorm.substring(0, 20)), 
        value: refData.title 
      };
    }
    
    // Check container-title presence
    if (refData['container-title']) {
      const containerNorm = normalizeText(refData['container-title']).toLowerCase();
      const entryNorm = normalized.toLowerCase();
      components.containerTitle = { 
        found: entryNorm.includes(containerNorm.substring(0, 15)), 
        value: refData['container-title'] 
      };
    }
    
    // Check publisher
    if (refData.publisher) {
      const pubNorm = normalizeText(refData.publisher).toLowerCase();
      const entryNorm = normalized.toLowerCase();
      components.publisher = { 
        found: entryNorm.includes(pubNorm.substring(0, 10)), 
        value: refData.publisher 
      };
    }
    
    // Check contributors (just verify author family names appear)
    if (refData.author && refData.author.length > 0) {
      const firstAuthor = refData.author[0];
      const authorName = firstAuthor.family || firstAuthor.literal || '';
      components.contributors = {
        found: normalized.toLowerCase().includes(authorName.toLowerCase()),
        value: authorName
      };
    }
  }
  
  return components;
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
 * Analyze ordering of components in the entry.
 * Returns the order in which key components appear.
 */
function analyzeOrdering(entry, refData) {
  const normalized = normalizeText(entry).toLowerCase();
  const positions = {};
  
  // Find positions of key components
  if (refData) {
    if (refData.author && refData.author[0]) {
      const name = (refData.author[0].family || refData.author[0].literal || '').toLowerCase();
      if (name) positions.contributors = normalized.indexOf(name);
    }
    
    if (refData.issued && refData.issued['date-parts']) {
      const year = String(refData.issued['date-parts'][0][0]);
      positions.year = normalized.indexOf(year);
    }
    
    if (refData.title) {
      const title = refData.title.substring(0, 15).toLowerCase();
      positions.title = normalized.indexOf(title);
    }
    
    if (refData['container-title']) {
      const container = refData['container-title'].substring(0, 10).toLowerCase();
      positions.containerTitle = normalized.indexOf(container);
    }
    
    if (refData.volume) {
      // Find volume number (be careful not to match within other numbers)
      const volRegex = new RegExp(`\\b${refData.volume}\\b`);
      const match = normalized.match(volRegex);
      if (match) positions.volume = match.index;
    }
    
    if (refData.page) {
      const pageStart = refData.page.split(/[-–]/)[0];
      positions.pages = normalized.indexOf(pageStart);
    }
  }
  
  // Filter out -1 (not found) and sort by position
  const found = Object.entries(positions)
    .filter(([_, pos]) => pos >= 0)
    .sort((a, b) => a[1] - b[1])
    .map(([key, _]) => key);
  
  return found;
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

function normalizeText(text) {
  return text
    .replace(/<[^>]+>/g, '')       // Strip HTML tags
    .replace(/&#38;/g, '&')        // HTML entity for &
    .replace(/_([^_]+)_/g, '$1')   // Strip markdown italics
    .replace(/\*\*([^*]+)\*\*/g, '$1') // Strip markdown bold
    .replace(/\s+/g, ' ')          // Normalize whitespace
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
  const projectRoot = path.resolve(__dirname, '..');
  const absStylePath = path.resolve(stylePath);
  
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
  
  const tempFile = path.join(projectRoot, '.migrated-temp.yaml');
  fs.writeFileSync(tempFile, migratedYaml);
  
  let output;
  try {
    output = execSync(
      `cargo run -q --bin csln_processor -- .migrated-temp.yaml`,
      { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
  } catch (e) {
    console.error('Processor failed:', e.stderr || e.message);
    try { fs.unlinkSync(tempFile); } catch {}
    return null;
  }

  try { fs.unlinkSync(tempFile); } catch {}
  
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

/**
 * Find the reference data for a bibliography entry by matching author/title.
 */
function findRefDataForEntry(entry, testItems) {
  const entryNorm = normalizeText(entry).toLowerCase();
  
  for (const [id, ref] of Object.entries(testItems)) {
    // Try to match by author family name
    if (ref.author && ref.author[0]) {
      const authorName = (ref.author[0].family || ref.author[0].literal || '').toLowerCase();
      if (authorName && entryNorm.includes(authorName)) {
        // Verify with title too
        if (ref.title) {
          const titleStart = ref.title.substring(0, 15).toLowerCase();
          if (entryNorm.includes(titleStart)) {
            return ref;
          }
        }
        // If title not in entry but author matches, still return
        return ref;
      }
    }
  }
  
  return null;
}

// Main
const args = process.argv.slice(2);
const stylePath = args.find(a => !a.startsWith('--')) || path.join(__dirname, '..', 'styles', 'apa.csl');
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
    console.log(JSON.stringify({ error: 'CSLN rendering failed' }));
  } else {
    console.log('\n❌ CSLN rendering failed\n');
  }
  process.exit(1);
}

// Analyze bibliography
const pairs = matchBibliographyEntries(oracle.bibliography, csln.bibliography);

const results = {
  style: styleName,
  citations: {
    total: Object.keys(testItems).length,
    passed: 0,
    failed: 0,
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
  if (oracleCit === cslnCit) {
    results.citations.passed++;
  } else {
    results.citations.failed++;
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
    
    if (oracleNorm === cslnNorm) {
      entryResult.match = true;
      results.bibliography.passed++;
    } else {
      results.bibliography.failed++;
      
      // Find reference data for this entry
      const refData = findRefDataForEntry(pair.oracle, testItems);
      
      // Parse components
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
