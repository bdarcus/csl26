'use strict';

const {
  compareText,
  findRefMatchForEntry,
  hasPrimaryNames,
  normalizeText,
} = require('../oracle-utils');
const {
  loadVerificationPolicy,
  resolveRegisteredDivergence,
} = require('./verification-policy');

const DIV_004_ID = 'div-004';
const DIV_005_ID = 'div-005';
const DIV_008_ID = 'div-008';
const DIV_009_ID = 'div-009';
const DIV_010_ID = 'div-010';
const DIV_011_ID = 'div-011';

// Recognized GB/T 7714—2025 date-annotation forms (§7.5.4.1 era-year
// parentheticals, §7.5.4.3 copyright/printing-year and approximate-year
// brackets) for div-011 masking. Stripping these from both citeproc's and
// Citum's text isolates whether an annotation citeproc-js drops (or, for
// open-ended ranges, spuriously adds — §8.4.2) is the sole delta.
const GBT_DATE_ANNOTATION_PATTERNS = [
  // Parenthetical annotation directly after a Gregorian year, e.g.
  // 1947（民国三十六年）, 1865 (清同治四年), 1949(中华民国三十八年八月).
  /(\d{4})\s*[（(][^）)]*[）)]/g,
  // Approximate-year brackets, e.g. [1936].
  /[[［](\d{4})[\]］]/g,
];

/**
 * Strip GB/T §7.5.4.1/§7.5.4.3 date-annotation forms for div-011 comparison.
 */
function stripGbtDateAnnotations(text) {
  let stripped = String(text || '');
  for (const pattern of GBT_DATE_ANNOTATION_PATTERNS) {
    stripped = stripped.replace(pattern, '$1');
  }
  // Bare "printed" suffix directly after a year, e.g. 1995印刷.
  stripped = stripped.replace(/(\d{4})印刷/g, '$1');
  return stripped;
}

// Models the GB/T punctuation-only citeproc divergence after semantic
// realization; see docs/specs/MULTILINGUAL.md §3.2a.
const FULL_WIDTH_TO_LATIN_PUNCTUATION = [
  [/：/g, ': '],
  [/，/g, ', '],
  [/（/g, '('],
  [/）/g, ')'],
];

// Mirrors `is_latin_script_language` in crates/citum-engine/src/values/mod.rs.
const NON_LATIN_SCRIPT_SUBTAGS = new Set([
  'hans', 'hant', 'hani', 'jpan', 'kore', 'hang', 'cyrl', 'arab', 'hebr', 'grek', 'deva',
]);
const NON_LATIN_PRIMARY_LANGUAGES = new Set([
  'zh', 'ja', 'ko', 'yue', 'wuu', 'nan', 'hak', 'cjy', 'cmn', 'hsn',
  'ru', 'be', 'bg', 'mk', 'sr', 'uk',
  'ar', 'fa', 'ur',
  'he', 'yi',
  'el',
  'hi', 'mr', 'ne',
]);

/**
 * Whether a BCP 47 language tag's script is Latin, for div-010 masking.
 * An absent or unrecognized tag is treated as not Latin — masking requires
 * positive evidence of a Latin-script item, mirroring the engine's gate.
 */
function isLatinScriptLanguage(lang) {
  if (!lang) return false;
  const subtags = String(lang).toLowerCase().split(/[-_]/);
  const primary = subtags.shift();
  if (!isMeaningfulLanguagePrimary(primary)) return false;

  for (const subtag of subtags) {
    if (subtag === 'latn') return true;
    if (NON_LATIN_SCRIPT_SUBTAGS.has(subtag)) return false;
  }

  return !NON_LATIN_PRIMARY_LANGUAGES.has(primary);
}

function isMeaningfulLanguagePrimary(primary) {
  return typeof primary === 'string'
    && !['und', 'mul', 'zxx'].includes(primary)
    && /^[a-z]{2,8}$/.test(primary);
}

/**
 * Map citeproc's hardcoded CJK delimiters to the Latin strings produced by
 * GB/T semantic realization, then collapse any resulting doubled space.
 */
function mapFullWidthToLatinPunctuation(text) {
  let mapped = String(text || '');
  for (const [pattern, replacement] of FULL_WIDTH_TO_LATIN_PUNCTUATION) {
    mapped = mapped.replace(pattern, replacement);
  }
  while (mapped.includes('  ')) {
    mapped = mapped.replace('  ', ' ');
  }
  return mapped;
}

function escapeRegex(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function arraysEqual(left, right) {
  if (left.length !== right.length) return false;
  return left.every((value, index) => value === right[index]);
}

function buildReferenceOrderIds(entries, testItems) {
  return entries
    .map((entry) => findRefMatchForEntry(entry, testItems)?.id || null)
    .filter(Boolean);
}

function buildNumericLabelMap(orderIds) {
  return new Map(orderIds.map((id, index) => [id, index + 1]));
}

function detectDiv004OrderDifference(oracleBibliography, citumOrderIds, testItems, divergenceRule) {
  if (!divergenceRule || !Array.isArray(oracleBibliography) || !Array.isArray(citumOrderIds)) {
    return null;
  }

  const oracleOrderIds = buildReferenceOrderIds(oracleBibliography, testItems);
  if (oracleOrderIds.length === 0 || oracleOrderIds.length !== citumOrderIds.length) {
    return null;
  }

  const oracleSet = new Set(oracleOrderIds);
  const citumSet = new Set(citumOrderIds);
  if (oracleSet.size !== citumSet.size || oracleSet.size !== oracleOrderIds.length) {
    return null;
  }
  for (const id of oracleSet) {
    if (!citumSet.has(id)) return null;
  }

  if (arraysEqual(oracleOrderIds, citumOrderIds)) {
    return null;
  }

  const anonymousIds = oracleOrderIds.filter((id) => !hasPrimaryNames(testItems[id]));
  if (anonymousIds.length === 0) {
    return null;
  }

  const anonymousSet = new Set(anonymousIds);

  // div-004 covers both the insertion-point of anonymous items relative to named
  // items AND their relative ordering within the anonymous group (since Citum
  // sorts anonymous entries by title while citeproc-js uses type-specific keys).
  // Named items may differ independently due to div-008; that check is orthogonal.
  // The compensation in explainCitationMismatchFromDiv004 uses per-item label maps
  // and masked comparison, so it handles any combination of ordering differences.

  return {
    divergenceId: DIV_004_ID,
    scopes: divergenceRule.scopes || [],
    tags: divergenceRule.tags || [],
    note: divergenceRule.note || null,
    oracleOrderIds,
    citumOrderIds,
    anonymousIds,
  };
}

function getFirstAuthorFamily(testItems, id) {
  const ref = testItems[id];
  if (!ref) return null;
  const primaryRoles = ['author', 'editor', 'translator', 'interviewer', 'recipient'];
  for (const role of primaryRoles) {
    const names = ref[role];
    if (Array.isArray(names) && names.length > 0 && names[0].family) {
      return names[0].family.toLowerCase().trim();
    }
  }
  return null;
}

function detectDiv008OrderDifference(oracleBibliography, citumOrderIds, testItems, divergenceRule) {
  if (!divergenceRule || !Array.isArray(oracleBibliography) || !Array.isArray(citumOrderIds)) {
    return null;
  }

  const oracleOrderIds = buildReferenceOrderIds(oracleBibliography, testItems);
  if (oracleOrderIds.length === 0 || oracleOrderIds.length !== citumOrderIds.length) {
    return null;
  }

  // Guard against duplicates/missing IDs from fuzzy matching — mirrors div-004.
  const oracleSet = new Set(oracleOrderIds);
  const citumSet = new Set(citumOrderIds);
  if (oracleSet.size !== citumSet.size || oracleSet.size !== oracleOrderIds.length) {
    return null;
  }
  for (const id of oracleSet) {
    if (!citumSet.has(id)) return null;
  }

  if (arraysEqual(oracleOrderIds, citumOrderIds)) {
    return null;
  }

  const citumPositionOf = new Map(citumOrderIds.map((id, i) => [id, i]));
  const swappedPairs = [];

  // Derive adjacency from the named-only subsequence so that anonymous items
  // interspersed between two same-family named items (a co-occurrence with
  // div-004) do not prevent detection. Two named items are "adjacent in
  // oracle" if no other named item lies between them.
  const oracleNamedIds = oracleOrderIds.filter((id) => hasPrimaryNames(testItems[id]));

  for (let i = 0; i < oracleNamedIds.length - 1; i++) {
    const idA = oracleNamedIds[i];
    const idB = oracleNamedIds[i + 1];

    const familyA = getFirstAuthorFamily(testItems, idA);
    const familyB = getFirstAuthorFamily(testItems, idB);
    if (!familyA || !familyB || familyA !== familyB) continue;

    const citumPosA = citumPositionOf.get(idA);
    const citumPosB = citumPositionOf.get(idB);
    if (citumPosA === undefined || citumPosB === undefined) continue;

    if (citumPosB < citumPosA) {
      swappedPairs.push([idA, idB]);
    }
  }

  if (swappedPairs.length === 0) return null;

  return {
    divergenceId: DIV_008_ID,
    scopes: divergenceRule.scopes || [],
    tags: divergenceRule.tags || [],
    note: divergenceRule.note || null,
    oracleOrderIds,
    citumOrderIds,
    swappedPairs,
    affectedIds: [...new Set(swappedPairs.flat())],
  };
}

function explainCitationMismatchFromDiv008(citationEntry, citationFixture, divergenceInfo) {
  if (!citationEntry || citationEntry.match || !citationFixture || !divergenceInfo) {
    return null;
  }

  const oracleLabelMap = buildNumericLabelMap(divergenceInfo.oracleOrderIds);
  const citumLabelMap = buildNumericLabelMap(divergenceInfo.citumOrderIds);
  const affectedSet = new Set(divergenceInfo.affectedIds || []);

  const itemIds = (citationFixture.items || [])
    .map((item) => item.id)
    .filter((id) => affectedSet.has(id) && oracleLabelMap.has(id) && citumLabelMap.has(id));

  if (itemIds.length === 0) {
    return null;
  }

  const oracleLabels = itemIds.map((id) => oracleLabelMap.get(id));
  const citumLabels = itemIds.map((id) => citumLabelMap.get(id));
  const maskedOracle = maskNumericCitationLabels(citationEntry.oracle, oracleLabels);
  const maskedCitum = maskNumericCitationLabels(citationEntry.citum, citumLabels);

  if (maskedOracle !== maskedCitum) {
    return null;
  }

  return {
    divergenceId: DIV_008_ID,
    tag: 'sort-derived-numeric-citation-label',
    itemIds,
    oracleLabels,
    citumLabels,
  };
}

function maskNumericCitationLabels(text, labels) {
  let masked = normalizeText(text || '');
  const sorted = [...new Set(labels.filter((label) => Number.isInteger(label) && label > 0))]
    .sort((left, right) => String(right).length - String(left).length);

  for (const label of sorted) {
    const pattern = new RegExp(`(^|[\\[(;,\\-]\\s*)${label}(?=$|[:\\])\\s;,\\-])`, 'g');
    masked = masked.replace(pattern, '$1#');
  }

  return normalizeText(masked);
}

function explainCitationMismatchFromDiv004(citationEntry, citationFixture, divergenceInfo) {
  if (!citationEntry || citationEntry.match || !citationFixture || !divergenceInfo) {
    return null;
  }

  const oracleLabelMap = buildNumericLabelMap(divergenceInfo.oracleOrderIds);
  const citumLabelMap = buildNumericLabelMap(divergenceInfo.citumOrderIds);
  const itemIds = (citationFixture.items || [])
    .map((item) => item.id)
    .filter((id) => oracleLabelMap.has(id) && citumLabelMap.has(id));

  if (itemIds.length === 0) {
    return null;
  }

  const oracleLabels = itemIds.map((id) => oracleLabelMap.get(id));
  const citumLabels = itemIds.map((id) => citumLabelMap.get(id));
  const maskedOracle = maskNumericCitationLabels(citationEntry.oracle, oracleLabels);
  const maskedCitum = maskNumericCitationLabels(citationEntry.citum, citumLabels);

  if (maskedOracle !== maskedCitum) {
    return null;
  }

  return {
    divergenceId: DIV_004_ID,
    tag: 'sort-derived-numeric-citation-label',
    itemIds,
    oracleLabels,
    citumLabels,
  };
}

function getStructuredArchiveInfo(ref) {
  return ref?.['archive-info'] || ref?.archive_info || null;
}

function getArchiveFragments(ref) {
  const archiveInfo = getStructuredArchiveInfo(ref);
  if (!archiveInfo || typeof archiveInfo !== 'object') {
    return [];
  }

  return [
    archiveInfo.collection,
    archiveInfo.location,
    archiveInfo.name,
    archiveInfo.place,
  ].filter((value) => typeof value === 'string' && value.trim().length > 0);
}

function stripTrailingArchiveFragments(text, fragments) {
  let stripped = text || '';
  // Set aside a trailing terminal mark so the archive-fragment suffix pattern
  // (anchored to end-of-string) still matches when the rendered text legitimately
  // ends in punctuation after the fragment (e.g. "..., Jerusalem.").
  const trailingPunctuationMatch = stripped.match(/[.,;:]\s*$/);
  const trailingPunctuation = trailingPunctuationMatch ? trailingPunctuationMatch[0] : '';
  if (trailingPunctuation) {
    stripped = stripped.slice(0, stripped.length - trailingPunctuation.length);
  }
  for (const fragment of [...fragments].reverse()) {
    const suffixPattern = new RegExp(`,\\s*${escapeRegex(fragment)}\\s*$`);
    stripped = stripped.replace(suffixPattern, '');
  }
  return stripped + trailingPunctuation;
}

function normalizeAncientYear(text, ref, oracleText) {
  const year = ref?.issued?.['date-parts']?.[0]?.[0];
  if (!Number.isInteger(year) || year >= 0) {
    return text;
  }

  const bcYear = `${Math.abs(year)} BC`;
  if (!normalizeText(oracleText || '').includes(normalizeText(bcYear))) {
    return text;
  }

  return String(text || '').replace(String(year), bcYear);
}

function explainCitationMismatchFromDiv005(citationEntry, citationFixture, testItems, divergenceRule) {
  if (!citationEntry || citationEntry.match || !citationFixture || !divergenceRule) {
    return null;
  }

  const itemIds = (citationFixture.items || []).map((item) => item.id).filter(Boolean);
  if (itemIds.length !== 1) {
    return null;
  }

  const ref = testItems[itemIds[0]];
  if (!ref || ref.type !== 'manuscript') {
    return null;
  }

  const archiveFragments = getArchiveFragments(ref);
  if (archiveFragments.length === 0) {
    return null;
  }

  const strippedCitum = stripTrailingArchiveFragments(citationEntry.citum, archiveFragments);
  const normalizedCitum = normalizeAncientYear(strippedCitum, ref, citationEntry.oracle);
  const comparison = compareText(citationEntry.oracle, normalizedCitum);
  if (!comparison.match || comparison.caseMismatch) {
    return null;
  }

  return {
    divergenceId: DIV_005_ID,
    tag: 'structured-archival-manuscript-detail',
    itemIds,
    archiveFragments,
    yearNormalized: strippedCitum !== normalizedCitum,
  };
}

/**
 * div-010: GB/T-style bilingual styles hardcode CJK full-width delimiters
 * (：，（）) for every item, including Latin-script references, where GB/T
 * practice is Latin half-width punctuation. citeproc-js reproduces the same
 * hardcoded full-width punctuation, so byte-parity does not catch this —
 * see docs/specs/MULTILINGUAL.md §3.2a and csl26-5y6k. Masks a mismatch only
 * when the item(s) are Latin-script and the delta is punctuation-only.
 */
function explainCitationMismatchFromDiv010(citationEntry, citationFixture, testItems, divergenceRule) {
  if (!citationEntry || citationEntry.match || !citationFixture || !divergenceRule) {
    return null;
  }

  const itemIds = (citationFixture.items || []).map((item) => item.id).filter(Boolean);
  if (itemIds.length === 0 || !itemIds.every((id) => isLatinScriptLanguage(testItems[id]?.language))) {
    return null;
  }

  const normalizedOracle = mapFullWidthToLatinPunctuation(citationEntry.oracle);
  const comparison = compareText(normalizedOracle, citationEntry.citum);
  if (!comparison.match || comparison.caseMismatch) {
    return null;
  }

  return { divergenceId: DIV_010_ID, tag: 'latin-script-punctuation-localization', itemIds };
}

function explainBibliographyMismatchFromDiv010(entry, testItems, divergenceRule) {
  if (!entry || entry.match || !divergenceRule) return null;
  const ref = testItems[entry.id];
  if (!ref || !isLatinScriptLanguage(ref.language)) return null;

  const normalizedOracle = mapFullWidthToLatinPunctuation(entry.oracle);
  const comparison = compareText(normalizedOracle, entry.citum);
  if (!comparison.match || comparison.caseMismatch) return null;

  return {
    divergenceId: DIV_010_ID,
    tag: 'latin-script-punctuation-localization',
    itemIds: [entry.id],
  };
}

/**
 * div-011: Citum echoes an author-supplied CSL cheater-syntax `issued:`
 * date-note override verbatim, including GB/T 7714—2025 date annotations
 * (§7.5.4.1 era-year parentheticals, §7.5.4.3 copyright/printing-year and
 * approximate-year brackets); citeproc-js either drops the annotation or, for
 * open-ended ranges, adds a spurious bracket. See verification-policy.yaml
 * div-011 for the standard-text citations that confirm Citum's rendering.
 * Gated on the item carrying a `note`-field `issued:` override, so this
 * cannot mask an unrelated date mismatch on an item without one.
 */
function explainBibliographyMismatchFromDiv011(entry, testItems, divergenceRule) {
  if (!entry || entry.match || !divergenceRule) return null;
  const ref = testItems[entry.id];
  if (!ref?.note || !/(?:^|\n)issued:/i.test(ref.note)) return null;

  const strippedOracle = stripGbtDateAnnotations(entry.oracle);
  const strippedCitum = stripGbtDateAnnotations(entry.citum);
  const comparison = compareText(strippedOracle, strippedCitum);
  if (!comparison.match || comparison.caseMismatch) return null;

  return { divergenceId: DIV_011_ID, tag: 'gbt-date-annotation-fidelity', itemIds: [entry.id] };
}

function explainBibliographyMismatchFromDiv009(entry, testItems, divergenceRule) {
  if (!entry || entry.match || !divergenceRule) return null;
  const ref = testItems[entry.id];
  const match = ref?.note?.match(/(?:^|\n)tex\.cstr:\s*([^\n\s]+)/i);
  if (!match || !ref.URL?.includes(match[1])) return null;
  const tail = `. CSTR:${match[1]}`;
  if (!entry.citum?.endsWith(tail)) return null;
  const comparison = compareText(entry.oracle, entry.citum.slice(0, -tail.length));
  if (!comparison.match || comparison.caseMismatch) return null;
  return { divergenceId: DIV_009_ID, tag: 'duplicate-url-identifier-tail', itemIds: [entry.id] };
}

function buildAdjustedOracleResult(rawResults, testCitations, testItems, divergenceInfo, div005Rule, div008Info, div009Rule, div010Rule, div011Rule) {
  const adjustedCitationEntries = (rawResults.citations?.entries || []).map((entry, index) => {
    const div004Adjustment = explainCitationMismatchFromDiv004(
      entry,
      testCitations[index],
      divergenceInfo
    );
    const div005Adjustment = explainCitationMismatchFromDiv005(
      entry,
      testCitations[index],
      testItems,
      div005Rule
    );
    const div008Adjustment = explainCitationMismatchFromDiv008(
      entry,
      testCitations[index],
      div008Info
    );
    const div010Adjustment = explainCitationMismatchFromDiv010(
      entry,
      testCitations[index],
      testItems,
      div010Rule
    );
    const appliedDivergence = div004Adjustment || div005Adjustment || div008Adjustment || div010Adjustment;
    return {
      ...entry,
      rawMatch: entry.match,
      match: entry.match || Boolean(appliedDivergence),
      appliedDivergence,
    };
  });

  const adjustedCitationPassed = adjustedCitationEntries.filter((entry) => entry.match).length;
  const adjustedCitationTotal = rawResults.citations?.total || adjustedCitationEntries.length;
  const adjustedBibliographyEntries = (rawResults.bibliography?.entries || []).map((entry) => {
    if (entry.match === null) {
      return {
        ...entry,
        rawMatch: null,
        match: null,
        appliedDivergence: null,
      };
    }
    const div009Adjustment = explainBibliographyMismatchFromDiv009(entry, testItems, div009Rule);
    const div010Adjustment = explainBibliographyMismatchFromDiv010(entry, testItems, div010Rule);
    const div011Adjustment = explainBibliographyMismatchFromDiv011(entry, testItems, div011Rule);
    const appliedDivergence = div009Adjustment || div010Adjustment || div011Adjustment;
    return { ...entry, rawMatch: entry.match, match: entry.match || Boolean(appliedDivergence), appliedDivergence };
  });
  const adjustedBibliographyPassed = adjustedBibliographyEntries.filter((entry) => entry.match).length;
  const adjustedBibliographyTotal = Number.isFinite(rawResults.bibliography?.total)
    ? rawResults.bibliography.total
    : adjustedBibliographyEntries.length;
  const divergenceSummary = {};

  if (divergenceInfo) {
    const adjustedCitationCount = adjustedCitationEntries
      .filter((entry) => entry.appliedDivergence?.divergenceId === DIV_004_ID)
      .length;
    divergenceSummary[DIV_004_ID] = {
      scopes: divergenceInfo.scopes,
      tags: divergenceInfo.tags,
      note: divergenceInfo.note,
      adjustedCitations: adjustedCitationCount,
      bibliographyOrderDifference: true,
      anonymousIds: divergenceInfo.anonymousIds,
    };
  }

  const div005Adjustments = adjustedCitationEntries
    .map((entry) => entry.appliedDivergence)
    .filter((entry) => entry?.divergenceId === DIV_005_ID);
  if (div005Rule && div005Adjustments.length > 0) {
    divergenceSummary[DIV_005_ID] = {
      scopes: div005Rule.scopes || [],
      tags: div005Rule.tags || [],
      note: div005Rule.note || null,
      adjustedCitations: div005Adjustments.length,
      itemIds: [...new Set(div005Adjustments.flatMap((entry) => entry.itemIds || []))],
    };
  }

  if (div008Info) {
    const div008AdjustedCount = adjustedCitationEntries
      .filter((entry) => entry.appliedDivergence?.divergenceId === DIV_008_ID)
      .length;
    divergenceSummary[DIV_008_ID] = {
      scopes: div008Info.scopes,
      tags: div008Info.tags,
      note: div008Info.note,
      adjustedCitations: div008AdjustedCount,
      bibliographyOrderDifference: true,
      swappedPairs: div008Info.swappedPairs,
      affectedIds: div008Info.affectedIds,
    };
  }
  const div009Adjustments = adjustedBibliographyEntries
    .map((entry) => entry.appliedDivergence)
    .filter((entry) => entry?.divergenceId === DIV_009_ID);
  if (div009Rule && div009Adjustments.length > 0) {
    divergenceSummary[DIV_009_ID] = { scopes: div009Rule.scopes || [], tags: div009Rule.tags || [], note: div009Rule.note || null, adjustedBibliography: div009Adjustments.length, itemIds: [...new Set(div009Adjustments.flatMap((entry) => entry.itemIds || []))] };
  }

  const div011Adjustments = adjustedBibliographyEntries
    .map((entry) => entry.appliedDivergence)
    .filter((entry) => entry?.divergenceId === DIV_011_ID);
  if (div011Rule && div011Adjustments.length > 0) {
    divergenceSummary[DIV_011_ID] = { scopes: div011Rule.scopes || [], tags: div011Rule.tags || [], note: div011Rule.note || null, adjustedBibliography: div011Adjustments.length, itemIds: [...new Set(div011Adjustments.flatMap((entry) => entry.itemIds || []))] };
  }

  const div010CitationAdjustments = adjustedCitationEntries
    .map((entry) => entry.appliedDivergence)
    .filter((entry) => entry?.divergenceId === DIV_010_ID);
  const div010BibliographyAdjustments = adjustedBibliographyEntries
    .map((entry) => entry.appliedDivergence)
    .filter((entry) => entry?.divergenceId === DIV_010_ID);
  if (div010Rule && (div010CitationAdjustments.length > 0 || div010BibliographyAdjustments.length > 0)) {
    divergenceSummary[DIV_010_ID] = {
      scopes: div010Rule.scopes || [],
      tags: div010Rule.tags || [],
      note: div010Rule.note || null,
      adjustedCitations: div010CitationAdjustments.length,
      adjustedBibliography: div010BibliographyAdjustments.length,
      itemIds: [
        ...new Set(
          [...div010CitationAdjustments, ...div010BibliographyAdjustments].flatMap(
            (entry) => entry.itemIds || []
          )
        ),
      ],
    };
  }

  return {
    citations: {
      ...(rawResults.citations || {}),
      passed: adjustedCitationPassed,
      failed: Math.max(0, adjustedCitationTotal - adjustedCitationPassed),
      entries: adjustedCitationEntries,
    },
    bibliography: {
      ...(rawResults.bibliography || {}),
      passed: adjustedBibliographyPassed,
      failed: Math.max(0, adjustedBibliographyTotal - adjustedBibliographyPassed),
      entries: adjustedBibliographyEntries,
    },
    divergenceSummary,
  };
}

function attachRegisteredDivergenceAdjustments(rawResults, oracleBibliography, citumOrderIds, testItems, testCitations) {
  const hasCitationFailures = (rawResults?.citations?.failed || 0) > 0;
  const hasBibliographyFailures = (rawResults?.bibliography?.failed || 0) > 0;
  const shouldInspectOrderDifference = (
    hasCitationFailures || hasBibliographyFailures
  ) && Array.isArray(citumOrderIds) && citumOrderIds.length > 0;

  const policy = loadVerificationPolicy();
  const div005Rule = resolveRegisteredDivergence(policy, DIV_005_ID);
  const div009Rule = resolveRegisteredDivergence(policy, DIV_009_ID);
  const div010Rule = resolveRegisteredDivergence(policy, DIV_010_ID);
  const div011Rule = resolveRegisteredDivergence(policy, DIV_011_ID);

  if (!shouldInspectOrderDifference) {
    return {
      ...rawResults,
      bibliographyOrder: null,
      adjusted: buildAdjustedOracleResult(
        rawResults, testCitations, testItems, null, div005Rule, null, div009Rule, div010Rule, div011Rule
      ),
    };
  }

  const div004Rule = resolveRegisteredDivergence(policy, DIV_004_ID);
  const div008Rule = resolveRegisteredDivergence(policy, DIV_008_ID);

  const divergenceInfo = detectDiv004OrderDifference(
    oracleBibliography,
    citumOrderIds,
    testItems,
    div004Rule
  );
  const div008Info = detectDiv008OrderDifference(
    oracleBibliography,
    citumOrderIds,
    testItems,
    div008Rule
  );

  const appliedDivergences = [
    divergenceInfo?.divergenceId,
    div008Info?.divergenceId,
  ].filter(Boolean);

  return {
    ...rawResults,
    bibliographyOrder: (divergenceInfo || div008Info)
      ? {
          oracleOrderIds: (divergenceInfo || div008Info).oracleOrderIds,
          citumOrderIds: (divergenceInfo || div008Info).citumOrderIds,
          appliedDivergence: appliedDivergences.length === 1
            ? appliedDivergences[0]
            : appliedDivergences,
        }
      : null,
    adjusted: buildAdjustedOracleResult(
      rawResults, testCitations, testItems, divergenceInfo, div005Rule, div008Info, div009Rule, div010Rule, div011Rule
    ),
  };
}

module.exports = {
  DIV_004_ID,
  DIV_005_ID,
  DIV_008_ID,
  DIV_009_ID,
  DIV_010_ID,
  DIV_011_ID,
  attachRegisteredDivergenceAdjustments,
  buildAdjustedOracleResult,
  buildNumericLabelMap,
  buildReferenceOrderIds,
  detectDiv004OrderDifference,
  detectDiv008OrderDifference,
  explainCitationMismatchFromDiv004,
  explainCitationMismatchFromDiv005,
  explainCitationMismatchFromDiv008,
  explainCitationMismatchFromDiv010,
  explainBibliographyMismatchFromDiv009,
  explainBibliographyMismatchFromDiv010,
  explainBibliographyMismatchFromDiv011,
  isLatinScriptLanguage,
  mapFullWidthToLatinPunctuation,
  stripGbtDateAnnotations,
};
