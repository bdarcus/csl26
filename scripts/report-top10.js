#!/usr/bin/env node
/**
 * Top-10 Style Compatibility Report Generator
 *
 * Generates a JSON report of compatibility metrics for Tier 1 top-10 styles
 * and optionally produces an HTML dashboard.
 *
 * Usage:
 *   node report-top10.js                                    # Output JSON to stdout
 *   node report-top10.js --write-html                       # Write HTML to docs/compat.html
 *   node report-top10.js --output-html /path/to/output.html # Write HTML to custom path
 *   node report-top10.js --styles-dir /path/to/csl           # Override CSL directory
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const TOP_10_STYLES = [
  { name: 'apa', dependents: 783, format: 'author-date' },
  { name: 'elsevier-with-titles', dependents: 672, format: 'numeric' },
  { name: 'elsevier-harvard', dependents: 665, format: 'author-date' },
  { name: 'elsevier-vancouver', dependents: 502, format: 'numeric' },
  { name: 'springer-vancouver-brackets', dependents: 472, format: 'numeric' },
  { name: 'springer-basic-author-date', dependents: 460, format: 'author-date' },
  { name: 'springer-basic-brackets', dependents: 352, format: 'numeric' },
  { name: 'springer-socpsych-author-date', dependents: 317, format: 'author-date' },
  { name: 'american-medical-association', dependents: 293, format: 'numeric' },
  { name: 'taylor-and-francis-chicago-author-date', dependents: 234, format: 'author-date' },
];

const TOTAL_DEPENDENTS = 7987;

/**
 * Parse command-line arguments
 */
function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    writeHtml: false,
    outputHtml: null,
    stylesDir: null,
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--write-html') {
      options.writeHtml = true;
    } else if (args[i] === '--output-html') {
      options.outputHtml = args[++i];
      options.writeHtml = true;
    } else if (args[i] === '--styles-dir') {
      options.stylesDir = args[++i];
    }
  }

  return options;
}

/**
 * Get git short commit hash or 'unknown' on error
 */
function getGitCommit() {
  try {
    return execSync('git rev-parse --short HEAD', {
      cwd: path.dirname(__dirname),
      encoding: 'utf8',
    }).trim();
  } catch {
    return 'unknown';
  }
}

/**
 * Get ISO timestamp
 */
function getTimestamp() {
  return new Date().toISOString();
}

/**
 * Find styles directory
 */
function getStylesDir(optionsDir) {
  if (optionsDir) return optionsDir;

  const projectRoot = path.dirname(__dirname);
  const defaultDir = path.join(projectRoot, 'styles-legacy');

  if (fs.existsSync(defaultDir)) {
    return defaultDir;
  }

  throw new Error(`Styles directory not found. Use --styles-dir to specify path.`);
}

/**
 * Run oracle.js for a single style and parse output
 */
function runOracle(stylePath, styleName) {
  try {
    const result = execSync(`node "${path.join(__dirname, 'oracle.js')}" "${stylePath}" --json`, {
      encoding: 'utf8',
      timeout: 120000,
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    return JSON.parse(result);
  } catch (error) {
    // Try to parse JSON from stderr or use error message
    if (error.stdout) {
      try {
        return JSON.parse(error.stdout);
      } catch {
        return { error: `Oracle execution failed: ${error.message}`, style: styleName };
      }
    }
    return { error: `Oracle execution failed: ${error.message}`, style: styleName };
  }
}

/**
 * Compute fidelity score from oracle result
 */
function computeFidelityScore(oracleResult) {
  if (oracleResult.error) {
    return 0;
  }

  const citations = oracleResult.citations || {};
  const bibliography = oracleResult.bibliography || {};

  const citationsPassed = citations.passed || 0;
  const citationsTotal = citations.total || 1;
  const biblioPassed = bibliography.passed || 0;
  const biblioTotal = bibliography.total || 1;

  const totalPassed = citationsPassed + biblioPassed;
  const totalTests = citationsTotal + biblioTotal;

  return totalTests > 0 ? Math.min(1, totalPassed / totalTests) : 0;
}

/**
 * Load known divergences
 */
function loadDivergences() {
  try {
    const divergencePath = path.join(__dirname, 'report-data', 'known-divergences.json');
    const content = fs.readFileSync(divergencePath, 'utf8');
    return JSON.parse(content);
  } catch {
    return {};
  }
}

/**
 * Generate compatibility report
 */
function generateReport(options) {
  const stylesDir = getStylesDir(options.stylesDir);
  const divergences = loadDivergences();

  const styles = [];
  let citationsTotal = 0;
  let citationsPassed = 0;
  let biblioTotal = 0;
  let biblioPassed = 0;

  for (const styleSpec of TOP_10_STYLES) {
    const stylePath = path.join(stylesDir, `${styleSpec.name}.csl`);

    if (!fs.existsSync(stylePath)) {
      styles.push({
        name: styleSpec.name,
        dependents: styleSpec.dependents,
        format: styleSpec.format,
        impactPct: (styleSpec.dependents / TOTAL_DEPENDENTS * 100).toFixed(2),
        fidelityScore: 0,
        citations: { passed: 0, total: 0 },
        bibliography: { passed: 0, total: 0 },
        knownDivergences: divergences[styleSpec.name] || [],
        error: `Style file not found: ${stylePath}`,
        oracleDetail: null,
      });
      continue;
    }

    const oracleResult = runOracle(stylePath, styleSpec.name);
    const fidelityScore = computeFidelityScore(oracleResult);

    const citations = oracleResult.citations || { passed: 0, total: 0 };
    const bibliography = oracleResult.bibliography || { passed: 0, total: 0 };

    citationsTotal += citations.total || 0;
    citationsPassed += citations.passed || 0;
    biblioTotal += bibliography.total || 0;
    biblioPassed += bibliography.passed || 0;

    styles.push({
      name: styleSpec.name,
      dependents: styleSpec.dependents,
      format: styleSpec.format,
      impactPct: (styleSpec.dependents / TOTAL_DEPENDENTS * 100).toFixed(2),
      fidelityScore: parseFloat(fidelityScore.toFixed(3)),
      citations,
      bibliography,
      knownDivergences: divergences[styleSpec.name] || [],
      error: oracleResult.error || null,
      oracleDetail: oracleResult.bibliography ? oracleResult.bibliography.entries : null,
    });
  }

  const totalImpact = (TOP_10_STYLES.reduce((sum, s) => sum + s.dependents, 0) / TOTAL_DEPENDENTS * 100).toFixed(2);

  return {
    generated: getTimestamp(),
    commit: getGitCommit(),
    totalImpact: parseFloat(totalImpact),
    citationsOverall: { passed: citationsPassed, total: citationsTotal },
    bibliographyOverall: { passed: biblioPassed, total: biblioTotal },
    styles,
  };
}

/**
 * Generate HTML dashboard
 */
function generateHtml(report) {
  const headerHtml = generateHtmlHeader(report);
  const statsHtml = generateHtmlStats(report);
  const tableHtml = generateHtmlTable(report);
  const footerHtml = generateHtmlFooter();

  return `${headerHtml}${statsHtml}${tableHtml}${footerHtml}`;
}

function generateHtmlHeader(report) {
  const generatedDate = new Date(report.generated).toUTCString();
  return `<!-- Auto-generated by report-top10.js. Do not edit manually. -->
<!DOCTYPE html>
<html lang="en" class="scroll-smooth">

<head>
    <meta charset="utf-8" />
    <meta content="width=device-width, initial-scale=1.0" name="viewport" />
    <title>CSLN | Style Compatibility Report</title>
    <meta name="description"
        content="Compatibility metrics for CSLN against citeproc-js reference implementation.">

    <script src="https://cdn.tailwindcss.com?plugins=forms,container-queries,typography"></script>
    <link
        href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap"
        rel="stylesheet" />
    <link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet" />

    <script>
        tailwind.config = {
            darkMode: "class",
            theme: {
                extend: {
                    colors: {
                        "primary": "#2a94d6",
                        "background-light": "#fdfbf7",
                        "accent-cream": "#f5f2eb",
                    },
                    fontFamily: {
                        "display": ["Inter", "sans-serif"],
                        "mono": ["JetBrains Mono", "monospace"]
                    },
                    borderRadius: {
                        "DEFAULT": "0.25rem",
                        "lg": "0.5rem",
                        "xl": "0.75rem",
                        "full": "9999px"
                    },
                },
            },
        }
    </script>
    <style type="text/tailwindcss">
        body {
            font-family: 'Inter', sans-serif;
            color: #374151;
        }
        .font-mono {
            font-family: 'JetBrains Mono', monospace;
        }
        .glass-nav {
            background: rgba(253, 251, 247, 0.85);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid rgba(42, 148, 214, 0.1);
        }
        .accordion-toggle {
            cursor: pointer;
            user-select: none;
        }
        .accordion-content {
            display: none;
        }
        .accordion-content.active {
            display: table-row;
        }
        .badge-perfect {
            background-color: rgba(16, 185, 129, 0.1);
            color: #047857;
        }
        .badge-partial {
            background-color: rgba(251, 191, 36, 0.1);
            color: #92400e;
        }
        .badge-failing {
            background-color: rgba(239, 68, 68, 0.1);
            color: #7f1d1d;
        }
        .badge-pending {
            background-color: rgba(148, 163, 184, 0.1);
            color: #475569;
        }
    </style>
</head>

<body class="bg-background-light text-slate-700 selection:bg-primary/20">

    <!-- Navigation -->
    <nav class="fixed top-0 w-full z-50 glass-nav">
        <div class="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
            <div class="flex items-center gap-2">
                <div class="w-8 h-8 bg-primary rounded flex items-center justify-center">
                    <span class="text-white font-mono font-bold">C</span>
                </div>
                <span class="font-mono text-xl font-bold tracking-tight text-slate-900">CSLN</span>
            </div>
            <div class="hidden md:flex items-center gap-8">
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="index.html#features">Features</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="index.html#roadmap">Roadmap</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="interactive-demo.html">Demo</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="examples.html">Examples</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="compat.html" style="color: #2a94d6; font-weight: 600;">Compat</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="index.html#schemas">Schemas</a>
                <a class="text-sm font-medium hover:text-primary transition-colors text-slate-600"
                    href="https://github.com/bdarcus/csl26">GitHub</a>
            </div>
        </div>
    </nav>

    <!-- Header Section -->
    <header class="pt-24 pb-12 px-6 border-b border-slate-200">
        <div class="max-w-7xl mx-auto">
            <div class="flex items-center justify-between mb-6">
                <div>
                    <h1 class="text-4xl md:text-5xl font-mono font-bold tracking-tight text-slate-900 mb-2">
                        Style Compatibility Report
                    </h1>
                    <p class="text-slate-500">Compatibility metrics for Tier 1 top-10 parent styles</p>
                </div>
            </div>
            <div class="flex flex-col sm:flex-row gap-4 items-start sm:items-center">
                <div class="text-sm text-slate-500 font-mono">Generated: ${generatedDate}</div>
                <div class="inline-flex items-center gap-2 px-3 py-1 rounded bg-slate-100 text-slate-700 text-xs font-mono border border-slate-200">
                    <span class="material-icons text-sm">code</span>
                    <span>${escapeHtml(report.commit)}</span>
                </div>
            </div>
        </div>
    </header>
`;
}

function generateHtmlStats(report) {
  const citationsPct = report.citationsOverall.total > 0
    ? ((report.citationsOverall.passed / report.citationsOverall.total) * 100).toFixed(1)
    : 0;
  const biblioPct = report.bibliographyOverall.total > 0
    ? ((report.bibliographyOverall.passed / report.bibliographyOverall.total) * 100).toFixed(1)
    : 0;

  return `
    <!-- Statistics Cards -->
    <section class="py-12 px-6 bg-accent-cream">
        <div class="max-w-7xl mx-auto">
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                <!-- Coverage Impact -->
                <div class="bg-white rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Coverage Impact</div>
                    <div class="text-3xl font-bold text-slate-900">${report.totalImpact}%</div>
                    <div class="text-xs text-slate-400 mt-2">of dependent styles</div>
                </div>

                <!-- Citations Overall -->
                <div class="bg-white rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Citations</div>
                    <div class="text-3xl font-bold text-slate-900">${report.citationsOverall.passed}/${report.citationsOverall.total}</div>
                    <div class="text-xs text-slate-400 mt-2">${citationsPct}% pass rate</div>
                </div>

                <!-- Bibliography Overall -->
                <div class="bg-white rounded-xl border border-slate-200 p-6">
                    <div class="text-sm font-medium text-slate-500 mb-2">Bibliography</div>
                    <div class="text-3xl font-bold text-slate-900">${report.bibliographyOverall.passed}/${report.bibliographyOverall.total}</div>
                    <div class="text-xs text-slate-400 mt-2">${biblioPct}% pass rate</div>
                </div>
            </div>
        </div>
    </section>
`;
}

function generateHtmlTable(report) {
  let tableRows = '';

  for (const style of report.styles) {
    const citationsPct = style.citations.total > 0
      ? ((style.citations.passed / style.citations.total) * 100).toFixed(0)
      : 'N/A';
    const biblioPct = style.bibliography.total > 0
      ? ((style.bibliography.passed / style.bibliography.total) * 100).toFixed(0)
      : 'N/A';

    const fidelityPct = (style.fidelityScore * 100).toFixed(1);

    let statusBadge = 'badge-pending';
    let statusText = 'Pending';

    if (style.error) {
      statusText = 'Error';
      statusBadge = 'badge-pending';
    } else if (style.fidelityScore === 1) {
      statusText = 'Perfect';
      statusBadge = 'badge-perfect';
    } else if (style.fidelityScore > 0) {
      statusText = 'Partial';
      statusBadge = 'badge-partial';
    } else {
      statusText = 'Failing';
      statusBadge = 'badge-failing';
    }

    const citationBadge = style.citations.passed === style.citations.total && style.citations.total > 0
      ? 'badge-perfect'
      : style.citations.passed > 0
        ? 'badge-partial'
        : 'badge-failing';

    const biblioBadge = style.bibliography.passed === style.bibliography.total && style.bibliography.total > 0
      ? 'badge-perfect'
      : style.bibliography.passed > 0
        ? 'badge-partial'
        : 'badge-failing';

    const toggleId = `toggle-${style.name}`;
    const contentId = `content-${style.name}`;

    tableRows += `
                <tr class="border-b border-slate-200 hover:bg-slate-50 accordion-toggle" data-toggle="${toggleId}">
                    <td class="px-6 py-4 text-sm font-medium text-slate-900">${style.name}</td>
                    <td class="px-6 py-4 text-sm text-slate-600">${style.format}</td>
                    <td class="px-6 py-4 text-sm text-slate-600">${style.dependents}</td>
                    <td class="px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${citationBadge}">
                            ${style.citations.passed}/${style.citations.total}
                        </span>
                    </td>
                    <td class="px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${biblioBadge}">
                            ${style.bibliography.passed}/${style.bibliography.total}
                        </span>
                    </td>
                    <td class="px-6 py-4 text-sm font-mono text-slate-600">${fidelityPct}%</td>
                    <td class="px-6 py-4">
                        <span class="inline-flex items-center px-3 py-1 rounded text-xs font-medium ${statusBadge}">
                            ${statusText}
                        </span>
                    </td>
                    <td class="px-6 py-4 text-right">
                        <button class="text-slate-500 hover:text-primary text-xs font-medium transition-colors" onclick="toggleAccordion('${contentId}')">
                            <span class="material-icons text-base align-middle">expand_more</span>
                        </button>
                    </td>
                </tr>
                <tr class="accordion-content" id="${contentId}">
                    <td colspan="8" class="px-6 py-4 bg-slate-50">
                        <div class="max-w-4xl">
${generateDetailContent(style)}
                        </div>
                    </td>
                </tr>
    `;
  }

  return `
    <!-- Compatibility Table -->
    <section class="py-12 px-6">
        <div class="max-w-7xl mx-auto">
            <div class="rounded-xl border border-slate-200 overflow-hidden">
                <table class="w-full">
                    <thead class="bg-slate-50 border-b border-slate-200">
                        <tr>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">Style</th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">Format</th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">Dependents</th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">Citations</th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">Bibliography</th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">Fidelity</th>
                            <th class="text-left px-6 py-4 text-xs font-semibold text-slate-700">Status</th>
                            <th class="px-6 py-4"></th>
                        </tr>
                    </thead>
                    <tbody>
${tableRows}
                    </tbody>
                </table>
            </div>
        </div>
    </section>
`;
}

function generateDetailContent(style) {
  let html = '';

  if (style.error) {
    html += `
                            <div class="p-4 rounded-lg bg-red-50 border border-red-200 mb-4">
                                <div class="text-sm font-medium text-red-700 mb-1">Error</div>
                                <div class="text-xs text-red-600 font-mono">${escapeHtml(style.error)}</div>
                            </div>
`;
  }

  if (style.knownDivergences && style.knownDivergences.length > 0) {
    html += `
                            <div class="p-4 rounded-lg bg-primary/5 border border-primary/20 mb-4">
                                <div class="text-sm font-semibold text-primary mb-2">CSLN Extensions</div>
`;
    for (const divergence of style.knownDivergences) {
      html += `
                                <div class="text-xs text-slate-700 mb-2">
                                    <strong>${escapeHtml(divergence.feature)}:</strong> ${escapeHtml(divergence.description)}
                                </div>
`;
    }
    html += `
                            </div>
`;
  }

  if (style.oracleDetail && style.oracleDetail.length > 0) {
    html += `
                            <div class="mt-4">
                                <div class="text-xs font-semibold text-slate-900 mb-2">Bibliography Entries (${style.oracleDetail.length})</div>
                                <div class="overflow-x-auto">
                                    <table class="w-full text-xs border-collapse">
                                        <thead>
                                            <tr class="border-b border-slate-300 bg-slate-100">
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">#</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">Oracle</th>
                                                <th class="text-left px-2 py-1 font-medium text-slate-700">CSLN</th>
                                                <th class="text-center px-2 py-1 font-medium text-slate-700">Match</th>
                                            </tr>
                                        </thead>
                                        <tbody>
`;

    for (let i = 0; i < style.oracleDetail.length; i++) {
      const entry = style.oracleDetail[i];
      const matchIcon = entry.match === true ? '✓' : entry.match === false ? '✗' : '–';
      const matchColor = entry.match === true ? 'text-emerald-600' : entry.match === false ? 'text-red-600' : 'text-slate-400';

      const oracleText = entry.oracle ? entry.oracle.substring(0, 80) : '(empty)';
      const cslnText = entry.csln ? entry.csln.substring(0, 80) : '(empty)';

      html += `
                                            <tr class="border-b border-slate-200 hover:bg-slate-50">
                                                <td class="px-2 py-1 text-slate-600">${i + 1}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600" title="${escapeHtml(entry.oracle || '')}">${escapeHtml(oracleText)}</td>
                                                <td class="px-2 py-1 font-mono text-slate-600" title="${escapeHtml(entry.csln || '')}">${escapeHtml(cslnText)}</td>
                                                <td class="px-2 py-1 text-center font-bold ${matchColor}">${matchIcon}</td>
                                            </tr>
`;
    }

    html += `
                                        </tbody>
                                    </table>
                                </div>
                            </div>
`;
  }

  return html;
}

function generateHtmlFooter() {
  return `

    <!-- Footer -->
    <footer class="py-12 px-6 border-t border-slate-200 bg-white">
        <div class="max-w-7xl mx-auto">
            <div class="flex flex-col md:flex-row justify-between items-center gap-8">
                <div class="flex items-center gap-2">
                    <div class="w-6 h-6 bg-primary rounded flex items-center justify-center">
                        <span class="text-white font-mono text-xs font-bold">C</span>
                    </div>
                    <span class="font-mono text-lg font-bold text-slate-900">CSLN</span>
                </div>
                <div class="flex gap-8 text-sm font-medium text-slate-500">
                    <a class="hover:text-primary transition-colors" href="https://github.com/bdarcus/csl26">GitHub</a>
                    <a class="hover:text-primary transition-colors" href="index.html#roadmap">Roadmap</a>
                    <a class="hover:text-primary transition-colors" href="examples.html">Examples</a>
                </div>
                <div class="text-sm text-slate-400">
                    © 2026 CSLN Project. MIT Licensed.
                </div>
            </div>
        </div>
    </footer>

    <script>
        function toggleAccordion(contentId) {
            const content = document.getElementById(contentId);
            if (content) content.classList.toggle('active');
        }
    </script>

</body>

</html>
`;
}

function escapeHtml(text) {
  if (!text) return '';
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Main entry point
 */
function main() {
  try {
    const options = parseArgs();
    const report = generateReport(options);

    // Output JSON to stdout
    console.log(JSON.stringify(report, null, 2));

    // Generate and write HTML if requested
    if (options.writeHtml) {
      const htmlPath = options.outputHtml || path.join(path.dirname(__dirname), 'docs', 'compat.html');
      const htmlDir = path.dirname(htmlPath);

      if (!fs.existsSync(htmlDir)) {
        fs.mkdirSync(htmlDir, { recursive: true });
      }

      const htmlContent = generateHtml(report);
      fs.writeFileSync(htmlPath, htmlContent, 'utf8');
      process.stderr.write(`HTML report written to: ${htmlPath}\n`);
    }
  } catch (error) {
    process.stderr.write(`Error: ${error.message}\n`);
    process.exit(1);
  }
}

main();
