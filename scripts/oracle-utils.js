/**
 * Shared utilities for oracle scripts (oracle.js and oracle-yaml.js).
 *
 * Extracts common functions to enable consistent component parsing,
 * text normalization, and reference data lookup across oracle variants.
 */

'use strict';

const fs = require('fs');
const path = require('path');

// -- Text normalization --

/**
 * Normalize text by stripping HTML, markdown formatting, and
 * collapsing whitespace.
 */
function normalizeText(text) {
  return text
    .replace(/<[^>]+>/g, '')           // Strip HTML tags
    .replace(/&#38;/g, '&')            // HTML entity for &
    .replace(/_([^_]+)_/g, '$1')       // Strip markdown italics
    .replace(/\*\*([^*]+)\*\*/g, '$1') // Strip markdown bold
    .replace(/\bet al\./gi, 'et al')   // Normalize equivalent et-al punctuation
    // Strip bibliography numbering prefix (allow hidden directional marks).
    .replace(/^[\s\u200e\u200f\u202a-\u202e\u2066-\u2069]*\d+\.\s*/, '')
    .replace(/\s+([,.;:])/g, '$1')     // Normalize stray spaces before punctuation
    .replace(/\s+/g, ' ')             // Normalize whitespace
    .trim();
}

// -- Reference data lookup --

/**
 * Find the reference data matching a bibliography entry.
 *
 * Uses multi-field scoring (author + year + title) to disambiguate
 * entries that share an author name.
 *
 * @param {string} entry - The bibliography entry text
 * @param {Object} testItems - Map of item IDs to reference data
 * @returns {Object|null} The best-matching reference, or null
 */
function findRefDataForEntry(entry, testItems) {
  const entryLower = normalizeText(entry).toLowerCase();
  const candidates = [];

  for (const [id, ref] of Object.entries(testItems)) {
    let score = 0;
    let nameMatch = false;

    // Check author family name
    if (ref.author && ref.author.length > 0) {
      const first = ref.author[0];
      const name = (first.family || first.literal || '').toLowerCase();
      if (name && entryLower.includes(name)) {
        score += 1;
        nameMatch = true;
      }
    }

    // Check editor if no author match
    if (!nameMatch && ref.editor && ref.editor.length > 0) {
      const first = ref.editor[0];
      const name = (first.family || first.literal || '').toLowerCase();
      if (name && entryLower.includes(name)) {
        score += 1;
        nameMatch = true;
      }
    }

    if (!nameMatch) continue;

    // Year match (strongly discriminating)
    if (ref.issued && ref.issued['date-parts'] && ref.issued['date-parts'][0]) {
      const year = String(ref.issued['date-parts'][0][0]);
      if (entryLower.includes(year)) {
        score += 2;
      }
    }

    // Title match (full title is the strongest signal)
    if (ref.title) {
      const titleLower = normalizeText(ref.title).toLowerCase();
      if (entryLower.includes(titleLower)) {
        score += 4;
      } else {
        // Partial title: use first 40 chars (styles may sentence-case or truncate)
        const prefix = titleLower.substring(0, Math.min(40, titleLower.length));
        if (prefix.length >= 10 && entryLower.includes(prefix)) {
          score += 2;
        }
      }
    }

    candidates.push({ id, ref, score });
  }

  if (candidates.length === 0) return null;

  // Return highest-scoring candidate
  candidates.sort((a, b) => b.score - a.score);
  return candidates[0].ref;
}

// -- Position helpers --

function escapeRegex(str) {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Find the position of a string field value in the entry.
 * Uses case-insensitive full-field matching.
 *
 * @returns {{ start: number, end: number }|null}
 */
function findFieldPosition(entryLower, fieldValue) {
  if (!fieldValue) return null;
  const normalized = normalizeText(String(fieldValue)).toLowerCase();
  if (!normalized) return null;

  const idx = entryLower.indexOf(normalized);
  if (idx >= 0) {
    return { start: idx, end: idx + normalized.length };
  }
  return null;
}

/**
 * Find a numeric field value with digit-boundary guards to prevent
 * false positives. For example, volume "10" must not match "2010".
 *
 * @returns {{ start: number, end: number }|null}
 */
function findNumericFieldPosition(entryLower, value) {
  if (value == null) return null;
  const str = String(value);
  // Digit-boundary: not preceded or followed by another digit
  const regex = new RegExp(`(?<![0-9])${escapeRegex(str)}(?![0-9])`);
  const match = entryLower.match(regex);
  if (match) {
    return { start: match.index, end: match.index + str.length };
  }
  return null;
}

/**
 * Expand a name component position to include given name or initials if nearby.
 * @param {string} entryLower - Normalized lower-case entry text
 * @param {Object} familyPos - {start, end} of the family name
 * @param {string} givenName - The given name to search for
 */
function expandNamePosition(entryLower, familyPos, givenName) {
  if (!familyPos) return null;

  if (!givenName) return familyPos;

  const givenLower = givenName.toLowerCase();
  const givenInitial = givenName.charAt(0).toLowerCase();

  // Try finding full given name or initial near the family name
  const searchWindow = 50;
  const start = Math.max(0, familyPos.start - searchWindow);
  const end = Math.min(entryLower.length, familyPos.end + searchWindow);
  const text = entryLower.substring(start, end);

  let givenPos = null;
  let givenLen = 0;

  // 1. Try full given name
  const idxFull = text.indexOf(givenLower);
  if (idxFull !== -1) {
    givenPos = start + idxFull;
    givenLen = givenLower.length;
  } else {
    // 2. Try initial with word boundary
    const initialRegex = new RegExp(`\\b${escapeRegex(givenInitial)}\\.?`, 'i');
    const match = text.match(initialRegex);
    if (match) {
      givenPos = start + match.index;
      givenLen = match[0].length;
    }
  }

  if (givenPos !== null) {
    // Merge ranges
    const mergedStart = Math.min(familyPos.start, givenPos);
    const mergedEnd = Math.max(familyPos.end, givenPos + givenLen);
    return { start: mergedStart, end: mergedEnd };
  }

  return familyPos;
}

// -- Component parsing --

/**
 * Parse rendered bibliography entry into semantic components.
 *
 * @param {string} entry - The bibliography entry text
 * @param {Object} refData - The reference data (from JSON fixture)
 * @returns {Object} Components found: { contributors, year, title, ... }
 */
function parseComponents(entry, refData) {
  const result = {
    contributors: { found: false, value: null, pos: null },
    year: { found: false, value: null, pos: null },
    title: { found: false, value: null, pos: null },
    containerTitle: { found: false, value: null, pos: null },
    volume: { found: false, value: null, pos: null },
    issue: { found: false, value: null, pos: null },
    pages: { found: false, value: null, pos: null },
    publisher: { found: false, value: null, pos: null },
    doi: { found: false, value: null, pos: null },
    edition: { found: false, value: null, pos: null },
    editors: { found: false, value: null, pos: null }
  };

  const entryLower = normalizeText(entry).toLowerCase();

  // Contributors (authors/editors)
  if (refData.author && refData.author.length > 0) {
    const first = refData.author[0];
    const familyName = (first.family || first.literal || '').toLowerCase();
    if (familyName) {
      const familyPos = findFieldPosition(entryLower, familyName);
      const fullPos = expandNamePosition(entryLower, familyPos, first.given);
      if (fullPos) {
        result.contributors = {
          found: true,
          value: entry.substring(fullPos.start, fullPos.end),
          pos: fullPos
        };
      }
    }
  }

  // Year
  if (refData.issued && refData.issued['date-parts'] && refData.issued['date-parts'][0]) {
    const year = String(refData.issued['date-parts'][0][0]);
    const yearPos = findNumericFieldPosition(entryLower, year);
    if (yearPos) {
      result.year = {
        found: true,
        value: entry.substring(yearPos.start, yearPos.end),
        pos: yearPos
      };
    }
  }

  // Title
  if (refData.title) {
    const titlePos = findFieldPosition(entryLower, refData.title);
    if (titlePos) {
      result.title = {
        found: true,
        value: entry.substring(titlePos.start, titlePos.end),
        pos: titlePos
      };
    }
  }

  // Container title (journal, book series, etc.)
  if (refData['container-title']) {
    const containerPos = findFieldPosition(entryLower, refData['container-title']);
    if (containerPos) {
      result.containerTitle = {
        found: true,
        value: entry.substring(containerPos.start, containerPos.end),
        pos: containerPos
      };
    }
  }

  // Volume
  if (refData.volume != null) {
    const volPos = findNumericFieldPosition(entryLower, refData.volume);
    if (volPos) {
      result.volume = {
        found: true,
        value: entry.substring(volPos.start, volPos.end),
        pos: volPos
      };
    }
  }

  // Issue
  if (refData.issue != null) {
    const issuePos = findNumericFieldPosition(entryLower, refData.issue);
    if (issuePos) {
      result.issue = {
        found: true,
        value: entry.substring(issuePos.start, issuePos.end),
        pos: issuePos
      };
    }
  }

  // Pages
  if (refData.page) {
    const pagePos = findFieldPosition(entryLower, refData.page);
    if (pagePos) {
      result.pages = {
        found: true,
        value: entry.substring(pagePos.start, pagePos.end),
        pos: pagePos
      };
    }
  }

  // Publisher
  if (refData.publisher) {
    const pubPos = findFieldPosition(entryLower, refData.publisher);
    if (pubPos) {
      result.publisher = {
        found: true,
        value: entry.substring(pubPos.start, pubPos.end),
        pos: pubPos
      };
    }
  }

  // DOI
  if (refData.DOI) {
    const doiPos = findFieldPosition(entryLower, refData.DOI);
    if (doiPos) {
      result.doi = {
        found: true,
        value: entry.substring(doiPos.start, doiPos.end),
        pos: doiPos
      };
    }
  }

  // Edition
  if (refData.edition != null) {
    const edPos = findFieldPosition(entryLower, String(refData.edition));
    if (edPos) {
      result.edition = {
        found: true,
        value: entry.substring(edPos.start, edPos.end),
        pos: edPos
      };
    }
  }

  // Editors
  if (refData.editor && refData.editor.length > 0) {
    const first = refData.editor[0];
    const familyName = (first.family || first.literal || '').toLowerCase();
    if (familyName) {
      const familyPos = findFieldPosition(entryLower, familyName);
      const fullPos = expandNamePosition(entryLower, familyPos, first.given);
      if (fullPos) {
        result.editors = {
          found: true,
          value: entry.substring(fullPos.start, fullPos.end),
          pos: fullPos
        };
      }
    }
  }

  return result;
}

/**
 * Analyze component ordering in a bibliography entry.
 *
 * @param {Object} components - Parsed components from parseComponents()
 * @returns {Array<string>} Ordered list of component names by position
 */
function analyzeOrdering(components) {
  const positioned = Object.entries(components)
    .filter(([_, data]) => data.found && data.pos)
    .map(([name, data]) => ({ name, pos: data.pos }))
    .sort((a, b) => a.pos.start - b.pos.start);

  return positioned.map(item => item.name);
}

/**
 * Load a locale XML file.
 * @param {string} lang - Language code (e.g., 'en-US')
 * @returns {string} XML content
 */
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

module.exports = {
  normalizeText,
  parseComponents,
  analyzeOrdering,
  findRefDataForEntry,
  loadLocale,
  findFieldPosition,
  findNumericFieldPosition,
  expandNamePosition,
  escapeRegex
};
