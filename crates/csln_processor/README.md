# CSLN Processor

The core citation and bibliography processing engine for CSLN.

## Djot Citation Syntax

The processor includes a native parser for Djot documents that supports a rich citation syntax.

### Basic Citations

| Syntax | Description | Example |
|--------|-------------|---------|
| `[@key]` | Basic parenthetical citation | (Smith, 2023) |
| `[@key1; @key2]` | Multiple citations | (Smith, 2023; Jones, 2022) |
| `[prefix @key suffix]` | Global affixes | (see Smith, 2023, for more) |

### Narrative (Integral) Citations

Narrative citations are integrated into the text flow.

| Syntax | Description | Example |
|--------|-------------|---------|
| `@key` | Standard narrative | Smith (2023) |
| `[+@key]` | Narrative within brackets | Smith (2023) |
| `@key(infix)` | Narrative with custom text | Smith argues (2023) |

### Visibility Modifiers

Modifiers appear immediately before the `@` symbol.

| Modifier | Description | Syntax | Result |
|----------|-------------|--------|--------|
| `-` | Suppress Author | `[-@key]` | (2023) |
| `+` | Author Only | `[+@key]` | Smith |
| `!` | Hidden (Nocite) | `[!@key]` | *rendered only in bibliography* |

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
- **Mixed visibility**: `[see -@smith2023, p. 45; @jones2022]` → (see 2023, p. 45; Jones, 2022)
- **Integral narrative**: `As [+@kuhn1962] showed...` → As Kuhn (1962) showed...
