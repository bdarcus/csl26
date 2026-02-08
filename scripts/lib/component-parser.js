/**
 * Hardened component parser for oracle.js.
 *
 * Replaces fragile substring prefix matching with exact field-aware
 * matching against known reference data. Prioritizes precision over
 * recall: false negatives are acceptable, false positives are not.
 *
 * Key improvements over the original parseComponents():
 * - Title: full-field match (was 20-char prefix)
 * - Container-title: full-field match (was 15-char prefix)
 * - Publisher: full-field match (was 10-char prefix)
 * - findRefDataForEntry: multi-field scoring (was first-author-only)
 * - Volume/issue: word-boundary matching (was includes())
 * - All components track their position in the entry string
 */

'use strict';

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
    .replace(/\s+/g, ' ')             // Normalize whitespace
    .trim();
}

// -- Reference data lookup --

/**
 * Find the reference data matching a bibliography entry.
 *
 * Uses multi-field scoring (author + year + title) to disambiguate
 * entries that share an author name. This fixes the original
 * findRefDataForEntry which returned the first author match
 * regardless of whether multiple refs shared that author.
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
    // 2. Try initial
    // Basic check for "g." or "g "
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

// -- Component extraction --

/**
 * Extract structured components from a bibliography entry.
 *
 * Uses exact field matching against known reference data instead of
 * substring prefix heuristics. Every component includes a position
 * for ordering analysis and delimiter detection.
 *
 * @param {string} entry - The bibliography entry text
 * @param {Object|null} refData - The reference data for this entry
 * @returns {Object} Structured components with found/value/position
 */
function parseComponents(entry, refData) {
  const normalized = normalizeText(entry);
  const entryLower = normalized.toLowerCase();

  const components = {
    raw: entry,
    contributors: { found: false, value: null, position: null },
    year: { found: false, value: null, position: null },
    title: { found: false, value: null, position: null },
    containerTitle: { found: false, value: null, position: null },
    volume: { found: false, value: null, position: null },
    issue: { found: false, value: null, position: null },
    pages: { found: false, value: null, position: null },
    publisher: { found: false, value: null, position: null },
    place: { found: false, value: null, position: null },
    doi: { found: false, value: null, position: null },
    url: { found: false, value: null, position: null },
    edition: { found: false, value: null, position: null },
    editors: { found: false, value: null, position: null },
  };

  // === Pattern-based components (not dependent on refData) ===

  // Year in parentheses: (2020) — don't capture trailing period (it's a delimiter)
  const yearParensMatch = normalized.match(/\((\d{4})\)/);
  // Year standalone: 2020 followed by punctuation (don't capture the punctuation)
  const yearStandaloneMatch = normalized.match(/(?:^|\s)(\d{4})(?=[.,])/);
  const yearMatch = yearParensMatch || yearStandaloneMatch;
  if (yearMatch) {
    const idx = normalized.indexOf(yearMatch[0]);
    if (yearParensMatch) {
      // Parenthesized: position covers "(2020)" — wrap is part of the component
      components.year = {
        found: true,
        value: yearMatch[1],
        position: { start: idx, end: idx + yearMatch[0].length },
      };
    } else {
      // Standalone: position covers just the 4-digit year (skip leading space)
      const yearStart = idx + yearMatch[0].indexOf(yearMatch[1]);
      components.year = {
        found: true,
        value: yearMatch[1],
        position: { start: yearStart, end: yearStart + yearMatch[1].length },
      };
    }
  }

  // DOI
  const doiMatch = normalized.match(
    /(?:https?:\/\/doi\.org\/|doi:\s*|DOI:\s*)(10\.\d+\/[^\s]+)/i
  );
  if (doiMatch) {
    const idx = normalized.indexOf(doiMatch[0]);
    components.doi = {
      found: true,
      value: doiMatch[1],
      position: { start: idx, end: idx + doiMatch[0].length },
    };
  }

  // URL (only if no DOI found)
  if (!components.doi.found) {
    const urlMatch = normalized.match(/https?:\/\/[^\s]+/i);
    if (urlMatch) {
      const idx = normalized.indexOf(urlMatch[0]);
      components.url = {
        found: true,
        value: urlMatch[0],
        position: { start: idx, end: idx + urlMatch[0].length },
      };
    }
  }

  // Page ranges: pp. 123-456, 123-456, 123–456
  const pagesMatch = normalized.match(/(?:pp?\.\s*)?(\d+)[\-–](\d+)/);
  if (pagesMatch) {
    const idx = normalized.indexOf(pagesMatch[0]);
    components.pages = {
      found: true,
      value: `${pagesMatch[1]}-${pagesMatch[2]}`,
      position: { start: idx, end: idx + pagesMatch[0].length },
    };
  }

  // Edition: 2nd ed., Silver Anniversary Edition
  const editionMatch = normalized.match(
    /(\d+(?:st|nd|rd|th)\s+ed\.|[A-Za-z\s]+Edition)/i
  );
  if (editionMatch) {
    const idx = normalized.indexOf(editionMatch[0]);
    components.edition = {
      found: true,
      value: editionMatch[1],
      position: { start: idx, end: idx + editionMatch[0].length },
    };
  }

  // Editor markers: (Ed.), (Eds.), edited by
  const editorsMatch = normalized.match(/\(Eds?\.\)|edited by|Ed\.|Eds\./i);
  if (editorsMatch) {
    const idx = normalized.indexOf(editorsMatch[0]);
    components.editors = {
      found: true,
      value: true,
      position: { start: idx, end: idx + editorsMatch[0].length },
    };
  }

  // === Reference-data-driven components ===

  if (refData) {
    // Contributors: first author (or editor if no author)
    const principalNames = (refData.author && refData.author.length > 0)
      ? refData.author : refData.editor;

    if (principalNames && principalNames.length > 0) {
      const first = principalNames[0];
      const family = first.family || first.literal || '';
      const given = first.given || '';
      if (family) {
        let pos = findFieldPosition(entryLower, family);
        if (pos) {
          pos = expandNamePosition(entryLower, pos, given);
          components.contributors = { found: true, value: family, position: pos };
        }
      }
    }

    // Secondary editors (only if not already the principal contributor)
    if (refData.editor && refData.editor.length > 0) {
      const firstEditor = refData.editor[0];
      const family = firstEditor.family || firstEditor.literal || '';
      const given = firstEditor.given || '';
      if (family) {
        let idx = entryLower.indexOf(family.toLowerCase());

        // Check for overlap with contributors (author)
        if (idx !== -1 && components.contributors.found &&
          idx >= components.contributors.position.start &&
          idx < components.contributors.position.end) {
          // Overlap! Search for next occurrence
          idx = entryLower.indexOf(family.toLowerCase(), idx + 1);
        }

        if (idx !== -1) {
          let pos = { start: idx, end: idx + family.length };
          // Expand using given name if possible
          pos = expandNamePosition(entryLower, pos, given);

          components.editors = {
            found: true,
            value: family,
            position: pos
          };
        }
      }
    }

    // Title: full-field exact match
    if (refData.title) {
      const pos = findFieldPosition(entryLower, refData.title);
      if (pos) {
        components.title = { found: true, value: refData.title, position: pos };
      }
    }

    // Container-title: full-field exact match
    if (refData['container-title']) {
      const pos = findFieldPosition(entryLower, refData['container-title']);
      if (pos) {
        components.containerTitle = {
          found: true,
          value: refData['container-title'],
          position: pos,
        };
      }
    }

    // Publisher: full-field exact match
    if (refData.publisher) {
      const pos = findFieldPosition(entryLower, refData.publisher);
      if (pos) {
        components.publisher = {
          found: true,
          value: refData.publisher,
          position: pos,
        };
      }
    }

    // Publisher-place
    if (refData['publisher-place']) {
      const pos = findFieldPosition(entryLower, refData['publisher-place']);
      if (pos) {
        components.place = {
          found: true,
          value: refData['publisher-place'],
          position: pos,
        };
      }
    }

    // Volume/issue: prefer combined vol(issue) pattern first
    const volIssueMatch = normalized.match(/(\d+)\s*\((\d+)\)/);
    if (volIssueMatch) {
      const matchedVol = volIssueMatch[1];
      const matchedIssue = volIssueMatch[2];
      const idx = normalized.indexOf(volIssueMatch[0]);

      if (refData.volume && matchedVol === String(refData.volume)) {
        components.volume = {
          found: true,
          value: matchedVol,
          position: { start: idx, end: idx + matchedVol.length },
        };
      }
      if (refData.issue && matchedIssue === String(refData.issue)) {
        components.issue = {
          found: true,
          value: matchedIssue,
          position: {
            start: idx + matchedVol.length + 1,
            end: idx + volIssueMatch[0].length - 1,
          },
        };
      }
    } else {
      // Individual volume/issue with digit-boundary guards
      if (refData.volume) {
        const pos = findNumericFieldPosition(entryLower, refData.volume);
        if (pos) {
          components.volume = {
            found: true,
            value: String(refData.volume),
            position: pos,
          };
        }
      }
      if (refData.issue) {
        const pos = findNumericFieldPosition(entryLower, refData.issue);
        if (pos) {
          components.issue = {
            found: true,
            value: String(refData.issue),
            position: pos,
          };
        }
      }
    }
  }

  return components;
}

// -- Ordering analysis --

/**
 * Analyze component ordering by their positions in the entry.
 * Returns component names in the order they appear.
 *
 * @param {string} entry - The bibliography entry text
 * @param {Object|null} refData - The reference data for this entry
 * @returns {string[]} Component names sorted by position
 */
function analyzeOrdering(entry, refData) {
  const components = parseComponents(entry, refData);

  const positioned = [];
  for (const [name, comp] of Object.entries(components)) {
    if (name === 'raw') continue;
    if (comp.found && comp.position) {
      positioned.push({ name, start: comp.position.start });
    }
  }

  positioned.sort((a, b) => a.start - b.start);
  return positioned.map(p => p.name);
}

// -- Delimiter detection --

/**
 * Detect delimiters between adjacent matched components.
 *
 * Extracts the literal text between the end of one component and the
 * start of the next. Useful for inferring style-specific separators
 * (". " vs ", " vs ": ") for the template inferrer.
 *
 * @param {string} entry - The bibliography entry text
 * @param {Object|null} refData - The reference data for this entry
 * @returns {Array<{ between: [string, string], delimiter: string }>}
 */
function detectDelimiters(entry, refData) {
  const normalized = normalizeText(entry);
  const components = parseComponents(entry, refData);
  const delimiters = [];

  // Collect positioned components
  const positioned = [];
  for (const [name, comp] of Object.entries(components)) {
    if (name === 'raw') continue;
    if (comp.found && comp.position) {
      positioned.push({ name, start: comp.position.start, end: comp.position.end });
    }
  }

  positioned.sort((a, b) => a.start - b.start);

  // Extract text between each adjacent pair
  for (let i = 0; i < positioned.length - 1; i++) {
    const current = positioned[i];
    const next = positioned[i + 1];

    // Only capture short delimiters (avoid capturing large unmatched spans)
    if (next.start > current.end && (next.start - current.end) <= 15) {
      const between = normalized.substring(current.end, next.start);
      delimiters.push({
        between: [current.name, next.name],
        delimiter: between,
      });
    }
  }

  return delimiters;
}

module.exports = {
  normalizeText,
  findRefDataForEntry,
  parseComponents,
  analyzeOrdering,
  detectDelimiters,
  // Exported for testing
  findFieldPosition,
  findNumericFieldPosition,
};
