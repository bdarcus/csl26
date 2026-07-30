const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const {
  bibliographyComparisonMatches,
  compareComponents,
  cleanupOracleTempWorkspace,
  createOracleTempWorkspace,
  loadFixtures,
  matchBibliographyEntries,
  normalizeFixtureItems,
  parseCitumRenderOutput,
  refsDataForProcessor,
  expandCitationsForCitum,
  collapseClusteredCitumCitations,
  resolveAuthoredStylePath,
} = require('./oracle');
const { compareText, parseComponents } = require('./oracle-utils');
const {
  attachRegisteredDivergenceAdjustments,
  detectDiv004OrderDifference,
  detectDiv008OrderDifference,
  explainCitationMismatchFromDiv005,
  explainCitationMismatchFromDiv008,
  explainCitationMismatchFromDiv010,
  explainBibliographyMismatchFromDiv010,
  explainBibliographyMismatchFromDiv011,
  isLatinScriptLanguage,
  stripGbtDateAnnotations,
} = require('./lib/oracle-divergences');
const { loadVerificationPolicy, resolveRegisteredDivergence } = require('./lib/verification-policy');

const projectRoot = path.resolve(__dirname, '..');
const oracleScript = path.join(__dirname, 'oracle.js');
const hasLegacyStyles = fs.existsSync(path.join(projectRoot, 'styles-legacy', 'apa.csl'));

function runOracle(stylePath) {
  return new Promise((resolve, reject) => {
    const proc = spawn('node', [oracleScript, stylePath, '--json'], {
      cwd: projectRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', (chunk) => {
      stdout += chunk;
    });

    proc.stderr.on('data', (chunk) => {
      stderr += chunk;
    });

    proc.on('error', reject);
    proc.on('close', (code) => {
      if (code === 2) {
        reject(new Error(`oracle fatal for ${stylePath}: ${stderr || stdout}`));
        return;
      }

      try {
        resolve({
          code,
          json: JSON.parse(stdout),
          stderr,
        });
      } catch (error) {
        reject(new Error(`failed to parse oracle output for ${stylePath}: ${error.message}\n${stdout}\n${stderr}`));
      }
    });
  });
}

test('oracle temp workspaces are unique and removable', () => {
  const first = createOracleTempWorkspace();
  const second = createOracleTempWorkspace();

  assert.notEqual(first.dir, second.dir);
  assert.equal(fs.existsSync(first.dir), true);
  assert.equal(fs.existsSync(second.dir), true);

  cleanupOracleTempWorkspace(first);
  cleanupOracleTempWorkspace(second);

  assert.equal(fs.existsSync(first.dir), false);
  assert.equal(fs.existsSync(second.dir), false);
});

test('parallel oracle invocations do not collide on temp files', {
  timeout: 240000,
  skip: !hasLegacyStyles,
}, async () => {
  const styles = [
    path.join(projectRoot, 'styles-legacy', 'american-association-for-cancer-research.csl'),
    path.join(projectRoot, 'styles-legacy', 'american-institute-of-physics.csl'),
  ];

  const results = await Promise.all(styles.map((stylePath) => runOracle(stylePath)));

  for (let i = 0; i < results.length; i += 1) {
    const stylePath = styles[i];
    const expectedStyle = path.basename(stylePath, '.csl');
    const result = results[i];

    assert.ok([0, 1].includes(result.code), `unexpected oracle exit code for ${expectedStyle}: ${result.code}`);
    assert.equal(result.json.style, expectedStyle);
    assert.equal(result.json.error, undefined, `oracle reported fatal error for ${expectedStyle}`);
  }

  for (const tempFile of ['.migrated-refs.json', '.migrated-citations.json', '.migrated-temp.yaml']) {
    assert.equal(
      fs.existsSync(path.join(projectRoot, tempFile)),
      false,
      `legacy shared temp file should not exist: ${tempFile}`
    );
  }
});

test('oracle supports citation-only scope and skips bibliography', {
  skip: !hasLegacyStyles,
}, async () => {
  const stylePath = path.join(projectRoot, 'styles-legacy', 'apa.csl');

  const { code, stdout } = await new Promise((resolve, reject) => {
    const proc = spawn('node', [oracleScript, stylePath, '--json', '--scope', 'citation'], {
      cwd: projectRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    proc.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    proc.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    proc.on('error', reject);
    proc.on('close', (closeCode) => resolve({ code: closeCode, stdout, stderr }));
  });

  // Exit code is 0 (all match) or 1 (some citation diffs), never the old 2/throw.
  assert.ok(code === 0 || code === 1, `unexpected exit code ${code}`);
  const result = JSON.parse(stdout);
  assert.equal(result.summary.scope, 'citation');
  assert.ok(result.citations.total > 0, 'citation-only run should grade citations');
  assert.equal(result.bibliography.total, 0, 'citation-only run should skip bibliography');
});

test('normalizeFixtureItems handles wrapped and array fixtures by item id', () => {
  const wrapped = JSON.parse(
    fs.readFileSync(path.join(projectRoot, 'tests', 'fixtures', 'compound-numeric-refs.json'), 'utf8')
  );
  const zoteroWrapped = JSON.parse(
    fs.readFileSync(path.join(projectRoot, 'tests', 'fixtures', 'test-items-library', 'chicago-18th.json'), 'utf8')
  );
  const arrayFixture = JSON.parse(
    fs.readFileSync(path.join(projectRoot, 'tests', 'fixtures', 'references-humanities-note.json'), 'utf8')
  );

  const wrappedItems = normalizeFixtureItems(wrapped);
  const zoteroItems = normalizeFixtureItems(zoteroWrapped);
  const arrayItems = normalizeFixtureItems(arrayFixture);

  assert.ok(wrappedItems['zwart1983']);
  assert.ok(wrappedItems['astm-e2881']);
  assert.ok(zoteroItems['6188419/UDAG5V6W']);
  assert.ok(zoteroItems['6188419/SIQ8AUN7']);
  assert.ok(arrayItems['ginzburg1976']);
  assert.ok(arrayItems['foucault-interview']);
});

test('loadFixtures preserves raw wrapped fixtures for processor rendering', () => {
  const { refsData, testItems } = loadFixtures(
    path.join(projectRoot, 'tests', 'fixtures', 'compound-numeric-refs.json'),
    path.join(projectRoot, 'tests', 'fixtures', 'citations-compound-numeric.json')
  );

  assert.ok(Array.isArray(refsData.references));
  assert.ok(refsData.sets);
  assert.ok(testItems['zwart1983']);
  assert.ok(testItems['johnson2021-patent']);
});

test('clustered citation fixtures expand for Citum and collapse back to oracle id', () => {
  const { citations, clustered } = expandCitationsForCitum([
    {
      id: 'apa-four-author-first-subsequent',
      clusters: [
        { id: 'first', items: [{ id: 'ITEM-29' }] },
        { id: 'subsequent', items: [{ id: 'ITEM-29' }] },
      ],
    },
  ]);

  assert.deepEqual(citations.map((citation) => citation.id), [
    'apa-four-author-first-subsequent__first',
    'apa-four-author-first-subsequent__subsequent',
  ]);
  const collapsed = collapseClusteredCitumCitations(
    {
      citations: {
        'apa-four-author-first-subsequent__first': '(Smith, Lee, Kumar, & Zhou, 2021)',
        'apa-four-author-first-subsequent__subsequent': '(Smith et al, 2021)',
      },
      bibliography: [],
    },
    clustered
  );

  assert.equal(
    collapsed.citations['apa-four-author-first-subsequent'],
    '(Smith, Lee, Kumar, & Zhou, 2021) | (Smith et al, 2021)'
  );
  assert.equal(collapsed.citations['apa-four-author-first-subsequent__first'], undefined);
});

test('refsDataForProcessor preserves wrapped fixture sets', () => {
  const wrapped = JSON.parse(
    fs.readFileSync(path.join(projectRoot, 'tests', 'fixtures', 'compound-numeric-refs.json'), 'utf8')
  );

  const refsData = refsDataForProcessor(wrapped);

  assert.ok(Array.isArray(refsData.references));
  assert.ok(refsData.sets);
  assert.deepEqual(Object.keys(refsData.sets).sort(), [
    'catalysis-studies',
    'peroxisome-biogenesis',
  ]);
});

test('refsDataForProcessor converts Zotero item wrappers into a processor-ready array', () => {
  const zoteroWrapped = JSON.parse(
    fs.readFileSync(path.join(projectRoot, 'tests', 'fixtures', 'test-items-library', 'chicago-18th.json'), 'utf8')
  );

  const refsData = refsDataForProcessor(zoteroWrapped);

  assert.ok(Array.isArray(refsData));
  assert.equal(refsData.length, 403);
  assert.equal(refsData[0].id, '6188419/UDAG5V6W');
  const normalizedDate = refsData.find((item) => item.id === '6188419/TZUZU9ZP')?.issued?.['date-parts']?.[0]?.[0];
  assert.equal(normalizedDate, 2017);
  const normalizedSeason = refsData.find((item) => item.id === '6188419/PTREB4H3')?.issued?.season;
  assert.equal(normalizedSeason, 21);
});

test('div-004 detection recognizes missing-name order drift without treating named order as changed', () => {
  const refsData = JSON.parse(
    fs.readFileSync(path.join(projectRoot, 'tests', 'fixtures', 'references-expanded.json'), 'utf8')
  );
  const testItems = Object.fromEntries(
    Object.entries(refsData).filter(([key]) => key !== 'comment')
  );
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-004');

  const divergence = detectDiv004OrderDifference(
    [
      'Kuhn, Thomas S. The Structure of Scientific Revolutions. 1962.',
      'Brown v. Board of Education.',
    ],
    ['ITEM-20', 'ITEM-1'],
    {
      'ITEM-1': testItems['ITEM-1'],
      'ITEM-20': testItems['ITEM-20'],
    },
    divergenceRule
  );

  assert.equal(divergence?.divergenceId, 'div-004');
  assert.deepEqual(divergence?.anonymousIds, ['ITEM-20']);
});

test('registered divergence adjustments convert sort-derived numeric label drift into adjusted passes', () => {
  const rawResults = {
    style: 'association-for-computing-machinery',
    citations: {
      total: 1,
      passed: 0,
      failed: 1,
      entries: [{ id: 'cite-1', oracle: '[1]', citum: '[2]', match: false }],
    },
    bibliography: {
      total: 2,
      passed: 2,
      failed: 0,
      entries: [],
    },
    citationsByType: {},
    componentSummary: {},
    orderingIssues: 0,
  };

  const adjusted = attachRegisteredDivergenceAdjustments(
    rawResults,
    [
      'Kuhn, Thomas S. The Structure of Scientific Revolutions. 1962.',
      'Brown v. Board of Education.',
    ],
    ['ITEM-20', 'ITEM-1'],
    {
      'ITEM-1': {
        id: 'ITEM-1',
        type: 'book',
        title: 'The Structure of Scientific Revolutions',
        author: [{ family: 'Kuhn', given: 'Thomas S.' }],
      },
      'ITEM-20': {
        id: 'ITEM-20',
        type: 'legal_case',
        title: 'Brown v. Board of Education',
      },
    },
    [
      {
        id: 'cite-1',
        items: [{ id: 'ITEM-1' }],
      },
    ]
  );

  assert.equal(adjusted.adjusted.citations.passed, 1);
  assert.equal(adjusted.adjusted.citations.failed, 0);
  assert.equal(
    adjusted.adjusted.citations.entries[0].appliedDivergence?.divergenceId,
    'div-004'
  );
});

test('registered divergence adjustments recompute the bibliography passed/failed aggregate, not just entries', () => {
  // Regression guard: buildAdjustedOracleResult previously recomputed
  // citations.passed/failed from adjusted entries but only spread the raw
  // bibliography.passed/failed through unchanged, silently neutering every
  // bibliography-scope divergence (div-009, div-010) for any consumer that
  // reads the aggregate instead of walking entries by hand.
  const rawResults = {
    citations: { total: 0, passed: 0, failed: 0, entries: [] },
    bibliography: {
      total: 2,
      passed: 1,
      failed: 1,
      entries: [
        {
          id: 'gbt-latin-book',
          oracle: 'New York：Bantam Dell Publishing Group，1988.',
          citum: 'New York: Bantam Dell Publishing Group, 1988.',
          match: false,
        },
        {
          id: 'gbt-other-book',
          oracle: 'Alpha.',
          citum: 'Alpha.',
          match: true,
        },
      ],
    },
  };
  const testItems = {
    'gbt-latin-book': { id: 'gbt-latin-book', type: 'book', language: 'en-US' },
    'gbt-other-book': { id: 'gbt-other-book', type: 'book', language: 'en-US' },
  };

  const adjusted = attachRegisteredDivergenceAdjustments(rawResults, [], [], testItems, []);

  assert.equal(adjusted.adjusted.bibliography.passed, 2);
  assert.equal(adjusted.adjusted.bibliography.failed, 0);
});

test('registered divergence adjustments keep heuristic-unpaired rows outside compatibility totals', () => {
  const rawResults = {
    citations: { total: 0, passed: 0, failed: 0, entries: [] },
    bibliography: {
      total: 0,
      passed: 0,
      failed: 0,
      entries: [{
        index: 1,
        oracle: 'Oracle-only similarity candidate',
        citum: null,
        match: null,
        exactMatch: null,
        comparisonState: 'unresolved-unpaired',
        pairingMethod: 'similarity',
        compatibilityEligible: false,
        exactParityEligible: false,
      }],
    },
  };

  const adjusted = attachRegisteredDivergenceAdjustments(rawResults, [], [], {}, []);

  assert.equal(adjusted.adjusted.bibliography.total, 0);
  assert.equal(adjusted.adjusted.bibliography.passed, 0);
  assert.equal(adjusted.adjusted.bibliography.failed, 0);
  assert.equal(adjusted.adjusted.bibliography.entries[0].match, null);
});

test('div-005 recognizes structured archival manuscript detail as an intentional citation divergence', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-005');
  const entry = {
    id: 'hn-dead-sea-scrolls',
    oracle: '“The Community Rule (1QS),” Manuscript scroll, 100 BC',
    citum: '“The Community Rule (1QS)”, Manuscript scroll, -100, Shrine of the Book, Israel Antiquities Authority, Jerusalem',
    match: false,
  };
  const citationFixture = {
    id: 'hn-dead-sea-scrolls',
    items: [{ id: 'dead-sea-scrolls' }],
  };
  const testItems = {
    'dead-sea-scrolls': {
      id: 'dead-sea-scrolls',
      type: 'manuscript',
      issued: { 'date-parts': [[-100]] },
      'archive-info': {
        name: 'Israel Antiquities Authority',
        location: 'Shrine of the Book',
        place: 'Jerusalem',
      },
    },
  };

  const adjustment = explainCitationMismatchFromDiv005(
    entry,
    citationFixture,
    testItems,
    divergenceRule
  );

  assert.equal(adjustment?.divergenceId, 'div-005');
  assert.deepEqual(adjustment?.itemIds, ['dead-sea-scrolls']);
});

test('div-005 still recognizes the archive-fragment divergence when both sides end in a terminal period', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-005');
  const entry = {
    id: 'hn-dead-sea-scrolls',
    oracle: '“The Community Rule (1QS),” Manuscript scroll, 100 BC.',
    citum: '“The Community Rule (1QS)”, Manuscript scroll, -100, Shrine of the Book, Israel Antiquities Authority, Jerusalem.',
    match: false,
  };
  const citationFixture = {
    id: 'hn-dead-sea-scrolls',
    items: [{ id: 'dead-sea-scrolls' }],
  };
  const testItems = {
    'dead-sea-scrolls': {
      id: 'dead-sea-scrolls',
      type: 'manuscript',
      issued: { 'date-parts': [[-100]] },
      'archive-info': {
        name: 'Israel Antiquities Authority',
        location: 'Shrine of the Book',
        place: 'Jerusalem',
      },
    },
  };

  const adjustment = explainCitationMismatchFromDiv005(
    entry,
    citationFixture,
    testItems,
    divergenceRule
  );

  assert.equal(adjustment?.divergenceId, 'div-005');
  assert.deepEqual(adjustment?.itemIds, ['dead-sea-scrolls']);
});

test('div-010 masks a Latin-script citation mismatch that is punctuation-only after remapping', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-010');
  const entry = {
    id: 'gbt-latin-cite',
    oracle: '（Boobier，2020）',
    citum: '(Boobier, 2020)',
    match: false,
  };
  const citationFixture = { id: 'gbt-latin-cite', items: [{ id: 'gbt-latin-book' }] };
  const testItems = {
    'gbt-latin-book': { id: 'gbt-latin-book', type: 'book', language: 'en-US' },
  };

  const adjustment = explainCitationMismatchFromDiv010(entry, citationFixture, testItems, divergenceRule);

  assert.equal(adjustment?.divergenceId, 'div-010');
  assert.deepEqual(adjustment?.itemIds, ['gbt-latin-book']);
});

test('div-010 language detection rejects non-informative, private-use, and malformed tags', () => {
  for (const language of ['und', 'mul', 'zxx', 'x-private', 'x-Latn', 'e', '1en', '123']) {
    assert.equal(isLatinScriptLanguage(language), false, language);
  }
  assert.equal(isLatinScriptLanguage('en-US'), true);
  assert.equal(isLatinScriptLanguage('zh-Latn'), true);
});

test('div-010 does not mask non-informative, private-use, or malformed language tags', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-010');
  const entry = {
    id: 'gbt-unknown-language-cite',
    oracle: '（Boobier，2020）',
    citum: '(Boobier, 2020)',
    match: false,
  };
  const citationFixture = { id: entry.id, items: [{ id: 'gbt-unknown-language-book' }] };

  for (const language of ['und', 'mul', 'zxx', 'x-private', 'x-Latn', 'e', '1en', '123']) {
    const testItems = {
      'gbt-unknown-language-book': { id: 'gbt-unknown-language-book', type: 'book', language },
    };
    assert.equal(
      explainCitationMismatchFromDiv010(entry, citationFixture, testItems, divergenceRule),
      null,
      language
    );
  }
});

test('div-010 does not mask a CJK-script citation, even with the same full-width delta', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-010');
  const entry = {
    id: 'gbt-cjk-cite',
    oracle: '（博伯尔，2023）',
    citum: '（博伯尔，2023）',
    match: false,
  };
  const citationFixture = { id: 'gbt-cjk-cite', items: [{ id: 'gbt-cjk-book' }] };
  const testItems = {
    'gbt-cjk-book': { id: 'gbt-cjk-book', type: 'book', language: 'zh-CN' },
  };

  const adjustment = explainCitationMismatchFromDiv010(entry, citationFixture, testItems, divergenceRule);

  assert.equal(adjustment, null);
});

test('div-010 does not mask a Latin-script citation when the delta is not punctuation-only', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-010');
  const entry = {
    id: 'gbt-latin-cite-wrong-year',
    oracle: '（Boobier，2020）',
    citum: '(Boobier, 2021)',
    match: false,
  };
  const citationFixture = { id: 'gbt-latin-cite-wrong-year', items: [{ id: 'gbt-latin-book' }] };
  const testItems = {
    'gbt-latin-book': { id: 'gbt-latin-book', type: 'book', language: 'en-US' },
  };

  const adjustment = explainCitationMismatchFromDiv010(entry, citationFixture, testItems, divergenceRule);

  assert.equal(adjustment, null);
});

test('div-010 masks a Latin-script bibliography mismatch that is punctuation-only after remapping', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-010');
  const entry = {
    id: 'gbt-latin-book',
    oracle: 'New York：Bantam Dell Publishing Group，1988.',
    citum: 'New York: Bantam Dell Publishing Group, 1988.',
    match: false,
  };
  const testItems = {
    'gbt-latin-book': { id: 'gbt-latin-book', type: 'book', language: 'en-US' },
  };

  const adjustment = explainBibliographyMismatchFromDiv010(entry, testItems, divergenceRule);

  assert.equal(adjustment?.divergenceId, 'div-010');
  assert.deepEqual(adjustment?.itemIds, ['gbt-latin-book']);
});

test('stripGbtDateAnnotations removes era parentheticals, approximate-year brackets, and printing suffix', () => {
  assert.equal(stripGbtDateAnnotations('1947（民国三十六年）'), '1947');
  assert.equal(stripGbtDateAnnotations('1705 (康熙四十四年)'), '1705');
  assert.equal(stripGbtDateAnnotations('[1936]'), '1936');
  assert.equal(stripGbtDateAnnotations('1995印刷'), '1995');
  assert.equal(stripGbtDateAnnotations('1957—1990'), '1957—1990');
});

test('div-011 masks a GB/T era-year annotation citeproc-js drops, confirmed against the standard\'s own §7.5.4.1 example', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-011');
  const entry = {
    id: 'gbt7714.7.5.4.1:1',
    oracle: '[74][M]. 1947',
    citum: '[74][M]. 1947（民国三十六年）',
    match: false,
  };
  const testItems = {
    'gbt7714.7.5.4.1:1': { id: 'gbt7714.7.5.4.1:1', note: 'issued: 1947（民国三十六年）' },
  };

  const adjustment = explainBibliographyMismatchFromDiv011(entry, testItems, divergenceRule);

  assert.equal(adjustment?.divergenceId, 'div-011');
  assert.deepEqual(adjustment?.itemIds, ['gbt7714.7.5.4.1:1']);
});

test('div-011 masks citeproc-js adding a spurious uncertainty bracket to an open-ended date range (§8.4.2)', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-011');
  const entry = {
    id: 'gbt7714.8.4.2:2',
    oracle: '[111]…北京：北京图书馆，1957—[1990]',
    citum: '[111]…北京：北京图书馆，1957—1990',
    match: false,
  };
  const testItems = {
    'gbt7714.8.4.2:2': { id: 'gbt7714.8.4.2:2', note: 'issued: 1957/1990' },
  };

  const adjustment = explainBibliographyMismatchFromDiv011(entry, testItems, divergenceRule);

  assert.equal(adjustment?.divergenceId, 'div-011');
});

test('div-011 does not mask a mismatch on an item without a note-field issued override', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-011');
  const entry = {
    id: 'gbt-no-override',
    oracle: '[1]. 1947',
    citum: '[1]. 1947（民国三十六年）',
    match: false,
  };
  const testItems = {
    'gbt-no-override': { id: 'gbt-no-override' },
  };

  assert.equal(explainBibliographyMismatchFromDiv011(entry, testItems, divergenceRule), null);
});

test('div-011 does not mask a mismatch that survives annotation stripping', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-011');
  const entry = {
    id: 'gbt7714.7.5.4.1:1',
    oracle: '[74][M]. 1947',
    citum: '[74][M]. 1948（民国三十六年）',
    match: false,
  };
  const testItems = {
    'gbt7714.7.5.4.1:1': { id: 'gbt7714.7.5.4.1:1', note: 'issued: 1947（民国三十六年）' },
  };

  assert.equal(explainBibliographyMismatchFromDiv011(entry, testItems, divergenceRule), null);
});

test('registered divergence adjustments skip order inspection without failures', () => {
  const rawResults = {
    citations: {
      total: 1,
      passed: 1,
      failed: 0,
      entries: [{ id: 'cite-1', oracle: '[1]', citum: '[1]', match: true }],
    },
    bibliography: {
      total: 1,
      passed: 1,
      failed: 0,
      entries: [{ oracle: 'Alpha.', citum: 'Alpha.', match: true }],
    },
  };

  const adjusted = attachRegisteredDivergenceAdjustments(
    rawResults,
    ['Alpha.'],
    ['ITEM-1'],
    {
      'ITEM-1': {
        id: 'ITEM-1',
        type: 'book',
        title: 'Alpha',
        author: [{ family: 'Able' }],
      },
    },
    [{ id: 'cite-1', items: [{ id: 'ITEM-1' }] }]
  );

  assert.equal(adjusted.bibliographyOrder, null);
  assert.equal(adjusted.adjusted.citations.passed, 1);
  assert.deepEqual(adjusted.adjusted.divergenceSummary, {});
});

test('div-008 detection identifies same-family named items reversed in citum output', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-008');

  const testItems = {
    'ITEM-A': { id: 'ITEM-A', type: 'article-journal', title: 'Given-First Paper', author: [{ family: 'Smith', given: 'Jane' }] },
    'ITEM-B': { id: 'ITEM-B', type: 'article-journal', title: 'Title-First Paper', author: [{ family: 'Smith', given: 'Patricia' }] },
  };

  // Oracle: Jane before Patricia (given-name J < P)
  // Citum:  Patricia before Jane (title G < T)
  const divergence = detectDiv008OrderDifference(
    ['Smith, Jane. Given-First Paper.', 'Smith, Patricia. Title-First Paper.'],
    ['ITEM-B', 'ITEM-A'],
    testItems,
    divergenceRule
  );

  assert.equal(divergence?.divergenceId, 'div-008');
  assert.deepEqual(divergence?.swappedPairs, [['ITEM-A', 'ITEM-B']]);
  assert.deepEqual(divergence?.affectedIds, ['ITEM-A', 'ITEM-B']);
});

test('div-008 detection fires when same-family items are separated by an anonymous item', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-008');

  const testItems = {
    'ITEM-A': { id: 'ITEM-A', type: 'article-journal', title: 'Given-First Paper', author: [{ family: 'Smith', given: 'Jane' }] },
    'ITEM-ANON': { id: 'ITEM-ANON', type: 'legal_case', title: 'Anonymous Case' },
    'ITEM-B': { id: 'ITEM-B', type: 'article-journal', title: 'Title-First Paper', author: [{ family: 'Smith', given: 'Patricia' }] },
  };

  // Oracle named-only order: Jane, Patricia — but an anonymous item sits between them in full order.
  const divergence = detectDiv008OrderDifference(
    [
      'Smith, Jane. Given-First Paper.',
      'Anonymous Case.',
      'Smith, Patricia. Title-First Paper.',
    ],
    ['ITEM-B', 'ITEM-ANON', 'ITEM-A'],
    testItems,
    divergenceRule
  );

  assert.equal(divergence?.divergenceId, 'div-008');
  assert.deepEqual(divergence?.swappedPairs, [['ITEM-A', 'ITEM-B']]);
});

test('div-008 detection returns null when oracle and citum order agree', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-008');

  const testItems = {
    'ITEM-A': { id: 'ITEM-A', type: 'article-journal', title: 'Alpha', author: [{ family: 'Smith', given: 'Jane' }] },
    'ITEM-B': { id: 'ITEM-B', type: 'article-journal', title: 'Beta', author: [{ family: 'Smith', given: 'Patricia' }] },
  };

  const result = detectDiv008OrderDifference(
    ['Smith, Jane. Alpha.', 'Smith, Patricia. Beta.'],
    ['ITEM-A', 'ITEM-B'],
    testItems,
    divergenceRule
  );

  assert.equal(result, null);
});

test('explainCitationMismatchFromDiv008 masks labels for swapped same-family items', () => {
  const policy = loadVerificationPolicy();
  const divergenceRule = resolveRegisteredDivergence(policy, 'div-008');

  const testItems = {
    'ITEM-A': { id: 'ITEM-A', type: 'article-journal', title: 'Given-First', author: [{ family: 'Smith', given: 'Jane' }] },
    'ITEM-B': { id: 'ITEM-B', type: 'article-journal', title: 'Title-First', author: [{ family: 'Smith', given: 'Patricia' }] },
  };

  const div008Info = detectDiv008OrderDifference(
    ['Smith, Jane. Given-First.', 'Smith, Patricia. Title-First.'],
    ['ITEM-B', 'ITEM-A'],
    testItems,
    divergenceRule
  );

  // oracle: ITEM-A=[1], ITEM-B=[2]; citum: ITEM-B=[1], ITEM-A=[2]
  const entry = { id: 'cite-smith', oracle: '[1]', citum: '[2]', match: false };
  const citationFixture = { id: 'cite-smith', items: [{ id: 'ITEM-A' }] };

  const adjustment = explainCitationMismatchFromDiv008(entry, citationFixture, div008Info);

  assert.equal(adjustment?.divergenceId, 'div-008');
  assert.deepEqual(adjustment?.itemIds, ['ITEM-A']);
  assert.deepEqual(adjustment?.oracleLabels, [1]);
  assert.deepEqual(adjustment?.citumLabels, [2]);
});

test('div-008 and div-004 fire independently when both conditions are present', () => {
  const rawResults = {
    style: 'acm-sig-proceedings',
    citations: {
      total: 2,
      passed: 0,
      failed: 2,
      entries: [
        { id: 'cite-smith-jane', oracle: '[1]', citum: '[2]', match: false },
        { id: 'cite-anon', oracle: '[3]', citum: '[2]', match: false },
      ],
    },
    bibliography: {
      total: 3,
      passed: 3,
      failed: 0,
      entries: [],
    },
    citationsByType: {},
    componentSummary: {},
    orderingIssues: 0,
  };

  const testItems = {
    'ITEM-A': { id: 'ITEM-A', type: 'article-journal', title: 'Given-First', author: [{ family: 'Smith', given: 'Jane' }] },
    'ITEM-B': { id: 'ITEM-B', type: 'article-journal', title: 'Title-First', author: [{ family: 'Smith', given: 'Patricia' }] },
    'ITEM-ANON': { id: 'ITEM-ANON', type: 'legal_case', title: 'Anon Case' },
  };

  // Oracle: Jane[1], Patricia[2], Anon[3]
  // Citum:  Patricia[1], Jane[2], Anon[3]  ← div-008 (Smith swap); anon stays last
  const adjusted = attachRegisteredDivergenceAdjustments(
    rawResults,
    [
      'Smith, Jane. Given-First.',
      'Smith, Patricia. Title-First.',
      'Anon Case.',
    ],
    ['ITEM-B', 'ITEM-A', 'ITEM-ANON'],
    testItems,
    [
      { id: 'cite-smith-jane', items: [{ id: 'ITEM-A' }] },
      { id: 'cite-anon', items: [{ id: 'ITEM-ANON' }] },
    ]
  );

  const summary = adjusted.adjusted.divergenceSummary;
  // Both divergences must appear in the summary independently.
  assert.ok(summary['div-008'], 'div-008 should be in divergence summary');
  assert.ok(summary['div-004'], 'div-004 should also be in divergence summary');
  assert.deepEqual(summary['div-008'].swappedPairs, [['ITEM-A', 'ITEM-B']]);

  // The Smith Jane citation is explained by whichever divergence fires first —
  // both produce equivalent label masking; the exact divergenceId is not load-bearing.
  const smithEntry = adjusted.adjusted.citations.entries[0];
  assert.equal(smithEntry.match, true, 'a registered divergence should explain the Smith citation mismatch');
  assert.ok(
    ['div-004', 'div-008'].includes(smithEntry.appliedDivergence?.divergenceId),
    `applied divergence should be div-004 or div-008, got: ${smithEntry.appliedDivergence?.divergenceId}`
  );
});

test('resolveAuthoredStylePath prefers styles/<name>.yaml over embedded', () => {
  const dir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'oracle-resolve-'));
  try {
    fs.mkdirSync(path.join(dir, 'embedded'));
    fs.writeFileSync(path.join(dir, 'acme.yaml'), 'info: {}');
    fs.writeFileSync(path.join(dir, 'embedded', 'acme.yaml'), 'info: {}');

    const resolved = resolveAuthoredStylePath(dir, 'acme');
    assert.equal(resolved, path.join(dir, 'acme.yaml'));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('resolveAuthoredStylePath falls back to styles/embedded/<name>.yaml', () => {
  const dir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'oracle-resolve-'));
  try {
    fs.mkdirSync(path.join(dir, 'embedded'));
    fs.writeFileSync(path.join(dir, 'embedded', 'beta.yaml'), 'info: {}');

    const resolved = resolveAuthoredStylePath(dir, 'beta');
    assert.equal(resolved, path.join(dir, 'embedded', 'beta.yaml'));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('resolveAuthoredStylePath returns null when no authored YAML exists', () => {
  const dir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'oracle-resolve-'));
  try {
    fs.mkdirSync(path.join(dir, 'embedded'));
    const resolved = resolveAuthoredStylePath(dir, 'missing');
    assert.equal(resolved, null);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('resolveAuthoredStylePath finds versioned name in embedded/ via prefix scan', () => {
  // 'apa' should find 'apa-7th.yaml' in embedded/ when no exact match exists
  const dir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'oracle-resolve-'));
  try {
    const embeddedDir = path.join(dir, 'embedded');
    fs.mkdirSync(embeddedDir);
    fs.writeFileSync(path.join(embeddedDir, 'apa-7th.yaml'), '{}');
    const resolved = resolveAuthoredStylePath(dir, 'apa');
    assert.equal(resolved, path.join(embeddedDir, 'apa-7th.yaml'));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('resolveAuthoredStylePath prefers embedded/ prefix over styles/ prefix', () => {
  // A root-level 'apa-variant.yaml' must not shadow embedded 'apa-7th.yaml'.
  const dir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'oracle-resolve-'));
  try {
    const embeddedDir = path.join(dir, 'embedded');
    fs.mkdirSync(embeddedDir);
    fs.writeFileSync(path.join(embeddedDir, 'apa-7th.yaml'), '{}');
    fs.writeFileSync(path.join(dir, 'apa-classic.yaml'), '{}');
    // embedded prefix wins over root prefix
    const resolved = resolveAuthoredStylePath(dir, 'apa');
    assert.equal(resolved, path.join(embeddedDir, 'apa-7th.yaml'));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('resolveAuthoredStylePath picks the first embedded prefix match deterministically', () => {
  const dir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'oracle-resolve-'));
  try {
    const embeddedDir = path.join(dir, 'embedded');
    fs.mkdirSync(embeddedDir);
    fs.writeFileSync(path.join(embeddedDir, 'apa-7th.yaml'), '{}');
    fs.writeFileSync(path.join(embeddedDir, 'apa-6th.yaml'), '{}');
    const resolved = resolveAuthoredStylePath(dir, 'apa');
    assert.equal(resolved, path.join(embeddedDir, 'apa-6th.yaml'));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('parseCitumRenderOutput keeps integral citations separate from keyed citations', () => {
  const parsed = parseCitumRenderOutput(
    [
      '=== apa-7th.yaml ===',
      '',
      'CITATIONS (Non-Integral):',
      '  [single-item] (Kuhn, 1962)',
      '',
      'CITATIONS (Integral):',
      '  Kuhn (1962)',
      '',
      'BIBLIOGRAPHY:',
      '  [ITEM-1] Kuhn, T. S. (1962). _The Structure of Scientific Revolutions_.',
      '',
    ].join('\n'),
    { 'ITEM-1': { id: 'ITEM-1' } }
  );

  assert.deepEqual(parsed.citations, { 'single-item': '(Kuhn, 1962)' });
  assert.deepEqual(parsed.integralCitations, ['Kuhn (1962)']);
  assert.deepEqual(parsed.bibliographyOrderIds, ['ITEM-1']);
  assert.deepEqual(parsed.bibliographyIds, ['ITEM-1']);
  assert.deepEqual(parsed.bibliography, [
    'Kuhn, T. S. (1962). _The Structure of Scientific Revolutions_.',
  ]);
});

test('component parsing slices normalized citeproc flush-layout text', () => {
  const entry = [
    '  <div class="csl-entry">',
    '    <div class="csl-left-margin">[189]</div><div class="csl-right-inline">Chernik B E. Introduction to library services for library technicians[M]. Littleton, Colo.：Libraries Unlimited, Inc.，1982.</div>',
    '  </div>',
  ].join('\n');
  const components = parseComponents(entry, {
    id: 'ITEM-189',
    type: 'book',
    author: [{ family: 'Chernik', given: 'B. E.' }],
    title: 'Introduction to library services for library technicians',
    publisher: 'Libraries Unlimited, Inc.',
    issued: { 'date-parts': [[1982]] },
  });

  assert.equal(components.title.value, 'Introduction to library services for library technicians');
  assert.equal(components.publisher.value, 'Libraries Unlimited, Inc.');
  assert.equal(components.year.value, '1982');
});

test('bibliography entries pair by item id instead of text similarity', () => {
  const pairs = matchBibliographyEntries(
    ['Oracle alpha', 'Oracle beta'],
    ['Citum beta', 'Citum alpha'],
    ['ITEM-A', 'ITEM-B'],
    ['ITEM-B', 'ITEM-A']
  );

  assert.equal(pairs.length, 2);
  assert.deepEqual(
    pairs.map(({ id, oracle, citum, pairingMethod, comparisonState, compatibilityEligible }) => ({
      id,
      oracle,
      citum,
      pairingMethod,
      comparisonState,
      compatibilityEligible,
    })),
    [
      {
        id: 'ITEM-A',
        oracle: 'Oracle alpha',
        citum: 'Citum alpha',
        pairingMethod: 'id',
        comparisonState: 'paired',
        compatibilityEligible: true,
      },
      {
        id: 'ITEM-B',
        oracle: 'Oracle beta',
        citum: 'Citum beta',
        pairingMethod: 'id',
        comparisonState: 'paired',
        compatibilityEligible: true,
      },
    ]
  );
});

test('bibliography ID pairing treats one-sided output as a compatibility-eligible cardinality failure', () => {
  const pairs = matchBibliographyEntries(
    ['Oracle alpha'],
    ['Citum beta'],
    ['ITEM-A'],
    ['ITEM-B']
  );

  assert.deepEqual(
    pairs.map(({ id, comparisonState, pairingMethod, compatibilityEligible }) => ({
      id,
      comparisonState,
      pairingMethod,
      compatibilityEligible,
    })),
    [
      {
        id: 'ITEM-A',
        comparisonState: 'oracle-only',
        pairingMethod: 'id',
        compatibilityEligible: true,
      },
      {
        id: 'ITEM-B',
        comparisonState: 'citum-only',
        pairingMethod: 'id',
        compatibilityEligible: true,
      },
    ]
  );
});

test('bibliography similarity pairing leaves unrelated one-sided outputs unresolved', () => {
  const pairs = matchBibliographyEntries(
    ['Alpha bibliography record'],
    ['Zulu completely unrelated output']
  );

  assert.equal(pairs.length, 2);
  assert.deepEqual(
    pairs.map(({ pairingMethod, comparisonState, compatibilityEligible }) => ({
      pairingMethod,
      comparisonState,
      compatibilityEligible,
    })),
    [
      {
        pairingMethod: 'similarity',
        comparisonState: 'unresolved-unpaired',
        compatibilityEligible: false,
      },
      {
        pairingMethod: 'similarity',
        comparisonState: 'unresolved-unpaired',
        compatibilityEligible: false,
      },
    ]
  );
});

test('GB/T bibliography grading requires exact normalized equality', () => {
  const comparison = compareText(
    'Alpha beta gamma delta',
    'Alpha beta gamma delta extra'
  );
  assert.equal(comparison.match, true, 'similarity remains available for diagnostics');
  assert.equal(
    bibliographyComparisonMatches('gb-t-7714-2025-numeric', comparison),
    false
  );
  assert.equal(bibliographyComparisonMatches('apa-7th', comparison), true);
});

test('GB/T bibliography grading catches a missing terminal period', () => {
  const comparison = compareText(
    'New York：Bantam Dell Publishing Group，1988.',
    'New York：Bantam Dell Publishing Group，1988'
  );
  assert.equal(comparison.match, true, 'similarity remains available for diagnostics');
  assert.equal(
    bibliographyComparisonMatches('gb-t-7714-2025-numeric', comparison),
    false
  );
  assert.equal(bibliographyComparisonMatches('apa-7th', comparison), true);
});

test('compareText reports a case mismatch even when a trailing-punctuation difference also co-occurs', () => {
  const comparison = compareText(
    'Skinner, Quentin. “Meaning and Understanding in the History of Ideas.” History and Theory, vol. 8, no. 1, 2002, pp. 3–53, https://doi.org/10.2307/2504188.',
    'Skinner, Quentin. “Meaning and understanding in the history of ideas.” History and Theory, vol. 8, no. 1, 2002, pp. 3–53, https://doi.org/10.2307/2504188'
  );
  assert.equal(comparison.match, false);
  assert.equal(comparison.caseMismatch, true);
});

test('compareText does not report a case mismatch for a pure trailing-punctuation difference', () => {
  const comparison = compareText(
    'LeCun, Yann. “Deep Learning.” Nature, https://doi.org/10.1038/nature14539.',
    'LeCun, Yann. “Deep Learning.” Nature, https://doi.org/10.1038/nature14539'
  );
  assert.equal(comparison.caseMismatch, false);
  assert.equal(comparison.match, true, 'similarity fallback still applies for non-strict styles');
});

test('compareText keeps estimated-date brackets visible to exact parity', () => {
  const comparison = compareText(
    'Smith, John. [1750?]. Title of First Work.',
    'Smith, John. 1750? Title of First Work.'
  );
  assert.equal(comparison.match, true, 'lenient compatibility remains unchanged');
  assert.equal(comparison.exactMatch, false);
  assert.equal(comparison.exactExpected, 'Smith, John. [1750?]. Title of First Work.');
  assert.equal(comparison.exactActual, 'Smith, John. 1750? Title of First Work.');
});

test('compareText exact parity preserves punctuation and case', () => {
  const punctuation = compareText(
    'LeCun, Yann. “Deep Learning.” Nature.',
    'LeCun, Yann. “Deep Learning.” Nature'
  );
  assert.equal(punctuation.match, true);
  assert.equal(punctuation.exactMatch, false);

  const letterCase = compareText(
    'Skinner, Quentin. Meaning and Understanding.',
    'Skinner, Quentin. Meaning and understanding.',
    { caseSensitive: false }
  );
  assert.equal(letterCase.match, true);
  assert.equal(letterCase.exactMatch, false);
});

test('compareText exact parity preserves role-label drift', () => {
  const caseDrift = compareText(
    'Smith, John, and Jane Doe (Eds.). A Long Collected Work.',
    'Smith, John, and Jane Doe (eds.). A Long Collected Work.',
    { caseSensitive: false }
  );
  assert.equal(caseDrift.match, true);
  assert.equal(caseDrift.exactMatch, false);

  const missingLabel = compareText(
    'Smith, John, and Jane Doe (Eds.). A Long Collected Work.',
    'Smith, John, and Jane Doe. A Long Collected Work.'
  );
  assert.equal(missingLabel.match, true);
  assert.equal(missingLabel.exactMatch, false);
});

test('compareText exact parity removes markup but preserves visible numbering', () => {
  const comparison = compareText(
    '<div class="csl-entry">193. <i>Smith</i> and Doe.</div>',
    '**Smith** and Doe.'
  );
  assert.equal(comparison.exactMatch, false);
  assert.equal(comparison.exactAdjudication, 'unresolved');
  assert.equal(comparison.exactExpected, '193. Smith and Doe.');
  assert.equal(comparison.exactActual, 'Smith and Doe.');
});

test('compareText exact parity preserves bracketed and parenthesized numbering', () => {
  const bracketed = compareText('[39] Smith and Doe.', '[40] Smith and Doe.');
  const parenthesized = compareText('(39) Smith and Doe.', '(40) Smith and Doe.');

  assert.equal(bracketed.exactMatch, false);
  assert.equal(parenthesized.exactMatch, false);
  assert.equal(bracketed.exactAdjudication, 'unresolved');
  assert.equal(parenthesized.exactAdjudication, 'unresolved');
});

test('compareText exact parity: second-field-align left-margin and right-inline concatenate flush, no inserted space', () => {
  // citeproc-js renders second-field-align bibliographies (numeric styles
  // like IEEE) as adjacent sibling <div class="csl-left-margin"> (list
  // number) and <div class="csl-right-inline"> (entry body) divs with the
  // visual gap done via CSS, not a text character -- there is genuinely no
  // space in the rendered text between them. A style whose own bibliography
  // template renders the same bracketed number as literal content (IEEE)
  // therefore already agrees with the oracle on a flush, spaceless join;
  // verified across the full embedded+exemplar oracle-parity corpus that
  // zero mismatches are whitespace-only at this boundary (2026-07-30 spike,
  // see docs/architecture/audits/2026-07-30_EMBEDDED_PARITY_CLASS_A.md).
  // This test guards against reintroducing a join-space "fix" that would
  // regress this currently-passing pairing.
  const comparison = compareText(
    '<div class="csl-left-margin">[2]</div><div class="csl-right-inline">S. Hawking, A Brief History of Time.</div>',
    '[2]S. Hawking, A Brief History of Time.'
  );
  assert.equal(comparison.exactExpected, '[2]S. Hawking, A Brief History of Time.');
  assert.equal(comparison.exactActual, '[2]S. Hawking, A Brief History of Time.');
  assert.equal(comparison.exactMatch, true);
});

test('compareComponents reports differing component values as mismatches', () => {
  const { differences, matches } = compareComponents(
    {
      title: { found: true, value: 'Alpha' },
      contributors: { found: false },
      year: { found: false },
      containerTitle: { found: false },
      volume: { found: false },
      issue: { found: false },
      pages: { found: false },
      publisher: { found: false },
      doi: { found: false },
      edition: { found: false },
      editors: { found: false },
      translators: { found: false },
      interviewers: { found: false },
      recipients: { found: false },
    },
    {
      title: { found: true, value: 'Beta' },
      contributors: { found: false },
      year: { found: false },
      containerTitle: { found: false },
      volume: { found: false },
      issue: { found: false },
      pages: { found: false },
      publisher: { found: false },
      doi: { found: false },
      edition: { found: false },
      editors: { found: false },
      translators: { found: false },
      interviewers: { found: false },
      recipients: { found: false },
    },
    {}
  );

  assert.deepEqual(matches, []);
  assert.deepEqual(differences, [{
    component: 'title',
    issue: 'value_mismatch',
    expected: 'Alpha',
    found: 'Beta',
    detail: 'Value differs between oracle and Citum',
  }]);
});
