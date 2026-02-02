#!/usr/bin/env node
/**
 * Batch Oracle Aggregator
 * 
 * Runs structured oracle against multiple styles and aggregates
 * failure patterns to identify high-impact issues.
 * 
 * Usage:
 *   node oracle-batch-aggregate.js ../styles/ --top 20
 *   node oracle-batch-aggregate.js ../styles/ --top 50 --json
 *   node oracle-batch-aggregate.js ../styles/ --styles apa,ieee,nature
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// Priority parent styles (from STYLE_PRIORITY.md)
const PRIORITY_STYLES = [
  'apa',
  'elsevier-harvard',
  'elsevier-with-titles',
  'springer-basic-author-date',
  'ieee',
  'elsevier-vancouver',
  'american-medical-association',
  'nature',
  'cell',
  'chicago-author-date',
  'vancouver',
  'harvard-cite-them-right',
  'modern-language-association',
  'american-chemical-society',
  'american-sociological-association',
  'chicago-fullnote-bibliography',
  'turabian-fullnote-bibliography',
  'oscola',
  'bluebook-law-review',
  'american-political-science-association',
];

function runStructuredOracle(stylePath) {
  const scriptPath = path.join(__dirname, 'oracle-structured.js');
  
  try {
    const output = execSync(
      `node "${scriptPath}" "${stylePath}" --json`,
      { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'], timeout: 60000 }
    );
    return JSON.parse(output);
  } catch (e) {
    // Try to parse output even if exit code is non-zero
    if (e.stdout) {
      try {
        return JSON.parse(e.stdout);
      } catch {
        return { error: e.message, style: path.basename(stylePath, '.csl') };
      }
    }
    return { error: e.message, style: path.basename(stylePath, '.csl') };
  }
}

function aggregateResults(results) {
  const summary = {
    totalStyles: results.length,
    citationsPerfect: 0,
    bibliographyPerfect: 0,
    citationsPartial: 0,
    bibliographyPartial: 0,
    componentIssues: {},
    orderingIssues: 0,
    styleBreakdown: [],
    errors: [],
  };
  
  for (const result of results) {
    if (result.error) {
      summary.errors.push({ style: result.style, error: result.error });
      continue;
    }
    
    // Track citation success
    if (result.citations.passed === result.citations.total) {
      summary.citationsPerfect++;
    } else if (result.citations.passed > 0) {
      summary.citationsPartial++;
    }
    
    // Track bibliography success
    if (result.bibliography.passed === result.bibliography.total) {
      summary.bibliographyPerfect++;
    } else if (result.bibliography.passed > 0) {
      summary.bibliographyPartial++;
    }
    
    // Aggregate component issues
    if (result.componentSummary) {
      for (const [issue, count] of Object.entries(result.componentSummary)) {
        summary.componentIssues[issue] = (summary.componentIssues[issue] || 0) + count;
      }
    }
    
    // Track ordering issues
    if (result.orderingIssues) {
      summary.orderingIssues += result.orderingIssues;
    }
    
    // Style breakdown
    summary.styleBreakdown.push({
      style: result.style,
      citations: `${result.citations.passed}/${result.citations.total}`,
      bibliography: `${result.bibliography.passed}/${result.bibliography.total}`,
      citationsPct: Math.round((result.citations.passed / result.citations.total) * 100),
      bibliographyPct: Math.round((result.bibliography.passed / result.bibliography.total) * 100),
    });
  }
  
  // Sort style breakdown by bibliography success (ascending - worst first)
  summary.styleBreakdown.sort((a, b) => a.bibliographyPct - b.bibliographyPct);
  
  return summary;
}

// Parse arguments
const args = process.argv.slice(2);
const stylesDir = args.find(a => !a.startsWith('--')) || path.join(__dirname, '..', 'styles');
const jsonOutput = args.includes('--json');

// Get top N or specific styles
let topN = 20;
const topArg = args.findIndex(a => a === '--top');
if (topArg >= 0 && args[topArg + 1]) {
  topN = parseInt(args[topArg + 1], 10);
}

let specificStyles = null;
const stylesArg = args.findIndex(a => a === '--styles');
if (stylesArg >= 0 && args[stylesArg + 1]) {
  specificStyles = args[stylesArg + 1].split(',');
}

// Determine which styles to test
let stylesToTest = [];

if (specificStyles) {
  stylesToTest = specificStyles.map(s => path.join(stylesDir, `${s}.csl`));
} else {
  // Use priority styles, limited to topN
  for (const styleName of PRIORITY_STYLES.slice(0, topN)) {
    const stylePath = path.join(stylesDir, `${styleName}.csl`);
    if (fs.existsSync(stylePath)) {
      stylesToTest.push(stylePath);
    }
  }
}

if (!jsonOutput) {
  console.log(`\n=== Batch Oracle Aggregator ===\n`);
  console.log(`Testing ${stylesToTest.length} styles...\n`);
}

// Run oracle for each style
const results = [];
for (let i = 0; i < stylesToTest.length; i++) {
  const stylePath = stylesToTest[i];
  const styleName = path.basename(stylePath, '.csl');
  
  if (!jsonOutput) {
    process.stdout.write(`[${i + 1}/${stylesToTest.length}] ${styleName}... `);
  }
  
  const result = runStructuredOracle(stylePath);
  results.push(result);
  
  if (!jsonOutput) {
    if (result.error) {
      console.log(`ERROR`);
    } else {
      console.log(`C:${result.citations.passed}/${result.citations.total} B:${result.bibliography.passed}/${result.bibliography.total}`);
    }
  }
}

// Aggregate results
const summary = aggregateResults(results);

// Output
if (jsonOutput) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  console.log('\n=== SUMMARY ===\n');
  
  console.log(`Styles tested: ${summary.totalStyles}`);
  console.log(`Citations 100%: ${summary.citationsPerfect}/${summary.totalStyles} (${Math.round(summary.citationsPerfect / summary.totalStyles * 100)}%)`);
  console.log(`Bibliography 100%: ${summary.bibliographyPerfect}/${summary.totalStyles} (${Math.round(summary.bibliographyPerfect / summary.totalStyles * 100)}%)`);
  
  if (Object.keys(summary.componentIssues).length > 0) {
    console.log('\n--- TOP COMPONENT ISSUES ---');
    const sorted = Object.entries(summary.componentIssues)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10);
    for (const [issue, count] of sorted) {
      console.log(`  ${issue}: ${count} occurrences`);
    }
  }
  
  if (summary.orderingIssues > 0) {
    console.log(`\n--- ORDERING ISSUES: ${summary.orderingIssues} total ---`);
  }
  
  console.log('\n--- STYLE BREAKDOWN (worst first) ---');
  console.log('Style                          | Citations | Bibliography');
  console.log('-------------------------------|-----------|-------------');
  for (const s of summary.styleBreakdown.slice(0, 15)) {
    const name = s.style.padEnd(30);
    const cit = s.citations.padStart(9);
    const bib = s.bibliography.padStart(12);
    console.log(`${name} | ${cit} | ${bib}`);
  }
  
  if (summary.errors.length > 0) {
    console.log('\n--- ERRORS ---');
    for (const err of summary.errors) {
      console.log(`  ${err.style}: ${err.error.substring(0, 80)}`);
    }
  }
  
  console.log();
}
