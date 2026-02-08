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
        // if (comp1 === 'editors') console.error(`  - entry delim: "${delim}"`);
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
/**
 * Detect common prefix patterns across all entries, returning global consensus and overrides.
 */
function detectPrefixPatterns(componentName, entries, refByEntry) {
  const results = {}; // type -> prefix -> count

  for (let idx = 0; idx < entries.length; idx++) {
    const refData = refByEntry[idx];
    if (!refData) continue;

    const comps = parseComponents(entries[idx], refData);
    const comp = comps[componentName];
    if (!comp?.found || !comp.position) continue;

    const type = refData.type || 'unknown';
    const normalized = normalizeText(entries[idx]);
    const matchText = normalized.slice(comp.position.start, comp.position.end);
    const beforeStart = Math.max(0, comp.position.start - 20);
    const before = normalized.slice(beforeStart, comp.position.start);

    let prefix = null;
    // DOI: "https://doi.org/" is inside the match
    if (/^https?:\/\/doi\.org\//i.test(matchText)) {
      prefix = 'https://doi.org/';
    }
    // Pages: "pp." or "p." is inside the match
    else if (/^pp?\.\s*/i.test(matchText)) {
      prefix = 'pp. ';
    }
    // Container group prefixes: "In " or "on "
    else if ((componentName === 'editors' || componentName.startsWith('containerTitle')) &&
      /(?:In|on)\s+$/i.test(before)) {
      const match = before.match(/(?:In|on)\s+$/i);
      prefix = match[0];
    }

    if (prefix) {
      if (!results[type]) results[type] = {};
      results[type][prefix] = (results[type][prefix] || 0) + 1;
    }
  }

  // Determine winner per type (require 50% threshold)
  const typeWinners = {};
  for (const [type, pfxs] of Object.entries(results)) {
    const typeTotal = entries.filter((e, i) => {
      if ((refByEntry[i]?.type || 'unknown') !== type) return false;
      const c = parseComponents(e, refByEntry[i]);
      return c[componentName]?.found;
    }).length;

    const best = Object.entries(pfxs).sort((a, b) => b[1] - a[1]);
    if (best.length > 0 && best[0][1] / typeTotal >= 0.5) {
      typeWinners[type] = best[0][0];
    }
  }

  // Determine global consensus
  const allTypesWithComp = new Set(refByEntry.filter(r => r).map(r => r.type || 'unknown'));
  const globalCounts = {};
  for (const pfx of Object.values(typeWinners)) {
    globalCounts[pfx] = (globalCounts[pfx] || 0) + 1;
  }
  const bestGlobal = Object.entries(globalCounts).sort((a, b) => b[1] - a[1]);

  const globalWinner = (bestGlobal.length > 0 && bestGlobal[0][1] / allTypesWithComp.size > 0.5)
    ? bestGlobal[0][0] : null;

  const overrides = {};
  for (const [type, winner] of Object.entries(typeWinners)) {
    if (winner !== globalWinner) {
      overrides[type] = winner;
    }
  }

  return { globalWinner, overrides };
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
/**
 * Detect wrap patterns across all entries, returning global consensus and overrides.
 */
function detectWrapPatterns(componentName, entries, refByEntry) {
  const results = {}; // type -> wrap -> count

  for (let idx = 0; idx < entries.length; idx++) {
    const refData = refByEntry[idx];
    if (!refData) continue;

    const comps = parseComponents(entries[idx], refData);
    const comp = comps[componentName];
    if (!comp?.found || !comp.position) continue;

    const type = refData.type || 'unknown';
    const normalized = normalizeText(entries[idx]);
    const charBefore = normalized[comp.position.start - 1] || '';
    const charAfter = normalized[comp.position.end] || '';

    let wrap = null;
    if (charBefore === '(' && charAfter === ')') {
      wrap = 'parentheses';
    } else if (charBefore === '[' && charAfter === ']') {
      wrap = 'brackets';
    }

    if (wrap) {
      if (!results[type]) results[type] = {};
      results[type][wrap] = (results[type][wrap] || 0) + 1;
    }
  }

  // Determine winner per type (require 70% threshold within type)
  const typeWinners = {};
  for (const [type, wraps] of Object.entries(results)) {
    // Count entries of this type that HAVE this component
    const typeTotal = entries.filter((e, i) => {
      if ((refByEntry[i]?.type || 'unknown') !== type) return false;
      const c = parseComponents(e, refByEntry[i]);
      return c[componentName]?.found;
    }).length;

    const best = Object.entries(wraps).sort((a, b) => b[1] - a[1]);
    if (best.length > 0 && best[0][1] / typeTotal >= 0.7) {
      typeWinners[type] = best[0][0];
    }
  }

  // Determine global consensus across ALL types that have the component
  const allTypesWithComp = new Set();
  for (let i = 0; i < entries.length; i++) {
    const type = refByEntry[i]?.type || 'unknown';
    const c = parseComponents(entries[i], refByEntry[i]);
    if (c[componentName]?.found) allTypesWithComp.add(type);
  }

  const globalCounts = {};
  for (const wrap of Object.values(typeWinners)) {
    globalCounts[wrap] = (globalCounts[wrap] || 0) + 1;
  }
  const bestGlobal = Object.entries(globalCounts).sort((a, b) => b[1] - a[1]);

  // Global winner must appear in >50% of ALL types that have the component
  const globalWinner = (bestGlobal.length > 0 && bestGlobal[0][1] / allTypesWithComp.size > 0.5)
    ? bestGlobal[0][0] : null;

  const overrides = {};
  for (const [type, winner] of Object.entries(typeWinners)) {
    if (winner !== globalWinner) {
      overrides[type] = winner;
    }
  }

  return { globalWinner, overrides, typeWinners };
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

    // Extract a tight window around the component position
    const normalized = normalizeText(entry);
    const pos = comps[parserName].position;
    const start = Math.max(0, pos.start - 5);
    const end = Math.min(normalized.length, pos.end + 5);
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
      yaml += `${indent}  delimiter: ${component.delimiter || 'none'}\n`;
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

  // Detect prefixes for all components
  const prefixes = {};
  for (const compName of consensusOrdering) {
    const patterns = detectPrefixPatterns(compName, rendered.entries, refByEntry);
    if (patterns.globalWinner || Object.keys(patterns.overrides).length > 0) {
      prefixes[compName] = patterns;
    }
  }
  console.error('Detected prefixes:', JSON.stringify(prefixes, null, 2));

  // Detect wrap patterns for components not already handled in mapComponentToYaml
  const wrapPatterns = {};
  for (const compName of ['issue', 'pages', 'year', 'volume']) {
    const patterns = detectWrapPatterns(compName, rendered.entries, refByEntry);
    if (patterns.globalWinner || Object.keys(patterns.overrides).length > 0) {
      wrapPatterns[compName] = patterns;
    }
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
  let template = [];
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
      const issuePatterns = wrapPatterns['issue'] || null;
      const volumeComp = { number: 'volume' };
      const issueComp = { number: 'issue' };
      if (issuePatterns && (issuePatterns.globalWinner || Object.keys(issuePatterns.overrides).length > 0)) {
        if (issuePatterns.globalWinner) issueComp.wrap = issuePatterns.globalWinner;
        // Apply overrides to issue specifically
        if (Object.keys(issuePatterns.overrides).length > 0) {
          issueComp.overrides = {};
          for (const [type, wrap] of Object.entries(issuePatterns.overrides)) {
            issueComp.overrides[type] = { wrap };
          }
        }
      }

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
        const patterns = prefixes[componentName] || prefixes[parserName];
        if (patterns.globalWinner) yamlComponent.prefix = patterns.globalWinner;
        if (Object.keys(patterns.overrides).length > 0) {
          if (!yamlComponent.overrides) yamlComponent.overrides = {};
          for (const [type, pfx] of Object.entries(patterns.overrides)) {
            if (!yamlComponent.overrides[type]) yamlComponent.overrides[type] = {};
            yamlComponent.overrides[type].prefix = pfx;
          }
        }
      }
      // Apply detected wrap (for components not already handled)
      if (wrapPatterns[componentName]) {
        const patterns = wrapPatterns[componentName];
        if (patterns.globalWinner && !yamlComponent.wrap) {
          yamlComponent.wrap = patterns.globalWinner;
        }
        if (Object.keys(patterns.overrides).length > 0) {
          if (!yamlComponent.overrides) yamlComponent.overrides = {};
          for (const [type, wrap] of Object.entries(patterns.overrides)) {
            if (!yamlComponent.overrides[type]) yamlComponent.overrides[type] = {};
            yamlComponent.overrides[type].wrap = wrap;
          }
        }
      }

      // Apply detected formatting (italic/quotes)
      if (formattingPatterns[componentName]) {
        Object.assign(yamlComponent, formattingPatterns[componentName]);
      }

      yamlComponent._componentName = componentName;

      // Ensure prefix doesn't duplicate opening wrap char
      if (yamlComponent.prefix && yamlComponent.wrap === 'parentheses' && yamlComponent.prefix.endsWith('(')) {
        yamlComponent.prefix = yamlComponent.prefix.slice(0, -1);
      } else if (yamlComponent.prefix && yamlComponent.wrap === 'brackets' && yamlComponent.prefix.endsWith('[')) {
        yamlComponent.prefix = yamlComponent.prefix.slice(0, -1);
      }
      if (yamlComponent.prefix === '') delete yamlComponent.prefix;

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

  // Detect common bibliography entry suffix (e.g. trailing period)
  let entrySuffix = null;
  {
    const suffixCounts = {};
    for (const text of rendered.entries) {
      const match = text.match(/([.,;: ]+)$/);
      if (match) {
        const s = match[1];
        suffixCounts[s] = (suffixCounts[s] || 0) + 1;
      }
    }
    const bestSuffix = Object.entries(suffixCounts).sort((a, b) => b[1] - a[1]);
    // Require 70% consensus for a global suffix
    if (bestSuffix.length > 0 && bestSuffix[0][1] / rendered.entries.length >= 0.7) {
      entrySuffix = bestSuffix[0][0];
    }
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
    const totalEntries = rendered.entries.length;

    for (let j = i - 1; j >= 0; j--) {
      const parserCurrName = parserNameMap[currName] || currName;

      // Handle volume+issue grouping
      let altCandidates;
      if (template[j].items) {
        altCandidates = ['issue', 'volume'];
      } else {
        const rawAlt = templateNames[j];
        if (!rawAlt) continue;
        altCandidates = [parserNameMap[rawAlt] || rawAlt];
      }

      // Determine frequency threshold: non-adjacent always gets 0.2;
      // immediate predecessor gets 0.
      const minFrac = (j === i - 1) ? 0 : 0.2;

      let found = false;
      for (const altName of altCandidates) {
        let pairDelim = findDelimiterConsensus(
          rendered.entries, refByEntry, altName, parserCurrName, minFrac
        );

        if (pairDelim) {
          // If the component already has a prefix (from detectPrefixPatterns),
          // check if that prefix is part of the detected pair delimiter.
          if (currComp.prefix && pairDelim.endsWith(currComp.prefix)) {
            pairDelim = pairDelim.slice(0, -currComp.prefix.length);
          }
        }

        if (pairDelim && pairDelim !== delimiterConsensus) {
          // console.error(`Pair delim between ${altName} and ${parserCurrName}: "${pairDelim}"`);
          // Strip preceding wrap from previous component
          const predPatterns = wrapPatterns[altName];
          if (predPatterns) {
            // If it's wrapped globally or in common types, strip it
            const wrap = predPatterns.globalWinner || Object.values(predPatterns.typeWinners)[0];
            if (wrap === 'parentheses' && pairDelim.startsWith(')')) {
              pairDelim = pairDelim.slice(1);
            } else if (wrap === 'brackets' && pairDelim.startsWith(']')) {
              pairDelim = pairDelim.slice(1);
            }
          }

          // Strip succeeding wrap from current component
          const currPatterns = wrapPatterns[parserCurrName];
          if (currPatterns) {
            const wrap = currPatterns.globalWinner || Object.values(currPatterns.typeWinners)[0];
            if (wrap === 'parentheses' && pairDelim.endsWith('(')) {
              pairDelim = pairDelim.slice(0, -1);
            } else if (wrap === 'brackets' && pairDelim.endsWith('[')) {
              pairDelim = pairDelim.slice(0, -1);
            }
          }

          if (pairDelim === delimiterConsensus) {
            found = true;
            break;
          }

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
  // Also clean up trailing entry suffix from the last component's prefix
  if (entrySuffix && template.length > 0) {
    const lastComp = template[template.length - 1];
    if (lastComp.prefix && lastComp.prefix.includes(entrySuffix)) {
      // If prefix is just the entry suffix, or ends with it
      if (lastComp.prefix === entrySuffix) {
        delete lastComp.prefix;
      } else if (lastComp.prefix.endsWith(entrySuffix)) {
        lastComp.prefix = lastComp.prefix.slice(0, -entrySuffix.length);
        if (lastComp.prefix === '') delete lastComp.prefix;
      }
    }
  }

  // Post-process to group container elements (editors + container titles)
  template = applyContainerGrouping(template);

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
      entrySuffix,
      entryCount: rendered.entries.length,
      section,
    },
  };
}

/**
 * Group adjacent container-related components into a shared list.
 * This handles the "In Editor, Book Title" pattern by moving prefixes
 * and delimiters to the group level.
 */
function applyContainerGrouping(template) {
  const containerTypes = ['editors', 'containerTitleMonograph'];
  const newTemplate = [];

  for (let i = 0; i < template.length; i++) {
    const comp = template[i];
    const name = comp._componentName;
    // console.error(`Checking comp: ${name}`);

    if (containerTypes.includes(name)) {
      // Look ahead for sequential container components
      let j = i + 1;
      while (j < template.length && containerTypes.includes(template[j]._componentName)) {
        j++;
      }

      if (j > i + 1) {
        // console.error(`Found group from ${i} to ${j}: ${template.slice(i, j).map(c => c._componentName).join(', ')}`);
        // We found a sequence of 2+ container components.
        // Group them into an items block.
        const groupItems = template.slice(i, j);

        const group = {
          items: groupItems.map(c => {
            const { _componentName, ...rest } = c;
            return rest;
          }),
          _componentName: 'containerGroup'
        };

        // 1. Move the prefix of the first item to the group level
        if (groupItems[0].prefix) {
          group.prefix = groupItems[0].prefix;
          delete group.items[0].prefix;
        }

        // Similarly for overrides of the first item
        if (groupItems[0].overrides) {
          for (const [type, ov] of Object.entries(groupItems[0].overrides)) {
            if (ov.prefix) {
              if (!group.overrides) group.overrides = {};
              if (!group.overrides[type]) group.overrides[type] = {};
              group.overrides[type].prefix = ov.prefix;
              delete group.items[0].overrides[type].prefix;
              if (Object.keys(group.items[0].overrides[type]).length === 0) {
                delete group.items[0].overrides[type];
              }
            }
          }
          if (Object.keys(group.items[0].overrides).length === 0) {
            delete group.items[0].overrides;
          }
        }

        // 2. Try to identify a common delimiter from the second item's prefix
        // (which usually stores the delimiter between the first and second item).
        if (group.items[1].prefix) {
          group.delimiter = group.items[1].prefix;
          delete group.items[1].prefix;
        }

        newTemplate.push(group);
        i = j - 1;
        continue;
      }
    }
    newTemplate.push(comp);
  }
  return newTemplate;
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
  detectPrefixPatterns,
  detectWrapPatterns,
  detectVolumeIssueGrouping,
  detectFormatting,
  mapComponentToYaml,
  generateYaml,
  detectNameOrder,
  detectNameOrderPatterns,
};
