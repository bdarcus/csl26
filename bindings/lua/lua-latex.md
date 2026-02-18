# CSLN LuaLaTeX Integration

This directory contains a proof-of-concept LuaJIT FFI binding for the CSLN (Citation Style Language Next) processor.

## Architecture

The integration follows a "Hybrid Rust-Lua" model:
1.  **Core Logic (Rust)**: All complex citation rules, disambiguation, and formatting are handled by the high-performance Rust core.
2.  **FFI Layer (C)**: A thin C-compatible interface exports the core functions.
3.  **Lua Binding**: A LuaJIT FFI script loads the shared library and provides a clean Lua API.

## Benefits

*   **Performance**: Rendering thousands of citations is nearly instantaneous, avoiding the performance bottlenecks of pure-Lua CSL implementations.
*   **Memory Efficiency**: The Rust processor manages its own state, and Lua only interacts with it through small JSON fragments.
*   **Native LaTeX Support**: The processor includes a dedicated LaTeX renderer that handles escaping and formatting commands (`\textit`, `\textsc`, etc.) natively.

## How to Integrate

1.  **Build the Shared Library**:
    ```bash
    cargo build --package csln_processor --release --features ffi
    ```
    This produces `target/release/libcsln_processor.dylib` (macOS), `.so` (Linux), or `.dll` (Windows). Note the `--features ffi` flag is required to enable the C-FFI exports.

2.  **Usage in Lua**:
    ```lua
    local csln = require("csln")
    
    -- Initialize with style and bibliography JSON
    local proc = csln.new(style_json, bib_json)
    
    -- Render a citation
    local output = proc:render_citation([[{"items": [{"id": "kuhn1962"}]} ]])
    print(output) -- Output: (Kuhn, 1962)
    
    -- Cleanup
    proc:free()
    ```

3.  **Library Resolution**:
    `bindings/lua/csln.lua` now resolves the shared library in this order:
    - `CSLN_LIB_PATH` (if set)
    - `target/release/<platform library>`
    - `target/debug/<platform library>`
    - system loader path (bare library name)

    This is Linux-first by default (`libcsln_processor.so` on Linux, `.dylib` on macOS, `.dll` on Windows).

4.  **Memory Management**:
    The binding attaches a LuaJIT `ffi.gc` finalizer to the native processor pointer, so resources are reclaimed even if `:free()` is not called. Calling `:free()` is still recommended for deterministic cleanup.

## Comparison with `citeproc-lua`

While `citeproc-lua` is a faithful implementation of CSL 1.0 in pure Lua, CSLN offers:
*   **Declarative Templates**: YAML-based styles that are easier to author and maintain than CSL 1.0 XML.
*   **Type Safety**: Leveraging Rust's type system to prevent common citation rendering bugs.
*   **Modern Features**: Native support for EDTF dates, multilingual data, and advanced legal citation tiers.

## A Note for BibLaTeX Developers

While `biblatex` (paired with `biber`) remains the gold standard for citation management in the LaTeX ecosystem, CSLN offers a complementary approach that may be of interest to `biblatex` developers and power users:

*   **Modern Data Model**: CSLN's internal model is built on EDTF and structured multilingual data, potentially offering a more robust foundation for non-Western or complex academic metadata.
*   **Performance**: By moving the heavy lifting of sorting, disambiguation, and template application from TeX macros (or Perl) to Rust, we can achieve significant speed improvements, especially in LuaLaTeX where the integration is seamless.
*   **Portability**: CSLN styles are declarative YAML files, making them portable across different document formats (HTML, Typst, LaTeX) without rewriting the logic.

We would love to hear from the `biblatex` community about how CSLN might serve as a high-performance rendering backend or data-processing component.

## Feedback Wanted

We are particularly interested in:
*   The viability of distributing the shared library for LuaLaTeX users.
*   The granularity of the FFI API (stateless vs stateful).
*   Any specific LaTeX-specific formatting requirements that the current `Latex` renderer might be missing.
