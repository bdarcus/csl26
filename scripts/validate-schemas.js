const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');
const Ajv = require('ajv');
const addFormats = require('ajv-formats');

const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);
ajv.addFormat('uint8', true);

const rootDir = path.join(__dirname, '..');
const schemaDir = path.join(rootDir, 'crates/csln/generated_schemas');

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

const CSLN_SCHEMA = yaml.DEFAULT_SCHEMA.extend([ModeDependentType]);

function validate(filePath, schemaKey) {
  const content = fs.readFileSync(filePath, 'utf8');
  let data;
  if (filePath.endsWith('.yaml') || filePath.endsWith('.yml')) {
    data = yaml.load(content, { schema: CSLN_SCHEMA });
  } else if (filePath.endsWith('.json')) {
    data = JSON.parse(content);
  } else {
    return; // Skip other formats
  }

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

let allValid = true;

// Validate Styles
console.log('\n--- Validating Styles ---');
const styleDirs = [path.join(rootDir, 'styles')];
styleDirs.forEach(dir => {
  fs.readdirSync(dir).forEach(file => {
    if (file.endsWith('.yaml') || file.endsWith('.json')) {
      if (!validate(path.join(dir, file), 'style')) allValid = false;
    }
  });
});

// Validate Locales
console.log('\n--- Validating Locales ---');
const localeDir = path.join(rootDir, 'locales');
fs.readdirSync(localeDir).forEach(file => {
  if (file.endsWith('.yaml') || file.endsWith('.json')) {
    if (!validate(path.join(localeDir, file), 'locale')) allValid = false;
  }
});

// Validate Bibliographies in examples
console.log('\n--- Validating Examples (Bibliographies) ---');
const examplesDir = path.join(rootDir, 'examples');
fs.readdirSync(examplesDir).forEach(file => {
  if (file.endsWith('.yaml') || file.endsWith('.json')) {
    // Basic heuristic to distinguish bib from style in examples
    if (file.includes('bib') || file.includes('ref')) {
      if (!validate(path.join(examplesDir, file), 'bib')) allValid = false;
    } else if (file.includes('style')) {
      if (!validate(path.join(examplesDir, file), 'style')) allValid = false;
    }
  }
});

if (!allValid) {
  process.exit(1);
}
