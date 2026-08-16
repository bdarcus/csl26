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
  const monthMap = {
    January: 'Jan',
    February: 'Feb',
    March: 'Mar',
    April: 'Apr',
    May: 'May',
    June: 'Jun',
    July: 'Jul',
    August: 'Aug',
    September: 'Sep',
    October: 'Oct',
    November: 'Nov',
    December: 'Dec',
  };

  return text
    .replace(/<[^>]+>/g, '')           // Strip HTML tags
    .replace(/&#38;/g, '&')            // HTML entity for &
    .replace(/_([^_]+)_/g, '$1')       // Strip markdown italics
    .replace(/\*\*([^*]+)\*\*/g, '$1') // Strip markdown bold
    .replace(/\[(Internet)\]/gi, '')   // Normalize optional medium marker
    .replace(/\bsec\.?\b/gi, 'section')
    .replace(/\((eds?|ed)\.\)/gi, 'editors')
    .replace(/\b(eds?|ed)\.?\b/gi, 'editors')
    .replace(
      /\[(cited)\s+(\d{4}),?\s+([A-Za-z]+)\s+(\d{1,2})\]/gi,
      (_, a, y, m, d) => `[${a} ${y} ${m} ${d}]`
    )
    .replace(
      /(Accessed)\s+(\d{4}),?\s+([A-Za-z]+)\s+(\d{1,2})/gi,
      (_, a, y, m, d) => `${a} ${d} ${m} ${y}`
    )
    .replace(
      /\b(January|February|March|April|May|June|July|August|September|October|November|December)\b/g,
      m => monthMap[m] || m
    )
    .replace(/\bet al\./gi, 'et al')   // Normalize equivalent et-al punctuation
    .replace(/;\./g, ';')
    .replace(/\s+([,.;:])/g, '$1')     // Normalize stray spaces before punctuation
    .replace(/\s+/g, ' ')             // Normalize whitespace
    // Strip bibliography numbering prefix after whitespace normalization (allow hidden directional marks).
    // Only strip when the numeric label is followed by actual entry content.
    .replace(/^[\u200e\u200f\u202a-\u202e\u2066-\u2069]*\d+\.(?=[^\d])\s*/, '')
    .trim();
}

/**
 * Normalize renderer output for exact textual parity.
 *
 * This removes transport-only markup while preserving every visible text
 * character, including numbering, case, punctuation, brackets, and labels.
 *
 * @param {string} text
 * @returns {string}
 */
function normalizeExactText(text) {
  return String(text ?? '')
    .replace(/<[^>]+>/g, '')
    .replace(/&#38;/g, '&')
    .replace(/&amp;/g, '&')
    .replace(/_([^_]+)_/g, '$1')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Tokenize normalized text for order/punctuation-tolerant similarity checks.
 *
 * Similarity intentionally remains case-insensitive so punctuation/order
 * regressions can still be matched after exact case-aware equality fails.
 *
 * @param {string} text
 * @returns {string[]}
 */
function tokenizeForSimilarity(text) {
  return normalizeText(text || '')
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s]/gu, ' ')
    .split(/\s+/)
    .filter(Boolean)
    .filter((token) => token.length > 1);
}

/**
 * Compute bag-of-words similarity for normalized text.
 *
 * @param {string} leftText
 * @param {string} rightText
 * @returns {number}
 */
function textSimilarity(leftText, rightText) {
  const left = tokenizeForSimilarity(leftText);
  const right = tokenizeForSimilarity(rightText);
  if (left.length === 0 && right.length === 0) return 1;
  if (left.length === 0 || right.length === 0) return 0;

  const leftCounts = new Map();
  const rightCounts = new Map();
  for (const token of left) {
    leftCounts.set(token, (leftCounts.get(token) || 0) + 1);
  }
  for (const token of right) {
    rightCounts.set(token, (rightCounts.get(token) || 0) + 1);
  }

  let intersect = 0;
  let union = 0;
  const keys = new Set([...leftCounts.keys(), ...rightCounts.keys()]);
  for (const key of keys) {
    const leftCount = leftCounts.get(key) || 0;
    const rightCount = rightCounts.get(key) || 0;
    intersect += Math.min(leftCount, rightCount);
    union += Math.max(leftCount, rightCount);
  }

  return union > 0 ? intersect / union : 0;
}

/**
 * Return true when texts differ only by letter case after normalization.
 *
 * @param {string} leftText
 * @param {string} rightText
 * @returns {boolean}
 */
function isCaseOnlyMismatch(leftText, rightText) {
  const left = normalizeText(leftText);
  const right = normalizeText(rightText);
  return left !== right && left.toLowerCase() === right.toLowerCase();
}

/**
 * Compare texts with exact case-aware equality first, then similarity fallback.
 *
 * @param {string} expectedText
 * @param {string} actualText
 * @param {{ caseSensitive?: boolean, similarityThreshold?: number }} [options]
 * @returns {{
 *   rawExpected: string,
 *   rawActual: string,
 *   exactExpected: string,
 *   exactActual: string,
 *   exactMatch: boolean,
 *   exactAdjudication: 'matched' | 'unresolved',
 *   expected: string,
 *   actual: string,
 *   match: boolean,
 *   caseMismatch: boolean,
 *   similarity: number
 * }}
 */
function compareText(expectedText, actualText, options = {}) {
  const caseSensitive = options.caseSensitive !== false;
  const similarityThreshold = options.similarityThreshold ?? 0.60;
  const rawExpected = String(expectedText ?? '');
  const rawActual = String(actualText ?? '');
  const exactExpected = normalizeExactText(rawExpected);
  const exactActual = normalizeExactText(rawActual);
  const exactMatch = exactExpected === exactActual;
  const expected = normalizeText(rawExpected);
  const actual = normalizeText(rawActual);
  const parity = {
    rawExpected,
    rawActual,
    exactExpected,
    exactActual,
    exactMatch,
    exactAdjudication: exactMatch ? 'matched' : 'unresolved',
  };

  if (expected === actual) {
    return { ...parity, expected, actual, match: true, caseMismatch: false, similarity: 1 };
  }

  // Case-mismatch detection must stay meaningful even when expected/actual also
  // differ by trailing punctuation (e.g. one side is missing a terminal period) --
  // otherwise a real case defect goes undetected just because an unrelated
  // punctuation defect happens to co-occur on the same entry. Comparing the
  // punctuation-stripped forms for inequality (not just their lowercased forms)
  // keeps a pure trailing-punctuation difference from being misreported as a
  // case mismatch: two strings that are already identical once trailing
  // punctuation is ignored are trivially case-insensitive-equal too, but that's
  // not a case defect.
  const stripTrailingPunctuation = (text) => text.replace(/[.,;:]\s*$/, '');
  const strippedExpected = stripTrailingPunctuation(expected);
  const strippedActual = stripTrailingPunctuation(actual);
  const caseMismatch =
    strippedExpected !== strippedActual && strippedExpected.toLowerCase() === strippedActual.toLowerCase();
  if (caseSensitive && caseMismatch) {
    return { ...parity, expected, actual, match: false, caseMismatch: true, similarity: 1 };
  }

  const similarity = textSimilarity(expected, actual);
  return {
    ...parity,
    expected,
    actual,
    match: similarity >= similarityThreshold,
    caseMismatch,
    similarity,
  };
}

// -- Reference data lookup --

/**
 * Return true when a reference has any primary person-like contributor.
 *
 * @param {Object} ref
 * @returns {boolean}
 */
function hasPrimaryNames(ref) {
  return ['author', 'editor', 'translator', 'interviewer', 'recipient'].some(
    (role) => Array.isArray(ref?.[role]) && ref[role].length > 0
  );
}

function findRefMatchForEntry(entry, testItems) {
  const entryLower = normalizeText(entry).toLowerCase();
  const candidates = [];
  const nameRoles = ['author', 'editor', 'translator', 'interviewer', 'recipient'];

  for (const [id, ref] of Object.entries(testItems)) {
    let score = 0;
    let nameMatch = false;

    for (const role of nameRoles) {
      if (nameMatch || !ref[role] || ref[role].length === 0) continue;
      const first = ref[role][0];
      const name = (first.family || first.literal || '').toLowerCase();
      if (name && entryLower.includes(name)) {
        score += 1;
        nameMatch = true;
      }
    }

    if (!nameMatch) {
      // Fall back to title for anonymous items AND items whose only primary names
      // are corporate literals (e.g. {literal: "State of JS Team"}) — these may
      // not appear verbatim in oracle text when expanded differently.
      const hasOnlyLiteralNames = hasPrimaryNames(ref) &&
        ['author', 'editor', 'translator', 'interviewer', 'recipient']
          .flatMap((role) => ref[role] || [])
          .filter(Boolean)
          .every((n) => n.literal && !n.family);
      if ((!hasPrimaryNames(ref) || hasOnlyLiteralNames) && ref.title) {
        const titleLower = normalizeText(ref.title).toLowerCase();
        if (entryLower.includes(titleLower)) {
          score += 4;
        } else {
          const prefix = titleLower.substring(0, Math.min(40, titleLower.length));
          if (prefix.length >= 10 && entryLower.includes(prefix)) {
            score += 2;
          }
        }
      }
      if (score === 0) continue;
    }

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

    candidates.push({ id: ref.id || id, ref, score });
  }

  if (candidates.length === 0) return null;

  // Return highest-scoring candidate
  candidates.sort((a, b) => b.score - a.score);
  return candidates[0];
}

function findRefDataForEntry(entry, testItems) {
  return findRefMatchForEntry(entry, testItems)?.ref || null;
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

function positionsOverlap(left, right) {
  if (!left || !right) return false;
  return left.start < right.end && right.start < left.end;
}

function sliceMatchedValue(normalizedEntry, pos) {
  if (!pos) return null;
  return normalizedEntry.substring(pos.start, pos.end);
}

function findContributorComponent(normalizedEntry, names, occupiedPositions = []) {
  if (!Array.isArray(names) || names.length === 0) return null;

  const first = names[0];
  const family = first.family || first.literal || '';
  const given = first.given || '';
  if (!family) return null;

  const entryLower = normalizedEntry.toLowerCase();
  let idx = entryLower.indexOf(family.toLowerCase());
  while (idx !== -1) {
    let pos = { start: idx, end: idx + family.length };
    pos = expandNamePosition(entryLower, pos, given);
    if (!occupiedPositions.some((occupied) => positionsOverlap(occupied, pos))) {
      return {
        found: true,
        value: sliceMatchedValue(normalizedEntry, pos),
        pos,
      };
    }
    idx = entryLower.indexOf(family.toLowerCase(), idx + 1);
  }

  return null;
}

/**
 * Find a cite item's own primary-name occurrence for segment-anchoring
 * purposes -- family name only, deliberately not expanded to a nearby
 * given-name/initial the way `findContributorComponent` does.
 *
 * `findContributorComponent`'s given-name expansion scans a 50-character
 * window around the family name, which is safe within a single
 * bibliography entry (one person's name, one nearby initial) but not
 * within a multi-item citation cluster: a short cluster like
 * "(Smith 2020; Jones 2021)" puts a *different* person's initial ("J")
 * well within 50 characters of "Smith", so the expansion would sweep
 * "Jones" into "Smith"'s anchor. Segmentation only needs a stable
 * boundary, not the full name span, so this intentionally skips that
 * expansion; `parseComponents`/`compareComponents` still does full
 * given-name-aware component comparison afterward, once each item's text
 * has already been sliced down to its own segment.
 */
function findFamilyNameAnchor(normalizedEntry, names, occupiedPositions = []) {
  if (!Array.isArray(names) || names.length === 0) return null;

  const family = names[0].family || names[0].literal || '';
  if (!family) return null;

  const entryLower = normalizedEntry.toLowerCase();
  const familyLower = family.toLowerCase();
  let idx = entryLower.indexOf(familyLower);
  while (idx !== -1) {
    const pos = { start: idx, end: idx + family.length };
    if (!occupiedPositions.some((occupied) => positionsOverlap(occupied, pos))) {
      return { found: true, value: sliceMatchedValue(normalizedEntry, pos), pos };
    }
    idx = entryLower.indexOf(familyLower, idx + 1);
  }
  return null;
}

/**
 * Find a reference's issued year within a rendered citation, with an
 * optional trailing single-letter disambiguation suffix (e.g. "2019a").
 * Used as the fallback anchor for `splitCitationIntoItemSegments` when a
 * cite item's own author name has already been consumed by an earlier
 * item in the same cluster (the same-author-collapsed case, e.g. a
 * rendered "Garcia 2019a, 2019b" where "Garcia" occurs only once).
 */
function findYearComponent(normalizedEntry, refData, occupiedPositions = []) {
  const year = refData?.issued?.['date-parts']?.[0]?.[0];
  if (year == null) return null;

  const entryLower = normalizedEntry.toLowerCase();
  const regex = new RegExp(`(?<![0-9])${escapeRegex(String(year))}[a-z]?`, 'gi');
  let match = regex.exec(entryLower);
  while (match !== null) {
    const pos = { start: match.index, end: match.index + match[0].length };
    if (!occupiedPositions.some((occupied) => positionsOverlap(occupied, pos))) {
      return { found: true, value: sliceMatchedValue(normalizedEntry, pos), pos };
    }
    match = regex.exec(entryLower);
  }
  return null;
}

/**
 * Locate the boundary of each cited item's own text within a rendered
 * multi-item citation cluster (e.g. "(Garcia, 2019a; Garcia, 2019b)"),
 * anchored on each item's own primary-name occurrence, falling back to
 * its year (with disambiguation suffix) when the name has already been
 * claimed by an earlier item in the cluster.
 *
 * Returns `items.length` segments in cite order, or `null` if any item's
 * anchor can't be distinctly located, or if the located anchors don't
 * come out in cite order (a same-author collapse with unpredictable
 * suffix placement, for example). Callers must treat `null` as "not
 * segmentable" -- guessing at a boundary risks attributing one item's
 * rendered text to a different item's reference data, which would
 * silently corrupt the component comparison rather than just skip it.
 *
 * @param {string} citationText - one side's rendered citation cluster (oracle or Citum)
 * @param {Array<{id: string}>} items - cite.items, in cite order
 * @param {Object} testItems - id -> reference data lookup
 * @returns {Array<{start: number, end: number, refData: Object}>|null}
 */
function splitCitationIntoItemSegments(citationText, items, testItems) {
  const normalized = normalizeText(citationText);
  const anchors = [];
  const occupiedPositions = [];

  for (const item of items) {
    const refData = testItems[item.id];
    if (!refData) return null;

    const names = refData.author && refData.author.length > 0 ? refData.author : refData.editor;
    const nameAnchor = Array.isArray(names) && names.length > 0
      ? findFamilyNameAnchor(normalized, names, occupiedPositions)
      : null;

    // Always also try to claim this item's own year, even when the name is
    // what determines the segment boundary -- otherwise a later item that
    // must fall back to year-anchoring (the same-author-collapsed case,
    // e.g. "Garcia 2019a, 2019b") could mistake an earlier item's own
    // unclaimed year for its own, since only the name would be occupied.
    const yearAnchor = findYearComponent(normalized, refData, occupiedPositions);

    const anchor = nameAnchor || yearAnchor;
    if (!anchor) return null;

    anchors.push(anchor.pos);
    occupiedPositions.push(anchor.pos);
    if (yearAnchor && yearAnchor.pos !== anchor.pos) {
      occupiedPositions.push(yearAnchor.pos);
    }
  }

  // Cite order is assumed to match render order -- true for every style
  // this repo ships (items within a cluster aren't independently
  // reordered). Verify defensively rather than assume: if the anchors
  // didn't come out left-to-right in cite order, bail rather than guess.
  for (let i = 1; i < anchors.length; i++) {
    if (anchors[i].start < anchors[i - 1].start) return null;
  }

  return anchors.map((anchor, i) => {
    const end = i + 1 < anchors.length ? anchors[i + 1].start : normalized.length;
    return { start: anchor.start, end, refData: testItems[items[i].id] };
  });
}

/**
 * Compare a rendered citation cluster against the oracle, component by
 * component, per cite item. Single-item clusters compare directly (the
 * same 1:1 shape as a bibliography entry); multi-item clusters are
 * segmented first via `splitCitationIntoItemSegments`.
 *
 * Returns `{ segmented: false }` -- not zeroed-out matches/differences --
 * when either side can't be segmented, so callers can exclude these
 * citations from an aggregate rate instead of miscounting them as
 * complete mismatches.
 *
 * @param {string} oracleText
 * @param {string} citumText
 * @param {Array<{id: string}>} items - cite.items, in cite order
 * @param {Object} testItems - id -> reference data lookup
 */
function compareCitationComponents(oracleText, citumText, items, testItems) {
  if (!Array.isArray(items) || items.length === 0) return { segmented: false };

  if (items.length === 1) {
    const refData = testItems[items[0].id];
    if (!refData) return { segmented: false };
    const oracleComp = parseComponents(oracleText, refData);
    const citumComp = parseComponents(citumText, refData);
    const { matches, differences } = compareComponents(oracleComp, citumComp, refData);
    return { segmented: true, matches, differences };
  }

  const oracleSegments = splitCitationIntoItemSegments(oracleText, items, testItems);
  const citumSegments = splitCitationIntoItemSegments(citumText, items, testItems);
  if (!oracleSegments || !citumSegments) return { segmented: false };

  const normalizedOracle = normalizeText(oracleText);
  const normalizedCitum = normalizeText(citumText);
  const allMatches = [];
  const allDifferences = [];

  for (let i = 0; i < items.length; i++) {
    const refData = testItems[items[i].id];
    const oracleSlice = sliceMatchedValue(normalizedOracle, oracleSegments[i]) ?? '';
    const citumSlice = sliceMatchedValue(normalizedCitum, citumSegments[i]) ?? '';
    const oracleComp = parseComponents(oracleSlice, refData);
    const citumComp = parseComponents(citumSlice, refData);
    const { matches, differences } = compareComponents(oracleComp, citumComp, refData);
    allMatches.push(...matches);
    allDifferences.push(...differences);
  }

  return { segmented: true, matches: allMatches, differences: allDifferences };
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
  const normalized = normalizeText(entry);
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
    editors: { found: false, value: null, pos: null },
    translators: { found: false, value: null, pos: null },
    interviewers: { found: false, value: null, pos: null },
    recipients: { found: false, value: null, pos: null }
  };

  const entryLower = normalized.toLowerCase();

  const principalNames = (refData.author && refData.author.length > 0)
    ? refData.author
    : refData.editor;
  const principalComponent = findContributorComponent(normalized, principalNames);
  if (principalComponent) {
    result.contributors = principalComponent;
  }

  // Year
  if (refData.issued && refData.issued['date-parts'] && refData.issued['date-parts'][0]) {
    const year = String(refData.issued['date-parts'][0][0]);
    const yearPos = findNumericFieldPosition(entryLower, year);
    if (yearPos) {
      result.year = {
        found: true,
        value: sliceMatchedValue(normalized, yearPos),
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
        value: sliceMatchedValue(normalized, titlePos),
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
        value: sliceMatchedValue(normalized, containerPos),
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
        value: sliceMatchedValue(normalized, volPos),
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
        value: sliceMatchedValue(normalized, issuePos),
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
        value: sliceMatchedValue(normalized, pagePos),
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
        value: sliceMatchedValue(normalized, pubPos),
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
        value: sliceMatchedValue(normalized, doiPos),
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
        value: sliceMatchedValue(normalized, edPos),
        pos: edPos
      };
    }
  }

  const occupiedPositions = [];
  if (result.contributors.found && result.contributors.pos) {
    occupiedPositions.push(result.contributors.pos);
  }

  const secondaryRoleMappings = [
    ['editor', 'editors'],
    ['translator', 'translators'],
    ['interviewer', 'interviewers'],
    ['recipient', 'recipients'],
  ];

  for (const [refKey, componentKey] of secondaryRoleMappings) {
    const component = findContributorComponent(normalized, refData[refKey], occupiedPositions);
    if (component) {
      result[componentKey] = component;
      occupiedPositions.push(component.pos);
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
function analyzeOrdering(entryOrComponents, refData = null) {
  const components = typeof entryOrComponents === 'string'
    ? parseComponents(entryOrComponents, refData)
    : entryOrComponents;
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

/**
 * Compare two component sets and identify differences.
 *
 * Symmetric heuristic diff: matches indicate components present and equal in
 * both renderings; differences flag value mismatches, missing components, or
 * extra components.
 */
function compareComponents(oracleComp, citumComp, _refData) {
  const differences = [];
  const matches = [];

  const keys = ['contributors', 'year', 'title', 'containerTitle', 'volume',
    'issue', 'pages', 'publisher', 'doi', 'edition', 'editors',
    'translators', 'interviewers', 'recipients'];

  for (const key of keys) {
    const oracle = oracleComp[key];
    const citum = citumComp[key];

    if (!oracle.found && !citum.found) continue;

    if (oracle.found && citum.found) {
      if (oracle.value === citum.value ||
        (typeof oracle.value === 'boolean' && oracle.value === citum.value)) {
        matches.push({ component: key, status: 'match' });
      } else {
        differences.push({
          component: key,
          issue: 'value_mismatch',
          expected: oracle.value,
          found: citum.value,
          detail: 'Value differs between oracle and Citum',
        });
      }
    } else if (oracle.found && !citum.found) {
      differences.push({
        component: key,
        issue: 'missing',
        expected: oracle.value,
        detail: `Missing in Citum output`
      });
    } else if (!oracle.found && citum.found) {
      differences.push({
        component: key,
        issue: 'extra',
        found: citum.value,
        detail: `Extra in Citum output (not in oracle)`
      });
    }
  }

  return { differences, matches };
}

module.exports = {
  compareText,
  normalizeExactText,
  normalizeText,
  isCaseOnlyMismatch,
  parseComponents,
  compareComponents,
  findYearComponent,
  splitCitationIntoItemSegments,
  compareCitationComponents,
  analyzeOrdering,
  findRefMatchForEntry,
  hasPrimaryNames,
  findRefDataForEntry,
  loadLocale,
  findFieldPosition,
  findNumericFieldPosition,
  expandNamePosition,
  escapeRegex,
  textSimilarity,
  tokenizeForSimilarity,
};
