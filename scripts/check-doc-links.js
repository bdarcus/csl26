#!/usr/bin/env node
// Fail on internal <a href> links in docs/**/*.html that resolve to a
// non-existent file. Catches the FEATURES.md failure mode: a doc-card or
// sidebar link surviving after its target was removed or never landed.
//
// Run after scripts/build-doc-pages.js (and, for full coverage, build-docs /
// build-layout) so pages this script itself generates are already on disk.

const fs = require('fs');
const path = require('path');

const ROOT = path.join(__dirname, '..');
const DOCS_DIR = path.join(ROOT, 'docs');

// Built by compat-report.yml (test-report.sh / migration-test-report.sh) and
// not committed, so they're legitimately absent from a plain checkout.
// docs/compat.html is committed and needs no exception.
const KNOWN_ELSEWHERE = new Set(['docs/behavior-report.html', 'docs/migration-behavior-report.html']);

// Pre-existing broken links tracked separately; see the referenced bean.
const KNOWN_BROKEN = new Set([
    'docs/news/index.html', // csl26-hlfh
]);

function walkHtmlFiles(dir, out = []) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            if (entry.name === 'node_modules') continue;
            walkHtmlFiles(full, out);
        } else if (entry.name.endsWith('.html')) {
            out.push(full);
        }
    }
    return out;
}

function checkFile(file, problems) {
    const content = fs.readFileSync(file, 'utf8');
    const dir = path.dirname(file);
    const hrefRe = /href="([^"]+)"/g;
    let match;
    while ((match = hrefRe.exec(content))) {
        const href = match[1];
        if (/^([a-z][a-z0-9+.-]*:)?\/\//i.test(href)) continue; // absolute / protocol-relative
        if (href.startsWith('#') || href.startsWith('mailto:')) continue;

        const hashIndex = href.indexOf('#');
        const pathPart = hashIndex === -1 ? href : href.slice(0, hashIndex);
        if (!pathPart || !/\.(md|html)$/.test(pathPart)) continue; // skip JSON/assets/directory links

        const resolved = path.normalize(path.join(dir, pathPart));
        const relToRepo = path.relative(ROOT, resolved).split(path.sep).join('/');
        if (KNOWN_ELSEWHERE.has(relToRepo) || KNOWN_BROKEN.has(relToRepo)) continue;

        if (!fs.existsSync(resolved)) {
            problems.push(`${path.relative(ROOT, file)} -> "${href}" (resolved: ${relToRepo})`);
        }
    }
}

function main() {
    const files = walkHtmlFiles(DOCS_DIR);
    const problems = [];
    for (const file of files) {
        checkFile(file, problems);
    }
    if (problems.length) {
        console.error(`Found ${problems.length} broken internal link(s):\n`);
        for (const p of problems) console.error(`  ${p}`);
        process.exit(1);
    }
    console.log(`Checked ${files.length} HTML files under docs/ — no broken internal links.`);
}

main();
