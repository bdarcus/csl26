#!/usr/bin/env node
/**
 * scripts/merge-migration.js
 *
 * Merges the output of csln-migrate (base.yaml) with the inferred templates
 * from infer-template.js (citation.json, bibliography.json).
 *
 * Usage:
 *   node scripts/merge-migration.js <style-name> <base-yaml> <citation-json> <bibliography-json>
 */

const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');
const CUSTOM_TAG_SCHEMA = yaml.DEFAULT_SCHEMA.extend([
    new yaml.Type('!custom', {
        kind: 'mapping',
        construct(data) {
            return data || {};
        },
    }),
]);

// Parse arguments
const args = process.argv.slice(2);
if (args.length !== 4) {
    console.error('Usage: node scripts/merge-migration.js <style-name> <base-yaml> <citation-json> <bibliography-json>');
    process.exit(1);
}

const [styleName, basePath, citePath, bibPath] = args;
const outputPath = path.join(__dirname, '..', 'styles', `${styleName}.yaml`);

function isNumericProcessing(baseData) {
    return baseData?.options?.processing === 'numeric';
}

function applyNumericCitationFallback(baseData) {
    if (!isNumericProcessing(baseData)) return false;
    if (!baseData.citation || typeof baseData.citation !== 'object') return false;

    const citationTemplate = baseData.citation.template;
    if (!Array.isArray(citationTemplate) || citationTemplate.length > 0) return false;

    // An explicit empty template blocks preset resolution in csln_core.
    // Remove it and fall back to the numeric citation preset instead.
    delete baseData.citation.template;
    if (!baseData.citation['use-preset']) {
        baseData.citation['use-preset'] = 'numeric-citation';
    }
    return true;
}

function normalizeNumericLocatorPattern(baseData) {
    if (!isNumericProcessing(baseData)) return false;
    const citation = baseData.citation;
    if (!citation || typeof citation !== 'object' || !Array.isArray(citation.template)) {
        return false;
    }

    if (citation.wrap !== 'parentheses' || citation.template.length !== 1) return false;
    const first = citation.template[0];
    if (!first || !Array.isArray(first.items)) return false;

    const numberComp = first.items.find((item) => item && item.number === 'citation-number');
    const locatorComp = first.items.find((item) => item && item.variable === 'locator');
    if (!numberComp || !locatorComp) return false;

    // Normalize the pattern emitted by XML fallback for AMA-like numeric variants:
    // "(1, p. 23)" -> "1(p23)" style rendering expected by oracle.
    if (locatorComp['show-label'] === true && locatorComp.prefix === ', ') {
        delete locatorComp['show-label'];
        delete locatorComp.prefix;
        locatorComp['strip-label-periods'] = true;
        locatorComp.wrap = 'parentheses';
        delete citation.wrap;
        return true;
    }

    return false;
}

try {
    // 1. Load Base YAML (from csln-migrate)
    if (!fs.existsSync(basePath)) throw new Error(`Base YAML not found: ${basePath}`);
    const baseContent = fs.readFileSync(basePath, 'utf8');
    const baseData = yaml.load(baseContent, { schema: CUSTOM_TAG_SCHEMA });

    // 2. Load Inferred Templates (from infer-template.js)
    if (!fs.existsSync(citePath)) throw new Error(`Citation JSON not found: ${citePath}`);
    const citeData = JSON.parse(fs.readFileSync(citePath, 'utf8'));

    if (!fs.existsSync(bibPath)) throw new Error(`Bibliography JSON not found: ${bibPath}`);
    const bibData = JSON.parse(fs.readFileSync(bibPath, 'utf8'));

    // 3. Merge Data
    // Merge citation template (supports legacy fragment key "bibliography")
    // Guard against empty inferred templates, which can overwrite a valid
    // base citation template from csln-migrate and tank citation fidelity.
    const citationTemplate = citeData.citation?.template || citeData.bibliography?.template;
    const hasInferredCitationTemplate =
        Array.isArray(citationTemplate) && citationTemplate.length > 0;
    if (hasInferredCitationTemplate) {
        if (!baseData.citation) baseData.citation = {};
        baseData.citation.template = citationTemplate;

        // Merge other inferred citation properties if present and not in base
        // (e.g., wrap, delimiter)
        if (citeData.meta) {
            // Only if base doesn't have them or they are empty?
            // For now, let's trust the inferred ones for formatting properties if available
        }
    }

    // OPT-1.4: Confidence gate — reject low-confidence inferences early
    if (bibData.meta && typeof bibData.meta.confidence === 'number') {
        const confidence = bibData.meta.confidence;
        if (confidence < 0.70) {
            console.error('❌ Low Confidence Template Rejected\n');
            console.error(`Confidence: ${(confidence * 100).toFixed(0)}% (threshold: 70%)`);
            console.error(`Source: bibliography template inference\n`);
            console.error('The inferred template is unlikely to produce correct output.');
            console.error('');
            console.error('Next Steps:');
            console.error('  1. Use @dstyleplan for Phase 1 manual research');
            console.error('  2. Check if the style has exotic features (complex conditionals)');
            console.error('  3. Try re-running with a different fixture set');
            process.exit(1);
        }
    }

    if (citeData.meta && typeof citeData.meta.confidence === 'number') {
        const confidence = citeData.meta.confidence;
        if (confidence < 0.70) {
            console.error('⚠️  Low Confidence Citation Template\n');
            console.error(`Confidence: ${(confidence * 100).toFixed(0)}% (threshold: 70%)`);
            console.error('Proceeding with merge, but citation template may need manual review.');
            console.error('');
        }
    }

    // Merge bibliography template. As with citations, avoid clobbering
    // existing content with an empty inferred array.
    const bibliographyTemplate = bibData.bibliography?.template;
    const hasInferredBibliographyTemplate =
        Array.isArray(bibliographyTemplate) && bibliographyTemplate.length > 0;
    if (hasInferredBibliographyTemplate) {
        if (!baseData.bibliography) baseData.bibliography = {};
        baseData.bibliography.template = bibliographyTemplate;
    }

    // Post-merge normalization for numeric citation robustness.
    applyNumericCitationFallback(baseData);
    normalizeNumericLocatorPattern(baseData);

    // Add auto-generation comment header
    const comment = `# Generated by CSLN Migration Workflow
# Date: ${new Date().toISOString().split('T')[0]}
# Base Options: csln-migrate (Rust)
# Templates: infer-template.js (Output-Driven)
#
# REVIEW INSTRUCTIONS:
# 1. Verify 'options' block against requirements.
# 2. Check 'template' blocks against Oracle output.
# 3. Add any missing conditional logic or specific overrides.
#
`;

    // 4. Write Output
    // Use a custom schema or options if needed to preserve order/formatting preferences?
    // js-yaml doesn't preserve comments, so we prepend the header.
    // We want to keep key order close to standard if possible.

    // Custom dump to ensure top-level order
    const orderedData = {
        version: baseData.version || "",
        info: baseData.info || {},
        options: baseData.options || {},
        citation: baseData.citation || {},
        bibliography: baseData.bibliography || {}
    };

    const yamlStr = yaml.dump(orderedData, {
        lineWidth: 120,
        noRefs: true,
        quotingType: '"'
    });

    fs.writeFileSync(outputPath, comment + yamlStr);
    console.log(`Successfully merged migration data to: ${outputPath}`);

} catch (err) {
    console.error('❌ Migration Merge Failed\n');
    console.error(`Error: ${err.message}`);

    if (err.message.includes('not found')) {
        console.error('\nMissing required input files:');
        console.error(`  - Base YAML: ${basePath}`);
        console.error(`  - Citation template: ${citePath}`);
        console.error(`  - Bibliography template: ${bibPath}`);
        console.error('\nEnsure prep-migration.sh completed successfully.');
    } else if (err.message.includes('parse')) {
        console.error('\nInvalid JSON/YAML format in input files.');
        console.error('Check that infer-template.js and csln-migrate produced valid output.');
    }

    console.error('\nNext Steps:');
    console.error('  1. Re-run prep-migration.sh to regenerate input files');
    console.error('  2. Check error logs from previous pipeline steps');
    console.error('  3. Verify CSL source file is valid XML');
    process.exit(2);
}
