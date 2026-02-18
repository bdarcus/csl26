# Style Update Checklist: [Style Name]

## Mode: [ ] Language | [ ] Output | [ ] Full

## Language Modernization
- [ ] Replace prefix/suffix pairs with `wrap:` (parentheses, brackets, quotes)
- [ ] Replace implicit spacing with `delimiter:`
- [ ] Move repeated overrides to context-level options (tier-2)
- [ ] Consolidate type-specific overrides where behavior is shared
- [ ] Add/improve inline comments explaining non-obvious logic
- [ ] Align with `common-patterns.yaml` snippets

## Output Coverage
- [ ] Journal article
- [ ] Book
- [ ] Edited chapter
- [ ] Webpage
- [ ] Report
- [ ] Conference paper
- [ ] Thesis/dissertation
- [ ] Legal citation (if applicable)

## Edge Cases
- [ ] 1 author, 2 authors, 3+ authors (shortening)
- [ ] No date / n.d.
- [ ] Multiple works same author-year (disambiguation)
- [ ] Titles with/without subtitles
- [ ] DOI vs URL fallback logic
- [ ] Locators in citations

## Testing
- [ ] `cargo run --bin csln -- render refs -b examples/comprehensive.yaml -s <style-path>`
- [ ] Verify output matches target reference examples (documentation or oracle)
- [ ] Oracle comparison (only if a legacy CSL counterpart exists)
- [ ] Full regression test (`cargo test`)

## Notes
[Document any decisions, tradeoffs, or deferred work]
