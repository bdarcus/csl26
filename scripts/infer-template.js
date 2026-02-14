#!/usr/bin/env node
/**
 * CLI wrapper for the template inferrer module.
 *
 * Generates CSLN templates from CSL 1.0 styles using output-driven inference.
 *
 * Usage:
 *   node scripts/infer-template.js <style-path>
 *   node scripts/infer-template.js <style-path> --section=citation
 *   node scripts/infer-template.js <style-path> --json
 *   node scripts/infer-template.js <style-path> --fragment
 *   node scripts/infer-template.js <style-path> --verbose
 */

'use strict';

const fs = require('fs');
const path = require('path');
const { inferTemplate } = require('./lib/template-inferrer');

// Parse arguments
const args = process.argv.slice(2);

// Extract style path (first non-flag argument)
const stylePath = args.find(a => !a.startsWith('--'));

// Extract options
const section = args
  .find(a => a.startsWith('--section='))
  ?.split('=')[1] || 'bibliography';

const jsonOutput = args.includes('--json');
const fragmentOutput = args.includes('--fragment');
const verbose = args.includes('--verbose');

// Validate style path
if (!stylePath) {
  console.error('Usage: node scripts/infer-template.js <style-path> [--section=bibliography|citation] [--json] [--verbose]');
  process.exit(1);
}

if (!fs.existsSync(stylePath)) {
  console.error(`Error: Style file not found: ${stylePath}`);
  process.exit(2);
}

const styleName = path.basename(stylePath, '.csl');

// Run inference
if (!jsonOutput && !fragmentOutput) {
  console.error(`Inferring ${section} template for: ${styleName}`);
}

const result = inferTemplate(stylePath, section);

if (!result) {
  if (jsonOutput || fragmentOutput) {
    console.log(JSON.stringify({
      error: 'Template inference failed',
      reason: 'Unable to extract consistent component ordering',
      style: styleName
    }));
  } else {
    console.error('❌ Template Inference Failed\n');
    console.error(`Style: ${styleName}`);
    console.error('Reason: Unable to extract consistent component ordering from citeproc-js output\n');
    console.error('This typically means the style has:');
    console.error('  - Complex conditional logic (position-based rendering)');
    console.error('  - Inconsistent delimiter patterns across reference types');
    console.error('  - Heavy use of substitution rules\n');
    console.error('Next Steps:');
    console.error('  1. Use @dstyleplan for manual architecture design');
    console.error('  2. Check CSL source for complex <choose> blocks');
    console.error('  3. Report issue if this is a standard author-date/numeric style');
  }
  process.exit(2);
}

// Output
if (fragmentOutput) {
  // Compact JSON fragment for Rust template resolver consumption.
  // Outputs to stdout only, no stderr noise.
  console.log(JSON.stringify({
    meta: {
      style: styleName,
      confidence: result.meta.confidence,
      delimiter: result.meta.delimiterConsensus,
      entrySuffix: result.meta.entrySuffix,
    },
    bibliography: {
      template: result.template,
    },
  }));
  process.exit(0);
} else if (jsonOutput) {
  // Full result object as JSON
  console.log(JSON.stringify({
    style: styleName,
    section: result.meta.section,
    template: result.template,
    meta: result.meta,
  }, null, 2));
} else {
  // Human-readable YAML template
  console.log(`\n=== Template for: ${styleName} (${section}) ===\n`);
  console.log(result.yaml);

  // Summary metadata on stderr
  const { confidence, delimiterConsensus, entriesPerType, typesAnalyzed } = result.meta;

  console.error(`\nConfidence: ${(confidence * 100).toFixed(0)}% | Delimiter: "${delimiterConsensus}" | Types: ${typesAnalyzed.length} | Entries: ${result.meta.entryCount}`);

  if (verbose) {
    console.error('\n--- Per-Type Confidence ---');
    const ptc = result.meta.perTypeConfidence || {};
    for (const [type, data] of Object.entries(ptc)) {
      console.error(`  ${type}: ${(data.confidence * 100).toFixed(0)}% (${data.expectedComponents} components, ${data.entryCount} entries)`);
    }

    // Suppress overrides summary
    const suppressCount = result.template.filter(
      c => c.overrides && Object.keys(c.overrides).length > 0
    ).length;
    if (suppressCount > 0) {
      console.error(`\n--- Suppress Overrides ---`);
      console.error(`  ${suppressCount} component(s) with type-specific suppression`);
      for (const comp of result.template) {
        if (comp.overrides && Object.keys(comp.overrides).length > 0) {
          const types = Object.keys(comp.overrides).join(', ');
          const mainKey = Object.keys(comp).find(k => !['form', 'wrap', 'overrides'].includes(k));
          console.error(`    ${mainKey}=${comp[mainKey]}: suppress in [${types}]`);
        }
      }
    }
  }

  console.error();
}

process.exit(0);
