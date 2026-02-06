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
 *   node oracle-batch-aggregate.js ../styles/ --all --parallel 8
 *   node oracle-batch-aggregate.js ../styles/ --all --save corpus-results.json
 */

const { execSync, spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

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

/**
 * Run oracle for a single style (synchronous).
 */
function runStructuredOracle(stylePath) {
  const scriptPath = path.join(__dirname, 'oracle.js');
  
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

/**
 * Run oracle for a single style (async with promise).
 */
function runStructuredOracleAsync(stylePath) {
  return new Promise((resolve) => {
    const scriptPath = path.join(__dirname, 'oracle.js');
    const styleName = path.basename(stylePath, '.csl');
    
    const proc = spawn('node', [scriptPath, stylePath, '--json'], {
      stdio: ['pipe', 'pipe', 'pipe']
    });
    
    let stdout = '';
    let stderr = '';
    
    proc.stdout.on('data', (data) => { stdout += data; });
    proc.stderr.on('data', (data) => { stderr += data; });
    
    const timeout = setTimeout(() => {
      proc.kill();
      resolve({ error: 'timeout', style: styleName });
    }, 120000); // 2 minute timeout per style
    
    proc.on('close', () => {
      clearTimeout(timeout);
      try {
        resolve(JSON.parse(stdout));
      } catch {
        resolve({ error: stderr || 'parse error', style: styleName });
      }
    });
  });
}

/**
 * Run styles in parallel batches.
 */
async function runParallel(stylePaths, concurrency, onProgress) {
  const results = [];
  let completed = 0;
  
  // Process in batches
  for (let i = 0; i < stylePaths.length; i += concurrency) {
    const batch = stylePaths.slice(i, i + concurrency);
    const batchResults = await Promise.all(
      batch.map(stylePath => runStructuredOracleAsync(stylePath))
    );
    
    results.push(...batchResults);
    completed += batch.length;
    
    if (onProgress) {
      onProgress(completed, stylePaths.length, batchResults);
    }
  }
  
  return results;
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
const runAll = args.includes('--all');

// Get parallel concurrency
let concurrency = os.cpus().length; // Default to CPU count
const parallelArg = args.findIndex(a => a === '--parallel');
if (parallelArg >= 0 && args[parallelArg + 1]) {
  concurrency = parseInt(args[parallelArg + 1], 10);
}

// Get save path
let savePath = null;
const saveArg = args.findIndex(a => a === '--save');
if (saveArg >= 0 && args[saveArg + 1]) {
  savePath = args[saveArg + 1];
}

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
} else if (runAll) {
  // Get all .csl files in the styles directory (parent styles only)
  const files = fs.readdirSync(stylesDir)
    .filter(f => f.endsWith('.csl'))
    .map(f => path.join(stylesDir, f));
  stylesToTest = files;
} else {
  // Use priority styles, limited to topN
  for (const styleName of PRIORITY_STYLES.slice(0, topN)) {
    const stylePath = path.join(stylesDir, `${styleName}.csl`);
    if (fs.existsSync(stylePath)) {
      stylesToTest.push(stylePath);
    }
  }
}

// Main execution
async function main() {
  const startTime = Date.now();
  
  if (!jsonOutput) {
    console.log(`\n=== Batch Oracle Aggregator ===\n`);
    console.log(`Testing ${stylesToTest.length} styles...`);
    if (runAll) {
      console.log(`Parallel workers: ${concurrency}`);
      console.log(`Estimated time: ~${Math.ceil(stylesToTest.length * 1.2 / concurrency / 60)} minutes\n`);
    } else {
      console.log();
    }
  }

  let results;
  
  if (runAll || stylesToTest.length > 50) {
    // Use parallel execution for large batches
    results = await runParallel(stylesToTest, concurrency, (completed, total, batch) => {
      if (!jsonOutput) {
        const elapsed = ((Date.now() - startTime) / 1000).toFixed(0);
        const rate = (completed / elapsed).toFixed(1);
        const eta = Math.ceil((total - completed) / rate / 60);
        process.stdout.write(`\r[${completed}/${total}] ${rate}/s, ETA: ${eta}m    `);
      }
    });
    if (!jsonOutput) console.log('\n');
  } else {
    // Sequential for small batches (easier to debug)
    results = [];
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
  }

  // Aggregate results
  const summary = aggregateResults(results);
  
  // Add metadata
  summary.metadata = {
    timestamp: new Date().toISOString(),
    duration: ((Date.now() - startTime) / 1000).toFixed(1) + 's',
    concurrency: runAll ? concurrency : 1,
  };
  
  // Save to file if requested
  if (savePath) {
    fs.writeFileSync(savePath, JSON.stringify(summary, null, 2));
    if (!jsonOutput) {
      console.log(`Results saved to: ${savePath}`);
    }
  }

  // Output
  if (jsonOutput) {
    console.log(JSON.stringify(summary, null, 2));
  } else {
    console.log('\n=== SUMMARY ===\n');
    
    console.log(`Styles tested: ${summary.totalStyles}`);
    console.log(`Duration: ${summary.metadata.duration}`);
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
      for (const err of summary.errors.slice(0, 10)) {
        console.log(`  ${err.style}: ${err.error.substring(0, 60)}`);
      }
      if (summary.errors.length > 10) {
        console.log(`  ... and ${summary.errors.length - 10} more`);
      }
    }
    
    console.log();
  }
}

main().catch(e => {
  console.error('Error:', e);
  process.exit(1);
});
