# Migration Strategy Analysis: XML Compiler vs Output-Driven

## Context

Bean `csl26-rh2u` and the broader epic `csl26-ifiw` track a fundamental problem: the template compiler produces bibliography templates with wrong component ordering, duplicate/missing components, and incorrect suppress logic. Current results: **87-100% citation match, 0% bibliography match** across all top parent styles. The template compiler (`crates/csln_migrate/src/template_compiler/mod.rs`, 2,077 lines) is the bottleneck.

**Design origin:** CSL 1.0 was designed with XSLT - a side-effect-free language where nodes are processed in document order and macro calls are simple substitutions. This means the XML node order in the layout IS the rendering order. The challenge is not node ordering itself, but that macros like `source` contain `choose/if/else` branches creating different component sequences for different reference types (e.g., journals get container-title + volume(issue) + pages, while chapters get editor + container-title + pages). Flattening these type-specific branches into one declarative template with overrides is the core difficulty.

---

## Approach A: XML Semantic Compiler (Status Quo)

**How it works:** Parse CSL 1.0 XML, inline macros, upsample nodes to an intermediate representation, then compile into CSLN's flat TemplateComponent model. Runs post-processing passes (reorder, deduplicate, group) to fix structural issues.

### Pros

1. **Semantic fidelity** - Works from the actual style definition, which encodes the author's intent across all reference types, not just observed output for tested types.
2. **Complete conditional coverage** - Has access to ALL choose/if/else branches. APA has 126 choose blocks covering 50+ reference types; output-driven only sees what test data exercises.
3. **Options extraction works well** - Global settings (name formatting, et-al rules, initialize-with, date forms, page-range-format) are reliably extracted from XML attributes. This is why citations already work at 87-100%.
4. **Deterministic and scalable** - Same XML input always produces same CSLN output. One compiler handles all 2,844 styles without per-style inference runs.
5. **Provenance tracking** - When something fails, you can trace the exact CslNode to CslnNode to TemplateComponent chain. Debugging infrastructure already exists.
6. **Handles latent features** - Substitute rules, disambiguation, subsequent-author-substitute, locale terms - all encoded in XML regardless of whether test data triggers them.
7. **Significant investment** - 7,300 lines of working code. The options pipeline, upsampler, and preset detector are solid.

### Cons

1. **Fundamental model mismatch** - CSL 1.0 is procedural (macros, choose/if/else, groups with implicit suppression). CSLN is declarative (flat templates with typed overrides). Bridging this is the hardest translation problem in the project.
2. **Type-specific branch flattening is the unsolved problem** - While XML node order correctly reflects rendering order (per the XSLT design), macros like APA's `source` contain 50+ choose/if/else paths that produce different component sequences per reference type. The source_order tracking attempt (reverted in commit `1c9ad45`) correctly preserved node order but could not capture which components appear for which types. The hard problem is not ordering - it is inferring type-specific suppress overrides from deeply nested conditional logic.
3. **Combinatorial explosion** - APA has 99 macros and 126 choose blocks. Flattening these into a flat template with correct suppress overrides for every type is an extremely high-dimensional mapping problem.
4. **Heuristic passes are fragile** - The reorder, deduplicate, and grouping passes use pattern-matching heuristics. Fixing one style's layout frequently breaks another.
5. **Group semantics mismatch** - CSL 1.0 groups suppress their delimiter when a child is empty; CSLN has no equivalent implicit behavior. This creates phantom components and incorrect spacing.
6. **Diminishing returns** - The easy 87% came cheaply; the remaining gap involves the hardest cases where the two models diverge most.

---

## Approach B: Output-Driven / Reverse Engineering

**How it works:** Run citeproc-js with diverse test references, parse rendered output strings into structured components, cross-reference with input data to infer variable-to-output mappings, generate CSLN YAML directly from observed patterns.

### Pros

1. **Directly targets the success criterion** - The oracle comparison IS the definition of correctness. Deriving the template from the output closes the loop: observed output leads to template leads to processor leads to same output.
2. **Bypasses the branch-flattening problem entirely** - citeproc-js already resolves which choose/if/else path to take for each reference type. Component ordering and type-specific behavior are directly observed.
3. **Naturally resolves group semantics** - Group delimiter behavior, implicit suppression, and macro interaction effects are all resolved by citeproc-js before inference begins. No need to replicate that logic.
4. **Type-specific overrides emerge naturally** - Comparing outputs across reference types directly reveals differences: "publisher appears for chapters but not journals" becomes `suppress: true` for `article-journal`.
5. **Human-intuitive** - Produces templates resembling what a style author would write by reading a style guide: "Author (Year). Title. *Journal*, volume(issue), pages."
6. **Well-suited to CSLN's design** - The flat template model was designed to be what a human would write. This approach produces exactly that.
7. **Simpler conceptually** - No need to understand CSL 1.0's macro expansion, choose/if/else flattening, or group suppression.

### Cons

1. **Test data coverage problem** - You only learn about behavior you observe. CSL 1.0 has 50+ reference types; the current fixture has 16 items covering only 7 types (article-journal, book, chapter, report, thesis, paper-conference, webpage). Styles with rare type-specific behavior (legal, patent, dataset) will be missed.
2. **Oracle parser fragility** - The existing oracle.js component parser (`parseComponents()`) uses fragile heuristics: publisher detection relies on 10-character prefix substring matching, container-title on 15-character prefixes, and title on 20-character prefixes. `findRefDataForEntry()` returns the first author match even when multiple references share an author. These must be substantially hardened before serving as a template generation foundation.
3. **Ambiguous parsing** - Regex-based component extraction is inherently fragile. Is "Cambridge" a publisher or a place? Is "15" a volume or a page number? Context-dependent resolution requires complex heuristics.
4. **Loses metadata linkage** - Output strings do not reveal which CSL variable produced which output token. Cross-referencing with input data helps but is not foolproof (e.g., "Smith" could be author or editor).
5. **Cannot extract global options** - Output "Smith, J." does not tell you whether `initialize-with` is `. ` or the input only had initials. Options like name-as-sort-order, et-al, and page-range-format must still come from XML.
6. **Delimiter and formatting inference** - Inferring delimiters between components (". " vs ", " vs ": ") and formatting (italics, bold) from rendered strings is non-trivial. "Nature" in italics looks the same as "Nature" in plain text unless the output format preserves markup.
7. **Does not scale** - Each of 2,844 styles needs its own citeproc-js inference run with sufficient test data. This creates a permanent dependency on citeproc-js as infrastructure.
8. **Non-deterministic** - Different test data sets may produce different inferred templates. The approach is probabilistic, not deterministic.
9. **Cannot discover latent features** - Substitute rules, disambiguation, subsequent-author-substitute only trigger under specific conditions. Test data may never exercise them.
10. **Locale conflation** - Output "pp. 1-10" does not reveal whether "pp." is a locale term or a hardcoded prefix. This matters for CSLN's multilingual locale system.
11. **Compensating errors** - If the CSLN processor has bugs, the output-driven approach produces templates that compensate for those bugs rather than being correct.

---

## Approach C: Hand-Authoring High-Impact Styles

**How it works:** A human (or LLM-assisted human) reads the style guide and hand-authors CSLN YAML templates, using the existing `examples/apa-style.yaml` as a model. This approach targets the top 10 parent styles covering 60% of dependent styles.

### Pros

1. **Proven to work** - `examples/apa-style.yaml` already exists: 11 components, correct ordering, proper type-specific overrides, correct delimiters. This is the gold standard.
2. **Highest fidelity** - A knowledgeable author understands the intent behind a style guide, not just observed output. They can handle edge cases, locale terms, and rare types correctly.
3. **No infrastructure dependency** - No need for oracle.js hardening, expanded test fixtures, or citeproc-js inference runs.
4. **Directly produces the target format** - The CSLN template model was designed to be human-readable and human-writable.

### Cons

1. **Does not scale** - Hand-authoring 300 parent styles is not feasible.
2. **Requires domain expertise** - The author needs to understand both the style guide and the CSLN template model.
3. **Error-prone** - Manual work introduces human error, especially for complex styles with many type-specific overrides.
4. **Still needs verification** - Oracle comparison is still needed to validate correctness.

---

## Architect's Recommendation: Hybrid Approach

**Verdict: Neither approach alone is sufficient. Use a hybrid strategy combining all three.**

The critical insight is that these approaches fail at *different things*:

| Capability | XML Compiler | Output-Driven | Hand-Authored |
|---|---|---|---|
| Global options (names, dates, et-al) | Excellent | Cannot do | Manual |
| Template component ordering | Failed (0% bib) | Good (if parser is hardened) | Excellent |
| Type-specific overrides/suppress | Fragile (heuristic) | Good (observable) | Excellent |
| Coverage of rare types | Complete | Test-data dependent | Domain-expert dependent |
| Scalability to 2,844 styles | One compiler | Per-style inference | Not feasible |
| Locale term handling | Direct | Cannot distinguish | Manual |
| Substitute/disambiguation | Encoded in XML | Requires special test data | Manual |
| Delimiter inference | Direct from XML | Non-trivial | Manual |

### Concrete Architecture

1. **Keep the XML pipeline for OPTIONS** - The options extractor, preset detector, locale handling, and processing mode detection all work. This is ~2,500 lines of solid code that does not need replacement.

2. **Hand-author templates for the top 5-10 parent styles** - Starting from `examples/apa-style.yaml` as a model, use style guides and oracle verification to produce gold-standard templates. This covers 60% of dependent styles with the highest confidence.

3. **Build output-driven template inference for the next tier** - For parent styles beyond the top 10, use citeproc-js output + input data cross-referencing to generate template structure. This requires hardening oracle.js first.

4. **Retain the XML compiler as a fallback** - For the remaining parent styles, the XML compiler provides a reasonable starting point. It already gets citations right.

5. **Use oracle comparison as cross-validation for all approaches** - Where hand-authored, output-inferred, and XML-compiled templates agree, confidence is high.

### Why hybrid, not pure output-driven

- You still need XML for options (the output-driven approach literally cannot extract `initialize-with`, `name-as-sort-order`, or `et-al-min` from rendered strings)
- You still need XML for rare reference types not covered by test data
- You still need XML for locale terms, substitute rules, and disambiguation
- The existing options pipeline is proven and does not need replacement

### Why hybrid, not pure XML compiler

- The template compiler has hit a wall. The 0% bibliography match across ALL top styles is not a bug to fix; it is evidence that flattening type-specific choose/if/else branches into a single declarative template with overrides is fundamentally harder than the XML approach can handle with heuristic post-processing passes.
- The template structure for most styles is simple: 8-12 components in a predictable order. Hand-authoring or inferring this is far more reliable than deducing it from 126 choose blocks.

### Estimated effort

- Hand-authored top 10 templates: ~5-10 hours of domain-expert time (APA already done)
- Oracle.js parser hardening: ~300-500 lines (replace substring matching with proper field-aware parsing)
- Output-driven template inferrer: ~500-800 lines (extend hardened oracle.js + add variable cross-referencing)
- Integration with options pipeline: ~200 lines
- Test fixture expansion: ~200 lines of JSON (15 → 25-30 reference items)
- Testing and validation: Use existing oracle infrastructure

### Risk mitigation

- Expand test fixtures from 16 references to 25-30, covering all major reference types (add article-newspaper, dataset, legal_case, entry at minimum)
- Use the XML's choose/if type conditions as a validation checklist (ensure inferred template has overrides for all types the XML mentions)
- Start with APA (the most complex, 99 macros) as proof-of-concept; the hand-authored version already exists
- **Preserve citation template generation** - The XML compiler achieves 87-100% citation match; any template changes must not regress this
- Harden oracle.js component parser before building inference on top of it

---

## Files Referenced

- `crates/csln_migrate/src/template_compiler/mod.rs` - Current template compiler (2,077 lines), the bottleneck
- `crates/csln_migrate/src/lib.rs` - MacroInliner with macro expansion logic
- `crates/csln_migrate/src/upsampler.rs` - CslNode to CslnNode conversion (works well)
- `crates/csln_migrate/src/options_extractor/` - Options pipeline (works well, keep)
- `crates/csln_core/src/template.rs` - CSLN template model (target schema)
- `scripts/oracle.js` - Oracle with component parser (needs hardening before use as inference foundation)
- `examples/apa-style.yaml` - Hand-authored APA style (gold standard, 11 components)
- `tests/fixtures/references-expanded.json` - Test fixture (16 items, 7 types - needs expansion)
- `.beans/csl26-rh2u--preserve-macro-call-order-from-csl-10-during-parsi.md` - The triggering bean
- `.beans/csl26-m3lb--implement-hybrid-migration-strategy.md` - Implementation milestone
