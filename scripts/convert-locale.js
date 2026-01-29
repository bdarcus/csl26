#!/usr/bin/env node
/**
 * Convert CSL XML locale files to CSLN YAML format.
 * 
 * Usage:
 *   node convert-locale.js locales-en-US.xml > ../locales/en-US.yaml
 *   node convert-locale.js --all <locales-dir> <output-dir>
 * 
 * Downloads locales from: https://github.com/citation-style-language/locales
 */

const fs = require('fs');
const path = require('path');

/**
 * Parse a CSL XML locale file and convert to CSLN YAML format.
 */
function parseLocaleXML(xmlContent) {
    // Extract locale lang
    const langMatch = xmlContent.match(/xml:lang="([^"]+)"/);
    const lang = langMatch ? langMatch[1] : 'en-US';

    const locale = {
        locale: lang,
        dates: {
            months: { long: [], short: [] },
            seasons: []
        },
        roles: {},
        terms: {}
    };

    // Parse terms section
    const termsMatch = xmlContent.match(/<terms>([\s\S]*?)<\/terms>/);
    if (!termsMatch) {
        console.error('No <terms> section found');
        return locale;
    }

    const termsContent = termsMatch[1];

    // Parse all term elements
    // Match: <term name="X">value</term> or <term name="X" form="Y">value</term>
    // Or with single/multiple children
    const termRegex = /<term\s+([^>]+)>([\s\S]*?)<\/term>/g;
    const simpleTermRegex = /<term\s+name="([^"]+)"(?:\s+form="([^"]+)")?(?:\s*\/>|>([\s\S]*?)<\/term>)/g;

    let match;
    while ((match = simpleTermRegex.exec(termsContent)) !== null) {
        const name = match[1];
        const form = match[2] || 'long';
        const content = match[3] || '';

        // Check for single/multiple children
        const singleMatch = content.match(/<single>([\s\S]*?)<\/single>/);
        const multipleMatch = content.match(/<multiple>([\s\S]*?)<\/multiple>/);

        // Decode HTML entities
        const decode = (s) => s
            .replace(/&amp;/g, '&')
            .replace(/&lt;/g, '<')
            .replace(/&gt;/g, '>')
            .replace(/&quot;/g, '"')
            .replace(/&#39;/g, "'")
            .trim();

        // Handle special term categories
        if (name.startsWith('month-')) {
            const monthNum = parseInt(name.replace('month-', ''), 10) - 1;
            if (form === 'short') {
                locale.dates.months.short[monthNum] = decode(content);
            } else {
                locale.dates.months.long[monthNum] = decode(content);
            }
        } else if (name.startsWith('season-')) {
            const seasonNum = parseInt(name.replace('season-', ''), 10) - 1;
            locale.dates.seasons[seasonNum] = decode(content);
        } else if (isRoleTerm(name)) {
            // Contributor role terms
            if (!locale.roles[name]) {
                locale.roles[name] = {};
            }
            if (singleMatch && multipleMatch) {
                locale.roles[name][form] = {
                    singular: decode(singleMatch[1]),
                    plural: decode(multipleMatch[1])
                };
            } else {
                locale.roles[name][form] = decode(content);
            }
        } else {
            // General terms
            const key = termNameToKey(name);
            if (!locale.terms[key]) {
                locale.terms[key] = {};
            }
            if (singleMatch && multipleMatch) {
                locale.terms[key][form] = {
                    singular: decode(singleMatch[1]),
                    plural: decode(multipleMatch[1])
                };
            } else if (content.trim()) {
                locale.terms[key][form] = decode(content);
            }
        }
    }

    return locale;
}

/**
 * Check if a term name is a contributor role.
 */
function isRoleTerm(name) {
    const roles = [
        'author', 'chair', 'collection-editor', 'compiler', 'composer',
        'container-author', 'contributor', 'curator', 'director', 'editor',
        'editor-translator', 'editortranslator', 'editorial-director',
        'executive-producer', 'guest', 'host', 'illustrator', 'interviewer',
        'narrator', 'organizer', 'original-author', 'performer', 'producer',
        'recipient', 'reviewed-author', 'script-writer', 'series-creator', 'translator'
    ];
    return roles.includes(name);
}

/**
 * Convert term name to a more YAML-friendly key.
 */
function termNameToKey(name) {
    return name.replace(/-/g, '_');
}

/**
 * Convert locale object to YAML string.
 */
function toYAML(obj, indent = 0) {
    const spaces = '  '.repeat(indent);
    let yaml = '';

    for (const [key, value] of Object.entries(obj)) {
        if (value === null || value === undefined) {
            continue;
        }

        if (typeof value === 'string') {
            // Escape special characters
            if (value.includes(':') || value.includes('#') || value.includes("'") ||
                value.includes('"') || value.includes('\n') || value.startsWith(' ')) {
                yaml += `${spaces}${key}: "${value.replace(/"/g, '\\"')}"\n`;
            } else {
                yaml += `${spaces}${key}: ${value}\n`;
            }
        } else if (Array.isArray(value)) {
            yaml += `${spaces}${key}:\n`;
            for (const item of value) {
                if (typeof item === 'string') {
                    yaml += `${spaces}  - ${item}\n`;
                } else if (typeof item === 'object') {
                    yaml += `${spaces}  -\n${toYAML(item, indent + 2)}`;
                }
            }
        } else if (typeof value === 'object') {
            if (Object.keys(value).length === 0) {
                continue;
            }
            yaml += `${spaces}${key}:\n${toYAML(value, indent + 1)}`;
        } else if (typeof value === 'boolean') {
            yaml += `${spaces}${key}: ${value}\n`;
        } else if (typeof value === 'number') {
            yaml += `${spaces}${key}: ${value}\n`;
        }
    }

    return yaml;
}

/**
 * Main execution
 */
function main() {
    const args = process.argv.slice(2);

    if (args.length === 0) {
        console.error('Usage: node convert-locale.js <locale.xml>');
        console.error('       node convert-locale.js --all <locales-dir> <output-dir>');
        process.exit(1);
    }

    if (args[0] === '--all') {
        // Batch convert all locale files in a directory
        const localesDir = args[1];
        const outputDir = args[2];

        if (!localesDir || !outputDir) {
            console.error('--all requires <locales-dir> and <output-dir>');
            process.exit(1);
        }

        if (!fs.existsSync(outputDir)) {
            fs.mkdirSync(outputDir, { recursive: true });
        }

        const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.xml'));
        console.error(`Converting ${files.length} locale files...`);

        for (const file of files) {
            const xmlPath = path.join(localesDir, file);
            const xmlContent = fs.readFileSync(xmlPath, 'utf8');
            const locale = parseLocaleXML(xmlContent);
            const yamlContent = toYAML(locale);

            const outputFile = file.replace('locales-', '').replace('.xml', '.yaml');
            const outputPath = path.join(outputDir, outputFile);
            fs.writeFileSync(outputPath, yamlContent);
            console.error(`  ${file} -> ${outputFile}`);
        }

        console.error('Done!');
    } else {
        // Single file conversion
        const xmlPath = args[0];
        if (!fs.existsSync(xmlPath)) {
            console.error(`File not found: ${xmlPath}`);
            process.exit(1);
        }

        const xmlContent = fs.readFileSync(xmlPath, 'utf8');
        const locale = parseLocaleXML(xmlContent);
        const yamlContent = toYAML(locale);
        console.log(yamlContent);
    }
}

main();
