#!/usr/bin/env node
/**
 * Focused Migration Oracle (7-item subset)
 *
 * Fast validation script for migration workflow.
 * Uses same 7-item subset as prep-migration.sh for consistency.
 *
 * Usage:
 *   node oracle-migration.js styles-legacy/apa.csl
 *   node oracle-migration.js styles-legacy/apa.csl --json
 *
 * Exit codes:
 *   0 - Success (meets quality threshold ≥5/7 items)
 *   1 - Failed validation (below threshold)
 *   2 - Fatal error (file not found, parse error, CSLN rendering failed)
 */

const CSL = require('citeproc');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const {
    normalizeText,
    loadLocale,
} = require('./oracle-utils');

// 7-item focused test set (same as oracle-simple.js)
const testItems = {
    "ITEM-1": {
        "id": "ITEM-1",
        "type": "article-journal",
        "title": "The Structure of Scientific Revolutions",
        "author": [
            { "family": "Kuhn", "given": "Thomas S." }
        ],
        "issued": { "date-parts": [[1962]] },
        "container-title": "International Encyclopedia of Unified Science",
        "volume": "2",
        "issue": "2",
        "publisher": "University of Chicago Press",
        "publisher-place": "Chicago",
        "DOI": "10.1234/example"
    },
    "ITEM-2": {
        "id": "ITEM-2",
        "type": "book",
        "title": "A Brief History of Time",
        "author": [
            { "family": "Hawking", "given": "Stephen" }
        ],
        "issued": { "date-parts": [[1988]] },
        "publisher": "Bantam Dell Publishing Group",
        "publisher-place": "New York"
    },
    "ITEM-3": {
        "id": "ITEM-3",
        "type": "article-journal",
        "title": "Deep Learning",
        "author": [
            { "family": "LeCun", "given": "Yann" },
            { "family": "Bengio", "given": "Yoshua" },
            { "family": "Hinton", "given": "Geoffrey" }
        ],
        "issued": { "date-parts": [[2015]] },
        "container-title": "Nature",
        "volume": "521",
        "page": "436-444",
        "DOI": "10.1038/nature14539"
    },
    "ITEM-4": {
        "id": "ITEM-4",
        "type": "chapter",
        "title": "The Role of Deliberate Practice",
        "author": [
            { "family": "Ericsson", "given": "K. Anders" }
        ],
        "editor": [
            { "family": "Ericsson", "given": "K. Anders" },
            { "family": "Charness", "given": "Neil" },
            { "family": "Feltovich", "given": "Paul J." },
            { "family": "Hoffman", "given": "Robert R." }
        ],
        "issued": { "date-parts": [[2006]] },
        "container-title": "The Cambridge Handbook of Expertise and Expert Performance",
        "publisher": "Cambridge University Press",
        "page": "683-703"
    },
    "ITEM-5": {
        "id": "ITEM-5",
        "type": "report",
        "title": "World Development Report 2023",
        "author": [
            { "literal": "World Bank" }
        ],
        "issued": { "date-parts": [[2023]] },
        "publisher": "World Bank Group",
        "publisher-place": "Washington, DC"
    },
    "ITEM-14": {
        "id": "ITEM-14",
        "type": "book",
        "title": "Handbook of Research Methods in Social Psychology",
        "editor": [
            { "family": "Reis", "given": "Harry T." },
            { "family": "Judd", "given": "Charles M." }
        ],
        "issued": { "date-parts": [[2000]] },
        "publisher": "Cambridge University Press",
        "publisher-place": "Cambridge"
    },
    "ITEM-15": {
        "id": "ITEM-15",
        "type": "article-journal",
        "title": "The Role of Theory in Research",
        "issued": { "date-parts": [[2018]] },
        "container-title": "Journal of Theoretical Psychology",
        "volume": "28",
        "issue": "3",
        "page": "201-215"
    }
};

function createSys(items) {
    return {
        retrieveLocale: function (lang) {
            return loadLocale(lang);
        },
        retrieveItem: function (id) {
            return items[id];
        }
    };
}

function renderWithCiteprocJs(stylePath) {
    const styleXml = fs.readFileSync(stylePath, 'utf8');
    const sys = createSys(testItems);
    const citeproc = new CSL.Engine(sys, styleXml);

    const itemIds = Object.keys(testItems);
    citeproc.updateItems(itemIds);

    const citations = [];
    const bibliography = [];

    for (const id of itemIds) {
        citations.push({ id, text: citeproc.makeCitationCluster([{ id }]) });
    }

    const bib = citeproc.makeBibliography();
    if (bib && bib[1]) {
        const entryIds = bib[0].entry_ids;
        bib[1].forEach((entry, idx) => {
            bibliography.push({ id: entryIds[idx][0], text: entry });
        });
    }

    return { citations, bibliography };
}

function renderWithCsln(stylePath) {
    const styleName = path.basename(stylePath, '.csl');
    const cslnStylePath = path.join(__dirname, '..', 'styles', `${styleName}.yaml`);

    if (!fs.existsSync(cslnStylePath)) {
        console.error(`❌ CSLN style not found: ${cslnStylePath}`);
        console.error('\nRun prep-migration.sh first to generate the CSLN style.');
        process.exit(2);
    }

    // Create temp fixture with 7-item subset
    const tmpFixture = path.join(__dirname, '..', '.tmp-migration-fixture.json');
    fs.writeFileSync(tmpFixture, JSON.stringify(testItems, null, 2));

    try {
        // Render bibliography
        const bibOutput = execSync(
            `cargo run -q --bin csln -- process ${tmpFixture} ${cslnStylePath} --bib --show-keys`,
            { encoding: 'utf8', cwd: path.join(__dirname, '..') }
        );

        // Render citations
        const citeOutput = execSync(
            `cargo run -q --bin csln -- process ${tmpFixture} ${cslnStylePath} --cite --show-keys`,
            { encoding: 'utf8', cwd: path.join(__dirname, '..') }
        );

        // Parse output
        const citations = [];
        const bibliography = [];

        const citeLines = citeOutput.split('\n').filter(line => line.match(/^\s*\[ITEM-\d+\]/));
        citeLines.forEach(line => {
            const match = line.match(/^\s*\[(ITEM-\d+)\]\s+(.+)$/);
            if (match) {
                citations.push({ id: match[1], text: match[2] });
            }
        });

        const bibLines = bibOutput.split('\n').filter(line => line.match(/^\s*\[ITEM-\d+\]/));
        bibLines.forEach(line => {
            const match = line.match(/^\s*\[(ITEM-\d+)\]\s+(.+)$/);
            if (match) {
                bibliography.push({ id: match[1], text: match[2] });
            }
        });

        return { citations, bibliography };
    } finally {
        // Cleanup
        if (fs.existsSync(tmpFixture)) {
            fs.unlinkSync(tmpFixture);
        }
    }
}

function compareOutputs(oracle, csln) {
    let citationMatches = 0;
    let bibliographyMatches = 0;
    const mismatches = [];

    // Build maps for CSLN outputs (indexed by ID)
    const cslnCiteMap = new Map();
    csln.citations.forEach(cite => {
        cslnCiteMap.set(cite.id, normalizeText(cite.text));
    });

    const cslnBibMap = new Map();
    csln.bibliography.forEach(bib => {
        cslnBibMap.set(bib.id, normalizeText(bib.text));
    });

    // Compare citations by ID
    for (const oracleCite of oracle.citations) {
        const oracleText = normalizeText(oracleCite.text);
        const cslnText = cslnCiteMap.get(oracleCite.id) || '';

        if (oracleText === cslnText) {
            citationMatches++;
        } else {
            mismatches.push({
                type: 'citation',
                id: oracleCite.id,
                oracle: oracleText,
                csln: cslnText
            });
        }
    }

    // Compare bibliography by ID
    for (const oracleBib of oracle.bibliography) {
        const oracleText = normalizeText(oracleBib.text);
        const cslnText = cslnBibMap.get(oracleBib.id) || '';

        if (oracleText === cslnText) {
            bibliographyMatches++;
        } else {
            mismatches.push({
                type: 'bibliography',
                id: oracleBib.id,
                oracle: oracleText,
                csln: cslnText
            });
        }
    }

    return {
        citations: { matches: citationMatches, total: oracle.citations.length },
        bibliography: { matches: bibliographyMatches, total: oracle.bibliography.length },
        mismatches
    };
}

// Main
const args = process.argv.slice(2);
const stylePath = args.find(arg => !arg.startsWith('--'));
const jsonOutput = args.includes('--json');

if (!stylePath) {
    console.error('Usage: node oracle-migration.js <style.csl> [--json]');
    process.exit(2);
}

if (!fs.existsSync(stylePath)) {
    console.error(`Style not found: ${stylePath}`);
    process.exit(2);
}

console.error('\n=== Migration Oracle (7-item focused test) ===\n');
console.error('Rendering with citeproc-js...');
const oracle = renderWithCiteprocJs(stylePath);

console.error('Rendering with CSLN...');
const csln = renderWithCsln(stylePath);

console.error('Comparing outputs...\n');
const results = compareOutputs(oracle, csln);

if (jsonOutput) {
    console.log(JSON.stringify(results, null, 2));
} else {
    if (results.mismatches.length > 0) {
        console.log('\n┌──────────────────────────────────────────────────────────────┐');
        console.log('│                   MIGRATION MISMATCH ANALYSIS                │');
        console.log('└──────────────────────────────────────────────────────────────┘\n');

        results.mismatches.forEach(mm => {
            console.log(`ID: [${mm.id}] | TYPE: ${mm.type.toUpperCase()}`);
            console.log(`┌─────────┬────────────────────────────────────────────────────┐`);
            console.log(`│ SOURCE  │ RENDERING                                          │`);
            console.log(`├─────────┼────────────────────────────────────────────────────┤`);

            const wrap = (text, width) => {
                const lines = [];
                for (let i = 0; i < text.length; i += width) {
                    lines.push(text.substring(i, i + width));
                }
                return lines.length > 0 ? lines : [""];
            };

            const oracleLines = wrap(mm.oracle, 50);
            const cslnLines = wrap(mm.csln, 50);

            console.log(`│ ORACLE  │ ${oracleLines[0].padEnd(50)} │`);
            for (let i = 1; i < oracleLines.length; i++) console.log(`│         │ ${oracleLines[i].padEnd(50)} │`);

            console.log(`├─────────┼────────────────────────────────────────────────────┤`);

            console.log(`│ CSLN    │ ${cslnLines[0].padEnd(50)} │`);
            for (let i = 1; i < cslnLines.length; i++) console.log(`│         │ ${cslnLines[i].padEnd(50)} │`);

            console.log(`└─────────┴────────────────────────────────────────────────────┘\n`);
        });
    } else {
        console.log('\n✅ PERFECT MATCH (100%)\n');
    }

    console.log(`Citations: ${results.citations.matches}/${results.citations.total} match`);
    console.log(`Bibliography: ${results.bibliography.matches}/${results.bibliography.total} match`);

    const totalMatches = results.citations.matches + results.bibliography.matches;
    const totalItems = results.citations.total + results.bibliography.total;

    console.log(`\nOverall Score: ${Math.round(totalMatches / totalItems * 100)}%`);

    // Success threshold: ≥5/7 items (71%)
    const threshold = 5;
    if (totalMatches >= threshold * 2) {  // *2 because citations + bibliography
        console.log('\n✅ PASS (≥71% match)');
        process.exit(0);
    } else {
        console.log('\n❌ FAIL (<71% match)');
        process.exit(1);
    }
}
