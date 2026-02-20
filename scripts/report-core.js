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
 *   node report-core.js --styles-dir /path/to/csl            # Override CSL directory
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const yaml = require('js-yaml');

const CUSTOM_TAG_SCHEMA = yaml.DEFAULT_SCHEMA.extend([
  new yaml.Type('!custom', {
    kind: 'mapping',
    construct(data) {
      return data || {};
    },
  }),
]);

const STYLE_METADATA = {
  'apa-7th': { source: 'apa', dependents: 783, format: 'author-date' },
  'elsevier-with-titles': { dependents: 672, format: 'numeric' },
  'elsevier-harvard': { dependents: 665, format: 'author-date' },
  'elsevier-vancouver': { dependents: 502, format: 'numeric' },
  'springer-vancouver-brackets': { dependents: 472, format: 'numeric' },
  'springer-basic-author-date': { dependents: 460, format: 'author-date' },
  'springer-basic-brackets': { dependents: 352, format: 'numeric' },
  'springer-socpsych-author-date': { dependents: 317, format: 'author-date' },
  'american-medical-association': { dependents: 293, format: 'numeric' },
  'taylor-and-francis-chicago-author-date': { dependents: 234, format: 'author-date' },
  'springer-mathphys-brackets': { dependents: 201, format: 'numeric' },
  'multidisciplinary-digital-publishing-institute': { dependents: 180, format: 'numeric' },
  'ieee': { dependents: 176, format: 'numeric' },
  'nlm-citation-sequence-superscript': { dependents: 121, format: 'numeric' },
  'nlm-citation-sequence': { dependents: 116, format: 'numeric' },
  'karger-journals': { dependents: 85, format: 'numeric' },
  'institute-of-physics-numeric': { dependents: 82, format: 'numeric' },
  'thieme-german': { dependents: 74, format: 'numeric' },
  'mary-ann-liebert-vancouver': { dependents: 72, format: 'numeric' },
  'biomed-central': { dependents: 66, format: 'numeric' },
  'chicago-notes': { dependents: 5, format: 'note', hasBibliography: false },
};

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

/**
 * Parse command-line arguments
 */
function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    writeHtml: false,
    outputHtml: null,
    stylesDir: null,
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--write-html') {
      options.writeHtml = true;
    } else if (args[i] === '--output-html') {
      options.outputHtml = args[++i];
      options.writeHtml = true;
    } else if (args[i] === '--styles-dir') {
      options.stylesDir = args[++i];
    }
  }

  return options;
}

/**
 * Get git short commit hash or 'unknown' on error
 */
function getGitCommit() {
  try {
    return execSync('git rev-parse --short HEAD', {
      cwd: path.dirname(__dirname),
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

  const projectRoot = path.dirname(__dirname);
  const defaultDir = path.join(projectRoot, 'styles-legacy');

  if (fs.existsSync(defaultDir)) {
    return defaultDir;
  }

  throw new Error(`Styles directory not found. Use --styles-dir to specify path.`);
}

function discoverCoreStyles() {
  const stylesRoot = path.join(path.dirname(__dirname), 'styles');
  if (!fs.existsSync(stylesRoot)) {
    throw new Error(`Core styles directory not found: ${stylesRoot}`);
  }

  return Object.entries(STYLE_METADATA)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, meta]) => {
      const stylePath = path.join(stylesRoot, `${name}.yaml`);
      if (!fs.existsSync(stylePath)) {
        throw new Error(`Core style YAML not found: ${stylePath}`);
      }
      return {
        name,
        sourceName: meta.source || name,
        dependents: meta.dependents ?? null,
        format: meta.format || 'unknown',
        hasBibliography: meta.hasBibliography ?? true,
      };
    });
}

/**
 * Run oracle.js for a single style and parse output
 */
function runOracle(stylePath, styleName, styleFormat) {
  const oracleScript = path.join(__dirname, 'oracle.js');
  const noteCitationsFixture = path.join(
    path.dirname(__dirname),
    'tests',
    'fixtures',
    'citations-note-expanded.json'
  );
  const command = styleFormat === 'note'
    ? `node "${oracleScript}" "${stylePath}" --json --citations-fixture "${noteCitationsFixture}"`
    : `node "${oracleScript}" "${stylePath}" --json`;

  try {
    const result = execSync(command, {
      encoding: 'utf8',
      timeout: 120000,
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    return JSON.parse(result);
  } catch (error) {
    // Exit code 1 means validation failed but execution was successful (JSON is in stdout)
    if (error.status === 1 && error.stdout) {
      try {
        return JSON.parse(error.stdout.toString());
      } catch {
        return { error: `Oracle execution failed to parse JSON: ${error.message}`, style: styleName };
      }
    }
    // Fatal error (exit code 2 or other)
    const stderr = error.stderr ? error.stderr.toString() : '';
    return { error: `Oracle fatal error: ${error.message}\n${stderr}`, style: styleName };
  }
}

/**
 * Compute fidelity score from oracle result
 */
function computeFidelityScore(oracleResult) {
  if (oracleResult.error) {
    return 0;
  }

  const citations = oracleResult.citations || {};
  const bibliography = oracleResult.bibliography || {};

  const citationsPassed = citations.passed || 0;
  const citationsTotal = citations.total || 0;
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

function loadStyleYaml(styleName) {
  const stylePath = path.join(path.dirname(__dirname), 'styles', `${styleName}.yaml`);
  if (!fs.existsSync(stylePath)) {
    return { stylePath, styleData: null, error: `Style YAML not found: ${stylePath}` };
  }
  try {
    const styleData = yaml.load(fs.readFileSync(stylePath, 'utf8'), { schema: CUSTOM_TAG_SCHEMA });
    return { stylePath, styleData, error: null };
  } catch (error) {
    return { stylePath, styleData: null, error: `YAML parse error: ${error.message}` };
  }
}

function flattenTemplateComponents(components) {
  const flattened = [];
  for (const component of components || []) {
    if (!component || typeof component !== 'object') continue;
    flattened.push(component);
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
  const scopes = [];

  if (Array.isArray(citation.template)) {
    scopes.push({ name: 'citation.template', components: citation.template });
  }
  if (Array.isArray(citation.integral?.template)) {
    scopes.push({ name: 'citation.integral.template', components: citation.integral.template });
  }
  if (Array.isArray(citation['non-integral']?.template)) {
    scopes.push({ name: 'citation.non-integral.template', components: citation['non-integral'].template });
  }
  if (Array.isArray(bibliography.template)) {
    scopes.push({ name: 'bibliography.template', components: bibliography.template });
  }

  for (const [typeKey, template] of Object.entries(typeTemplates)) {
    if (Array.isArray(template)) {
      scopes.push({
        name: `bibliography.type-templates.${typeKey}`,
        components: template,
      });
    }
  }

  return scopes;
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

function countTemplatePresetUses(node) {
  let count = 0;
  function visit(value) {
    if (!value || typeof value !== 'object') return;
    if (Array.isArray(value)) {
      for (const item of value) visit(item);
      return;
    }
    for (const [key, child] of Object.entries(value)) {
      if (key === 'use-preset') count += 1;
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
        } else if (typeof value['use-preset'] === 'string' && value['use-preset'].trim()) {
          uses += 1;
          fields.push(key);
        }
      }
    }
  }

  return { uses, fields };
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
  const typeTemplateSet = new Set(Object.keys(typeTemplates));
  const assessedTypes = CORE_FALLBACK_TYPES.filter((type) => !typeTemplateSet.has(type));
  const flattenedBase = flattenTemplateComponents(Array.isArray(bibliography.template) ? bibliography.template : []);

  if (assessedTypes.length === 0) {
    return {
      score: 100,
      assessedTypes: 0,
      passingTypes: 0,
      note: 'all core types have explicit type-templates',
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
  const scopes = collectTemplateScopes(styleData);
  const scopedComponents = scopes
    .map((scope) => ({
      name: scope.name,
      components: flattenTemplateComponents(scope.components),
    }))
    .filter((scope) => scope.components.length > 0);
  const flattened = scopedComponents.flatMap((scope) => scope.components);

  if (flattened.length === 0) {
    return {
      score: 0,
      totalComponents: 0,
      duplicates: 0,
      withinScopeDuplicates: 0,
      crossScopeRepeats: 0,
      overrideDensity: 0,
      targetComponents: 0,
    };
  }

  const semanticKeys = flattened.map(componentSemanticKey);
  let withinScopeDuplicates = 0;
  const keyScopeCount = new Map();

  for (const scope of scopedComponents) {
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

  const weightedDuplicates = withinScopeDuplicates + (crossScopeRepeats * 0.25);
  const duplicateRatio = weightedDuplicates / semanticKeys.length;
  const overrideCount = flattened.reduce(
    (sum, component) => sum + Object.keys(component.overrides || {}).length,
    0
  );
  const overrideDensity = overrideCount / flattened.length;
  const typeTemplateCoverage = Object.keys(styleData?.bibliography?.['type-templates'] || {})
    .reduce((sum, rawKey) => {
      const parsed = parseOverrideKey(rawKey).filter((key) => key !== 'default');
      return sum + (parsed.length || 1);
    }, 0);

  const componentTargets = {
    'author-date': 52,
    numeric: 55,
    note: 65,
  };
  const targetBase = componentTargets[format] || 55;
  const targetBonus = clamp(0, 35, Math.max(0, typeTemplateCoverage - 3) * 2.5);
  const target = targetBase + targetBonus;
  const componentPenalty = Math.max(0, flattened.length - target) * 0.9;
  const duplicatePenalty = duplicateRatio * 24;
  const overridePenalty = Math.max(0, overrideDensity - 1.5) * 12;
  const score = 100 - componentPenalty - duplicatePenalty - overridePenalty;

  return {
    score: safePct(score),
    totalComponents: flattened.length,
    duplicates: parseFloat(weightedDuplicates.toFixed(1)),
    withinScopeDuplicates,
    crossScopeRepeats,
    overrideDensity: parseFloat(overrideDensity.toFixed(2)),
    targetComponents: parseFloat(target.toFixed(1)),
  };
}

function computePresetUsageScore(styleData, concisionScore) {
  const templateUses = countTemplatePresetUses(styleData);
  const { uses: optionUses, fields: optionPresetFields } = countOptionsPresetUses(styleData);
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

function computeQualityMetrics(styleSpec, oracleResult) {
  const loaded = loadStyleYaml(styleSpec.name);
  if (!loaded.styleData) {
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

  const typeCoverage = computeTypeCoverageScore(oracleResult.citationsByType || {});
  let fallbackRobustness = computeFallbackRobustness(loaded.styleData);
  const concision = computeConcisionScore(loaded.styleData, styleSpec.format);
  const presetUsage = computePresetUsageScore(loaded.styleData, concision.score);
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

/**
 * Generate compatibility report
 */
function generateReport(options) {
  const stylesDir = getStylesDir(options.stylesDir);
  const coreStyles = discoverCoreStyles();
  const divergences = loadDivergences();

  const styles = [];
  let citationsTotal = 0;
  let citationsPassed = 0;
  let biblioTotal = 0;
  let biblioPassed = 0;
  let qualityTotal = 0;
  let qualityCount = 0;
  let errorCount = 0;

  for (const styleSpec of coreStyles) {
    const stylePath = path.join(stylesDir, `${styleSpec.sourceName}.csl`);

    if (!fs.existsSync(stylePath)) {
      styles.push({
        name: styleSpec.name,
        dependents: styleSpec.dependents,
        format: styleSpec.format,
        hasBibliography: styleSpec.hasBibliography,
        impactPct: styleSpec.dependents != null
          ? (styleSpec.dependents / TOTAL_DEPENDENTS * 100).toFixed(2)
          : null,
        fidelityScore: 0,
        citations: { passed: 0, total: 0 },
        bibliography: { passed: 0, total: 0 },
        knownDivergences: divergences[styleSpec.name] || [],
        citationsByType: {},
        error: `Style file not found: ${stylePath}`,
        oracleDetail: null,
        qualityScore: 0,
        qualityBreakdown: null,
      });
      errorCount++;
      continue;
    }

    const oracleResult = runOracle(stylePath, styleSpec.name, styleSpec.format);
    if (oracleResult.error) {
      errorCount++;
      process.stderr.write(`Error processing ${styleSpec.name}: ${oracleResult.error}\n`);
    }
    const fidelityScore = computeFidelityScore(oracleResult);

    const citations = oracleResult.citations || { passed: 0, total: 0 };
    const bibliography = oracleResult.bibliography || { passed: 0, total: 0 };

    citationsTotal += citations.total || 0;
    citationsPassed += citations.passed || 0;
    biblioTotal += bibliography.total || 0;
    biblioPassed += bibliography.passed || 0;

    const componentMatchRate = computeComponentMatchRate(oracleResult);
    const qualityMetrics = computeQualityMetrics(styleSpec, oracleResult);
    const qualityScore = qualityMetrics.score / 100;
    qualityTotal += qualityScore;
    qualityCount += 1;

    let statusTier = 'failing';
    if (oracleResult.error) {
      statusTier = 'error';
    } else if (fidelityScore === 1.0) {
      statusTier = 'perfect';
    } else if (fidelityScore > 0) {
      statusTier = 'partial';
    }

    styles.push({
      name: styleSpec.name,
      dependents: styleSpec.dependents,
      format: styleSpec.format,
      hasBibliography: styleSpec.hasBibliography,
      impactPct: styleSpec.dependents != null
        ? (styleSpec.dependents / TOTAL_DEPENDENTS * 100).toFixed(2)
        : null,
      fidelityScore: parseFloat(fidelityScore.toFixed(3)),
      citations,
      bibliography,
      knownDivergences: divergences[styleSpec.name] || [],
      citationsByType: oracleResult.citationsByType || {},
      error: oracleResult.error || null,
      componentMatchRate,
      statusTier,
      componentSummary: oracleResult.componentSummary || {},
      citationEntries: oracleResult.citations ? oracleResult.citations.entries : null,
      oracleDetail: oracleResult.bibliography ? oracleResult.bibliography.entries : null,
      qualityScore: parseFloat(qualityScore.toFixed(3)),
      qualityBreakdown: qualityMetrics,
    });
  }

  const knownDependents = coreStyles
    .filter((s) => typeof s.dependents === 'number')
    .reduce((sum, s) => sum + s.dependents, 0);
  const totalImpact = ((knownDependents / TOTAL_DEPENDENTS) * 100).toFixed(2);

  return {
    report: {
      generated: getTimestamp(),
      commit: getGitCommit(),
      totalImpact: parseFloat(totalImpact),
      totalStyles: coreStyles.length,
      citationsOverall: { passed: citationsPassed, total: citationsTotal },
      bibliographyOverall: { passed: biblioPassed, total: biblioTotal },
      qualityOverall: {
        score: qualityCount > 0 ? parseFloat((qualityTotal / qualityCount).toFixed(3)) : 0,
      },
      styles,
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
  const tableHtml = generateHtmlTable(report);
  const footerHtml = generateHtmlFooter();

  return `${headerHtml}${statsHtml}${tableHtml}${footerHtml}`;
}

function generateHtmlHeader(report) {
  const generatedDate = new Date(report.generated).toUTCString();
  return `<!-- Auto-generated by report-core.js. Do not edit manually. -->
<!DOCTYPE html>
<html lang="en" class="scroll-smooth">

<head>
    <meta charset="utf-8" />
    <meta content="width=device-width, initial-scale=1.0" name="viewport" />
    <title>CSLN | Style Compatibility Report</title>
    <meta name="description"
        content="Compatibility metrics for CSLN against citeproc-js reference implementation.">

    <script src="https://cdn.tailwindcss.com?plugins=forms,container-queries,typography"></script>
    <link
        href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap"
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
                        "display": ["Inter", "sans-serif"],
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
            font-family: 'Inter', sans-serif;
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
</head>

<body class="bg-background-light text-slate-700 selection:bg-primary/20">

    <!-- Navigation -->
    <nav class="fixed top-0 w-full z-50 glass-nav">
        <div class="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
            <div class="flex items-center gap-2">
                <div class="w-8 h-8 bg-primary rounded flex items-center justify-center">
                    <span class="text-white font-mono font-bold">C</span>
                </div>
                <span class="font-mono text-xl font-bold tracking-tight text-slate-900">CSLN</span>
            </div>
            <div class="hidden md:flex items-center gap-8">
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="index.html#features">Features</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="index.html#roadmap">Roadmap</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="interactive-demo.html">Demo</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="examples.html">Examples</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="compat.html" style="color: #2a94d6; font-weight: 600;">Compat</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="index.html#schemas">Schemas</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="https://github.com/bdarcus/csl26">GitHub</a>
            </div>
        </div>
    </nav>

    <!-- Header Section -->
    <header class="pt-24 pb-12 px-6 border-b border-slate-200">
        <div class="max-w-7xl mx-auto">
            <div class="flex items-center justify-between mb-6">
                <div>
                    <h1 class="text-4xl md:text-5xl font-mono font-bold tracking-tight text-slate-900 mb-2">
                        Style Compatibility Report
                    </h1>
                    <p class="text-slate-500">Compatibility metrics for core styles in <code>styles/</code></p>
                </div>
            </div>
            <div class="flex flex-col sm:flex-row gap-4 items-start sm:items-center">
                <div class="text-sm text-slate-500 font-mono">Generated: ${generatedDate}</div>
                <div class="inline-flex items-center gap-2 px-3 py-1 rounded bg-slate-100 text-slate-700 text-xs font-mono border border-slate-200">
                    <span class="material-icons text-sm">code</span>
                    <span>${escapeHtml(report.commit)}</span>
                </div>
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

  return `
    <!-- Statistics Cards -->
    <section class="py-12 px-6 bg-accent-cream">
        <div class="max-w-7xl mx-auto">
            <div class="grid grid-cols-1 md:grid-cols-4 gap-6">
                <!-- Core Styles -->
                <div class="bg-white rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Core Styles</div>
                    <div class="text-3xl font-bold text-slate-900">${report.totalStyles}</div>
                    <div class="text-xs text-slate-400 mt-2">${report.totalImpact}% known dependent coverage</div>
                </div>

                <!-- Citations Overall -->
                <div class="bg-white rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Citations</div>
                    <div class="text-3xl font-bold text-slate-900">${report.citationsOverall.passed}/${report.citationsOverall.total}</div>
                    <div class="text-xs text-slate-400 mt-2">${citationsPct}% pass rate</div>
                </div>

                <!-- Bibliography Overall -->
                <div class="bg-white rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Bibliography</div>
                    <div class="text-3xl font-bold text-slate-900">${report.bibliographyOverall.passed}/${report.bibliographyOverall.total}</div>
                    <div class="text-xs text-slate-400 mt-2">${biblioPct}% pass rate</div>
                </div>

                <!-- Quality Overall -->
                <div class="bg-white rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Quality (SQI)</div>
                    <div class="text-3xl font-bold text-slate-900">${qualityPct}%</div>
                    <div class="text-xs text-slate-400 mt-2">Type coverage, fallback, concision, presets</div>
                </div>
            </div>
        </div>
    </section>
`;
}

function generateHtmlTable(report) {
  let tableRows = '';

  for (const style of report.styles) {
    const fidelityPct = (style.fidelityScore * 100).toFixed(1);
    const qualityPct = ((style.qualityScore || 0) * 100).toFixed(1);
    const citationRate = style.citations.total > 0 ? style.citations.passed / style.citations.total : -1;
    const bibliographyRate = style.hasBibliography && style.bibliography.total > 0
      ? style.bibliography.passed / style.bibliography.total
      : -1;
    const dependentsValue = style.dependents ?? -1;
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
                    data-style-name="${escapeHtml(style.name.toLowerCase())}"
                    data-format="${escapeHtml(String(style.format).toLowerCase())}"
                    data-dependents="${dependentsValue}"
                    data-citation-rate="${citationRate}"
                    data-bibliography-rate="${bibliographyRate}"
                    data-component-rate="${componentRateValue}"
                    data-fidelity="${style.fidelityScore}"
                    data-quality="${style.qualityScore || 0}"
                    data-sqi-tier-rank="${sqiTierRank}">
                    <td class="px-6 py-4 text-sm font-medium text-slate-900">${style.name}</td>
                    <td class="px-6 py-4 text-sm text-slate-600">${style.format}</td>
                    <td class="px-6 py-4 text-sm text-slate-600">${style.dependents ?? '—'}</td>
                    <td class="px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${citationBadge}">
                            ${style.citations.passed}/${style.citations.total}
                        </span>
                    </td>
                    <td class="px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${biblioBadge}">
                            ${biblioText}
                        </span>
                    </td>
                    <td class="px-6 py-4">
                        ${componentRateHtml}
                    </td>
                    <td class="px-6 py-4 text-sm font-mono text-slate-600">${fidelityPct}%</td>
                    <td class="px-6 py-4 text-sm font-mono text-slate-600">${qualityPct}%</td>
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
                    <td colspan="10" class="px-6 py-4 bg-slate-50">
                        <div class="max-w-4xl">
${generateDetailContent(style)}
                        </div>
                    </td>
                </tr>
    `;
  }

  return `
    <!-- Compatibility Table -->
    <section class="py-12 px-6">
        <div class="max-w-7xl mx-auto">
            <div class="rounded-xl border border-slate-200 overflow-hidden">
                <table class="w-full">
                    <thead class="bg-slate-50 border-b border-slate-200">
                        <tr>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('style-name')">
                                    Style <span class="text-slate-400" id="sort-ind-style-name">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('format')">
                                    Format <span class="text-slate-400" id="sort-ind-format">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('dependents')">
                                    Dependents <span class="text-slate-400" id="sort-ind-dependents">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('citation-rate')">
                                    Citations <span class="text-slate-400" id="sort-ind-citation-rate">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('bibliography-rate')">
                                    Bibliography <span class="text-slate-400" id="sort-ind-bibliography-rate">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('component-rate')">
                                    Components <span class="text-slate-400" id="sort-ind-component-rate">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
                                <button class="inline-flex items-center gap-1 hover:text-primary transition-colors" onclick="sortCompatTable('fidelity')">
                                    Fidelity <span class="text-slate-400" id="sort-ind-fidelity">↕</span>
                                </button>
                            </th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">
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

function generateDetailContent(style) {
  let html = '';

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
    html += `
                            <div class="mb-4 p-3 rounded border border-slate-200 bg-white">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Quality (SQI): ${overall}%</div>
                                <div class="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs font-mono">
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">type ${typeCoverage.toFixed(1)}%</div>
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">fallback ${fallback.toFixed(1)}%</div>
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">concision ${concision.toFixed(1)}%</div>
                                    <div class="px-2 py-1 rounded bg-slate-100 text-slate-700">presets ${presets.toFixed(1)}%</div>
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
    const failedEntries = style.citationEntries.filter(e => !e.match);
    if (failedEntries.length === 0) {
      html += `
                            <div class="mb-4 p-3 rounded bg-emerald-50 border border-emerald-200">
                                <div class="text-xs font-semibold text-emerald-700">All ${style.citationEntries.length} citations match ✓</div>
                            </div>
`;
    } else {
      html += `
                            <div class="mb-4">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Failed Citations (${failedEntries.length}/${style.citationEntries.length})</div>
                                <div class="overflow-x-auto">
                                    <table class="w-full text-xs border-collapse">
                                        <thead>
                                            <tr class="border-b border-slate-300 bg-slate-100">
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">#</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Oracle</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">CSLN</th>
                                                <th class="text-center px-2 py-1 font-medium text-slate-700">Match</th>
                                            </tr>
                                        </thead>
                                        <tbody>
`;
      for (const entry of failedEntries) {
        const oracleText = entry.oracle ? entry.oracle.substring(0, 100) : '(empty)';
        const cslnText = entry.csln ? entry.csln.substring(0, 100) : '(empty)';
        html += `
                                            <tr class="border-b border-slate-200 hover:bg-slate-50">
                                                <td class="px-2 py-1 text-slate-600">${escapeHtml(entry.id)}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(entry.oracle || '')}">${escapeHtml(oracleText)}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(entry.csln || '')}">${escapeHtml(cslnText)}</td>
                                                <td class="px-2 py-1 text-center font-bold text-red-600">✗</td>
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

  if (style.oracleDetail && style.oracleDetail.length > 0) {
    html += `
                            <div class="mt-4">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Bibliography Entries (${style.oracleDetail.length})</div>
                                <div class="overflow-x-auto">
                                    <table class="w-full text-xs border-collapse">
                                        <thead>
                                            <tr class="border-b border-slate-300 bg-slate-100">
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">#</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Oracle</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">CSLN</th>
                                                <th class="text-center px-2 py-1 font-medium text-slate-700">Match</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Issues</th>
                                            </tr>
                                        </thead>
                                        <tbody>
`;

    for (let i = 0; i < style.oracleDetail.length; i++) {
      const entry = style.oracleDetail[i];
      const matchIcon = entry.match === true ? '✓' : entry.match === false ? '✗' : '–';
      const matchColor = entry.match === true ? 'text-emerald-600' : entry.match === false ? 'text-red-600' : 'text-slate-400';

      const oracleText = entry.oracle ? entry.oracle.substring(0, 100) : '(empty)';
      const cslnText = entry.csln ? entry.csln.substring(0, 100) : '(empty)';

      let issuesText = '—';
      if (!entry.match) {
        if (entry.issues && entry.issues.length > 0) {
          issuesText = entry.issues
            .map(iss => iss.component ? `${iss.component}:${iss.issue}` : iss.issue)
            .join(', ');
        }
      }

      html += `
                                            <tr class="border-b border-slate-200 hover:bg-slate-50">
                                                <td class="px-2 py-1 text-slate-600">${i + 1}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(entry.oracle || '')}">${escapeHtml(oracleText)}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600 text-xs" title="${escapeHtml(entry.csln || '')}">${escapeHtml(cslnText)}</td>
                                                <td class="px-2 py-1 text-center font-bold ${matchColor}">${matchIcon}</td>
                                                <td class="px-2 py-1 text-slate-600 text-xs font-mono">${escapeHtml(issuesText)}</td>
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

  if (style.knownDivergences && style.knownDivergences.length > 0) {
    html += `
                            <div class="p-4 rounded-lg bg-primary/5 border border-primary/20 mt-4">
                                <div class="text-sm font-semibold text-primary mb-2">CSLN Extensions</div>
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
    <footer class="py-12 px-6 border-t border-slate-200 bg-white">
        <div class="max-w-7xl mx-auto">
            <div class="flex flex-col md:flex-row justify-between items-center gap-8">
                <div class="flex items-center gap-2">
                    <div class="w-6 h-6 bg-primary rounded flex items-center justify-center">
                        <span class="text-white font-mono text-xs font-bold">C</span>
                    </div>
                    <span class="font-mono text-lg font-bold text-slate-900">CSLN</span>
                </div>
                <div class="flex gap-8 text-sm font-medium text-slate-500">
                    <a class="hover:text-primary transition-colors" href="https://github.com/bdarcus/csl26">GitHub</a>
                    <a class="hover:text-primary transition-colors" href="index.html#roadmap">Roadmap</a>
                    <a class="hover:text-primary transition-colors" href="examples.html">Examples</a>
                </div>
                <div class="text-sm text-slate-400">
                    © 2026 CSLN Project. MIT Licensed.
                </div>
            </div>
        </div>
    </footer>

    <script>
        const sortState = { key: null, direction: 1 };

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

            const summaryRows = Array.from(tbody.querySelectorAll('tr.accordion-toggle'));
            const rowPairs = summaryRows.map((summary) => {
                const detailId = summary.dataset.detailId;
                const detail = detailId ? document.getElementById(detailId) : null;
                return { summary, detail };
            });

            const defaultAsc = key === 'style-name' || key === 'format';
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

            rowPairs.sort((a, b) => {
                const left = a.summary.dataset[key] || '';
                const right = b.summary.dataset[key] || '';
                const numericKeys = new Set([
                    'dependents',
                    'citation-rate',
                    'bibliography-rate',
                    'component-rate',
                    'fidelity',
                    'quality',
                    'sqi-tier-rank',
                ]);

                if (numericKeys.has(key)) {
                    return (asNumber(left) - asNumber(right)) * sortState.direction;
                }
                return asText(left).localeCompare(asText(right)) * sortState.direction;
            });

            for (const pair of rowPairs) {
                tbody.appendChild(pair.summary);
                if (pair.detail) tbody.appendChild(pair.detail);
            }

            updateSortIndicators(key, sortState.direction);
        }
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
function main() {
  try {
    const options = parseArgs();
    const { report, errorCount } = generateReport(options);

    // Output JSON to stdout
    console.log(JSON.stringify(report, null, 2));

    // Generate and write HTML if requested
    if (options.writeHtml) {
      const htmlPath = options.outputHtml || path.join(path.dirname(__dirname), 'docs', 'compat.html');
      const htmlDir = path.dirname(htmlPath);

      if (!fs.existsSync(htmlDir)) {
        fs.mkdirSync(htmlDir, { recursive: true });
      }

      const htmlContent = generateHtml(report);
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

main();
