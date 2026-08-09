#!/usr/bin/env node
/** Validate every explicitly registered style coverage audit and regenerate it byte for byte. */

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const { loadReportProvenance } = require('./lib/report-metadata');
const {
  PROJECT_ROOT,
  readData,
  resolveRepoPath,
  validateCoveragePacket,
  validateRegistrationFiles,
  verifyManifestFiles,
} = require('./lib/style-coverage-audits');
const { validateManifest } = require('./style-coverage-review');

const GENERATOR_PATH = path.join(PROJECT_ROOT, 'scripts', 'style-coverage-review.js');

function parseArgs(argv = process.argv.slice(2)) {
  const options = { statusStyle: null, citumBin: null };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === '--status' || flag === '--citum-bin') {
      const value = argv[++index];
      if (!value) throw new Error(`Missing value for ${flag}`);
      if (flag === '--status') options.statusStyle = value;
      else options.citumBin = path.resolve(value);
    } else {
      throw new Error(`Unknown argument: ${flag}`);
    }
  }
  return options;
}

function sourceCitumBinary(explicitPath = null, dependencies = {}) {
  if (explicitPath) {
    if (!fs.existsSync(explicitPath)) throw new Error(`Citum binary does not exist: ${explicitPath}`);
    return explicitPath;
  }
  const run = dependencies.execFileSync || execFileSync;
  const environment = dependencies.environment || process.env;
  run('cargo', ['build', '--quiet', '--bin', 'citum'], {
    cwd: PROJECT_ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const targetRoot = environment.CARGO_TARGET_DIR
    ? path.resolve(environment.CARGO_TARGET_DIR)
    : path.join(PROJECT_ROOT, 'target');
  const binary = path.join(targetRoot, 'debug', process.platform === 'win32' ? 'citum.exe' : 'citum');
  if (!fs.existsSync(binary)) throw new Error(`Source-built Citum binary is missing: ${binary}`);
  return binary;
}

function runRegenerationCheck(registration, citumBin, dependencies = {}) {
  const run = dependencies.execFileSync || execFileSync;
  try {
    run(process.execPath, [
      GENERATOR_PATH,
      '--manifest', resolveRepoPath(registration.manifest),
      '--json-out', resolveRepoPath(registration.packet),
      '--markdown-out', resolveRepoPath(registration.markdown),
      '--citum-bin', citumBin,
      '--check',
    ], {
      cwd: PROJECT_ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch (error) {
    const detail = String(error.stderr || error.stdout || error.message).trim();
    throw new Error(`${registration.style_id}: byte regeneration failed${detail ? `: ${detail}` : ''}`);
  }
}

function checkRegistration(registration, citumBin, dependencies = {}) {
  validateRegistrationFiles(registration);
  const manifest = validateManifest(readData(resolveRepoPath(registration.manifest)));
  const packet = readData(resolveRepoPath(registration.packet));
  if (manifest.style.id !== registration.style_id) {
    throw new Error(`${registration.style_id}: manifest style ID is ${manifest.style.id}`);
  }
  validateCoveragePacket(packet, registration, manifest);
  verifyManifestFiles(manifest);
  runRegenerationCheck(registration, citumBin, dependencies);
}

function selectRegistrations(provenance, statusStyle) {
  const registrations = provenance.coverage_audits || [];
  if (!statusStyle) return registrations;
  return registrations.filter((entry) => entry.style_id === statusStyle);
}

function runChecks(options, dependencies = {}) {
  const provenance = dependencies.provenance || loadReportProvenance();
  const registrations = selectRegistrations(provenance, options.statusStyle);
  if (options.statusStyle && registrations.length === 0) {
    return { status: 'not registered', registrations: [] };
  }
  if (registrations.length === 0) {
    throw new Error('No style coverage audits are registered');
  }
  const citumBin = dependencies.citumBin
    || sourceCitumBinary(options.citumBin, dependencies);
  for (const registration of registrations) {
    checkRegistration(registration, citumBin, dependencies);
  }
  return { status: 'current', registrations };
}

function main() {
  let options;
  try {
    options = parseArgs();
    const result = runChecks(options);
    if (options.statusStyle) {
      process.stdout.write(`${options.statusStyle}: ${result.status}\n`);
    } else {
      process.stdout.write(`Coverage audits current: ${result.registrations.length} registered\n`);
    }
  } catch (error) {
    if (options?.statusStyle) {
      process.stderr.write(`${options.statusStyle}: stale — ${error.message}\n`);
    } else {
      process.stderr.write(`Coverage audit check failed: ${error.message}\n`);
    }
    process.exitCode = 1;
  }
}

if (require.main === module) main();

module.exports = {
  checkRegistration,
  parseArgs,
  runChecks,
  runRegenerationCheck,
  selectRegistrations,
  sourceCitumBinary,
};
