const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  applyFixes,
  convertItemsAliasInText,
  expandAnonymousAnchorsInText,
  lintAnonymousAnchors,
  lintEmptyObjectLiterals,
  lintEmptyStyleVersion,
  lintDeprecatedTemplateTerms,
  lintPhraseLikeTermMessages,
  lintHardcodedLocaleProse,
  lintLegacyItemsAlias,
  lintParsedStyle,
  listStyleFiles,
  loadLocaleAffixValueSet,
  normalizeAffixText,
  removeEmptyVersionInText,
  stripAnonymousAnchorMarkersInText,
  summarize,
} = require('./style-structure-lint');

function writeTempStyle(content) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'style-structure-lint-'));
  const filePath = path.join(tempDir, 'fixture.yaml');
  fs.writeFileSync(filePath, content);
  return filePath;
}

test('STYLE001 detects anonymous generated anchors', () => {
  const content = `version: ""
citation:
  template:
    - contributor: author
      shorten: &id001
        min: 4
        use-first: 1
    - contributor: editor
      shorten: *id001
`;

  const violations = lintAnonymousAnchors('styles/fixture.yaml', content);

  assert.equal(violations.length, 2);
  assert.equal(violations[0].ruleId, 'STYLE001');
});

test('STYLE012 rejects whitespace-form empty objects in fields and comments', () => {
  const content = `options:
  titles:
    component: {}
    label: { }
    periodical: {
    }
# do not recommend {  }
`;

  const violations = lintEmptyObjectLiterals('styles/fixture.yaml', content);

  assert.equal(violations.length, 4);
  assert.equal(violations.every((violation) => violation.ruleId === 'STYLE012'), true);
  assert.deepEqual(violations.map((violation) => violation.line), [3, 4, 5, 7]);
});

test('tracked style corpus contains no literal empty objects', () => {
  const violations = listStyleFiles().flatMap((filePath) =>
    lintEmptyObjectLiterals(filePath, fs.readFileSync(filePath, 'utf8'))
  );

  assert.deepEqual(violations, []);
});

test('default style discovery returns unique tracked paths', () => {
  const files = listStyleFiles();
  const relativePaths = files.map((filePath) =>
    path.relative(process.cwd(), filePath).split(path.sep).join('/')
  );

  assert.equal(new Set(files).size, files.length);
  assert.equal(relativePaths.some((filePath) => filePath.startsWith('styles/embedded/')), false);
  assert.equal(
    relativePaths.every(
      (filePath) =>
        /^styles\/(?!embedded\/).+\.yaml$/.test(filePath) ||
        /^crates\/citum-schema-style\/embedded\/styles\/[^/]+\.yaml$/.test(filePath)
    ),
    true
  );
});

test('rule-filtered summaries isolate STYLE012 from unrelated findings', () => {
  const summary = summarize(
    [
      {
        fixed: false,
        violations: [
          { ruleId: 'STYLE008' },
          { ruleId: 'STYLE012' },
        ],
      },
    ],
    ['STYLE012']
  );

  assert.equal(summary.filesWithViolations, 1);
  assert.deepEqual(summary.violations, [{ ruleId: 'STYLE012' }]);
});

test('STYLE002 flags empty substitute candidates that should use none', () => {
  const content = `version: ""
options:
  substitute:
    candidates: []
    overrides:
      legal_case:
        - title
`;
  const data = {
    version: '',
    options: {
      substitute: {
        candidates: [],
        overrides: {
          legal_case: ['title'],
        },
      },
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data);

  assert.equal(violations.some((violation) => violation.ruleId === 'STYLE002'), true);
});

test('STYLE003 flags duplicate citation shorten blocks that can be hoisted safely', () => {
  const content = `version: ""
citation:
  template:
    - contributor: author
      shorten:
        min: 4
        use-first: 1
    - contributor: editor
      shorten:
        min: 4
        use-first: 1
`;
  const data = {
    version: '',
    citation: {
      template: [
        { contributor: 'author', shorten: { min: 4, 'use-first': 1 } },
        { contributor: 'editor', shorten: { min: 4, 'use-first': 1 } },
      ],
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data);

  assert.equal(violations.some((violation) => violation.ruleId === 'STYLE003'), true);
});

test('STYLE003 does not flag when some contributor components intentionally differ', () => {
  const content = `version: ""
bibliography:
  template:
    - contributor: author
      shorten:
        min: 5
        use-first: 1
    - contributor: editor
`;
  const data = {
    version: '',
    bibliography: {
      template: [
        { contributor: 'author', shorten: { min: 5, 'use-first': 1 } },
        { contributor: 'editor' },
      ],
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data);

  assert.equal(violations.some((violation) => violation.ruleId === 'STYLE003'), false);
});

test('STYLE004 flags type variants identical to the base template', () => {
  const content = `version: ""
bibliography:
  template:
    - contributor: author
    - title: primary
  type-variants:
    article-journal:
      - contributor: author
      - title: primary
`;
  const data = {
    version: '',
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
      ],
      'type-variants': {
        'article-journal': [
          { contributor: 'author' },
          { title: 'primary' },
        ],
      },
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data);

  assert.equal(violations.some((violation) => violation.ruleId === 'STYLE004'), true);
});

test('STYLE004 skips Template V3 diff variants', () => {
  const content = `version: ""
bibliography:
  template:
    - contributor: author
    - title: primary
  type-variants:
    article-journal:
      modify:
        - match:
            title: primary
          suffix: "."
`;
  const data = {
    version: '',
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
      ],
      'type-variants': {
        'article-journal': {
          modify: [
            { match: { title: 'primary' }, suffix: '.' },
          ],
        },
      },
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data);

  assert.equal(violations.some((violation) => violation.ruleId === 'STYLE004'), false);
});

test('applyFixes replaces empty substitute candidates, hoists shorten config, and drops duplicate variants', () => {
  const style = {
    version: '',
    options: {
      substitute: {
        candidates: [],
        overrides: {
          'legal-case': ['title'],
        },
      },
    },
    citation: {
      template: [
        { contributor: 'author', shorten: { min: 4, 'use-first': 1 } },
        { contributor: 'editor', shorten: { min: 4, 'use-first': 1 } },
      ],
      'type-variants': {
        article: [
          { contributor: 'author', shorten: { min: 4, 'use-first': 1 } },
          { contributor: 'editor', shorten: { min: 4, 'use-first': 1 } },
        ],
      },
    },
  };

  const changed = applyFixes(style);

  assert.equal(changed, true);
  assert.deepEqual(style.options.substitute, {
    candidates: 'none',
    overrides: { 'legal-case': ['title'] },
  });
  assert.deepEqual(style.citation.options.contributors.shorten, { min: 4, 'use-first': 1 });
  assert.equal(style.citation.template.every((component) => component.shorten === undefined), true);
  assert.equal(style.citation['type-variants'], undefined);
});

test('yaml round-trip autofix removes anonymous shorten anchors from authored text', () => {
  const filePath = writeTempStyle(`version: ""
citation:
  template:
    - contributor: author
      shorten: &id001
        min: 4
        use-first: 1
    - contributor: editor
      shorten: *id001
`);
  const yaml = require('js-yaml');
  const style = yaml.load(fs.readFileSync(filePath, 'utf8'));

  applyFixes(style);
  fs.writeFileSync(filePath, yaml.dump(style, { noRefs: true, lineWidth: -1 }));
  const output = fs.readFileSync(filePath, 'utf8');

  assert.equal(output.includes('&id001'), false);
  assert.equal(output.includes('*id001'), false);
});

test('STYLE001 text fixer expands aliases without reformatting unrelated YAML', () => {
  const input = `info:
  title: Example
citation:
  template:
    - contributor: author
      shorten: &id001
        min: 4
        use-first: 1
    - contributor: editor
      shorten: *id001
  delimiter: ". "
`;

  const output = expandAnonymousAnchorsInText(input);

  assert.equal(output, `info:
  title: Example
citation:
  template:
    - contributor: author
      shorten:
        min: 4
        use-first: 1
    - contributor: editor
      shorten:
        min: 4
        use-first: 1
  delimiter: ". "
`);
});

test('STYLE001 text fixer strips stray anchor markers left behind after expansion', () => {
  const input = `bibliography:
  template:
    - group:
      - contributor: editor
        shorten: &id003
          min: 4
          use-first: 3
      - title: parent-monograph
`;

  const output = stripAnonymousAnchorMarkersInText(input);

  assert.equal(output, `bibliography:
  template:
    - group:
      - contributor: editor
        shorten:
          min: 4
          use-first: 3
      - title: parent-monograph
`);
});

test('STYLE001 text fixer expands inline aliases and removes inline anchor definitions', () => {
  const input = `bibliography:
  template:
    - contributor: translator
      label: &id001 {term: translator, form: short, placement: suffix}
    - contributor: editor
      label: *id001
`;

  const output = expandAnonymousAnchorsInText(input);

  assert.equal(output, `bibliography:
  template:
    - contributor: translator
      label: {term: translator, form: short, placement: suffix}
    - contributor: editor
      label: {term: translator, form: short, placement: suffix}
`);
});

test('STYLE005 detects legacy items aliases in style templates', () => {
  const content = `bibliography:
  template:
    - items:
        - contributor: author
`;

  const violations = lintLegacyItemsAlias('styles/fixture.yaml', content);

  assert.equal(violations.length, 1);
  assert.equal(violations[0].ruleId, 'STYLE005');
});

test('STYLE005 text fixer rewrites items to group without touching unrelated items text', () => {
  const input = `bibliography:
  template:
    - items:
        - contributor: author
docs:
  note: "citation items remain separate"
`;

  const output = convertItemsAliasInText(input);

  assert.equal(output, `bibliography:
  template:
    - group:
        - contributor: author
docs:
  note: "citation items remain separate"
`);
});

test('STYLE006 flags raw page label prefixes on page components and diffs', () => {
  const content = `bibliography:
  template:
    - number: pages
      prefix: 'pp. '
  type-variants:
    chapter:
      modify:
        - match:
            number: pages
          prefix: 'pp. '
`;
  const data = {
    bibliography: {
      template: [
        { number: 'pages', prefix: 'pp. ' },
      ],
      'type-variants': {
        chapter: {
          modify: [
            { match: { number: 'pages' }, prefix: 'pp. ' },
          ],
        },
      },
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data)
    .filter((violation) => violation.ruleId === 'STYLE006');

  assert.equal(violations.length, 2);
  assert.equal(violations.every((violation) => violation.line === null), true);
});

test('STYLE007 detects and text-fixes empty style versions', () => {
  const content = `version: ""
info:
  title: Example
`;

  const violations = lintEmptyStyleVersion('styles/fixture.yaml', content);
  const output = removeEmptyVersionInText(content);

  assert.equal(violations.length, 1);
  assert.equal(violations[0].ruleId, 'STYLE007');
  assert.equal(output, `info:
  title: Example
`);
});

test('STYLE008 detects rendered template terms but ignores role labels', () => {
  const content = `bibliography:
  template:
    - term: accessed
      form: long
    - contributor: editor
      label:
        term: editor
        form: short
`;

  const violations = lintDeprecatedTemplateTerms('styles/fixture.yaml', content);

  assert.equal(violations.length, 1);
  assert.equal(violations[0].ruleId, 'STYLE008');
  assert.equal(violations[0].line, 3);
});

test('STYLE009 rejects phrase-like term-backed messages', () => {
  const content = `bibliography:
  template:
    - message: term.in
    - message: term.accessed
`;

  const violations = lintPhraseLikeTermMessages('styles/fixture.yaml', content);

  assert.equal(violations.length, 2);
  assert.equal(violations[0].ruleId, 'STYLE009');
  assert.equal(violations[0].line, 3);
});

test('STYLE009 allows lexical term-backed messages', () => {
  const content = `bibliography:
  template:
    - message: term.no-date
    - message: term.edition
    - message: term.volume
    - message: term.ibid
    - message: term.personal-communication
    - message: term.and
`;

  const violations = lintPhraseLikeTermMessages('styles/fixture.yaml', content);

  assert.equal(violations.length, 0);
});

test('STYLE010 flags a hardcoded prefix that duplicates a locale role verb', () => {
  const localeValueSet = new Set(['translated by', 'written by']);
  const content = `bibliography:
  template:
    - contributor: translator
      prefix: "Translated by "
`;

  const violations = lintHardcodedLocaleProse('styles/fixture.yaml', content, localeValueSet);

  assert.equal(violations.length, 1);
  assert.equal(violations[0].ruleId, 'STYLE010');
  assert.equal(violations[0].line, 4);
  assert.equal(violations[0].fixable, false);
});

test('STYLE010 ignores structural punctuation and non-linguistic literals', () => {
  const localeValueSet = new Set(['translated by']);
  const content = `bibliography:
  template:
    - number: issue
      prefix: " ("
      suffix: ")"
    - variable: doi
      prefix: ". https://doi.org/"
`;

  const violations = lintHardcodedLocaleProse('styles/fixture.yaml', content, localeValueSet);

  assert.equal(violations.length, 0);
});

test('STYLE010 does not flag prose with no locale equivalent', () => {
  const localeValueSet = new Set(['translated by']);
  const content = `bibliography:
  template:
    - date: issued
      prefix: "Recorded "
`;

  const violations = lintHardcodedLocaleProse('styles/fixture.yaml', content, localeValueSet);

  assert.equal(violations.length, 0);
});

test('STYLE011 flags an options.messages entry that duplicates a locale message', () => {
  const localeValueSet = new Set(['of']);
  const content = `options:
  messages:
    pattern.chicago-of: "of {$container}"
`;
  const data = {
    options: {
      messages: {
        'pattern.chicago-of': 'of {$container}',
      },
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data, localeValueSet);
  const style011 = violations.filter((violation) => violation.ruleId === 'STYLE011');

  assert.equal(style011.length, 1);
  assert.equal(style011[0].line, 3);
  assert.equal(style011[0].fixable, false);
});

test('STYLE011 checks citation- and bibliography-scoped options.messages too', () => {
  const localeValueSet = new Set(['of']);
  const content = `citation:
  options:
    messages:
      pattern.chicago-of: "of {$container}"
`;
  const data = {
    citation: {
      options: {
        messages: {
          'pattern.chicago-of': 'of {$container}',
        },
      },
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data, localeValueSet);
  const style011 = violations.filter((violation) => violation.ruleId === 'STYLE011');

  assert.equal(style011.length, 1);
});

test('STYLE011 does not flag a message with no locale equivalent', () => {
  const localeValueSet = new Set(['of']);
  const content = `options:
  messages:
    pattern.chicago-episode: "episode {$number}"
`;
  const data = {
    options: {
      messages: {
        'pattern.chicago-episode': 'episode {$number}',
      },
    },
  };

  const violations = lintParsedStyle('styles/fixture.yaml', content, data, localeValueSet);

  assert.equal(violations.some((violation) => violation.ruleId === 'STYLE011'), false);
});

test('normalizeAffixText strips quotes, punctuation, and casefolds', () => {
  assert.equal(normalizeAffixText('"Translated by "'), 'translated by');
  assert.equal(normalizeAffixText('", written by "'), 'written by');
  assert.equal(normalizeAffixText('" ("'), '');
  assert.equal(normalizeAffixText('". https://doi.org/"'), 'https://doi.org/');
});

test('normalizeAffixText strips a trailing YAML comment without leaking it into the value', () => {
  assert.equal(normalizeAffixText('"Illustrated by " # FIX locale-specific string'), 'illustrated by');
  assert.equal(
    normalizeAffixText('", written by "  # FIX locale-specific string; also duplicating logic'),
    'written by'
  );
});

test('STYLE010 flags a hardcoded prefix even when annotated with a trailing FIX comment', () => {
  const localeValueSet = new Set(['illustrated by']);
  const content = `bibliography:
  template:
    - contributor: illustrator
      form: long
      prefix: "Illustrated by " # FIX locale-specific string
`;

  const violations = lintHardcodedLocaleProse('styles/fixture.yaml', content, localeValueSet);

  assert.equal(violations.length, 1);
  assert.equal(violations[0].ruleId, 'STYLE010');
});

test('loadLocaleAffixValueSet builds a matchable set from en-US.yaml', () => {
  const values = loadLocaleAffixValueSet();

  assert.ok(values.has('translated by'));
  assert.ok(values.has('edited by'));
  assert.ok(values.has('written by'));
  // Multi-line .match MF2 messages are excluded — no single literal to compare against.
  assert.ok(!values.has('track'));
});

test('applyFixes removes empty version properties', () => {
  const style = {
    version: '',
    info: { title: 'Example' },
  };

  const changed = applyFixes(style);

  assert.equal(changed, true);
  assert.deepEqual(style, { info: { title: 'Example' } });
});
