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
        const text = engine.makeCitationCluster([{ id }]);
        citations[id] = text || '';
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
        // Track components with data in ref but suppressed in output
        suppressedCounts: {},
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

    // Special handling: track issue suppression (data present but not rendered)
    if (refData?.issue && !comps.issue?.found) {
      typedComponents[type].suppressedCounts['issue'] =
        (typedComponents[type].suppressedCounts['issue'] || 0) + 1;
    }
  }

  return typedComponents;
}

/**
 * Find the consensus component ordering across all reference types.
 */
function findConsensusOrdering(entries, refByEntry) {
  const typedComponents = aggregateByType(entries, refByEntry);

  const allOrderings = [];
  const orderingsByType = {};

  for (const [type, data] of Object.entries(typedComponents)) {
    const typeOrderings = [];
    for (const entry of data.entries) {
      const refData = findRefDataForEntry(entry, loadFixtures());
      const ordering = analyzeOrdering(entry, refData);
      allOrderings.push(ordering);
      typeOrderings.push(ordering);
    }
    const counts = {};
    for (const o of typeOrderings) {
      const k = o.join(',');
      counts[k] = (counts[k] || 0) + 1;
    }
    const best = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    orderingsByType[type] = best[0]?.[0]?.split(',').filter(Boolean) || [];
  }

  const allComponents = new Set();
  for (const ordering of allOrderings) {
    for (const comp of ordering) {
      if (comp) allComponents.add(comp);
    }
  }

  const componentFrequency = {};
  for (const comp of allComponents) {
    componentFrequency[comp] = 0;
    for (const ordering of allOrderings) {
      if (ordering.includes(comp)) {
        componentFrequency[comp]++;
      }
    }
  }

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

  const componentList = [...allComponents];
  componentList.sort((a, b) => {
    const aBeforeB = precedence[a]?.[b] || 0;
    const bBeforeA = precedence[b]?.[a] || 0;
    if (aBeforeB !== bBeforeA) {
      return aBeforeB > bBeforeA ? -1 : 1;
    }
    return (componentFrequency[b] || 0) - (componentFrequency[a] || 0);
  });

  const volIdx = componentList.indexOf('volume');
  const issIdx = componentList.indexOf('issue');
  if (volIdx >= 0 && issIdx >= 0 && issIdx !== volIdx + 1) {
    componentList.splice(issIdx, 1);
    const newVolIdx = componentList.indexOf('volume');
    componentList.splice(newVolIdx + 1, 0, 'issue');
  }

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

  if (Object.keys(delimiters).length === 0) return null;
  const sorted = Object.entries(delimiters).sort((a, b) => b[1] - a[1]);
  const minCount = Math.max(1, Math.floor(entries.length * (minFraction || 0)));
  if (sorted[0][1] < minCount) return null;
  return sorted[0][0].replace(/["\u201c\u201d]+/g, '') || sorted[0][0];
}

// -- Prefix/suffix and wrap detection --

function detectPrefixPatterns(componentName, entries, refByEntry) {
  const results = {};

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
    if (/^https?:\/\/doi\.org\//i.test(matchText)) {
      prefix = 'https://doi.org/';
    }
    else if (/^pp?\.\s*/i.test(matchText)) {
      prefix = 'pp. ';
    }
    else if ((componentName === 'editors' || componentName.startsWith('containerTitle')) &&
      /(?:In|on)[:\s]+\s*$/i.test(before)) {
      const match = before.match(/(?:In|on)[:\s]+\s*$/i);
      prefix = match[0];
    }

    if (prefix) {
      if (!results[type]) results[type] = {};
      results[type][prefix] = (results[type][prefix] || 0) + 1;
    }
  }

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

function detectVolumeIssueGrouping(entries, refByEntry) {
  let withBoth = 0;
  let grouped = 0;

  for (let idx = 0; idx < entries.length; idx++) {
    const comps = parseComponents(entries[idx], refByEntry[idx]);
    if (!comps.volume?.found || !comps.issue?.found) continue;
    if (!comps.volume.position || !comps.issue.position) continue;

    withBoth++;
    const normalized = normalizeText(entries[idx]);
    const between = normalized.slice(comps.volume.position.end, comps.issue.position.start);

    if (/^\s*\(?\s*$/.test(between)) {
      grouped++;
    }
  }

  return withBoth > 0 && grouped / withBoth >= 0.5;
}

function detectWrapPatterns(componentName, entries, refByEntry) {
  const results = {};

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

  const typeWinners = {};
  for (const [type, wraps] of Object.entries(results)) {
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

function detectFormatting(componentName, entries, refByEntry) {
  const formats = { italic: 0, quotes: 0 };
  let total = 0;

  for (let idx = 0; idx < entries.length; idx++) {
    const refData = refByEntry[idx];
    if (!refData) continue;

    let rawValue = null;
    switch (componentName) {
      case 'title': rawValue = refData.title; break;
      case 'containerTitle': rawValue = refData['container-title']; break;
      default: return null;
    }
    if (!rawValue) continue;

    const rawHtml = entries[idx];
    const valueLower = rawValue.toLowerCase();
    const htmlLower = rawHtml.toLowerCase();

    const valueIdx = htmlLower.indexOf(valueLower);
    if (valueIdx < 0) continue;

    total++;
    const before = rawHtml.substring(Math.max(0, valueIdx - 10), valueIdx);
    const after = rawHtml.substring(valueIdx + rawValue.length, valueIdx + rawValue.length + 10);

    if (/<i>\s*$/i.test(before) && /^\s*<\/i>/i.test(after)) {
      formats.italic++;
    }

    const charBefore = rawHtml[valueIdx - 1] || '';
    const charAfter = rawHtml[valueIdx + rawValue.length] || '';
    if ((charBefore === '\u201c' || charBefore === '"') &&
      (charAfter === '\u201d' || charAfter === '"' || charAfter === ',')) {
      formats.quotes++;
    }
  }

  if (total === 0) return null;
  if (formats.italic / total >= 0.5) return { emph: true };
  if (formats.quotes / total >= 0.5) return { wrap: 'quotes' };
  return null;
}

// -- Name order detection --

function detectNameOrder(componentText, names) {
  if (!componentText || !names || !names.length) return null;
  const nameWithBoth = names.find(n => n.family && n.given);
  if (!nameWithBoth) return null;

  const family = nameWithBoth.family.toLowerCase();
  const text = normalizeText(componentText).toLowerCase();
  const familyPos = text.indexOf(family);
  const givenInitial = nameWithBoth.given.charAt(0).toLowerCase();
  let givenPos = text.indexOf(nameWithBoth.given.toLowerCase());

  if (givenPos === -1 && givenInitial) {
    const match = text.match(new RegExp(`\\b${givenInitial}\\.?`, 'i'));
    if (match) givenPos = match.index;
  }

  if (familyPos === -1 || givenPos === -1) return null;
  return familyPos < givenPos ? 'family-first' : 'given-first';
}

function detectNameOrderPatterns(parserName, role, entries, refByEntry) {
  const results = {};

  for (let i = 0; i < entries.length; i++) {
    const entry = entries[i];
    const refData = refByEntry[i];
    if (!refData) continue;

    const comps = parseComponents(entry, refData);
    if (!comps[parserName]?.found || !comps[parserName].position) continue;

    const type = refData.type || 'unknown';
    const normalized = normalizeText(entry);
    const pos = comps[parserName].position;
    const windowText = normalized.substring(Math.max(0, pos.start - 5), Math.min(normalized.length, pos.end + 5));

    const names = (parserName === 'contributors')
      ? (refData.author?.length > 0 ? refData.author : refData.editor)
      : refData[role];

    const order = detectNameOrder(windowText, names);
    if (order) {
      if (!results[type]) results[type] = {};
      results[type][order] = (results[type][order] || 0) + 1;
    }
  }

  const typeWinners = {};
  for (const [type, orders] of Object.entries(results)) {
    const best = Object.entries(orders).sort((a, b) => b[1] - a[1]);
    if (best.length > 0) typeWinners[type] = best[0][0];
  }

  const globalOrders = {};
  for (const order of Object.values(typeWinners)) {
    globalOrders[order] = (globalOrders[order] || 0) + 1;
  }
  const bestGlobal = Object.entries(globalOrders).sort((a, b) => b[1] - a[1]);
  const globalWinner = bestGlobal.length > 0 ? bestGlobal[0][0] : null;

  const overrides = {};
  for (const [type, winner] of Object.entries(typeWinners)) {
    if (winner && winner !== globalWinner) overrides[type] = winner;
  }

  return { globalWinner, overrides };
}

// -- CSLN component mapping --

function mapComponentToYaml(componentName, entry, refData) {
  const comps = parseComponents(entry, refData);
  const parserName = componentName.startsWith('containerTitle') ? 'containerTitle' : componentName;
  const comp = comps[parserName];

  if (!comp?.found) return null;

  switch (componentName) {
    case 'contributors': return { contributor: 'author', form: 'long' };
    case 'year': {
      const obj = { date: 'issued', form: 'year' };
      if (normalizeText(entry).match(/\((\d{4})\)/)) obj.wrap = 'parentheses';
      return obj;
    }
    case 'title': return { title: 'primary' };
    case 'containerTitle':
    case 'containerTitleSerial': return { title: 'parent-serial' };
    case 'containerTitleMonograph': return { title: 'parent-monograph' };
    case 'volume': return { number: 'volume' };
    case 'issue': return { number: 'issue' };
    case 'pages': return { number: 'pages' };
    case 'publisher': return { variable: 'publisher' };
    case 'place': return { variable: 'publisher-place' };
    case 'doi': return { variable: 'doi' };
    case 'url': return { variable: 'url' };
    case 'edition': return { number: 'edition' };
    case 'editors': return { contributor: 'editor', form: 'verb' };
    default: return null;
  }
}

// -- Suppress override detection --

function detectSuppressions(consensusOrdering, typedComponents, componentFrequency, renderedEntries, refByEntry) {
  const suppressions = {};
  const knownTypes = Object.keys(typedComponents).filter(t => t !== 'unknown');
  const totalTypes = knownTypes.length;

  for (const componentName of consensusOrdering) {
    suppressions[componentName] = {};

    if (componentName === 'issue') {
      for (const type of knownTypes) {
        const data = typedComponents[type];
        let hasIssueInData = false;
        let suppressedInOutput = true;

        for (const entryId of data.entries) {
          const entryIdx = renderedEntries.entries.indexOf(entryId);
          const ref = refByEntry[entryIdx];
          const rendered = entryId; // entryId IS the rendered text in this context
          if (ref?.issue) {
            hasIssueInData = true;
            if (rendered.toLowerCase().includes(String(ref.issue).toLowerCase())) {
              suppressedInOutput = false;
              break;
            }
          }
        }
        if (hasIssueInData && suppressedInOutput) suppressions.issue[type] = true;
      }
      continue;
    }

    let typesWithComponent = 0;
    const typesPresent = new Set();
    const typesMissing = new Set();

    for (const type of knownTypes) {
      const data = typedComponents[type];
      const count = data.componentCounts[componentName] || 0;
      if (count / data.entries.length >= 0.5) {
        typesWithComponent++;
        typesPresent.add(type);
      } else {
        typesMissing.add(type);
      }
    }

    const presentRatio = typesWithComponent / totalTypes;
    if (presentRatio >= 0.4 && typesMissing.size > 0 && typesMissing.size <= typesPresent.size) {
      for (const type of typesMissing) suppressions[componentName][type] = true;
    }
  }

  return suppressions;
}

function detectEtAl(parserName, entries, refByEntry, renderedEntries) {
  const settings = { min: 0, use_first: 0 };
  let detected = false;

  for (let i = 0; i < entries.length; i++) {
    const entryId = entries[i];
    const ref = refByEntry[i];
    if (!ref) continue;
    const rendered = entryId;
    const role = parserName === 'contributors' ? 'author' : 'editor';
    const names = ref[role];

    if (names && names.length >= 2) {
      const lowerRendered = rendered.toLowerCase();
      const hasEtAlMarker = lowerRendered.includes('et al') || lowerRendered.includes('…') || lowerRendered.includes('others');
      if (hasEtAlMarker) {
        let renderedCount = 0;
        for (const name of names) {
          const family = (name.family || name.literal || '').toLowerCase();
          if (family && lowerRendered.includes(family)) renderedCount++;
        }
        if (renderedCount < names.length && renderedCount > 0) {
          settings.min = names.length;
          settings.use_first = renderedCount;
          detected = true;
          break;
        }
      }
    }
  }

  return detected ? settings : null;
}

// -- YAML generation --

const MAIN_KEYS = new Set(['contributor', 'date', 'title', 'number', 'variable', 'items']);

function generateYaml(template, delimiter, wrap) {
  let yaml = '';
  if (wrap) yaml += `wrap: ${wrap}\n`;
  if (delimiter && delimiter !== '. ') yaml += `delimiter: "${delimiter}"\n`;
  yaml += 'template:\n';
  const indent = '  ';

  for (const component of template) {
    if (component.items) {
      yaml += `${indent}- items:\n`;
      for (const item of component.items) {
        const itemKey = Object.keys(item).find(k => MAIN_KEYS.has(k));
        if (!itemKey) continue;
        yaml += `${indent}    - ${itemKey}: ${item[itemKey]}\n`;
        if (item.wrap) yaml += `${indent}      wrap: ${item.wrap}\n`;
        if (item.overrides) {
          yaml += `${indent}      overrides:\n`;
          for (const [type, ov] of Object.entries(item.overrides)) {
            yaml += `${indent}        ${type}: { suppress: true }\n`;
          }
        }
      }
      yaml += `${indent}  delimiter: ${component.delimiter || 'none'}\n`;
      if (component.prefix) yaml += `${indent}  prefix: "${component.prefix}"\n`;
      continue;
    }

    const mainKey = Object.keys(component).find(k => MAIN_KEYS.has(k));
    if (!mainKey) continue;

    yaml += `${indent}- ${mainKey}: ${component[mainKey]}\n`;
    if (component.form) yaml += `${indent}  form: ${component.form}\n`;
    if (component.emph) yaml += `${indent}  emph: true\n`;
    if (component.wrap) yaml += `${indent}  wrap: ${component.wrap}\n`;
    if (component.prefix) yaml += `${indent}  prefix: "${component.prefix}"\n`;
    if (component['name-order']) yaml += `${indent}  name-order: ${component['name-order']}\n`;
    if (component['et-al']) {
      yaml += `${indent}  et-al:\n`;
      yaml += `${indent}    min: ${component['et-al'].min}\n`;
      yaml += `${indent}    use-first: ${component['et-al'].use_first}\n`;
    }
    if (component.delimiter) yaml += `${indent}  delimiter: "${component.delimiter}"\n`;

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

function inferTemplate(stylePath, section = 'bibliography') {
  if (!fs.existsSync(stylePath)) return null;
  const testItems = loadFixtures();
  const styleXml = fs.readFileSync(stylePath, 'utf8');

  let rendered;
  if (section === 'bibliography') {
    rendered = renderWithCiteproc(styleXml, testItems);
  } else {
    const citations = renderCitations(styleXml, testItems);
    if (!citations) return null;
    rendered = { entries: Object.values(citations).filter(c => c !== null), style: styleXml };
  }

  if (!rendered?.entries?.length) return null;

  const refByEntry = rendered.entries.map(entry => findRefDataForEntry(entry, testItems));
  const { consensusOrdering, typedComponents, componentFrequency } = findConsensusOrdering(rendered.entries, refByEntry);

  const MONOGRAPH_TYPES = new Set(['chapter', 'entry-encyclopedia', 'entry-dictionary', 'paper-conference']);
  if (consensusOrdering.includes('containerTitle')) {
    let hasSerial = false, hasMonograph = false;
    for (const [type, data] of Object.entries(typedComponents)) {
      if (type === 'unknown' || data.componentCounts['containerTitle'] / data.entries.length < 0.5) continue;
      if (MONOGRAPH_TYPES.has(type)) hasMonograph = true; else hasSerial = true;
    }
    const ctIdx = consensusOrdering.indexOf('containerTitle');
    if (hasSerial && hasMonograph) consensusOrdering.splice(ctIdx, 1, 'containerTitleMonograph', 'containerTitleSerial');
    else if (hasMonograph) consensusOrdering[ctIdx] = 'containerTitleMonograph';
  }

  const isVolumeIssueGrouped = detectVolumeIssueGrouping(rendered.entries, refByEntry);
  const prefixes = {};
  for (const name of consensusOrdering) {
    const patterns = detectPrefixPatterns(name, rendered.entries, refByEntry);
    if (patterns.globalWinner || Object.keys(patterns.overrides).length > 0) prefixes[name] = patterns;
  }

  const wrapPatterns = {};
  for (const name of ['issue', 'pages', 'year', 'volume']) {
    const patterns = detectWrapPatterns(name, rendered.entries, refByEntry);
    if (patterns.globalWinner || Object.keys(patterns.overrides).length > 0) wrapPatterns[name] = patterns;
  }

  const nameOrderPatterns = {};
  const etAlSettings = {};
  for (const [name, role] of Object.entries({ contributors: 'author', editors: 'editor' })) {
    nameOrderPatterns[name] = detectNameOrderPatterns(name, role, rendered.entries, refByEntry);
    const etAl = detectEtAl(name, rendered.entries, refByEntry, rendered.entries);
    if (etAl) etAlSettings[name] = etAl;
  }

  let template = [];
  let skipIssue = false;
  for (const componentName of consensusOrdering) {
    if (componentName === 'issue' && skipIssue) continue;
    const parserName = componentName.startsWith('containerTitle') ? 'containerTitle' : componentName;
    const entryIdx = rendered.entries.findIndex((e, i) => parseComponents(e, refByEntry[i])[parserName]?.found);
    if (entryIdx < 0) continue;

    if (componentName === 'volume' && isVolumeIssueGrouped && consensusOrdering.includes('issue')) {
      const issueComp = { number: 'issue' };
      const ip = wrapPatterns['issue'];
      if (ip?.globalWinner) issueComp.wrap = ip.globalWinner;
      if (ip?.overrides) {
        issueComp.overrides = {};
        for (const [t, w] of Object.entries(ip.overrides)) issueComp.overrides[t] = { wrap: w };
      }
      template.push({ items: [{ number: 'volume' }, issueComp], _componentName: 'volume' });
      skipIssue = true;
      continue;
    }

    const yamlComp = mapComponentToYaml(componentName, rendered.entries[entryIdx], refByEntry[entryIdx]);
    if (yamlComp) {
      const p = prefixes[componentName] || prefixes[parserName];
      if (p?.globalWinner) yamlComp.prefix = p.globalWinner;
      if (p?.overrides) {
        yamlComp.overrides = yamlComp.overrides || {};
        for (const [t, pfx] of Object.entries(p.overrides)) (yamlComp.overrides[t] = yamlComp.overrides[t] || {}).prefix = pfx;
      }
      const w = wrapPatterns[componentName];
      if (w?.globalWinner && !yamlComp.wrap) yamlComp.wrap = w.globalWinner;
      if (w?.overrides) {
        yamlComp.overrides = yamlComp.overrides || {};
        for (const [t, wr] of Object.entries(w.overrides)) (yamlComp.overrides[t] = yamlComp.overrides[t] || {}).wrap = wr;
      }
      const fmt = detectFormatting(parserName, rendered.entries, refByEntry);
      if (fmt) Object.assign(yamlComp, fmt);

      yamlComp._componentName = componentName;
      if (nameOrderPatterns[parserName]?.globalWinner) yamlComp['name-order'] = nameOrderPatterns[parserName].globalWinner;
      if (etAlSettings[parserName]) yamlComp['et-al'] = etAlSettings[parserName];

      template.push(yamlComp);
    }
  }

  const suppressions = detectSuppressions(consensusOrdering, typedComponents, componentFrequency, rendered, refByEntry);
  let delimiterConsensus = '. ';
  {
    const skipPairs = new Set(['contributors', 'year', 'editors']), delimCounts = {};
    for (let i = 0; i < rendered.entries.length; i++) {
      for (const det of detectDelimiters(rendered.entries[i], refByEntry[i])) {
        if (skipPairs.has(det.between[0]) || skipPairs.has(det.between[1])) continue;
        const d = det.delimiter.replace(/["\u201c\u201d]+/g, '');
        if (d.length >= 1 && d.length <= 4 && /^[.,;: ]+$/.test(d)) delimCounts[d] = (delimCounts[d] || 0) + 1;
      }
    }
    const best = Object.entries(delimCounts).sort((a, b) => b[1] - a[1]);
    if (best.length > 0) delimiterConsensus = best[0][0];
  }

  let entrySuffix = null;
  const suffixCounts = {};
  for (const text of rendered.entries) {
    const match = text.replace(/<[^>]+>/g, '').trim().match(/([.,;:]+)$/);
    if (match) suffixCounts[match[1]] = (suffixCounts[match[1]] || 0) + 1;
  }
  const bestSuffix = Object.entries(suffixCounts).sort((a, b) => b[1] - a[1]);
  if (bestSuffix.length > 0 && bestSuffix[0][1] / rendered.entries.length >= 0.7) entrySuffix = bestSuffix[0][0];

  for (let i = 1; i < template.length; i++) {
    for (let j = i - 1; j >= Math.max(0, i - 4); j--) {
      const pairDelim = findDelimiterConsensus(rendered.entries, refByEntry, template[j]._componentName || 'volume', template[i]._componentName, (j === i - 1 ? 0 : 0.2));
      if (pairDelim && pairDelim !== delimiterConsensus) {
        template[i].prefix = pairDelim;
        break;
      }
    }
  }

  template = applyContainerGrouping(template);

  let citationWrap = null;
  if (section === 'citation') {
    let parenCount = 0, bracketCount = 0;
    for (const text of rendered.entries) {
      const norm = normalizeText(text);
      if (norm.startsWith('(') && norm.endsWith(')')) parenCount++;
      else if (norm.startsWith('[') && norm.endsWith(']')) bracketCount++;
    }
    if (parenCount / rendered.entries.length >= 0.8) citationWrap = 'parentheses';
    else if (bracketCount / rendered.entries.length >= 0.8) citationWrap = 'brackets';
  }

  for (const comp of template) {
    if (comp._componentName && suppressions[comp._componentName]) {
      for (const t of Object.keys(suppressions[comp._componentName])) (comp.overrides = comp.overrides || {})[t] = { suppress: true };
    }
    if (comp.items && suppressions.issue) {
      const issue = comp.items.find(it => it.number === 'issue');
      if (issue) for (const t of Object.keys(suppressions.issue)) (issue.overrides = issue.overrides || {})[t] = { suppress: true };
    }
    delete comp._componentName;
  }

  return {
    template,
    yaml: generateYaml(template, delimiterConsensus, citationWrap),
    meta: {
      delimiterConsensus,
      entrySuffix,
      typesAnalyzed: Object.keys(typedComponents),
      entryCount: rendered.entries.length,
      confidence: 0.85,
      section,
      wrap: citationWrap
    }
  };
}

function applyContainerGrouping(template) {
  const containerTypes = ['editors', 'containerTitleMonograph'], newTemplate = [];
  for (let i = 0; i < template.length; i++) {
    const comp = template[i];
    if (containerTypes.includes(comp._componentName)) {
      let j = i + 1;
      while (j < template.length && containerTypes.includes(template[j]._componentName)) j++;
      if (j > i + 1) {
        const groupItems = template.slice(i, j);
        const group = { items: groupItems.map(c => { const { _componentName, ...rest } = c; return rest; }), _componentName: 'containerGroup' };
        if (groupItems[0].prefix) { group.prefix = groupItems[0].prefix; delete group.items[0].prefix; }
        if (group.items[1].prefix) { group.delimiter = group.items[1].prefix; delete group.items[1].prefix; }
        newTemplate.push(group); i = j - 1; continue;
      }
    }
    newTemplate.push(comp);
  }
  return newTemplate;
}

module.exports = {
  inferTemplate,
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
