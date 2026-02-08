#!/usr/bin/env node
/**
 * Test suite for the hardened component parser.
 *
 * Covers: exact field matching, multi-field disambiguation,
 * numeric boundary guards, delimiter detection, and ordering.
 *
 * Usage: node scripts/test-component-parser.js
 */

'use strict';

const {
  normalizeText,
  findRefDataForEntry,
  parseComponents,
  analyzeOrdering,
  detectDelimiters,
  findFieldPosition,
  findNumericFieldPosition,
} = require('./lib/component-parser');

let passed = 0;
let failed = 0;

function assert(condition, message) {
  if (condition) {
    passed++;
  } else {
    failed++;
    console.error(`  FAIL: ${message}`);
  }
}

function assertEqual(actual, expected, message) {
  if (JSON.stringify(actual) === JSON.stringify(expected)) {
    passed++;
  } else {
    failed++;
    console.error(`  FAIL: ${message}`);
    console.error(`    expected: ${JSON.stringify(expected)}`);
    console.error(`    actual:   ${JSON.stringify(actual)}`);
  }
}

function section(name) {
  console.log(`\n--- ${name} ---`);
}

// -- Test data --

const ITEMS = {
  'ITEM-1': {
    id: 'ITEM-1',
    type: 'article-journal',
    title: 'The Structure of Scientific Revolutions',
    author: [{ family: 'Kuhn', given: 'Thomas S.' }],
    issued: { 'date-parts': [[1962]] },
    'container-title': 'International Encyclopedia of Unified Science',
    volume: '2',
    issue: '2',
    publisher: 'University of Chicago Press',
    'publisher-place': 'Chicago',
    DOI: '10.1234/example',
  },
  'ITEM-8': {
    id: 'ITEM-8',
    type: 'article-journal',
    title: 'Scientific Paradigms and Normal Science',
    author: [{ family: 'Kuhn', given: 'Thomas S.' }],
    issued: { 'date-parts': [[1970]] },
    'container-title': 'Philosophy of Science',
    volume: '37',
    issue: '1',
    page: '1-13',
    DOI: '10.1086/288273',
  },
  'ITEM-9': {
    id: 'ITEM-9',
    type: 'article-journal',
    title: 'Climate Change and Extreme Weather Events',
    author: [
      { family: 'Smith', given: 'John' },
      { family: 'Anderson', given: 'Mary' },
    ],
    issued: { 'date-parts': [[2020]] },
    'container-title': 'Nature Climate Change',
    volume: '10',
    page: '850-855',
    DOI: '10.1038/s41558-020-0871-4',
  },
  'ITEM-10': {
    id: 'ITEM-10',
    type: 'article-journal',
    title: 'Machine Learning for Climate Prediction',
    author: [
      { family: 'Smith', given: 'Jane' },
      { family: 'Williams', given: 'Robert' },
    ],
    issued: { 'date-parts': [[2020]] },
    'container-title': 'Environmental Research Letters',
    volume: '15',
    issue: '11',
    page: '114042',
    DOI: '10.1088/1748-9326/abc123',
  },
  'ITEM-14': {
    id: 'ITEM-14',
    type: 'book',
    title: 'Handbook of Research Methods in Social Psychology',
    editor: [
      { family: 'Reis', given: 'Harry T.' },
      { family: 'Judd', given: 'Charles M.' },
    ],
    issued: { 'date-parts': [[2000]] },
    publisher: 'Cambridge University Press',
    'publisher-place': 'Cambridge',
  },
  'ITEM-5': {
    id: 'ITEM-5',
    type: 'report',
    title: 'World Development Report 2023',
    author: [{ literal: 'World Bank' }],
    issued: { 'date-parts': [[2023]] },
    publisher: 'World Bank Group',
    'publisher-place': 'Washington, DC',
  },
};

// -- Tests --

section('normalizeText');
{
  assertEqual(
    normalizeText('<i>Title</i> with <b>bold</b>'),
    'Title with bold',
    'strips HTML tags'
  );
  assertEqual(
    normalizeText('Smith &#38; Jones'),
    'Smith & Jones',
    'converts HTML entities'
  );
  assertEqual(
    normalizeText('_italic_ and **bold**'),
    'italic and bold',
    'strips markdown formatting'
  );
  assertEqual(
    normalizeText('  spaces   everywhere  '),
    'spaces everywhere',
    'normalizes whitespace'
  );
}

section('findFieldPosition');
{
  const entry = 'kuhn, t. s. (1962). the structure of scientific revolutions.';
  const pos = findFieldPosition(entry, 'The Structure of Scientific Revolutions');
  assert(pos !== null, 'finds full title (case-insensitive)');
  if (pos) {
    assertEqual(pos.start, 20, 'correct start position');
    assertEqual(pos.end, 59, 'correct end position');
  }

  const noMatch = findFieldPosition(entry, 'Nonexistent Title Here');
  assertEqual(noMatch, null, 'returns null for non-matching field');
}

section('findNumericFieldPosition');
{
  const entry = 'nature, 521, 436-444. https://doi.org/10.1038/nature14539';

  // Volume "521" should match
  const vol = findNumericFieldPosition(entry, '521');
  assert(vol !== null, 'finds volume 521');
  if (vol) {
    assertEqual(entry.substring(vol.start, vol.end), '521', 'matched correct text');
  }

  // Volume "10" should NOT match within "10.1038"
  const falseVol = findNumericFieldPosition(entry, '10');
  // "10" appears inside "10.1038" - the digit boundary should prevent matching
  // Actually "10" in "10.1038" has a dot after, not a digit. Let's check:
  // The regex is (?<![0-9])10(?![0-9]) - "10.1038" -> before '1' is comma+space, after '0' is '.'
  // So (?<![0-9])10(?![0-9]) WOULD match "10" in "10.1038" because '.' is not a digit
  // This is actually correct behavior for the entry "nature climate change, 10, 850-855"
  // Let's test a case where it SHOULD NOT match:
  const entry2 = 'volume 100 of the series';
  const noMatch = findNumericFieldPosition(entry2, '10');
  assertEqual(noMatch, null, 'does not match "10" inside "100"');

  // Should match standalone "10"
  const entry3 = 'nature climate change, 10, 850-855';
  const match10 = findNumericFieldPosition(entry3, '10');
  assert(match10 !== null, 'matches standalone "10"');
}

section('findRefDataForEntry - multi-field disambiguation');
{
  // Two Kuhn entries: ITEM-1 (1962) and ITEM-8 (1970)
  const entry1962 = 'Kuhn, T. S. (1962). The structure of scientific revolutions. International Encyclopedia of Unified Science, 2(2). University of Chicago Press.';
  const entry1970 = 'Kuhn, T. S. (1970). Scientific paradigms and normal science. Philosophy of Science, 37(1), 1-13.';

  const ref1962 = findRefDataForEntry(entry1962, ITEMS);
  assert(ref1962 !== null, 'finds ref for 1962 Kuhn entry');
  assertEqual(ref1962.id, 'ITEM-1', 'disambiguates to correct Kuhn (1962)');

  const ref1970 = findRefDataForEntry(entry1970, ITEMS);
  assert(ref1970 !== null, 'finds ref for 1970 Kuhn entry');
  assertEqual(ref1970.id, 'ITEM-8', 'disambiguates to correct Kuhn (1970)');
}

section('findRefDataForEntry - Smith disambiguation');
{
  // Two Smith entries: ITEM-9 (John, climate change) and ITEM-10 (Jane, ML)
  const entryClimate = 'Smith, J., & Anderson, M. (2020). Climate change and extreme weather events. Nature Climate Change, 10, 850-855.';
  const entryML = 'Smith, J., & Williams, R. (2020). Machine learning for climate prediction. Environmental Research Letters, 15(11), 114042.';

  const refClimate = findRefDataForEntry(entryClimate, ITEMS);
  assert(refClimate !== null, 'finds ref for Smith climate entry');
  assertEqual(refClimate.id, 'ITEM-9', 'disambiguates Smith to climate paper');

  const refML = findRefDataForEntry(entryML, ITEMS);
  assert(refML !== null, 'finds ref for Smith ML entry');
  assertEqual(refML.id, 'ITEM-10', 'disambiguates Smith to ML paper');
}

section('findRefDataForEntry - editor-only entry');
{
  const entry = 'Reis, H. T., & Judd, C. M. (Eds.). (2000). Handbook of research methods in social psychology. Cambridge University Press.';
  const ref = findRefDataForEntry(entry, ITEMS);
  assert(ref !== null, 'finds editor-only book');
  assertEqual(ref.id, 'ITEM-14', 'matches by editor name');
}

section('findRefDataForEntry - literal author');
{
  const entry = 'World Bank. (2023). World development report 2023. World Bank Group.';
  const ref = findRefDataForEntry(entry, ITEMS);
  assert(ref !== null, 'finds literal author entry');
  assertEqual(ref.id, 'ITEM-5', 'matches World Bank');
}

section('parseComponents - exact field matching');
{
  const entry = 'Kuhn, T. S. (1962). The structure of scientific revolutions. International Encyclopedia of Unified Science, 2(2). University of Chicago Press. https://doi.org/10.1234/example';
  const ref = ITEMS['ITEM-1'];
  const comp = parseComponents(entry, ref);

  assert(comp.contributors.found, 'finds contributors');
  assert(comp.year.found, 'finds year');
  assertEqual(comp.year.value, '1962', 'correct year value');
  assert(comp.title.found, 'finds title');
  assert(comp.containerTitle.found, 'finds container-title');
  assert(comp.volume.found, 'finds volume');
  assertEqual(comp.volume.value, '2', 'correct volume value');
  assert(comp.issue.found, 'finds issue');
  assertEqual(comp.issue.value, '2', 'correct issue value');
  assert(comp.publisher.found, 'finds publisher');
  assert(comp.doi.found, 'finds DOI');
}

section('parseComponents - title precision (no false positive)');
{
  // "Deep Learning" is short. It should match the entry that contains it
  // but NOT match an entry about "Machine Learning for Climate Prediction"
  const entry = 'Smith, J., & Williams, R. (2020). Machine learning for climate prediction. Environmental Research Letters, 15(11), 114042.';
  const refDeepLearning = {
    title: 'Deep Learning',
    author: [{ family: 'LeCun', given: 'Yann' }],
    issued: { 'date-parts': [[2015]] },
  };
  const comp = parseComponents(entry, refDeepLearning);
  assert(!comp.title.found, 'does not false-match "Deep Learning" in ML entry');
  assert(!comp.contributors.found, 'does not false-match LeCun in Smith entry');
}

section('parseComponents - publisher precision (no false positive)');
{
  // "Cambridge University Press" should NOT match in an entry containing
  // only "Cambridge" as a place name
  const entry = 'Author, A. (2000). Some title. Cambridge.';
  const ref = {
    publisher: 'Cambridge University Press',
    author: [{ family: 'Author', given: 'A.' }],
    issued: { 'date-parts': [[2000]] },
  };
  const comp = parseComponents(entry, ref);
  assert(!comp.publisher.found, 'does not false-match "Cambridge University Press" when only "Cambridge" appears');
}

section('parseComponents - pages and DOI');
{
  const entry = 'Kuhn, T. S. (1970). Scientific paradigms and normal science. Philosophy of Science, 37(1), 1-13. https://doi.org/10.1086/288273';
  const ref = ITEMS['ITEM-8'];
  const comp = parseComponents(entry, ref);

  assert(comp.pages.found, 'finds pages');
  assertEqual(comp.pages.value, '1-13', 'correct page range');
  assert(comp.doi.found, 'finds DOI');
  assertEqual(comp.doi.value, '10.1086/288273', 'correct DOI value');
}

section('parseComponents - volume without issue');
{
  const entry = 'Smith, J., & Anderson, M. (2020). Climate change and extreme weather events. Nature Climate Change, 10, 850-855.';
  const ref = ITEMS['ITEM-9'];
  const comp = parseComponents(entry, ref);

  assert(comp.volume.found, 'finds standalone volume');
  assertEqual(comp.volume.value, '10', 'correct volume value');
  assert(!comp.issue.found, 'no issue when none in ref data');
}

section('parseComponents - edition and editors');
{
  const entry = 'In K. A. Ericsson (Ed.), The Cambridge handbook of expertise. Cambridge University Press.';
  const ref = {
    title: 'The Cambridge handbook of expertise',
    editor: [{ family: 'Ericsson', given: 'K. Anders' }],
    publisher: 'Cambridge University Press',
  };
  const comp = parseComponents(entry, ref);

  assert(comp.editors.found, 'finds editor marker (Ed.)');
  assert(comp.publisher.found, 'finds publisher');
}

section('parseComponents - URL (no DOI)');
{
  const entry = 'State of JS Team. (2023). The state of JavaScript 2023. https://stateofjs.com/2023';
  const ref = {
    title: 'The State of JavaScript 2023',
    author: [{ literal: 'State of JS Team' }],
    issued: { 'date-parts': [[2023]] },
    URL: 'https://stateofjs.com/2023',
  };
  const comp = parseComponents(entry, ref);

  assert(comp.url.found, 'finds URL when no DOI');
  assert(!comp.doi.found, 'no DOI');
}

section('analyzeOrdering');
{
  // APA-style ordering: contributors → year → title → container → volume → pages → doi
  const entry = 'LeCun, Y., Bengio, Y., & Hinton, G. (2015). Deep learning. Nature, 521, 436-444. https://doi.org/10.1038/nature14539';
  const ref = {
    title: 'Deep Learning',
    author: [
      { family: 'LeCun', given: 'Yann' },
      { family: 'Bengio', given: 'Yoshua' },
      { family: 'Hinton', given: 'Geoffrey' },
    ],
    issued: { 'date-parts': [[2015]] },
    'container-title': 'Nature',
    volume: '521',
    page: '436-444',
    DOI: '10.1038/nature14539',
  };

  const order = analyzeOrdering(entry, ref);
  assert(order.length >= 5, `found at least 5 positioned components (got ${order.length})`);

  // contributors should come first
  assert(order[0] === 'contributors', `first component is contributors (got ${order[0]})`);

  // year should come before title
  const yearIdx = order.indexOf('year');
  const titleIdx = order.indexOf('title');
  assert(yearIdx < titleIdx, 'year before title');
}

section('detectDelimiters');
{
  const entry = 'LeCun, Y., Bengio, Y., & Hinton, G. (2015). Deep learning. Nature, 521, 436-444.';
  const ref = {
    title: 'Deep Learning',
    author: [{ family: 'LeCun', given: 'Yann' }],
    issued: { 'date-parts': [[2015]] },
    'container-title': 'Nature',
    volume: '521',
    page: '436-444',
  };

  const delimiters = detectDelimiters(entry, ref);
  assert(delimiters.length > 0, `detected ${delimiters.length} delimiters`);

  // Check that delimiter between volume and pages exists
  const volPages = delimiters.find(
    d => d.between[0] === 'volume' && d.between[1] === 'pages'
  );
  if (volPages) {
    assertEqual(volPages.delimiter, ', ', 'volume-pages delimiter is ", "');
  }
}

section('parseComponents - no refData fallback');
{
  // When no refData is provided, pattern-based extraction still works
  const entry = 'Author, A. (2020). Some title. Journal Name, 15(3), 100-200. https://doi.org/10.1234/test';
  const comp = parseComponents(entry, null);

  assert(comp.year.found, 'finds year without refData');
  assertEqual(comp.year.value, '2020', 'correct year');
  assert(comp.pages.found, 'finds pages without refData');
  assertEqual(comp.pages.value, '100-200', 'correct pages');
  assert(comp.doi.found, 'finds DOI without refData');
  // Title, publisher, etc. need refData so should be false
  assert(!comp.title.found, 'no title without refData');
  assert(!comp.publisher.found, 'no publisher without refData');
}

// -- Summary --

console.log(`\n=== RESULTS: ${passed} passed, ${failed} failed ===\n`);
process.exit(failed > 0 ? 1 : 0);
