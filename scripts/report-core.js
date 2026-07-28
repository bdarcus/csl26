#!/usr/bin/env node
/**
 * Core Styles Compatibility Report Generator
 *
 * Generates a JSON report of compatibility metrics for core styles in styles/
 * and optionally produces an HTML dashboard.
 *
 * Usage:
 *   node report-core.js                                      # Output JSON to stdout
 *   node report-core.js --write-html                         # Write HTML to docs/compat.html
 *   node report-core.js --output-html /path/to/output.html   # Write HTML to custom path
 *   node report-core.js --style chicago-author-date          # Output one official style report
 *   node report-core.js --styles-dir /path/to/csl            # Override CSL directory
 *   node report-core.js --styles chicago-author-date,apa-7th # Limit report to selected styles
 */

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const { execFileSync, spawn } = require('child_process');
const yaml = require('js-yaml');
const {
  loadFixtureSufficiency,
  loadVerificationPolicy,
  resolveScopeAuthority,
  resolveFixtureSufficiency,
  resolveVerificationPolicy,
  resolveStyleData,
} = require('./lib/verification-policy');
const {
  DEFAULT_CITATIONS_FIXTURE,
  DEFAULT_REFS_FIXTURE,
  FIXTURE_SET_CITATIONS,
  FIXTURE_SET_REFS,
  NOTE_CITATIONS_FIXTURE,
  getAdditionalFixtureSetNames,
  getEffectiveVerificationScopes,
  inferLegacySourceName,
} = require('./lib/style-verification');
const {
  getLineagePresentation,
  loadReportProvenance,
} = require('./lib/report-metadata');
const {
  auditNoteStyle,
  discoverNoteStyles,
} = require('./lib/note-position-audit');
const {
  buildFamilyAggregates,
  buildInheritanceIndex,
} = require('./lib/report-inheritance');
const {
  compareText,
  normalizeText,
  textSimilarity,
  parseComponents,
  compareComponents,
} = require('./oracle-utils');
const { maybeDatasetErrorForFile } = require('./lib/dataset-guard');

const KNOWN_DEPENDENTS = {
  'apa-7th': 783,
  'elsevier-with-titles': 672,
  'elsevier-harvard': 665,
  'elsevier-vancouver': 502,
  'springer-vancouver-brackets': 472,
  'springer-basic-author-date': 460,
  'springer-basic-brackets': 352,
  'springer-socpsych-author-date': 317,
  'american-medical-association': 293,
  'taylor-and-francis-chicago-author-date': 234,
  'chicago-author-date': 8,
  'springer-mathphys-brackets': 201,
  'multidisciplinary-digital-publishing-institute': 180,
  'ieee': 176,
  'nlm-citation-sequence-superscript': 121,
  'nlm-citation-sequence': 116,
  'karger-journals': 85,
  'institute-of-physics-numeric': 82,
  'thieme-german': 74,
  'mary-ann-liebert-vancouver': 72,
  'biomed-central': 66,
  'chicago-notes': 5,
};

// chicago-notes-18th-script has no CSL/citeproc counterpart (it is a derivative
// embedded style; its behavior is covered by engine multilingual tests).
const SKIPPED_STYLES = [
  'alpha',
  'chicago-18-base',
  'chicago-notes-18th-script',
  'iso690-author-date',
  'iso690-numeric',
];
const PROJECT_ROOT = path.dirname(__dirname);
const DEFAULT_REPORT_CACHE_DIR = path.join(PROJECT_ROOT, '.oracle-cache', 'report-core');
const DEFAULT_PARALLELISM = 4;
const DEFAULT_PROCESS_TIMEOUT_MS = 240000;
const CSL_SNAPSHOT_DIR = path.join(PROJECT_ROOT, 'tests', 'snapshots', 'csl');
const BIBLATEX_SNAPSHOT_DIR = path.join(PROJECT_ROOT, 'tests', 'snapshots', 'biblatex');
const COMPOUND_SNAPSHOT_DIR = path.join(PROJECT_ROOT, 'tests', 'snapshots', 'compound');
const REPORT_CACHE_VERSION = 9;

const TOTAL_DEPENDENTS = 7987;
const CORE_FALLBACK_TYPES = [
  'article-journal',
  'book',
  'chapter',
  'report',
  'thesis',
  'paper-conference',
  'webpage',
];
const BENCHMARK_LABELS = {
  'citeproc-js': 'citeproc-js',
  'citeproc-js-live': 'citeproc-js',
  'citum-baseline': 'Citum baseline',
  biblatex: 'biblatex',
  documentary: 'documentary',
};

const BENCHMARK_RUNNERS = {
  CITEPROC_ORACLE: 'citeproc-oracle',
  NATIVE_SMOKE: 'native-smoke',
};

function consumeFlagValue(args, index, flag) {
  const value = args[index + 1];
  if (typeof value !== 'string' || value.trim() === '') {
    throw new Error(`Missing value for ${flag}`);
  }
  return {
    nextIndex: index + 1,
    value,
  };
}

/**
 * Parse command-line arguments
 */
function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    writeHtml: false,
    outputHtml: null,
    styleName: null,
    styleFile: null,
    stylesDir: null,
    styles: null,
    parallelism: DEFAULT_PARALLELISM,
    allowLiveFallback: false,
    timings: false,
    cacheDir: DEFAULT_REPORT_CACHE_DIR,
    citumBin: process.env.CITUM_BIN || null,
    caseSensitive: true,
    allFeatures: false,
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--write-html') {
      options.writeHtml = true;
    } else if (args[i] === '--output-html') {
      const consumed = consumeFlagValue(args, i, '--output-html');
      i = consumed.nextIndex;
      options.outputHtml = consumed.value;
      options.writeHtml = true;
    } else if (args[i] === '--style') {
      const consumed = consumeFlagValue(args, i, '--style');
      i = consumed.nextIndex;
      options.styleName = consumed.value.trim();
    } else if (args[i] === '--style-file') {
      const consumed = consumeFlagValue(args, i, '--style-file');
      i = consumed.nextIndex;
      options.styleFile = path.resolve(consumed.value);
    } else if (args[i] === '--styles-dir') {
      const consumed = consumeFlagValue(args, i, '--styles-dir');
      i = consumed.nextIndex;
      options.stylesDir = consumed.value;
    } else if (args[i] === '--styles') {
      const consumed = consumeFlagValue(args, i, '--styles');
      i = consumed.nextIndex;
      options.styles = consumed.value
        .split(',')
        .map((style) => style.trim())
        .filter(Boolean);
      if (options.styles.length === 0) {
        throw new Error('Missing value for --styles');
      }
    } else if (args[i] === '--parallelism') {
      options.parallelism = Math.max(1, parseInt(args[++i], 10) || DEFAULT_PARALLELISM);
    } else if (args[i] === '--allow-live-fallback' || args[i] === '--refresh-missing') {
      options.allowLiveFallback = true;
    } else if (args[i] === '--timings') {
      options.timings = true;
    } else if (args[i] === '--cache-dir') {
      options.cacheDir = path.resolve(args[++i]);
    } else if (args[i] === '--citum-bin') {
      options.citumBin = path.resolve(args[++i]);
    } else if (args[i] === '--case-sensitive') {
      options.caseSensitive = true;
    } else if (args[i] === '--case-insensitive') {
      options.caseSensitive = false;
    } else if (args[i] === '--all-features') {
      options.allFeatures = true;
    }
  }

  if (options.styleName && options.styles?.length) {
    throw new Error('Flags --style and --styles are mutually exclusive');
  }
  if (options.styleFile && !options.styleName) {
    throw new Error('Flag --style-file requires --style');
  }

  return options;
}

/**
 * Get git short commit hash or 'unknown' on error
 */
function getGitCommit() {
  try {
    return execFileSync('git', ['rev-parse', '--short', 'HEAD'], {
      cwd: PROJECT_ROOT,
      encoding: 'utf8',
    }).trim();
  } catch {
    return 'unknown';
  }
}

/**
 * Get ISO timestamp
 */
function getTimestamp() {
  return new Date().toISOString();
}

/**
 * Find styles directory
 */
function getStylesDir(optionsDir) {
  if (optionsDir) return optionsDir;

  const defaultDir = path.join(PROJECT_ROOT, 'styles-legacy');

  if (fs.existsSync(defaultDir)) {
    return defaultDir;
  }

  throw new Error(`Styles directory not found. Use --styles-dir to specify path.`);
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function hashContent(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function cloneJson(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}

function hashFile(filePath) {
  return hashContent(fs.readFileSync(filePath));
}

// oracle.js's actual output also depends on these two files (registered
// divergence detectors and the policy that enables them), but neither is
// `oracle.js` itself, so a cache key hashing only `liveScript` silently
// serves stale pre-fix results after editing either — see the div-011
// investigation in docs/architecture/audits/2026-07-22_GBT_DATE_ANNOTATION_FIDELITY.md.
function oracleDivergenceDepsHash() {
  const oracleDivergencesPath = path.join(__dirname, 'lib', 'oracle-divergences.js');
  const verificationPolicyPath = path.join(__dirname, 'report-data', 'verification-policy.yaml');
  return hashContent(hashFile(oracleDivergencesPath) + hashFile(verificationPolicyPath));
}

function equivalentText(expected, actual, options = {}) {
  return compareText(expected, actual, options).match;
}

function fixtureHash(refsFixture, citationsFixture) {
  const hash = crypto.createHash('sha256');
  hash.update(fs.readFileSync(refsFixture));
  hash.update(fs.readFileSync(citationsFixture));
  return hash.digest('hex').slice(0, 16);
}

function resolveCitumBinary(explicitPath = null, allFeatures = false) {
  const cargoTargetDir = process.env.CARGO_TARGET_DIR
    ? path.resolve(process.env.CARGO_TARGET_DIR)
    : null;
  const candidates = [
    explicitPath,
    process.env.CITUM_BIN,
    cargoTargetDir ? path.join(cargoTargetDir, 'debug', 'citum') : null,
    cargoTargetDir ? path.join(cargoTargetDir, 'release', 'citum') : null,
    path.join(PROJECT_ROOT, 'target', 'debug', 'citum'),
    path.join(PROJECT_ROOT, 'target', 'release', 'citum'),
  ].filter(Boolean);

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return path.resolve(candidate);
    }
  }

  const buildArgs = ['build', '-q', '--bin', 'citum'];
  if (allFeatures) {
    buildArgs.push('--all-features');
  }

  execFileSync('cargo', buildArgs, {
    cwd: PROJECT_ROOT,
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
  });

  const builtBinary = cargoTargetDir
    ? path.join(cargoTargetDir, 'debug', 'citum')
    : path.join(PROJECT_ROOT, 'target', 'debug', 'citum');
  if (!fs.existsSync(builtBinary)) {
    throw new Error(`Expected Citum binary after build: ${builtBinary}`);
  }
  return builtBinary;
}

function createTimerBucket() {
  return { count: 0, totalMs: 0, cacheHits: 0 };
}

function createReportRuntime(options = {}) {
  const cacheDir = path.resolve(options.cacheDir || DEFAULT_REPORT_CACHE_DIR);
  ensureDir(cacheDir);

  return {
    allowLiveFallback: Boolean(options.allowLiveFallback),
    allFeatures: Boolean(options.allFeatures),
    cacheDir,
    caseSensitive: options.caseSensitive !== false,
    timings: new Map(),
    stylesDir: options.stylesDir ? path.resolve(options.stylesDir) : null,
    citumBin: resolveCitumBinary(options.citumBin, options.allFeatures),
    recordTiming(kind, durationMs, cacheHit = false) {
      const bucket = this.timings.get(kind) || createTimerBucket();
      bucket.count += 1;
      bucket.totalMs += durationMs;
      if (cacheHit) bucket.cacheHits += 1;
      this.timings.set(kind, bucket);
    },
  };
}

function spawnProcess(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd || PROJECT_ROOT,
      env: options.env || process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    let settled = false;
    const timeoutMs = options.timeout ?? DEFAULT_PROCESS_TIMEOUT_MS;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      child.kill('SIGKILL');
      reject(new Error(`Command timed out: ${command} ${args.join(' ')}`));
    }, timeoutMs);

    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('error', (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(error);
    });
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ code: code ?? 0, stdout, stderr });
    });
  });
}

async function runCachedJsonJob(runtime, config) {
  const keyJson = JSON.stringify({
    version: REPORT_CACHE_VERSION,
    ...config.cacheKey,
  });
  const cacheFile = path.join(runtime.cacheDir, `${hashContent(keyJson)}.json`);
  if (fs.existsSync(cacheFile)) {
    const started = Date.now();
    try {
      const cached = JSON.parse(fs.readFileSync(cacheFile, 'utf8'));
      runtime.recordTiming(config.kind, Date.now() - started, true);
      return cached;
    } catch {
      // Parallel report runs can observe partially written cache files.
      // Remove the corrupt cache entry and recompute it below.
      fs.rmSync(cacheFile, { force: true });
    }
  }

  const started = Date.now();
  const result = await config.compute();
  runtime.recordTiming(config.kind, Date.now() - started, false);
  const tempCacheFile = `${cacheFile}.${process.pid}.${Date.now()}.tmp`;
  fs.writeFileSync(tempCacheFile, JSON.stringify(result));
  fs.renameSync(tempCacheFile, cacheFile);
  return result;
}

async function mapWithConcurrency(items, limit, worker) {
  const results = new Array(items.length);
  let cursor = 0;

  async function runNext() {
    while (true) {
      const index = cursor;
      cursor += 1;
      if (index >= items.length) return;
      results[index] = await worker(items[index], index);
    }
  }

  const workers = Array.from({ length: Math.min(limit, items.length || 1) }, () => runNext());
  await Promise.all(workers);
  return results;
}

function serializeTimingSummary(runtime) {
  return Object.fromEntries(
    [...runtime.timings.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([kind, stats]) => [
        kind,
        {
          count: stats.count,
          cacheHits: stats.cacheHits,
          totalMs: parseFloat(stats.totalMs.toFixed(1)),
          averageMs: stats.count > 0 ? parseFloat((stats.totalMs / stats.count).toFixed(1)) : 0,
        },
      ])
  );
}

function inferStyleUpstream(styleData) {
  const sourceId = styleData?.info?.source?.['csl-id'];
  if (typeof sourceId !== 'string' || sourceId.trim().length === 0) {
    return 'unknown';
  }

  const normalized = sourceId.toLowerCase();
  if (normalized.includes('ctan.org/pkg/biblatex') || normalized.includes('biblatex')) {
    return 'biblatex';
  }
  if (normalized.includes('zotero.org/styles')) {
    return 'csl';
  }
  return 'unknown';
}

function inferLineageKey(styleData, hasLegacySource) {
  const upstream = inferStyleUpstream(styleData);
  const adaptedBy = styleData?.info?.source?.['adapted-by'];

  if (upstream === 'biblatex') return 'biblatex-derived';
  if (upstream === 'csl') return 'csl-derived';
  if (adaptedBy === 'citum-migrate') return 'csl-derived';
  if (hasLegacySource) return 'csl-derived';
  return 'citum-native';
}

function normalizeBenchmarkSource(authority) {
  if (!authority) return 'unknown';
  return authority === 'citeproc-js-live' ? 'citeproc-js' : authority;
}

function formatAuthorityLabel(authority, authorityId = null) {
  const normalized = normalizeBenchmarkSource(authority);
  if ((normalized === 'biblatex' || normalized === 'documentary') && authorityId) {
    return `${BENCHMARK_LABELS[normalized] || normalized}: ${authorityId}`;
  }
  return BENCHMARK_LABELS[normalized] || String(normalized);
}

function computeImpactPct(cslReach) {
  return cslReach != null
    ? (cslReach / TOTAL_DEPENDENTS * 100).toFixed(2)
    : null;
}

function discoverCoreStyles(provenanceConfig = loadReportProvenance()) {
  const stylesRoot = path.join(path.dirname(__dirname), 'styles');
  const embeddedRoot = path.join(stylesRoot, 'embedded');
  if (!fs.existsSync(stylesRoot)) {
    throw new Error(`Core styles directory not found: ${stylesRoot}`);
  }

  const findStyles = (dir) => {
    if (!fs.existsSync(dir)) return [];
    return fs.readdirSync(dir)
      .filter((entry) => entry.endsWith('.yaml'))
      .map((filename) => ({
        filename,
        stylePath: path.join(dir, filename),
        name: path.basename(filename, '.yaml'),
      }))
      // Hidden family roots (-core publisher bases, -base embedded family
      // roots like gb-t-7714-2025-base) are inheritance targets, not public
      // styles; they have no CSL source and are excluded from the report.
      .filter((style) => !style.name.endsWith('-core') && !style.name.endsWith('-base'));
  };

  const allStyles = [
    ...findStyles(stylesRoot),
    ...findStyles(embeddedRoot),
  ].filter((style) => !SKIPPED_STYLES.includes(style.name))
    .sort((a, b) => a.name.localeCompare(b.name));

  if (allStyles.length === 0) {
    throw new Error(`No style YAML files found in: ${stylesRoot}`);
  }

  const inheritanceIndex = buildInheritanceIndex({
    styleRoots: [stylesRoot, embeddedRoot],
  });

  return allStyles.map(({ stylePath, name }) => {
    let rawStyleData = null;
    let styleData = null;

    try {
      rawStyleData = yaml.load(fs.readFileSync(stylePath, 'utf8'));
      styleData = resolveStyleData(cloneJson(rawStyleData));
    } catch {
      rawStyleData = null;
      styleData = null;
    }
    const sourceName = inferLegacySourceName(name, rawStyleData);

    const legacySourcePath = path.join(path.dirname(__dirname), 'styles-legacy', `${sourceName}.csl`);
    const hasLegacySource = fs.existsSync(legacySourcePath);
    const lineageKey = provenanceConfig.styles?.[name]?.lineage
      || inferLineageKey(styleData, hasLegacySource);
    const origin = getLineagePresentation(lineageKey, provenanceConfig);
    const cslReach = KNOWN_DEPENDENTS[name] ?? KNOWN_DEPENDENTS[sourceName] ?? null;
    const inheritanceMetadata = inheritanceIndex.records.get(name) || {
      inheritance: {
        directParent: null,
        chain: [name],
        familyRoot: name,
        complete: true,
        missingParent: null,
        cycle: null,
        implementationForm: 'standalone',
      },
      registry: { kind: null, aliases: [], aliasCount: 0 },
      measurementEvidence: { behavioralBand: null, derivability: null },
    };

    return {
      name,
      sourceName,
      stylePath,
      cslReach,
      dependents: cslReach,
      format: inferStyleFormat(styleData),
      hasBibliography: hasBibliographyTemplate(styleData),
      originKey: origin.key,
      originLabel: origin.label,
      originSortRank: origin.sortRank,
      ...inheritanceMetadata,
      measurementEvidenceSources: inheritanceIndex.evidenceSources,
    };
  });
}

function resolveSelectedStyles(coreStyles, selectedNames = null) {
  if (!Array.isArray(selectedNames) || selectedNames.length === 0) {
    return coreStyles;
  }

  const requested = new Set(selectedNames);
  const known = new Set(coreStyles.map((style) => style.name));
  const unknown = [...requested].filter((name) => !known.has(name)).sort((a, b) => a.localeCompare(b));
  if (unknown.length > 0) {
    throw new Error(`Unknown style name(s) for --styles: ${unknown.join(', ')}`);
  }

  return coreStyles.filter((style) => requested.has(style.name));
}

function buildNoteStyleLookup() {
  return new Map(discoverNoteStyles().map((style) => [style.name, style]));
}

function selectPrimaryComparator(styleSpec, verificationPolicy) {
  return verificationPolicy.authority;
}

function inferStyleFormat(styleData) {
  const processing = styleData?.options?.processing;
  if (typeof processing === 'string') {
    return processing;
  }
  if (processing && typeof processing === 'object') {
    if (Object.prototype.hasOwnProperty.call(processing, 'note')) {
      return 'note';
    }
    if (Object.prototype.hasOwnProperty.call(processing, 'author-date')) {
      return 'author-date';
    }
    if (
      Object.prototype.hasOwnProperty.call(processing, 'label') ||
      Object.prototype.hasOwnProperty.call(processing, 'numeric')
    ) {
      return 'numeric';
    }
    // Processing objects with sort/group/disambiguate keys are author-date
    if (
      Object.prototype.hasOwnProperty.call(processing, 'sort') ||
      Object.prototype.hasOwnProperty.call(processing, 'group') ||
      Object.prototype.hasOwnProperty.call(processing, 'disambiguate')
    ) {
      return 'author-date';
    }
  }

  const citation = styleData?.citation || {};
  const candidateTemplates = [
    citation.template,
    citation.integral?.template,
    citation['non-integral']?.template,
  ].filter(Array.isArray);
  const usesCitationNumbers = candidateTemplates.some((template) =>
    flattenTemplateComponents(template).some((component) => Boolean(component?.number))
  );

  if (usesCitationNumbers) return 'numeric';

  // Styles with non-integral/integral citation sections are always author-date
  if (citation['non-integral'] || citation['integral']) return 'author-date';

  // Styles whose citation template uses contributor + date (but no number) are author-date
  const allTemplates = candidateTemplates.concat(
    [citation.template].filter(Array.isArray)
  );
  const flat = allTemplates.flatMap((t) => flattenTemplateComponents(t));
  const hasContributor = flat.some((c) => Boolean(c?.contributor));
  const hasDate = flat.some((c) => Boolean(c?.date));
  if (hasContributor && hasDate) return 'author-date';

  return 'unknown';
}

function hasBibliographyTemplate(styleData) {
  const bibliography = styleData?.bibliography;
  if (!bibliography || typeof bibliography !== 'object') {
    return false;
  }

  const hasTemplate = Array.isArray(bibliography.template) && bibliography.template.length > 0;
  const hasPreset = typeof bibliography['extends'] === 'string' && bibliography['extends'].trim().length > 0;
  const typeTemplates = bibliography['type-templates'];
  const hasTypeTemplates = Boolean(typeTemplates && Object.keys(typeTemplates).length > 0);
  const typeVariants = bibliography['type-variants'];
  const hasTypeVariants = Boolean(typeVariants && Object.keys(typeVariants).length > 0);
  return hasTemplate || hasPreset || hasTypeTemplates || hasTypeVariants;
}

function buildEmptyOracleResult(overrides = {}) {
  return {
    citations: { passed: 0, total: 0, entries: [] },
    bibliography: { passed: 0, total: 0, entries: [] },
    adjusted: {
      citations: { passed: 0, total: 0, entries: [] },
      bibliography: { passed: 0, total: 0, entries: [] },
      divergenceSummary: {},
    },
    citationsByType: {},
    componentSummary: {},
    ...overrides,
  };
}

function cloneOracleSection(section = {}) {
  return {
    passed: section.passed || 0,
    total: section.total || 0,
    entries: [...(section.entries || [])],
  };
}

function cloneOracleResult(oracleResult = {}) {
  return {
    citations: cloneOracleSection(oracleResult.citations),
    bibliography: cloneOracleSection(oracleResult.bibliography),
    adjusted: {
      citations: cloneOracleSection(oracleResult.adjusted?.citations),
      bibliography: cloneOracleSection(oracleResult.adjusted?.bibliography),
      divergenceSummary: { ...(oracleResult.adjusted?.divergenceSummary || {}) },
    },
    citationsByType: { ...(oracleResult.citationsByType || {}) },
    componentSummary: { ...(oracleResult.componentSummary || {}) },
    oracleSource: oracleResult.oracleSource || null,
    error: oracleResult.error || null,
  };
}

function getEffectiveOracleSection(oracleResult, sectionName) {
  const adjustedSection = oracleResult?.adjusted?.[sectionName];
  if (adjustedSection) {
    const adjustedTotal = Number(adjustedSection.total || 0);
    const adjustedPassed = Number(adjustedSection.passed || 0);
    const hasDivergences = Object.keys(oracleResult?.adjusted?.divergenceSummary || {}).length > 0;
    if (adjustedTotal > 0 || adjustedPassed > 0 || hasDivergences) {
      return adjustedSection;
    }
  }
  return oracleResult?.[sectionName] || { passed: 0, total: 0, entries: [] };
}

function countCaseMismatches(section) {
  return (section?.entries || []).filter((entry) => entry?.caseMismatch === true).length;
}

function collectCaseMismatchSummary(oracleResult) {
  const citations = getEffectiveOracleSection(oracleResult, 'citations');
  const bibliography = getEffectiveOracleSection(oracleResult, 'bibliography');
  const citationCount = countCaseMismatches(citations);
  const bibliographyCount = countCaseMismatches(bibliography);
  return {
    citations: citationCount,
    bibliography: bibliographyCount,
    total: citationCount + bibliographyCount,
  };
}

function summarizeExactParity(oracleResult, hasBibliography = true) {
  const sections = [oracleResult?.citations];
  if (hasBibliography) sections.push(oracleResult?.bibliography);
  let passed = 0;
  let total = 0;
  let notComparable = 0;

  for (const section of sections) {
    const entries = section?.entries || [];
    for (const entry of entries) {
      if (entry?.exactParityEligible === false || entry?.exactMatch === null) {
        notComparable++;
        continue;
      }
      if (typeof entry?.exactMatch === 'boolean') {
        total++;
        if (entry.exactMatch) passed++;
        continue;
      }
      const { benchmark, citum } = getComparisonEntryTexts(entry);
      if (!benchmark || !citum) {
        notComparable++;
        continue;
      }
      total++;
      if (compareText(benchmark, citum).exactMatch) passed++;
    }
  }

  return {
    passed,
    total,
    notComparable,
    rate: total > 0 ? parseFloat((passed / total).toFixed(3)) : null,
    status: 'unadjudicated',
    gating: false,
  };
}

function summarizeBibliographyPairing(oracleResult) {
  const summary = {
    paired: 0,
    unresolvedUnpaired: 0,
    idProvenOracleOnly: 0,
    idProvenCitumOnly: 0,
    totalObservations: 0,
  };

  for (const entry of oracleResult?.bibliography?.entries || []) {
    summary.totalObservations++;
    const state = bibliographyComparisonState(entry);
    if (state === 'paired') {
      summary.paired++;
    } else if (state === 'oracle-only' && entry?.pairingMethod === 'id') {
      summary.idProvenOracleOnly++;
    } else if (state === 'citum-only' && entry?.pairingMethod === 'id') {
      summary.idProvenCitumOnly++;
    } else {
      summary.unresolvedUnpaired++;
    }
  }

  return summary;
}

function tagOracleResultEvidence(oracleResult, evidence) {
  if (!oracleResult || !evidence) return oracleResult;
  const sections = [
    oracleResult.citations,
    oracleResult.bibliography,
    oracleResult.adjusted?.citations,
    oracleResult.adjusted?.bibliography,
  ];
  for (const section of sections) {
    for (const entry of section?.entries || []) {
      entry.evidenceRunId = evidence.id;
      entry.evidenceRunLabel = evidence.label;
      entry.evidenceAuthority = evidence.authority;
    }
  }
  return oracleResult;
}

function mergeDivergenceDetails(base = {}, extra = {}) {
  const merged = { ...base };

  for (const [key, value] of Object.entries(extra)) {
    if (Array.isArray(value)) {
      const current = Array.isArray(merged[key]) ? merged[key] : [];
      merged[key] = [...new Set([...current, ...value])];
      continue;
    }

    if (typeof value === 'number') {
      merged[key] = (typeof merged[key] === 'number' ? merged[key] : 0) + value;
      continue;
    }

    if (typeof value === 'boolean') {
      merged[key] = Boolean(merged[key]) || value;
      continue;
    }

    merged[key] = value;
  }

  return merged;
}

function mergeDivergenceSummaries(...summaries) {
  const merged = {};

  for (const summary of summaries) {
    for (const [divergenceId, details] of Object.entries(summary || {})) {
      merged[divergenceId] = mergeDivergenceDetails(merged[divergenceId], details);
    }
  }

  return merged;
}

function getCslSnapshotStatus(stylePath, refsFixture, citationsFixture) {
  const styleName = path.basename(stylePath, '.csl');
  const snapshotPath = path.join(CSL_SNAPSHOT_DIR, `${styleName}.json`);
  if (!fs.existsSync(snapshotPath)) {
    return {
      ok: false,
      status: 'missing',
      message: `Snapshot missing for ${styleName}. Run: node scripts/oracle-snapshot.js ${stylePath}`,
      snapshotPath,
    };
  }

  let snapshot;
  try {
    snapshot = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
  } catch (error) {
    return {
      ok: false,
      status: 'invalid',
      message: `Snapshot parse failed for ${styleName}: ${error.message}`,
      snapshotPath,
    };
  }

  const expectedHash = fixtureHash(refsFixture, citationsFixture);
  if (snapshot.fixture_hash !== expectedHash) {
    return {
      ok: false,
      status: 'stale',
      message: `Snapshot stale for ${styleName}. Run: node scripts/oracle-snapshot.js ${stylePath}`,
      snapshotPath,
    };
  }

  return { ok: true, status: 'ok', snapshotPath };
}

async function runNodeOracleScript(scriptPath, args) {
  const result = await spawnProcess(process.execPath, [scriptPath, ...args]);
  return result;
}

function resolveBenchmarkFixturePath(filePath) {
  return path.isAbsolute(filePath) ? filePath : path.join(PROJECT_ROOT, filePath);
}

function resolveBenchmarkRunConfig(benchmarkRun) {
  return {
    ...benchmarkRun,
    refsFixture: resolveBenchmarkFixturePath(benchmarkRun.refsFixture),
    citationsFixture: benchmarkRun.citationsFixture
      ? resolveBenchmarkFixturePath(benchmarkRun.citationsFixture)
      : null,
  };
}

function toRepoRelativePath(filePath) {
  if (!filePath) return null;
  const relativePath = path.relative(PROJECT_ROOT, filePath);
  return relativePath && !relativePath.startsWith('..')
    ? relativePath
    : filePath;
}

async function runCiteprocSnapshotOracle(runtime, stylePath, styleName, styleFormat, refsFixture = DEFAULT_REFS_FIXTURE, citationsFixture = null, styleYamlPath = null) {
  const resolvedCitationsFixture = citationsFixture
    || (styleFormat === 'note' ? NOTE_CITATIONS_FIXTURE : DEFAULT_CITATIONS_FIXTURE);
  const snapshotStatus = getCslSnapshotStatus(stylePath, refsFixture, resolvedCitationsFixture);
  const fastScript = path.join(__dirname, 'oracle-fast.js');
  const liveScript = path.join(__dirname, 'oracle.js');
  const cacheKey = {
    backend: 'citeprocSnapshot',
    styleName,
    stylePath,
    refsFixture,
    citationsFixture: resolvedCitationsFixture,
    styleHash: hashFile(stylePath),
    // YAML changes must also bust the cache: the oracle runs citum with the
    // YAML but the CSL hash alone does not change when the YAML is edited.
    styleYamlHash: styleYamlPath && fs.existsSync(styleYamlPath) ? hashFile(styleYamlPath) : null,
    refsHash: hashFile(refsFixture),
    citationsHash: hashFile(resolvedCitationsFixture),
    snapshotStatus: snapshotStatus.status,
    snapshotHash: snapshotStatus.ok ? hashFile(snapshotStatus.snapshotPath) : null,
    fastScriptHash: hashFile(fastScript),
    liveScriptHash: hashFile(liveScript),
    oracleDivergenceDepsHash: oracleDivergenceDepsHash(),
    citumBin: runtime.citumBin,
    citumBinHash: hashFile(runtime.citumBin),
    allowLiveFallback: runtime.allowLiveFallback,
    caseSensitive: runtime.caseSensitive,
  };

  return runCachedJsonJob(runtime, {
    kind: 'citeprocSnapshot',
    cacheKey,
    async compute() {
      const fastArgs = [
        stylePath,
        '--json',
        '--refs-fixture',
        refsFixture,
        '--citations-fixture',
        resolvedCitationsFixture,
        runtime.caseSensitive ? '--case-sensitive' : '--case-insensitive',
      ];
      if (runtime.allFeatures) {
        fastArgs.push('--all-features');
      }

      if (snapshotStatus.ok) {
        const fast = await runNodeOracleScript(fastScript, fastArgs);
        if ((fast.code === 0 || fast.code === 1) && fast.stdout.trim()) {
          try {
            return JSON.parse(fast.stdout);
          } catch (error) {
            return buildEmptyOracleResult({
              error: `Snapshot oracle JSON parse failed for ${styleName}: ${error.message}`,
              style: styleName,
              oracleSource: 'citeproc-js',
            });
          }
        }

        const fastFailure = (fast.stderr || fast.stdout || `exit ${fast.code}`).trim();
        if (!runtime.allowLiveFallback) {
          return buildEmptyOracleResult({
            error: `Snapshot oracle failed for ${styleName}: ${fastFailure}`,
            style: styleName,
            oracleSource: 'citeproc-js',
            snapshotStatus: snapshotStatus.status,
          });
        }
      }

      if (!runtime.allowLiveFallback) {
        return buildEmptyOracleResult({
          error: snapshotStatus.message || `Snapshot oracle unavailable for ${styleName}`,
          style: styleName,
          oracleSource: 'citeproc-js',
          snapshotStatus: snapshotStatus.status,
        });
      }

      const live = await runNodeOracleScript(liveScript, fastArgs);
      if ((live.code === 0 || live.code === 1) && live.stdout.trim()) {
        try {
          const parsed = JSON.parse(live.stdout);
          parsed.oracleSource = 'citeproc-js-live';
          return parsed;
        } catch (error) {
          return buildEmptyOracleResult({
            error: `Live oracle JSON parse failed for ${styleName}: ${error.message}`,
            style: styleName,
            oracleSource: 'citeproc-js-live',
          });
        }
      }

      return buildEmptyOracleResult({
        error: `Live oracle failed for ${styleName}: ${live.stderr || live.stdout || `exit ${live.code}`}`.trim(),
        style: styleName,
        oracleSource: 'citeproc-js-live',
      });
    },
  });
}

async function runNativeOracle(runtime, styleName) {
  const scriptPath = path.join(__dirname, 'oracle-native.js');
  const styleYamlPath = path.join(PROJECT_ROOT, 'styles', `${styleName}.yaml`);
  const fixturePath = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'compound-numeric-refs.yaml');
  const snapshotPath = path.join(COMPOUND_SNAPSHOT_DIR, `${styleName}.txt`);

  return runCachedJsonJob(runtime, {
    kind: 'nativeSnapshot',
    cacheKey: {
      backend: 'nativeSnapshot',
      styleName,
      styleYamlPath,
      fixturePath,
      snapshotPath,
      styleHash: hashFile(styleYamlPath),
      fixtureHash: hashFile(fixturePath),
      snapshotHash: fs.existsSync(snapshotPath) ? hashFile(snapshotPath) : null,
      scriptHash: hashFile(scriptPath),
      citumBin: runtime.citumBin,
      citumBinHash: hashFile(runtime.citumBin),
    },
    async compute() {
      const result = await runNodeOracleScript(scriptPath, [
        styleName,
        styleYamlPath,
        fixturePath,
        COMPOUND_SNAPSHOT_DIR,
      ]);
      if ((result.code === 0 || result.code === 1) && result.stdout.trim()) {
        try {
          return JSON.parse(result.stdout);
        } catch (error) {
          return buildEmptyOracleResult({
            error: `Native oracle parse failed: ${error.message}`,
            style: styleName,
            oracleSource: 'citum-baseline',
          });
        }
      }
      return buildEmptyOracleResult({
        error: `Native oracle failed: ${result.stderr || result.stdout || `exit ${result.code}`}`.trim(),
        style: styleName,
        oracleSource: 'citum-baseline',
      });
    },
  });
}

async function runCiteprocBenchmarkOracle(runtime, stylePath, styleName, benchmarkRun) {
  const resolvedRun = resolveBenchmarkRunConfig(benchmarkRun);
  const liveScript = path.join(__dirname, 'oracle.js');
  return runCachedJsonJob(runtime, {
    kind: 'benchmarkCiteproc',
    cacheKey: {
      backend: 'benchmarkCiteproc',
      styleName,
      stylePath,
      benchmarkRunId: resolvedRun.id,
      benchmarkRunner: resolvedRun.runner,
      scope: resolvedRun.scope,
      refsFixture: resolvedRun.refsFixture,
      citationsFixture: resolvedRun.citationsFixture,
      styleHash: hashFile(stylePath),
      refsHash: hashFile(resolvedRun.refsFixture),
      citationsHash: resolvedRun.citationsFixture ? hashFile(resolvedRun.citationsFixture) : null,
      liveScriptHash: hashFile(liveScript),
      oracleDivergenceDepsHash: oracleDivergenceDepsHash(),
      citumBin: runtime.citumBin,
      citumBinHash: hashFile(runtime.citumBin),
      caseSensitive: runtime.caseSensitive,
    },
    async compute() {
      const args = [
        stylePath,
        '--json',
        '--scope', resolvedRun.scope,
        '--refs-fixture', resolvedRun.refsFixture,
        runtime.caseSensitive ? '--case-sensitive' : '--case-insensitive',
      ];
      if (runtime.allFeatures) {
        args.push('--all-features');
      }
      if (resolvedRun.citationsFixture && resolvedRun.scope !== 'bibliography') {
        args.push('--citations-fixture', resolvedRun.citationsFixture);
      }
      const result = await runNodeOracleScript(liveScript, args);
      if ((result.code === 0 || result.code === 1) && result.stdout.trim()) {
        try {
          const parsed = JSON.parse(result.stdout);
          parsed.oracleSource = 'citeproc-js-live';
          return parsed;
        } catch (error) {
          return buildEmptyOracleResult({
            error: `Benchmark oracle JSON parse failed for ${styleName}/${resolvedRun.id}: ${error.message}`,
            style: styleName,
            oracleSource: 'citeproc-js-live',
          });
        }
      }

      return buildEmptyOracleResult({
        error: `Benchmark oracle failed for ${styleName}/${resolvedRun.id}: ${result.stderr || result.stdout || `exit ${result.code}`}`.trim(),
        style: styleName,
        oracleSource: 'citeproc-js-live',
      });
    },
  });
}

function countRenderedBibliographyEntries(rendered) {
  return rendered?.bibliography?.entries?.length || 0;
}

async function runNativeSmokeBenchmark(runtime, styleYamlPath, benchmarkRun) {
  const resolvedRun = resolveBenchmarkRunConfig(benchmarkRun);
  const rendered = await renderCitumJson(runtime, styleYamlPath, resolvedRun.refsFixture, 'bib');
  return {
    bibliographyEntries: countRenderedBibliographyEntries(rendered),
  };
}

async function renderCitumJson(runtime, styleYamlPath, refsFixture, mode = 'both', citationsFixture = null) {
  const resolvedCitationsFixture = citationsFixture ? path.resolve(citationsFixture) : null;
  const cacheKey = {
    backend: 'citumRender',
    styleYamlPath,
    refsFixture,
    citationsFixture: resolvedCitationsFixture,
    mode,
    styleHash: hashFile(styleYamlPath),
    refsHash: hashFile(refsFixture),
    citationsHash: resolvedCitationsFixture ? hashFile(resolvedCitationsFixture) : null,
    citumBin: runtime.citumBin,
    citumBinHash: hashFile(runtime.citumBin),
  };

  return runCachedJsonJob(runtime, {
    kind: 'citumRender',
    cacheKey,
    async compute() {
      const args = [
        'render', 'refs',
        '-b', refsFixture,
        '-s', styleYamlPath,
        '--json',
        '--mode', mode,
      ];
      if (resolvedCitationsFixture) {
        args.splice(4, 0, '-c', resolvedCitationsFixture);
      }
      const result = await spawnProcess(runtime.citumBin, args, { cwd: PROJECT_ROOT });
      if (result.code !== 0) {
        throw new Error((result.stderr || result.stdout || `exit ${result.code}`).trim());
      }
      return JSON.parse(result.stdout);
    },
  });
}

/**
 * Expand compound bibliography blocks into individual entries.
 *
 * Some biblatex styles (chem-acs, chem-rsc, chem-biochem) produce compound
 * bibliography entries where multiple sub-entries are merged into a single
 * string: "(1) text1 (2) text2 ... (N) textN".  Citum renders individual
 * entries, so we expand the compound blocks before comparing.
 */
function expandCompoundBibEntries(entries) {
  const result = [];
  for (const entry of entries) {
    // Compound block: starts with "(N) " and contains at least one more " (M) "
    if (/^\(\d+\) /.test(entry) && / \(\d+\) /.test(entry)) {
      // Split at positions where a space is immediately followed by "(M) "
      const subEntries = entry.split(/(?= \(\d+\) )/).map((s) => s.trim()).filter(Boolean);
      if (subEntries.length > 1) {
        result.push(...subEntries);
        continue;
      }
    }
    result.push(entry);
  }
  return result;
}

async function runBiblatexSnapshotOracle(runtime, styleName, styleYamlPath, authorityId) {
  const snapshotPath = path.join(BIBLATEX_SNAPSHOT_DIR, `${styleName}.json`);

  if (!fs.existsSync(snapshotPath)) {
    return buildEmptyOracleResult({
      error: `Biblatex snapshot not found: ${snapshotPath}`,
      oracleSource: 'biblatex',
      authorityId,
    });
  }

  return runCachedJsonJob(runtime, {
    kind: 'biblatexSnapshot',
    cacheKey: {
      backend: 'biblatexSnapshot',
      styleName,
      authorityId,
      styleYamlPath,
      refsFixture: DEFAULT_REFS_FIXTURE,
      snapshotPath,
      styleHash: hashFile(styleYamlPath),
      refsHash: hashFile(DEFAULT_REFS_FIXTURE),
      snapshotHash: hashFile(snapshotPath),
      caseSensitive: runtime.caseSensitive,
    },
    async compute() {
      let snapshot;
      try {
        snapshot = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
      } catch (error) {
        return buildEmptyOracleResult({
          error: `Biblatex snapshot parse failed: ${error.message}`,
          oracleSource: 'biblatex',
          authorityId,
        });
      }

      try {
        // Build ordered ref list from the fixture so we can track per-type pass
        // rates and run per-position component diffs. The biblatex snapshot
        // entries correspond positionally to fixture entries after expansion of
        // compound blocks.
        const fixtureData = JSON.parse(fs.readFileSync(DEFAULT_REFS_FIXTURE, 'utf8'));
        const fixtureRefs = Object.entries(fixtureData)
          .filter(([key]) => key !== 'comment')
          .map(([, ref]) => (typeof ref === 'object' && ref !== null ? ref : null));

        const rendered = await renderCitumJson(runtime, styleYamlPath, DEFAULT_REFS_FIXTURE, 'bib');
        const actualEntries = rendered?.bibliography?.entries?.map((entry) => entry.text) || [];
        const expectedEntries = expandCompoundBibEntries(snapshot.bibliography || []);
        const observationCount = Math.max(expectedEntries.length, actualEntries.length);
        let total = 0;
        let passed = 0;
        const entries = [];
        const citationsByType = {};

        for (let i = 0; i < observationCount; i++) {
          const expected = expectedEntries[i] ?? null;
          const actual = actualEntries[i] ?? null;
          const isPaired = expected !== null && actual !== null;
          const comparison = isPaired
            ? compareText(expected, actual, {
              caseSensitive: runtime.caseSensitive,
            })
            : null;
          const match = comparison?.match ?? null;
          if (isPaired) {
            total += 1;
            if (match) passed += 1;
          }
          const entryResult = {
            expected: comparison?.expected ?? expected,
            actual: comparison?.actual ?? actual,
            rawExpected: comparison?.rawExpected ?? expected,
            rawActual: comparison?.rawActual ?? actual,
            exactExpected: comparison?.exactExpected ?? null,
            exactActual: comparison?.exactActual ?? null,
            exactMatch: comparison?.exactMatch ?? null,
            exactAdjudication: comparison?.exactAdjudication ?? 'not-comparable',
            match,
            caseMismatch: comparison?.caseMismatch ?? false,
            pairingMethod: 'position',
            comparisonState: isPaired ? 'paired' : 'unresolved-unpaired',
            compatibilityEligible: isPaired,
            exactParityEligible: isPaired,
            issues: isPaired
              ? []
              : [{
                issue: 'unpaired_output',
                detail: expected === null
                  ? 'Positional pairing found no benchmark counterpart'
                  : 'Positional pairing found no Citum counterpart',
              }],
          };

          // On mismatch, run the same symmetric component-diff heuristic used
          // for citeproc-js styles so the Components column gets a comparable
          // signal for biblatex-authority styles.
          if (isPaired && !match) {
            const refData = fixtureRefs[i];
            if (refData) {
              const expectedComp = parseComponents(comparison.expected, refData);
              const actualComp = parseComponents(comparison.actual, refData);
              const { differences, matches } =
                compareComponents(expectedComp, actualComp, refData);
              entryResult.components = { matches, differences };
            }
          }

          entries.push(entryResult);

          // Track per-type stats using the fixture entry at this position.
          if (isPaired) {
            const refType = (fixtureRefs[i] && fixtureRefs[i].type) || 'unknown';
            const typeStats = citationsByType[refType] || { passed: 0, total: 0 };
            typeStats.total += 1;
            if (match) typeStats.passed += 1;
            citationsByType[refType] = typeStats;
          }
        }

        return buildEmptyOracleResult({
          citations: { passed: 0, total: 0, entries: [] },
          bibliography: { passed, total, entries },
          citationsByType,
          oracleSource: 'biblatex',
          authorityId,
        });
      } catch (error) {
        return buildEmptyOracleResult({
          error: `Biblatex comparator failed: ${error.message}`,
          oracleSource: 'biblatex',
          authorityId,
        });
      }
    },
  });
}

async function runFamilyFixtureOracle(runtime, stylePath, styleName, fixtureSetName) {
  const refsFixture = FIXTURE_SET_REFS[fixtureSetName];
  const citationsFixture = FIXTURE_SET_CITATIONS[fixtureSetName];
  if (!refsFixture || !citationsFixture) return null;
  if (!fs.existsSync(refsFixture) || !fs.existsSync(citationsFixture)) return null;

  const liveScript = path.join(__dirname, 'oracle.js');
  return runCachedJsonJob(runtime, {
    kind: 'familyFixture',
    cacheKey: {
      backend: 'familyFixture',
      styleName,
      stylePath,
      refsFixture,
      citationsFixture,
      styleHash: hashFile(stylePath),
      refsHash: hashFile(refsFixture),
      citationsHash: hashFile(citationsFixture),
      liveScriptHash: hashFile(liveScript),
      oracleDivergenceDepsHash: oracleDivergenceDepsHash(),
      citumBin: runtime.citumBin,
      citumBinHash: hashFile(runtime.citumBin),
      caseSensitive: runtime.caseSensitive,
    },
    async compute() {
      const args = [
        stylePath,
        '--json',
        '--refs-fixture', refsFixture,
        '--citations-fixture', citationsFixture,
        runtime.caseSensitive ? '--case-sensitive' : '--case-insensitive',
      ];
      if (runtime.allFeatures) {
        args.push('--all-features');
      }
      const result = await runNodeOracleScript(liveScript, args);
      if ((result.code === 0 || result.code === 1) && result.stdout.trim()) {
        try {
          return JSON.parse(result.stdout);
        } catch (error) {
          process.stderr.write(`[family-fixture] ${styleName}/${fixtureSetName}: ${error.message}\n`);
          return null;
        }
      }
      process.stderr.write(`[family-fixture] ${styleName}/${fixtureSetName}: ${result.stderr || result.stdout || `exit ${result.code}`}\n`);
      return null;
    },
  });
}

/**
 * Merge a family fixture oracle result into the main oracle result.
 * Adds passed/total counts from the family run to the main result.
 */
function mergeOracleResults(main, extra) {
  if (!extra) return main;

  const mCit = main.citations || { passed: 0, total: 0 };
  const eCit = extra.citations || { passed: 0, total: 0 };
  const mBib = main.bibliography || { passed: 0, total: 0 };
  const eBib = extra.bibliography || { passed: 0, total: 0 };
  const mAdjCit = getEffectiveOracleSection(main, 'citations');
  const eAdjCit = getEffectiveOracleSection(extra, 'citations');
  const mAdjBib = getEffectiveOracleSection(main, 'bibliography');
  const eAdjBib = getEffectiveOracleSection(extra, 'bibliography');

  main.citations = {
    passed: (mCit.passed || 0) + (eCit.passed || 0),
    total: (mCit.total || 0) + (eCit.total || 0),
    entries: [...(mCit.entries || []), ...(eCit.entries || [])],
  };
  main.bibliography = {
    passed: (mBib.passed || 0) + (eBib.passed || 0),
    total: (mBib.total || 0) + (eBib.total || 0),
    entries: [...(mBib.entries || []), ...(eBib.entries || [])],
  };
  main.adjusted = {
    citations: {
      passed: (mAdjCit.passed || 0) + (eAdjCit.passed || 0),
      total: (mAdjCit.total || 0) + (eAdjCit.total || 0),
      entries: [...(mAdjCit.entries || []), ...(eAdjCit.entries || [])],
    },
    bibliography: {
      passed: (mAdjBib.passed || 0) + (eAdjBib.passed || 0),
      total: (mAdjBib.total || 0) + (eAdjBib.total || 0),
      entries: [...(mAdjBib.entries || []), ...(eAdjBib.entries || [])],
    },
    divergenceSummary: mergeDivergenceSummaries(
      main.adjusted?.divergenceSummary,
      extra.adjusted?.divergenceSummary
    ),
  };

  return main;
}

function mergeCitationResults(main, extra) {
  if (!extra) return main;

  const mainCitations = main.citations || { passed: 0, total: 0, entries: [] };
  const extraCitations = extra.citations || { passed: 0, total: 0, entries: [] };
  const mainAdjusted = getEffectiveOracleSection(main, 'citations');
  const extraAdjusted = getEffectiveOracleSection(extra, 'citations');
  main.citations = {
    passed: (mainCitations.passed || 0) + (extraCitations.passed || 0),
    total: (mainCitations.total || 0) + (extraCitations.total || 0),
    entries: [...(mainCitations.entries || []), ...(extraCitations.entries || [])],
  };
  main.adjusted = {
    citations: {
      passed: (mainAdjusted.passed || 0) + (extraAdjusted.passed || 0),
      total: (mainAdjusted.total || 0) + (extraAdjusted.total || 0),
      entries: [...(mainAdjusted.entries || []), ...(extraAdjusted.entries || [])],
    },
    bibliography: getEffectiveOracleSection(main, 'bibliography'),
    divergenceSummary: mergeDivergenceSummaries(
      main.adjusted?.divergenceSummary,
      extra.adjusted?.divergenceSummary
    ),
  };

  const mergedByType = { ...(main.citationsByType || {}) };
  for (const [refType, stats] of Object.entries(extra.citationsByType || {})) {
    const current = mergedByType[refType] || { passed: 0, total: 0 };
    mergedByType[refType] = {
      passed: (current.passed || 0) + (stats.passed || 0),
      total: (current.total || 0) + (stats.total || 0),
    };
  }
  main.citationsByType = mergedByType;

  return main;
}

function mergeOracleErrors(...results) {
  return results
    .map((result) => result?.error)
    .filter(Boolean)
    .join('\n');
}

async function executeBenchmarkRuns(benchmarkRuns, executor) {
  const results = [];
  for (const benchmarkRun of benchmarkRuns || []) {
    results.push(await executor(benchmarkRun));
  }
  return results;
}

function mergeBenchmarkRunIntoOracle(oracleResult, benchmarkRunRecord) {
  if (!benchmarkRunRecord?.countTowardFidelity || !benchmarkRunRecord?.oracleResult) {
    return oracleResult;
  }
  if (benchmarkRunRecord.scope === 'citation') {
    mergeCitationResults(oracleResult, benchmarkRunRecord.oracleResult);
    return oracleResult;
  }
  mergeOracleResults(oracleResult, benchmarkRunRecord.oracleResult);
  return oracleResult;
}

function formatBenchmarkRunRecord(benchmarkRun, extras = {}) {
  return {
    id: benchmarkRun.id,
    label: benchmarkRun.label,
    runner: benchmarkRun.runner,
    scope: benchmarkRun.scope,
    countTowardFidelity: benchmarkRun.countTowardFidelity,
    minPassRate: benchmarkRun.minPassRate ?? null,
    refsFixture: benchmarkRun.refsFixture,
    citationsFixture: benchmarkRun.citationsFixture || null,
    ...extras,
  };
}

function summarizeSection(section = null) {
  if (!section) return null;
  const total = section.total || 0;
  const passed = section.passed || 0;
  return {
    passed,
    total,
    failed: Math.max(0, total - passed),
    matchRate: total > 0 ? parseFloat((passed / total).toFixed(3)) : null,
  };
}

function summarizeBenchmarkRunRecord(benchmarkRunRecord) {
  return {
    id: benchmarkRunRecord.id,
    label: benchmarkRunRecord.label,
    runner: benchmarkRunRecord.runner,
    scope: benchmarkRunRecord.scope,
    status: benchmarkRunRecord.status,
    countTowardFidelity: benchmarkRunRecord.countTowardFidelity,
    minPassRate: benchmarkRunRecord.minPassRate ?? null,
    citations: summarizeSection(benchmarkRunRecord.citations),
    bibliography: summarizeSection(benchmarkRunRecord.bibliography),
    bibliographyEntries: benchmarkRunRecord.bibliographyEntries ?? null,
  };
}

function buildRichInputEvidenceSummary(styleRecord, benchmarkRunResults) {
  return {
    headlineGate: {
      citations: summarizeSection(styleRecord.rawCitations),
      bibliography: summarizeSection(styleRecord.rawBibliography),
      qualityScore: styleRecord.qualityScore,
    },
    officialSupplemental: benchmarkRunResults.map(summarizeBenchmarkRunRecord),
  };
}

function toPublishedBenchmarkRunRecord(benchmarkRunRecord) {
  return {
    id: benchmarkRunRecord.id,
    label: benchmarkRunRecord.label,
    runner: benchmarkRunRecord.runner,
    scope: benchmarkRunRecord.scope,
    countTowardFidelity: benchmarkRunRecord.countTowardFidelity,
    refsFixture: toRepoRelativePath(benchmarkRunRecord.refsFixture),
    citationsFixture: toRepoRelativePath(benchmarkRunRecord.citationsFixture),
    status: benchmarkRunRecord.status,
    error: benchmarkRunRecord.error || null,
    citations: benchmarkRunRecord.citations
      ? {
        passed: benchmarkRunRecord.citations.passed || 0,
        total: benchmarkRunRecord.citations.total || 0,
      }
      : null,
    bibliography: benchmarkRunRecord.bibliography
      ? {
        passed: benchmarkRunRecord.bibliography.passed || 0,
        total: benchmarkRunRecord.bibliography.total || 0,
      }
      : null,
    bibliographyEntries: benchmarkRunRecord.bibliographyEntries ?? null,
  };
}

function determineBenchmarkStatus(oracleResult, minPassRate) {
  if (oracleResult.error) return 'error';
  if (minPassRate != null) {
    // Registered divergences (verification-policy.yaml `divergences`) mask mismatches that
    // are known-intentional, not defects — the gate must judge conformance against the
    // adjusted counts, or every registered divergence is purely decorative and never
    // actually keeps a style's declared min_pass_rate green.
    const bib = oracleResult.adjusted?.bibliography || oracleResult.bibliography || { passed: 0, total: 0 };
    const cit = oracleResult.adjusted?.citations || oracleResult.citations || { passed: 0, total: 0 };
    const totalPassed = (bib.passed || 0) + (cit.passed || 0);
    const totalItems = (bib.total || 0) + (cit.total || 0);
    const matchRate = totalItems > 0 ? totalPassed / totalItems : 0;
    return matchRate >= minPassRate ? 'pass' : 'fail';
  }
  return 'ok';
}

async function runBenchmarkRun(runtime, styleSpec, stylePath, styleYamlPath, benchmarkRun) {
  const resolvedRun = resolveBenchmarkRunConfig(benchmarkRun);
  try {
    if (resolvedRun.runner === BENCHMARK_RUNNERS.CITEPROC_ORACLE) {
      const oracleResult = await runCiteprocBenchmarkOracle(runtime, stylePath, styleSpec.name, resolvedRun);
      const benchmarkStatus = determineBenchmarkStatus(oracleResult, resolvedRun.minPassRate);
      return formatBenchmarkRunRecord(resolvedRun, {
        status: benchmarkStatus,
        error: oracleResult.error || null,
        oracleResult,
        citations: oracleResult.citations || { passed: 0, total: 0, entries: [] },
        bibliography: oracleResult.bibliography || { passed: 0, total: 0, entries: [] },
      });
    }

    if (resolvedRun.runner === BENCHMARK_RUNNERS.NATIVE_SMOKE) {
      const nativeResult = await runNativeSmokeBenchmark(runtime, styleYamlPath, resolvedRun);
      return formatBenchmarkRunRecord(resolvedRun, {
        status: 'pass',
        error: null,
        bibliographyEntries: nativeResult.bibliographyEntries,
      });
    }

    return formatBenchmarkRunRecord(resolvedRun, {
      status: 'error',
      error: `Unsupported benchmark runner: ${resolvedRun.runner}`,
    });
  } catch (error) {
    return formatBenchmarkRunRecord(resolvedRun, {
      status: 'error',
      error: error.message,
    });
  }
}

function composeScopedOracleResult(citationResult, bibliographyResult) {
  const combinedError = mergeOracleErrors(citationResult, bibliographyResult);
  return {
    citations: citationResult?.citations || { passed: 0, total: 0, entries: [] },
    bibliography: bibliographyResult?.bibliography || { passed: 0, total: 0, entries: [] },
    adjusted: {
      citations: getEffectiveOracleSection(citationResult, 'citations'),
      bibliography: getEffectiveOracleSection(bibliographyResult, 'bibliography'),
      divergenceSummary: mergeDivergenceSummaries(
        citationResult?.adjusted?.divergenceSummary,
        bibliographyResult?.adjusted?.divergenceSummary
      ),
    },
    citationsByType: citationResult?.citationsByType || {},
    componentSummary: bibliographyResult?.componentSummary || {},
    oracleSource: bibliographyResult?.oracleSource || citationResult?.oracleSource || 'citeproc-js',
    error: combinedError || null,
  };
}

/**
 * Compute fidelity score from oracle result.
 *
 * For note styles (hasBibliography === false) the oracle still runs bibliography
 * scenarios and returns 0/N results — those must be excluded so they don't
 * deflate the citation-only fidelity score.
 */
function computeFidelityScore(oracleResult, hasBibliography) {
  if (oracleResult.error) {
    return 0;
  }

  const citations = getEffectiveOracleSection(oracleResult, 'citations');

  const citationsPassed = citations.passed || 0;
  const citationsTotal = citations.total || 0;

  if (hasBibliography === false) {
    return citationsTotal > 0 ? Math.min(1, citationsPassed / citationsTotal) : 0;
  }

  const bibliography = getEffectiveOracleSection(oracleResult, 'bibliography');
  const biblioPassed = bibliography.passed || 0;
  const biblioTotal = bibliography.total || 0;

  const totalPassed = citationsPassed + biblioPassed;
  const totalTests = citationsTotal + biblioTotal;

  return totalTests > 0 ? Math.min(1, totalPassed / totalTests) : 0;
}

/**
 * Load known divergences
 */
function loadDivergences() {
  try {
    const divergencePath = path.join(__dirname, 'report-data', 'known-divergences.json');
    const content = fs.readFileSync(divergencePath, 'utf8');
    return JSON.parse(content);
  } catch {
    return {};
  }
}

/**
 * Compute component match rate from oracle result
 */
function computeComponentMatchRate(oracleResult) {
  if (oracleResult.error || !oracleResult.bibliography) return null;

  let totalMatches = 0;
  let totalComponents = 0;

  for (const entry of oracleResult.bibliography.entries || []) {
    if (entry.match) {
      totalMatches += 11;
      totalComponents += 11;
    } else if (entry.components) {
      const matches = (entry.components.matches || []).length;
      const diffs = (entry.components.differences || []).length;
      totalMatches += matches;
      totalComponents += matches + diffs;
    }
  }

  return totalComponents > 0 ? parseFloat((totalMatches / totalComponents).toFixed(3)) : null;
}

function clamp(min, max, value) {
  return Math.max(min, Math.min(max, value));
}

function safePct(value) {
  return parseFloat(clamp(0, 100, value).toFixed(1));
}

function loadStyleYaml(styleName, stylePathOverride = null) {
  const stylePath = resolveStyleYamlPath(styleName, stylePathOverride);
  if (!fs.existsSync(stylePath)) {
    return {
      stylePath,
      rawStyleData: null,
      resolvedStyleData: null,
      error: `Style YAML not found: ${stylePath}`,
    };
  }
  try {
    const rawStyleData = yaml.load(fs.readFileSync(stylePath, 'utf8'));
    return {
      stylePath,
      rawStyleData,
      resolvedStyleData: resolveStyleData(cloneJson(rawStyleData)),
      error: null,
    };
  } catch (error) {
    return {
      stylePath,
      rawStyleData: null,
      resolvedStyleData: null,
      error: `YAML parse error: ${error.message}`,
    };
  }
}

function resolveStyleYamlPath(styleName, stylePathOverride = null) {
  if (stylePathOverride) {
    return path.resolve(stylePathOverride);
  }

  const stylesRoot = path.join(path.dirname(__dirname), 'styles');
  const candidatePaths = [
    path.join(stylesRoot, `${styleName}.yaml`),
    path.join(stylesRoot, 'embedded', `${styleName}.yaml`),
  ];

  return candidatePaths.find((candidatePath) => fs.existsSync(candidatePath)) || candidatePaths[0];
}

function flattenTemplateComponents(components) {
  const flattened = [];
  for (const component of components || []) {
    if (!component || typeof component !== 'object') continue;
    flattened.push(component);
    if (Array.isArray(component.group)) {
      flattened.push(...flattenTemplateComponents(component.group));
    }
    if (Array.isArray(component.items)) {
      flattened.push(...flattenTemplateComponents(component.items));
    }
  }
  return flattened;
}

function collectTemplateScopes(styleData) {
  const citation = styleData?.citation || {};
  const bibliography = styleData?.bibliography || {};
  const typeTemplates = bibliography['type-templates'] || {};
  const citationTypeVariants = citation['type-variants'] || {};
  const integralTypeVariants = citation.integral?.['type-variants'] || {};
  const nonIntegralTypeVariants = citation['non-integral']?.['type-variants'] || {};
  const bibliographyTypeVariants = bibliography['type-variants'] || {};
  const scopes = [];
  let variantSelectorCount = 0;

  function addScope(name, components, meta = {}) {
    if (!Array.isArray(components)) return;
    scopes.push({
      name,
      components,
      ...meta,
    });
  }

  function addVariantScopes(prefix, variants, kind) {
    for (const [rawKey, template] of Object.entries(variants || {})) {
      const isDiffForm = !Array.isArray(template) && Boolean(template) && typeof template === 'object';
      const components = Array.isArray(template) ? template : templateVariantAuthoredComponents(template);
      if (!Array.isArray(components)) continue;
      const typeSelectors = parseOverrideKey(rawKey).filter((key) => key !== 'default');
      variantSelectorCount += typeSelectors.length || 1;
      addScope(`${prefix}.${rawKey}`, components, {
        kind,
        isDiffForm,
        typeSelectors,
        typeSelectorCount: typeSelectors.length || 1,
      });
    }
  }

  addScope('citation.template', citation.template, { kind: 'template' });
  addScope('citation.integral.template', citation.integral?.template, { kind: 'template' });
  addScope('citation.non-integral.template', citation['non-integral']?.template, { kind: 'template' });
  addScope('bibliography.template', bibliography.template, { kind: 'template' });
  addVariantScopes('citation.type-variants', citationTypeVariants, 'citation-type-variant');
  addVariantScopes('citation.integral.type-variants', integralTypeVariants, 'citation-type-variant');
  addVariantScopes('citation.non-integral.type-variants', nonIntegralTypeVariants, 'citation-type-variant');
  addVariantScopes('bibliography.type-variants', bibliographyTypeVariants, 'bibliography-type-variant');

  for (const [typeKey, template] of Object.entries(typeTemplates)) {
    if (!Array.isArray(template)) continue;
    const typeSelectors = parseOverrideKey(typeKey).filter((key) => key !== 'default');
    addScope(`bibliography.type-templates.${typeKey}`, template, {
      kind: 'bibliography-type-template',
      typeSelectors,
      typeSelectorCount: typeSelectors.length || 1,
    });
  }

  return { scopes, variantSelectorCount };
}

function templateVariantAuthoredComponents(variant) {
  if (!variant || typeof variant !== 'object' || Array.isArray(variant)) return null;
  const components = [];
  for (const operation of variant.modify || []) {
    const component = { ...(operation.match || {}), __isDiff: true };
    for (const [key, value] of Object.entries(operation)) {
      if (key !== 'match') component[key] = value;
    }
    components.push(component);
  }
  for (const operation of variant.remove || []) {
    components.push({ ...(operation.match || {}), suppress: true, __isDiff: true });
  }
  for (const operation of variant.add || []) {
    if (operation.component) {
      components.push({ ...operation.component, __isDiff: true });
    }
  }
  return components.length > 0 ? components : null;
}

function parseOverrideKey(rawKey) {
  const key = String(rawKey || '').trim();
  if (!key) return [];
  if (key === 'default') return ['default'];
  return key
    .replace(/^\[/, '')
    .replace(/\]$/, '')
    .split(',')
    .map((part) => part.trim())
    .map((part) => part.replace(/^['"]|['"]$/g, ''))
    .filter(Boolean);
}

function resolveOverrideForType(overrides, refType) {
  if (!overrides || typeof overrides !== 'object') return null;
  let defaultOverride = null;
  for (const [rawKey, value] of Object.entries(overrides)) {
    const keys = parseOverrideKey(rawKey);
    if (keys.includes('default')) {
      defaultOverride = value;
      continue;
    }
    if (keys.includes(refType)) return value;
  }
  return defaultOverride;
}

function componentVisibleForType(component, refType) {
  const baseSuppressed = component?.suppress === true;
  const override = resolveOverrideForType(component?.overrides, refType);
  if (override && typeof override === 'object' && Object.prototype.hasOwnProperty.call(override, 'suppress')) {
    return override.suppress !== true;
  }
  return !baseSuppressed;
}

function isAnchorComponent(component) {
  return Boolean(component?.contributor || component?.title || component?.date);
}

function componentSemanticKey(component) {
  if (component.contributor) return `contributor:${component.contributor}`;
  if (component.title) return `title:${component.title}`;
  if (component.date) return `date:${component.date}:${component.form || 'default'}`;
  if (component.number) return `number:${component.number}`;
  if (component.variable) return `variable:${component.variable}`;
  if (component.items) return 'items-group';
  return Object.keys(component).sort().join('|') || 'unknown';
}

function stableStructure(value) {
  if (Array.isArray(value)) {
    return value.map(stableStructure);
  }
  if (!value || typeof value !== 'object') {
    return value;
  }
  const normalized = {};
  for (const key of Object.keys(value).sort()) {
    if (key === '__isDiff') continue;
    normalized[key] = stableStructure(value[key]);
  }
  return normalized;
}

function fingerprintValue(value) {
  return JSON.stringify(stableStructure(value));
}

function fingerprintComponent(component) {
  return fingerprintValue(component);
}

function fingerprintScopeComponents(components) {
  return fingerprintValue(components || []);
}

function collectPatternFingerprints(components, fingerprints = []) {
  for (const component of components || []) {
    if (!component || typeof component !== 'object') continue;
    const clone = { ...component };
    delete clone.overrides;
    fingerprints.push(fingerprintComponent(clone));
    if (Array.isArray(component.group)) {
      fingerprints.push(`group:${fingerprintValue(component.group)}`);
      collectPatternFingerprints(component.group, fingerprints);
    }
    if (Array.isArray(component.items)) {
      fingerprints.push(`items:${fingerprintValue(component.items)}`);
      collectPatternFingerprints(component.items, fingerprints);
    }
  }
  return fingerprints;
}

function componentDistance(left, right) {
  const leftShape = stableStructure(left);
  const rightShape = stableStructure(right);
  const leftKeys = new Set(Object.keys(leftShape || {}));
  const rightKeys = new Set(Object.keys(rightShape || {}));
  const allKeys = new Set([...leftKeys, ...rightKeys]);
  let distance = 0;

  for (const key of allKeys) {
    const leftValue = leftShape?.[key];
    const rightValue = rightShape?.[key];
    if (JSON.stringify(leftValue) !== JSON.stringify(rightValue)) {
      distance += 1;
    }
  }

  return distance;
}

function scopeDistance(leftComponents, rightComponents) {
  const left = leftComponents || [];
  const right = rightComponents || [];
  const maxLength = Math.max(left.length, right.length);
  let distance = Math.abs(left.length - right.length);

  for (let index = 0; index < maxLength; index += 1) {
    if (!left[index] || !right[index]) continue;
    distance += componentDistance(left[index], right[index]);
  }

  return distance;
}

function countTemplatePresetUses(node) {
  let count = 0;
  function visit(value) {
    if (!value || typeof value !== 'object') return;
    if (Array.isArray(value)) {
      for (const item of value) visit(item);
      return;
    }
    for (const [key, child] of Object.entries(value)) {
      if (key === 'extends') count += 1;
      if (key === 'preset' && typeof child === 'string' && child.trim()) count += 1;
      visit(child);
    }
  }
  visit(node);
  return count;
}

function countOptionsPresetUses(styleData) {
  const optionScopes = [
    styleData?.options,
    styleData?.citation?.options,
    styleData?.bibliography?.options,
  ].filter(Boolean);

  const keys = ['processing', 'contributors', 'dates', 'titles', 'substitute'];
  let uses = 0;
  const fields = [];

  for (const options of optionScopes) {
    for (const key of keys) {
      const value = options[key];
      if (typeof value === 'string') {
        uses += 1;
        fields.push(key);
      } else if (value && typeof value === 'object') {
        if (typeof value.preset === 'string' && value.preset.trim()) {
          uses += 1;
          fields.push(key);
        } else if (typeof value['extends'] === 'string' && value['extends'].trim()) {
          uses += 1;
          fields.push(key);
        }
      }
    }
  }

  return { uses, fields };
}

function hasRootExtends(styleData) {
  return typeof styleData?.extends === 'string' && styleData.extends.trim().length > 0;
}

function computeTypeCoverageScore(citationsByType) {
  const entries = Object.entries(citationsByType || {})
    .filter(([, stats]) => (stats?.total || 0) > 0);

  if (entries.length === 0) {
    return {
      score: 0,
      observedTypes: 0,
      averageTypePassRate: 0,
      breadthFactor: 0,
    };
  }

  const averageTypePassRate = entries
    .map(([, stats]) => stats.passed / stats.total)
    .reduce((sum, rate) => sum + rate, 0) / entries.length;
  const breadthFactor = clamp(0, 1, entries.length / 4);
  const score = ((averageTypePassRate * 0.7) + (breadthFactor * 0.3)) * 100;

  return {
    score: safePct(score),
    observedTypes: entries.length,
    averageTypePassRate: parseFloat((averageTypePassRate * 100).toFixed(1)),
    breadthFactor: parseFloat((breadthFactor * 100).toFixed(1)),
  };
}

function computeFallbackRobustness(styleData) {
  const bibliography = styleData?.bibliography || {};
  const typeTemplates = bibliography['type-templates'] || {};
  const typeVariants = bibliography['type-variants'] || {};
  const explicitTypes = new Set();

  [typeTemplates, typeVariants].forEach((map) => {
    for (const rawKey of Object.keys(map)) {
      for (const type of parseOverrideKey(rawKey)) {
        if (type !== 'default') explicitTypes.add(type);
      }
    }
  });

  const assessedTypes = CORE_FALLBACK_TYPES.filter((type) => !explicitTypes.has(type));
  if (typeof bibliography['extends'] === 'string' && bibliography['extends'].trim()) {
    return {
      score: 100,
      assessedTypes: assessedTypes.length,
      passingTypes: assessedTypes.length,
      note: `embedded bibliography preset: ${bibliography['extends']} (assumed robust)`,
    };
  }
  const flattenedBase = flattenTemplateComponents(Array.isArray(bibliography.template) ? bibliography.template : []);

  if (assessedTypes.length === 0) {
    return {
      score: 100,
      assessedTypes: 0,
      passingTypes: 0,
      note: 'all core types have explicit type-templates or type-variants',
    };
  }

  let passingTypes = 0;
  for (const refType of assessedTypes) {
    const visible = flattenedBase.filter((component) => componentVisibleForType(component, refType));
    const anchorCount = visible.filter(isAnchorComponent).length;
    if (visible.length > 0 && anchorCount >= 2) passingTypes += 1;
  }

  return {
    score: safePct((passingTypes / assessedTypes.length) * 100),
    assessedTypes: assessedTypes.length,
    passingTypes,
    note: 'base bibliography template only',
  };
}

function computeConcisionScore(styleData, format) {
  const { scopes, variantSelectorCount } = collectTemplateScopes(styleData);
  const scopedComponents = scopes
    .map((scope) => ({
      name: scope.name,
      kind: scope.kind,
      isDiffForm: Boolean(scope.isDiffForm),
      typeSelectors: scope.typeSelectors || [],
      typeSelectorCount: scope.typeSelectorCount || 0,
      scopeFingerprint: fingerprintScopeComponents(scope.components),
      patternFingerprints: collectPatternFingerprints(scope.components),
      components: flattenTemplateComponents(scope.components),
      originalComponents: scope.components,
    }))
    .filter((scope) => scope.components.length > 0);
  const diffScopes = scopedComponents.filter((scope) => scope.isDiffForm);
  const duplicateAnalysisScopes = scopedComponents.filter((scope) => !scope.isDiffForm);
  const diffVariantOperations = diffScopes.reduce((sum, scope) => sum + scope.components.length, 0);
  const flattened = scopedComponents
    .filter((scope) => !scope.isDiffForm)
    .flatMap((scope) => scope.components);

  if (flattened.length === 0) {
    if (hasRootExtends(styleData)) {
      return {
        score: 100,
        scopeCount: scopedComponents.length,
        totalComponents: 0,
        duplicates: 0,
        withinScopeDuplicates: 0,
        crossScopeRepeats: 0,
        exactDuplicateScopes: 0,
        nearDuplicateScopes: 0,
        repeatedPatterns: 0,
        diffVariantScopes: diffScopes.length,
        diffVariantOperations,
        variantSelectors: variantSelectorCount,
        overrideDensity: 0,
        targetComponents: 0,
        inheritedPreset: styleData.extends,
        note: `root extends: ${styleData.extends}`,
      };
    }

    return {
      score: 0,
      scopeCount: 0,
      totalComponents: 0,
      duplicates: 0,
      withinScopeDuplicates: 0,
      crossScopeRepeats: 0,
      exactDuplicateScopes: 0,
      nearDuplicateScopes: 0,
      repeatedPatterns: 0,
      diffVariantScopes: diffScopes.length,
      diffVariantOperations,
      variantSelectors: 0,
      overrideDensity: 0,
      targetComponents: 0,
    };
  }

  const semanticKeys = flattened.map(componentSemanticKey);
  let withinScopeDuplicates = 0;
  const keyScopeCount = new Map();

  for (const scope of duplicateAnalysisScopes) {
    const keys = scope.components.map(componentSemanticKey);
    const uniqueInScope = new Set(keys);
    withinScopeDuplicates += Math.max(0, keys.length - uniqueInScope.size);
    for (const key of uniqueInScope) {
      keyScopeCount.set(key, (keyScopeCount.get(key) || 0) + 1);
    }
  }

  let crossScopeRepeats = 0;
  for (const count of keyScopeCount.values()) {
    crossScopeRepeats += Math.max(0, count - 1);
  }

  const scopeFingerprintCounts = new Map();
  for (const scope of duplicateAnalysisScopes) {
    scopeFingerprintCounts.set(
      scope.scopeFingerprint,
      (scopeFingerprintCounts.get(scope.scopeFingerprint) || 0) + 1
    );
  }
  let exactDuplicateScopes = 0;
  for (const count of scopeFingerprintCounts.values()) {
    exactDuplicateScopes += Math.max(0, count - 1);
  }

  let nearDuplicateScopes = 0;
  const lengthBuckets = new Map();
  for (const scope of duplicateAnalysisScopes) {
    const len = scope.originalComponents.length;
    if (!lengthBuckets.has(len)) lengthBuckets.set(len, []);
    lengthBuckets.get(len).push(scope);
  }

  for (const bucket of lengthBuckets.values()) {
    for (let index = 0; index < bucket.length; index += 1) {
      for (let compareIndex = index + 1; compareIndex < bucket.length; compareIndex += 1) {
        const left = bucket[index];
        const right = bucket[compareIndex];
        if (left.scopeFingerprint === right.scopeFingerprint) continue;
        const distance = scopeDistance(left.originalComponents, right.originalComponents);
        if (distance > 0 && distance <= 2) {
          nearDuplicateScopes += 1;
        }
      }
    }
  }

  const patternCounts = new Map();
  for (const scope of duplicateAnalysisScopes) {
    const uniquePatterns = new Set(scope.patternFingerprints);
    for (const fingerprint of uniquePatterns) {
      patternCounts.set(fingerprint, (patternCounts.get(fingerprint) || 0) + 1);
    }
  }
  let repeatedPatterns = 0;
  for (const count of patternCounts.values()) {
    repeatedPatterns += Math.max(0, count - 1);
  }

  const weightedDuplicates = withinScopeDuplicates
    + (crossScopeRepeats * 0.35)
    + (exactDuplicateScopes * 2)
    + (nearDuplicateScopes * 1.25)
    + (repeatedPatterns * 0.15);
  const duplicateRatio = weightedDuplicates / Math.max(semanticKeys.length, 1);
  const overrideCount = flattened.reduce(
    (sum, component) => sum + Object.keys(component.overrides || {}).length,
    0
  );
  const overrideDensity = overrideCount / flattened.length;
  const typeTemplateCoverage = [
    styleData?.bibliography?.['type-templates'] || {},
    styleData?.bibliography?.['type-variants'] || {},
  ].reduce((total, map) => {
    return total + Object.keys(map).reduce((sum, rawKey) => {
      const parsed = parseOverrideKey(rawKey).filter((key) => key !== 'default');
      return sum + (parsed.length || 1);
    }, 0);
  }, 0);

  const componentTargets = {
    'author-date': 52,
    numeric: 55,
    note: 65,
  };
  const targetBase = componentTargets[format] || 55;
  const targetBonus = clamp(0, 120, Math.max(0, variantSelectorCount - 4) * 4);
  const typeTemplateBonus = clamp(0, 35, Math.max(0, typeTemplateCoverage - 3) * 2.5);
  const target = targetBase + targetBonus + typeTemplateBonus;
  const componentPenalty = Math.max(0, flattened.length - target) * 0.15;
  const duplicatePenalty = duplicateRatio * 10;
  const sprawlPenalty = Math.max(0, variantSelectorCount - 18) * 0.8;
  const exactDuplicatePenalty = exactDuplicateScopes * 4;
  const nearDuplicatePenalty = nearDuplicateScopes * 1.5;
  const patternPenalty = repeatedPatterns * 0.05;
  const overridePenalty = Math.max(0, overrideDensity - 1.5) * 12;
  const score = 100
    - componentPenalty
    - duplicatePenalty
    - sprawlPenalty
    - exactDuplicatePenalty
    - nearDuplicatePenalty
    - patternPenalty
    - overridePenalty;

  return {
    score: safePct(score),
    scopeCount: scopedComponents.length,
    totalComponents: flattened.length,
    duplicates: parseFloat(weightedDuplicates.toFixed(1)),
    withinScopeDuplicates,
    crossScopeRepeats,
    exactDuplicateScopes,
    nearDuplicateScopes,
    repeatedPatterns,
    diffVariantScopes: diffScopes.length,
    diffVariantOperations,
    variantSelectors: variantSelectorCount,
    overrideDensity: parseFloat(overrideDensity.toFixed(2)),
    targetComponents: parseFloat(target.toFixed(1)),
  };
}

function computePresetUsageScore(styleData, concisionScore) {
  const templateUses = countTemplatePresetUses(styleData);
  const { uses: optionUses, fields: optionPresetFields } = countOptionsPresetUses(styleData);
  const hasInheritedPreset = hasRootExtends(styleData);
  const hasAuthoredTemplateScopes = collectTemplateScopes(styleData).scopes.length > 0;

  if (hasInheritedPreset && !hasAuthoredTemplateScopes) {
    const uses = templateUses + optionUses;
    return {
      score: 100,
      uses,
      templateUses,
      optionUses,
      weightedUses: 5 + optionUses,
      optionPresetFields,
      inheritedPreset: styleData.extends,
      note: `root extends: ${styleData.extends}`,
    };
  }

  const weightedUses = (templateUses * 2) + optionUses;
  const uses = templateUses + optionUses;

  if (weightedUses >= 5) {
    return { score: 100, uses, templateUses, optionUses, weightedUses, optionPresetFields };
  }
  if (weightedUses >= 3) {
    return { score: 90, uses, templateUses, optionUses, weightedUses, optionPresetFields };
  }
  if (weightedUses >= 2) {
    return { score: 80, uses, templateUses, optionUses, weightedUses, optionPresetFields };
  }
  if (weightedUses >= 1) {
    return { score: 70, uses, templateUses, optionUses, weightedUses, optionPresetFields };
  }

  const baselineScore = concisionScore >= 80 ? 60 : 45;
  return {
    score: baselineScore,
    uses,
    templateUses,
    optionUses,
    weightedUses,
    optionPresetFields,
  };
}

function selectQualityAuthorshipData(authoredStyleData, effectiveStyleData) {
  const authoredScopes = collectTemplateScopes(authoredStyleData).scopes;
  if (authoredScopes.length === 0 && hasRootExtends(authoredStyleData)) {
    return authoredStyleData;
  }
  if (authoredScopes.length === 0 && effectiveStyleData) {
    return effectiveStyleData;
  }
  return authoredStyleData || effectiveStyleData;
}

function computeQualityMetrics(styleSpec, oracleResult, styleYamlPath = null) {
  const loaded = loadStyleYaml(styleSpec.name, styleYamlPath);
  if (!loaded.resolvedStyleData) {
    return {
      score: 0,
      error: loaded.error,
      subscores: {
        typeCoverage: { score: 0 },
        fallbackRobustness: { score: 0 },
        concision: { score: 0 },
        presetUsage: { score: 0 },
      },
    };
  }

  const authoredStyleData = loaded.rawStyleData || loaded.resolvedStyleData;
  const effectiveStyleData = loaded.resolvedStyleData;
  const qualityAuthorshipData = selectQualityAuthorshipData(authoredStyleData, effectiveStyleData);
  const typeCoverage = computeTypeCoverageScore(oracleResult.citationsByType || {});
  let fallbackRobustness = computeFallbackRobustness(effectiveStyleData);
  const concision = computeConcisionScore(qualityAuthorshipData, styleSpec.format);
  const presetUsage = computePresetUsageScore(qualityAuthorshipData, concision.score);
  const weights = {
    typeCoverage: 0.35,
    fallbackRobustness: 0.25,
    concision: 0.25,
    presetUsage: 0.15,
  };

  // Citation-only note styles don't define bibliography templates, so
  // bibliography fallback robustness is not applicable.
  if (styleSpec.hasBibliography === false) {
    fallbackRobustness = {
      score: 100,
      assessedTypes: 0,
      passingTypes: 0,
      note: 'not applicable for citation-only style',
      notApplicable: true,
    };
    weights.fallbackRobustness = 0;
  }

  const weightSum = Object.values(weights).reduce((sum, value) => sum + value, 0);
  const rawScore = (
    (typeCoverage.score * weights.typeCoverage) +
    (fallbackRobustness.score * weights.fallbackRobustness) +
    (concision.score * weights.concision) +
    (presetUsage.score * weights.presetUsage)
  );
  const score = weightSum > 0 ? rawScore / weightSum : 0;

  return {
    score: safePct(score),
    error: null,
    subscores: {
      typeCoverage,
      fallbackRobustness,
      concision,
      presetUsage,
    },
  };
}

function buildPresentationFields(styleSpec, stylePolicy, sufficiencyPolicy) {
  const citationAuthority = resolveScopeAuthority(stylePolicy, 'citation');
  const bibliographyAuthority = resolveScopeAuthority(stylePolicy, 'bibliography');
  const effectiveScopes = getEffectiveVerificationScopes(stylePolicy, styleSpec.hasBibliography);
  const hasBibliographyScope = effectiveScopes.includes('bibliography');
  return {
    cslReach: styleSpec.cslReach,
    dependents: styleSpec.cslReach,
    impactPct: computeImpactPct(styleSpec.cslReach),
    originKey: styleSpec.originKey,
    originLabel: styleSpec.originLabel,
    originSortRank: styleSpec.originSortRank,
    benchmarkSource: normalizeBenchmarkSource(stylePolicy.authority),
    benchmarkAuthorityId: stylePolicy.authorityId,
    benchmarkLabel: formatAuthorityLabel(stylePolicy.authority, stylePolicy.authorityId),
    citationAuthorityLabel: formatAuthorityLabel(citationAuthority.authority, citationAuthority.authorityId),
    bibliographyAuthorityLabel: hasBibliographyScope
      ? formatAuthorityLabel(bibliographyAuthority.authority, bibliographyAuthority.authorityId)
      : null,
    hasScopedAuthorities: hasBibliographyScope && (
      citationAuthority.authority !== bibliographyAuthority.authority
      || citationAuthority.authorityId !== bibliographyAuthority.authorityId
    ),
    secondarySources: stylePolicy.secondary,
    secondarySourceLabels: (stylePolicy.secondary || []).map((authority) => formatAuthorityLabel(authority)),
    regressionBaseline: stylePolicy.regressionBaseline,
    regressionBaselineLabel: stylePolicy.regressionBaseline
      ? formatAuthorityLabel(stylePolicy.regressionBaseline)
      : null,
    verificationScopes: effectiveScopes,
    fixtureFamily: sufficiencyPolicy.family,
    defaultReportSufficient: sufficiencyPolicy.defaultReportSufficient,
    requiredReferenceTypes: sufficiencyPolicy.requiredReferenceTypes,
    requiredScenarios: sufficiencyPolicy.requiredScenarios,
    fixtureSets: sufficiencyPolicy.fixtureSets,
    benchmarkRuns: stylePolicy.benchmarkRuns || [],
    verificationNote: stylePolicy.note,
    inheritance: styleSpec.inheritance,
    registry: styleSpec.registry,
    measurementEvidence: styleSpec.measurementEvidence,
  };
}

function preflightSnapshots(coreStyles, verificationPolicy, stylesDir) {
  const issues = [];

  for (const styleSpec of coreStyles) {
    const stylePolicy = resolveVerificationPolicy(styleSpec.name, verificationPolicy);
    const citationAuthority = resolveScopeAuthority(stylePolicy, 'citation');
    if (citationAuthority.authority !== 'citeproc-js') continue;

    const stylePath = path.join(stylesDir, `${styleSpec.sourceName}.csl`);
    if (!fs.existsSync(stylePath)) continue;
    const citationsFixture = styleSpec.format === 'note' ? NOTE_CITATIONS_FIXTURE : DEFAULT_CITATIONS_FIXTURE;
    const snapshotStatus = getCslSnapshotStatus(stylePath, DEFAULT_REFS_FIXTURE, citationsFixture);
    if (!snapshotStatus.ok) {
      issues.push({
        style: styleSpec.name,
        sourceName: styleSpec.sourceName,
        status: snapshotStatus.status,
        message: snapshotStatus.message,
      });
    }
  }

  return issues.sort((left, right) => left.style.localeCompare(right.style));
}

async function processStyleReport(runtime, styleSpec, context) {
  const {
    divergences,
    stylesDir,
    styleYamlOverridePath,
    verificationPolicy,
    fixtureSufficiency,
    noteStyles,
  } = context;
  const stylePolicy = resolveVerificationPolicy(styleSpec.name, verificationPolicy);
  const sufficiencyPolicy = resolveFixtureSufficiency(stylePolicy.fixtureFamily, fixtureSufficiency);
  const primaryComparator = selectPrimaryComparator(styleSpec, stylePolicy);
  const citationAuthority = resolveScopeAuthority(stylePolicy, 'citation');
  const bibliographyAuthority = resolveScopeAuthority(stylePolicy, 'bibliography');

  if (primaryComparator === 'citum-baseline') {
    const oracleResult = tagOracleResultEvidence(
      await runNativeOracle(runtime, styleSpec.name),
      {
        id: 'baseline',
        label: 'Core compatibility baseline',
        authority: 'citum-baseline',
      }
    );
    const pairingSummary = summarizeBibliographyPairing(oracleResult);
    const fidelityScore = computeFidelityScore(oracleResult, styleSpec.hasBibliography);
    const caseMismatches = collectCaseMismatchSummary(oracleResult);
    const exactParity = summarizeExactParity(oracleResult, styleSpec.hasBibliography);
    const bibliography = {
      ...getEffectiveOracleSection(oracleResult, 'bibliography'),
      unresolvedPairing: pairingSummary.unresolvedUnpaired,
    };
    const citations = getEffectiveOracleSection(oracleResult, 'citations');
    const rawBibliography = oracleResult.bibliography || { passed: 0, total: 0 };
    const rawCitations = oracleResult.citations || { passed: 0, total: 0 };
    const qualityMetrics = computeQualityMetrics(styleSpec, oracleResult, styleYamlOverridePath);
    const qualityScore = qualityMetrics.score / 100;
    let statusTier = 'failing';
    if (oracleResult.error) {
      statusTier = 'error';
    } else if (fidelityScore === 1.0) {
      statusTier = 'perfect';
    } else if (fidelityScore > 0) {
      statusTier = 'partial';
    }

    const notePositionAudit = styleSpec.format === 'note' && noteStyles.has(styleSpec.name)
      ? auditNoteStyle(noteStyles.get(styleSpec.name), { citumBin: runtime.citumBin })
      : null;

    return {
      styleRecord: {
        name: styleSpec.name,
        format: styleSpec.format,
        hasBibliography: styleSpec.hasBibliography,
        ...buildPresentationFields(styleSpec, stylePolicy, sufficiencyPolicy),
        fidelityScore: parseFloat(fidelityScore.toFixed(3)),
        compatibilityScore: parseFloat(fidelityScore.toFixed(3)),
        citations,
        bibliography,
        rawCitations,
        rawBibliography,
        knownDivergences: divergences[styleSpec.name] || [],
        adjustedDivergences: oracleResult.adjusted?.divergenceSummary || {},
        caseMismatches,
        exactParity,
        pairingSummary,
        citationsByType: oracleResult.citationsByType || {},
        error: oracleResult.error || null,
        componentMatchRate: null,
        statusTier,
        componentSummary: {},
        citationEntries: null,
        oracleDetail: oracleResult.bibliography ? oracleResult.bibliography.entries : null,
        qualityScore: parseFloat(qualityScore.toFixed(3)),
        qualityBreakdown: qualityMetrics,
        oracleSource: 'citum-baseline',
        notePositionAudit,
      },
      errorCount: oracleResult.error ? 1 : 0,
      citations,
      bibliography,
      qualityScore,
    };
  }

  const stylePath = stylePolicy.cslSource
    ? path.join(PROJECT_ROOT, stylePolicy.cslSource)
    : path.join(stylesDir, `${styleSpec.sourceName}.csl`);
  const styleYamlPath = styleYamlOverridePath || styleSpec.stylePath || resolveStyleYamlPath(styleSpec.name);

  if (citationAuthority.authority === 'citeproc-js' && !fs.existsSync(stylePath)) {
    return {
      styleRecord: {
        name: styleSpec.name,
        format: styleSpec.format,
        hasBibliography: styleSpec.hasBibliography,
        ...buildPresentationFields(styleSpec, stylePolicy, sufficiencyPolicy),
        fidelityScore: 0,
        compatibilityScore: 0,
        citations: { passed: 0, total: 0 },
        bibliography: { passed: 0, total: 0 },
        knownDivergences: divergences[styleSpec.name] || [],
        citationsByType: {},
        error: `Style file not found: ${stylePath}`,
        oracleDetail: null,
        qualityScore: 0,
        qualityBreakdown: null,
      },
      errorCount: 1,
      citations: { passed: 0, total: 0 },
      bibliography: { passed: 0, total: 0 },
      qualityScore: 0,
    };
  }

  let oracleResult;
  if (citationAuthority.authority === 'citeproc-js' && bibliographyAuthority.authority === 'biblatex') {
    const [citationResult, bibliographyResult] = await Promise.all([
      runCiteprocSnapshotOracle(runtime, stylePath, styleSpec.name, styleSpec.format, undefined, null, styleYamlPath),
      runBiblatexSnapshotOracle(runtime, styleSpec.name, styleYamlPath, bibliographyAuthority.authorityId),
    ]);
    oracleResult = composeScopedOracleResult(citationResult, bibliographyResult);
  } else if (primaryComparator === 'biblatex') {
    oracleResult = await runBiblatexSnapshotOracle(runtime, styleSpec.name, styleYamlPath, stylePolicy.authorityId);
  } else {
    oracleResult = await runCiteprocSnapshotOracle(runtime, stylePath, styleSpec.name, styleSpec.format, undefined, null, styleYamlPath);
  }
  tagOracleResultEvidence(oracleResult, {
    id: 'baseline',
    label: 'Core compatibility baseline',
    authority: formatAuthorityLabel(
      bibliographyAuthority.authority,
      bibliographyAuthority.authorityId
    ),
  });

  const familySets = getAdditionalFixtureSetNames(sufficiencyPolicy.fixtureSets || []);
  if (citationAuthority.authority === 'citeproc-js' && fs.existsSync(stylePath)) {
    for (const setName of familySets) {
      const extra = await runFamilyFixtureOracle(runtime, stylePath, styleSpec.name, setName);
      if (!extra) continue;
      tagOracleResultEvidence(extra, {
        id: `family:${setName}`,
        label: `Family fixture: ${setName}`,
        authority: formatAuthorityLabel(
          bibliographyAuthority.authority,
          bibliographyAuthority.authorityId
        ),
      });
      if (bibliographyAuthority.authority === 'citeproc-js') {
        mergeOracleResults(oracleResult, extra);
      } else {
        mergeCitationResults(oracleResult, extra);
      }
    }
  }

  const benchmarkRunResults = await executeBenchmarkRuns(stylePolicy.benchmarkRuns || [], async (benchmarkRun) =>
    runBenchmarkRun(runtime, styleSpec, stylePath, styleYamlPath, benchmarkRun)
  );
  for (const benchmarkRunResult of benchmarkRunResults) {
    tagOracleResultEvidence(benchmarkRunResult.oracleResult, {
      id: `benchmark:${benchmarkRunResult.id}`,
      label: benchmarkRunResult.label,
      authority: benchmarkRunResult.runner === 'citeproc-oracle'
        ? 'citeproc-js'
        : benchmarkRunResult.runner,
    });
    mergeBenchmarkRunIntoOracle(oracleResult, benchmarkRunResult);
  }
  const benchmarkErrors = benchmarkRunResults
    .filter((benchmarkRunResult) => benchmarkRunResult.error)
    .map((benchmarkRunResult) => `benchmark ${benchmarkRunResult.id}: ${benchmarkRunResult.error}`);
  const publishedBenchmarkRunResults = benchmarkRunResults.map(toPublishedBenchmarkRunRecord);

  const fidelityScore = computeFidelityScore(oracleResult, styleSpec.hasBibliography);
  const caseMismatches = collectCaseMismatchSummary(oracleResult);
  const exactParity = summarizeExactParity(oracleResult, styleSpec.hasBibliography);
  const pairingSummary = summarizeBibliographyPairing(oracleResult);
  const citations = getEffectiveOracleSection(oracleResult, 'citations');
  const bibliography = {
    ...getEffectiveOracleSection(oracleResult, 'bibliography'),
    unresolvedPairing: pairingSummary.unresolvedUnpaired,
  };
  const rawCitations = oracleResult.citations || { passed: 0, total: 0 };
  const rawBibliography = oracleResult.bibliography || { passed: 0, total: 0 };
  const componentMatchRate = computeComponentMatchRate(oracleResult);
  const qualityMetrics = computeQualityMetrics(styleSpec, oracleResult, styleYamlPath);
  const qualityScore = qualityMetrics.score / 100;

  let statusTier = 'failing';
  const combinedStyleError = [oracleResult.error, ...benchmarkErrors].filter(Boolean).join('\n') || null;

  if (combinedStyleError) {
    statusTier = 'error';
  } else if (fidelityScore === 1.0) {
    statusTier = 'perfect';
  } else if (fidelityScore > 0) {
    statusTier = 'partial';
  }

  const notePositionAudit = styleSpec.format === 'note' && noteStyles.has(styleSpec.name)
    ? auditNoteStyle(noteStyles.get(styleSpec.name), { citumBin: runtime.citumBin })
    : null;

  const styleRecord = {
    name: styleSpec.name,
    format: styleSpec.format,
    hasBibliography: styleSpec.hasBibliography,
    ...buildPresentationFields(styleSpec, stylePolicy, sufficiencyPolicy),
    fidelityScore: parseFloat(fidelityScore.toFixed(3)),
    compatibilityScore: parseFloat(fidelityScore.toFixed(3)),
    citations,
    bibliography,
    rawCitations,
    rawBibliography,
    knownDivergences: divergences[styleSpec.name] || [],
    adjustedDivergences: oracleResult.adjusted?.divergenceSummary || {},
    caseMismatches,
    exactParity,
    pairingSummary,
    citationsByType: oracleResult.citationsByType || {},
    error: combinedStyleError,
    componentMatchRate,
    statusTier,
    componentSummary: oracleResult.componentSummary || {},
    citationEntries: oracleResult.citations ? oracleResult.citations.entries : null,
    oracleDetail: oracleResult.bibliography ? oracleResult.bibliography.entries : null,
    benchmarkRunResults: publishedBenchmarkRunResults,
    qualityScore: parseFloat(qualityScore.toFixed(3)),
    qualityBreakdown: qualityMetrics,
    oracleSource: oracleResult.oracleSource || primaryComparator,
    styleYamlPath: toRepoRelativePath(styleYamlPath) || styleYamlPath,
    notePositionAudit,
  };
  styleRecord.richInputEvidence = buildRichInputEvidenceSummary(styleRecord, publishedBenchmarkRunResults);

  return {
    styleRecord,
    errorCount: combinedStyleError ? 1 : 0,
    citations,
    bibliography,
    qualityScore,
  };
}

/**
 * Generate compatibility report
 */
async function generateReport(options) {
  const stylesDir = getStylesDir(options.stylesDir);
  const provenanceConfig = loadReportProvenance();
  const discoveredStyles = discoverCoreStyles(provenanceConfig);
  const coreStyles = options.styleName
    ? discoveredStyles.filter((style) => style.name === options.styleName)
    : resolveSelectedStyles(discoveredStyles, options.styles);
  if (options.styleName && coreStyles.length === 0) {
    throw new Error(`Core style not found for --style: ${options.styleName}`);
  }
  const divergences = loadDivergences();
  const verificationPolicy = loadVerificationPolicy();
  const fixtureSufficiency = loadFixtureSufficiency();
  const generated = getTimestamp();
  const gitCommit = getGitCommit();
  const runtime = createReportRuntime(options);
  const preflightIssues = preflightSnapshots(coreStyles, verificationPolicy, stylesDir);
  const noteStyles = buildNoteStyleLookup();

  if (!runtime.allowLiveFallback && preflightIssues.length > 0) {
    for (const issue of preflightIssues) {
      process.stderr.write(`[snapshot ${issue.status}] ${issue.style}: ${issue.message}\n`);
    }
  }

  const styleJobs = await mapWithConcurrency(coreStyles, options.parallelism || DEFAULT_PARALLELISM, async (styleSpec) => {
    const result = await processStyleReport(runtime, styleSpec, {
      divergences,
      stylesDir,
      styleYamlOverridePath: options.styleFile,
      verificationPolicy,
      fixtureSufficiency,
      noteStyles,
    });
    if (result.styleRecord.error) {
      process.stderr.write(`Error processing ${styleSpec.name}: ${result.styleRecord.error}\n`);
    }
    return result;
  });

  const styles = styleJobs
    .map((job) => job.styleRecord)
    .sort((left, right) => left.name.localeCompare(right.name));
  let citationsTotal = 0;
  let citationsPassed = 0;
  let biblioTotal = 0;
  let biblioPassed = 0;
  let citationCaseMismatchTotal = 0;
  let bibliographyCaseMismatchTotal = 0;
  let exactParityPassed = 0;
  let exactParityTotal = 0;
  let exactParityNotComparable = 0;
  const pairingOverall = {
    paired: 0,
    unresolvedUnpaired: 0,
    idProvenOracleOnly: 0,
    idProvenCitumOnly: 0,
    totalObservations: 0,
  };
  let qualityTotal = 0;
  let qualityCount = 0;
  let errorCount = 0;

  for (const job of styleJobs) {
    const citations = job.citations || { passed: 0, total: 0 };
    const bibliography = job.bibliography || { passed: 0, total: 0 };
    citationsTotal += citations.total || 0;
    citationsPassed += citations.passed || 0;
    // Exclude bibliography totals for note styles: the oracle returns 0/N
    // results for bibliography scenarios even when a style has no bibliography,
    // which would inflate the denominator of the portfolio-level bib rate.
    if (job.styleRecord.hasBibliography !== false) {
      biblioTotal += bibliography.total || 0;
      biblioPassed += bibliography.passed || 0;
    }
    citationCaseMismatchTotal += job.styleRecord.caseMismatches?.citations || 0;
    bibliographyCaseMismatchTotal += job.styleRecord.caseMismatches?.bibliography || 0;
    exactParityPassed += job.styleRecord.exactParity?.passed || 0;
    exactParityTotal += job.styleRecord.exactParity?.total || 0;
    exactParityNotComparable += job.styleRecord.exactParity?.notComparable || 0;
    for (const key of Object.keys(pairingOverall)) {
      pairingOverall[key] += job.styleRecord.pairingSummary?.[key] || 0;
    }
    qualityTotal += job.qualityScore || 0;
    qualityCount += 1;
    errorCount += job.errorCount || 0;
  }

  const knownDependents = coreStyles
    .filter((s) => typeof s.cslReach === 'number')
    .reduce((sum, s) => sum + s.cslReach, 0);
  const totalImpact = ((knownDependents / TOTAL_DEPENDENTS) * 100).toFixed(2);
  const families = buildFamilyAggregates(styles);

  return {
    report: {
      generated,
      commit: gitCommit,
      source: 'scripts/report-core.js',
      metadata: {
        timestamp: generated,
        gitCommit,
        fixture: 'tests/fixtures/references-expanded.json',
        styleSelector: options.styleName
          ? `style:${options.styleName}`
          : options.styles?.length
            ? 'selected-styles'
            : 'core-styles',
        styleYamlOverride: options.styleFile ? toRepoRelativePath(options.styleFile) || options.styleFile : null,
        styles: coreStyles.map((style) => style.name),
        generator: 'scripts/report-core.js',
        richInputEvidence: {
          status: 'official-supplemental',
          headlineGate: 'baseline-fixtures',
        },
        extraFixtures: [
          'tests/fixtures/citations-note-expanded.json',
          'tests/fixtures/references-note-positions.json',
          'tests/fixtures/citations-note-positions.json',
        ],
        parallelism: options.parallelism || DEFAULT_PARALLELISM,
        cacheDir: runtime.cacheDir,
        preflight: {
          snapshotIssues: preflightIssues,
          allowLiveFallback: runtime.allowLiveFallback,
        },
        oracleComparison: {
          caseSensitive: runtime.caseSensitive,
          exactTextParity: 'informational-pre-divergence',
        },
        measurementEvidenceSources: coreStyles[0]?.measurementEvidenceSources || {
          behavioralBands: null,
          derivability: null,
        },
        ...(options.timings ? { timings: serializeTimingSummary(runtime) } : {}),
      },
      totalImpact: parseFloat(totalImpact),
      totalStyles: coreStyles.length,
      citationsOverall: { passed: citationsPassed, total: citationsTotal },
      bibliographyOverall: {
        passed: biblioPassed,
        total: biblioTotal,
        unresolvedPairing: pairingOverall.unresolvedUnpaired,
      },
      caseMismatchesOverall: {
        citations: citationCaseMismatchTotal,
        bibliography: bibliographyCaseMismatchTotal,
        total: citationCaseMismatchTotal + bibliographyCaseMismatchTotal,
      },
      exactParityOverall: {
        passed: exactParityPassed,
        total: exactParityTotal,
        notComparable: exactParityNotComparable,
        rate: exactParityTotal > 0
          ? parseFloat((exactParityPassed / exactParityTotal).toFixed(3))
          : null,
        status: 'unadjudicated',
        gating: false,
      },
      pairingOverall,
      qualityOverall: {
        score: qualityCount > 0 ? parseFloat((qualityTotal / qualityCount).toFixed(3)) : 0,
      },
      styles,
      families,
    },
    errorCount
  };
}

/**
 * Generate HTML dashboard
 */
function generateHtml(report) {
  const headerHtml = generateHtmlHeader(report);
  const statsHtml = generateHtmlStats(report);
  const sqiExplainerHtml = generateHtmlSqiExplainer();
  const tableHtml = generateHtmlTable(report);
  const footerHtml = generateHtmlFooter();

  return `${headerHtml}${statsHtml}${sqiExplainerHtml}${tableHtml}${footerHtml}`;
}

function generateHtmlHeader(report) {
  const generatedDate = new Date(report.generated).toUTCString();
  return `<!-- Auto-generated by report-core.js. Do not edit manually. -->
<!DOCTYPE html>
<html lang="en" class="scroll-smooth">

<head>
    <meta charset="utf-8" />
    <meta content="width=device-width, initial-scale=1.0" name="viewport" />
    <title>Citum | Style Compatibility Report</title>
    <meta name="description"
        content="Compatibility metrics for Citum against declared style authority sources.">

    <script src="https://cdn.tailwindcss.com?plugins=forms,container-queries,typography"></script>
    <link
        href="https://fonts.googleapis.com/css2?family=Libre+Franklin:wght@300;400;500;600;700&amp;family=JetBrains+Mono:wght@400;500;700&amp;family=Newsreader:ital,opsz,wght@0,6..72,200..800;1,6..72,200..800&amp;display=swap"
        rel="stylesheet" />
    <link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet" />

    <script>
        tailwind.config = {
            darkMode: "class",
            theme: {
                extend: {
                    colors: {
                        "primary": "#2a94d6",
                        "background-light": "#fdfbf7",
                        "accent-cream": "#f5f2eb",
                    },
                    fontFamily: {
                        "display": ["Libre Franklin", "sans-serif"],
                        "mono": ["JetBrains Mono", "monospace"]
                    },
                    borderRadius: {
                        "DEFAULT": "0.25rem",
                        "lg": "0.5rem",
                        "xl": "0.75rem",
                        "full": "9999px"
                    },
                },
            },
        }
    </script>
    <style type="text/tailwindcss">
        body {
            font-family: 'Libre Franklin', sans-serif;
            color: #374151;
        }
        .font-mono {
            font-family: 'JetBrains Mono', monospace;
        }
        .glass-nav {
            background: rgba(253, 251, 247, 0.85);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid rgba(42, 148, 214, 0.1);
        }
        .accordion-toggle {
            cursor: pointer;
            user-select: none;
        }
        .accordion-content {
            display: none;
        }
        .accordion-content.active {
            display: table-row;
        }
        .badge-perfect {
            background-color: rgba(16, 185, 129, 0.1);
            color: #047857;
        }
        .badge-partial {
            background-color: rgba(251, 191, 36, 0.1);
            color: #92400e;
        }
        .badge-failing {
            background-color: rgba(239, 68, 68, 0.1);
            color: #7f1d1d;
        }
        .badge-pending {
            background-color: rgba(148, 163, 184, 0.1);
            color: #475569;
        }
    </style>
    <link rel="stylesheet" href="assets/citum-theme.css">
</head>

<body class="bg-background-light text-slate-700 selection:bg-primary/20">

    <!-- Navigation -->
    <nav class="fixed top-0 w-full z-50 glass-nav">
        <div class="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
            <div class="flex items-center gap-2 shrink-0">
                <a href="index.html" class="flex items-center gap-2 group">
                    <div class="w-8 h-8 bg-primary rounded flex items-center justify-center group-hover:brightness-110 transition-all">
                        <span class="text-white font-mono font-bold">C</span>
                    </div>
                    <span class="font-mono text-xl font-bold tracking-tight text-slate-900">Citum</span>
                </a>
            </div>
            <div class="hidden md:flex items-center gap-3 lg:gap-4 xl:gap-6 min-w-0 overflow-x-auto whitespace-nowrap pl-4">
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="https://citum.org">Home</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="index.html">Docs</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="demo.html">Demo</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="examples.html">Examples</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="guides/style-author-guide.html">Style Guide</a>
                <a class="text-sm font-medium text-primary font-semibold"
                    href="reports.html">Reports</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="https://github.com/citum/citum-core">GitHub</a>
            </div>
            <button type="button"
                class="md:hidden inline-flex items-center justify-center rounded-lg border border-slate-200 bg-[var(--citum-surface)]/80 px-3 py-2 text-slate-700"
                data-nav-toggle aria-expanded="false" aria-controls="mobile-nav">
                <span class="material-icons text-[20px]">menu</span>
            </button>
        </div>
        <div id="mobile-nav" class="md:hidden hidden border-t border-slate-200 bg-background-light/95 px-6 py-4" data-mobile-menu>
            <div class="flex flex-col gap-3 text-sm font-medium text-slate-700">
                <a class="hover:text-primary transition-colors" href="https://citum.org">Home</a>
                <a class="hover:text-primary transition-colors" href="index.html">Docs</a>
                <a class="hover:text-primary transition-colors" href="demo.html">Demo</a>
                <a class="hover:text-primary transition-colors" href="examples.html">Examples</a>
                <a class="hover:text-primary transition-colors" href="guides/style-author-guide.html">Style Guide</a>
                <a class="text-primary font-semibold" href="reports.html">Reports</a>
                <a class="hover:text-primary transition-colors" href="https://github.com/citum/citum-core">GitHub</a>
            </div>
        </div>
    </nav>

    <!-- Header Section -->
    <header class="pt-24 pb-12 px-6 border-b border-slate-200">
        <div class="max-w-7xl mx-auto">
            <div class="flex items-center justify-between mb-6">
                <div>
                    <h1 class="text-4xl md:text-5xl font-bold tracking-tight text-slate-900 mb-2">
                        Style Compatibility Report
                    </h1>
                    <p class="text-slate-500">Compatibility metrics for styles in <code>styles/</code></p>
                </div>
            </div>
            <div class="flex flex-col sm:flex-row gap-4 items-start sm:items-center">
                <div class="text-sm text-slate-500 font-mono">Generated: ${generatedDate}</div>
                <div class="inline-flex items-center gap-2 px-3 py-1 rounded bg-slate-100 text-slate-700 text-xs font-mono border border-slate-200">
                    <span class="material-icons text-sm">code</span>
                    <span>${escapeHtml(report.commit)}</span>
                </div>
            </div>
            <div class="mt-5 flex flex-wrap gap-3 text-sm font-medium">
                <a class="text-primary hover:underline" href="compat.html">Compatibility</a>
                <a class="text-slate-600 hover:text-primary transition-colors" href="behavior-report.html">Engine Behavior Coverage</a>
                <a class="text-slate-600 hover:text-primary transition-colors" href="migration-behavior-report.html">Migration Behavior Coverage</a>
            </div>
        </div>
    </header>
`;
}

function generateHtmlStats(report) {
  const citationsPct = report.citationsOverall.total > 0
    ? ((report.citationsOverall.passed / report.citationsOverall.total) * 100).toFixed(1)
    : 0;
  const biblioPct = report.bibliographyOverall.total > 0
    ? ((report.bibliographyOverall.passed / report.bibliographyOverall.total) * 100).toFixed(1)
    : 0;
  const qualityPct = report.qualityOverall
    ? (report.qualityOverall.score * 100).toFixed(1)
    : '0.0';
  const exactParityPct = report.exactParityOverall?.total > 0
    ? ((report.exactParityOverall.passed / report.exactParityOverall.total) * 100).toFixed(1)
    : '0.0';

  return `
    <!-- Statistics Cards -->
    <section class="py-12 px-6 bg-accent-cream">
        <div class="max-w-7xl mx-auto">
            <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-5 gap-6">
                <!-- Core Styles -->
                <div class="bg-[var(--citum-surface)] rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Core Styles</div>
                    <div class="text-3xl font-bold text-slate-900">${report.totalStyles}</div>
                    <div class="text-xs text-slate-400 mt-2">${report.totalImpact}% known CSL dependent coverage</div>
                </div>

                <!-- Citations Overall -->
                <div class="bg-[var(--citum-surface)] rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Citations</div>
                    <div class="text-3xl font-bold text-slate-900">${report.citationsOverall.passed}/${report.citationsOverall.total}</div>
                    <div class="text-xs text-slate-400 mt-2">${citationsPct}% pass rate</div>
                </div>

                <!-- Bibliography Overall -->
                <div class="bg-[var(--citum-surface)] rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Bibliography</div>
                    <div class="text-3xl font-bold text-slate-900">${report.bibliographyOverall.passed}/${report.bibliographyOverall.total}</div>
                    <div class="text-xs text-slate-400 mt-2">${biblioPct}% pass rate · ${report.bibliographyOverall.unresolvedPairing || 0} unresolved pairing candidates excluded</div>
                </div>

                <!-- Unadjudicated Oracle Text Parity -->
                <div class="bg-[var(--citum-surface)] rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Oracle Text Parity</div>
                    <div class="text-3xl font-bold text-slate-900">${report.exactParityOverall?.passed || 0}/${report.exactParityOverall?.total || 0}</div>
                    <div class="text-xs text-slate-400 mt-2">${exactParityPct}% unadjudicated, non-gating · ${report.exactParityOverall?.notComparable || 0} N/A</div>
                </div>

                <!-- Quality Overall -->
                <div class="bg-[var(--citum-surface)] rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Quality (SQI)</div>
                    <div class="text-3xl font-bold text-slate-900">${qualityPct}%</div>
                    <div class="text-xs text-slate-400 mt-2">Type coverage, fallback, concision, presets</div>
                </div>
            </div>
        </div>
    </section>
`;
}

function generateHtmlSqiExplainer() {
  return `
    <!-- SQI Explainer -->
    <section class="py-8 px-6">
        <div class="max-w-7xl mx-auto">
            <div class="bg-[var(--citum-surface)] rounded-xl border border-slate-200 p-6">
                <h2 class="text-lg font-semibold text-slate-900 mb-2">How To Read This Report</h2>
                <p class="text-sm text-slate-600 mb-3">
                    <strong>Compatibility</strong> is the existing lenient regression gate; it can tolerate meaningful text-level drift.
                    <strong>Oracle text parity</strong> is the stricter, symmetric comparison of visible renderer text.
                    <strong>SQI</strong> (Style Quality Index) is secondary: it scores maintainability and fallback quality.
                </p>
                <p class="text-sm text-slate-600 mb-4">
                    Current working targets remain <code>&gt;=95% compatibility</code> and <code>&gt;=90 SQI</code>.
                    Oracle text parity is informational until each drift is adjudicated and family-level ratchets are defined.
                </p>
                <p class="text-sm text-slate-600 mb-4">
                    <strong>Lineage</strong> shows the source family a style derives from.
                    <strong>Oracle text parity</strong> preserves numbering, case, punctuation, brackets, and role labels after transport markup is removed.
                    A drift may be a Citum defect, an oracle defect, an intentional divergence, or unresolved; parity alone does not assign fault.
                    Similarity outputs without an established counterpart are excluded from both metrics and appear only as neutral pairing diagnostics.
                    <strong>Authority</strong> shows the declared benchmark used for compatibility checks.
                    <strong>Regression baseline</strong> is listed separately when Citum snapshots are retained only as an internal guardrail.
                    <strong>CSL Reach</strong> is the count of dependent legacy CSL styles for comparable parents and is blank when there is no meaningful CSL analogue.
                </p>
                <a class="text-sm font-medium text-primary hover:underline" href="reference/SQI.md">
                    Read the full SQI definition and scoring details
                </a>
            </div>
        </div>
    </section>
`;
}

function generateHtmlTable(report) {
  let tableRows = '';
  const stylesByName = new Map(report.styles.map((style) => [style.name, style]));
  const families = Array.isArray(report.families) && report.families.length > 0
    ? report.families
    : report.styles.map((style) => ({
      root: style.inheritance?.familyRoot || style.name,
      aggregateCslReach: style.cslReach || 0,
      members: [style.name],
      memberCount: 1,
      aliases: style.registry?.aliases || [],
      aliasCount: style.registry?.aliasCount || 0,
    }));

  for (const family of families) {
    const familyAliasText = family.aliasCount > 0
      ? `${family.aliasCount} alias${family.aliasCount === 1 ? '' : 'es'}`
      : 'no aliases';
    tableRows += `
                <tr class="family-header border-y border-slate-300 bg-slate-100"
                    data-family-root="${escapeHtml(family.root)}">
                    <td colspan="13" class="px-6 py-3">
                        <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
                            <span class="font-semibold text-slate-900">${escapeHtml(family.root)}</span>
                            <span class="text-xs text-slate-500">${family.memberCount} member${family.memberCount === 1 ? '' : 's'}</span>
                            <span class="text-xs text-slate-500">${escapeHtml(familyAliasText)}</span>
                            <span class="text-xs font-mono text-slate-600">aggregate CSL reach ${family.aggregateCslReach}</span>
                        </div>
                    </td>
                </tr>
`;

    for (const styleName of family.members) {
      const style = stylesByName.get(styleName);
      if (!style) continue;
    const compatibilityScore = style.compatibilityScore ?? style.fidelityScore;
    const fidelityPct = (compatibilityScore * 100).toFixed(1);
    const qualityPct = ((style.qualityScore || 0) * 100).toFixed(1);
    const exactParityRate = style.exactParity?.total > 0
      ? style.exactParity.passed / style.exactParity.total
      : -1;
    const citationRate = style.citations.total > 0 ? style.citations.passed / style.citations.total : -1;
    const bibliographyRate = style.hasBibliography && style.bibliography.total > 0
      ? style.bibliography.passed / style.bibliography.total
      : -1;
    const cslReachValue = style.cslReach ?? -1;
    const componentRateValue = style.componentMatchRate !== null ? style.componentMatchRate : -1;

    const sqiScore = style.qualityBreakdown?.score ?? (style.qualityScore || 0) * 100;
    let sqiTier = 'D';
    let sqiTierRank = 1;
    let sqiBadgeClass = 'badge-failing';
    if (style.error || style.qualityBreakdown?.error) {
      sqiTier = 'ERR';
      sqiTierRank = 0;
      sqiBadgeClass = 'badge-pending';
    } else if (sqiScore >= 90) {
      sqiTier = 'A';
      sqiTierRank = 4;
      sqiBadgeClass = 'badge-perfect';
    } else if (sqiScore >= 80) {
      sqiTier = 'B';
      sqiTierRank = 3;
      sqiBadgeClass = 'bg-blue-100 text-blue-700';
    } else if (sqiScore >= 70) {
      sqiTier = 'C';
      sqiTierRank = 2;
      sqiBadgeClass = 'badge-partial';
    }

    const citationBadge = style.citations.total === 0
      ? 'badge-pending'
      : style.citations.passed === style.citations.total
        ? 'badge-perfect'
        : style.citations.passed > 0
          ? 'badge-partial'
          : 'badge-failing';

    const biblioBadge = !style.hasBibliography
      ? 'badge-pending'
      : style.bibliography.passed === style.bibliography.total && style.bibliography.total > 0
        ? 'badge-perfect'
        : style.bibliography.passed > 0
          ? 'badge-partial'
          : 'badge-failing';
    const biblioText = style.hasBibliography
      ? `${style.bibliography.passed}/${style.bibliography.total}`
      : 'N/A';
    const unresolvedPairingText = style.pairingSummary?.unresolvedUnpaired > 0
      ? `<div class="mt-1 text-[10px] text-slate-400">${style.pairingSummary.unresolvedUnpaired} unpaired N/A</div>`
      : '';
    const exactParityBadge = style.exactParity?.total > 0 && style.exactParity.passed === style.exactParity.total
      ? 'badge-perfect'
      : style.exactParity?.passed > 0
        ? 'badge-partial'
        : 'badge-failing';
    const exactParityText = style.exactParity?.total > 0
      ? `${style.exactParity.passed}/${style.exactParity.total}`
      : '—';
    const inheritanceChain = style.inheritance?.chain || [style.name];
    const evidenceBand = style.measurementEvidence?.behavioralBand?.band || 'unavailable';
    const searchText = [
      style.name,
      style.inheritance?.familyRoot,
      style.inheritance?.implementationForm,
      ...(style.registry?.aliases || []),
      ...inheritanceChain,
      evidenceBand,
      style.measurementEvidence?.derivability?.verdict,
    ].filter(Boolean).join(' ').toLowerCase();

    let componentRateHtml = '—';
    if (style.componentMatchRate !== null) {
      const rate = style.componentMatchRate;
      const pct = (rate * 100).toFixed(0);
      let componentBadgeClass = 'bg-red-100 text-red-700';
      if (rate >= 0.9) {
        componentBadgeClass = 'bg-emerald-100 text-emerald-700';
      } else if (rate >= 0.7) {
        componentBadgeClass = 'bg-amber-100 text-amber-700';
      }
      componentRateHtml = `<span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${componentBadgeClass}">${pct}%</span>`;
    }

    const toggleId = `toggle-${style.name}`;
    const contentId = `content-${style.name}`;
    tableRows += `
                <tr class="border-b border-slate-200 hover:bg-slate-50 accordion-toggle"
                    data-toggle="${toggleId}"
                    data-detail-id="${contentId}"
                    data-family-root="${escapeHtml(family.root)}"
                    data-style-name="${escapeHtml(style.name.toLowerCase())}"
                    data-search="${escapeHtml(searchText)}"
                    data-format="${escapeHtml(String(style.format).toLowerCase())}"
                    data-origin="${escapeHtml(String(style.originLabel || '').toLowerCase())}"
                    data-csl-reach="${cslReachValue}"
                    data-citation-rate="${citationRate}"
                    data-bibliography-rate="${bibliographyRate}"
                    data-component-rate="${componentRateValue}"
                    data-fidelity="${compatibilityScore}"
                    data-exact-parity="${exactParityRate}"
                    data-quality="${style.qualityScore || 0}"
                    data-sqi-tier-rank="${sqiTierRank}">
                    <td class="px-6 py-4 text-sm font-medium text-slate-900">
                        <div>${style.name}</div>
                        <div class="mt-1 text-[11px] font-normal text-slate-500">
                            ${escapeHtml(style.inheritance?.implementationForm || 'standalone')} · ${escapeHtml(inheritanceChain.join(' → '))}
                        </div>
                    </td>
                    <td class="hidden md:table-cell px-6 py-4 text-sm text-slate-600">${style.format}</td>
                    <td class="hidden md:table-cell px-6 py-4 text-sm text-slate-600">${escapeHtml(style.originLabel || '—')}</td>
                    <td class="hidden md:table-cell px-6 py-4 text-sm text-slate-500 font-mono">${escapeHtml(style.benchmarkLabel || '—')}</td>
                    <td class="hidden md:table-cell px-6 py-4 text-sm text-slate-600">${style.cslReach ?? '—'}</td>
                    <td class="hidden md:table-cell px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${citationBadge}">
                            ${style.citations.passed}/${style.citations.total}
                        </span>
                    </td>
                    <td class="hidden md:table-cell px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${biblioBadge}">
                            ${biblioText}
                        </span>
                        ${unresolvedPairingText}
                    </td>
                    <td class="hidden md:table-cell px-6 py-4">
                        ${componentRateHtml}
                    </td>
                    <td class="px-6 py-4 text-sm font-mono text-slate-600">${fidelityPct}%</td>
                    <td class="px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${exactParityBadge}">
                            ${exactParityText}
                        </span>
                        ${style.exactParity?.notComparable > 0 ? `<div class="mt-1 text-[10px] text-slate-400">${style.exactParity.notComparable} N/A</div>` : ''}
                    </td>
                    <td class="hidden md:table-cell px-6 py-4 text-sm font-mono text-slate-600">${qualityPct}%</td>
                    <td class="px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${sqiBadgeClass}">
                            ${sqiTier}
                        </span>
                    </td>
                    <td class="px-6 py-4 text-right">
                        <button class="text-slate-500 hover:text-primary text-xs font-medium transition-colors" onclick="toggleAccordion('${contentId}')">
                            <span class="material-icons text-base align-middle">expand_more</span>
                        </button>
                    </td>
                </tr>
                <tr class="accordion-content" id="${contentId}">
                    <td colspan="13" class="px-6 py-4 bg-slate-50">
                        <div class="max-w-4xl">
${generateDetailContent(style)}
                        </div>
                    </td>
                </tr>
    `;
    }
  }

  return `
    <!-- Compatibility Table -->
    <section class="py-12 px-6">
        <div class="max-w-7xl mx-auto">
            <div class="rounded-xl border border-slate-200 overflow-hidden overflow-x-auto">
                <div class="px-6 py-4 bg-slate-50 border-b border-slate-200 flex flex-col sm:flex-row gap-3 sm:items-center sm:justify-between">
                    <label for="style-search" class="text-xs font-semibold text-slate-700">Search Styles</label>
                    <div class="flex items-center gap-3 w-full sm:w-auto">
                        <input
                            id="style-search"
                            type="search"
                            placeholder="Filter by style name..."
                            class="w-full sm:w-80 rounded-md border-slate-300 text-sm focus:border-primary focus:ring-primary"
                        />
                        <span id="style-search-count" class="text-xs text-slate-500 font-mono whitespace-nowrap"></span>
                    </div>
                </div>
                <table class="w-full">
                    <thead class="bg-slate-50 border-b border-slate-200">
                        <tr>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('style-name')">
                                    Style <span class="text-slate-400" id="sort-ind-style-name">↕</span>
                                </button>
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('format')">
                                    Format <span class="text-slate-400" id="sort-ind-format">↕</span>
                                </button>
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('origin')">
                                    Lineage <span class="text-slate-400" id="sort-ind-origin">↕</span>
                                </button>
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                Authority
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('csl-reach')">
                                    CSL Reach <span class="text-slate-400" id="sort-ind-csl-reach">↕</span>
                                </button>
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('citation-rate')">
                                    Citations <span class="text-slate-400" id="sort-ind-citation-rate">↕</span>
                                </button>
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('bibliography-rate')">
                                    Bibliography <span class="text-slate-400" id="sort-ind-bibliography-rate">↕</span>
                                </button>
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('component-rate')">
                                    Components <span class="text-slate-400" id="sort-ind-component-rate">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('fidelity')">
                                    Compatibility <span class="text-slate-400" id="sort-ind-fidelity">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('exact-parity')">
                                    Oracle Text <span class="text-slate-400" id="sort-ind-exact-parity">↕</span>
                                </button>
                            </th>
                            <th class="hidden md:table-cell text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('quality')">
                                    Quality <span class="text-slate-400" id="sort-ind-quality">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('sqi-tier-rank')">
                                    SQI Tier <span class="text-slate-400" id="sort-ind-sqi-tier-rank">↕</span>
                                </button>
                            </th>
                            <th class="px-6 py-4"></th>
                        </tr>
                    </thead>
                    <tbody>
${tableRows}
                    </tbody>
                </table>
            </div>
        </div>
    </section>
`;
}

function getComparisonEntryTexts(entry) {
  return {
    benchmark: entry?.oracle ?? entry?.expected ?? '',
    citum: entry?.citum ?? entry?.actual ?? '',
  };
}

function getRawComparisonEntryTexts(entry) {
  const normalized = getComparisonEntryTexts(entry);
  return {
    benchmark: entry?.rawOracle ?? entry?.rawExpected ?? normalized.benchmark,
    citum: entry?.rawCitum ?? entry?.rawActual ?? normalized.citum,
  };
}

function getExactComparisonEntryTexts(entry) {
  const raw = getRawComparisonEntryTexts(entry);
  const comparison = compareText(raw.benchmark, raw.citum);
  return {
    benchmark: entry?.exactOracle ?? entry?.exactExpected ?? comparison.exactExpected,
    citum: entry?.exactCitum ?? entry?.exactActual ?? comparison.exactActual,
  };
}

function renderTextPreview(text, limit = 120) {
  const value = String(text ?? '');
  if (!value) return '(empty)';
  return escapeHtml(value.length > limit ? `${value.substring(0, limit)}…` : value);
}

function renderInlineTextDifference(benchmarkText, citumText) {
  const benchmark = Array.from(String(benchmarkText ?? ''));
  const citum = Array.from(String(citumText ?? ''));
  let prefixLength = 0;
  while (
    prefixLength < benchmark.length
    && prefixLength < citum.length
    && benchmark[prefixLength] === citum[prefixLength]
  ) {
    prefixLength += 1;
  }

  let suffixLength = 0;
  while (
    suffixLength < benchmark.length - prefixLength
    && suffixLength < citum.length - prefixLength
    && benchmark[benchmark.length - 1 - suffixLength] === citum[citum.length - 1 - suffixLength]
  ) {
    suffixLength += 1;
  }

  const prefixStart = Math.max(0, prefixLength - 48);
  const benchmarkChangeEnd = benchmark.length - suffixLength;
  const citumChangeEnd = citum.length - suffixLength;
  const suffixEnd = Math.min(benchmark.length, benchmarkChangeEnd + 32);
  const commonPrefix = benchmark.slice(prefixStart, prefixLength).join('');
  const commonSuffix = benchmark.slice(benchmarkChangeEnd, suffixEnd).join('');

  const renderSide = (characters, changeEnd, emptyLabel) => {
    if (characters.length === 0) {
      return `<mark class="rounded bg-amber-200 px-0.5 text-slate-900">${escapeHtml(emptyLabel)}</mark>`;
    }
    const changed = characters.slice(prefixLength, Math.min(changeEnd, prefixLength + 72)).join('');
    const changeWasTruncated = changeEnd > prefixLength + 72;
    const before = `${prefixStart > 0 ? '…' : ''}${commonPrefix}`;
    const after = `${commonSuffix}${suffixEnd < benchmark.length ? '…' : ''}`;
    return `${escapeHtml(before)}<mark class="rounded bg-amber-200 px-0.5 text-slate-900">${escapeHtml(changed)}${changeWasTruncated ? '…' : ''}</mark>${escapeHtml(after)}`;
  };

  return {
    benchmark: renderSide(benchmark, benchmarkChangeEnd, 'no benchmark entry'),
    citum: renderSide(citum, citumChangeEnd, 'no Citum entry'),
    position: prefixLength + 1,
  };
}

function bibliographyComparisonState(entry) {
  if (entry?.comparisonState) return entry.comparisonState;
  const { benchmark, citum } = getRawComparisonEntryTexts(entry);
  if (benchmark && citum) return 'paired';
  if (entry?.pairingMethod === 'id') return benchmark ? 'oracle-only' : 'citum-only';
  return 'unresolved-unpaired';
}

function groupBibliographyEvidence(entries, fallbackAuthority) {
  const groups = new Map();
  for (const entry of entries || []) {
    const id = entry.evidenceRunId || 'unattributed';
    const group = groups.get(id) || {
      id,
      label: entry.evidenceRunLabel || 'Unattributed comparison evidence',
      authority: entry.evidenceAuthority || fallbackAuthority,
      entries: [],
    };
    group.entries.push(entry);
    groups.set(id, group);
  }
  return [...groups.values()];
}

function renderPairedBibliographyTable(entries) {
  if (entries.length === 0) return '';
  let rows = '';
  for (const entry of entries) {
    const matchIcon = entry.match === true ? '✓' : '✗';
    const matchColor = entry.match === true ? 'text-emerald-600' : 'text-red-600';
    const rawTexts = getRawComparisonEntryTexts(entry);
    const exactMatch = typeof entry.exactMatch === 'boolean'
      ? entry.exactMatch
      : compareText(rawTexts.benchmark, rawTexts.citum).exactMatch;
    const exactIcon = exactMatch ? '✓' : '✗';
    const exactColor = exactMatch ? 'text-emerald-600' : 'text-amber-700';
    const texts = exactMatch ? getComparisonEntryTexts(entry) : getExactComparisonEntryTexts(entry);
    const diff = exactMatch
      ? null
      : renderInlineTextDifference(texts.benchmark, texts.citum);
    const benchmarkText = diff?.benchmark ?? renderTextPreview(texts.benchmark);
    const citumText = diff?.citum ?? renderTextPreview(texts.citum);

    let issuesText = '—';
    if (entry.match && !exactMatch) {
      issuesText = `Unresolved Oracle Drift${diff ? ` (Δ${diff.position})` : ''}`;
    } else if (entry.match === false && entry.issues?.length > 0) {
      issuesText = entry.issues
        .map((issue) => issue.component ? `${issue.component}:${issue.issue}` : issue.issue)
        .join(', ');
    }

    rows += `
                                            <tr class="border-b border-slate-200 hover:bg-slate-50">
                                                <td class="px-2 py-1 text-slate-600">${escapeHtml(entry.id || entry.index || '—')}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(texts.benchmark)}">${benchmarkText}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(texts.citum)}">${citumText}</td>
                                                <td class="px-2 py-1 text-center font-bold ${matchColor}">${matchIcon}</td>
                                                <td class="px-2 py-1 text-center font-bold ${exactColor}">${exactIcon}</td>
                                                <td class="px-2 py-1 text-slate-600 text-xs font-mono">${escapeHtml(issuesText)}</td>
                                            </tr>
`;
  }

  return `
                                <div class="overflow-x-auto">
                                    <table class="w-full text-xs border-collapse">
                                        <thead>
                                            <tr class="border-b border-slate-300 bg-slate-100">
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Item</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Benchmark</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Citum</th>
                                                <th class="text-center px-2 py-1 font-medium text-slate-700">Compatibility</th>
                                                <th class="text-center px-2 py-1 font-medium text-slate-700">Oracle Text</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Issues</th>
                                            </tr>
                                        </thead>
                                        <tbody>${rows}
                                        </tbody>
                                    </table>
                                </div>
`;
}

function renderIdProvenCardinalityFailures(entries) {
  if (entries.length === 0) return '';
  const rows = entries.map((entry) => {
    const state = bibliographyComparisonState(entry);
    const texts = getExactComparisonEntryTexts(entry);
    const oracleOnly = state === 'oracle-only';
    const output = oracleOnly ? texts.benchmark : texts.citum;
    const side = oracleOnly ? 'Oracle only' : 'Citum only';
    return `
                                            <tr class="border-b border-red-100">
                                                <td class="px-2 py-1 font-mono text-slate-600">${escapeHtml(entry.id || '—')}</td>
                                                <td class="px-2 py-1 text-slate-700">${side}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(output)}">${renderTextPreview(output)}</td>
                                                <td class="px-2 py-1 text-center font-bold text-red-600">✗</td>
                                                <td class="px-2 py-1 text-center font-medium text-slate-400">N/A</td>
                                            </tr>
`;
  }).join('');

  return `
                                <div class="mt-4">
                                    <div class="text-xs font-semibold text-red-700 mb-1">ID-proven output cardinality failures (${entries.length})</div>
                                    <div class="text-xs text-slate-500 mb-2">The same item ID was emitted by only one renderer. This affects compatibility, but no text-parity comparison exists.</div>
                                    <div class="overflow-x-auto">
                                        <table class="w-full text-xs border-collapse">
                                            <thead>
                                                <tr class="border-b border-red-200 bg-red-50">
                                                    <th class="text-left px-2 py-1 font-medium text-slate-700">Item ID</th>
                                                    <th class="text-left px-2 py-1 font-medium text-slate-700">Observed side</th>
                                                    <th class="text-left px-2 py-1 font-medium text-slate-700">Output</th>
                                                    <th class="text-center px-2 py-1 font-medium text-slate-700">Compatibility</th>
                                                    <th class="text-center px-2 py-1 font-medium text-slate-700">Oracle Text</th>
                                                </tr>
                                            </thead>
                                            <tbody>${rows}
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
`;
}

function renderUnresolvedPairingDiagnostics(entries) {
  if (entries.length === 0) return '';
  const rows = entries.map((entry) => {
    const texts = getExactComparisonEntryTexts(entry);
    const benchmarkSide = Boolean(texts.benchmark);
    const output = benchmarkSide ? texts.benchmark : texts.citum;
    return `
                                                <tr class="border-b border-slate-200">
                                                    <td class="px-2 py-1 text-slate-600">${benchmarkSide ? 'Oracle candidate' : 'Citum candidate'}</td>
                                                    <td class="px-2 py-1 font-mono text-slate-500 text-xs" title="${escapeHtml(output)}">${renderTextPreview(output)}</td>
                                                    <td class="px-2 py-1 text-center font-medium text-slate-400">N/A</td>
                                                </tr>
`;
  }).join('');

  return `
                                <details class="mt-4 rounded border border-slate-200 bg-slate-100/70">
                                    <summary class="cursor-pointer px-3 py-2 text-xs font-semibold text-slate-600">Unpaired outputs—pairing unresolved (${entries.length})</summary>
                                    <div class="px-3 pb-3">
                                        <p class="mb-2 text-xs text-slate-500">Similarity pairing could not establish counterparts. These candidates are excluded from compatibility and oracle-text parity.</p>
                                        <div class="overflow-x-auto">
                                            <table class="w-full text-xs border-collapse">
                                                <thead>
                                                    <tr class="border-b border-slate-300 bg-slate-200/70">
                                                        <th class="text-left px-2 py-1 font-medium text-slate-600">Observed side</th>
                                                        <th class="text-left px-2 py-1 font-medium text-slate-600">Candidate output</th>
                                                        <th class="text-center px-2 py-1 font-medium text-slate-600">Comparison</th>
                                                    </tr>
                                                </thead>
                                                <tbody>${rows}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                </details>
`;
}

function renderBibliographyEvidence(style) {
  if (!style.oracleDetail?.length) return '';
  const fallbackAuthority = style.bibliographyAuthorityLabel || style.benchmarkLabel || 'configured authority';
  const groups = groupBibliographyEvidence(style.oracleDetail, fallbackAuthority);
  const summary = style.pairingSummary || {};
  let html = `
                            <div class="mt-4">
                                <div class="text-xs font-semibold text-slate-900 mb-1">Bibliography Evidence</div>
                                <div class="text-xs text-slate-500 mb-3">${summary.paired || 0} paired · ${summary.unresolvedUnpaired || 0} unresolved-unpaired · ${(summary.idProvenOracleOnly || 0) + (summary.idProvenCitumOnly || 0)} ID-proven one-sided</div>
`;

  for (const group of groups) {
    const paired = group.entries.filter((entry) => bibliographyComparisonState(entry) === 'paired');
    const cardinalityFailures = group.entries.filter((entry) => {
      const state = bibliographyComparisonState(entry);
      return entry.pairingMethod === 'id' && (state === 'oracle-only' || state === 'citum-only');
    });
    const unresolved = group.entries.filter((entry) => bibliographyComparisonState(entry) === 'unresolved-unpaired');
    html += `
                                <section class="mb-5 rounded border border-slate-200 bg-[var(--citum-surface)] p-3">
                                    <div class="mb-1 text-xs font-semibold text-slate-800">${escapeHtml(group.label)}</div>
                                    <div class="mb-3 text-[11px] text-slate-500">Authority: ${escapeHtml(group.authority || fallbackAuthority)} · ${paired.length} paired · ${unresolved.length} unresolved · ${cardinalityFailures.length} ID-proven one-sided</div>
                                    ${renderPairedBibliographyTable(paired)}
                                    ${renderIdProvenCardinalityFailures(cardinalityFailures)}
                                    ${renderUnresolvedPairingDiagnostics(unresolved)}
                                </section>
`;
  }

  html += `
                            </div>
`;
  return html;
}

function generateDetailContent(style) {
  let html = '';
  const secondaryLabels = Array.isArray(style.secondarySourceLabels) && style.secondarySourceLabels.length > 0
    ? style.secondarySourceLabels.join(', ')
    : '—';
  const cslReachText = style.cslReach != null ? String(style.cslReach) : '—';
  const inheritanceChain = style.inheritance?.chain?.join(' → ') || style.name;
  const aliasText = style.registry?.aliases?.length > 0
    ? style.registry.aliases.join(', ')
    : '—';
  const band = style.measurementEvidence?.behavioralBand;
  const derivability = style.measurementEvidence?.derivability;
  const chainWarning = style.inheritance?.cycle
    ? `Cycle: ${style.inheritance.cycle.join(' → ')}`
    : style.inheritance?.missingParent
      ? `Missing parent: ${style.inheritance.missingParent}`
      : null;

  html += `
                            <div class="mb-4 p-3 rounded border border-slate-200 bg-[var(--citum-surface)]">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Verification Context</div>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                                    <div><span class="font-semibold text-slate-700">Lineage:</span> <span class="font-mono text-slate-600">${escapeHtml(style.originLabel || '—')}</span></div>
                                    <div><span class="font-semibold text-slate-700">Authority:</span> <span class="font-mono text-slate-600">${escapeHtml(style.benchmarkLabel || '—')}</span></div>
                                    ${style.hasScopedAuthorities ? `<div><span class="font-semibold text-slate-700">Citation authority:</span> <span class="font-mono text-slate-600">${escapeHtml(style.citationAuthorityLabel || '—')}</span></div>` : ''}
                                    ${style.hasScopedAuthorities ? `<div><span class="font-semibold text-slate-700">Bibliography authority:</span> <span class="font-mono text-slate-600">${escapeHtml(style.bibliographyAuthorityLabel || '—')}</span></div>` : ''}
                                    <div><span class="font-semibold text-slate-700">Secondary:</span> <span class="font-mono text-slate-600">${escapeHtml(secondaryLabels)}</span></div>
                                    ${style.regressionBaselineLabel ? `<div><span class="font-semibold text-slate-700">Regression baseline:</span> <span class="font-mono text-slate-600">${escapeHtml(style.regressionBaselineLabel)}</span></div>` : ''}
                                    <div><span class="font-semibold text-slate-700">CSL Reach:</span> <span class="font-mono text-slate-600">${escapeHtml(cslReachText)}</span></div>
                                    <div><span class="font-semibold text-slate-700">Family:</span> <span class="font-mono text-slate-600">${escapeHtml(style.inheritance?.familyRoot || style.name)}</span></div>
                                    <div><span class="font-semibold text-slate-700">Implementation:</span> <span class="font-mono text-slate-600">${escapeHtml(style.inheritance?.implementationForm || 'standalone')}</span></div>
                                    <div class="md:col-span-2"><span class="font-semibold text-slate-700">Inheritance:</span> <span class="font-mono text-slate-600">${escapeHtml(inheritanceChain)}</span></div>
                                    <div><span class="font-semibold text-slate-700">Registry kind:</span> <span class="font-mono text-slate-600">${escapeHtml(style.registry?.kind || '—')}</span></div>
                                    <div><span class="font-semibold text-slate-700">Aliases (${style.registry?.aliasCount || 0}):</span> <span class="font-mono text-slate-600">${escapeHtml(aliasText)}</span></div>
                                    <div><span class="font-semibold text-slate-700">Near-clone band:</span> <span class="font-mono text-slate-600">${escapeHtml(band?.band || 'unavailable')}${band?.target ? ` → ${escapeHtml(band.target)}` : ''}</span></div>
                                    <div><span class="font-semibold text-slate-700">Derivability:</span> <span class="font-mono text-slate-600">${escapeHtml(derivability?.verdict || 'unavailable')}${derivability?.target ? ` → ${escapeHtml(derivability.target)}` : ''}</span></div>
                                </div>
                                ${chainWarning ? `<div class="mt-3 text-xs font-mono text-amber-700">${escapeHtml(chainWarning)}</div>` : ''}
                                ${style.verificationNote ? `<div class="mt-3 text-xs text-slate-600"><strong>Note:</strong> ${escapeHtml(style.verificationNote)}</div>` : ''}
                            </div>
`;

  if (style.error) {
    html += `
                            <div class="p-4 rounded-lg bg-red-50 border border-red-200 mb-4">
                                <div class="text-sm font-medium text-red-700 mb-1">Error</div>
                                <div class="text-xs text-red-600 font-mono">${escapeHtml(style.error)}</div>
                            </div>
`;
  }

  if (style.qualityBreakdown) {
    const qb = style.qualityBreakdown;
    const overall = ((style.qualityScore || 0) * 100).toFixed(1);
    const typeCoverage = qb.subscores?.typeCoverage?.score ?? 0;
    const fallback = qb.subscores?.fallbackRobustness?.score ?? 0;
    const concision = qb.subscores?.concision?.score ?? 0;
    const presets = qb.subscores?.presetUsage?.score ?? 0;
    const concisionDetail = qb.subscores?.concision || {};
    html += `
                            <div class="mb-4 p-3 rounded border border-slate-200 bg-[var(--citum-surface)]">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Quality (SQI): ${overall}%</div>
                                <div class="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs font-mono">
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">type ${typeCoverage.toFixed(1)}%</div>
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">fallback ${fallback.toFixed(1)}%</div>
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">concision ${concision.toFixed(1)}%</div>
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">presets ${presets.toFixed(1)}%</div>
                                </div>
                                <div class="mt-2 text-xs text-slate-500 font-mono">
                                    scopes ${concisionDetail.scopeCount ?? 0} · variants ${concisionDetail.variantSelectors ?? 0} · exact dupes ${concisionDetail.exactDuplicateScopes ?? 0} · near dupes ${concisionDetail.nearDuplicateScopes ?? 0}
                                </div>
                            </div>
`;
  }

  if (style.notePositionAudit) {
    const audit = style.notePositionAudit;
    const regression = audit.regression || { status: audit.status, issues: audit.issues || [], profile: audit.profile };
    const hasConformance = Boolean(audit.conformance);
    const conformance = audit.conformance || { status: 'not-evaluated', issues: [], family: 'n/a', unresolved: [] };
    const regressionBadgeClass = regression.status === 'pass'
      ? 'bg-emerald-100 text-emerald-700'
      : regression.status === 'configuration-gap'
        ? 'bg-amber-100 text-amber-700'
        : 'bg-red-100 text-red-700';
    const conformanceBadgeClass = conformance.status === 'pass'
      ? 'bg-sky-100 text-sky-700'
      : conformance.status === 'not-evaluated'
        ? 'bg-slate-100 text-slate-700'
      : 'bg-amber-100 text-amber-700';
    const regressionIssueText = regression.issues.length === 0
      ? '<span class="text-xs text-slate-500">No repeated-note issues detected.</span>'
      : regression.issues
        .map((issue) => `<div class="text-xs font-mono text-slate-600">${escapeHtml(issue.kind)}: ${escapeHtml(issue.message)}</div>`)
        .join('');
    const conformanceIssueText = !hasConformance
      ? '<span class="text-xs text-slate-500">Normative conformance was not evaluated for this report artifact.</span>'
      : conformance.issues.length === 0
      ? '<span class="text-xs text-slate-500">No settled conformance gaps detected.</span>'
      : conformance.issues
        .map((issue) => `<div class="text-xs font-mono text-slate-600">${escapeHtml(issue.kind)}: ${escapeHtml(issue.message)}</div>`)
        .join('');
    const unresolvedText = !hasConformance
      ? '<span class="text-xs text-slate-500">No conformance metadata available.</span>'
      : Array.isArray(conformance.unresolved) && conformance.unresolved.length > 0
      ? `<div class="text-xs text-slate-500">Unresolved: ${escapeHtml(conformance.unresolved.join(', '))}</div>`
      : '<span class="text-xs text-slate-500">No unresolved conformance dimensions recorded.</span>';
    html += `
                            <div class="mb-4 p-3 rounded border border-slate-200 bg-[var(--citum-surface)]">
                                <div class="text-xs font-semibold text-slate-900 mb-3">Repeated-Note Audit</div>
                                <div class="mb-3">
                                    <div class="flex items-center justify-between gap-3 mb-2">
                                        <div class="text-xs font-semibold text-slate-700">Regression Layer</div>
                                        <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium ${regressionBadgeClass}">
                                            ${escapeHtml(regression.status)}
                                        </span>
                                    </div>
                                    <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs mb-2">
                                        <div><span class="font-semibold text-slate-700">Profile:</span> <span class="font-mono text-slate-600">${escapeHtml(regression.profile || '—')}</span></div>
                                        <div><span class="font-semibold text-slate-700">Issues:</span> <span class="font-mono text-slate-600">${regression.issues.length}</span></div>
                                    </div>
                                    ${regressionIssueText}
                                </div>
                                <div>
                                    <div class="flex items-center justify-between gap-3 mb-2">
                                        <div class="text-xs font-semibold text-slate-700">Normative Conformance</div>
                                        <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium ${conformanceBadgeClass}">
                                            ${escapeHtml(conformance.status)}
                                        </span>
                                    </div>
                                    <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs mb-2">
                                        <div><span class="font-semibold text-slate-700">Family:</span> <span class="font-mono text-slate-600">${escapeHtml(conformance.family || '—')}</span></div>
                                        <div><span class="font-semibold text-slate-700">Issues:</span> <span class="font-mono text-slate-600">${conformance.issues.length}</span></div>
                                    </div>
                                    ${conformanceIssueText}
                                    <div class="mt-2">
                                        ${unresolvedText}
                                    </div>
                                </div>
                            </div>
`;
  }

  if (Array.isArray(style.benchmarkRunResults) && style.benchmarkRunResults.length > 0) {
    html += `
                            <div class="mb-4 p-3 rounded border border-slate-200 bg-[var(--citum-surface)]">
                                <div class="text-xs font-semibold text-slate-900 mb-1">Official Supplemental Rich Benchmark Evidence</div>
                                <div class="text-xs text-slate-500 mb-3">Baseline compatibility remains the gate. Rich benchmark runs extend the official evidence for configured styles.</div>
                                <div class="space-y-3">
`;
    for (const benchmarkRun of style.benchmarkRunResults) {
      const statusClass = benchmarkRun.status === 'pass'
        ? 'bg-emerald-100 text-emerald-700'
        : benchmarkRun.status === 'ok'
          ? 'bg-slate-100 text-slate-600'
          : 'bg-red-100 text-red-700';
      const scopeText = benchmarkRun.scope === 'both' ? 'citation + bibliography' : benchmarkRun.scope;
      const contributionText = benchmarkRun.countTowardFidelity ? 'counts toward compatibility' : 'diagnostic only';
      const thresholdText = benchmarkRun.minPassRate != null
        ? `${(benchmarkRun.minPassRate * 100).toFixed(0)}%`
        : 'none';
      const bibliographyText = benchmarkRun.bibliography
        ? `${benchmarkRun.bibliography.passed}/${benchmarkRun.bibliography.total}`
        : benchmarkRun.bibliographyEntries != null
          ? `${benchmarkRun.bibliographyEntries} rendered`
          : '—';
      const citationsText = benchmarkRun.citations
        ? `${benchmarkRun.citations.passed}/${benchmarkRun.citations.total}`
        : '—';
      const comparisonText = benchmarkRun.runner === 'native-smoke'
        ? 'none (render-only smoke test)'
        : 'configured oracle';

      html += `
                                    <div class="rounded border border-slate-200 p-3">
                                        <div class="flex items-center justify-between gap-3 mb-2">
                                            <div class="text-xs font-semibold text-slate-800">${escapeHtml(benchmarkRun.label)}</div>
                                            <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium ${statusClass}">
                                                ${escapeHtml(benchmarkRun.status)}
                                            </span>
                                        </div>
                                        <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                                            <div><span class="font-semibold text-slate-700">Runner:</span> <span class="font-mono text-slate-600">${escapeHtml(benchmarkRun.runner)}</span></div>
                                            <div><span class="font-semibold text-slate-700">Scope:</span> <span class="font-mono text-slate-600">${escapeHtml(scopeText)}</span></div>
                                            <div><span class="font-semibold text-slate-700">Contribution:</span> <span class="font-mono text-slate-600">${escapeHtml(contributionText)}</span></div>
                                            <div><span class="font-semibold text-slate-700">Oracle comparison:</span> <span class="font-mono text-slate-600">${escapeHtml(comparisonText)}</span></div>
                                            <div><span class="font-semibold text-slate-700">Threshold:</span> <span class="font-mono text-slate-600">${escapeHtml(thresholdText)}</span></div>
                                            <div><span class="font-semibold text-slate-700">Bibliography:</span> <span class="font-mono text-slate-600">${escapeHtml(bibliographyText)}</span></div>
                                            ${benchmarkRun.citations ? `<div><span class="font-semibold text-slate-700">Citations:</span> <span class="font-mono text-slate-600">${escapeHtml(citationsText)}</span></div>` : ''}
                                            <div><span class="font-semibold text-slate-700">Refs fixture:</span> <span class="font-mono text-slate-600">${escapeHtml(benchmarkRun.refsFixture)}</span></div>
                                        </div>
                                        ${benchmarkRun.error ? `<div class="mt-2 text-xs font-mono text-red-600">${escapeHtml(benchmarkRun.error)}</div>` : ''}
                                    </div>
`;
    }
    html += `
                                </div>
                            </div>
`;
  }

  if (style.componentSummary && Object.keys(style.componentSummary).length > 0) {
    const issues = Object.entries(style.componentSummary)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8);
    html += `
                            <div class="mb-4">
                                <div class="text-xs font-semibold text-slate-700 mb-2">Top Component Issues</div>
                                <div class="flex flex-wrap gap-2">
`;
    for (const [issue, count] of issues) {
      html += `
                                    <span class="px-2 py-1 rounded bg-slate-100 text-slate-600 text-xs font-mono">
                                        ${escapeHtml(issue)} <span class="font-bold">×${count}</span>
                                    </span>
`;
    }
    html += `
                                </div>
                            </div>
`;
  }

  if (style.citationsByType && Object.keys(style.citationsByType).length > 0) {
    const citationTypes = Object.entries(style.citationsByType)
      .sort((a, b) => {
        const aPct = a[1].total > 0 ? (a[1].passed / a[1].total) : 0;
        const bPct = b[1].total > 0 ? (b[1].passed / b[1].total) : 0;
        if (aPct !== bPct) return aPct - bPct;
        return a[0].localeCompare(b[0]);
      });
    html += `
                            <div class="mb-4">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Citation Type Coverage (${citationTypes.length} types)</div>
                                <div class="flex flex-wrap gap-2">
`;
    for (const [type, stats] of citationTypes) {
      const pct = stats.total > 0 ? Math.round((stats.passed / stats.total) * 100) : 0;
      let badgeClass = 'bg-red-100 text-red-700';
      if (pct === 100) {
        badgeClass = 'bg-emerald-100 text-emerald-700';
      } else if (pct >= 70) {
        badgeClass = 'bg-amber-100 text-amber-700';
      }
      html += `
                                    <span class="px-2 py-1 rounded ${badgeClass} text-xs font-mono">
                                        ${escapeHtml(type)} ${stats.passed}/${stats.total}
                                    </span>
`;
    }
    html += `
                                </div>
                            </div>
`;
  }

  if (style.citationEntries && style.citationEntries.length > 0) {
    const citationFindings = style.citationEntries.filter((entry) => {
      if (!entry.match) return true;
      if (typeof entry.exactMatch === 'boolean') return !entry.exactMatch;
      const texts = getRawComparisonEntryTexts(entry);
      return !compareText(texts.benchmark, texts.citum).exactMatch;
    });
    if (citationFindings.length === 0) {
      html += `
                            <div class="mb-4 p-3 rounded bg-emerald-50 border border-emerald-200">
                                <div class="text-xs font-semibold text-emerald-700">All ${style.citationEntries.length} citations are compatible and exact ✓</div>
                            </div>
`;
    } else {
      html += `
                            <div class="mb-4">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Citation Findings (${citationFindings.length}/${style.citationEntries.length})</div>
                                <div class="overflow-x-auto">
                                    <table class="w-full text-xs border-collapse">
                                        <thead>
                                            <tr class="border-b border-slate-300 bg-slate-100">
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">#</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Benchmark</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Citum</th>
                                                <th class="text-center px-2 py-1 font-medium text-slate-700">Status</th>
                                            </tr>
                                        </thead>
                                        <tbody>
`;
      for (const entry of citationFindings) {
        const rawTexts = getRawComparisonEntryTexts(entry);
        const exactMatch = typeof entry.exactMatch === 'boolean'
          ? entry.exactMatch
          : compareText(rawTexts.benchmark, rawTexts.citum).exactMatch;
        const texts = exactMatch ? getComparisonEntryTexts(entry) : getExactComparisonEntryTexts(entry);
        const diff = exactMatch
          ? null
          : renderInlineTextDifference(texts.benchmark, texts.citum);
        const benchmarkText = diff?.benchmark ?? renderTextPreview(texts.benchmark);
        const citumText = diff?.citum ?? renderTextPreview(texts.citum);
        const statusText = entry.match ? 'Unresolved Oracle Drift' : 'Compatibility Fail';
        const statusColor = entry.match ? 'text-amber-700' : 'text-red-600';
        html += `
                                            <tr class="border-b border-slate-200 hover:bg-slate-50">
                                                <td class="px-2 py-1 text-slate-600">${escapeHtml(entry.id)}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(texts.benchmark)}">${benchmarkText}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(texts.citum)}">${citumText}</td>
                                                <td class="px-2 py-1 text-center font-semibold ${statusColor}">${statusText}${diff ? ` <span class="text-slate-400">(Δ${diff.position})</span>` : ''}</td>
                                            </tr>
`;
      }
      html += `
                                        </tbody>
                                    </table>
                                </div>
                            </div>
`;
    }
  }

  html += renderBibliographyEvidence(style);

  if (style.knownDivergences && style.knownDivergences.length > 0) {
    html += `
                            <div class="p-4 rounded-lg bg-primary/5 border border-primary/20 mt-4">
                                <div class="text-sm font-semibold text-primary mb-2">Citum Extensions</div>
`;
    for (const divergence of style.knownDivergences) {
      html += `
                                <div class="text-xs text-slate-700 mb-2">
                                    <strong>${escapeHtml(divergence.feature)}:</strong> ${escapeHtml(divergence.description)}
                                </div>
`;
    }
    html += `
                            </div>
`;
  }

  return html;
}

function generateHtmlFooter() {
  return `

    <!-- Footer -->
    <footer class="py-12 px-6 border-t border-slate-200 bg-[var(--citum-surface)]">
        <div class="max-w-7xl mx-auto">
            <div class="flex flex-col md:flex-row justify-between items-center gap-8">
                <div class="flex items-center gap-2">
                    <a href="index.html" class="flex items-center gap-2 group">
                        <div class="w-6 h-6 bg-primary rounded flex items-center justify-center group-hover:brightness-110 transition-all">
                            <span class="text-white font-mono text-xs font-bold">C</span>
                        </div>
                        <span class="font-mono text-lg font-bold text-slate-900">Citum</span>
                    </a>
                </div>
                <div class="flex gap-8 text-sm font-medium text-slate-500">
                    <a class="hover:text-primary transition-colors" href="https://github.com/citum/citum-core">GitHub</a>
                    <a class="hover:text-primary transition-colors" href="index.html">Docs</a>
                    <a class="hover:text-primary transition-colors" href="examples.html">Examples</a>
                    <a class="hover:text-primary transition-colors" href="reports.html">Reports</a>
                </div>
                <div class="text-sm text-slate-400">
                    © 2026 Citum Project. MIT Licensed.
                </div>
            </div>
        </div>
    </footer>

    <script>
        const navToggle = document.querySelector("[data-nav-toggle]");
        const mobileMenu = document.querySelector("[data-mobile-menu]");
        if (navToggle && mobileMenu) {
            navToggle.addEventListener("click", () => {
                const expanded = navToggle.getAttribute("aria-expanded") === "true";
                navToggle.setAttribute("aria-expanded", String(!expanded));
                mobileMenu.classList.toggle("hidden", expanded);
            });
        }

        const sortState = { key: null, direction: 1 };
        const filterState = { query: '' };

        function toggleAccordion(contentId) {
            const content = document.getElementById(contentId);
            if (content) content.classList.toggle('active');
        }

        function updateSortIndicators(activeKey, direction) {
            document.querySelectorAll('[id^="sort-ind-"]').forEach((el) => {
                el.textContent = '↕';
                el.classList.remove('text-primary');
                el.classList.add('text-slate-400');
            });
            const active = document.getElementById('sort-ind-' + activeKey);
            if (active) {
                active.textContent = direction > 0 ? '↑' : '↓';
                active.classList.remove('text-slate-400');
                active.classList.add('text-primary');
            }
        }

        function sortCompatTable(key) {
            const tbody = document.querySelector('table tbody');
            if (!tbody) return;

            const familyHeaders = Array.from(tbody.querySelectorAll('tr.family-header'));
            const summaryRows = Array.from(tbody.querySelectorAll('tr.accordion-toggle'));
            const rowPairs = summaryRows.map((summary) => {
                const detailId = summary.dataset.detailId;
                const detail = detailId ? document.getElementById(detailId) : null;
                return { summary, detail };
            });

            const defaultAsc = key === 'style-name' || key === 'format' || key === 'origin';
            if (sortState.key === key) {
                sortState.direction *= -1;
            } else {
                sortState.key = key;
                sortState.direction = defaultAsc ? 1 : -1;
            }

            const asNumber = (value) => {
                const parsed = Number(value);
                return Number.isNaN(parsed) ? -Infinity : parsed;
            };
            const asText = (value) => String(value || '').toLowerCase();

            // Convert kebab-case key to camelCase for dataset access
            const datasetKey = key.replace(/-([a-z])/g, (g) => g[1].toUpperCase());

            rowPairs.sort((a, b) => {
                const left = a.summary.dataset[datasetKey] || '';
                const right = b.summary.dataset[datasetKey] || '';
                const numericKeys = new Set([
                    'csl-reach',
                    'citation-rate',
                    'bibliography-rate',
                    'component-rate',
                    'fidelity',
                    'exact-parity',
                    'quality',
                    'sqi-tier-rank',
                ]);

                if (key === 'csl-reach') {
                    const leftUnknown = left === '' || Number(left) < 0;
                    const rightUnknown = right === '' || Number(right) < 0;
                    if (leftUnknown !== rightUnknown) {
                        return leftUnknown ? 1 : -1;
                    }
                }

                if (numericKeys.has(key)) {
                    return (asNumber(left) - asNumber(right)) * sortState.direction;
                }
                return asText(left).localeCompare(asText(right)) * sortState.direction;
            });

            const pairsByFamily = new Map();
            for (const pair of rowPairs) {
                const root = pair.summary.dataset.familyRoot || '';
                const members = pairsByFamily.get(root) || [];
                members.push(pair);
                pairsByFamily.set(root, members);
            }

            for (const header of familyHeaders) {
                tbody.appendChild(header);
                const members = pairsByFamily.get(header.dataset.familyRoot || '') || [];
                for (const pair of members) {
                    tbody.appendChild(pair.summary);
                    if (pair.detail) tbody.appendChild(pair.detail);
                }
            }

            updateSortIndicators(key, sortState.direction);
        }

        function updateFilterCount(visible, total) {
            const count = document.getElementById('style-search-count');
            if (!count) return;
            count.textContent = visible === total
                ? total + ' styles'
                : visible + ' of ' + total + ' styles';
        }

        function applyStyleFilter() {
            const tbody = document.querySelector('table tbody');
            if (!tbody) return;

            const summaryRows = Array.from(tbody.querySelectorAll('tr.accordion-toggle'));
            const query = filterState.query;
            let visible = 0;

            for (const summary of summaryRows) {
                const detailId = summary.dataset.detailId;
                const detail = detailId ? document.getElementById(detailId) : null;
                const haystack = (summary.dataset.search || summary.dataset.styleName || '').toLowerCase();
                const isMatch = !query || haystack.includes(query);

                summary.style.display = isMatch ? '' : 'none';
                if (detail) {
                    detail.style.display = isMatch ? '' : 'none';
                    if (!isMatch) detail.classList.remove('active');
                }
                if (isMatch) visible += 1;
            }

            const familyHeaders = Array.from(tbody.querySelectorAll('tr.family-header'));
            for (const header of familyHeaders) {
                const root = header.dataset.familyRoot || '';
                const hasVisibleMember = summaryRows.some((summary) =>
                    summary.dataset.familyRoot === root && summary.style.display !== 'none'
                );
                header.style.display = hasVisibleMember ? '' : 'none';
            }

            updateFilterCount(visible, summaryRows.length);
        }

        function initStyleSearch() {
            const input = document.getElementById('style-search');
            if (!input) return;
            input.addEventListener('input', (event) => {
                filterState.query = String(event.target.value || '').trim().toLowerCase();
                applyStyleFilter();
            });

            const tbody = document.querySelector('table tbody');
            if (tbody) {
                tbody.addEventListener('click', (event) => {
                    const row = event.target.closest('tr.accordion-toggle');
                    if (!row) return;
                    // Don't re-toggle if the expand button itself was clicked (it has its own onclick)
                    if (event.target.closest('button')) return;
                    const detailId = row.dataset.detailId;
                    if (detailId) toggleAccordion(detailId);
                });
            }

            applyStyleFilter();
        }

        document.addEventListener('DOMContentLoaded', initStyleSearch);
    </script>

</body>

</html>
`;
}

function escapeHtml(text) {
  if (!text) return '';
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Main entry point
 */
async function main() {
  try {
    const options = parseArgs();
    const stylesDir = options.stylesDir || path.join(PROJECT_ROOT, 'styles-legacy');
    const datasetMessage = maybeDatasetErrorForFile(stylesDir, 'report-core.js');
    if (datasetMessage) {
      throw new Error(datasetMessage);
    }
    const { report, errorCount } = await generateReport(options);

    // Output JSON to stdout
    console.log(JSON.stringify(report, null, 2));

    // Generate and write HTML if requested
    if (options.writeHtml) {
      const htmlPath = options.outputHtml || path.join(path.dirname(__dirname), 'docs', 'compat.html');
      const htmlDir = path.dirname(htmlPath);

      if (!fs.existsSync(htmlDir)) {
        fs.mkdirSync(htmlDir, { recursive: true });
      }

      const htmlContent = generateHtml(report).replace(/[ \t]+$/gm, '');
      fs.writeFileSync(htmlPath, htmlContent, 'utf8');
      process.stderr.write(`HTML report written to: ${htmlPath}\n`);
    }

    if (errorCount > 0) {
      process.stderr.write(`\nTotal styles with errors: ${errorCount}\n`);
      process.exit(1);
    }
  } catch (error) {
    process.stderr.write(`Error: ${error.message}\n`);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  buildNoteStyleLookup,
  collectTemplateScopes,
  computeComponentMatchRate,
  computeConcisionScore,
  computeFallbackRobustness,
  computePresetUsageScore,
  computeFidelityScore,
  createReportRuntime,
  discoverCoreStyles,
  equivalentText,
  expandCompoundBibEntries,
  formatAuthorityLabel,
  getEffectiveOracleSection,
  getCslSnapshotStatus,
  generateHtml,
  generateReport,
  hasRootExtends,
  getComparisonEntryTexts,
  loadStyleYaml,
  mapWithConcurrency,
  normalizeBenchmarkSource,
  parseArgs,
  preflightSnapshots,
  resolveSelectedStyles,
  runCachedJsonJob,
  buildEmptyOracleResult,
  cloneOracleResult,
  executeBenchmarkRuns,
  mergeBenchmarkRunIntoOracle,
  mergeOracleResults,
  toPublishedBenchmarkRunRecord,
  determineBenchmarkStatus,
  selectQualityAuthorshipData,
  selectPrimaryComparator,
  serializeTimingSummary,
  summarizeBibliographyPairing,
  summarizeExactParity,
  textSimilarity,
  mergeDivergenceSummaries,
};
