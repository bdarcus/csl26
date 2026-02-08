#!/usr/bin/env node
/**
 * Output-Driven Template Inference Engine for CSL26
 *
 * Given a CSL 1.0 style file, this module:
 * 1. Renders all test fixture references through citeproc-js
 * 2. Extracts structured components from bibliography (and citations for note styles)
 * 3. Groups entries by reference type
 * 4. Builds consensus component ordering across all types
 * 5. Infers delimiters between adjacent components
 * 6. Detects type-specific suppress overrides
 * 7. Generates CSLN YAML template array
 *
 * Main export: inferTemplate(stylePath, section) → { template, yaml, meta }
 */

'use strict';

const CSL = require('citeproc');
const fs = require('fs');
const path = require('path');
const {
  normalizeText,
  parseComponents,
  analyzeOrdering,
  findRefDataForEntry,
  detectDelimiters,
} = require('./component-parser');

// -- Locale and fixture loading --

/**
 * Load CSL locale XML by language code.
 * Falls back to en-US if the requested locale is not found.
 */
function loadLocale(lang) {
  const localePath = path.join(__dirname, '..', `locales-${lang}.xml`);
  if (fs.existsSync(localePath)) {
    return fs.readFileSync(localePath, 'utf8');
  }
  const fallback = path.join(__dirname, '..', 'locales-en-US.xml');
  if (fs.existsSync(fallback)) {
    return fs.readFileSync(fallback, 'utf8');
  }
  throw new Error(`Locale not found: ${lang}`);
}

/**
 * Load test fixture items for analysis.
 * Filter out the comment field and return a map of ID → reference data.
 */
function loadFixtures() {
  const fixturesPath = path.join(__dirname, '..', '..', 'tests', 'fixtures', 'references-expanded.json');
  if (!fs.existsSync(fixturesPath)) {
    throw new Error(`Fixtures not found at ${fixturesPath}`);
  }
  const fixturesData = JSON.parse(fs.readFileSync(fixturesPath, 'utf8'));
  return Object.fromEntries(
    Object.entries(fixturesData).filter(([key]) => key !== 'comment')
  );
}

// -- Citeproc-js rendering --

/**
 * Render bibliography entries using citeproc-js.
 *
 * @param {string} styleXml - The CSL 1.0 style XML
 * @param {Object} testItems - Map of item IDs to CSL JSON reference data
 * @param {string} lang - Locale language code (default: 'en-US')
 * @returns {Object|null} { entries: Array<string>, style } or null if rendering fails
 */
function renderWithCiteproc(styleXml, testItems, lang = 'en-US') {
  try {
    const localeXml = loadLocale(lang);
    const engine = new CSL.Engine({
      retrieveLocale: () => localeXml,
      retrieveItem: (id) => testItems[id],
    }, styleXml);

    // Get all item IDs
    const itemIds = Object.keys(testItems);

    // Set bibliography for the engine
    engine.setOutputFormat('html');
    engine.updateItems(itemIds);

    // Get bibliography entries
    const bibResult = engine.makeBibliography();
    if (!bibResult || !bibResult[1]) {
      return null;
    }

    return {
      entries: bibResult[1],
      style: styleXml,
    };
  } catch (error) {
    console.error(`Failed to render with citeproc-js: ${error.message}`);
    return null;
  }
}

/**
 * Render note-style citations (for position tracking in note styles).
 * Returns array of citation strings indexed by item ID.
 */
function renderCitations(styleXml, testItems, lang = 'en-US') {
  try {
    const localeXml = loadLocale(lang);
    const engine = new CSL.Engine({
      retrieveLocale: () => localeXml,
      retrieveItem: (id) => testItems[id],
    }, styleXml);

    const itemIds = Object.keys(testItems);
    engine.setOutputFormat('html');
    engine.updateItems(itemIds);

    const citations = {};
    for (const id of itemIds) {
      try {
        const citationResult = engine.makeCitation([{ id }]);
        citations[id] = citationResult || '';
      } catch (e) {
        citations[id] = null;
      }
    }

    return citations;
  } catch (error) {
    console.error(`Failed to render citations: ${error.message}`);
    return null;
  }
}

// -- Component aggregation and ordering --

/**
 * Aggregate components across all entries of a single reference type.
 * Returns components found in any entry, plus their aggregate positions.
 */
function aggregateByType(entries, refByEntry) {
  const typedComponents = {};

  for (let idx = 0; idx < entries.length; idx++) {
    const entry = entries[idx];
    const refData = refByEntry[idx];

    // Parse components
    const comps = parseComponents(entry, refData);
    const type = refData?.type || 'unknown';

    if (!typedComponents[type]) {
      typedComponents[type] = {
        entries: [],
        componentCounts: {},
        componentInstances: {},
      };
    }

    typedComponents[type].entries.push(entry);

    // Track which components appear in this entry
    for (const [name, comp] of Object.entries(comps)) {
      if (name === 'raw') continue;
      if (comp.found) {
        typedComponents[type].componentCounts[name] =
          (typedComponents[type].componentCounts[name] || 0) + 1;

        if (!typedComponents[type].componentInstances[name]) {
          typedComponents[type].componentInstances[name] = [];
        }
        typedComponents[type].componentInstances[name].push(comp);
      }
    }
  }

  return typedComponents;
}

/**
 * Find the consensus component ordering across all reference types.
 *
 * Strategy: Build a merged ordering that includes ALL observed components.
 * Uses pairwise precedence voting — for each pair of components (A, B),
 * count how many entries have A before B vs B before A. The component
 * with more "before" votes goes first in the merged order.
 *
 * This handles the key problem: different reference types have different
 * component subsets (journals have containerTitle+volume; books have
 * publisher+place), but they all share the same relative ordering for
 * the components they have in common.
 */
function findConsensusOrdering(entries, refByEntry) {
  const typedComponents = aggregateByType(entries, refByEntry);

  // Collect all individual orderings across all entries
  const allOrderings = [];
  const orderingsByType = {};

  for (const [type, data] of Object.entries(typedComponents)) {
    const typeOrderings = [];
    for (const entry of data.entries) {
      const refData = refByEntry[entries.indexOf(entry)];
      const ordering = analyzeOrdering(entry, refData);
      allOrderings.push(ordering);
      typeOrderings.push(ordering);
    }
    // Most common ordering per type (for diagnostics)
    const counts = {};
    for (const o of typeOrderings) {
      const k = o.join(',');
      counts[k] = (counts[k] || 0) + 1;
    }
    const best = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    orderingsByType[type] = best[0]?.[0]?.split(',').filter(Boolean) || [];
  }

  // Collect all unique component names seen across all entries
  const allComponents = new Set();
  for (const ordering of allOrderings) {
    for (const comp of ordering) {
      if (comp) allComponents.add(comp);
    }
  }

  // Count how often each component appears across all entries
  const componentFrequency = {};
  for (const comp of allComponents) {
    componentFrequency[comp] = 0;
    for (const ordering of allOrderings) {
      if (ordering.includes(comp)) {
        componentFrequency[comp]++;
      }
    }
  }

  // Build pairwise precedence matrix: precedence[A][B] = count of A before B
  const precedence = {};
  for (const comp of allComponents) {
    precedence[comp] = {};
  }
  for (const ordering of allOrderings) {
    for (let i = 0; i < ordering.length; i++) {
      for (let j = i + 1; j < ordering.length; j++) {
        const a = ordering[i];
        const b = ordering[j];
        if (a && b) {
          precedence[a][b] = (precedence[a][b] || 0) + 1;
        }
      }
    }
  }

  // Sort components using pairwise votes as a comparator.
  // For sort(a, b): negative = a first, positive = b first.
  // If aBeforeB > bBeforeA, a should go first (return negative).
  const componentList = [...allComponents];
  componentList.sort((a, b) => {
    const aBeforeB = precedence[a]?.[b] || 0;
    const bBeforeA = precedence[b]?.[a] || 0;
    if (aBeforeB !== bBeforeA) {
      return aBeforeB > bBeforeA ? -1 : 1;
    }
    // Tie-break: more frequent components first
    return (componentFrequency[b] || 0) - (componentFrequency[a] || 0);
  });

  // Post-process: ensure issue immediately follows volume (they always
  // appear as a pair in citation styles, e.g. "volume(issue)")
  const volIdx = componentList.indexOf('volume');
  const issIdx = componentList.indexOf('issue');
  if (volIdx >= 0 && issIdx >= 0 && issIdx !== volIdx + 1) {
    componentList.splice(issIdx, 1);
    const newVolIdx = componentList.indexOf('volume');
    componentList.splice(newVolIdx + 1, 0, 'issue');
  }

  // Filter to components that appear in at least 10% of entries
  const minFrequency = Math.max(1, Math.floor(allOrderings.length * 0.1));
  const consensusOrdering = componentList.filter(
    comp => (componentFrequency[comp] || 0) >= minFrequency
  );

  return {
    consensusOrdering,
    orderingsByType,
    typedComponents,
    componentFrequency,
  };
}

// -- Delimiter detection and consensus --

/**
 * Find the most common delimiter between two component names.
 */
function findDelimiterConsensus(entries, refByEntry, comp1, comp2) {
  const delimiters = {};

  for (let idx = 0; idx < entries.length; idx++) {
    const entry = entries[idx];
    const refData = refByEntry[idx];
    const dets = detectDelimiters(entry, refData);

    for (const det of dets) {
      if (det.between[0] === comp1 && det.between[1] === comp2) {
        const delim = det.delimiter;
        delimiters[delim] = (delimiters[delim] || 0) + 1;
      }
    }
  }

  // Return most common delimiter
  if (Object.keys(delimiters).length === 0) {
    return null;
  }
  const sorted = Object.entries(delimiters).sort((a, b) => b[1] - a[1]);
  return sorted[0][0];
}

// -- CSLN component mapping --

/**
 * Map parser component names to CSLN template component objects.
 *
 * For container-title, we output BOTH parent-serial and parent-monograph
 * as separate components. The suppress override logic will ensure only
 * one renders per type. This matches the APA hand-authored approach.
 */
function mapComponentToYaml(componentName, entry, refData) {
  const comps = parseComponents(entry, refData);
  const comp = comps[componentName];

  if (!comp || !comp.found) {
    return null;
  }

  switch (componentName) {
    case 'contributors':
      return { contributor: 'author', form: 'long' };

    case 'year': {
      const obj = { date: 'issued', form: 'year' };
      // Check for parentheses: (2020)
      const yearParensMatch = normalizeText(entry).match(/\((\d{4})\)/);
      if (yearParensMatch) {
        obj.wrap = 'parentheses';
      }
      return obj;
    }

    case 'title':
      return { title: 'primary' };

    case 'containerTitle':
      // Default to parent-serial; type-specific overrides will handle
      // chapter/conference → parent-monograph via suppress logic
      return { title: 'parent-serial' };

    case 'volume':
      return { number: 'volume' };

    case 'issue':
      return { number: 'issue' };

    case 'pages':
      return { number: 'pages' };

    case 'publisher':
      return { variable: 'publisher' };

    case 'place':
      return { variable: 'publisher-place' };

    case 'doi':
      return { variable: 'doi' };

    case 'url':
      return { variable: 'url' };

    case 'edition':
      return { number: 'edition' };

    case 'editors':
      return { contributor: 'editor', form: 'verb' };

    default:
      return null;
  }
}

// -- Suppress override detection --

/**
 * Detect which components should have type-specific suppress overrides.
 *
 * Strategy: For each component, check which types have it and which don't.
 * Only generate overrides for the MINORITY case:
 * - If most types have it → suppress for the few that don't
 * - If few types have it → no overrides (handled by the component just
 *   not having data for those types, so it renders empty naturally)
 *
 * Skip the "unknown" pseudo-type (unrecognized CSL types).
 */
function detectSuppressions(consensusOrdering, typedComponents, componentFrequency) {
  const suppressions = {};
  const knownTypes = Object.keys(typedComponents).filter(t => t !== 'unknown');
  const totalTypes = knownTypes.length;

  for (const componentName of consensusOrdering) {
    suppressions[componentName] = {};

    // Count how many known types have this component in >50% of their entries
    let typesWithComponent = 0;
    const typesPresent = new Set();
    const typesMissing = new Set();

    for (const type of knownTypes) {
      const data = typedComponents[type];
      const count = data.componentCounts[componentName] || 0;
      const total = data.entries.length;

      if (count / total >= 0.5) {
        typesWithComponent++;
        typesPresent.add(type);
      } else {
        typesMissing.add(type);
      }
    }

    // Only generate suppress overrides if the component is present in
    // a clear majority of types but missing from a few specific ones
    const presentRatio = typesWithComponent / totalTypes;

    if (presentRatio >= 0.4 && typesMissing.size > 0 && typesMissing.size <= typesPresent.size) {
      // Suppress in the minority of types that lack it
      for (const type of typesMissing) {
        suppressions[componentName][type] = true;
      }
    }
    // If component is in the minority of types, no suppress overrides —
    // the processor handles missing data gracefully
  }

  return suppressions;
}

// -- YAML generation --

/**
 * Generate CSLN YAML template from component array and suppressions.
 */
function generateYaml(template) {
  let yaml = 'template:\n';
  const indent = '  ';

  for (const component of template) {
    // Find the main key (contributor, date, title, number, variable)
    const mainKey = Object.keys(component).find(
      k => !['form', 'wrap', 'overrides'].includes(k)
    );
    if (!mainKey) continue;

    yaml += `${indent}- ${mainKey}: ${component[mainKey]}\n`;

    if (component.form) {
      yaml += `${indent}  form: ${component.form}\n`;
    }
    if (component.wrap) {
      yaml += `${indent}  wrap: ${component.wrap}\n`;
    }

    // Add suppress overrides from component object
    if (component.overrides && Object.keys(component.overrides).length > 0) {
      yaml += `${indent}  overrides:\n`;
      for (const [type, override] of Object.entries(component.overrides)) {
        yaml += `${indent}    ${type}:\n`;
        for (const [key, val] of Object.entries(override)) {
          yaml += `${indent}      ${key}: ${val}\n`;
        }
      }
    }
  }

  return yaml;
}

// -- Main inference function --

/**
 * Infer CSLN template from a CSL 1.0 style file.
 *
 * @param {string} stylePath - Path to CSL 1.0 .csl file
 * @param {string} section - 'bibliography' (default) or 'citation'
 * @returns {Object|null} { template: Array, yaml: string, meta: Object } or null
 */
function inferTemplate(stylePath, section = 'bibliography') {
  // Validate inputs
  if (!fs.existsSync(stylePath)) {
    console.error(`Style file not found: ${stylePath}`);
    return null;
  }

  if (!['bibliography', 'citation'].includes(section)) {
    console.error(`Invalid section: ${section}. Use 'bibliography' or 'citation'.`);
    return null;
  }

  // Load fixtures and style
  let testItems;
  try {
    testItems = loadFixtures();
  } catch (error) {
    console.error(`Failed to load fixtures: ${error.message}`);
    return null;
  }

  const styleXml = fs.readFileSync(stylePath, 'utf8');

  // Render with citeproc-js
  let rendered;
  if (section === 'bibliography') {
    rendered = renderWithCiteproc(styleXml, testItems);
  } else {
    // For citations, use makeCitation instead
    const citations = renderCitations(styleXml, testItems);
    if (!citations) {
      console.error('Failed to render citations');
      return null;
    }
    // Convert to entries array (simpler handling)
    rendered = {
      entries: Object.values(citations).filter(c => c !== null),
      style: styleXml,
    };
  }

  if (!rendered || !rendered.entries || rendered.entries.length === 0) {
    console.error(`No entries rendered for section: ${section}`);
    return null;
  }

  // Build mapping from entry to reference data
  const refByEntry = rendered.entries.map(entry => {
    const refData = findRefDataForEntry(entry, testItems);
    return refData;
  });

  // Find consensus ordering
  const { consensusOrdering, typedComponents, componentFrequency } = findConsensusOrdering(
    rendered.entries,
    refByEntry
  );

  if (consensusOrdering.length === 0) {
    console.error('No component ordering could be established');
    return null;
  }

  // Build template array
  const template = [];
  for (const componentName of consensusOrdering) {
    // Use first entry that has this component to get the mapping
    const entryIdx = rendered.entries.findIndex((entry, idx) => {
      const comps = parseComponents(entry, refByEntry[idx]);
      return comps[componentName] && comps[componentName].found;
    });

    if (entryIdx >= 0) {
      const yamlComponent = mapComponentToYaml(
        componentName,
        rendered.entries[entryIdx],
        refByEntry[entryIdx]
      );

      if (yamlComponent) {
        // Track original component name for suppression logic
        yamlComponent._componentName = componentName;
        template.push(yamlComponent);
      }
    }
  }

  // Detect suppress overrides
  const suppressions = detectSuppressions(consensusOrdering, typedComponents, componentFrequency);

  // Attach overrides to template objects and clean up internal fields
  for (const comp of template) {
    const compSuppressions = suppressions[comp._componentName];
    if (compSuppressions && Object.keys(compSuppressions).length > 0) {
      comp.overrides = {};
      for (const [type] of Object.entries(compSuppressions)) {
        comp.overrides[type] = { suppress: true };
      }
    }
    delete comp._componentName;
  }

  // Generate YAML
  const yaml = generateYaml(template);

  // Calculate metadata
  const typesAnalyzed = Object.keys(typedComponents);
  const entriesPerType = Object.fromEntries(
    typesAnalyzed.map(type => [type, typedComponents[type].entries.length])
  );

  // Per-type confidence: for each type, what fraction of its expected
  // components were found in each entry? Average across all entries.
  // "Expected" = components that appear in >50% of entries for that type.
  const perTypeConfidence = {};
  let totalWeightedConfidence = 0;
  let totalEntries = 0;

  for (const [type, data] of Object.entries(typedComponents)) {
    if (type === 'unknown') continue;

    // Components expected for this type (present in >50% of its entries)
    const expectedComponents = [];
    for (const compName of consensusOrdering) {
      const count = data.componentCounts[compName] || 0;
      if (count / data.entries.length >= 0.5) {
        expectedComponents.push(compName);
      }
    }

    if (expectedComponents.length === 0) continue;

    // For each entry of this type, what fraction of expected components found?
    let typeTotal = 0;
    for (const entry of data.entries) {
      const idx = rendered.entries.indexOf(entry);
      const comps = parseComponents(entry, refByEntry[idx]);
      let found = 0;
      for (const compName of expectedComponents) {
        if (comps[compName]?.found) found++;
      }
      typeTotal += found / expectedComponents.length;
    }

    const typeConfidence = typeTotal / data.entries.length;
    perTypeConfidence[type] = {
      confidence: typeConfidence,
      expectedComponents: expectedComponents.length,
      entryCount: data.entries.length,
    };
    totalWeightedConfidence += typeConfidence * data.entries.length;
    totalEntries += data.entries.length;
  }

  const confidence = totalEntries > 0 ? totalWeightedConfidence / totalEntries : 0;

  // Find consensus delimiter
  let delimiterConsensus = '. '; // default
  if (consensusOrdering.length > 1) {
    const delim = findDelimiterConsensus(
      rendered.entries,
      refByEntry,
      consensusOrdering[0],
      consensusOrdering[1]
    );
    if (delim) {
      delimiterConsensus = delim;
    }
  }

  return {
    template,
    yaml,
    meta: {
      typesAnalyzed,
      entriesPerType,
      confidence,
      perTypeConfidence,
      delimiterConsensus,
      entryCount: rendered.entries.length,
      section,
    },
  };
}

module.exports = {
  inferTemplate,
  // Exported for testing
  loadLocale,
  loadFixtures,
  renderWithCiteproc,
  renderCitations,
  aggregateByType,
  findConsensusOrdering,
  findDelimiterConsensus,
  detectSuppressions,
  mapComponentToYaml,
  generateYaml,
};
