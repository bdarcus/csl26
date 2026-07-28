'use strict';

const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');

const DEFAULT_REGISTRY_PATH = path.join(
  __dirname,
  '..',
  '..',
  'crates',
  'citum-schema-style',
  'embedded',
  'registry',
  'default.yaml'
);
const DEFAULT_REPORT_DATA_DIR = path.join(__dirname, '..', 'report-data');

function readYaml(filePath) {
  try {
    return yaml.load(fs.readFileSync(filePath, 'utf8')) || {};
  } catch {
    return {};
  }
}

function readStyleInventory(styleRoots) {
  const inventory = new Map();
  for (const root of styleRoots) {
    if (!fs.existsSync(root)) continue;
    const filenames = fs.readdirSync(root)
      .filter((filename) => filename.endsWith('.yaml'))
      .sort((left, right) => left.localeCompare(right));
    for (const filename of filenames) {
      const name = path.basename(filename, '.yaml');
      if (!inventory.has(name)) {
        inventory.set(name, {
          name,
          path: path.join(root, filename),
          data: readYaml(path.join(root, filename)),
        });
      }
    }
  }
  return inventory;
}

function normalizeParent(value) {
  if (typeof value !== 'string') return null;
  const parent = value.trim().replace(/\.yaml$/, '');
  return parent || null;
}

function hasRenderingStructure(styleData) {
  const structuralKeys = new Set(['template', 'type-templates', 'type-variants']);
  const visit = (value) => {
    if (!value || typeof value !== 'object') return false;
    if (Array.isArray(value)) return false;
    return Object.entries(value).some(([key, child]) =>
      structuralKeys.has(key) || visit(child)
    );
  };
  return visit(styleData?.citation) || visit(styleData?.bibliography);
}

function classifyImplementation(styleData) {
  if (!normalizeParent(styleData?.extends)) return 'standalone';
  return hasRenderingStructure(styleData) ? 'structural-wrapper' : 'config-wrapper';
}

function resolveInheritance(name, inventory) {
  const chain = [];
  const seen = new Map();
  let current = name;
  let missingParent = null;
  let cycle = null;

  while (current) {
    if (seen.has(current)) {
      cycle = chain.slice(seen.get(current)).concat(current);
      chain.push(current);
      break;
    }
    seen.set(current, chain.length);
    chain.push(current);

    const style = inventory.get(current);
    if (!style) {
      missingParent = current;
      break;
    }
    current = normalizeParent(style.data?.extends);
  }

  const familyRoot = cycle
    ? [...new Set(cycle)].sort((left, right) => left.localeCompare(right))[0]
    : chain[chain.length - 1] || name;

  return {
    directParent: normalizeParent(inventory.get(name)?.data?.extends),
    chain,
    familyRoot,
    complete: !missingParent && !cycle,
    missingParent,
    cycle,
  };
}

function loadRegistry(registryPath = DEFAULT_REGISTRY_PATH) {
  const registry = readYaml(registryPath);
  return new Map((registry.styles || []).map((entry) => [
    entry.id,
    {
      kind: entry.kind || null,
      aliases: [...new Set(entry.aliases || [])].sort((left, right) => left.localeCompare(right)),
    },
  ]));
}

function findLatestArtifact(reportDataDir, prefix) {
  if (!fs.existsSync(reportDataDir)) return null;
  const candidates = fs.readdirSync(reportDataDir)
    .filter((filename) => filename.startsWith(prefix) && filename.endsWith('.tsv'))
    .sort((left, right) => right.localeCompare(left));
  return candidates.length > 0 ? path.join(reportDataDir, candidates[0]) : null;
}

function parseTsv(filePath) {
  if (!filePath) return [];
  const lines = fs.readFileSync(filePath, 'utf8').split(/\r?\n/).filter(Boolean);
  if (lines.length === 0) return [];
  const headers = lines[0].split('\t');
  return lines.slice(1).map((line) => {
    const values = line.split('\t');
    return Object.fromEntries(headers.map((header, index) => [header, values[index] ?? '']));
  });
}

function loadMeasurementEvidence(reportDataDir = DEFAULT_REPORT_DATA_DIR) {
  const bandPath = findLatestArtifact(reportDataDir, 'alias-candidates-band-registered-');
  const derivabilityPath = findLatestArtifact(reportDataDir, 'delta-derivability-styles-');
  const bands = new Map(parseTsv(bandPath)
    .filter((row) => row.candidate_id)
    .map((row) => [row.candidate_id, {
      target: row.best_target || null,
      similarity: row.similarity ? Number(row.similarity) : null,
      citationMatch: row.citation_match ? Number(row.citation_match) : null,
      bibliographyMatch: row.bib_match ? Number(row.bib_match) : null,
      evidenceUrl: row.evidence_url || null,
      confidenceNote: row.confidence_note || null,
      band: row.band || null,
      source: bandPath ? path.basename(bandPath) : null,
    }]));
  const derivability = new Map(parseTsv(derivabilityPath)
    .filter((row) => row.candidate_id)
    .map((row) => [row.candidate_id, {
      target: row.target_id || null,
      similarity: row.similarity ? Number(row.similarity) : null,
      standaloneFidelity: row.standalone_fidelity ? Number(row.standalone_fidelity) : null,
      wrapperFidelity: row.wrapper_fidelity ? Number(row.wrapper_fidelity) : null,
      wrapperBytes: row.wrapper_bytes ? Number(row.wrapper_bytes) : null,
      wrapperKeyCount: row.wrapper_key_count ? Number(row.wrapper_key_count) : null,
      verdict: row.verdict || null,
      detail: row.detail || null,
      source: derivabilityPath ? path.basename(derivabilityPath) : null,
    }]));
  return {
    bands,
    derivability,
    sources: {
      behavioralBands: bandPath ? path.basename(bandPath) : null,
      derivability: derivabilityPath ? path.basename(derivabilityPath) : null,
    },
  };
}

function buildInheritanceIndex({
  styleRoots,
  registryPath = DEFAULT_REGISTRY_PATH,
  reportDataDir = DEFAULT_REPORT_DATA_DIR,
}) {
  const inventory = readStyleInventory(styleRoots);
  const registry = loadRegistry(registryPath);
  const evidence = loadMeasurementEvidence(reportDataDir);
  const records = new Map();

  for (const [name, style] of inventory) {
    const registryEntry = registry.get(name) || { kind: null, aliases: [] };
    records.set(name, {
      inheritance: {
        ...resolveInheritance(name, inventory),
        implementationForm: classifyImplementation(style.data),
      },
      registry: {
        kind: registryEntry.kind,
        aliases: registryEntry.aliases,
        aliasCount: registryEntry.aliases.length,
      },
      measurementEvidence: {
        behavioralBand: evidence.bands.get(name) || null,
        derivability: evidence.derivability.get(name) || null,
      },
    });
  }

  return { inventory, records, evidenceSources: evidence.sources };
}

function summarizeCounts(summaries) {
  return summaries.reduce((total, summary) => ({
    passed: total.passed + (summary?.passed || 0),
    total: total.total + (summary?.total || 0),
    notComparable: total.notComparable + (summary?.notComparable || 0),
    unresolvedPairing: total.unresolvedPairing + (summary?.unresolvedPairing || 0),
  }), {
    passed: 0,
    total: 0,
    notComparable: 0,
    unresolvedPairing: 0,
  });
}

function summarizePairing(summaries) {
  return summaries.reduce((total, summary) => ({
    paired: total.paired + (summary?.paired || 0),
    unresolvedUnpaired: total.unresolvedUnpaired + (summary?.unresolvedUnpaired || 0),
    idProvenOracleOnly: total.idProvenOracleOnly + (summary?.idProvenOracleOnly || 0),
    idProvenCitumOnly: total.idProvenCitumOnly + (summary?.idProvenCitumOnly || 0),
    totalObservations: total.totalObservations + (summary?.totalObservations || 0),
  }), {
    paired: 0,
    unresolvedUnpaired: 0,
    idProvenOracleOnly: 0,
    idProvenCitumOnly: 0,
    totalObservations: 0,
  });
}

function buildFamilyAggregates(styles) {
  const grouped = new Map();
  for (const style of styles) {
    const root = style.inheritance?.familyRoot || style.name;
    const family = grouped.get(root) || {
      root,
      aggregateCslReach: 0,
      members: [],
      aliases: new Set(),
      compatibility: [],
      exactParity: [],
      pairing: [],
    };
    family.aggregateCslReach += typeof style.cslReach === 'number' ? style.cslReach : 0;
    family.members.push(style.name);
    for (const alias of style.registry?.aliases || []) family.aliases.add(alias);
    family.compatibility.push({
      passed: (style.citations?.passed || 0) + (style.bibliography?.passed || 0),
      total: (style.citations?.total || 0) + (style.bibliography?.total || 0),
      unresolvedPairing: style.pairingSummary?.unresolvedUnpaired || 0,
    });
    family.exactParity.push(style.exactParity);
    family.pairing.push(style.pairingSummary);
    grouped.set(root, family);
  }

  return [...grouped.values()]
    .map((family) => ({
      root: family.root,
      aggregateCslReach: family.aggregateCslReach,
      members: family.members.sort((left, right) => left.localeCompare(right)),
      memberCount: family.members.length,
      aliases: [...family.aliases].sort((left, right) => left.localeCompare(right)),
      aliasCount: family.aliases.size,
      compatibility: summarizeCounts(family.compatibility),
      exactParity: summarizeCounts(family.exactParity),
      pairing: summarizePairing(family.pairing),
    }))
    .sort((left, right) =>
      right.aggregateCslReach - left.aggregateCslReach
      || left.root.localeCompare(right.root)
    );
}

module.exports = {
  buildFamilyAggregates,
  buildInheritanceIndex,
  classifyImplementation,
  findLatestArtifact,
  loadMeasurementEvidence,
  readStyleInventory,
  resolveInheritance,
};
