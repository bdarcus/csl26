# CSLN Processor

The core citation and bibliography processing engine for CSLN.

## Djot Citation Syntax

The processor includes a native parser for Djot documents that supports a rich citation syntax.

### Basic Citations

| Syntax | Description | Example (APA) |
|--------|-------------|---------------|
| `[@key]` | Basic parenthetical citation | (Smith, 2023) |
| `[@key1; @key2]` | Multiple citations | (Smith, 2023; Jones, 2022) |
| `[prefix ; @key]` | Global prefix | (see Smith, 2023) |
| `[@key ; suffix]` | Global suffix | (Smith, 2023 for more) |
| `[prefix ; @key ; suffix]` | Both global affixes | (see Smith, 2023 for more) |

**Note on Semicolons**: Global affixes must be separated from cite keys by a semicolon `;`. Without the semicolon, text before/after keys may be parsed as part of the items or ignored depending on context.

### Narrative (Integral) Citations

Narrative citations are integrated into the text flow. For numeric styles, these render as **Author [1]**.

| Syntax | Description | Example |
|--------|-------------|---------|
| `@key` | Standard narrative shorthand | Smith (2023) |
| `[+@key]` | Explicit narrative | Smith (2023) |

### Visibility Modifiers

Modifiers appear immediately before the `@` symbol.

| Modifier | Description | Syntax | Result (Non-Integral) | Result (Integral) |
|----------|-------------|--------|-----------------------|-------------------|
| `-` | Suppress Author | `[-@key]` | (2023) | (2023) |
| `!` | Hidden (Nocite) | `[!@key]` | *bibliography only* | *bibliography only* |

### Locators (Pinpoints)

Locators follow a comma after the citekey.

| Type | Syntax | Result |
|------|--------|--------|
| **Page** | `[@key, 45]` or `[@key, p. 45]` | (Smith, 2023, p. 45) |
| **Chapter** | `[@key, ch. 5]` | (Smith, 2023, ch. 5) |
| **Structured**| `[@key, chapter: 2, page: 10]` | (Smith, 2023, ch. 2, p. 10) |

Supported labels: `p`/`page`, `vol`/`volume`, `ch`/`chapter`, `sec`/`section`, `fig`/`figure`, `note`, `part`, `col`.

### Complex Examples

- **Narrative with locator**: `@smith2023[p. 45]` → Smith (2023, p. 45)
- **Mixed visibility**: `[see ; -@smith2023, p. 45; @jones2022]` → (see 2023, p. 45; Jones, 2022)
- **Global affixes**: `[compare ; @kuhn1962; @watson1953 ; for discussion]` → (compare Kuhn, 1962; Watson & Crick, 1953 for discussion)
