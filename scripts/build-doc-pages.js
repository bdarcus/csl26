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
];

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

marked.setOptions({ renderer });

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

        const md = fs.readFileSync(srcPath, 'utf8');
        const body = marked.parse(md);
        const rootPrefix = rootPrefixFor(page.out);

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
