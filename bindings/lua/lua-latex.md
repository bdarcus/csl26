# CSLN LuaLaTeX Integration

The CSLN processor can be used directly inside LuaLaTeX documents via a
LuaJIT FFI binding to the Rust shared library.  No Biber, no `.bbl` file,
and no shell-escape are required — citations and bibliography are rendered in
a single `lualatex` pass.

## Architecture

```
.tex  \cite{key}
  └─► \directlua  ──► csln.lua (LuaJIT FFI)  ──► libcsln_processor (Rust)
                                                         │
                         tex.sprint(rendered) ◄──────────┘
```

1. **Core logic (Rust)** — all citation rules, disambiguation, locale terms,
   and formatting live in the Rust processor.
2. **C FFI layer** — thin `#[no_mangle]` exports in `crates/csln_processor/src/ffi.rs`.
3. **Lua binding** — `bindings/lua/csln.lua` wraps the FFI and provides a
   clean Lua API including a `build_citation_json` helper for the full CSLN
   citation model.
4. **LaTeX package** — `bindings/latex/csln.sty` wires the Lua API into
   standard LaTeX citation commands.

---

## Quick start

### 1 — Build the shared library

```bash
cargo build --package csln_processor --release --features ffi
```

This produces:
- `target/release/libcsln_processor.dylib` (macOS)
- `target/release/libcsln_processor.so` (Linux)
- `target/release/csln_processor.dll` (Windows)

### 2 — Make the library findable

Either set `CSLN_LIB_PATH` to the absolute path of the `.dylib`/`.so`, or
copy / symlink it into your document directory.

### 3 — Compile your document

```bash
lualatex csln-example.tex
```

---

## LaTeX package (`csln.sty`)

```latex
\usepackage[style=apa-7th, bibfile=refs]{csln}        % CSLN YAML bib
\usepackage[style=apa-7th, bibfile=refs.bib]{csln}    % biblatex .bib
```

The bibliography format is selected automatically by file extension:
`.bib` → biblatex parser; anything else → CSLN YAML parser.

### Citation commands

All commands are **style-driven** — punctuation, brackets, and output form
are determined by the style YAML, not by the command name.

| Command | CSLN model |
|---|---|
| `\cite[loc]{key}` | non-integral; optional locator |
| `\cite[pre][suf]{key}` | non-integral; item prefix + suffix |
| `\cites{k1, k2, k3}` | non-integral; comma-separated keys |
| `\textcite[loc]{key}` | integral (author in running text) |
| `\textcites{k1, k2}` | integral; multiple keys |
| `\citestart` / `\citeitem[loc]{key}` / `\citeend` | multi-item with per-item locators |
| `\printcslnbibliography` | full bibliography |
| `\printbibliography` | alias for the above |

Locator labels are inferred from biblatex optional-argument conventions:
`p.` / `pp.` → `page`, `ch.` → `chapter`, `§` → `section`, `vol.` → `volume`, etc.
A bare number defaults to `page`.

### Library / Lua path resolution

| Env var | Effect |
|---|---|
| `CSLN_LIB_PATH` | Absolute path to the shared library |
| `CSLN_LUA_PATH` | Absolute path to `csln.lua` |

If neither is set, the package looks for `csln.lua` alongside `csln.sty`,
then falls back to `kpse` resolution.

---

## Lua API (`csln.lua`)

```lua
local csln = require("csln")

-- File-based constructors (preferred for LaTeX use)
local proc = csln.from_yaml("/path/to/style.yaml", "/path/to/refs.yaml")
local proc = csln.from_bib("/path/to/style.yaml", "/path/to/refs.bib")

-- In-memory JSON constructor (lower-level)
local proc = csln.new(style_json, bib_json)

-- Render a single citation (bare key shorthand)
local s = proc:render_citation("kuhn1962")

-- Render with the full citation model
local s = proc:render_citation({
    mode            = "integral",          -- or "non-integral" (default)
    suppress_author = false,
    prefix          = "e.g.,",
    suffix          = "passim",
    items = {
        { id = "kuhn1962",   label = "page",    locator = "52" },
        { id = "lecun2015",  label = "chapter", locator = "3",
          prefix = "see also" },
    },
})

-- Render bibliography
local bbl = proc:render_bibliography()

-- HTML / plain text variants
local s   = proc:render_citation_html(opts)
local s   = proc:render_citation_plain(opts)
local bbl = proc:render_bibliography_html()
local bbl = proc:render_bibliography_plain()

proc:free()   -- optional; GC finalizer handles this automatically
```

---

## Example document

See `bindings/latex/csln-example.tex` for a full document that demonstrates:
- Integral citations with `\textcite`
- Non-integral citations with and without locators
- Multi-key citations with `\cites`
- Per-item locators with `\citestart` / `\citeitem` / `\citeend`
- Bibliography rendering with `\printcslnbibliography`

---

## Comparison with `citeproc-lua`

While `citeproc-lua` is a pure-Lua implementation of CSL 1.0, CSLN offers:

- **Declarative YAML styles** — easier to author and read than CSL 1.0 XML
- **Native EDTF date support** — covers intervals, approximate dates, and seasons
- **Multilingual data model** — structured per-language title and contributor data
- **Performance** — disambiguation, sorting, and template rendering in Rust
