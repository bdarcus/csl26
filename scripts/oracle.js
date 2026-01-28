#!/usr/bin/env node
/**
 * CSL 1.0 Oracle - Renders citations using citeproc-js
 * 
 * Usage: node oracle.js <style.csl> [--bib] [--cite]
 * 
 * Outputs JSON with rendered citations/bibliography for comparison.
 */

const CSL = require('citeproc');
const fs = require('fs');
const path = require('path');

// Sample test data - a typical journal article
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
            { "family": "Cambridge", "given": "The Editors of" }
        ],
        "issued": { "date-parts": [[2006]] },
        "container-title": "The Cambridge Handbook of Expertise",
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
    }
};

// Load locale from file
const localesDir = path.join(__dirname);
function loadLocale(lang) {
    const localePath = path.join(localesDir, `locales-${lang}.xml`);
    if (fs.existsSync(localePath)) {
        return fs.readFileSync(localePath, 'utf8');
    }
    // Fallback to en-US
    const fallback = path.join(localesDir, 'locales-en-US.xml');
    if (fs.existsSync(fallback)) {
        return fs.readFileSync(fallback, 'utf8');
    }
    throw new Error(`Locale not found: ${lang}`);
}

function createSys(items) {
    return {
        retrieveLocale: function(lang) {
            return loadLocale(lang);
        },
        retrieveItem: function(id) {
            return items[id];
        }
    };
}

function renderWithStyle(stylePath, items, options = {}) {
    const styleXml = fs.readFileSync(stylePath, 'utf8');
    const sys = createSys(items);
    
    const citeproc = new CSL.Engine(sys, styleXml);
    
    const itemIds = Object.keys(items);
    citeproc.updateItems(itemIds);
    
    const result = {
        style: path.basename(stylePath),
        items: itemIds.length
    };
    
    // Generate citations
    if (options.cite !== false) {
        result.citations = [];
        for (const id of itemIds) {
            const citation = citeproc.makeCitationCluster([{ id }]);
            result.citations.push({
                id,
                text: citation
            });
        }
    }
    
    // Generate bibliography
    if (options.bib !== false) {
        const bibResult = citeproc.makeBibliography();
        if (bibResult) {
            result.bibliography = {
                params: bibResult[0],
                entries: bibResult[1].map((entry, i) => ({
                    id: itemIds[i],
                    html: entry.trim()
                }))
            };
        }
    }
    
    return result;
}

// CLI
const args = process.argv.slice(2);
if (args.length === 0) {
    console.log('Usage: node oracle.js <style.csl> [--bib] [--cite] [--json]');
    console.log('');
    console.log('Examples:');
    console.log('  node oracle.js ../styles/apa.csl');
    console.log('  node oracle.js ../styles/chicago-author-date.csl --bib');
    process.exit(1);
}

const stylePath = args[0];
const showBib = args.includes('--bib') || !args.includes('--cite');
const showCite = args.includes('--cite') || !args.includes('--bib');
const jsonOutput = args.includes('--json');

if (!fs.existsSync(stylePath)) {
    console.error(`Style not found: ${stylePath}`);
    process.exit(1);
}

try {
    const result = renderWithStyle(stylePath, testItems, { bib: showBib, cite: showCite });
    
    if (jsonOutput) {
        console.log(JSON.stringify(result, null, 2));
    } else {
        console.log(`\n=== ${result.style} ===\n`);
        
        if (result.citations) {
            console.log('CITATIONS:');
            for (const c of result.citations) {
                console.log(`  [${c.id}] ${c.text}`);
            }
            console.log('');
        }
        
        if (result.bibliography) {
            console.log('BIBLIOGRAPHY:');
            for (const entry of result.bibliography.entries) {
                // Strip HTML tags for console output
                const text = entry.html.replace(/<[^>]+>/g, '');
                console.log(`  ${text}`);
            }
        }
    }
} catch (e) {
    console.error('Error:', e.message);
    if (process.env.DEBUG) console.error(e.stack);
    process.exit(1);
}
