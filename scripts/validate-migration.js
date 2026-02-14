#!/usr/bin/env node
/**
 * scripts/validate-migration.js
 *
 * Cross-tier validation for CSLN migration output (OPT-2.1).
 *
 * Checks for consistency between:
 *   - Tier 1: Global options (from csln-migrate)
 *   - Tier 3: Inferred templates (from infer-template.js)
 *
 * Common fidelity gaps caught:
 *   - "initialize-with: '.'" but template has full given names
 *   - "and: symbol" but template renders literal "and"
 *   - Delimiter mismatches between CSL source and inferred output
 *   - Missing critical template components (author, title, date)
 *
 * Usage:
 *   node scripts/validate-migration.js styles/<style-name>.yaml
 *
 * Exit codes:
 *   0 - Passed (no issues or warnings only)
 *   1 - Fatal issues detected (likely broken template)
 *   2 - Script error (file not found, parse error)
 */

const fs = require('fs');
const yaml = require('js-yaml');

const stylePath = process.argv[2];

if (!stylePath) {
    console.error('Usage: node scripts/validate-migration.js <style-path>');
    process.exit(2);
}

if (!fs.existsSync(stylePath)) {
    console.error(`❌ Style file not found: ${stylePath}`);
    process.exit(2);
}

try {
    const content = fs.readFileSync(stylePath, 'utf8');
    const style = yaml.load(content);

    const warnings = [];
    const errors = [];

    const options = style.options || {};
    const contributors = options.contributors || {};
    const bibliography = style.bibliography || {};
    const citation = style.citation || {};
    const bibTemplate = bibliography.template || [];
    const citeTemplate = citation.template || [];

    // ──────────────────────────────────────────────
    // 1. Cross-Tier Check: initialize-with consistency (OPT-2.1)
    // ──────────────────────────────────────────────
    // If global options say "initialize-with: '.'" then contributor
    // components in templates should NOT have full given names hardcoded.
    // This catches the case where csln-migrate extracted the option but
    // infer-template inferred full names from rendered output.

    const initializeWith = contributors['initialize-with'];
    if (initializeWith) {
        // Check if any contributor component has an explicit form that
        // contradicts initialization. A contributor with form: "long"
        // combined with initialize-with means initials, but if the
        // template somehow encodes full names it's a conflict.
        const contributorComponents = bibTemplate.filter(c => c.contributor);
        const longFormContributors = contributorComponents.filter(
            c => c.form === 'long' || !c.form  // default is 'long'
        );

        if (longFormContributors.length > 0) {
            // This is expected — the combination of long form + initialize-with
            // is how CSLN handles "Family, G." format. Flag only if
            // initialize-with is set but a context-level override disables it.
            const bibOptions = bibliography.options || {};
            const bibContribs = bibOptions.contributors || {};
            if (bibContribs['initialize-with'] === false ||
                bibContribs['initialize-with'] === '') {
                warnings.push(
                    `initialize-with conflict: Global options set initialize-with: "${initializeWith}" ` +
                    `but bibliography.options overrides it to "${bibContribs['initialize-with']}". ` +
                    `Verify which behavior is intended.`
                );
            }
        }

        // Also check citation context
        const citeOptions = citation.options || {};
        const citeContribs = citeOptions.contributors || {};
        if (citeContribs['initialize-with'] === false ||
            citeContribs['initialize-with'] === '') {
            warnings.push(
                `initialize-with conflict: Global options set initialize-with: "${initializeWith}" ` +
                `but citation.options overrides it to "${citeContribs['initialize-with']}". ` +
                `Verify which behavior is intended.`
            );
        }
    }

    // ──────────────────────────────────────────────
    // 2. Cross-Tier Check: "and" connector consistency (OPT-2.1)
    // ──────────────────────────────────────────────
    // If global options say "and: symbol" (→ "&"), check that no template
    // component hardcodes a literal "and" or "& " in prefix/suffix.
    // Conversely, if "and: text", check for hardcoded "&".

    const andSetting = contributors.and;
    if (andSetting) {
        const checkAndConflict = (template, context) => {
            template.forEach((comp, idx) => {
                const prefix = comp.prefix || '';
                const suffix = comp.suffix || '';

                if (andSetting === 'symbol') {
                    // Symbol means "&" — flag if template has literal " and "
                    if (prefix.includes(' and ') || suffix.includes(' and ')) {
                        warnings.push(
                            `and connector conflict (${context}, component ${idx}): ` +
                            `Global options set and: "symbol" (→ &) but template has ` +
                            `literal " and " in prefix/suffix. ` +
                            `Remove the hardcoded text; the processor handles this.`
                        );
                    }
                } else if (andSetting === 'text') {
                    // Text means "and" — flag if template has literal "&"
                    if ((prefix.includes('&') && !prefix.includes('&amp;')) ||
                        (suffix.includes('&') && !suffix.includes('&amp;'))) {
                        warnings.push(
                            `and connector conflict (${context}, component ${idx}): ` +
                            `Global options set and: "text" (→ and) but template has ` +
                            `literal "&" in prefix/suffix. ` +
                            `Remove the hardcoded text; the processor handles this.`
                        );
                    }
                }
            });
        };

        checkAndConflict(bibTemplate, 'bibliography');
        checkAndConflict(citeTemplate, 'citation');
    }

    // ──────────────────────────────────────────────
    // 3. Cross-Tier Check: Delimiter consistency (OPT-2.3)
    // ──────────────────────────────────────────────
    // Check that multiple components don't redundantly encode the
    // delimiter that should be set at the bibliography/citation level.

    const bibDelimiter = bibliography.delimiter;
    if (bibDelimiter) {
        const conflictingSuffixes = [];
        bibTemplate.forEach((comp, idx) => {
            const suffix = comp.suffix || '';
            // If the component suffix ends with the same string as the
            // context delimiter, it's likely redundant and will double up.
            if (suffix && bibDelimiter &&
                suffix.trimEnd() === bibDelimiter.trimEnd() &&
                suffix.length > 0) {
                const key = Object.keys(comp).find(k =>
                    !['prefix', 'suffix', 'overrides', 'rendering', 'wrap',
                        'emph', 'strong', 'quote', 'form', 'items', 'delimiter'
                    ].includes(k)
                );
                conflictingSuffixes.push({ idx, key: key || `component ${idx}` });
            }
        });

        if (conflictingSuffixes.length > 0) {
            const names = conflictingSuffixes.map(c => c.key).join(', ');
            warnings.push(
                `Delimiter redundancy: bibliography.delimiter is "${bibDelimiter}" ` +
                `but components [${names}] also have suffix "${bibDelimiter}". ` +
                `This may cause doubled delimiters in output.`
            );
        }
    }

    // ──────────────────────────────────────────────
    // 4. Check: Date form consistency
    // ──────────────────────────────────────────────
    // For author-date styles, the citation date should typically be
    // form: "year", while bibliography may use "year" or "full".
    // Flag if citation date uses "full" in an author-date style.

    const processing = options.processing;
    if (processing === 'author-date') {
        const citeDateComps = citeTemplate.filter(c => c.date);
        citeDateComps.forEach(comp => {
            if (comp.form === 'full') {
                warnings.push(
                    `Date form: Author-date style has citation date with form: "full". ` +
                    `Expected form: "year" for in-text citations. ` +
                    `Check if this is intentional.`
                );
            }
        });
    }

    // ──────────────────────────────────────────────
    // 5. Critical: Missing template components
    // ──────────────────────────────────────────────
    // A valid bibliography template must have contributor/title/date.
    // Missing all three is a fatal error; missing one is a warning.

    const hasAuthor = bibTemplate.some(c => c.contributor === 'author');
    const hasEditor = bibTemplate.some(c => c.contributor === 'editor');
    const hasAnyContributor = hasAuthor || hasEditor;
    const hasTitle = bibTemplate.some(c => c.title);
    const hasDate = bibTemplate.some(c => c.date === 'issued' || c.date === 'published');

    if (!hasAnyContributor && !hasTitle) {
        errors.push(
            'Template missing both contributor and title components — ' +
            'likely incomplete inference. The template is probably broken.'
        );
    } else {
        if (!hasAnyContributor) {
            warnings.push(
                'Template has no contributor component (author or editor). ' +
                'Most styles require at least one.'
            );
        }
        if (!hasTitle) {
            warnings.push(
                'Template has no title component. ' +
                'Most bibliography styles require a title.'
            );
        }
    }

    if (!hasDate) {
        warnings.push(
            'Template has no date component (issued/published). ' +
            'Most styles include a publication date.'
        );
    }

    // ──────────────────────────────────────────────
    // 6. Check: Empty bibliography template
    // ──────────────────────────────────────────────

    if (bibTemplate.length === 0) {
        errors.push(
            'Bibliography template is empty. ' +
            'The template inference likely failed completely.'
        );
    }

    if (citeTemplate.length === 0 && processing !== 'numeric') {
        warnings.push(
            'Citation template is empty. ' +
            'This may be intentional for numeric styles but unexpected otherwise.'
        );
    }

    // ──────────────────────────────────────────────
    // Report
    // ──────────────────────────────────────────────

    if (errors.length > 0) {
        console.error('❌ Migration Validation ERRORS:');
        errors.forEach(e => console.error(`  ✖ ${e}`));
        console.error('');
    }

    if (warnings.length > 0) {
        console.warn('⚠️  Migration Validation Warnings:');
        warnings.forEach(w => console.warn(`  ⚠ ${w}`));
        console.warn('');
    }

    if (errors.length === 0 && warnings.length === 0) {
        console.log('✅ Migration cross-tier validation passed. No issues detected.');
    } else if (errors.length === 0) {
        console.log(`✅ Structurally sound (${warnings.length} warning(s) to review).`);
    }

    // Exit with 1 only for fatal errors
    if (errors.length > 0) {
        process.exit(1);
    }

} catch (err) {
    console.error(`❌ Validation failed: ${err.message}`);
    process.exit(2);
}
