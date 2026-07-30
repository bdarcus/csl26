#!/usr/bin/env node
/**
 * Style overlap measurement for a candidate shared-parent refactor.
 *
 * Question: do a set of sibling "-core" styles (currently standalone,
 * hand-tuned embedded styles that share a publisher/family but not an
 * `extends` parent) actually share enough structure to justify factoring
 * out a common base?
 *
 * This is deliberately NOT `measure-delta-derivability.js`: that script
 * migrates *legacy CSL* through citum-migrate with `--minimize-wrapper` and
 * measures what the *converter* can synthesize as an `extends` wrapper. It
 * answers "how good is the converter," not "do these hand-tuned artifacts
 * share extractable structure" -- which is the question here, and the two
 * are not interchangeable.
 *
 * Method, per pair of styles in a family:
 *
 *   1. Option overlap -- flatten every scalar `options.*` key-path (skipping
 *      `info` and the component-list keys `template`/`type-templates`/
 *      `type-variants`), then report the fraction of key-paths present in
 *      both styles with an identical value. This is the metric that matters
 *      most for a shared-parent proposal, because `extends` deep-merges
 *      options field-level (see docs/specs/STYLE_INHERITANCE.md and commits
 *      303a38f0, 85e94e81) -- so overlapping options are exactly what a
 *      parent could hoist.
 *   2. Component overlap -- collect every component object anywhere in the
 *      style (template, type-templates, type-variants), normalize each to a
 *      key-sorted JSON string, and compare as sets (order-independent, so
 *      an inserted component in one style does not zero the score the way
 *      an index-anchored comparison would).
 *   3. Preset attribution -- for every option key-path shared with an
 *      identical value, check whether an existing preset in
 *      crates/citum-schema-style/src/presets.rs already expresses it (the
 *      preset enum variant names are kebab-case and match the option string
 *      value directly, e.g. `options.contributors: vancouver` ==
 *      `ContributorPreset::Vancouver`). Overlap that a preset already covers
 *      argues for using that preset, not synthesizing a new shared parent --
 *      without this split, a headline overlap number is uninterpretable.
 *
 * This script does not refactor anything. It writes a TSV report; the
 * accompanying audit doc records the recommendation. See
 * docs/architecture/audits/ and citum-core plan/bean csl26-edjj for the
 * refactor gate (fidelity flat-or-better, SQI up, check-core-quality holds)
 * that must pass before any of this becomes a code change.
 *
 * Usage:
 *   node scripts/measure-style-overlap.js --family elsevier --family taylor-and-francis
 *   node scripts/measure-style-overlap.js --styles a-core,b-core,c-core --label custom
 *   node scripts/measure-style-overlap.js --out scripts/report-data/style-overlap-2026-07-30.tsv
 */

'use strict';

const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');

const WORKSPACE_ROOT = path.resolve(__dirname, '..');
const EMBEDDED_STYLES_DIR = path.join(WORKSPACE_ROOT, 'styles', 'embedded');
const PRESETS_RS_PATH = path.join(
  WORKSPACE_ROOT,
  'crates',
  'citum-schema-style',
  'src',
  'presets.rs'
);

const KNOWN_FAMILIES = {
  elsevier: [
    'elsevier-harvard-core',
    'elsevier-vancouver-core',
    'elsevier-with-titles-core',
  ],
  'taylor-and-francis': [
    'taylor-and-francis-chicago-author-date-core',
    'taylor-and-francis-council-of-science-editors-author-date-core',
    'taylor-and-francis-national-library-of-medicine-core',
  ],
};

// Option key-path leaf names whose scalar string values are preset-name
// candidates (mirrors the `options.<field>: <preset-name>` shorthand
// documented in presets.rs).
const PRESET_OPTION_FIELDS = new Set([
  'contributors',
  'dates',
  'titles',
  'sort',
  'substitute',
  'multilingual',
]);

function parseArgs(argv) {
  const families = [];
  let styles = null;
  let label = null;
  let out = null;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--family') {
      families.push(argv[++i]);
    } else if (arg === '--styles') {
      styles = argv[++i].split(',').map((s) => s.trim()).filter(Boolean);
    } else if (arg === '--label') {
      label = argv[++i];
    } else if (arg === '--out') {
      out = argv[++i];
    }
  }
  return { families, styles, label, out };
}

function readYaml(filePath) {
  return yaml.load(fs.readFileSync(filePath, 'utf8')) || {};
}

/** Load known preset name sets by scanning presets.rs enum variant names. */
function loadKnownPresetNames() {
  const text = fs.readFileSync(PRESETS_RS_PATH, 'utf8');
  const names = new Set();
  // Match `pub enum X { ... }` blocks and pull bare identifier variants
  // (ignores doc comments, #[...] attributes, and variants carrying data).
  const enumBlockRe = /pub enum \w+\s*\{([\s\S]*?)\n\}/g;
  let enumMatch;
  while ((enumMatch = enumBlockRe.exec(text)) !== null) {
    const body = enumMatch[1];
    const variantRe = /^\s{4}([A-Z][A-Za-z0-9]*),\s*$/gm;
    let variantMatch;
    while ((variantMatch = variantRe.exec(body)) !== null) {
      const pascalName = variantMatch[1];
      const kebabName = pascalName
        .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
        .toLowerCase();
      names.add(kebabName);
    }
  }
  return names;
}

/**
 * Flatten scalar `options.*` key-paths only. Skips `info` (provenance, not
 * behavior) and the component-list keys, whose contents are compared
 * separately by `collectComponents`.
 */
function flattenOptionPaths(styleData) {
  const out = {};
  const options = styleData.options;
  if (!options || typeof options !== 'object') return out;

  const visit = (value, pathPrefix) => {
    if (value === null || typeof value !== 'object') {
      out[pathPrefix] = JSON.stringify(value);
      return;
    }
    if (Array.isArray(value)) return; // arrays under options are not scalar leaves we compare
    for (const [key, child] of Object.entries(value)) {
      visit(child, pathPrefix ? `${pathPrefix}.${key}` : key);
    }
  };
  visit(options, 'options');

  // bibliography.options / citation.options follow the same nested-object shape.
  for (const scope of ['bibliography', 'citation']) {
    const scopeOptions = styleData[scope] && styleData[scope].options;
    if (scopeOptions && typeof scopeOptions === 'object') {
      visit(scopeOptions, `${scope}.options`);
    }
  }

  return out;
}

/**
 * Collect every component object anywhere under template/type-templates/
 * type-variants, normalized to a key-sorted JSON string so comparison is
 * order-independent.
 */
function collectComponents(styleData) {
  const components = [];
  const visit = (value) => {
    if (Array.isArray(value)) {
      for (const item of value) {
        if (item && typeof item === 'object' && !Array.isArray(item)) {
          components.push(JSON.stringify(item, Object.keys(item).sort()));
        }
        visit(item);
      }
      return;
    }
    if (value && typeof value === 'object') {
      for (const child of Object.values(value)) visit(child);
    }
  };
  for (const key of ['template', 'type-templates', 'type-variants']) {
    if (styleData[key] !== undefined) visit(styleData[key]);
    if (styleData.bibliography && styleData.bibliography[key] !== undefined) {
      visit(styleData.bibliography[key]);
    }
    if (styleData.citation && styleData.citation[key] !== undefined) {
      visit(styleData.citation[key]);
    }
  }
  return components;
}

function jaccard(setA, setB) {
  let intersection = 0;
  for (const item of setA) if (setB.has(item)) intersection++;
  const union = new Set([...setA, ...setB]).size;
  return { intersection, union, score: union === 0 ? 0 : intersection / union };
}

function sharedIdenticalPaths(optsA, optsB) {
  const keys = new Set([...Object.keys(optsA), ...Object.keys(optsB)]);
  const shared = [];
  for (const key of keys) {
    if (optsA[key] !== undefined && optsA[key] === optsB[key]) shared.push(key);
  }
  return { keys, shared };
}

function isPresetExpressible(pathKey, value, knownPresetNames) {
  const leaf = pathKey.split('.').pop();
  if (!PRESET_OPTION_FIELDS.has(leaf)) return false;
  let parsed;
  try {
    parsed = JSON.parse(value);
  } catch {
    return false;
  }
  return typeof parsed === 'string' && knownPresetNames.has(parsed);
}

function measurePair(nameA, dataA, nameB, dataB, knownPresetNames) {
  const optsA = flattenOptionPaths(dataA);
  const optsB = flattenOptionPaths(dataB);
  const { keys, shared } = sharedIdenticalPaths(optsA, optsB);

  const presetExpressible = shared.filter((key) =>
    isPresetExpressible(key, optsA[key], knownPresetNames)
  );

  const componentsA = new Set(collectComponents(dataA));
  const componentsB = new Set(collectComponents(dataB));
  const componentOverlap = jaccard(componentsA, componentsB);

  return {
    styleA: nameA,
    styleB: nameB,
    optionOverlapPct: keys.size === 0 ? 0 : (shared.length / keys.size) * 100,
    optionSharedCount: shared.length,
    optionTotalCount: keys.size,
    componentOverlapPct: componentOverlap.score * 100,
    componentSharedCount: componentOverlap.intersection,
    componentTotalCount: componentOverlap.union,
    presetExpressibleCount: presetExpressible.length,
    presetExpressiblePaths: presetExpressible,
    nonPresetSharedPaths: shared.filter((key) => !presetExpressible.includes(key)),
  };
}

function measureFamily(label, styleNames, knownPresetNames) {
  const loaded = styleNames.map((name) => {
    const filePath = path.join(EMBEDDED_STYLES_DIR, `${name}.yaml`);
    if (!fs.existsSync(filePath)) {
      throw new Error(`Style not found for family "${label}": ${filePath}`);
    }
    return [name, readYaml(filePath)];
  });

  const pairs = [];
  for (let i = 0; i < loaded.length; i++) {
    for (let j = i + 1; j < loaded.length; j++) {
      const [nameA, dataA] = loaded[i];
      const [nameB, dataB] = loaded[j];
      pairs.push(measurePair(nameA, dataA, nameB, dataB, knownPresetNames));
    }
  }

  // Three-way (or N-way) common shared option paths: present with an
  // identical value across every member of the family, not just a pair.
  const allOptions = loaded.map(([, data]) => flattenOptionPaths(data));
  const commonPaths = Object.keys(allOptions[0]).filter((key) =>
    allOptions.every((opts) => opts[key] === allOptions[0][key])
  );
  const commonPresetExpressible = commonPaths.filter((key) =>
    isPresetExpressible(key, allOptions[0][key], knownPresetNames)
  );

  return { label, members: styleNames, pairs, commonPaths, commonPresetExpressible };
}

function writeTsv(outPath, familyResults) {
  const header = [
    'family',
    'style_a',
    'style_b',
    'option_overlap_pct',
    'option_shared',
    'option_total',
    'component_overlap_pct',
    'component_shared',
    'component_total',
    'preset_expressible_shared',
    'non_preset_shared_paths',
  ].join('\t');
  const lines = [header];
  for (const family of familyResults) {
    for (const pair of family.pairs) {
      lines.push(
        [
          family.label,
          pair.styleA,
          pair.styleB,
          pair.optionOverlapPct.toFixed(1),
          pair.optionSharedCount,
          pair.optionTotalCount,
          pair.componentOverlapPct.toFixed(1),
          pair.componentSharedCount,
          pair.componentTotalCount,
          pair.presetExpressibleCount,
          pair.nonPresetSharedPaths.join(';') || '(none)',
        ].join('\t')
      );
    }
  }
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, lines.join('\n') + '\n');
}

function printSummary(familyResults) {
  for (const family of familyResults) {
    console.log(`\n### ${family.label} (${family.members.join(', ')})`);
    for (const pair of family.pairs) {
      console.log(
        `  opts ${pair.optionOverlapPct.toFixed(1)}% (${pair.optionSharedCount}/${pair.optionTotalCount})` +
          `   components ${pair.componentOverlapPct.toFixed(1)}% (${pair.componentSharedCount}/${pair.componentTotalCount})` +
          `   preset-expressible: ${pair.presetExpressibleCount}` +
          `   ${pair.styleA} vs ${pair.styleB}`
      );
    }
    console.log(
      `  ${family.members.length}-way common option paths: ${family.commonPaths.length}` +
        ` (${family.commonPresetExpressible.length} already preset-expressible)`
    );
    const nonPresetCommon = family.commonPaths.filter(
      (p) => !family.commonPresetExpressible.includes(p)
    );
    if (nonPresetCommon.length > 0) {
      console.log(`  non-preset common paths: ${nonPresetCommon.join(', ')}`);
    }
  }
}

function main() {
  const { families, styles, label, out } = parseArgs(process.argv.slice(2));
  const knownPresetNames = loadKnownPresetNames();

  const familyResults = [];
  if (styles) {
    familyResults.push(measureFamily(label || 'custom', styles, knownPresetNames));
  } else {
    const selected = families.length > 0 ? families : Object.keys(KNOWN_FAMILIES);
    for (const familyName of selected) {
      const members = KNOWN_FAMILIES[familyName];
      if (!members) {
        throw new Error(
          `Unknown family "${familyName}". Known families: ${Object.keys(KNOWN_FAMILIES).join(', ')}. ` +
            'Use --styles for an ad hoc set.'
        );
      }
      familyResults.push(measureFamily(familyName, members, knownPresetNames));
    }
  }

  printSummary(familyResults);

  if (out) {
    writeTsv(path.resolve(WORKSPACE_ROOT, out), familyResults);
    console.log(`\nWrote ${out}`);
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  loadKnownPresetNames,
  flattenOptionPaths,
  collectComponents,
  measurePair,
  measureFamily,
};
