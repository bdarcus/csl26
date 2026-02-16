# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/bdarcus/csl26/releases/tag/csln_core-v0.4.0) - 2026-02-16

### Added

- configurable bibliography grouping
- *(djot)* implement citation visibility modifiers and grouping 
- *(processor)* support infix variable in integral citations
- *(core)* add locale term role labels
- *(citations)* add infix support for integrals
- *(processor)* implement djot document processing and structured locators
- *(processor)* add document-level processing prototype
- *(cli)* support complex citation models as input
- *(core)* add Standard and Software types
- *(core)* add Patent and Dataset reference types
- *(core)* add Tier 1 legal reference types
- *(presets)* add options-level preset support
- *(processor)* implement strip-periods in term and number labels
- implement schema generation, validation
- add performance benchmarking
- *(edtf)* implement modern winnow-based parser
- *(dates)* implement EDTF uncertainty, approximation, and range rendering
- add CBOR binary format support and conversion tool
- *(core)* implement declarative hyperlink configuration
- *(multilingual)* implement multilingual support
- *(multilingual)* implement holistic parallel metadata for names and titles
- *(core)* add InputBibliography and TemplateDate fallback support
- migrate and process localized terms
- implement pluggable output rendering and semantic markup
- *(rendering)* implement inner/outer affixes
- improve template inference and sync tests
- wire up three-tier options architecture
- *(skills)* add styleauthor skill and agent for LLM-driven style creation
- *(core,processor)* add locator support, mode-dependent logic, and integral citation templates
- *(core,processor)* add locator support and refine punctuation rendering
- *(migrate)* implement complete source_order tracking system
- *(migrate)* add custom delimiter support for CSL 1.0 compatibility
- improve APA bibliography formatting and core infrastructure
- unify reference models and fix processor tests
- add comprehensive bibliographic examples and schema updates
- *(core)* add prefix_inside_wrap for flexible wrap ordering
- *(core)* implement editor label format standardization
- *(migrate)* support type-conditional substitution extraction
- *(reference)* support parent reference by ID
- *(migrate)* infer month format from CSL date-parts
- *(locale)* expose locator terms for page labels
- *(options)* add substitute presets and style-aware contributor matching
- *(options)* add configurable URL trailing period
- *(core,processor)* implement curly quote rendering
- *(core)* enhance citation model and add bibliography separator config
- *(render)* implement structured hyperlinking in templates
- *(contributor)* implement et-al-use-last truncation
- *(core)* expose embedded templates via use-preset
- *(core)* add embedded priority templates for Phase 2
- *(core)* add style preset vocabulary for Phase 1
- add new CSLN reference model and biblatex crate
- *(migrate)* extract bibliography entry suffix from CSL layout
- *(migrate)* extract volume-pages delimiter from CSL styles
- *(locale)* implement punctuation-in-quote as locale option
- *(processor)* implement declarative title and contributor rendering logic
- *(core)* add else-if branches and type-specific bibliography templates
- *(core)* add overrides support to contributor and date components
- *(processor)* support per-component name conjunction override
- *(migrate)* extract bibliography sort and fix citation delimiter
- *(core)* add multi-language locale support
- *(processor)* add citation layout support
- *(core)* add json schema generation support and docs
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
- *(csln_migrate)* add TemplateCompiler for clean CSLN output
- *(csln_core, csln_migrate)* add CSLN schema and OptionsExtractor
- Locale Ingestion and Advanced Name Mapping
- Enhanced Names handling and Verification
- Proof-of-concept CSLN Renderer
- Initial commit of CSLN Architecture

### Fixed

- remove per-crate changelogs and configure single release
- *(processor)* author substitution and grouping bugs
- *(locale)* handle nested Forms in role term extraction
- *(core)* alias DOI/URL/ISBN/ISSN for CSL-JSON
- *(processor)* resolve mode-dependent conjunctions and implement deep config merging
- *(core)* enable initialize-with override on contributor components
- *(migrate)* preserve macro call order across choose branches
- *(migrate)* preserve label_form from CSL 1.0 Label nodes
- *(migrate)* improve contributor and bibliography migration
- *(reference)* extract actual day from EDTF dates
- *(sort)* strip leading articles and fix anonymous work formatting
- *(migrate)* improve CSL extraction and template generation
- *(migrate)* resolve template nesting regression with recursive deduplication
- variable-once rule and serde parsing for Variable components

### Other

- release v0.3.0 ([#168](https://github.com/bdarcus/csl26/pull/168))
- release v0.3.0
- add automated code versioning
- *(core)* strict typing with custom fields
- final clippy fixes and document processing polish
- *(examples)* add info field and restructure bibliography files
- remove processor magic and fix punctuation suppression
- document mode-dependent citation formatting
- rename binaries from underscores to hyphens
- Revert "feat(migrate): implement complete source_order tracking system"
- modularize core and processor crates
- modularize core crates
- *(reference)* convert parent-by-id TODO
- *(core)* remove feature gate from embedded templates
- *(core)* use DelimiterPunctuation enum for volume_pages_delimiter
- fix formatting and clippy warnings
- add architecture principles and improve code comments
- Add editor role labels (Ed.) for verb form
- Add locale support for terms and date formatting
- add refactor plan for csln core alignment and baseline analysis

## [0.3.0](https://github.com/bdarcus/csl26/releases/tag/csln_core-v0.3.0) - 2026-02-15

### Added

- *(processor)* support infix variable in integral citations
- *(core)* add locale term role labels
- *(citations)* add infix support for integrals
- *(processor)* implement djot document processing and structured locators
- *(processor)* add document-level processing prototype
- *(cli)* support complex citation models as input
- *(core)* add Standard and Software types
- *(core)* add Patent and Dataset reference types
- *(core)* add Tier 1 legal reference types
- *(presets)* add options-level preset support
- *(processor)* implement strip-periods in term and number labels
- implement schema generation, validation
- add performance benchmarking
- *(edtf)* implement modern winnow-based parser
- *(dates)* implement EDTF uncertainty, approximation, and range rendering
- add CBOR binary format support and conversion tool
- *(core)* implement declarative hyperlink configuration
- *(multilingual)* implement multilingual support
- *(multilingual)* implement holistic parallel metadata for names and titles
- *(core)* add InputBibliography and TemplateDate fallback support
- migrate and process localized terms
- implement pluggable output rendering and semantic markup
- *(rendering)* implement inner/outer affixes
- improve template inference and sync tests
- wire up three-tier options architecture
- *(skills)* add styleauthor skill and agent for LLM-driven style creation
- *(core,processor)* add locator support, mode-dependent logic, and integral citation templates
- *(core,processor)* add locator support and refine punctuation rendering
- *(migrate)* implement complete source_order tracking system
- *(migrate)* add custom delimiter support for CSL 1.0 compatibility
- improve APA bibliography formatting and core infrastructure
- unify reference models and fix processor tests
- add comprehensive bibliographic examples and schema updates
- *(core)* add prefix_inside_wrap for flexible wrap ordering
- *(core)* implement editor label format standardization
- *(migrate)* support type-conditional substitution extraction
- *(reference)* support parent reference by ID
- *(migrate)* infer month format from CSL date-parts
- *(locale)* expose locator terms for page labels
- *(options)* add substitute presets and style-aware contributor matching
- *(options)* add configurable URL trailing period
- *(core,processor)* implement curly quote rendering
- *(core)* enhance citation model and add bibliography separator config
- *(render)* implement structured hyperlinking in templates
- *(contributor)* implement et-al-use-last truncation
- *(core)* expose embedded templates via use-preset
- *(core)* add embedded priority templates for Phase 2
- *(core)* add style preset vocabulary for Phase 1
- add new CSLN reference model and biblatex crate
- *(migrate)* extract bibliography entry suffix from CSL layout
- *(migrate)* extract volume-pages delimiter from CSL styles
- *(locale)* implement punctuation-in-quote as locale option
- *(processor)* implement declarative title and contributor rendering logic
- *(core)* add else-if branches and type-specific bibliography templates
- *(core)* add overrides support to contributor and date components
- *(processor)* support per-component name conjunction override
- *(migrate)* extract bibliography sort and fix citation delimiter
- *(core)* add multi-language locale support
- *(processor)* add citation layout support
- *(core)* add json schema generation support and docs
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
- *(csln_migrate)* add TemplateCompiler for clean CSLN output
- *(csln_core, csln_migrate)* add CSLN schema and OptionsExtractor
- Locale Ingestion and Advanced Name Mapping
- Enhanced Names handling and Verification
- Proof-of-concept CSLN Renderer
- Initial commit of CSLN Architecture

### Fixed

- remove per-crate changelogs and configure single release
- *(processor)* author substitution and grouping bugs
- *(locale)* handle nested Forms in role term extraction
- *(core)* alias DOI/URL/ISBN/ISSN for CSL-JSON
- *(processor)* resolve mode-dependent conjunctions and implement deep config merging
- *(core)* enable initialize-with override on contributor components
- *(migrate)* preserve macro call order across choose branches
- *(migrate)* preserve label_form from CSL 1.0 Label nodes
- *(migrate)* improve contributor and bibliography migration
- *(reference)* extract actual day from EDTF dates
- *(sort)* strip leading articles and fix anonymous work formatting
- *(migrate)* improve CSL extraction and template generation
- *(migrate)* resolve template nesting regression with recursive deduplication
- variable-once rule and serde parsing for Variable components

### Other

- release v0.3.0
- add automated code versioning
- *(core)* strict typing with custom fields
- final clippy fixes and document processing polish
- *(examples)* add info field and restructure bibliography files
- remove processor magic and fix punctuation suppression
- document mode-dependent citation formatting
- rename binaries from underscores to hyphens
- Revert "feat(migrate): implement complete source_order tracking system"
- modularize core and processor crates
- modularize core crates
- *(reference)* convert parent-by-id TODO
- *(core)* remove feature gate from embedded templates
- *(core)* use DelimiterPunctuation enum for volume_pages_delimiter
- fix formatting and clippy warnings
- add architecture principles and improve code comments
- Add editor role labels (Ed.) for verb form
- Add locale support for terms and date formatting
- add refactor plan for csln core alignment and baseline analysis
