'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const Ajv = require('ajv');
const yaml = require('js-yaml');

const PROJECT_ROOT = path.resolve(__dirname, '..', '..');
const PACKET_SCHEMA_PATH = path.join(
  PROJECT_ROOT,
  'scripts',
  'report-data',
  'style-coverage-packet.schema.json'
);
const GENERATOR_PATH = path.join(PROJECT_ROOT, 'scripts', 'style-coverage-review.js');
const PACKET_SCHEMA = 'citum.style-coverage-packet/v1';
const REPORT_AUDIT_SCHEMA = 'citum.report-coverage-audit/v1';

let packetValidator = null;

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function resolveRepoPath(filePath) {
  return path.isAbsolute(filePath) ? filePath : path.join(PROJECT_ROOT, filePath);
}

function readData(filePath) {
  const source = fs.readFileSync(filePath, 'utf8');
  return /\.ya?ml$/i.test(filePath) ? (yaml.load(source) || {}) : JSON.parse(source);
}

function sha256File(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

function getPacketValidator() {
  if (!packetValidator) {
    const schema = JSON.parse(fs.readFileSync(PACKET_SCHEMA_PATH, 'utf8'));
    packetValidator = new Ajv({ allErrors: true, strict: false }).compile(schema);
  }
  return packetValidator;
}

function validatePacketSchema(packet) {
  const validate = getPacketValidator();
  if (!validate(packet)) {
    const details = validate.errors
      .map((error) => `${error.instancePath || '/'} ${error.message}`)
      .join('; ');
    throw new Error(`Invalid coverage packet schema: ${details}`);
  }
  return packet;
}

function observationIdentity(styleId, observation) {
  return [
    styleId,
    observation.surface,
    observation.fixtureId,
    observation.referenceId,
    observation.referenceType,
    observation.field,
    observation.occurrence,
  ].map((part) => encodeURIComponent(String(part))).join('/');
}

function outputIdForObservation(observation) {
  if (observation.surface === 'bibliography') return observation.referenceId;
  const separator = observation.occurrence.lastIndexOf(':');
  assert(separator > 0, `Citation observation has invalid occurrence: ${observation.observationId}`);
  return observation.occurrence.slice(0, separator);
}

function comparisonStateForObservation(observation) {
  if (observation.comparisonEligibility === 'not-comparable') return 'not-comparable';
  return observation.exactMatch ? 'exact-match' : 'mismatch';
}

function compareStringArrays(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function validateCoveragePacket(packet, registration, manifest) {
  validatePacketSchema(packet);
  const label = registration.style_id;
  const audit = packet.auditManifest;
  const expectedManifestPath = registration.manifest.replaceAll(path.sep, '/');
  const manifestSha = sha256File(resolveRepoPath(registration.manifest));
  const generatorSha = sha256File(GENERATOR_PATH);

  assert(packet.schema === PACKET_SCHEMA, `${label}: packet schema is not ${PACKET_SCHEMA}`);
  assert(manifest.style.id === label, `${label}: manifest style ID is ${manifest.style.id}`);
  assert(audit.style.id === label, `${label}: packet style ID is ${audit.style.id}`);
  assert(packet.packetId === manifest['packet-id'], `${label}: packet ID does not match manifest`);
  assert(audit.manifest.path === expectedManifestPath, `${label}: packet manifest path does not match registration`);
  assert(audit.manifest.sha256 === manifestSha, `${label}: packet manifest hash is stale`);
  assert(audit.generator.path === 'scripts/style-coverage-review.js', `${label}: unexpected packet generator path`);
  assert(audit.generator.sha256 === generatorSha, `${label}: packet generator hash is stale`);
  assert(audit.generator.arguments.manifest === expectedManifestPath, `${label}: packet generator arguments do not match registration`);
  assert(audit.source.revision === manifest.source.revision, `${label}: packet source revision does not match manifest`);
  assert(audit.source.worktreeClean === manifest.source['worktree-clean'], `${label}: packet worktree-clean state does not match manifest`);
  assert(audit.source.baselineEligible, `${label}: packet is not baseline eligible`);
  assert(audit.source.worktreeClean, `${label}: packet was generated from a dirty worktree`);
  assert(audit.expectedObservations === manifest['expected-observations'], `${label}: packet expected-observation count does not match manifest`);
  assert(compareStringArrays(audit.surfaces, manifest.surfaces), `${label}: packet surfaces do not match manifest`);
  assert(packet.observations.length === manifest['expected-observations'], `${label}: missing or extra observations`);

  const identities = new Set();
  const dispositionCounts = { rendered: 0, fallback: 0, suppressed: 0, uncovered: 0 };
  const comparisonCounts = { comparable: 0, 'not-comparable': 0 };
  const outputs = new Map();
  let relevant = 0;
  let excluded = 0;

  for (const [index, observation] of packet.observations.entries()) {
    const expectedIdentity = observationIdentity(label, observation);
    assert(observation.observationId === expectedIdentity, `${label}: invalid observation identity at row ${index + 1}`);
    assert(!identities.has(expectedIdentity), `${label}: duplicate observation identity ${expectedIdentity}`);
    identities.add(expectedIdentity);
    assert(observation.row === index + 1, `${label}: observation rows are not contiguous at ${expectedIdentity}`);

    if (observation.relevance === 'excluded') {
      excluded += 1;
      assert(observation.renderDisposition == null, `${label}: excluded observation has a render disposition`);
      assert(Boolean(observation.relevanceRationale), `${label}: excluded observation has no rationale`);
    } else {
      relevant += 1;
      assert(Object.hasOwn(dispositionCounts, observation.renderDisposition), `${label}: relevant observation has no valid disposition`);
      dispositionCounts[observation.renderDisposition] += 1;
      if (observation.renderDisposition === 'suppressed') {
        assert(Boolean(observation.omissionRationale), `${label}: suppressed observation has no rationale`);
      }
    }

    comparisonCounts[observation.comparisonEligibility] += 1;
    if (observation.comparisonEligibility === 'comparable') {
      assert(typeof observation.exactMatch === 'boolean', `${label}: comparable observation has no exact result`);
    } else {
      assert(observation.exactMatch == null, `${label}: not-comparable observation has an exact result`);
    }

    const outputKey = `${observation.surface}/${outputIdForObservation(observation)}`;
    const state = comparisonStateForObservation(observation);
    const priorState = outputs.get(outputKey);
    assert(priorState == null || priorState === state, `${label}: inconsistent comparison state for ${outputKey}`);
    outputs.set(outputKey, state);
  }

  const dispositionTotal = Object.values(dispositionCounts).reduce((sum, count) => sum + count, 0);
  assert(relevant + excluded === packet.observations.length, `${label}: relevance counts do not partition observations`);
  assert(dispositionTotal === relevant, `${label}: disposition counts do not partition relevant observations`);
  assert(packet.summary.populatedObservations === packet.observations.length, `${label}: populated count drift`);
  assert(packet.summary.relevantObservations === relevant, `${label}: relevant count drift`);
  assert(packet.summary.excludedObservations === excluded, `${label}: excluded count drift`);
  for (const [name, count] of Object.entries(dispositionCounts)) {
    assert(packet.summary.renderDisposition[name] === count, `${label}: render-disposition count drift`);
  }
  for (const [name, count] of Object.entries(comparisonCounts)) {
    assert(packet.summary.comparisonEligibility[name] === count, `${label}: comparison-eligibility count drift`);
  }

  const outputStates = [...outputs.values()];
  const comparableOutputs = outputStates.filter((state) => state !== 'not-comparable');
  const passedOutputs = outputStates.filter((state) => state === 'exact-match');
  const notComparableOutputs = outputStates.filter((state) => state === 'not-comparable');
  const joined = packet.summary.joinedExactParity;
  assert(joined.total === comparableOutputs.length, `${label}: joined exact-parity total drift`);
  assert(joined.passed === passedOutputs.length, `${label}: joined exact-parity passed drift`);
  assert(joined.notComparable === notComparableOutputs.length, `${label}: joined exact-parity not-comparable drift`);
  assert(joined.passed <= joined.total, `${label}: exact-parity passed count exceeds total`);

  return packet;
}

function verifyFileSpec(fileSpec, label) {
  const absolute = resolveRepoPath(fileSpec.path);
  assert(fs.existsSync(absolute), `${label} does not exist: ${fileSpec.path}`);
  const actual = sha256File(absolute);
  assert(actual === fileSpec.sha256, `${label} hash mismatch: expected ${fileSpec.sha256}, got ${actual}`);
}

function verifyManifestFiles(manifest) {
  verifyFileSpec(manifest.style, 'style');
  for (const [index, entry] of manifest.style.chain.entries()) {
    verifyFileSpec(entry, `style.chain[${index}]`);
  }
  verifyFileSpec(manifest.fixtures.references, 'fixtures.references');
  if (manifest.fixtures.citations) verifyFileSpec(manifest.fixtures.citations, 'fixtures.citations');
  verifyFileSpec(manifest.authority.report, 'authority.report');
  for (const [index, entry] of manifest.authority.inputs.entries()) {
    verifyFileSpec(entry, `authority.inputs[${index}]`);
  }
}

function validateRegistrationFiles(registration) {
  for (const key of ['manifest', 'packet', 'markdown', 'adjudication_record']) {
    const absolute = resolveRepoPath(registration[key]);
    assert(fs.existsSync(absolute), `${registration.style_id}: registered ${key} does not exist: ${registration[key]}`);
  }
  assert(!registration.adjudication_href.endsWith('.json'), `${registration.style_id}: supporting link must not target raw JSON`);
}

function selectAuthorityStyle(report, styleId) {
  const styles = Array.isArray(report.styles) ? report.styles : [report];
  const selected = styles.find((style) => style?.name === styleId || style?.style === styleId);
  assert(selected, `Authority report has no style entry for ${styleId}`);
  return selected;
}

function entryMap(entries, evidenceRunId, surface) {
  const result = new Map();
  for (const entry of entries || []) {
    if ((entry.evidenceRunId || 'baseline') !== evidenceRunId || entry.id == null) continue;
    const id = String(entry.id);
    assert(!result.has(id), `Duplicate ${surface} authority output identity: ${id}`);
    result.set(id, entry);
  }
  return result;
}

function buildCoverageAuditView(packet, registration, authorityReport, currentReport = null) {
  const styleReport = selectAuthorityStyle(authorityReport, registration.style_id);
  const currentStyleReport = currentReport
    ? selectAuthorityStyle(currentReport, registration.style_id)
    : null;
  const evidenceRunId = packet.auditManifest.authority.evidenceRunId;
  const evidence = {
    citation: entryMap(styleReport.citationEntries, evidenceRunId, 'citation'),
    bibliography: entryMap(styleReport.oracleDetail, evidenceRunId, 'bibliography'),
  };
  const currentEvidence = currentStyleReport
    ? {
      citation: entryMap(currentStyleReport.citationEntries, evidenceRunId, 'citation'),
      bibliography: entryMap(currentStyleReport.oracleDetail, evidenceRunId, 'bibliography'),
    }
    : null;
  const groups = new Map();

  for (const observation of packet.observations) {
    const outputId = outputIdForObservation(observation);
    const key = `${observation.surface}/${outputId}`;
    let group = groups.get(key);
    if (!group) {
      const comparisonState = comparisonStateForObservation(observation);
      const authorityEntry = evidence[observation.surface].get(outputId) || null;
      assert(
        comparisonState !== 'mismatch' || authorityEntry,
        `Missing exact authority evidence for ${observation.surface}/${outputId}`
      );
      group = {
        outputId,
        surface: observation.surface,
        comparisonState,
        comparisonEligibility: observation.comparisonEligibility,
        exactMatch: observation.exactMatch,
        needsReview: comparisonState === 'mismatch',
        referenceIds: [],
        referenceTypes: [],
        dispositions: [],
        dispositionCounts: { rendered: 0, fallback: 0, suppressed: 0, uncovered: 0, excluded: 0 },
        fields: [],
        exactEvidence: comparisonState === 'mismatch' && authorityEntry
          ? {
            oracle: authorityEntry.exactOracle ?? authorityEntry.oracle ?? authorityEntry.expected ?? '',
            citum: authorityEntry.exactCitum ?? authorityEntry.citum ?? authorityEntry.actual ?? '',
          }
          : null,
      };
      const currentEntry = currentEvidence?.[observation.surface].get(outputId) || null;
      if (currentEntry && authorityEntry) {
        const beforeText = authorityEntry.exactCitum ?? authorityEntry.citum ?? authorityEntry.actual ?? '';
        const afterText = currentEntry.exactCitum ?? currentEntry.citum ?? currentEntry.actual ?? '';
        group.postChangeEvidence = {
          beforeExact: authorityEntry.exactMatch === true,
          afterExact: currentEntry.exactMatch === true,
          changed: beforeText !== afterText,
          beforeCitum: beforeText,
          afterCitum: afterText,
        };
      }
      groups.set(key, group);
    }

    const disposition = observation.relevance === 'excluded'
      ? 'excluded'
      : observation.renderDisposition;
    group.dispositionCounts[disposition] += 1;
    group.needsReview ||= disposition === 'uncovered';
    if (!group.referenceIds.includes(observation.referenceId)) group.referenceIds.push(observation.referenceId);
    if (!group.referenceTypes.includes(observation.referenceType)) group.referenceTypes.push(observation.referenceType);
    if (!group.dispositions.includes(disposition)) group.dispositions.push(disposition);
    group.fields.push({
      observationId: observation.observationId,
      field: observation.field,
      referenceId: observation.referenceId,
      referenceType: observation.referenceType,
      disposition,
      rationale: observation.relevanceRationale || observation.omissionRationale || null,
      templatePath: observation.templatePath,
    });
  }

  const outputGroups = [...groups.values()].sort((left, right) => (
    left.surface.localeCompare(right.surface) || left.outputId.localeCompare(right.outputId)
  ));
  const changedOutputs = outputGroups
    .filter((group) => group.postChangeEvidence?.changed)
    .map((group) => ({
      surface: group.surface,
      outputId: group.outputId,
      beforeExact: group.postChangeEvidence.beforeExact,
      afterExact: group.postChangeEvidence.afterExact,
    }));
  return {
    schema: REPORT_AUDIT_SCHEMA,
    status: 'current',
    packetId: packet.packetId,
    evidence: packet.auditManifest.resolution.coverageMode,
    sourceRevision: packet.auditManifest.source.revision,
    baselineEligible: packet.auditManifest.source.baselineEligible,
    adjudicationRecord: {
      label: 'Read the maintainer adjudication',
      href: registration.adjudication_href,
    },
    summary: packet.summary,
    postChangeEvidence: currentStyleReport
      ? {
        schema: 'citum.report-coverage-audit-post-change/v1',
        status: 'measured',
        beforeExactParity: styleReport.exactParity,
        afterExactParity: currentStyleReport.exactParity,
        changedOutputs,
        unavailableOutputs: outputGroups.filter((group) => !group.postChangeEvidence).length,
      }
      : null,
    filters: {
      surfaces: ['bibliography', 'citation'],
      dispositions: ['rendered', 'fallback', 'suppressed', 'uncovered', 'excluded'],
      comparisonStates: ['exact-match', 'mismatch', 'not-comparable'],
    },
    outputGroups,
  };
}

function loadCoverageAuditViews(provenanceConfig, selectedStyles = null, currentReports = null) {
  const selected = selectedStyles ? new Set(selectedStyles) : null;
  const views = new Map();
  for (const registration of provenanceConfig.coverage_audits || []) {
    if (selected && !selected.has(registration.style_id)) continue;
    validateRegistrationFiles(registration);
    const manifest = readData(resolveRepoPath(registration.manifest));
    const packet = readData(resolveRepoPath(registration.packet));
    validateCoveragePacket(packet, registration, manifest);
    const authorityReport = readData(resolveRepoPath(manifest.authority.report.path));
    const currentReport = currentReports?.get(registration.style_id) || null;
    views.set(registration.style_id, buildCoverageAuditView(packet, registration, authorityReport, currentReport));
  }
  return views;
}

function coverageAuditMetadata(provenanceConfig) {
  return {
    registry: 'scripts/report-data/report-provenance.yaml',
    registeredStyles: (provenanceConfig.coverage_audits || []).map((entry) => ({
      styleId: entry.style_id,
      adjudicationHref: entry.adjudication_href,
    })),
  };
}

module.exports = {
  PACKET_SCHEMA,
  PACKET_SCHEMA_PATH,
  PROJECT_ROOT,
  REPORT_AUDIT_SCHEMA,
  buildCoverageAuditView,
  comparisonStateForObservation,
  coverageAuditMetadata,
  loadCoverageAuditViews,
  observationIdentity,
  outputIdForObservation,
  readData,
  resolveRepoPath,
  sha256File,
  validateCoveragePacket,
  validatePacketSchema,
  validateRegistrationFiles,
  verifyManifestFiles,
};
