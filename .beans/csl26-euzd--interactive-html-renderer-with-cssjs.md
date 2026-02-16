---
# csl26-euzd
title: Interactive HTML Renderer with CSS/JS
status: todo
type: feature
priority: normal
created_at: 2026-02-16T03:03:36Z
updated_at: 2026-02-16T03:03:36Z
blocking:
    - csl26-li63
---

## Overview

Provide optional CSS and JavaScript libraries to enhance HTML output with interactive features similar to the Typst PDF renderer. Users can optionally include these assets to enable rich citation/bibliography interactivity in web documents.

**Design Philosophy:** Progressive enhancement - the HTML works perfectly without CSS/JS, but becomes interactive when the libraries are included.

**Status:** HTML OutputFormat already exists with semantic markup (`csln-bibliography`, `csln-entry`, `id` attributes). This bean adds optional styling and interactivity on top of that foundation.

## Interactive Features

### Core Functionality (JavaScript)
* **Clickable Citations:** Smooth scroll from in-text citations to bibliography entries
* **Bidirectional Navigation:** Click bibliography entry to reveal/highlight all citation locations
* **Hover Tooltips:** Show reference metadata preview on citation hover (author, title, year, DOI)
* **Visual Highlighting:** Highlight citation ↔ bibliography connections on hover/click
* **Scroll-to-Citation:** From bibliography entry, cycle through citation locations in document
* **Copy Citation:** Right-click citation to copy formatted text to clipboard

### Styling (CSS)
* **Clean Typography:** Modern, readable styles for citations and bibliography
* **Responsive Design:** Mobile-friendly layout with touch-friendly targets
* **Visual Hierarchy:** Clear distinction between citation and bibliography sections
* **Accessibility:** WCAG 2.1 AA compliant (focus indicators, color contrast, keyboard navigation)
* **Customizable:** CSS variables for easy theming (colors, fonts, spacing)

## Architecture

```
HTML Document
    ↓
CSLN HTML Renderer (semantic markup)
    ↓
HTML with class/id attributes
    ↓
Optional Enhancement:
    • csln-interactive.css (styling)
    • csln-interactive.js (behavior)
```

### HTML Structure (Already Implemented)

```html
<p>According to <span class="csln-citation" data-ref="smith2020">Smith (2020)</span>...</p>

<div class="csln-bibliography">
  <div class="csln-entry" id="ref-smith2020">
    <span class="csln-author">Smith, J.</span>
    <span class="csln-date">(2020)</span>
    <span class="csln-title">Title here</span>
    <a href="https://doi.org/...">DOI</a>
  </div>
</div>
```

### Enhancement Requirements

1. **Update HTML renderer** to add `data-ref` attributes to citation spans
2. **Create CSS library** (`csln-interactive.css`) with theming variables
3. **Create JS library** (`csln-interactive.js`) with interactive behaviors
4. **Bundle assets** in `assets/html/` directory
5. **Document usage** with CDN links and self-hosted options

## Implementation Steps

### Phase 1: Foundation (Weeks 1-2)
1. Add `data-ref` attribute to citation rendering in `html.rs`
2. Add `data-cites` attribute to bibliography entries (reverse lookup)
3. Update `entry()` to store metadata as `data-*` attributes (for tooltips)

### Phase 2: CSS (Week 3)
4. Create `assets/html/csln-interactive.css`:
   - Base styles for citations and bibliography
   - Hover/focus states with smooth transitions
   - CSS custom properties for theming
   - Responsive layout queries
   - High contrast mode support
5. Create light/dark theme variants

### Phase 3: JavaScript (Weeks 4-5)
6. Create `assets/html/csln-interactive.js`:
   - Citation click → smooth scroll to bibliography
   - Hover tooltip with reference metadata
   - Highlight citation ↔ entry connections
   - Bidirectional navigation
   - Keyboard navigation support (Tab, Enter, Esc)
   - Copy citation functionality
7. Add accessibility features (ARIA attributes, screen reader support)

### Phase 4: Documentation & Testing (Week 6)
8. Write usage guide (`docs/HTML_INTERACTIVITY.md`)
9. Create demo HTML page with examples
10. Test across browsers (Chrome, Firefox, Safari, Edge)
11. Test accessibility (NVDA, VoiceOver, keyboard-only navigation)
12. Add unit tests for JS functions

## File Structure

```
assets/
  html/
    csln-interactive.css           # Main CSS (light theme)
    csln-interactive.dark.css      # Dark theme variant
    csln-interactive.min.css       # Minified production version
    csln-interactive.js            # Main JavaScript
    csln-interactive.min.js        # Minified production version
    demo.html                      # Interactive demo page

docs/
  HTML_INTERACTIVITY.md           # Usage guide
```

## Usage Example

```html
<!DOCTYPE html>
<html>
<head>
  <!-- Optional: Include CSLN interactive enhancements -->
  <link rel="stylesheet" href="csln-interactive.css">
  <script src="csln-interactive.js" defer></script>
</head>
<body>
  <!-- CSLN-generated HTML with semantic markup -->
  <article>
    <p>Citation here <span class="csln-citation" data-ref="doe2023">...</span></p>
  </article>

  <div class="csln-bibliography">
    <div class="csln-entry" id="ref-doe2023">...</div>
  </div>
</body>
</html>
```

## CSS Customization

```css
:root {
  --csln-citation-color: #0066cc;
  --csln-citation-hover-bg: #f0f8ff;
  --csln-entry-highlight-bg: #ffffcc;
  --csln-font-family: system-ui, sans-serif;
  --csln-border-radius: 4px;
}
```

## JavaScript API (Optional)

For advanced users who want programmatic control:

```javascript
// Initialize with custom options
CSLN.init({
  smoothScroll: true,
  tooltipDelay: 300,
  highlightDuration: 2000,
  theme: 'auto' // 'light', 'dark', or 'auto'
});

// Manual control
CSLN.scrollToCitation('ref-doe2023');
CSLN.highlightEntry('ref-doe2023');
```

## Browser Support

* **Modern browsers:** Chrome/Edge 90+, Firefox 88+, Safari 14+
* **Fallback:** Graceful degradation - links work without JS
* **Bundle size:** ~8KB CSS + ~15KB JS (minified + gzipped)

## Dependencies

* **HTML OutputFormat** (already exists in `crates/csln_processor/src/render/html.rs`)
* No external JavaScript dependencies (vanilla JS)
* CSS uses modern standards (Grid, Custom Properties, CSS Variables)

## Success Criteria

* Citation click → smooth scroll to bibliography entry
* Hover citation → show tooltip with metadata
* Bibliography entry click → highlight all citations
* Keyboard navigation works (Tab, Enter, Esc)
* WCAG 2.1 AA compliance verified with axe DevTools
* Works in all major browsers (Chrome, Firefox, Safari, Edge)
* Demo page showcases all features with APA 7th style
* Documentation with CDN and self-hosted setup

## Alignment

* Companion to csl26-93yh (Typst interactive PDF)
* Issue #105 (pluggable renderers)
* Issue #155 (hyperlink configuration)
* CLAUDE.md goal: hybrid processing (batch + interactive modes)

## Estimated Complexity

Low-Medium (6 weeks):
* HTML renderer updates: straightforward (add `data-*` attributes)
* CSS: moderate (responsive + accessible design)
* JavaScript: moderate (DOM manipulation, smooth scrolling, tooltips)
* Testing: important (cross-browser + accessibility)

## Risks & Mitigations

* **Risk:** Large bundle size impacts page load
  - **Mitigation:** Minify + gzip, lazy load JS, use efficient selectors
* **Risk:** JavaScript conflicts with existing page scripts
  - **Mitigation:** Namespace all functions under `CSLN` object, use event delegation
* **Risk:** Accessibility issues with dynamic tooltips
  - **Mitigation:** Use ARIA live regions, test with screen readers early
* **Risk:** Browser compatibility (older browsers)
  - **Mitigation:** Polyfills for critical features, progressive enhancement

## Future Extensions

* **Citation export:** Copy as BibTeX, RIS, CSL JSON
* **Search/filter:** Filter bibliography by author, year, type
* **Annotations:** User highlights and notes (requires backend)
* **Print optimization:** Hide interactive elements in print media

