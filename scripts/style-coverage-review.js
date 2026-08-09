#!/usr/bin/env node
/**
 * Build an auditable populated-field coverage and exact-parity packet.
 *
 * Coverage is structural inference from a style resolved by Citum itself. It
 * is not a claim that a conditional component consumed a value at runtime.
 */

'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const Ajv = require('ajv');
const yaml = require('js-yaml');

const PROJECT_ROOT = path.dirname(__dirname);
const GENERATOR_PATH = path.join(PROJECT_ROOT, 'scripts', 'style-coverage-review.js');
const MANIFEST_SCHEMA_PATH = path.join(
  PROJECT_ROOT,
  'scripts',
  'report-data',
  'style-coverage-audit-manifest.schema.json'
);
const MANIFEST_SCHEMA = 'citum.style-coverage-audit-manifest/v1';
const PACKET_SCHEMA = 'citum.style-coverage-packet/v1';
const GENERATOR_VERSION = 1;

function codepointCompare(left, right) {
  if (left === right) return 0;
  return left < right ? -1 : 1;
}

function stableValue(value) {
  if (Array.isArray(value)) return value.map(stableValue);
  if (!value || typeof value !== 'object') return value;
  return Object.fromEntries(
    Object.keys(value)
      .sort(codepointCompare)
      .map((key) => [key, stableValue(value[key])])
  );
}

function stableJson(value) {
  return `${JSON.stringify(stableValue(value), null, 2)}\n`;
}

function sha256(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function sha256File(filePath) {
  return sha256(fs.readFileSync(filePath));
}

function readData(filePath) {
  const source = fs.readFileSync(filePath, 'utf8');
  return /\.ya?ml$/i.test(filePath) ? (yaml.load(source) || {}) : JSON.parse(source);
}

function resolveRepoPath(filePath) {
  return path.isAbsolute(filePath) ? filePath : path.join(PROJECT_ROOT, filePath);
}

function displayPath(filePath) {
  const absolute = path.resolve(filePath);
  const relative = path.relative(PROJECT_ROOT, absolute);
  return relative && !relative.startsWith('..') ? relative : absolute;
}

function validateManifest(manifest) {
  const schema = JSON.parse(fs.readFileSync(MANIFEST_SCHEMA_PATH, 'utf8'));
  const validate = new Ajv({ allErrors: true, strict: false }).compile(schema);
  if (!validate(manifest)) {
    const details = validate.errors
      .map((error) => `${error.instancePath || '/'} ${error.message}`)
      .join('; ');
    throw new Error(`Invalid coverage manifest: ${details}`);
  }
  return manifest;
}

function verifyFile(fileSpec, label) {
  const absolute = resolveRepoPath(fileSpec.path);
  if (!fs.existsSync(absolute)) throw new Error(`${label} does not exist: ${fileSpec.path}`);
  const actual = sha256File(absolute);
  if (actual !== fileSpec.sha256) {
    throw new Error(`${label} hash mismatch: expected ${fileSpec.sha256}, got ${actual}`);
  }
  return absolute;
}

function normalizeType(value) {
  return String(value || '').trim().replaceAll('_', '-');
}

function normalizeField(value) {
  if (value === 'DOI') return 'doi';
  if (value === 'URL') return 'url';
  return normalizeType(value);
}

function normalizeReferences(value) {
  const entries = Array.isArray(value) ? value : Object.values(value || {});
  return entries
    .filter((entry) => entry && typeof entry === 'object' && entry.id && entry.type)
    .sort((left, right) => codepointCompare(String(left.id), String(right.id)));
}

function populatedFields(reference) {
  return Object.entries(reference || {})
    .filter(([name, value]) => {
      if (['id', 'type', 'class'].includes(name)) return false;
      if (value == null || value === '') return false;
      if (Array.isArray(value)) return value.length > 0;
      if (typeof value === 'object') return Object.keys(value).length > 0;
      return true;
    })
    .map(([name]) => normalizeField(name))
    .sort(codepointCompare);
}

function selectorNames(selector) {
  if (Array.isArray(selector)) return selector.map(normalizeType);
  return String(selector || '')
    .replace(/^\[|\]$/g, '')
    .split(',')
    .map(normalizeType)
    .filter(Boolean);
}

function componentFields(component, componentPath) {
  if (Array.isArray(component)) {
    return component.flatMap((child, index) => componentFields(child, `${componentPath}[${index}]`));
  }
  if (!component || typeof component !== 'object') return [];

  const fields = [];
  const add = (field, suffix = '') => {
    if (typeof field === 'string' && field.trim()) {
      fields.push({ field: normalizeField(field), path: `${componentPath}${suffix}` });
    }
  };

  if (component.contributor != null) add(component.contributor, '.contributor');
  if (component.date != null) add(component.date, '.date');
  if (component.number != null) add(component.number, '.number');
  if (component.variable != null) add(component.variable, '.variable');
  if (component.identifier != null) {
    add(component.identifier || component.variable || 'identifier', '.identifier');
  }
  if (component.title != null) {
    const title = normalizeField(component.title);
    const mapped = {
      primary: 'title',
      short: 'title',
      'parent-monograph': 'container-title',
      'parent-serial': 'container-title',
      original: 'original-title',
    }[title] || title;
    add(mapped, '.title');
  }
  if (Array.isArray(component.group)) {
    fields.push(...componentFields(component.group, `${componentPath}.group`));
  }
  if (component.args && typeof component.args === 'object') {
    for (const key of Object.keys(component.args).sort(codepointCompare)) {
      fields.push(...componentFields(component.args[key], `${componentPath}.args.${key}`));
    }
  }
  return fields;
}

function fieldAliases(field) {
  const aliases = new Set([normalizeField(field)]);
  if (aliases.has('page')) aliases.add('pages');
  if (aliases.has('pages')) aliases.add('page');
  if (aliases.has('container-title-short')) aliases.add('container-title');
  return aliases;
}

function findFieldPath(field, template) {
  const aliases = fieldAliases(field);
  return componentFields(template, 'template').find((entry) => aliases.has(entry.field)) || null;
}

function selectTemplate(section, referenceType, surface) {
  const fallback = Array.isArray(section?.template) ? section.template : [];
  for (const [selector, template] of Object.entries(section?.['type-variants'] || {})) {
    if (selectorNames(selector).includes(referenceType)) {
      if (!Array.isArray(template)) {
        throw new Error(`Resolved ${surface} type variant ${selector} is not a concrete template`);
      }
      return {
        template,
        disposition: 'rendered',
        source: `resolved.${surface}.type-variants.${selector}`,
      };
    }
  }
  return {
    template: fallback,
    disposition: 'fallback',
    source: `resolved.${surface}.template`,
  };
}

function ruleMatches(rule, context) {
  const matches = (ruleValue, actual, normalizer = String) => (
    ruleValue == null || ruleValue === '*' || normalizer(ruleValue) === normalizer(actual)
  );
  return matches(rule.surface, context.surface)
    && matches(rule['reference-type'], context.referenceType, normalizeType)
    && matches(rule.field, context.field, normalizeField)
    && matches(rule['reference-id'], context.referenceId)
    && matches(rule.occurrence, context.occurrence);
}

function matchingRule(rules, context) {
  return (rules || []).find((rule) => ruleMatches(rule, context)) || null;
}

function selectStyleReport(report, styleId) {
  if (Array.isArray(report?.styles)) {
    const selected = report.styles.find((style) => style?.name === styleId);
    if (!selected) throw new Error(`Parity report has no styles[] entry for ${styleId}`);
    return selected;
  }
  if (report?.name === styleId || report?.style === styleId) return report;
  throw new Error(`Parity report does not describe style ${styleId}`);
}

function parityEntries(styleReport, surface, evidenceRunId) {
  const entries = surface === 'citation'
    ? (styleReport.citationEntries || styleReport.citations?.entries || [])
    : (styleReport.oracleDetail || styleReport.bibliography?.entries || []);
  const filtered = entries.filter((entry) => (
    (entry?.evidenceRunId || 'baseline') === evidenceRunId
  ));
  const byId = new Map();
  for (const entry of filtered) {
    if (!entry?.id) continue;
    if (byId.has(String(entry.id))) {
      throw new Error(`Duplicate ${surface} parity identity in ${evidenceRunId}: ${entry.id}`);
    }
    byId.set(String(entry.id), entry);
  }
  return byId;
}

function comparisonFor(entry) {
  if (entry?.exactParityEligible === false || typeof entry?.exactMatch !== 'boolean') {
    return { eligibility: 'not-comparable', exactMatch: null };
  }
  return { eligibility: 'comparable', exactMatch: entry.exactMatch };
}

function observationId(context) {
  return [
    context.styleId,
    context.surface,
    context.fixtureId,
    context.referenceId,
    context.referenceType,
    context.field,
    context.occurrence,
  ].map((part) => encodeURIComponent(String(part))).join('/');
}

function buildObservation(context, section, parityEntry, manifest) {
  const selected = selectTemplate(section, context.referenceType, context.surface);
  const fieldPath = findFieldPath(context.field, selected.template);
  const excluded = matchingRule(manifest.relevance?.excluded, context);
  const omission = excluded ? null : matchingRule(manifest['intentional-omissions'], context);
  let renderDisposition = null;
  if (!excluded) {
    if (omission) renderDisposition = 'suppressed';
    else if (fieldPath) renderDisposition = selected.disposition;
    else renderDisposition = 'uncovered';
  }
  const comparison = comparisonFor(parityEntry);

  return {
    observationId: observationId(context),
    surface: context.surface,
    fixtureId: context.fixtureId,
    occurrence: context.occurrence,
    referenceId: context.referenceId,
    referenceType: context.referenceType,
    field: context.field,
    relevance: excluded ? 'excluded' : 'relevant',
    relevanceRationale: excluded?.rationale || null,
    renderDisposition,
    omissionRationale: omission?.rationale || null,
    comparisonEligibility: comparison.eligibility,
    exactMatch: comparison.exactMatch,
    templatePath: fieldPath ? `${selected.source}.${fieldPath.path}` : null,
    coverageEvidence: 'inferred-structural',
  };
}

function summarize(observations, comparisonOutputs) {
  const renderDisposition = { rendered: 0, fallback: 0, suppressed: 0, uncovered: 0 };
  const comparisonEligibility = { comparable: 0, 'not-comparable': 0 };
  let relevant = 0;
  let excluded = 0;
  for (const observation of observations) {
    if (observation.relevance === 'excluded') excluded += 1;
    else {
      relevant += 1;
      renderDisposition[observation.renderDisposition] += 1;
    }
    comparisonEligibility[observation.comparisonEligibility] += 1;
  }
  const comparisons = [...comparisonOutputs.values()];
  const comparable = comparisons.filter((entry) => entry.eligibility === 'comparable');
  return {
    populatedObservations: observations.length,
    relevantObservations: relevant,
    excludedObservations: excluded,
    renderDisposition,
    comparisonEligibility,
    joinedExactParity: {
      passed: comparable.filter((entry) => entry.exactMatch).length,
      total: comparable.length,
      notComparable: comparisons.length - comparable.length,
    },
  };
}

function materializeFileSpec(fileSpec) {
  return {
    ...fileSpec,
    path: displayPath(resolveRepoPath(fileSpec.path)),
  };
}

function buildCoveragePacket({
  manifest,
  manifestPath,
  resolvedStyle,
  report,
  references,
  citations,
  runtime,
  enforceExpectedObservations = true,
}) {
  validateManifest(manifest);
  const styleId = manifest.style.id;
  const styleReport = selectStyleReport(report, styleId);
  const evidenceRunId = manifest.authority['evidence-run-id'];
  const bibliographyParity = parityEntries(styleReport, 'bibliography', evidenceRunId);
  const citationParity = parityEntries(styleReport, 'citation', evidenceRunId);
  const surfaces = new Set(manifest.surfaces);
  const referenceList = normalizeReferences(references);
  const referenceById = new Map(referenceList.map((reference) => [String(reference.id), reference]));
  const observations = [];
  const comparisonOutputs = new Map();

  if (surfaces.has('bibliography')) {
    for (const reference of referenceList) {
      const referenceId = String(reference.id);
      const comparison = comparisonFor(bibliographyParity.get(referenceId));
      comparisonOutputs.set(`bibliography/${referenceId}`, comparison);
      for (const field of populatedFields(reference)) {
        observations.push(buildObservation({
          styleId,
          surface: 'bibliography',
          fixtureId: manifest.fixtures.references.id,
          occurrence: 'entry',
          referenceId,
          referenceType: normalizeType(reference.type),
          field,
        }, resolvedStyle.bibliography || {}, bibliographyParity.get(referenceId), manifest));
      }
    }
  }

  if (surfaces.has('citation')) {
    if (!manifest.fixtures.citations) {
      throw new Error('Citation coverage requires fixtures.citations');
    }
    for (const scenario of citations || []) {
      const scenarioId = String(scenario.id || '');
      const parityEntry = citationParity.get(scenarioId);
      comparisonOutputs.set(`citation/${scenarioId}`, comparisonFor(parityEntry));
      for (let itemIndex = 0; itemIndex < (scenario.items || []).length; itemIndex += 1) {
        const item = scenario.items[itemIndex];
        const reference = referenceById.get(String(item.id));
        if (!reference) throw new Error(`Citation ${scenarioId} references missing item ${item.id}`);
        const referenceId = String(reference.id);
        const occurrence = `${scenarioId}:${itemIndex + 1}`;
        for (const field of populatedFields(reference)) {
          observations.push(buildObservation({
            styleId,
            surface: 'citation',
            fixtureId: manifest.fixtures.citations.id,
            occurrence,
            referenceId,
            referenceType: normalizeType(reference.type),
            field,
          }, resolvedStyle.citation || {}, parityEntry, manifest));
        }
      }
    }
  }

  observations.sort((left, right) => codepointCompare(left.observationId, right.observationId));
  const identities = new Set();
  for (let index = 0; index < observations.length; index += 1) {
    const observation = observations[index];
    if (identities.has(observation.observationId)) {
      throw new Error(`Duplicate observation identity: ${observation.observationId}`);
    }
    identities.add(observation.observationId);
    observation.row = index + 1;
  }
  if (enforceExpectedObservations && observations.length !== manifest['expected-observations']) {
    throw new Error(
      `Observation count mismatch: expected ${manifest['expected-observations']}, got ${observations.length}. `
      + 'Run with --update-manifest to re-pin hashes and the observation count after an intentional style edit.'
    );
  }
  if (![...comparisonOutputs.values()].some((entry) => typeof entry.exactMatch === 'boolean')) {
    throw new Error(`Parity report join produced no non-null exactMatch values for ${evidenceRunId}`);
  }

  return {
    schema: PACKET_SCHEMA,
    packetId: manifest['packet-id'],
    auditManifest: {
      schema: manifest.schema,
      source: {
        revision: manifest.source.revision,
        worktreeClean: !runtime.worktreeDirty,
        baselineEligible: manifest.source['worktree-clean'] && !runtime.worktreeDirty,
      },
      manifest: {
        path: displayPath(manifestPath),
        sha256: sha256File(manifestPath),
      },
      generator: {
        path: displayPath(GENERATOR_PATH),
        version: GENERATOR_VERSION,
        sha256: sha256File(GENERATOR_PATH),
        arguments: {
          manifest: displayPath(manifestPath),
        },
      },
      surfaces: manifest.surfaces,
      expectedObservations: manifest['expected-observations'],
      relevance: manifest.relevance,
      intentionalOmissions: manifest['intentional-omissions'],
      style: {
        id: styleId,
        path: displayPath(resolveRepoPath(manifest.style.path)),
        sha256: manifest.style.sha256,
        chain: manifest.style.chain.map(materializeFileSpec),
        resolvedSha256: sha256(Buffer.from(stableJson(resolvedStyle))),
      },
      fixtures: {
        references: materializeFileSpec(manifest.fixtures.references),
        citations: manifest.fixtures.citations
          ? materializeFileSpec(manifest.fixtures.citations)
          : null,
      },
      authority: {
        name: manifest.authority.name,
        version: manifest.authority.version,
        evidenceRunId,
        inputs: manifest.authority.inputs.map(materializeFileSpec),
        report: materializeFileSpec(manifest.authority.report),
      },
      resolution: {
        provider: 'citum-cli',
        providerVersion: runtime.resolverVersion,
        evidence: 'resolved-style',
        coverageMode: 'inferred-structural',
      },
    },
    summary: summarize(observations, comparisonOutputs),
    reportExactParity: styleReport.exactParity || null,
    observations,
  };
}

function markdownEscape(value) {
  return String(value ?? '—').replaceAll('|', '\\|').replaceAll('\n', ' ');
}

function markdownPacket(packet) {
  const dispositionRows = Object.entries(packet.summary.renderDisposition)
    .map(([name, count]) => `| ${name} | ${count} |`)
    .join('\n');
  const observationRows = packet.observations.map((observation) => (
    `| ${observation.row} | \`${markdownEscape(observation.observationId)}\` | `
    + `${observation.relevance} | ${observation.renderDisposition || '—'} | `
    + `${observation.comparisonEligibility} | ${observation.exactMatch ?? '—'} |`
  )).join('\n');
  const audit = packet.auditManifest;

  return `# Style Coverage Audit: ${packet.packetId}\n\n`
    + `- **Schema:** \`${packet.schema}\`\n`
    + `- **Style:** \`${audit.style.id}\`\n`
    + `- **Source revision:** \`${audit.source.revision}\`\n`
    + `- **Baseline eligible:** ${audit.source.baselineEligible ? 'yes' : 'no'}\n`
    + `- **Coverage evidence:** inferred structural coverage from a Citum-resolved style\n\n`
    + `Structural coverage identifies a resolved component path; it does not prove that a conditional component consumed the value at runtime.\n\n`
    + `## Coverage summary\n\n`
    + `| Render disposition | Relevant observations |\n|---|---:|\n${dispositionRows}\n\n`
    + `- Populated observations: **${packet.summary.populatedObservations}**\n`
    + `- Relevant observations: **${packet.summary.relevantObservations}**\n`
    + `- Excluded observations: **${packet.summary.excludedObservations}**\n\n`
    + `## Joined exact parity\n\n`
    + `- Passed: **${packet.summary.joinedExactParity.passed}/${packet.summary.joinedExactParity.total}**\n`
    + `- Not comparable: **${packet.summary.joinedExactParity.notComparable}**\n\n`
    + `## Complete observation index\n\n`
    + `| Row | Observation ID | Relevance | Render disposition | Comparison | Exact match |\n`
    + `|---:|---|---|---|---|---|\n${observationRows}\n`;
}

function parseArgs(argv = process.argv.slice(2)) {
  const options = {
    manifest: null,
    jsonOut: null,
    markdownOut: null,
    check: false,
    updateManifest: false,
    citumBin: null,
  };
  const values = new Set(['--manifest', '--json-out', '--markdown-out', '--citum-bin']);
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (values.has(flag)) {
      const value = argv[++index];
      if (!value) throw new Error(`Missing value for ${flag}`);
      options[{ '--manifest': 'manifest', '--json-out': 'jsonOut', '--markdown-out': 'markdownOut', '--citum-bin': 'citumBin' }[flag]] = path.resolve(value);
    } else if (flag === '--check') options.check = true;
    else if (flag === '--update-manifest') options.updateManifest = true;
    else throw new Error(`Unknown argument: ${flag}`);
  }
  if (!options.manifest) throw new Error('--manifest is required');
  if (!options.jsonOut || !options.markdownOut) {
    throw new Error('--json-out and --markdown-out are required');
  }
  if (options.check && options.updateManifest) {
    throw new Error('--check and --update-manifest are mutually exclusive');
  }
  return options;
}

function resolveCitumBinary(explicitPath) {
  if (explicitPath) return explicitPath;
  execFileSync('cargo', ['build', '--quiet', '--bin', 'citum'], { cwd: PROJECT_ROOT, stdio: 'pipe' });
  return path.join(PROJECT_ROOT, 'target', 'debug', 'citum');
}

function resolvedStyleFromCitum(citumBin, stylePath) {
  const output = execFileSync(citumBin, [
    'style', 'validate', stylePath, '--format', 'json', '--include-resolved',
  ], { cwd: PROJECT_ROOT, encoding: 'utf8' });
  const parsed = JSON.parse(output);
  if (!parsed['resolved-style']) throw new Error('Citum did not return resolved-style evidence');
  return parsed['resolved-style'];
}

function gitRuntime(citumBin) {
  const revision = execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: PROJECT_ROOT,
    encoding: 'utf8',
  }).trim();
  const status = execFileSync('git', ['status', '--porcelain'], {
    cwd: PROJECT_ROOT,
    encoding: 'utf8',
  }).trim();
  const resolverVersion = execFileSync(citumBin, ['--version'], {
    cwd: PROJECT_ROOT,
    encoding: 'utf8',
  }).trim();
  return { revision, worktreeDirty: status.length > 0, resolverVersion };
}

function assertOutput(expectedPath, actual, label) {
  if (!fs.existsSync(expectedPath)) throw new Error(`${label} does not exist: ${expectedPath}`);
  const expected = fs.readFileSync(expectedPath, 'utf8');
  if (expected !== actual) throw new Error(`${label} is stale: regenerate ${expectedPath}`);
}

function validateBaselineRuntime(manifest, runtime) {
  if (manifest.source['worktree-clean'] && runtime.worktreeDirty) {
    throw new Error('Baseline manifest requires a clean worktree');
  }
}

/**
 * Re-pin every hash a coverage manifest carries against the files on disk.
 * Used after an intentional edit to the audited style, one of its
 * inheritance-chain ancestors, or a pinned fixture — none of those hashes
 * self-heal, so a tuning pass must refresh them explicitly before the
 * packet can be regenerated. `source.revision` and `worktree-clean` are a
 * maintainer's baseline declaration and are left untouched here.
 */
function refreshManifestHashes(manifest) {
  manifest.style.sha256 = sha256File(resolveRepoPath(manifest.style.path));
  for (const entry of manifest.style.chain) {
    entry.sha256 = sha256File(resolveRepoPath(entry.path));
  }
  manifest.fixtures.references.sha256 = sha256File(resolveRepoPath(manifest.fixtures.references.path));
  if (manifest.fixtures.citations) {
    manifest.fixtures.citations.sha256 = sha256File(resolveRepoPath(manifest.fixtures.citations.path));
  }
  manifest.authority.report.sha256 = sha256File(resolveRepoPath(manifest.authority.report.path));
  for (const input of manifest.authority.inputs) {
    input.sha256 = sha256File(resolveRepoPath(input.path));
  }
  return manifest;
}

function writeManifestYaml(manifestPath, manifest) {
  fs.writeFileSync(manifestPath, yaml.dump(manifest, { lineWidth: -1, noRefs: true, sortKeys: false }));
}

function main() {
  try {
    const options = parseArgs();
    let manifest = validateManifest(readData(options.manifest));

    if (options.updateManifest) {
      refreshManifestHashes(manifest);
    }

    const stylePath = options.updateManifest
      ? resolveRepoPath(manifest.style.path)
      : verifyFile(manifest.style, 'style');
    if (!options.updateManifest) {
      for (const [index, chainEntry] of manifest.style.chain.entries()) {
        verifyFile(chainEntry, `style.chain[${index}]`);
      }
    }
    const referencesPath = options.updateManifest
      ? resolveRepoPath(manifest.fixtures.references.path)
      : verifyFile(manifest.fixtures.references, 'fixtures.references');
    const citationsPath = manifest.fixtures.citations
      ? (options.updateManifest
        ? resolveRepoPath(manifest.fixtures.citations.path)
        : verifyFile(manifest.fixtures.citations, 'fixtures.citations'))
      : null;
    const reportPath = options.updateManifest
      ? resolveRepoPath(manifest.authority.report.path)
      : verifyFile(manifest.authority.report, 'authority.report');
    if (!options.updateManifest) {
      for (const [index, input] of manifest.authority.inputs.entries()) {
        verifyFile(input, `authority.inputs[${index}]`);
      }
    }
    const citumBin = resolveCitumBinary(options.citumBin);
    const runtime = gitRuntime(citumBin);
    if (!options.updateManifest) validateBaselineRuntime(manifest, runtime);
    const resolvedStyle = resolvedStyleFromCitum(citumBin, stylePath);
    const report = readData(reportPath);
    const references = readData(referencesPath);
    const citations = citationsPath ? readData(citationsPath) : [];

    if (options.updateManifest) {
      const draft = buildCoveragePacket({
        manifest,
        manifestPath: options.manifest,
        resolvedStyle,
        report,
        references,
        citations,
        runtime,
        enforceExpectedObservations: false,
      });
      manifest['expected-observations'] = draft.summary.populatedObservations;
      writeManifestYaml(options.manifest, manifest);
      manifest = validateManifest(readData(options.manifest));
    }

    const packet = buildCoveragePacket({
      manifest,
      manifestPath: options.manifest,
      resolvedStyle,
      report,
      references,
      citations,
      runtime,
    });
    const json = stableJson(packet);
    const markdown = markdownPacket(packet);
    if (options.check) {
      assertOutput(options.jsonOut, json, 'JSON packet');
      assertOutput(options.markdownOut, markdown, 'Markdown packet');
      return;
    }
    fs.mkdirSync(path.dirname(options.jsonOut), { recursive: true });
    fs.mkdirSync(path.dirname(options.markdownOut), { recursive: true });
    fs.writeFileSync(options.jsonOut, json);
    fs.writeFileSync(options.markdownOut, markdown);
    if (options.updateManifest) {
      process.stdout.write(
        `Refreshed ${displayPath(options.manifest)}: `
        + `${manifest['expected-observations']} expected observations, `
        + `baseline eligible: ${packet.auditManifest.source.baselineEligible ? 'yes' : 'no'}.\n`
      );
    }
  } catch (error) {
    process.stderr.write(`Error: ${error.message}\n`);
    process.exitCode = 1;
  }
}

if (require.main === module) main();

module.exports = {
  buildCoveragePacket,
  codepointCompare,
  componentFields,
  markdownPacket,
  parseArgs,
  populatedFields,
  refreshManifestHashes,
  selectStyleReport,
  stableJson,
  validateBaselineRuntime,
  validateManifest,
  writeManifestYaml,
};
