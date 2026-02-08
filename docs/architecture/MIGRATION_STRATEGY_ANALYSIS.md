# Migration Strategy Analysis: XML Compiler vs Output-Driven

## Context

Bean `csl26-rh2u` and the broader epic `csl26-ifiw` track a fundamental problem: the template compiler produces bibliography templates with wrong component ordering, duplicate/missing components, and incorrect suppress logic. Current results: **87-100% citation match, 0% bibliography match** across all top parent styles. The template compiler (`crates/csln_migrate/src/template_compiler/mod.rs`, 2,077 lines) is the bottleneck.

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
2. **Source order has failed twice** - The attempt to track macro call ordering was reverted (commit `1c9ad45`). Component ordering emerges from runtime evaluation, not from XML node position.
3. **Combinatorial explosion** - APA has 99 macros and 126 choose blocks. Flattening these into a flat template with correct suppress overrides for every type is an extremely high-dimensional mapping problem.
4. **Heuristic passes are fragile** - The reorder, deduplicate, and grouping passes use pattern-matching heuristics. Fixing one style's layout frequently breaks another.
5. **Group semantics mismatch** - CSL 1.0 groups suppress their delimiter when a child is empty; CSLN has no equivalent implicit behavior. This creates phantom components and incorrect spacing.
6. **Diminishing returns** - The easy 87% came cheaply; the remaining gap involves the hardest cases where the two models diverge most.

---

## Approach B: Output-Driven / Reverse Engineering

**How it works:** Run citeproc-js with diverse test references, parse rendered output strings into structured components, cross-reference with input data to infer variable-to-output mappings, generate CSLN YAML directly from observed patterns.

### Pros

1. **Directly targets the success criterion** - The oracle comparison IS the definition of correctness. Deriving the template from the output closes the loop: observed output leads to template leads to processor leads to same output.
2. **Bypasses the source_order problem entirely** - Component ordering is directly observed, not inferred from XML traversal.
3. **Naturally resolves group semantics** - Group delimiter behavior, implicit suppression, and macro interaction effects are all resolved by citeproc-js before inference begins. No need to replicate that logic.
4. **Type-specific overrides emerge naturally** - Comparing outputs across reference types directly reveals differences: "publisher appears for chapters but not journals" becomes `suppress: true` for `article-journal`.
5. **Human-intuitive** - Produces templates resembling what a style author would write by reading a style guide: "Author (Year). Title. *Journal*, volume(issue), pages."
6. **Well-suited to CSLN's design** - The flat template model was designed to be what a human would write. This approach produces exactly that.
7. **Simpler conceptually** - No need to understand CSL 1.0's macro expansion, choose/if/else flattening, or group suppression.

### Cons

1. **Test data coverage problem** - You only learn about behavior you observe. CSL 1.0 has 50+ reference types; the current fixture has 15 items. Styles with rare type-specific behavior (legal, patent, dataset) will be missed.
2. **Ambiguous parsing** - Regex-based component extraction is inherently fragile. Is "Cambridge" a publisher or a place? Is "15" a volume or a page number? Context-dependent resolution requires complex heuristics.
3. **Loses metadata linkage** - Output strings do not reveal which CSL variable produced which output token. Cross-referencing with input data helps but is not foolproof (e.g., "Smith" could be author or editor).
4. **Cannot extract global options** - Output "Smith, J." does not tell you whether `initialize-with` is `. ` or the input only had initials. Options like name-as-sort-order, et-al, and page-range-format must still come from XML.
5. **Does not scale** - Each of 2,844 styles needs its own citeproc-js inference run with sufficient test data. This creates a permanent dependency on citeproc-js as infrastructure.
6. **Non-deterministic** - Different test data sets may produce different inferred templates. The approach is probabilistic, not deterministic.
7. **Cannot discover latent features** - Substitute rules, disambiguation, subsequent-author-substitute only trigger under specific conditions. Test data may never exercise them.
8. **Locale conflation** - Output "pp. 1-10" does not reveal whether "pp." is a locale term or a hardcoded prefix. This matters for CSLN's multilingual locale system.
9. **Compensating errors** - If the CSLN processor has bugs, the output-driven approach produces templates that compensate for those bugs rather than being correct.

---

## Architect's Recommendation: Hybrid Approach

**Verdict: Neither approach alone is sufficient. Use a hybrid strategy.**

The critical insight is that these approaches fail at *different things*:

| Capability | XML Compiler | Output-Driven |
|---|---|---|
| Global options (names, dates, et-al) | Excellent | Cannot do |
| Template component ordering | Failed (0% bib) | Excellent |
| Type-specific overrides/suppress | Fragile (heuristic) | Good (observable) |
| Coverage of rare types | Complete | Test-data dependent |
| Scalability to 2,844 styles | One compiler | Per-style inference |
| Locale term handling | Direct | Cannot distinguish |
| Substitute/disambiguation | Encoded in XML | Requires special test data |

### Concrete Architecture

1. **Keep the XML pipeline for OPTIONS** - The options extractor, preset detector, locale handling, and processing mode detection all work. This is ~2,500 lines of solid code that does not need replacement.

2. **Replace the template_compiler with an output-informed template generator** - For the top 10 parent styles (covering 60% of dependents), use citeproc-js output + input data cross-referencing to generate the template structure. This solves the ordering, suppress, and delimiter problems directly.

3. **Retain the XML compiler as a fallback** - For the remaining 290 parent styles, the XML compiler provides a reasonable starting point. It already gets citations right, and bibliography improvements from the top-10 work will generalize.

4. **Use both as cross-validation** - Where the output-driven template and XML-compiled template agree, confidence is high. Where they disagree, the output-driven version is likely correct for component structure, and the XML version is likely correct for options.

### Why hybrid, not pure output-driven

- You still need XML for options (the output-driven approach literally cannot extract `initialize-with`, `name-as-sort-order`, or `et-al-min` from rendered strings)
- You still need XML for rare reference types not covered by test data
- You still need XML for locale terms, substitute rules, and disambiguation
- The existing options pipeline is proven and does not need replacement

### Why hybrid, not pure XML compiler

- The template compiler has hit a wall. The source_order approach failed twice. The 0% bibliography match across ALL top styles is not a bug to fix; it is evidence of a fundamental model mismatch in the compilation approach.
- The template structure for most styles is simple: 8-12 components in a predictable order. Inferring this from output is far more reliable than deducing it from 126 choose blocks.

### Estimated effort

- Output-driven template inferrer: ~500-800 lines (extend existing oracle.js component parser + add variable cross-referencing)
- Integration with options pipeline: ~200 lines
- Testing and validation: Use existing oracle infrastructure

### Risk mitigation

- Expand test fixtures from 15 references to 25-30, covering all major reference types
- Use the XML's choose/if type conditions as a validation checklist (ensure inferred template has overrides for all types the XML mentions)
- Start with APA (the most complex, 99 macros) as proof-of-concept; if it works for APA, simpler styles will follow

---

## Files Referenced

- `crates/csln_migrate/src/template_compiler/mod.rs` - Current template compiler (2,077 lines), the bottleneck
- `crates/csln_migrate/src/lib.rs` - MacroInliner with source_order tracking
- `crates/csln_migrate/src/upsampler.rs` - CslNode to CslnNode conversion (works well)
- `crates/csln_migrate/src/options_extractor/` - Options pipeline (works well, keep)
- `crates/csln_core/src/template.rs` - CSLN template model (target schema)
- `scripts/oracle.js` - Oracle with component parser (foundation for output-driven)
- `examples/apa-style.yaml` - Hand-authored APA style (target example)
- `.beans/csl26-rh2u--preserve-macro-call-order-from-csl-10-during-parsi.md` - The triggering bean
