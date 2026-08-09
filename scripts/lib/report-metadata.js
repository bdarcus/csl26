'use strict';

const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');

const PROJECT_ROOT = path.resolve(__dirname, '..', '..');
const DEFAULT_PROVENANCE_PATH = path.join(PROJECT_ROOT, 'scripts', 'report-data', 'report-provenance.yaml');

const ALLOWED_LINEAGE_KEYS = new Set([
  'csl-derived',
  'biblatex-derived',
  'citum-native',
]);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function readYaml(filePath) {
  return yaml.load(fs.readFileSync(filePath, 'utf8')) || {};
}

function validateKeyMap(value, label) {
  assert(value && typeof value === 'object' && !Array.isArray(value), `${label} must be an object`);
  for (const key of ALLOWED_LINEAGE_KEYS) {
    assert(Object.prototype.hasOwnProperty.call(value, key), `${label} must define ${key}`);
  }
}

function validateReportProvenance(config) {
  assert(config && typeof config === 'object' && !Array.isArray(config), 'report-provenance.yaml must be an object');
  assert(config.version === 1, 'report-provenance.yaml version must be 1');
  assert(config.defaults && typeof config.defaults === 'object', 'report-provenance.yaml must define defaults');
  validateKeyMap(config.defaults.labels, 'report-provenance.yaml defaults.labels');
  validateKeyMap(config.defaults.sort_ranks, 'report-provenance.yaml defaults.sort_ranks');
  assert(config.styles && typeof config.styles === 'object' && !Array.isArray(config.styles), 'report-provenance.yaml must define styles');

  const coverageAudits = config.coverage_audits || [];
  assert(Array.isArray(coverageAudits), 'report-provenance.yaml coverage_audits must be an array');
  const registeredAuditStyles = new Set();
  for (const [index, audit] of coverageAudits.entries()) {
    const label = `report-provenance.yaml coverage_audits[${index}]`;
    assert(audit && typeof audit === 'object' && !Array.isArray(audit), `${label} must be an object`);
    for (const key of ['style_id', 'manifest', 'packet', 'markdown', 'adjudication_record', 'adjudication_href']) {
      assert(typeof audit[key] === 'string' && audit[key].trim() !== '', `${label}.${key} must be a non-empty string`);
    }
    assert(!registeredAuditStyles.has(audit.style_id), `${label}.style_id duplicates ${audit.style_id}`);
    assert(!audit.adjudication_href.endsWith('.json'), `${label}.adjudication_href must point to a human-readable record`);
    registeredAuditStyles.add(audit.style_id);
  }

  for (const [styleName, styleConfig] of Object.entries(config.styles)) {
    assert(styleConfig && typeof styleConfig === 'object' && !Array.isArray(styleConfig), `report-provenance.yaml styles.${styleName} must be an object`);
    if (styleConfig.lineage != null) {
      assert(ALLOWED_LINEAGE_KEYS.has(styleConfig.lineage), `report-provenance.yaml styles.${styleName}.lineage must be one of: ${[...ALLOWED_LINEAGE_KEYS].join(', ')}`);
    }
  }

  return config;
}

function loadReportProvenance(filePath = DEFAULT_PROVENANCE_PATH) {
  return validateReportProvenance(readYaml(filePath));
}

function getLineagePresentation(lineageKey, config) {
  const fallbackKey = ALLOWED_LINEAGE_KEYS.has(lineageKey) ? lineageKey : 'citum-native';
  return {
    key: fallbackKey,
    label: config.defaults.labels[fallbackKey],
    sortRank: config.defaults.sort_ranks[fallbackKey],
  };
}

module.exports = {
  ALLOWED_LINEAGE_KEYS,
  DEFAULT_PROVENANCE_PATH,
  getLineagePresentation,
  loadReportProvenance,
  validateReportProvenance,
};
