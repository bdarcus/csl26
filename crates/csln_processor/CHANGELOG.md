# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/bdarcus/csl26/releases/tag/csln_processor-v0.3.0) - 2026-02-15

### Added

- *(processor)* support infix variable in integral citations
- *(core)* add locale term role labels
- *(citations)* add infix support for integrals
- *(processor)* add HTML output for Djot document processing
- *(processor)* implement djot document processing and structured locators
- *(processor)* support hybrid and structured locators in Djot parser
- *(processor)* simplify Djot citation syntax by removing mandatory attribute
- *(processor)* implement WinnowCitationParser for Djot syntax
- *(processor)* add document-level processing prototype
- *(cli)* support complex citation models as input
- fix override fallback and migrate Springer style
- *(processor)* implement strip-periods in term and number labels
- add performance benchmarking
- *(edtf)* implement modern winnow-based parser
- *(dates)* implement EDTF uncertainty, approximation, and range rendering
- add CBOR binary format support and conversion tool
- *(processor)* implement multilingual BCP 47 resolution
- *(cli)* merge process and validate into csln
- *(core)* implement declarative hyperlink configuration
- *(multilingual)* implement multilingual support
- *(processor)* add integral citation mode to CLI output
- *(processor)* improve rendering engine and test dataset
- add structured html output for bibliography
- migrate and process localized terms
- implement pluggable output rendering and semantic markup
- *(rendering)* implement inner/outer affixes
- *(migration)* add styleauthor migration pathway
- improve template inference and sync tests
- wire up three-tier options architecture
- *(core,processor)* add locator support, mode-dependent logic, and integral citation templates
- *(core,processor)* add locator support and refine punctuation rendering
- *(migrate)* implement complete source_order tracking system
- *(migrate)* add custom delimiter support for CSL 1.0 compatibility
- improve APA bibliography formatting and core infrastructure
- unify reference models and fix processor tests
- *(core)* add prefix_inside_wrap for flexible wrap ordering
- *(processor)* improve bibliography separator handling
- *(core)* implement editor label format standardization
- *(locale)* expose locator terms for page labels
- *(options)* add substitute presets and style-aware contributor matching
- *(options)* add configurable URL trailing period
- *(processor)* add citation grouping and year suffix ordering
- *(processor)* Achieve 15/15 oracle parity for Chicago and APA ([#54](https://github.com/bdarcus/csl26/pull/54))
- *(test)* Expand test data to 15 reference items ([#53](https://github.com/bdarcus/csl26/pull/53))
- *(core,processor)* implement curly quote rendering
- *(render)* implement structured hyperlinking in templates
- *(contributor)* implement et-al-use-last truncation
- *(core)* expose embedded templates via use-preset
- add new CSLN reference model and biblatex crate
- *(migrate)* extract bibliography entry suffix from CSL layout
- *(locale)* implement punctuation-in-quote as locale option
- *(render)* add title quotes and fix period-inside-quotes
- *(processor)* implement declarative title and contributor rendering logic
- *(core)* add else-if branches and type-specific bibliography templates
- *(core)* add overrides support to contributor and date components
- *(processor)* support per-component name conjunction override
- *(processor)* fix name initials formatting and extraction
- *(processor)* add bibliography entry numbering for numeric styles
- *(migrate)* extract bibliography sort and fix citation delimiter
- *(core)* add multi-language locale support
- *(processor)* add citation layout support
- update legacy name parsing and processor support
- *(core)* implement style versioning and forward compatibility
- *(bib)* implement subsequent author substitution
- implement name and given-name disambiguation
- implement demote-non-dropping-particle (2,570 styles)
- implement delimiter-precedes-et-al (786 styles)
- implement page-range-format (minimal, chicago, expanded)
- add style-level options for name initialization
- achieve 5/5 oracle match with name_order control
- type-specific overrides for APA formatting

### Fixed

- *(processor)* prevent HTML escaping in docs
- *(processor)* use correct jotdown API
- *(processor)* author substitution and grouping bugs
- *(processor)* allow variable repetition with different context
- *(locale)* handle nested Forms in role term extraction
- *(bibliography)* preserve component suffixes in separator deduplication
- *(processor)* resolve mode-dependent conjunctions and implement deep config merging
- *(core)* enable initialize-with override on contributor components
- improve bibliography separator handling for wrapped components
- *(render,inferrer)* improve delimiter detection and URL suffix handling
- *(migrate)* improve contributor and bibliography migration
- *(processor)* add contributor labels and sorting fixes
- *(render)* suppress trailing period after URLs in nested lists
- *(sort)* strip leading articles and fix anonymous work formatting
- *(processor)* improve bibliography sorting with proper key chaining
- *(processor)* implement variable-once rule for substituted titles
- *(processor)* add context-aware delimiter for two-author bibliographies
- *(processor)* implement contributor verb and label forms
- *(migrate)* resolve template nesting regression with recursive deduplication
- *(processor)* correctly map ParentSerial/ParentMonograph to container_title
- *(processor)* use container_title for chapter book titles
- variable-once rule and serde parsing for Variable components

### Other

- add automated code versioning
- *(processor)* modularize document processing
- *(core)* strict typing with custom fields
- final clippy fixes and document processing polish
- remove processor magic and fix punctuation suppression
- *(beans)* track support for inner and outer affixes
- rename binaries from underscores to hyphens
- Revert "feat(migrate): implement complete source_order tracking system"
- modularize core and processor crates
- modularize core crates
- format code with cargo fmt
- add architecture principles and improve code comments
- Improve separator logic and document known limitations
- Fix test data: proper editor names for chapter
- Improve bibliography formatting for oracle match
- Implement List component for combined volume(issue) format
- Add editor role labels (Ed.) for verb form
- Add corporate author test case (World Bank)
- Improve bibliography formatting
- Fix bibliography rendering issues
- Add csln_processor CLI for testing
- Add locale support for terms and date formatting
- Add csln_processor crate with CSLN template rendering
