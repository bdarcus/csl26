#!/usr/bin/env node
/**
 * Structured Diff Oracle for Citum Migration (DEFAULT)
 *
 * Compares citeproc-js and Citum outputs at the component level,
 * identifying which specific parts of a bibliography entry differ.
 *
 * This is now the default oracle script. For simple string comparison,
 * use oracle-simple.js instead.
 *
 * Usage:
 *   node oracle.js ../styles/apa.csl
 *   node oracle.js ../styles/apa.csl --json
 *   node oracle.js ../styles/apa.csl --verbose
 *   node oracle.js ../styles/apa.csl --simple  # fallback to simple mode
 *
 * Exit codes:
 *   0 - Success (all citations and bibliography entries match)
 *   1 - Failed validation (some entries don't match)
 *   2 - Fatal error (file not found, parse error, Citum rendering failed)
 */

const CSL = require('citeproc');
const fs = require('fs');
const os = require('os');
const path = require('path');
const yaml = require('js-yaml');
const { execFileSync, execSync } = require('child_process');
const { bibliographyIdsForRenderedRows } = require('./lib/citeproc-bibliography');
const {
  compareText,
  normalizeText,
  parseComponents,
  compareComponents,
  compareCitationComponents,
  analyzeOrdering,
  findRefDataForEntry,
  loadLocale,
  textSimilarity,
} = require('./oracle-utils');
const { toCiteprocItem } = require('./lib/citeproc-locators');
const { maybeDatasetErrorForFile } = require('./lib/dataset-guard');
const { attachRegisteredDivergenceAdjustments } = require('./lib/oracle-divergences');
const { resolveStyleData } = require('./lib/verification-policy');

const DEFAULT_REFS_FIXTURE = path.join(__dirname, '..', 'tests', 'fixtures', 'references-expanded.json');
const DEFAULT_CITATIONS_FIXTURE = path.join(__dirname, '..', 'tests', 'fixtures', 'citations-expanded.json');
// Citation IDs where fuzzy token overlap can hide real disambiguation regressions.
const STRICT_CITATION_IDS = new Set([
  'et-al-single-long-list',
  'disambiguate-add-names-et-al',
  'disambiguate-year-suffix',
  'disambiguate-givenname',
  'et-al-with-locator',
  'subsequent-author-consecutive',
]);
// Embedded-core GB/T styles require exact normalized bibliography fidelity.
// Similarity remains available on the comparison result for diagnostics.
const STRICT_BIBLIOGRAPHY_STYLES = new Set([
  'gb-t-7714-2025-numeric',
  'gb-t-7714-2025-author-date',
  'gb-t-7714-2025-note',
]);

function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    stylePath: null,
    jsonOutput: false,
    verbose: false,
    refsFixture: DEFAULT_REFS_FIXTURE,
    citationsFixture: null,
    forceMigrate: false,
    caseSensitive: true,
    allFeatures: false,
    citumBin: null,
    scope: 'both',
    locale: null,
    migrate: {
      templateSource: null,
      minTemplateConfidence: null,
      templateDir: null,
    },
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--json') {
      options.jsonOutput = true;
    } else if (arg === '--verbose') {
      options.verbose = true;
    } else if (arg === '--case-sensitive') {
      options.caseSensitive = true;
    } else if (arg === '--case-insensitive') {
      options.caseSensitive = false;
    } else if (arg === '--all-features') {
      options.allFeatures = true;
    } else if (arg === '--citum-bin') {
      options.citumBin = path.resolve(args[++i]);
    } else if (arg === '--force-migrate') {
      options.forceMigrate = true;
    } else if (arg === '--refs-fixture') {
      options.refsFixture = path.resolve(args[++i]);
    } else if (arg === '--citations-fixture') {
      options.citationsFixture = path.resolve(args[++i]);
    } else if (arg === '--scope') {
      options.scope = args[++i];
    } else if (arg === '--locale') {
      options.locale = args[++i];
    } else if (arg === '--migrate-template-source') {
      options.migrate.templateSource = args[++i];
    } else if (arg === '--migrate-min-template-confidence') {
      options.migrate.minTemplateConfidence = args[++i];
    } else if (arg === '--migrate-template-dir') {
      options.migrate.templateDir = path.resolve(args[++i]);
    } else if (!arg.startsWith('--') && !options.stylePath) {
      options.stylePath = arg;
    }
  }

  if (!options.stylePath) {
    options.stylePath = path.join(__dirname, '..', 'styles-legacy', 'apa.csl');
  }
  if (!['both', 'bibliography', 'citation'].includes(options.scope)) {
    throw new Error(`Unsupported --scope value: ${options.scope}`);
  }
  if (options.scope !== 'bibliography' && !options.citationsFixture) {
    options.citationsFixture = DEFAULT_CITATIONS_FIXTURE;
  }

  return options;
}

function normalizeFixtureItems(fixturesData) {
  if (Array.isArray(fixturesData)) {
    return Object.fromEntries(fixturesData.map((item) => [item.id, item]));
  }

  if (fixturesData && Array.isArray(fixturesData.items)) {
    return Object.fromEntries(fixturesData.items.map((item) => [item.id, item]));
  }

  if (fixturesData && Array.isArray(fixturesData.references)) {
    return Object.fromEntries(fixturesData.references.map((item) => [item.id, item]));
  }

  return Object.fromEntries(
    Object.entries(fixturesData).filter(([key, value]) => key !== 'comment' && value && typeof value === 'object')
  );
}

function summarizeSection(section = {}) {
  const total = section.total || 0;
  const passed = section.passed || 0;
  const failed = section.failed ?? Math.max(0, total - passed);
  return {
    total,
    passed,
    failed,
    matchRate: total > 0 ? parseFloat((passed / total).toFixed(3)) : null,
  };
}

function buildOracleSummary(results, scope) {
  return {
    scope,
    citations: summarizeSection(results.citations),
    bibliography: summarizeSection(results.bibliography),
    orderingIssues: results.orderingIssues || 0,
    // Positional bibliography-entry-sequence mismatch (csl26-7u16) — distinct
    // from orderingIssues above, which only measures component order within
    // one already-mismatched entry. bibliographyOrderMismatch can be true
    // even when every entry's text matches and orderingIssues is 0.
    bibliographyOrderMismatch: Boolean(results.bibliographyOrder),
    bibliographyOrderExplainedBy: results.bibliographyOrder?.explained
      ? results.bibliographyOrder.appliedDivergence
      : null,
    componentSummary: { ...(results.componentSummary || {}) },
  };
}

function normalizeProcessorDates(value, parentKey = null) {
  const seasonMap = {
    Spring: 21,
    Summer: 22,
    Autumn: 23,
    Fall: 23,
    Winter: 24,
  };

  if (Array.isArray(value)) {
    if (parentKey === 'date-parts') {
      return value.map((row) =>
        Array.isArray(row)
          ? row.map((part) => (typeof part === 'string' && /^-?\d+$/.test(part) ? Number.parseInt(part, 10) : part))
          : row
      );
    }
    return value.map((entry) => normalizeProcessorDates(entry));
  }

  if (!value || typeof value !== 'object') {
    return value;
  }

  return Object.fromEntries(
    Object.entries(value).map(([key, entry]) => {
      if (key === 'season' && typeof entry === 'string' && Object.prototype.hasOwnProperty.call(seasonMap, entry)) {
        return [key, seasonMap[entry]];
      }
      return [key, normalizeProcessorDates(entry, key)];
    })
  );
}

function refsDataForProcessor(fixturesData) {
  if (fixturesData && Array.isArray(fixturesData.items)) {
    return normalizeProcessorDates(fixturesData.items);
  }
  return normalizeProcessorDates(fixturesData);
}

function loadFixtures(refsFixture, citationsFixture) {
  const fixturesData = JSON.parse(fs.readFileSync(refsFixture, 'utf8'));
  const testItems = normalizeFixtureItems(fixturesData);
  const testCitations = citationsFixture
    ? JSON.parse(fs.readFileSync(citationsFixture, 'utf8'))
    : [];
  return { refsData: fixturesData, testItems, testCitations };
}

function createOracleTempWorkspace() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'citum-oracle-'));
  return {
    dir,
    refsFile: path.join(dir, 'refs.json'),
    citationsFile: path.join(dir, 'citations.json'),
    styleFile: path.join(dir, 'style.yaml'),
  };
}

function cleanupOracleTempWorkspace(workspace) {
  if (!workspace?.dir) return;
  fs.rmSync(workspace.dir, { recursive: true, force: true });
}

/**
 * Compare component ordering between oracle and Citum.
 */
function compareOrdering(oracleOrder, citumOrder) {
  const issues = [];

  // Check if orders match
  if (JSON.stringify(oracleOrder) !== JSON.stringify(citumOrder)) {
    issues.push({
      issue: 'ordering',
      expected: oracleOrder,
      found: citumOrder,
      detail: `Component order differs`
    });
  }

  return issues;
}

function renderWithCiteprocJs(stylePath, testItems, testCitations, options = {}) {
  let cslXml;
  const isYaml = stylePath.endsWith('.yaml') || stylePath.endsWith('.yml');

  if (isYaml) {
    // If input is YAML, we need to find the corresponding legacy CSL for citeproc-js
    const styleName = path.basename(stylePath, path.extname(stylePath));
    const baseName = styleName.replace(/-7th$|-(?:author-date|notes|numeric)$/, '');
    const legacyDir = path.join(__dirname, '..', 'styles-legacy');

    const candidates = [
      path.join(legacyDir, `${styleName}.csl`),
      path.join(legacyDir, `${baseName}.csl`),
      path.join(legacyDir, 'apa.csl'), // Special case fallback for apa-7th refactor
    ];

    const cslPath = candidates.find((p) => fs.existsSync(p));
    if (!cslPath) {
      if (options.verbose) {
        console.warn(`No legacy CSL found for ${styleName}, skipping oracle rendering.`);
      }
      return { citations: {}, bibliography: [] };
    }
    cslXml = fs.readFileSync(cslPath, 'utf8');
  } else {
    cslXml = fs.readFileSync(stylePath, 'utf8');
  }

  const sys = {
    retrieveLocale: (lang) => loadLocale(lang),
    retrieveItem: (id) => testItems[id],
  };

  // Without --locale, citeproc-js resolves its own language from the CSL's
  // `default-locale` attribute (falling back to en-US) — a value authored
  // independently of, and not necessarily in sync with, citum's own
  // `info.default-locale`. Passing `options.locale` with forceLang=true makes
  // citeproc-js use the same locale citum resolved, closing that gap.
  const citeproc = options.locale
    ? new CSL.Engine(sys, cslXml, options.locale, true)
    : new CSL.Engine(sys, cslXml);
  citeproc.updateItems(Object.keys(testItems));

  const citations = {};
  if (options.scope !== 'bibliography') {
    testCitations.forEach((cite) => {
      // Scenarios with `clusters` use multi-cluster position tracking; flatten for oracle
      const clusters = cite.clusters
        ? cite.clusters
        : [{ id: cite.id, items: cite.items, 'suppress-author': cite['suppress-author'] }];

      const parts = [];
      for (const cluster of clusters) {
        const suppressAuthor = cluster['suppress-author'] === true;
        const citeprocItems = cluster.items.map((item) => toCiteprocItem(item, suppressAuthor));
        try {
          parts.push(citeproc.makeCitationCluster(citeprocItems));
        } catch (e) {
          if (options.verbose) {
            console.error(`Error rendering citation cluster ${cluster.id}:`, e.message);
          }
          parts.push(`ERROR: ${e.message}`);
        }
      }
      citations[cite.id] = parts.join(' | ');
    });
  }

  const bibResult = citeproc.makeBibliography();
  const bibliography = bibResult ? bibResult[1] : [];
  const bibliographyIds = bibliographyIdsForRenderedRows(bibResult?.[0], bibliography);

  return { citations, bibliography, bibliographyIds };
}

function buildMigrateCommand(absStylePath, migrateOptions = {}, allFeatures = false) {
  const parts = [
    'cargo run -q --bin citum-migrate' + (allFeatures ? ' --all-features' : '') + ' --',
    `"${absStylePath}"`,
  ];

  if (migrateOptions.templateSource) {
    parts.push(`--template-source ${migrateOptions.templateSource}`);
  }
  if (migrateOptions.minTemplateConfidence) {
    parts.push(`--min-template-confidence ${migrateOptions.minTemplateConfidence}`);
  }
  if (migrateOptions.templateDir) {
    parts.push(`--template-dir "${migrateOptions.templateDir}"`);
  }

  return parts.join(' ');
}

function expandCitationsForCitum(testCitations) {
  const clustered = new Map();
  const citations = [];

  for (const cite of testCitations) {
    if (!Array.isArray(cite.clusters)) {
      citations.push(cite);
      continue;
    }

    const clusterIds = [];
    cite.clusters.forEach((cluster, index) => {
      const clusterId = `${cite.id}__${cluster.id || index + 1}`;
      clusterIds.push(clusterId);
      citations.push({
        ...cluster,
        id: clusterId,
        items: cluster.items || [],
        'suppress-author': cluster['suppress-author'] === true,
      });
    });
    clustered.set(cite.id, clusterIds);
  }

  return { citations, clustered };
}

function collapseClusteredCitumCitations(rendered, clustered) {
  if (!clustered.size) {
    return rendered;
  }

  const citations = { ...rendered.citations };
  for (const [id, clusterIds] of clustered.entries()) {
    citations[id] = clusterIds
      .map((clusterId) => citations[clusterId])
      .filter((part) => part != null)
      .join(' | ');
    for (const clusterId of clusterIds) {
      delete citations[clusterId];
    }
  }

  return {
    ...rendered,
    citations,
  };
}

// Resolve an already-authored Citum YAML for a style name. Searches
// `styles/<name>.yaml`, then `styles/embedded/<name>.yaml` (the canonical
// parents shipped inside the citum binary), then a base-name prefix match
// inside `styles/`. Returns null when nothing matches.
function resolveAuthoredStylePath(stylesDir, styleName) {
  if (!fs.existsSync(stylesDir)) return null;
  const files = fs.readdirSync(stylesDir).sort();
  const exactMatch = `${styleName}.yaml`;

  if (files.includes(exactMatch)) {
    return path.join(stylesDir, exactMatch);
  }

  const embeddedDir = path.join(stylesDir, 'embedded');
  const embeddedPath = path.join(embeddedDir, exactMatch);
  if (fs.existsSync(embeddedPath)) {
    return embeddedPath;
  }

  const baseName = styleName.replace(/-\d+th$/, '').replace(/-\d+$/, '');

  // Prefix-scan styles/embedded/ first (e.g. 'apa' → 'apa-7th.yaml')
  // Embedded styles are canonical hand-authored YAMLs and take priority.
  const embeddedFiles = fs.existsSync(embeddedDir) ? fs.readdirSync(embeddedDir).sort() : [];
  const embeddedFound = embeddedFiles.find((f) =>
    f.endsWith('.yaml') &&
    (f.startsWith(`${styleName}-`) || f.startsWith(`${baseName}-`))
  );
  if (embeddedFound) return path.join(embeddedDir, embeddedFound);

  // Fall back to prefix-scan in styles/ root
  const found = files.find((f) =>
    f.endsWith('.yaml') &&
    (f.startsWith(`${styleName}-`) || f.startsWith(`${baseName}-`))
  );
  return found ? path.join(stylesDir, found) : null;
}

function parseCitumRenderOutput(output, testItems) {
  const lines = output.split('\n');
  const citations = {};
  const bibliography = {};
  const bibliographyOrderIds = [];
  const integralCitations = [];

  let section = null;
  for (const line of lines) {
    if (
      line.includes('CITATIONS (From file):') ||
      line.includes('CITATIONS (Non-Integral):')
    ) {
      section = 'citations';
      continue;
    } else if (line.includes('CITATIONS (Integral):')) {
      section = 'citations_integral';
      continue;
    } else if (line.includes('BIBLIOGRAPHY:')) {
      section = 'bibliography';
      continue;
    }

    if (!line.trim() || line.includes('===')) {
      continue;
    }

    if (section === 'citations') {
      const match = line.match(/^\s*\[([^\]]+)\]\s+(.+)/);
      if (match) {
        citations[match[1]] = match[2].trim();
      }
    } else if (section === 'citations_integral') {
      // Integral citations currently print without keys, so keep them for
      // inspection without merging them into the keyed citation comparison map.
      integralCitations.push(line.trim());
    } else if (section === 'bibliography') {
      const match = line.match(/^\s*\[([^\]]+)\]\s+(.+)/);
      if (match) {
        bibliography[match[1]] = match[2].trim();
        bibliographyOrderIds.push(match[1]);
      }
    }
  }

  const orderedBibliography = [];
  const bibliographyIds = [];
  Object.keys(testItems).forEach((id) => {
    if (bibliography[id]) {
      orderedBibliography.push(bibliography[id]);
      bibliographyIds.push(id);
    }
  });

  return {
    citations,
    bibliography: orderedBibliography,
    bibliographyIds,
    bibliographyOrderIds,
    integralCitations,
  };
}

function renderWithCitumProcessor(stylePath, refsData, testItems, testCitations, cliOptions = {}) {
  const projectRoot = path.resolve(__dirname, '..');
  const isYaml = stylePath.endsWith('.yaml') || stylePath.endsWith('.yml');
  const styleName = isYaml
    ? path.basename(stylePath, path.extname(stylePath))
    : path.basename(stylePath, '.csl');
  const stylesDir = path.join(projectRoot, 'styles');
  const workspace = createOracleTempWorkspace();
  const migrateOptions = cliOptions.migrate || {};
  const forceMigrate = Boolean(cliOptions.forceMigrate);

  try {
    // 1. Try to find a hand-authored style first
    let citumStylePath = null;

    if (isYaml && fs.existsSync(stylePath)) {
      citumStylePath = path.resolve(stylePath);
    } else if (!forceMigrate) {
      citumStylePath = resolveAuthoredStylePath(stylesDir, styleName);
    }

    const scope = cliOptions.scope || 'both';
    const bibliographyOnly = scope === 'bibliography';
    const citumCitationInput = expandCitationsForCitum(testCitations);

    fs.writeFileSync(workspace.refsFile, JSON.stringify(refsDataForProcessor(refsData), null, 2));
    if (!bibliographyOnly) {
      fs.writeFileSync(
        workspace.citationsFile,
        JSON.stringify(citumCitationInput.citations, null, 2)
      );
    }

    if (!citumStylePath) {
      // 2. Fall back to migration
      const absStylePath = path.resolve(stylePath);
      let migratedYaml;
      try {
        const migrateCmd = buildMigrateCommand(absStylePath, migrateOptions, cliOptions.allFeatures);
        migratedYaml = execSync(
          migrateCmd,
          { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
        );
      } catch (e) {
        console.error('Migration failed:', e.stderr || e.message);
        return null;
      }
      fs.writeFileSync(workspace.styleFile, migratedYaml);
      citumStylePath = workspace.styleFile;
    }

    let output;
    try {
      const renderArgs = [
        'render', 'refs',
        '-b', workspace.refsFile,
        '-s', citumStylePath,
      ];
      if (!bibliographyOnly) {
        renderArgs.push('-c', workspace.citationsFile);
      }
      renderArgs.push('--mode', bibliographyOnly ? 'bib' : 'both');
      renderArgs.push('--show-keys');
      if (cliOptions.locale) {
        renderArgs.push('--locale', cliOptions.locale);
      }
      if (cliOptions.citumBin) {
        output = execFileSync(cliOptions.citumBin, renderArgs, {
          cwd: projectRoot,
          encoding: 'utf8',
          stdio: ['pipe', 'pipe', 'pipe'],
        });
      } else {
        const cargoArgs = ['run', '-q', '--bin', 'citum'];
        if (cliOptions.allFeatures) cargoArgs.push('--all-features');
        cargoArgs.push('--', ...renderArgs);
        output = execFileSync(
          'cargo',
          cargoArgs,
          { cwd: projectRoot, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }
        );
      }
    } catch (e) {
      const errorMsg = e.stderr ? e.stderr.toString() : e.message;
      return { error: `Processor failed: ${errorMsg}`, citations: { passed: 0, total: 0 }, bibliography: { passed: 0, total: 0 } };
    }

    return collapseClusteredCitumCitations(
      parseCitumRenderOutput(output, testItems),
      citumCitationInput.clustered
    );
  } finally {
    cleanupOracleTempWorkspace(workspace);
  }
}

function equivalentText(oracleText, citumText, options = {}) {
  return compareText(oracleText, citumText, options).match;
}

function extractYearSuffixes(text) {
  return normalizeText(text).match(/\b\d{4}[a-z]\b/gi) || [];
}

function hasEtAl(text) {
  return /\bet al\b/i.test(normalizeText(text));
}

function splitCitationCluster(text) {
  const normalized = normalizeText(text)
    .replace(/^\(/, '')
    .replace(/\)$/, '');
  return normalized
    .split(/\s*;\s*/)
    .map((part) => part.trim())
    .filter(Boolean);
}

function extractLocatorNumber(text) {
  const match = normalizeText(text).match(/\b(?:p|pp|section|sec)\.?\s*(\d+)\b/i);
  return match ? match[1] : null;
}

function equivalentDisambiguationProbe(oracleText, citumText, citationId) {
  const oracleNorm = normalizeText(oracleText);
  const citumNorm = normalizeText(citumText);

  const oracleSuffixCount = extractYearSuffixes(oracleNorm).length;
  const citumSuffixCount = extractYearSuffixes(citumNorm).length;
  const oracleHasEtAl = hasEtAl(oracleNorm);
  const citumHasEtAl = hasEtAl(citumNorm);

  if (oracleHasEtAl && !citumHasEtAl) return false;
  if (oracleSuffixCount > 0 && citumSuffixCount === 0) return false;

  if (citationId === 'disambiguate-add-names-et-al') {
    if (oracleHasEtAl || oracleSuffixCount > 0) {
      const citumParts = splitCitationCluster(citumNorm);
      if (citumParts.length < 2) return false;
      if (new Set(citumParts).size !== citumParts.length) return false;
    }
  }

  if (citationId === 'et-al-with-locator') {
    const oracleLocator = extractLocatorNumber(oracleNorm);
    const citumLocator = extractLocatorNumber(citumNorm);
    if (oracleLocator && oracleLocator !== citumLocator) return false;
  }

  return true;
}

function equivalentCitationText(oracleText, citumText, citationId, options = {}) {
  if (options.caseSensitive !== false && compareText(oracleText, citumText, options).caseMismatch) {
    return false;
  }
  if (STRICT_CITATION_IDS.has(citationId)) {
    return equivalentDisambiguationProbe(oracleText, citumText, citationId);
  }
  return equivalentText(oracleText, citumText, options);
}

function collectCitationTypes(citation, testItems) {
  const types = new Set();
  const items = Array.isArray(citation.clusters)
    ? citation.clusters.flatMap((cluster) => cluster.items || [])
    : citation.items || [];
  for (const item of items) {
    const ref = testItems[item.id];
    if (ref && ref.type) {
      types.add(ref.type);
    } else {
      types.add('unknown');
    }
  }
  return [...types];
}

/**
 * Match bibliography entries between oracle and Citum by finding best matches.
 * Uses contributor names and titles to pair entries.
 */
function matchBibliographyEntries(oracleBib, citumBib, oracleIds = [], citumIds = []) {
  if (oracleIds.length === oracleBib.length && citumIds.length === citumBib.length) {
    const citumById = new Map(
      citumIds.map((id, index) => [id, { text: citumBib[index] }])
    );
    const oracleIdSet = new Set(oracleIds);
    const pairs = oracleIds.map((id, index) => {
      const citum = citumById.get(id);
      return {
        id,
        oracle: oracleBib[index],
        citum: citum?.text ?? null,
        score: citum ? textSimilarity(oracleBib[index], citum.text) : 0,
        pairingMethod: 'id',
        comparisonState: citum ? 'paired' : 'oracle-only',
        compatibilityEligible: true,
      };
    });

    for (let index = 0; index < citumIds.length; index++) {
      const id = citumIds[index];
      if (!oracleIdSet.has(id)) {
        pairs.push({
          id,
          oracle: null,
          citum: citumBib[index],
          score: 0,
          pairingMethod: 'id',
          comparisonState: 'citum-only',
          compatibilityEligible: true,
        });
      }
    }
    return pairs;
  }

  const pairs = [];
  const usedOracle = new Set();
  const usedCitum = new Set();
  const candidates = [];

  // Build all candidate pairings with similarity score.
  for (let oi = 0; oi < oracleBib.length; oi++) {
    for (let ci = 0; ci < citumBib.length; ci++) {
      const score = textSimilarity(
        normalizeText(oracleBib[oi]),
        normalizeText(citumBib[ci])
      );
      // Keep weak matches out to avoid accidental cross-pairing.
      if (score >= 0.20) {
        candidates.push({ oi, ci, score });
      }
    }
  }

  // Global greedy assignment (highest-similarity edges first).
  candidates.sort((a, b) => b.score - a.score);
  for (const candidate of candidates) {
    if (usedOracle.has(candidate.oi) || usedCitum.has(candidate.ci)) continue;
    usedOracle.add(candidate.oi);
    usedCitum.add(candidate.ci);
    pairs.push({
      oracle: oracleBib[candidate.oi],
      citum: citumBib[candidate.ci],
      score: candidate.score,
      pairingMethod: 'similarity',
      comparisonState: 'paired',
      compatibilityEligible: true,
    });
  }

  // Add unmatched oracle entries.
  for (let oi = 0; oi < oracleBib.length; oi++) {
    if (!usedOracle.has(oi)) {
      pairs.push({
        oracle: oracleBib[oi],
        citum: null,
        score: 0,
        pairingMethod: 'similarity',
        comparisonState: 'unresolved-unpaired',
        compatibilityEligible: false,
      });
    }
  }

  // Add unmatched Citum entries.
  for (let ci = 0; ci < citumBib.length; ci++) {
    if (!usedCitum.has(ci)) {
      pairs.push({
        oracle: null,
        citum: citumBib[ci],
        score: 0,
        pairingMethod: 'similarity',
        comparisonState: 'unresolved-unpaired',
        compatibilityEligible: false,
      });
    }
  }

  return pairs;
}

function bibliographyComparisonMatches(styleName, comparison, caseSensitive = true) {
  if (!STRICT_BIBLIOGRAPHY_STYLES.has(styleName)) {
    return comparison.match;
  }
  if (caseSensitive) {
    return comparison.expected === comparison.actual;
  }
  return comparison.expected.toLowerCase() === comparison.actual.toLowerCase();
}

function runOracle(cliOptions = parseArgs()) {
  const stylePath = cliOptions.stylePath;
  const jsonOutput = cliOptions.jsonOutput;
  const verbose = cliOptions.verbose;
  const datasetMessage = maybeDatasetErrorForFile(stylePath, 'oracle.js');

  if (datasetMessage) {
    console.error(datasetMessage);
    process.exitCode = 2;
    return;
  }
  if (!fs.existsSync(stylePath)) {
    console.error(`Style file not found: ${stylePath}`);
    process.exitCode = 2;
    return;
  }

  const { refsData, testItems, testCitations } = loadFixtures(
    cliOptions.refsFixture,
    cliOptions.citationsFixture
  );
  const styleName = path.basename(stylePath, path.extname(stylePath));

  if (!jsonOutput) {
    console.log(`\n=== Structured Diff Oracle: ${styleName} ===\n`);
    console.log('Rendering with citeproc-js (oracle)...');
  }

  const oracle = renderWithCiteprocJs(stylePath, testItems, testCitations, cliOptions);

  if (!jsonOutput) {
    console.log('Migrating and rendering with Citum...');
  }

  const citum = renderWithCitumProcessor(stylePath, refsData, testItems, testCitations, cliOptions);

  if (!citum || citum.error) {
    if (jsonOutput) {
      console.log(JSON.stringify({
        error: 'Citum rendering failed',
        reason: citum && citum.error ? citum.error : 'Processor execution error or migration output invalid',
        style: styleName
      }));
    } else {
      console.error('❌ Citum Rendering Failed\n');
      console.error(`Style: ${styleName}`);
      if (citum && citum.error) {
        console.error(`Reason: ${citum.error}\n`);
      } else {
        console.error('Reason: Processor execution error or invalid migration output\n');
      }
      console.error('Common causes:');
      console.error('  - Invalid YAML syntax in migrated style');
      console.error('  - Unsupported schema features in migration output');
      console.error('  - Missing required fields (info.id, version)\n');
      console.error('Next Steps:');
      console.error('  1. Check migration output: cargo run --bin citum-migrate -- <csl-path>');
      console.error('  2. Validate YAML syntax: yamllint .migrated-temp.yaml');
      console.error('  3. Check processor error: cargo run --bin citum -- render refs -b <refs> -s <style> -c <citations> --mode both');
    }
    process.exitCode = 2;
    return;
  }

  // Analyze bibliography (skipped for citation-only scope, the mirror of how
  // scope: bibliography skips citations below).
  const pairs = cliOptions.scope === 'citation'
    ? []
    : matchBibliographyEntries(
      oracle.bibliography,
      citum.bibliography,
      oracle.bibliographyIds,
      citum.bibliographyIds
    );

  const rawResults = {
    style: styleName,
    citations: {
      total: cliOptions.scope === 'bibliography' ? 0 : testCitations.length,
      passed: 0,
      failed: 0,
      entries: [],
    },
    citationsByType: {},
    bibliography: {
      total: 0,
      passed: 0,
      failed: 0,
      entries: [],
    },
    componentSummary: {},
    orderingIssues: 0,
  };

  // Check citations
  if (cliOptions.scope !== 'bibliography') {
    for (const cite of testCitations) {
      const id = cite.id;
      const comparison = compareText(oracle.citations[id] || '', citum.citations[id] || '', {
        caseSensitive: cliOptions.caseSensitive,
      });
      const match = STRICT_CITATION_IDS.has(id)
        ? equivalentCitationText(comparison.expected, comparison.actual, id, {
          caseSensitive: cliOptions.caseSensitive,
        })
        : comparison.match;
      if (match) {
        rawResults.citations.passed++;
      } else {
        rawResults.citations.failed++;
      }

      // Unlike bibliography entries (which always carry ~11 tracked
      // fields, so a passing entry can shortcut to "11/11 matched"),
      // citation text rarely carries more than contributors + year --
      // hardcoding a field count for a passing citation would badly
      // overweight it relative to what's actually present. Always run
      // real component detection instead, matching or not; a true match
      // naturally yields 0 differences without needing a shortcut.
      const itemComparison = compareCitationComponents(
        comparison.expected,
        comparison.actual,
        cite.items,
        testItems
      );
      const componentsSegmented = itemComparison.segmented;
      const components = itemComparison.segmented
        ? { matches: itemComparison.matches, differences: itemComparison.differences }
        : {};

      rawResults.citations.entries.push({
        id,
        oracle: comparison.expected,
        citum: comparison.actual,
        rawOracle: comparison.rawExpected,
        rawCitum: comparison.rawActual,
        exactOracle: comparison.exactExpected,
        exactCitum: comparison.exactActual,
        exactMatch: comparison.exactMatch,
        exactAdjudication: comparison.exactAdjudication,
        match,
        caseMismatch: comparison.caseMismatch,
        components,
        componentsSegmented,
      });

      const citationTypes = collectCitationTypes(cite, testItems);
      for (const type of citationTypes) {
        if (!rawResults.citationsByType[type]) {
          rawResults.citationsByType[type] = { total: 0, passed: 0 };
        }
        rawResults.citationsByType[type].total++;
        if (match) {
          rawResults.citationsByType[type].passed++;
        }
      }
    }
  }

  // Analyze bibliography entries
  for (let i = 0; i < pairs.length; i++) {
    const pair = pairs[i];
    const entryResult = {
      index: i + 1,
      id: pair.id || null,
      oracle: pair.oracle ? normalizeText(pair.oracle) : null,
      citum: pair.citum ? normalizeText(pair.citum) : null,
      rawOracle: pair.oracle ?? null,
      rawCitum: pair.citum ?? null,
      exactOracle: null,
      exactCitum: null,
      exactMatch: null,
      exactAdjudication: 'not-comparable',
      match: pair.compatibilityEligible ? false : null,
      caseMismatch: false,
      pairingMethod: pair.pairingMethod,
      comparisonState: pair.comparisonState,
      compatibilityEligible: pair.compatibilityEligible,
      exactParityEligible: pair.comparisonState === 'paired',
      components: {},
      ordering: null,
      issues: [],
    };

    if (!pair.oracle) {
      if (pair.compatibilityEligible) {
        entryResult.issues.push({ issue: 'extra_entry', detail: 'ID-proven entry in Citum but not oracle' });
        rawResults.bibliography.total++;
        rawResults.bibliography.failed++;
      } else {
        entryResult.issues.push({ issue: 'unpaired_output', detail: 'Similarity pairing found no benchmark counterpart' });
      }
    } else if (!pair.citum) {
      if (pair.compatibilityEligible) {
        entryResult.issues.push({ issue: 'missing_entry', detail: 'ID-proven entry in oracle but not Citum' });
        rawResults.bibliography.total++;
        rawResults.bibliography.failed++;
      } else {
        entryResult.issues.push({ issue: 'unpaired_output', detail: 'Similarity pairing found no Citum counterpart' });
      }
    } else {
      rawResults.bibliography.total++;
      // Both exist - compare
      const comparison = compareText(pair.oracle, pair.citum, {
        caseSensitive: cliOptions.caseSensitive,
      });
      entryResult.oracle = comparison.expected;
      entryResult.citum = comparison.actual;
      entryResult.rawOracle = comparison.rawExpected;
      entryResult.rawCitum = comparison.rawActual;
      entryResult.exactOracle = comparison.exactExpected;
      entryResult.exactCitum = comparison.exactActual;
      entryResult.exactMatch = comparison.exactMatch;
      entryResult.exactAdjudication = comparison.exactAdjudication;
      entryResult.caseMismatch = comparison.caseMismatch;
      const match = bibliographyComparisonMatches(
        styleName,
        comparison,
        cliOptions.caseSensitive
      );

      if (match) {
        entryResult.match = true;
        rawResults.bibliography.passed++;
      } else {
        rawResults.bibliography.failed++;

        // Find reference data for this entry
        const refData = pair.id
          ? testItems[pair.id]
          : findRefDataForEntry(pair.oracle, testItems);

        // Parse components (only if reference data found)
        if (refData) {
          const oracleComp = parseComponents(pair.oracle, refData);
          const citumComp = parseComponents(pair.citum, refData);

          // Compare components
          const { differences, matches } = compareComponents(oracleComp, citumComp, refData);
          entryResult.components = { differences, matches };

          // Analyze ordering
          const oracleOrder = analyzeOrdering(pair.oracle, refData);
          const citumOrder = analyzeOrdering(pair.citum, refData);
          const orderIssues = compareOrdering(oracleOrder, citumOrder);

          if (orderIssues.length > 0) {
            entryResult.ordering = { oracle: oracleOrder, citum: citumOrder };
            rawResults.orderingIssues++;
          }

          entryResult.issues = [...differences, ...orderIssues];

          // Track component issues for summary
          for (const diff of differences) {
            const key = `${diff.component}:${diff.issue}`;
            rawResults.componentSummary[key] = (rawResults.componentSummary[key] || 0) + 1;
          }
        } else {
          // No reference data found - skip component analysis
          entryResult.issues = [];
        }
      }
    }

    rawResults.bibliography.entries.push(entryResult);
  }

  const results = attachRegisteredDivergenceAdjustments(
    rawResults,
    oracle.bibliography,
    citum.bibliographyOrderIds || [],
    testItems,
    testCitations,
    cliOptions.scope === 'citation' ? null : oracle.bibliographyIds || null
  );
  results.summary = buildOracleSummary(results, cliOptions.scope);

  // Output
  if (jsonOutput) {
    console.log(JSON.stringify(results, null, 2));
  } else {
    // Human-readable output
    console.log('\n--- CITATIONS ---');
      console.log(`  ✅ Passed: ${results.citations.passed}/${results.citations.total}`);
    if (results.citations.failed > 0) {
      console.log(`  ❌ Failed: ${results.citations.failed}/${results.citations.total}`);
    }

    console.log('\n--- BIBLIOGRAPHY ---');
    console.log(`  ✅ Passed: ${results.bibliography.passed}/${results.bibliography.total}`);
    console.log(`  ❌ Failed: ${results.bibliography.failed}/${results.bibliography.total}`);

    if (Object.keys(results.componentSummary).length > 0) {
      console.log('\n--- COMPONENT ISSUES ---');
      const sorted = Object.entries(results.componentSummary)
        .sort((a, b) => b[1] - a[1]);
      for (const [issue, count] of sorted) {
        console.log(`  ${issue}: ${count} entries`);
      }
    }

    if (results.orderingIssues > 0) {
      console.log(`\n--- ORDERING ISSUES: ${results.orderingIssues} entries ---`);
    }

    if (results.bibliographyOrder) {
      const explainedNote = results.bibliographyOrder.explained
        ? ` (explained by ${JSON.stringify(results.bibliographyOrder.appliedDivergence)})`
        : ' (unexplained)';
      console.log(`\n--- BIBLIOGRAPHY ORDER MISMATCH${explainedNote} ---`);
      if (results.bibliographyOrder.firstDivergentIndex !== null) {
        console.log(`  First divergent position: ${results.bibliographyOrder.firstDivergentIndex}`);
      }
      console.log(`  Order Oracle: ${results.bibliographyOrder.oracleOrderIds.join(' → ')}`);
      console.log(`  Order Citum:  ${results.bibliographyOrder.citumOrderIds.join(' → ')}`);
    }

    if (verbose) {
      console.log('\n--- DETAILED FAILURES ---');

      // Citation failures
      const failedCitations = results.citations.entries.filter(e => !e.match);
      if (failedCitations.length > 0) {
        console.log('\nCitations:');
        for (const entry of failedCitations) {
          console.log(`  [${entry.id}]`);
          console.log(`    Oracle: ${entry.oracle}`);
          console.log(`    Citum:   ${entry.citum}`);
        }
      }

      // Bibliography failures
      for (const entry of results.bibliography.entries) {
        if (!entry.match && entry.oracle && entry.citum) {
          console.log(`\nEntry ${entry.index}:`);
          console.log(`  Oracle: ${entry.oracle}`);
          console.log(`  Citum:   ${entry.citum}`);
          if (entry.ordering) {
            console.log(`  Order Oracle: ${entry.ordering.oracle.join(' → ')}`);
            console.log(`  Order Citum:   ${entry.ordering.citum.join(' → ')}`);
          }
          for (const issue of entry.issues) {
            console.log(`  Issue: ${issue.component || issue.issue}: ${issue.detail || ''}`);
          }
        }
      }
    }

    console.log('\n=== SUMMARY ===');
    console.log(`Citations: ${results.citations.passed}/${results.citations.total} match`);
    console.log(`Bibliography: ${results.bibliography.passed}/${results.bibliography.total} match`);
    console.log();
  }

  const citationFailures = cliOptions.scope === 'bibliography' ? 0 : results.citations.failed;
  process.exitCode = citationFailures === 0 && results.bibliography.failed === 0 ? 0 : 1;
}

if (require.main === module) {
  runOracle();
}

module.exports = {
  bibliographyComparisonMatches,
  compareComponents,
  cleanupOracleTempWorkspace,
  createOracleTempWorkspace,
  loadFixtures,
  matchBibliographyEntries,
  normalizeFixtureItems,
  parseArgs,
  parseCitumRenderOutput,
  refsDataForProcessor,
  expandCitationsForCitum,
  collapseClusteredCitumCitations,
  renderWithCitumProcessor,
  resolveAuthoredStylePath,
  runOracle,
  STRICT_BIBLIOGRAPHY_STYLES,
};
