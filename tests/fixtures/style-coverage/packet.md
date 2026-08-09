# Style Coverage Audit: coverage-fixture-v1

- **Schema:** `citum.style-coverage-packet/v1`
- **Style:** `coverage-fixture`
- **Source revision:** `fixture`
- **Baseline eligible:** no
- **Coverage evidence:** inferred structural coverage from a Citum-resolved style

Structural coverage identifies a resolved component path; it does not prove that a conditional component consumed the value at runtime.

## Coverage summary

| Render disposition | Relevant observations |
|---|---:|
| rendered | 2 |
| fallback | 2 |
| suppressed | 1 |
| uncovered | 1 |

- Populated observations: **8**
- Relevant observations: **6**
- Excluded observations: **2**

## Joined exact parity

- Passed: **1/2**
- Not comparable: **1**

## Complete observation index

| Row | Observation ID | Relevance | Render disposition | Comparison | Exact match |
|---:|---|---|---|---|---|
| 1 | `coverage-fixture/bibliography/minimal-references/ITEM-1/book/license/entry` | excluded | — | comparable | false |
| 2 | `coverage-fixture/bibliography/minimal-references/ITEM-1/book/publisher/entry` | relevant | rendered | comparable | false |
| 3 | `coverage-fixture/bibliography/minimal-references/ITEM-1/book/title/entry` | relevant | rendered | comparable | false |
| 4 | `coverage-fixture/bibliography/minimal-references/ITEM-2/article-journal/issue/entry` | relevant | uncovered | not-comparable | — |
| 5 | `coverage-fixture/bibliography/minimal-references/ITEM-2/article-journal/title/entry` | relevant | fallback | not-comparable | — |
| 6 | `coverage-fixture/citation/minimal-citations/ITEM-1/book/license/cite-1%3A1` | excluded | — | comparable | true |
| 7 | `coverage-fixture/citation/minimal-citations/ITEM-1/book/publisher/cite-1%3A1` | relevant | suppressed | comparable | true |
| 8 | `coverage-fixture/citation/minimal-citations/ITEM-1/book/title/cite-1%3A1` | relevant | fallback | comparable | true |
