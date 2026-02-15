# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/bdarcus/csl26/releases/tag/csln_migrate-v0.3.0) - 2026-02-15

### Added

- *(presets)* add options-level preset support
- *(processor)* implement strip-periods in term and number labels
- *(dates)* implement EDTF uncertainty, approximation, and range rendering
- migrate and process localized terms
- *(rendering)* implement inner/outer affixes
- *(migration)* add styleauthor migration pathway
- improve template inference and sync tests
- refine container template inference grouping
- *(migrate)* integrate template resolution cascade with per-component delimiters
- *(migrate)* implement complete source_order tracking system
- *(migrate)* add custom delimiter support for CSL 1.0 compatibility
- *(migrate)* add variable provenance debugger
- improve APA bibliography formatting and core infrastructure
- *(core)* add prefix_inside_wrap for flexible wrap ordering
- *(migrate)* implement publisher-place visibility rules
- *(migrate)* improve migration fidelity and deduplication
- *(processor)* improve bibliography separator handling
- *(core)* implement editor label format standardization
- *(migrate)* support type-conditional substitution extraction
- *(migrate)* infer month format from CSL date-parts
- *(options)* add substitute presets and style-aware contributor matching
- *(options)* add configurable URL trailing period
- *(processor)* Achieve 15/15 oracle parity for Chicago and APA ([#54](https://github.com/bdarcus/csl26/pull/54))
- Complete Chicago Author-Date bibliography rendering (5/5) ([#52](https://github.com/bdarcus/csl26/pull/52))
- *(core,processor)* implement curly quote rendering
- *(core)* expose embedded templates via use-preset
- *(migrate)* add preset detection for extracted configs
- *(migrate)* extract bibliography entry suffix from CSL layout
- *(migrate)* extract volume-pages delimiter from CSL styles
- *(locale)* implement punctuation-in-quote as locale option
- *(render)* add title quotes and fix period-inside-quotes
- *(processor)* implement declarative title and contributor rendering logic
- *(migrate)* add chapter type_template for author-date styles
- *(migrate)* add type-specific template extraction (disabled)
- *(core)* add else-if branches and type-specific bibliography templates
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
- *(csln_migrate)* improve template ordering and author-date citation
- *(csln_migrate)* add TemplateCompiler for clean CSLN output
- *(csln_migrate)* integrate OptionsExtractor into migration CLI
- *(csln_core, csln_migrate)* add CSLN schema and OptionsExtractor
- Locale Ingestion and Advanced Name Mapping
- Enhanced Names handling and Verification
- Proof-of-concept CSLN Renderer
- Initial commit of CSLN Architecture

### Fixed

- remove per-crate changelogs and configure single release
- *(core)* enable initialize-with override on contributor components
- *(migrate)* correct contributor name order logic
- *(migrate)* preserve macro call order across choose branches
- improve list component merging in template compiler
- *(migrate)* preserve label_form from CSL 1.0 Label nodes
- *(migrate)* add date deduplication in lists
- *(migrate)* disable hardcoded component sorting
- *(migrate)* use IndexMap to preserve component ordering
- *(migrate)* add text-case support for term nodes and deduplicate numbers
- *(migrate)* improve contributor and bibliography migration
- *(migrate)* prevent duplicate list variables
- *(migrate)* improve bibliography component extraction for nested variables
- *(migrate)* extract bibliography delimiter from nested groups
- *(migrate)* handle Choose blocks in delimiter extraction
- *(migrate)* extract correct citation delimiter from innermost group
- *(migrate)* add space prefix to volume after journal name
- *(migrate)* detect numeric styles and position year at end
- *(migrate)* extract author from substitute when primary is rare role
- restore working template compiler from pre-modularization
- position year at end of bibliography for numeric styles
- *(migrate)* deduplicate nested lists and fix volume-issue grouping
- *(processor)* add contributor labels and sorting fixes
- *(migrate)* improve bibliography template extraction
- *(migrate)* use full names in bibliography for styles without style-level initialize-with ([#56](https://github.com/bdarcus/csl26/pull/56))
- *(migrate)* extract 'and' configuration from citation macros
- *(migrate)* use space suffix for chicago journal titles
- *(migrate)* remove comma before volume for chicago journals
- *(migrate)* chicago publisher-place visibility rules
- *(migrate)* suppress pages for chicago chapters
- *(migrate)* improve CSL extraction and template generation
- *(migrate)* recursive type overrides for nested components
- *(migrate)* context-aware contributor option extraction
- *(migrate)* resolve template nesting regression with recursive deduplication
- *(migrate)* add page formatting overrides for journals and chapters
- *(migrate)* add editor/container-title for chapters, suppress journal publisher
- *(migrate)* extract date wrapping from original CSL style
- *(migrate)* improve citation delimiter extraction
- *(migrate)* disable auto chapter type_template generation
- *(processor)* use container_title for chapter book titles
- *(migrate)* improve template compilation
- handle is-uncertain-date condition in migration
- variable-once rule and serde parsing for Variable components
- *(csln_migrate)* improve substitute extraction for real styles

### Other

- release v0.3.0
- add automated code versioning
- remove processor magic and fix punctuation suppression
- rename styles directory to styles-legacy
- rename binaries from underscores to hyphens
- Revert "feat(migrate): implement complete source_order tracking system"
- *(migrate)* implement occurrence-based template compilation
- modularize core and processor crates
- modularize core crates
- *(migrate)* convert remaining TODOs to issues
- *(core)* use DelimiterPunctuation enum for volume_pages_delimiter
- *(migrate)* fix doc comment indentation for clippy
- fix formatting and clippy warnings
- Improve separator logic and document known limitations
- Fix test data: proper editor names for chapter
- Combine volume(issue) in migrated bibliography
- Improve bibliography formatting for oracle match
- Fix linter warnings (cargo fix)
- Add end-to-end oracle test for migrated styles
- add refactor plan for csln core alignment and baseline analysis
