#!/usr/bin/env node
/** Regenerate every registered style coverage audit packet for release metadata. */

'use strict';

const path = require('node:path');
const { execFileSync } = require('node:child_process');

const { loadReportProvenance } = require('./lib/report-metadata');
const { PROJECT_ROOT, resolveRepoPath } = require('./lib/style-coverage-audits');

const GENERATOR_PATH = path.join(PROJECT_ROOT, 'scripts', 'style-coverage-review.js');

function parseArgs(argv = process.argv.slice(2)) {
  const options = { citumBin: null };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag !== '--citum-bin') throw new Error(`Unknown argument: ${flag}`);
    const value = argv[++index];
    if (!value) throw new Error('Missing value for --citum-bin');
    options.citumBin = path.resolve(value);
  }
  return options;
}

function refreshRegisteredAudits(options = {}, dependencies = {}) {
  const provenance = dependencies.provenance || loadReportProvenance();
  const run = dependencies.execFileSync || execFileSync;
  const citumBin = options.citumBin || path.join(PROJECT_ROOT, 'target', 'debug', 'citum');

  for (const registration of provenance.coverage_audits || []) {
    run(process.execPath, [
      GENERATOR_PATH,
      '--manifest', resolveRepoPath(registration.manifest),
      '--json-out', resolveRepoPath(registration.packet),
      '--markdown-out', resolveRepoPath(registration.markdown),
      '--citum-bin', citumBin,
    ], {
      cwd: PROJECT_ROOT,
      stdio: 'inherit',
    });
  }

  return { status: 'current', registrations: provenance.coverage_audits || [] };
}

function main() {
  try {
    refreshRegisteredAudits(parseArgs());
  } catch (error) {
    process.stderr.write(`Error: ${error.message}\n`);
    process.exitCode = 1;
  }
}

if (require.main === module) main();

module.exports = { parseArgs, refreshRegisteredAudits };
