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
const verbose = args.includes('--verbose');

// Validate style path
if (!stylePath) {
  console.error('Usage: node scripts/infer-template.js <style-path> [--section=bibliography|citation] [--json] [--verbose]');
  process.exit(1);
}

if (!fs.existsSync(stylePath)) {
  console.error(`Error: Style file not found: ${stylePath}`);
  process.exit(1);
}

const styleName = path.basename(stylePath, '.csl');

// Run inference
if (!jsonOutput) {
  console.error(`Inferring ${section} template for: ${styleName}`);
}

const result = inferTemplate(stylePath, section);

if (!result) {
  if (jsonOutput) {
    console.log(JSON.stringify({ error: 'Template inference failed' }));
  } else {
    console.error(`\nError: Failed to infer template for ${styleName}`);
  }
  process.exit(1);
}

// Output
if (jsonOutput) {
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
    console.error('\n--- Per-Type Entry Counts ---');
    for (const [type, count] of Object.entries(entriesPerType)) {
      console.error(`  ${type}: ${count} entries`);
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
          const mainKey = Object.keys(comp).find(k => !k.startsWith('_'));
          console.error(`    ${comp._componentName} (${mainKey}): suppress in [${types}]`);
        }
      }
    }
  }

  console.error();
}

process.exit(0);
