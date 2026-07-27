#!/usr/bin/env node
// Render selected markdown docs to themed HTML pages in docs/.
//
// Used for evergreen policy/architecture documents that should appear on
// docs.citum.org as proper pages instead of raw GitHub markdown views.
// Pair with scripts/build-layout.js, which fills the nav/footer markers
// after this script writes each page.

const fs = require('fs');
const path = require('path');
const { marked } = require('marked');

const DOCS_DIR = path.join(__dirname, '../docs');

const PAGES = [
    {
        src: 'reference/DATA_MODEL.md',
        out: 'reference/data-model.html',
        title: 'Data model',
        kicker: 'Reference',
        description:
            'Conceptual tour of the InputReference data model: reference classes, containers, contributors, dates, and ingest architecture.',
    },
    {
        src: 'policies/TYPE_ADDITION_POLICY.md',
        out: 'policies/type-addition-policy.html',
        title: 'Type addition policy',
        kicker: 'Policy',
        description:
            'Active policy governing when and how new data-model and style-discriminated types are added to Citum.',
    },
    {
        src: 'architecture/DESIGN_PRINCIPLES.md',
        out: 'architecture/design-principles.html',
        title: 'Design principles',
        kicker: 'Architecture',
        description:
            'Explicit templates, typed data, processor boundaries, and the explicit-over-magic principle that shape the Citum codebase.',
    },
    {
        src: 'architecture/MIGRATION_STRATEGY_ANALYSIS.md',
        out: 'architecture/migration-strategy.html',
        title: 'Migration strategy',
        kicker: 'Architecture',
        description:
            'Current strategy for migrating CSL 1.0 styles into Citum: hybrid XML pipeline and LLM-authored templates.',
    },
    {
        src: 'reference/NATIVE_FORMAT.md',
        out: 'reference/native-format.html',
        title: 'Native format examples',
        kicker: 'Reference',
        description: 'Worked, test-backed native-YAML examples for every InputReference class.',
    },
    {
        src: 'reference/BIBLATEX_MAPPING.md',
        out: 'reference/biblatex-mapping.html',
        title: 'BibLaTeX field mapping',
        kicker: 'Reference',
        description: 'Reference mapping between biblatex field names and the Citum input data model.',
    },
    {
        src: 'reference/generated/DATA_MODEL_FIELDS.md',
        out: 'reference/generated/data-model-fields.html',
        title: 'Data model field reference',
        kicker: 'Reference',
        description: 'Generated field tables and closed vocabularies for every InputReference class.',
    },
    {
        src: 'reference/generated/CSL_JSON_MAPPING.md',
        out: 'reference/generated/csl-json-mapping.html',
        title: 'CSL-JSON type mapping',
        kicker: 'Reference',
        description: 'Generated mapping from CSL 1.0.2 types to Citum reference types.',
    },
];

// src -> out, used to rewrite cross-page markdown links (e.g. "NATIVE_FORMAT.md")
// to the rendered .html path instead of leaving them pointed at raw markdown.
const PAGE_MAP = new Map(PAGES.map((p) => [p.src, p.out]));

const renderer = new marked.Renderer();

// marked@v5+ passes a token object to renderers; fall back to legacy signatures
// for older versions so this script is portable across the project's bumps.
function pluckHeading(arg1, arg2) {
    if (typeof arg1 === 'object') return { text: arg1.text, level: arg1.depth };
    return { text: arg1, level: arg2 };
}

renderer.heading = function (arg1, arg2) {
    const { text, level } = pluckHeading(arg1, arg2);
    // Strip the first H1 — we render the page title from front-matter instead.
    if (level === 1) return '';
    return `<h${level}>${marked.parseInline(text)}</h${level}>`;
};

// Mutable per-page context the link renderer below closes over; set before
// each marked.parse() call in build().
let currentSrcDir = '';
let currentRootPrefix = '';

const baseLink = renderer.link.bind(renderer);

// Rewrite relative .md links that resolve to another page in PAGES (e.g.
// DATA_MODEL.md linking to "NATIVE_FORMAT.md") to that page's rendered .html
// path. Links to markdown that isn't in PAGES (most "see also" references)
// are left as-is, since raw GitHub markdown is still the correct target for
// those until they're added here too.
renderer.link = function (token) {
    const href = token.href || '';
    const isRelative = !/^([a-z][a-z0-9+.-]*:)?\/\//i.test(href) && !href.startsWith('#');
    if (isRelative) {
        const hashIndex = href.indexOf('#');
        const pathPart = hashIndex === -1 ? href : href.slice(0, hashIndex);
        const hash = hashIndex === -1 ? '' : href.slice(hashIndex);
        if (pathPart.endsWith('.md')) {
            const resolvedSrc = path.posix.normalize(path.posix.join(currentSrcDir, pathPart));
            const mappedOut = PAGE_MAP.get(resolvedSrc);
            if (mappedOut) {
                return baseLink({ ...token, href: currentRootPrefix + mappedOut + hash });
            }
        }
    }
    return baseLink(token);
};

const baseTable = renderer.table.bind(renderer);

// Wrap generated tables in the themed shell (border, rounded corners, its own
// horizontal scroll) instead of a bare <table> — needed for the wide field
// tables in generated/DATA_MODEL_FIELDS.md so the page body never scrolls
// sideways.
renderer.table = function (token) {
    const html = baseTable(token).replace('<table>', '<table class="doc-table">');
    return `<div class="doc-table-shell">${html}</div>`;
};

marked.setOptions({ renderer });

// The four generated reference docs open with a "do not edit" banner comment
// meant for repo contributors, not site readers; strip any single leading
// HTML comment line before rendering.
function stripLeadingComment(md) {
    return md.replace(/^<!--[^\n]*-->\n+/, '');
}

const TEMPLATE = `<!-- PAGE_ID: docs -->
<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{{TITLE}} | Citum Docs</title>
    <meta name="description" content="{{DESCRIPTION}}" />
    <link rel="stylesheet" href="{{ROOT}}assets/citum-theme.css" />
</head>
<body>
    <nav class="site-nav">
        <!-- LAYOUT_NAV_START -->
        <!-- LAYOUT_NAV_END -->
    </nav>

    <main class="doc-shell">
        <header class="doc-section-header">
            <span class="citum-kicker">{{KICKER}}</span>
            <h1>{{TITLE}}</h1>
        </header>
        <section class="doc-section">
            <div class="doc-prose">
{{CONTENT}}
            </div>
        </section>
    </main>

    <footer style="padding: 3rem 0; border-top: 1px solid var(--citum-border); background: var(--citum-surface);">
        <!-- LAYOUT_FOOTER_START -->
        <!-- LAYOUT_FOOTER_END -->
    </footer>
    <script src="{{ROOT}}assets/citum-interactive.js"></script>
</body>
</html>
`;

function rootPrefixFor(outRelative) {
    const depth = outRelative.split('/').length - 1;
    return depth === 0 ? '' : '../'.repeat(depth);
}

function build() {
    for (const page of PAGES) {
        const srcPath = path.join(DOCS_DIR, page.src);
        const outPath = path.join(DOCS_DIR, page.out);

        if (!fs.existsSync(srcPath)) {
            console.error(`Markdown source missing: ${srcPath}`);
            process.exit(1);
        }

        const md = stripLeadingComment(fs.readFileSync(srcPath, 'utf8'));
        const rootPrefix = rootPrefixFor(page.out);

        currentSrcDir = path.posix.dirname(page.src);
        currentRootPrefix = rootPrefix;
        const body = marked.parse(md);

        const html = TEMPLATE
            .replace(/{{TITLE}}/g, page.title)
            .replace(/{{KICKER}}/g, page.kicker)
            .replace(/{{DESCRIPTION}}/g, page.description)
            .replace(/{{CONTENT}}/g, body)
            .replace(/{{ROOT}}/g, rootPrefix);

        fs.mkdirSync(path.dirname(outPath), { recursive: true });
        fs.writeFileSync(outPath, html);
        console.log(`Built: ${page.out}`);
    }
    console.log('Done. Run scripts/build-layout.js next to fill nav/footer.');
}

build();
