#!/usr/bin/env node
/**
 * scripts/check-coverage.js
 *
 * Analyzes reference type coverage in the generated style (OPT-2.2).
 *
 * Reports per-type component status with specific missing component names,
 * enabling rapid identification of incomplete template inference.
 *
 * Usage:
 *   node scripts/check-coverage.js <style-name>
 *
 * Exit codes:
 *   0 - All core types have coverage
 *   1 - One or more core types have zero components
 *   2 - Script error
 */

const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');

const styleName = process.argv[2];
const stylePath = path.join(__dirname, '..', 'styles', `${styleName}.yaml`);

if (!styleName) {
    console.error('Usage: node scripts/check-coverage.js <style-name>');
    process.exit(2);
}

if (!fs.existsSync(stylePath)) {
    console.error(`Style not found: ${stylePath}`);
    process.exit(2);
}

const REQUIRED_TYPES = [
    'article-journal',
    'book',
    'chapter',
    'report'
];

/**
 * Determine the semantic name of a template component.
 * Returns a human-readable identifier like "author", "title", "pages", etc.
 */
function getComponentName(comp) {
    if (comp.contributor) return comp.contributor;
    if (comp.title) return `title:${comp.title}`;
    if (comp.date) return `date:${comp.date}`;
    if (comp.number) return comp.number;
    if (comp.variable) return comp.variable;
    if (comp.items) return 'items-group';
    // Fallback: first key that isn't a rendering property
    const renderKeys = new Set([
        'prefix', 'suffix', 'overrides', 'rendering', 'wrap',
        'emph', 'strong', 'quote', 'form', 'delimiter',
        'small-caps', 'suppress'
    ]);
    const key = Object.keys(comp).find(k => !renderKeys.has(k));
    return key || 'unknown';
}

/**
 * Check whether a component is suppressed for a given reference type.
 */
function isSuppressed(comp, type) {
    if (!comp.overrides) return false;
    const override = comp.overrides[type];
    if (!override) return false;
    return override.suppress === true;
}

try {
    const style = yaml.load(fs.readFileSync(stylePath, 'utf8'));
    const bib = style.bibliography || {};
    const template = bib.template || [];

    console.log(`\n=== COVERAGE ANALYSIS: ${styleName} ===`);

    let hasZeroCoverage = false;

    REQUIRED_TYPES.forEach(type => {
        const activeComponents = [];
        const suppressedComponents = [];

        template.forEach(comp => {
            const name = getComponentName(comp);

            if (isSuppressed(comp, type)) {
                suppressedComponents.push(name);
            } else {
                activeComponents.push(name);
            }
        });

        const total = activeComponents.length + suppressedComponents.length;
        const active = activeComponents.length;

        let status;
        if (active === 0) {
            status = '❌';
            hasZeroCoverage = true;
        } else if (active < 3) {
            status = '⚠️';
        } else {
            status = '✅';
        }

        let line = `${status} ${type.padEnd(18)}: ${active}/${total} components active`;

        if (suppressedComponents.length > 0) {
            line += ` (suppressed: ${suppressedComponents.join(', ')})`;
        }

        console.log(line);
    });

    // Summary
    const coveredCount = REQUIRED_TYPES.filter(type => {
        return template.some(comp => !isSuppressed(comp, type));
    }).length;

    if (coveredCount < REQUIRED_TYPES.length) {
        console.log(`\n⚠️  Warning: Only ${coveredCount}/${REQUIRED_TYPES.length} core types have active components.`);
    } else {
        console.log(`\n✅ All ${REQUIRED_TYPES.length} core types have component coverage.`);
    }

    // List all components for reference
    console.log(`\nTemplate components (${template.length} total):`);
    template.forEach((comp, idx) => {
        const name = getComponentName(comp);
        const overrideTypes = comp.overrides ? Object.keys(comp.overrides) : [];
        const overrideStr = overrideTypes.length > 0
            ? ` [overrides: ${overrideTypes.join(', ')}]`
            : '';
        console.log(`  ${(idx + 1).toString().padStart(2)}. ${name}${overrideStr}`);
    });

    process.exit(hasZeroCoverage ? 1 : 0);

} catch (err) {
    console.error(`❌ Error analyzing coverage: ${err.message}`);
    process.exit(2);
}
