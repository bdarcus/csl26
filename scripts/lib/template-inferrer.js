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
function findDelimiterConsensus(entries, refByEntry, comp1, comp2, minFraction) {
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

  // Return most common delimiter, trimming stray quotes from formatting.
  // Require the pair to appear in at least minFraction of entries (default 0)
  // to avoid letting rare type-specific pairs set prefixes.
  if (Object.keys(delimiters).length === 0) {
    return null;
  }
  const sorted = Object.entries(delimiters).sort((a, b) => b[1] - a[1]);
  const minCount = Math.max(1, Math.floor(entries.length * (minFraction || 0)));
  if (sorted[0][1] < minCount) return null;
  return sorted[0][0].replace(/["\u201c\u201d]+/g, '') || sorted[0][0];
}

// -- Prefix/suffix and wrap detection --

/**
 * Detect common prefix patterns for a component across multiple entries.
 *
 * The component parser's position often INCLUDES the prefix (e.g. position
 * covers "https://doi.org/10.xxx" or "pp. 123-456"). So we check two zones:
 * 1. Text INSIDE the matched position before the core value
 * 2. Text BEFORE the matched position (for "In " before editor names)
 *
 * Returns the prefix string if >50% of entries share it, otherwise null.
 */
function detectPrefix(componentName, entries, refByEntry) {
  const prefixes = {};
  let total = 0;

  for (let idx = 0; idx < entries.length; idx++) {
    const comps = parseComponents(entries[idx], refByEntry[idx]);
    const comp = comps[componentName];
    if (!comp?.found || !comp.position) continue;

    const normalized = normalizeText(entries[idx]);
    total++;

    // Zone 1: text inside the match (position.start to value start)
    const matchText = normalized.slice(comp.position.start, comp.position.end);

    // Zone 2: text before the match (up to 20 chars)
    const beforeStart = Math.max(0, comp.position.start - 20);
    const before = normalized.slice(beforeStart, comp.position.start);

    // DOI: "https://doi.org/" is inside the match
    if (/^https?:\/\/doi\.org\//i.test(matchText)) {
      prefixes['https://doi.org/'] = (prefixes['https://doi.org/'] || 0) + 1;
    }
    // Pages: "pp." or "p." is inside the match
    else if (/^pp?\.\s*/i.test(matchText)) {
      prefixes['pp. '] = (prefixes['pp. '] || 0) + 1;
    }
    // Editors: "In " appears before the editor name/marker
    else if (/In\s+$/.test(before)) {
      prefixes['In '] = (prefixes['In '] || 0) + 1;
    }
  }

  if (total === 0) return null;
  const sorted = Object.entries(prefixes).sort((a, b) => b[1] - a[1]);
  if (sorted.length > 0 && sorted[0][1] / total >= 0.5) {
    return sorted[0][0];
  }
  return null;
}

/**
 * Detect if volume and issue appear as a grouped pattern like "12(3)"
 * with no separator between them. Returns true if >50% of entries
 * with both volume and issue show this pattern.
 */
function detectVolumeIssueGrouping(entries, refByEntry) {
  let withBoth = 0;
  let grouped = 0;

  for (let idx = 0; idx < entries.length; idx++) {
    const comps = parseComponents(entries[idx], refByEntry[idx]);
    if (!comps.volume?.found || !comps.issue?.found) continue;
    if (!comps.volume.position || !comps.issue.position) continue;

    withBoth++;
    const normalized = normalizeText(entries[idx]);
    // Check text between volume end and issue start
    const between = normalized.slice(
      comps.volume.position.end,
      comps.issue.position.start
    );

    // Grouped if only "(" or "(" with optional space between them
    if (/^\s*\(?\s*$/.test(between)) {
      grouped++;
    }
  }

  return withBoth > 0 && grouped / withBoth >= 0.5;
}

/**
 * Detect wrap pattern (parentheses/brackets) around a component.
 * Returns 'parentheses' or 'brackets' if >50% of entries show the pattern.
 */
function detectWrap(componentName, entries, refByEntry) {
  const wraps = {};
  let total = 0;

  for (let idx = 0; idx < entries.length; idx++) {
    const comps = parseComponents(entries[idx], refByEntry[idx]);
    const comp = comps[componentName];
    if (!comp?.found || !comp.position) continue;

    const normalized = normalizeText(entries[idx]);
    const charBefore = normalized[comp.position.start - 1] || '';
    const charAfter = normalized[comp.position.end] || '';

    total++;

    if (charBefore === '(' && charAfter === ')') {
      wraps['parentheses'] = (wraps['parentheses'] || 0) + 1;
    } else if (charBefore === '[' && charAfter === ']') {
      wraps['brackets'] = (wraps['brackets'] || 0) + 1;
    }
  }

  if (total === 0) return null;
  const sorted = Object.entries(wraps).sort((a, b) => b[1] - a[1]);
  if (sorted.length > 0 && sorted[0][1] / total >= 0.5) {
    return sorted[0][0];
  }
  return null;
}

// -- Formatting detection --

/**
 * Detect if a component is rendered with italic or quote formatting.
 *
 * Examines the raw HTML output from citeproc-js (before normalizeText
 * strips tags). Looks for <i>...</i> around the value (italic) or
 * quote characters (\u201c/\u201d or ") around it (quotes).
 *
 * Returns { emph: true }, { wrap: 'quotes' }, or null.
 */
function detectFormatting(componentName, entries, refByEntry) {
  const formats = { italic: 0, quotes: 0 };
  let total = 0;

  for (let idx = 0; idx < entries.length; idx++) {
    const refData = refByEntry[idx];
    if (!refData) continue;

    // Get the raw value to search for in the HTML entry
    let rawValue = null;
    switch (componentName) {
      case 'title':
        rawValue = refData.title;
        break;
      case 'containerTitle':
        rawValue = refData['container-title'];
        break;
      default:
        return null; // Only titles get formatting
    }
    if (!rawValue) continue;

    const rawHtml = entries[idx];
    const valueLower = rawValue.toLowerCase();
    const htmlLower = rawHtml.toLowerCase();

    // Check if value exists in the entry at all
    const valueIdx = htmlLower.indexOf(valueLower);
    if (valueIdx < 0) continue;

    total++;

    // Check for <i> wrapping: look for <i> before and </i> after
    const before = rawHtml.substring(Math.max(0, valueIdx - 10), valueIdx);
    const after = rawHtml.substring(
      valueIdx + rawValue.length,
      valueIdx + rawValue.length + 10
    );

    if (/<i>\s*$/i.test(before) && /^\s*<\/i>/i.test(after)) {
      formats.italic++;
    }

    // Check for quote wrapping: \u201c before, \u201d after (or ASCII ")
    const charBefore = rawHtml[valueIdx - 1] || '';
    const charAfter = rawHtml[valueIdx + rawValue.length] || '';
    if ((charBefore === '\u201c' || charBefore === '"') &&
      (charAfter === '\u201d' || charAfter === '"' || charAfter === ',')) {
      // Comma after is common: "Title," — check char before for opening quote
      if (charBefore === '\u201c' || charBefore === '"') {
        formats.quotes++;
      }
    }
  }

  if (total === 0) return null;
  if (formats.italic / total >= 0.5) return { emph: true };
  if (formats.quotes / total >= 0.5) return { wrap: 'quotes' };
  return null;
}

/**
 * Detect name order by comparing rendered output to input name data.
 * Returns 'family-first' or 'given-first' or null if can't determine.
 * @param {string} componentText - The text portion of the component
 * @param {Object} refData - The reference data
 * @param {string} role - The name role to check ('author', 'editor', etc.)
 */
/**
 * Detect name order by comparing rendered output to input name data.
 * Returns 'family-first' or 'given-first' or null if can't determine.
 * @param {string} componentText - The text portion of the component (e.g. window around name)
 * @param {Array<Object>} names - The list of name objects to check
 */
function detectNameOrder(componentText, names) {
  if (!componentText || !names || !names.length) return null;

  // Find first name with both family and given
  const nameWithBoth = names.find(n => n.family && n.given);
  if (!nameWithBoth) return null;

  const family = nameWithBoth.family;
  const given = nameWithBoth.given;

  // Normalize the component text for comparison (lowercase for case-insensitive matching)
  const text = normalizeText(componentText).toLowerCase();

  // Find positions in the rendered output
  const familyPos = text.indexOf(family.toLowerCase());
  // For given name, also check for initial form (e.g., "Thomas" -> "T." or "T")
  const givenInitial = given.charAt(0);
  let givenPos = text.indexOf(given.toLowerCase());

  // If full given not found, check for initial
  if (givenPos === -1 && givenInitial) {
    // Look for patterns like "T." or "T. S." at word boundaries
    const initialPattern = new RegExp(`\\b${givenInitial.toLowerCase()}\\.?`, 'i');
    const match = text.match(initialPattern);
    if (match) {
      givenPos = match.index;
    }
  }

  if (familyPos === -1 || givenPos === -1) {
    return null;
  }

  const result = familyPos < givenPos ? 'family-first' : 'given-first';
  return result;
}

/**
 * Detect name order patterns across all entries.
 * Returns global base order and type-specific overrides.
 */
function detectNameOrderPatterns(parserName, role, entries, refByEntry) {
  const results = {}; // type -> order -> count

  for (let i = 0; i < entries.length; i++) {
    const entry = entries[i];
    const refData = refByEntry[i];
    if (!refData) continue;

    // Check if entry actually has this component
    const comps = parseComponents(entry, refData);
    if (!comps[parserName] || !comps[parserName].found || !comps[parserName].position) continue;

    const type = refData.type || 'unknown';

    // Extract a window around the component position to include given names/initials
    const normalized = normalizeText(entry);
    const pos = comps[parserName].position;
    const start = Math.max(0, pos.start - 60);
    const end = Math.min(normalized.length, pos.end + 60);
    const windowText = normalized.substring(start, end);

    // Get relevant names for this component
    const names = (parserName === 'contributors')
      ? (refData.author && refData.author.length > 0 ? refData.author : refData.editor)
      : refData[role];

    const order = detectNameOrder(windowText, names);

    if (order) {
      if (!results[type]) results[type] = {};
      results[type][order] = (results[type][order] || 0) + 1;
    }
  }

  // Determine winner per type
  const typeWinners = {};
  for (const [type, orders] of Object.entries(results)) {
    const best = Object.entries(orders).sort((a, b) => b[1] - a[1]);
    if (best.length > 0) typeWinners[type] = best[0][0];
  }

  // Determine global consensus
  const globalOrders = {};
  for (const order of Object.values(typeWinners)) {
    globalOrders[order] = (globalOrders[order] || 0) + 1;
  }

  const bestGlobal = Object.entries(globalOrders).sort((a, b) => b[1] - a[1]);
  const globalWinner = bestGlobal.length > 0 ? bestGlobal[0][0] : null;

  // Find overrides
  const overrides = {};
  for (const [type, winner] of Object.entries(typeWinners)) {
    if (winner && winner !== globalWinner) {
      overrides[type] = winner;
    }
  }

  return { globalWinner, overrides };
}

// -- CSLN component mapping --

/**
 * Map parser component names to CSLN template component objects.
 */
function mapComponentToYaml(componentName, entry, refData) {
  const comps = parseComponents(entry, refData);
  // Map split container title names back to parser's name
  const parserName = componentName.startsWith('containerTitle')
    ? 'containerTitle' : componentName;
  const comp = comps[parserName];

  if (!comp || !comp.found) {
    return null;
  }

  switch (componentName) {
    case 'contributors': {
      return { contributor: 'author', form: 'long' };
    }

    case 'year': {
      const obj = { date: 'issued', form: 'year' };
      const yearParensMatch = normalizeText(entry).match(/\((\d{4})\)/);
      if (yearParensMatch) {
        obj.wrap = 'parentheses';
      }
      return obj;
    }

    case 'title':
      return { title: 'primary' };

    case 'containerTitle':
    case 'containerTitleSerial':
      return { title: 'parent-serial' };

    case 'containerTitleMonograph':
      return { title: 'parent-monograph' };

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

    case 'editors': {
      return { contributor: 'editor', form: 'verb' };
    }

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

/** Known component main keys for filtering metadata fields */
const MAIN_KEYS = new Set(['contributor', 'date', 'title', 'number', 'variable', 'items']);

/**
 * Generate CSLN YAML for a bibliography/citation section.
 * Includes delimiter at the section level if not the default ". ".
 */
function generateYaml(template, delimiter) {
  let yaml = '';
  if (delimiter && delimiter !== '. ') {
    yaml += `delimiter: "${delimiter}"\n`;
  }
  yaml += 'template:\n';
  const indent = '  ';

  for (const component of template) {
    // Items group (volume + issue)
    if (component.items) {
      yaml += `${indent}- items:\n`;
      for (const item of component.items) {
        const itemKey = Object.keys(item).find(k => MAIN_KEYS.has(k));
        if (!itemKey) continue;
        yaml += `${indent}    - ${itemKey}: ${item[itemKey]}\n`;
        if (item.wrap) yaml += `${indent}      wrap: ${item.wrap}\n`;
      }
      yaml += `${indent}  delimiter: none\n`;
      if (component.prefix) yaml += `${indent}  prefix: "${component.prefix}"\n`;
      continue;
    }

    // Find the main key (contributor, date, title, number, variable)
    const mainKey = Object.keys(component).find(k => MAIN_KEYS.has(k));
    if (!mainKey) continue;

    yaml += `${indent}- ${mainKey}: ${component[mainKey]}\n`;

    if (component.form) yaml += `${indent}  form: ${component.form}\n`;
    if (component.emph) yaml += `${indent}  emph: true\n`;
    if (component.wrap) yaml += `${indent}  wrap: ${component.wrap}\n`;
    if (component.prefix) yaml += `${indent}  prefix: "${component.prefix}"\n`;
    if (component['name-order']) yaml += `${indent}  name-order: ${component['name-order']}\n`;
    if (component.delimiter) yaml += `${indent}  delimiter: "${component.delimiter}"\n`;

    // Suppress overrides
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

  // Split containerTitle into serial/monograph based on reference types.
  // Monograph containers appear in chapters, encyclopedia entries, etc.
  // Serial containers appear in journal articles, magazine articles, etc.
  const MONOGRAPH_TYPES = new Set([
    'chapter', 'entry-encyclopedia', 'entry-dictionary', 'paper-conference',
  ]);
  if (consensusOrdering.includes('containerTitle')) {
    let hasSerial = false;
    let hasMonograph = false;
    for (const [type, data] of Object.entries(typedComponents)) {
      if (type === 'unknown') continue;
      const count = data.componentCounts['containerTitle'] || 0;
      if (count / data.entries.length < 0.5) continue;
      if (MONOGRAPH_TYPES.has(type)) {
        hasMonograph = true;
      } else {
        hasSerial = true;
      }
    }
    if (hasSerial && hasMonograph) {
      // Replace containerTitle with both variants at the same position
      const ctIdx = consensusOrdering.indexOf('containerTitle');
      consensusOrdering.splice(ctIdx, 1, 'containerTitleMonograph', 'containerTitleSerial');
    } else if (hasMonograph && !hasSerial) {
      const ctIdx = consensusOrdering.indexOf('containerTitle');
      consensusOrdering[ctIdx] = 'containerTitleMonograph';
    }
    // If only serial (default), leave as containerTitle → maps to parent-serial
  }

  // Detect volume(issue) grouping pattern
  const isVolumeIssueGrouped = detectVolumeIssueGrouping(
    rendered.entries, refByEntry
  );

  // Detect prefixes for known components
  const prefixes = {};
  for (const compName of ['editors', 'doi', 'pages']) {
    const prefix = detectPrefix(compName, rendered.entries, refByEntry);
    if (prefix) prefixes[compName] = prefix;
  }

  // Detect wrap patterns for components not already handled in mapComponentToYaml
  const wrapPatterns = {};
  for (const compName of ['issue', 'pages']) {
    const wrap = detectWrap(compName, rendered.entries, refByEntry);
    if (wrap) wrapPatterns[compName] = wrap;
  }

  // Detect formatting (italic/quotes) for title components
  const formattingPatterns = {};
  for (const compName of ['title', 'containerTitle']) {
    const fmt = detectFormatting(compName, rendered.entries, refByEntry);
    if (fmt) {
      formattingPatterns[compName] = fmt;
      // Apply to split variants too
      formattingPatterns['containerTitleSerial'] = fmt;
      formattingPatterns['containerTitleMonograph'] = fmt;
    }
  }

  // Detect name order patterns for contributors and editors
  const nameOrderPatterns = {};
  for (const [compName, role] of Object.entries({ contributors: 'author', editors: 'editor' })) {
    const patterns = detectNameOrderPatterns(compName, role, rendered.entries, refByEntry);
    if (patterns.globalWinner || Object.keys(patterns.overrides).length > 0) {
      nameOrderPatterns[compName] = patterns;
    }
  }

  // Build template array
  const template = [];
  let skipIssue = false; // when grouped into items with volume

  for (const componentName of consensusOrdering) {
    // Skip issue if already grouped with volume
    if (componentName === 'issue' && skipIssue) continue;

    // Map split container title names back to parser's componentName
    const parserName = componentName.startsWith('containerTitle')
      ? 'containerTitle' : componentName;

    // Use first entry that has this component to get the mapping
    const entryIdx = rendered.entries.findIndex((entry, idx) => {
      const comps = parseComponents(entry, refByEntry[idx]);
      return comps[parserName] && comps[parserName].found;
    });

    if (entryIdx < 0) continue;

    // Handle volume+issue grouping
    if (componentName === 'volume' && isVolumeIssueGrouped &&
      consensusOrdering.includes('issue')) {
      const issueWrap = wrapPatterns['issue'] || null;
      const volumeComp = { number: 'volume' };
      const issueComp = { number: 'issue' };
      if (issueWrap) issueComp.wrap = issueWrap;

      template.push({
        items: [volumeComp, issueComp],
        _componentName: 'volume',
      });
      skipIssue = true;
      continue;
    }

    const yamlComponent = mapComponentToYaml(
      componentName,
      rendered.entries[entryIdx],
      refByEntry[entryIdx]
    );

    if (yamlComponent) {
      // Apply detected prefix (check both split name and parser name)
      if (prefixes[componentName] || prefixes[parserName]) {
        yamlComponent.prefix = prefixes[componentName] || prefixes[parserName];
      }
      // Apply detected wrap (for components not already handled)
      if (wrapPatterns[componentName] && !yamlComponent.wrap) {
        yamlComponent.wrap = wrapPatterns[componentName];
      }

      // Apply detected formatting (italic/quotes)
      if (formattingPatterns[componentName]) {
        Object.assign(yamlComponent, formattingPatterns[componentName]);
      }

      yamlComponent._componentName = componentName;

      // Apply detected name order patterns
      if (nameOrderPatterns[parserName]) {
        const patterns = nameOrderPatterns[parserName];
        if (patterns.globalWinner) {
          yamlComponent['name-order'] = patterns.globalWinner;
        }
        // Apply overrides to existing yamlComponent.overrides if any
        if (Object.keys(patterns.overrides).length > 0) {
          if (!yamlComponent.overrides) yamlComponent.overrides = {};
          for (const [type, order] of Object.entries(patterns.overrides)) {
            if (!yamlComponent.overrides[type]) yamlComponent.overrides[type] = {};
            yamlComponent.overrides[type]['name-order'] = order;
          }
        }
      }

      template.push(yamlComponent);
    }
  }

  // Detect suppress overrides
  const suppressions = detectSuppressions(consensusOrdering, typedComponents, componentFrequency);

  // Find section-level delimiter by counting inter-component delimiters
  // across all entries. Skip contributor and year pairs (their positions
  // are affected by name initials and wrapping parens, not the section
  // delimiter).
  let delimiterConsensus = '. '; // default
  {
    const skipPairs = new Set(['contributors', 'year', 'editors']);
    const delimCounts = {};
    for (let idx = 0; idx < rendered.entries.length; idx++) {
      const dets = detectDelimiters(rendered.entries[idx], refByEntry[idx]);
      for (const det of dets) {
        // Skip pairs involving contributor names or year (noisy positions)
        if (skipPairs.has(det.between[0]) || skipPairs.has(det.between[1])) continue;
        const d = det.delimiter.replace(/["\u201c\u201d]+/g, '');
        if (d.length >= 1 && d.length <= 4 && /^[.,;: ]+$/.test(d)) {
          delimCounts[d] = (delimCounts[d] || 0) + 1;
        }
      }
    }
    const best = Object.entries(delimCounts).sort((a, b) => b[1] - a[1]);
    if (best.length > 0) delimiterConsensus = best[0][0];
  }

  // Detect per-component delimiter prefixes that differ from section-level delimiter.
  // For each adjacent pair in the template, if their delimiter differs from the
  // section-level delimiter, set it as a prefix on the second component.
  //
  // Also check non-adjacent pairs that become adjacent when intervening components
  // are suppressed (e.g., containerTitle → publisher → volume, where publisher is
  // suppressed for journal articles, making containerTitle → volume adjacent).
  const templateNames = template.map(c => c._componentName || (c.items ? 'volume' : null));

  // Map split template names back to component-parser names used by detectDelimiters.
  // The consensus ordering splits containerTitle into containerTitleSerial/Monograph,
  // but the parser always uses "containerTitle".
  const parserNameMap = {
    containerTitleSerial: 'containerTitle',
    containerTitleMonograph: 'containerTitle',
  };

  for (let i = 1; i < template.length; i++) {
    const currComp = template[i];
    const currName = templateNames[i];
    if (!currName) continue;
    if (currComp.prefix) continue; // Already has a prefix (from detectPrefix)

    // Check predecessors: immediate first, then non-adjacent (for
    // components suppressed for some types, e.g. containerTitle → volume).
    //
    // Frequency thresholds prevent rare type-specific pairs from setting
    // incorrect prefixes.  For immediate predecessors that appear in < 20%
    // of entries (e.g. editors only in chapters), require the pair itself
    // to also meet a 20% threshold.  Common predecessors keep minFrac = 0
    // to tolerate position-detection gaps (e.g. multi-author contributor).
    const parserCurrName = parserNameMap[currName] || currName;
    const totalEntries = rendered.entries.length;

    for (let j = i - 1; j >= 0; j--) {
      // For items groups, try both issue and volume as predecessor names,
      // since not all entries have issue numbers.
      let altCandidates;
      if (template[j].items) {
        altCandidates = ['issue', 'volume'];
      } else {
        const rawAlt = templateNames[j];
        if (!rawAlt) continue;
        altCandidates = [parserNameMap[rawAlt] || rawAlt];
      }

      // Determine frequency threshold: non-adjacent always gets 0.2;
      // immediate predecessor gets 0.2 if rare, 0 if common.
      let minFrac;
      if (j !== i - 1) {
        minFrac = 0.2;
      } else {
        const predName = templateNames[j] || (template[j].items ? 'volume' : null);
        const predParserName = predName ? (parserNameMap[predName] || predName) : null;
        const predFreq = predParserName ? (componentFrequency[predParserName] || 0) : 0;
        minFrac = (predFreq / totalEntries >= 0.2) ? 0 : 0.2;
      }

      let found = false;
      for (const altName of altCandidates) {
        const pairDelim = findDelimiterConsensus(
          rendered.entries, refByEntry, altName, parserCurrName, minFrac
        );
        if (pairDelim && pairDelim !== delimiterConsensus) {
          // Don't set whitespace-only prefix for wrapped components.
          // The renderer adds a space before opening parens/brackets automatically,
          // so " " prefix would create "( 1962)" instead of "(1962)".
          if (currComp.wrap && /^\s+$/.test(pairDelim)) {
            // Skip setting this as prefix - renderer handles it
          } else {
            currComp.prefix = pairDelim;
            found = true;
          }
          break;
        }
      }
      if (found) break;
      // Stop after checking three levels back
      if (j <= i - 4) break;
    }
  }

  // Attach overrides to template objects and clean up internal fields
  for (const comp of template) {
    if (comp._componentName) {
      const compSuppressions = suppressions[comp._componentName];
      if (compSuppressions && Object.keys(compSuppressions).length > 0) {
        if (!comp.overrides) comp.overrides = {};
        for (const [type] of Object.entries(compSuppressions)) {
          if (!comp.overrides[type]) comp.overrides[type] = {};
          comp.overrides[type].suppress = true;
        }
      }
      delete comp._componentName;
    }
  }

  // Generate YAML
  const yaml = generateYaml(template, delimiterConsensus);

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
  detectPrefix,
  detectWrap,
  detectVolumeIssueGrouping,
  detectFormatting,
  mapComponentToYaml,
  generateYaml,
  detectNameOrder,
  detectNameOrderPatterns,
};
