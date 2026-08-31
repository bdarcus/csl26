const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const yaml = require('js-yaml');
const Ajv2020 = require('ajv/dist/2020');
const addFormats = require('ajv-formats');

const ajv = new Ajv2020({ allErrors: true, strict: false });
addFormats(ajv);
ajv.addFormat('uint8', true);
ajv.addFormat('uint32', true);

const rootDir = path.join(__dirname, '..');
const schemaDir = path.join(rootDir, 'docs/schemas');
const skippedExampleFiles = new Set(['chicago-bib.yaml']);

// Styles with a known schema gap, tracked and expected to be removed from here.
// Keep this list empty-by-default; an entry is a debt, not a configuration knob.
const skippedStyleFiles = new Map([
  // Authors `contributor: narrator` and `contributor: contributor`, which are
  // not ContributorRole variants. `str_enum!` accepts them via its Unknown
  // fallback while the derived schema is a closed enum. See csl26-9gb9.
  ['chicago-author-date-18th.yaml', 'csl26-9gb9']
]);

const schemas = {
  style: JSON.parse(fs.readFileSync(path.join(schemaDir, 'style.json'), 'utf8')),
  bib: JSON.parse(fs.readFileSync(path.join(schemaDir, 'bib.json'), 'utf8')),
  locale: JSON.parse(fs.readFileSync(path.join(schemaDir, 'locale.json'), 'utf8')),
  citation: JSON.parse(fs.readFileSync(path.join(schemaDir, 'citation.json'), 'utf8'))
};

const ModeDependentType = new yaml.Type('!mode-dependent', {
  kind: 'mapping',
  construct: function (data) {
    return { 'mode-dependent': data };
  }
});

const Citum_SCHEMA = yaml.DEFAULT_SCHEMA.extend([ModeDependentType]);

function normalizeForSchemaKey(data, schemaKey) {
  if (
    schemaKey === 'citation' &&
    data &&
    typeof data === 'object' &&
    !Array.isArray(data) &&
    Array.isArray(data.citations)
  ) {
    return data.citations;
  }

  return data;
}

function validate(filePath, schemaKey) {
  const content = fs.readFileSync(filePath, 'utf8');
  let data;
  if (filePath.endsWith('.yaml') || filePath.endsWith('.yml')) {
    data = yaml.load(content, { schema: Citum_SCHEMA });
  } else if (filePath.endsWith('.json')) {
    data = JSON.parse(content);
  } else {
    return; // Skip other formats
  }

  data = normalizeForSchemaKey(data, schemaKey);

  const validateFn = ajv.compile(schemas[schemaKey]);
  const valid = validateFn(data);

  if (!valid) {
    console.error(`❌ ${filePath} failed validation against ${schemaKey} schema:`);
    console.error(JSON.stringify(validateFn.errors, null, 2));
    return false;
  } else {
    console.log(`✅ ${filePath} passed validation against ${schemaKey} schema.`);
    return true;
  }
}

function validateWithCli(kind, flag, filePath) {
  try {
    execFileSync(
      'cargo',
      ['run', '-q', '-p', 'citum', '--', 'check', flag, filePath],
      { cwd: rootDir, stdio: 'pipe' }
    );
    console.log(`✅ ${filePath} passed ${kind} validation via citum check.`);
    return true;
  } catch (error) {
    const stderr = error.stderr ? error.stderr.toString().trim() : '';
    const stdout = error.stdout ? error.stdout.toString().trim() : '';
    const details = stderr || stdout || error.message;
    console.error(`❌ ${filePath} failed ${kind} validation via citum check:`);
    console.error(details);
    return false;
  }
}

let allValid = true;

// `--scope=styles,locales` limits the run to the sections that are green and
// gated in CI. The examples section still has pre-existing failures in bib and
// citation fixtures, unrelated to style or locale schemas; running it locally
// without the flag remains the way to see them.
const scopeArg = process.argv.slice(2).find(arg => arg.startsWith('--scope='));
const scope = scopeArg
  ? new Set(
      scopeArg
        .slice('--scope='.length)
        .split(',')
        .map(part => part.trim())
        .filter(Boolean)
    )
  : null;
const inScope = section => scope === null || scope.has(section);

// Validate Styles
//
// `styles/` holds tracked exemplar styles; `styles/embedded` symlinks the
// crate's embedded tier. A plain readdir of `styles/` skips the subdirectory,
// which hid the embedded corpus — the larger and more load-bearing half — from
// every schema check. Both are validated.
if (inScope('styles')) {
console.log('\n--- Validating Styles ---');
const styleDirs = [
  path.join(rootDir, 'styles'),
  path.join(rootDir, 'crates/citum-schema-style/embedded/styles')
];
styleDirs.forEach(dir => {
  fs.readdirSync(dir).forEach(file => {
    if (skippedStyleFiles.has(file)) {
      console.log(`SKIP ${path.join(dir, file)} — known schema gap, see ${skippedStyleFiles.get(file)}`);
      return;
    }
    if (file.endsWith('.yaml') || file.endsWith('.json')) {
      if (!validate(path.join(dir, file), 'style')) allValid = false;
    }
  });
});
}

// Validate Locales
if (inScope('locales')) {
console.log('\n--- Validating Locales ---');
const localeDir = path.join(rootDir, 'locales');
fs.readdirSync(localeDir).forEach(file => {
  if (file.endsWith('.yaml') || file.endsWith('.json')) {
    if (!validate(path.join(localeDir, file), 'locale')) allValid = false;
  }
});
}

// Validate Bibliographies in examples
if (inScope('examples')) {
console.log('\n--- Validating Examples (Bibliographies) ---');
const examplesDir = path.join(rootDir, 'examples');
fs.readdirSync(examplesDir).forEach(file => {
  if (skippedExampleFiles.has(file)) {
    console.log(`SKIP ${path.join(examplesDir, file)} is a legacy example.`);
    return;
  }

  if (file.endsWith('.yaml') || file.endsWith('.json')) {
    // Basic heuristic to distinguish bib from style in examples
    if (file.includes('cite') || file.includes('citation')) {
      if (!validate(path.join(examplesDir, file), 'citation')) allValid = false;
    } else if (file.includes('bib') || file.includes('ref')) {
      const filePath = path.join(examplesDir, file);
      if (!validate(filePath, 'bib')) allValid = false;
      if (!validateWithCli('bibliography', '--bibliography', filePath)) allValid = false;
    } else if (file.includes('style')) {
      if (!validate(path.join(examplesDir, file), 'style')) allValid = false;
    }
  }
});
}

if (!allValid) {
  process.exit(1);
}
