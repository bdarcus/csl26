#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const yaml = require('js-yaml');

const PROJECT_ROOT = path.resolve(__dirname, '..');
const STYLES_DIR = path.join(PROJECT_ROOT, 'styles');
const REFERENCE_LOCALE_PATH = path.join(
  PROJECT_ROOT,
  'crates/citum-schema-style/embedded/locales/en-US.yaml'
);

const RULES = {
  STYLE001: 'Anonymous generated YAML anchors are not allowed in committed styles.',
  STYLE002: 'Inert substitute overrides should be removed when substitute.template is explicitly empty.',
  STYLE003: 'Duplicate component-level shorten config should be hoisted to the narrowest safe option scope.',
  STYLE004: 'Type variants identical to the base template should be removed.',
  STYLE005: 'Legacy items blocks should be authored as group blocks.',
  STYLE006: 'Page labels must use localized label-form settings instead of raw locale strings.',
  STYLE007: 'Empty style version values should be omitted so schema defaults apply.',
  STYLE008: 'Rendered template terms are deprecated; use message components instead.',
  STYLE009: 'Phrase-like term-backed messages must use localized pattern messages.',
  STYLE010: 'Hardcoded prefix/suffix prose duplicates an existing locale role verb, term, or message; use the locale mechanism instead so the text localizes. See docs/policies/LOCALIZATION_INTEGRITY.md.',
};

const FATAL_RULE_IDS = new Set(['STYLE008', 'STYLE009']);
const ALLOWED_TERM_MESSAGE_IDS = new Set([
  'and',
  'edition',
  'ibid',
  'no-date',
  'personal-communication',
  'volume',
]);

function parseArgs(argv = process.argv.slice(2)) {
  const args = {
    filePaths: [],
    strict: false,
    fix: false,
    json: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--strict') {
      args.strict = true;
    } else if (arg === '--fix') {
      args.fix = true;
    } else if (arg === '--json') {
      args.json = true;
    } else if (arg === '-h' || arg === '--help') {
      printUsage();
      process.exit(0);
    } else {
      args.filePaths.push(arg);
    }
  }

  return args;
}

function printUsage() {
  console.log(
    'Usage: node scripts/style-structure-lint.js [--fix] [--strict] [--json] [styles/file.yaml ...]'
  );
}

function repoRelative(filePath) {
  const relative = path.relative(PROJECT_ROOT, path.resolve(filePath));
  return relative && !relative.startsWith('..') ? relative : filePath;
}

function isProductionStyle(filePath) {
  return /^styles\/[^/]+\.yaml$/.test(repoRelative(filePath));
}

function isEmbeddedStyle(filePath) {
  return /^crates\/citum-schema-style\/embedded\/styles\/[^/]+\.yaml$/.test(repoRelative(filePath));
}

function listStyleFiles(filePaths = []) {
  if (filePaths.length === 0) {
    return fs.readdirSync(STYLES_DIR)
      .filter((entry) => entry.endsWith('.yaml'))
      .map((entry) => path.join(STYLES_DIR, entry))
      .sort();
  }

  return filePaths
    .map((filePath) => path.resolve(filePath))
    .filter((filePath) => isProductionStyle(filePath) || isEmbeddedStyle(filePath))
    .sort();
}

function sortValue(value) {
  if (Array.isArray(value)) {
    return value.map(sortValue);
  }
  if (value && typeof value === 'object') {
    return Object.keys(value)
      .sort()
      .reduce((result, key) => {
        result[key] = sortValue(value[key]);
        return result;
      }, {});
  }
  return value;
}

function stableStringify(value) {
  return JSON.stringify(sortValue(value));
}

function countIndent(value) {
  const match = String(value).match(/^ */);
  return match ? match[0].length : 0;
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function lineNumberForPattern(content, pattern) {
  const match = content.match(pattern);
  if (!match || typeof match.index !== 'number') return null;
  return content.slice(0, match.index).split('\n').length;
}

function getPathValue(root, pathSegments) {
  let current = root;
  for (const segment of pathSegments) {
    if (!current || typeof current !== 'object') return undefined;
    current = current[segment];
  }
  return current;
}

function ensureObject(root, pathSegments) {
  let current = root;
  for (const segment of pathSegments) {
    if (!current[segment] || typeof current[segment] !== 'object' || Array.isArray(current[segment])) {
      current[segment] = {};
    }
    current = current[segment];
  }
  return current;
}

function pruneEmptyObjects(node) {
  if (Array.isArray(node)) {
    for (const value of node) {
      pruneEmptyObjects(value);
    }
    return;
  }

  if (!node || typeof node !== 'object') {
    return;
  }

  for (const key of Object.keys(node)) {
    pruneEmptyObjects(node[key]);
    const value = node[key];
    if (
      value &&
      typeof value === 'object' &&
      !Array.isArray(value) &&
      Object.keys(value).length === 0
    ) {
      delete node[key];
    }
  }
}

function lintAnonymousAnchors(filePath, content) {
  const violations = [];
  const pattern = /^(\s*)([^#\n]+?):\s+([&*]id\d+)\s*$/gm;
  let match;

  while ((match = pattern.exec(content)) !== null) {
    violations.push({
      ruleId: 'STYLE001',
      file: repoRelative(filePath),
      line: content.slice(0, match.index).split('\n').length,
      message: `${RULES.STYLE001} Found ${match[3]}.`,
      fixable: true,
    });
  }

  return violations;
}

function lintLegacyItemsAlias(filePath, content) {
  const violations = [];
  const pattern = /^(\s*(?:-\s+)?)items:(\s.*)?$/gm;
  let match;

  while ((match = pattern.exec(content)) !== null) {
    violations.push({
      ruleId: 'STYLE005',
      file: repoRelative(filePath),
      line: content.slice(0, match.index).split('\n').length,
      message: `${RULES.STYLE005} Replace items: with group:.`,
      fixable: true,
    });
  }

  return violations;
}

function lintEmptyStyleVersion(filePath, content) {
  const violations = [];
  const pattern = /^version:\s*(['"]{2})\s*$/gm;
  let match;

  while ((match = pattern.exec(content)) !== null) {
    violations.push({
      ruleId: 'STYLE007',
      file: repoRelative(filePath),
      line: content.slice(0, match.index).split('\n').length,
      message: `${RULES.STYLE007} Remove the empty version field.`,
      fixable: true,
    });
  }

  return violations;
}

function lintDeprecatedTemplateTerms(filePath, content) {
  const violations = [];
  const lines = content.split('\n');
  const pattern = /^(\s*)(?:-\s*)?term:\s*([^#\s]+)/;

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const match = line.match(pattern);
    if (!match) continue;

    let previous = '';
    for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
      if (lines[cursor].trim() !== '') {
        previous = lines[cursor].trim();
        break;
      }
    }

    if (previous.startsWith('label:')) continue;

    violations.push({
      ruleId: 'STYLE008',
      file: repoRelative(filePath),
      line: index + 1,
      message: `${RULES.STYLE008} Found term: ${match[2]}.`,
      fixable: false,
    });
  }

  return violations;
}

function lintPhraseLikeTermMessages(filePath, content) {
  const violations = [];
  const pattern = /^(\s*)(?:-\s*)?message:\s+term\.([^#\s]+)/gm;
  let match;

  while ((match = pattern.exec(content)) !== null) {
    const messageId = match[2].trim();
    if (ALLOWED_TERM_MESSAGE_IDS.has(messageId)) continue;

    violations.push({
      ruleId: 'STYLE009',
      file: repoRelative(filePath),
      line: content.slice(0, match.index).split('\n').length,
      message: `${RULES.STYLE009} Found message: term.${messageId}.`,
      fixable: false,
    });
  }

  return violations;
}

const AFFIX_STRIP_PATTERN = /^[\s,.:;()"'‘’“”]+|[\s,.:;()"'‘’“”]+$/g;
const MF2_PLACEHOLDER_PATTERN = /\{\$[^}]+\}/g;

function normalizeAffixText(rawValue) {
  if (typeof rawValue !== 'string') return '';
  let value = rawValue.trim();

  // A YAML string value may be followed by a trailing `# comment` (e.g. the
  // FIX/TODO annotations this rule exists to catch). Extract only the quoted
  // value itself so the comment text never leaks into the comparison.
  if (value.startsWith('"')) {
    const closeIndex = value.indexOf('"', 1);
    value = closeIndex === -1 ? value.slice(1) : value.slice(1, closeIndex);
  } else if (value.startsWith("'")) {
    const closeIndex = value.indexOf("'", 1);
    value = closeIndex === -1 ? value.slice(1) : value.slice(1, closeIndex);
  } else {
    const commentIndex = value.search(/\s#/);
    if (commentIndex !== -1) value = value.slice(0, commentIndex);
  }

  value = value.replace(MF2_PLACEHOLDER_PATTERN, ' ');
  value = value.replace(AFFIX_STRIP_PATTERN, '');
  return value.trim().toLowerCase();
}

/**
 * Builds the set of normalized strings a hardcoded prefix/suffix may
 * legitimately be replaced by: role verb/short-verb/long/short labels,
 * plain terms, and single-line message pattern bodies. Multi-line MF2
 * `.match` messages (plural/gender dispatch) are intentionally excluded —
 * they have no single literal string to compare against.
 */
function loadLocaleAffixValueSet(localePath = REFERENCE_LOCALE_PATH) {
  const values = new Set();
  let locale;
  try {
    locale = yaml.load(fs.readFileSync(localePath, 'utf8'));
  } catch {
    return values;
  }
  if (!locale || typeof locale !== 'object') return values;

  const addValue = (value) => {
    const normalized = normalizeAffixText(value);
    if (normalized) values.add(normalized);
  };

  const addFormBlock = (formBlock) => {
    if (!formBlock) return;
    if (typeof formBlock === 'string') {
      addValue(formBlock);
      return;
    }
    if (typeof formBlock === 'object') {
      addValue(formBlock.singular);
      addValue(formBlock.plural);
    }
  };

  for (const role of Object.values(locale.roles || {})) {
    if (!role || typeof role !== 'object') continue;
    addFormBlock(role.long);
    addFormBlock(role.short);
    addValue(role.verb);
    addValue(role['verb-short']);
  }

  for (const term of Object.values(locale.terms || {})) {
    if (!term || typeof term !== 'object') continue;
    addFormBlock(term.long);
    addFormBlock(term.short);
  }

  for (const message of Object.values(locale.messages || {})) {
    if (typeof message !== 'string') continue;
    if (message.trim().startsWith('.match')) continue;
    addValue(message);
  }

  return values;
}

function lintHardcodedLocaleProse(filePath, content, localeValueSet) {
  const violations = [];
  if (!localeValueSet || localeValueSet.size === 0) return violations;

  const pattern = /^(\s*)(prefix|suffix):\s*(.+)$/gm;
  let match;

  while ((match = pattern.exec(content)) !== null) {
    const normalized = normalizeAffixText(match[3]);
    if (!normalized || !localeValueSet.has(normalized)) continue;

    violations.push({
      ruleId: 'STYLE010',
      file: repoRelative(filePath),
      line: content.slice(0, match.index).split('\n').length,
      message: `${RULES.STYLE010} Found ${match[2]}: ${match[3].trim()}`,
      fixable: false,
    });
  }

  return violations;
}

function collectSubstituteViolations(filePath, content, data) {
  const violations = [];
  const scopes = [
    { label: 'options', path: ['options'] },
    { label: 'citation.options', path: ['citation', 'options'] },
    { label: 'bibliography.options', path: ['bibliography', 'options'] },
  ];

  for (const scope of scopes) {
    const options = getPathValue(data, scope.path);
    const substitute = options?.substitute;
    if (!substitute || typeof substitute !== 'object' || Array.isArray(substitute)) continue;
    if (!Array.isArray(substitute.template) || substitute.template.length !== 0) continue;
    if (!Object.prototype.hasOwnProperty.call(substitute, 'overrides')) continue;

    const line = lineNumberForPattern(content, new RegExp(`^\\s*${escapeRegExp(scope.label.split('.').slice(-1)[0])}:`, 'm'))
      || lineNumberForPattern(content, /^\s*substitute:\s*$/m);
    violations.push({
      ruleId: 'STYLE002',
      file: repoRelative(filePath),
      line,
      message: `${RULES.STYLE002} Remove substitute.overrides from ${scope.label}.`,
      fixable: true,
      scopePath: scope.path,
    });
  }

  return violations;
}

function collectContributorComponents(node, trail = []) {
  const components = [];

  function visit(value, currentTrail) {
    if (Array.isArray(value)) {
      value.forEach((item, index) => visit(item, currentTrail.concat(index)));
      return;
    }

    if (!value || typeof value !== 'object') {
      return;
    }

    if (typeof value.contributor === 'string') {
      components.push({ component: value, path: currentTrail });
    }

    for (const [key, child] of Object.entries(value)) {
      if (child && typeof child === 'object') {
        visit(child, currentTrail.concat(key));
      }
    }
  }

  visit(node, trail);
  return components;
}

function collectShortenViolations(filePath, content, data) {
  const violations = [];

  for (const sectionName of ['citation', 'bibliography']) {
    const section = data[sectionName];
    if (!section || typeof section !== 'object') continue;

    const components = collectContributorComponents(section, [sectionName]);
    if (components.length < 2) continue;

    const withLocalShorten = components.filter(({ component }) => component.shorten);
    if (withLocalShorten.length < 2) continue;
    if (withLocalShorten.length !== components.length) continue;

    const candidateKey = stableStringify(withLocalShorten[0].component.shorten);
    if (!withLocalShorten.every(({ component }) => stableStringify(component.shorten) === candidateKey)) {
      continue;
    }

    const candidate = withLocalShorten[0].component.shorten;
    const localShorten = getPathValue(section, ['options', 'contributors', 'shorten']);
    const globalShorten = getPathValue(data, ['options', 'contributors', 'shorten']);
    const inherited = localShorten || globalShorten || null;

    if (inherited && stableStringify(inherited) !== candidateKey) {
      continue;
    }

    const line = lineNumberForPattern(content, /^\s+shorten:\s*(?:&id\d+)?\s*$/m);
    violations.push({
      ruleId: 'STYLE003',
      file: repoRelative(filePath),
      line,
      message: `${RULES.STYLE003} ${sectionName} repeats the same shorten block on every contributor component.`,
      fixable: true,
      sectionName,
      candidate,
    });
  }

  return violations;
}

function collectTemplateDuplicateViolations(filePath, content, data) {
  const violations = [];

  for (const sectionName of ['citation', 'bibliography']) {
    const section = data[sectionName];
    if (!section || typeof section !== 'object') continue;
    const baseTemplate = Array.isArray(section.template) ? section.template : null;
    const typeVariants = section['type-variants'];
    if (!baseTemplate || !typeVariants || typeof typeVariants !== 'object') continue;

    const baseKey = stableStringify(baseTemplate);
    for (const [variantName, variantTemplate] of Object.entries(typeVariants)) {
      if (!Array.isArray(variantTemplate)) continue;
      if (stableStringify(variantTemplate) !== baseKey) continue;

      const line = lineNumberForPattern(content, new RegExp(`^\\s*${escapeRegExp(variantName)}:\\s*$`, 'm'));
      violations.push({
        ruleId: 'STYLE004',
        file: repoRelative(filePath),
        line,
        message: `${RULES.STYLE004} ${sectionName}.type-variants.${variantName} duplicates ${sectionName}.template exactly.`,
        fixable: true,
        sectionName,
        variantName,
      });
    }
  }

  return violations;
}

function collectRawPageLabelPrefixViolations(filePath, content, data) {
  const violations = [];

  function visit(node) {
    if (Array.isArray(node)) {
      node.forEach(visit);
      return;
    }
    if (!node || typeof node !== 'object') {
      return;
    }

    const isPageComponent = node.number === 'pages';
    const isPageModify = node.match?.number === 'pages';
    const isPageAdd = node.component?.number === 'pages';
    if (typeof node.prefix === 'string' && /\bpp\.\s*$/.test(node.prefix) && (isPageComponent || isPageModify || isPageAdd)) {
      violations.push({
        ruleId: 'STYLE006',
        file: repoRelative(filePath),
        line: null,
        message: `${RULES.STYLE006} Use label-form: short on number: pages.`,
        fixable: false,
      });
    }

    Object.values(node).forEach(visit);
  }

  visit(data);
  return violations;
}

function lintParsedStyle(filePath, content, data) {
  return [
    ...collectSubstituteViolations(filePath, content, data),
    ...collectShortenViolations(filePath, content, data),
    ...collectTemplateDuplicateViolations(filePath, content, data),
    ...collectRawPageLabelPrefixViolations(filePath, content, data),
  ];
}

function parseAnonymousAnchorBlocks(content) {
  const lines = content.split('\n');
  const anchors = new Map();
  const pattern = /^(\s*(?:-\s+)?[^#\n][^:]*:\s*)&(id\d+)\s*$/;

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const match = line.match(pattern);
    if (!match) continue;

    const baseIndent = countIndent(line);
    const bodyLines = [];
    let cursor = index + 1;

    while (cursor < lines.length) {
      const candidate = lines[cursor];
      if (candidate.trim() === '') {
        bodyLines.push(candidate);
        cursor += 1;
        continue;
      }

      if (countIndent(candidate) <= baseIndent) {
        break;
      }

      bodyLines.push(candidate);
      cursor += 1;
    }

    const nonEmptyIndents = bodyLines
      .filter((candidate) => candidate.trim() !== '')
      .map((candidate) => countIndent(candidate));
    const childIndent = nonEmptyIndents.length > 0
      ? Math.min(...nonEmptyIndents) - baseIndent
      : 2;
    const body = bodyLines.map((candidate) => {
      if (candidate.trim() === '') {
        return { blank: true, delta: 0, text: '' };
      }

      const indent = countIndent(candidate);
      return {
        blank: false,
        delta: Math.max(0, indent - (baseIndent + childIndent)),
        text: candidate.slice(indent),
      };
    });

    anchors.set(match[2], {
      prefix: match[1].replace(/\s+$/, ''),
      childIndent,
      body,
    });
  }

  return anchors;
}

function parseInlineAnonymousAnchors(content) {
  const anchors = new Map();
  const lines = content.split('\n');
  const pattern = /^(\s*(?:-\s+)?[^#\n][^:]*:\s*)&(id\d+)\s+(.+?)\s*$/;

  for (const line of lines) {
    const match = line.match(pattern);
    if (!match) continue;
    anchors.set(match[2], {
      prefix: match[1].replace(/\s+$/, ''),
      value: match[3],
    });
  }

  return anchors;
}

function expandAnonymousAnchorsInText(content) {
  const anchors = parseAnonymousAnchorBlocks(content);
  const inlineAnchors = parseInlineAnonymousAnchors(content);
  if (anchors.size === 0 && inlineAnchors.size === 0) return stripAnonymousAnchorMarkersInText(content);

  const lines = content.split('\n');
  const pattern = /^(\s*(?:-\s+)?[^#\n][^:]*:\s*)([&*])(id\d+)\s*$/;
  const inlinePattern = /^(\s*(?:-\s+)?[^#\n][^:]*:\s*)\*(id\d+)\s*$/;
  const output = [];

  for (const line of lines) {
    const inlineMatch = line.match(inlinePattern);
    if (inlineMatch) {
      const anchor = inlineAnchors.get(inlineMatch[2]);
      if (anchor) {
        const prefix = inlineMatch[1].replace(/\s+$/, '');
        output.push(`${prefix} ${anchor.value}`);
        continue;
      }
    }

    const match = line.match(pattern);
    if (!match) {
      output.push(line);
      continue;
    }

    const prefix = match[1].replace(/\s+$/, '');
    const kind = match[2];
    const anchorId = match[3];
    output.push(prefix);

    if (kind !== '*') {
      continue;
    }

    const anchor = anchors.get(anchorId);
    if (!anchor) {
      continue;
    }

    const aliasIndent = countIndent(line);
    for (const entry of anchor.body) {
      if (entry.blank) {
        output.push('');
      } else {
        output.push(`${' '.repeat(aliasIndent + anchor.childIndent + entry.delta)}${entry.text}`);
      }
    }
  }

  return stripAnonymousAnchorMarkersInText(output.join('\n'));
}

function stripAnonymousAnchorMarkersInText(content) {
  return content
    .replace(
      /^(\s*(?:-\s+)?[^#\n][^:]*:\s*)&id\d+(\s*)$/gm,
      (_match, prefix, suffix = '') => `${prefix.replace(/\s+$/, '')}${suffix}`
    )
    .replace(
      /^(\s*(?:-\s+)?[^#\n][^:]*:\s*)&id\d+(\s+.+?)$/gm,
      (_match, prefix, suffix = '') => `${prefix.replace(/\s+$/, '')}${suffix}`
    );
}

function convertItemsAliasInText(content) {
  return content.replace(/^(\s*(?:-\s+)?)items:(\s.*)?$/gm, (_match, prefix, suffix = '') => {
    return `${prefix}group:${suffix}`;
  });
}

function removeEmptyVersionInText(content) {
  return content.replace(/^version:\s*(['"]{2})\s*\n?/gm, '');
}

function removeEmptyVersion(data) {
  if (!data || typeof data !== 'object' || data.version !== '') {
    return false;
  }

  delete data.version;
  return true;
}

function removeInertSubstituteOverrides(data) {
  let changed = false;
  const scopes = [
    ['options'],
    ['citation', 'options'],
    ['bibliography', 'options'],
  ];

  for (const scopePath of scopes) {
    const options = getPathValue(data, scopePath);
    const substitute = options?.substitute;
    if (!substitute || typeof substitute !== 'object' || Array.isArray(substitute)) continue;
    if (!Array.isArray(substitute.template) || substitute.template.length !== 0) continue;
    if (!Object.prototype.hasOwnProperty.call(substitute, 'overrides')) continue;

    delete substitute.overrides;
    changed = true;
  }

  return changed;
}

function hoistDuplicateShorten(data) {
  let changed = false;

  for (const sectionName of ['citation', 'bibliography']) {
    const section = data[sectionName];
    if (!section || typeof section !== 'object') continue;

    const components = collectContributorComponents(section, [sectionName]);
    if (components.length < 2) continue;

    const withLocalShorten = components.filter(({ component }) => component.shorten);
    if (withLocalShorten.length < 2) continue;
    if (withLocalShorten.length !== components.length) continue;

    const candidateKey = stableStringify(withLocalShorten[0].component.shorten);
    if (!withLocalShorten.every(({ component }) => stableStringify(component.shorten) === candidateKey)) {
      continue;
    }

    const sectionShorten = getPathValue(section, ['options', 'contributors', 'shorten']);
    const globalShorten = getPathValue(data, ['options', 'contributors', 'shorten']);
    const candidate = withLocalShorten[0].component.shorten;

    if (sectionShorten && stableStringify(sectionShorten) !== candidateKey) {
      continue;
    }
    if (!sectionShorten && globalShorten && stableStringify(globalShorten) !== candidateKey) {
      continue;
    }

    if (!sectionShorten && (!globalShorten || stableStringify(globalShorten) !== candidateKey)) {
      const contributors = ensureObject(section, ['options', 'contributors']);
      contributors.shorten = candidate;
      changed = true;
    }

    for (const { component } of withLocalShorten) {
      if (component.shorten) {
        delete component.shorten;
        changed = true;
      }
    }
  }

  return changed;
}

function removeDuplicateTypeVariants(data) {
  let changed = false;

  for (const sectionName of ['citation', 'bibliography']) {
    const section = data[sectionName];
    if (!section || typeof section !== 'object') continue;
    const baseTemplate = Array.isArray(section.template) ? section.template : null;
    const typeVariants = section['type-variants'];
    if (!baseTemplate || !typeVariants || typeof typeVariants !== 'object') continue;

    const baseKey = stableStringify(baseTemplate);
    for (const [variantName, variantTemplate] of Object.entries(typeVariants)) {
      if (!Array.isArray(variantTemplate)) continue;
      if (stableStringify(variantTemplate) !== baseKey) continue;
      delete typeVariants[variantName];
      changed = true;
    }

    if (Object.keys(typeVariants).length === 0) {
      delete section['type-variants'];
    }
  }

  return changed;
}

function applyFixes(data) {
  let changed = false;
  changed = removeEmptyVersion(data) || changed;
  changed = removeInertSubstituteOverrides(data) || changed;
  changed = hoistDuplicateShorten(data) || changed;
  changed = removeDuplicateTypeVariants(data) || changed;
  pruneEmptyObjects(data);
  return changed;
}

let cachedLocaleAffixValueSet = null;
function getLocaleAffixValueSet() {
  if (!cachedLocaleAffixValueSet) {
    cachedLocaleAffixValueSet = loadLocaleAffixValueSet();
  }
  return cachedLocaleAffixValueSet;
}

function lintStyleFile(filePath, options = {}) {
  const content = fs.readFileSync(filePath, 'utf8');
  const localeValueSet = getLocaleAffixValueSet();
  const violations = lintAnonymousAnchors(filePath, content);
  violations.push(...lintLegacyItemsAlias(filePath, content));
  violations.push(...lintEmptyStyleVersion(filePath, content));
  violations.push(...lintDeprecatedTemplateTerms(filePath, content));
  violations.push(...lintPhraseLikeTermMessages(filePath, content));
  violations.push(...lintHardcodedLocaleProse(filePath, content, localeValueSet));
  let workingContent = content;
  let fixed = false;

  if (options.fix && violations.some((violation) => violation.ruleId === 'STYLE007')) {
    const removed = removeEmptyVersionInText(workingContent);
    if (removed !== workingContent) {
      workingContent = removed;
      fixed = true;
    }
  }

  if (options.fix && violations.some((violation) => violation.ruleId === 'STYLE005')) {
    const converted = convertItemsAliasInText(workingContent);
    if (converted !== workingContent) {
      workingContent = converted;
      fixed = true;
    }
  }

  if (options.fix && violations.some((violation) => violation.ruleId === 'STYLE001')) {
    const expanded = expandAnonymousAnchorsInText(workingContent);
    if (expanded !== workingContent) {
      workingContent = expanded;
      fixed = true;
    }
  }

  let parsed = null;
  try {
    parsed = yaml.load(workingContent);
  } catch (error) {
    violations.push({
      ruleId: 'STYLE000',
      file: repoRelative(filePath),
      line: null,
      message: `Style YAML parse failed: ${error.message}`,
      fixable: false,
    });
  }

  if (parsed && typeof parsed === 'object') {
    violations.push(...lintParsedStyle(filePath, workingContent, parsed));
  }

  if (options.fix && parsed && typeof parsed === 'object') {
    const fixableRuleIds = new Set(['STYLE002', 'STYLE003', 'STYLE004', 'STYLE007']);
    const hasFixableViolations = violations.some((violation) => fixableRuleIds.has(violation.ruleId));
    const changedStructure = applyFixes(parsed);
    if (hasFixableViolations && changedStructure) {
      const output = `${yaml.dump(parsed, {
        noRefs: true,
        lineWidth: -1,
        sortKeys: false,
      })}`.replace(/\n?$/, '\n');
      workingContent = output;
      fixed = true;
    }
  }

  if (options.fix && fixed && workingContent !== content) {
    fs.writeFileSync(filePath, workingContent, 'utf8');
  }

  const refreshedContent = fixed ? workingContent : content;
  const refreshedViolations = [
    ...lintAnonymousAnchors(filePath, refreshedContent),
    ...lintLegacyItemsAlias(filePath, refreshedContent),
    ...lintEmptyStyleVersion(filePath, refreshedContent),
    ...lintDeprecatedTemplateTerms(filePath, refreshedContent),
    ...lintPhraseLikeTermMessages(filePath, refreshedContent),
    ...lintHardcodedLocaleProse(filePath, refreshedContent, localeValueSet),
  ];
  if (parsed && typeof parsed === 'object' && !fixed) {
    refreshedViolations.push(...lintParsedStyle(filePath, refreshedContent, parsed));
  } else if (fixed) {
    const refreshedParsed = yaml.load(refreshedContent);
    refreshedViolations.push(...lintParsedStyle(filePath, refreshedContent, refreshedParsed));
  }

  return {
    file: repoRelative(filePath),
    fixed,
    violations: refreshedViolations,
  };
}

function summarize(results) {
  const filesScanned = results.length;
  const filesWithViolations = results.filter((result) => result.violations.length > 0).length;
  const fixedFiles = results.filter((result) => result.fixed).length;
  const violations = results.flatMap((result) => result.violations);

  return {
    filesScanned,
    filesWithViolations,
    fixedFiles,
    violations,
  };
}

function printTextReport(summary) {
  for (const violation of summary.violations) {
    const location = violation.line ? `${violation.file}:${violation.line}` : violation.file;
    console.warn(`WARN ${violation.ruleId} ${location} ${violation.message}`);
  }

  if (summary.fixedFiles > 0) {
    console.log(`Auto-fixed ${summary.fixedFiles} style file(s).`);
  }

  console.log(
    `Scanned ${summary.filesScanned} style file(s); ${summary.filesWithViolations} file(s) with violations.`
  );
}

function main() {
  const args = parseArgs();
  const filePaths = listStyleFiles(args.filePaths);
  const results = filePaths.map((filePath) => lintStyleFile(filePath, { fix: args.fix }));
  const summary = summarize(results);

  if (args.json) {
    console.log(JSON.stringify(summary, null, 2));
  } else {
    printTextReport(summary);
  }

  if (
    summary.violations.some((violation) => FATAL_RULE_IDS.has(violation.ruleId)) ||
    (args.strict && summary.violations.length > 0)
  ) {
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  RULES,
  applyFixes,
  collectShortenViolations,
  collectSubstituteViolations,
  collectTemplateDuplicateViolations,
  collectRawPageLabelPrefixViolations,
  convertItemsAliasInText,
  expandAnonymousAnchorsInText,
  lintEmptyStyleVersion,
  lintDeprecatedTemplateTerms,
  lintPhraseLikeTermMessages,
  lintHardcodedLocaleProse,
  lintAnonymousAnchors,
  lintLegacyItemsAlias,
  lintParsedStyle,
  lintStyleFile,
  loadLocaleAffixValueSet,
  normalizeAffixText,
  parseAnonymousAnchorBlocks,
  parseInlineAnonymousAnchors,
  removeEmptyVersionInText,
  stripAnonymousAnchorMarkersInText,
  summarize,
};
