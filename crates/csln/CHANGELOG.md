# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/bdarcus/csl26/releases/tag/v0.3.0) - 2026-02-15

### Added

- *(processor)* add HTML output for Djot document processing
- *(processor)* implement WinnowCitationParser for Djot syntax
- *(processor)* add document-level processing prototype
- *(cli)* support complex citation models as input
- *(cli)* add --show-keys flag to process command
- implement schema generation, validation
- add CBOR binary format support and conversion tool
- *(cli)* merge process and validate into csln

### Fixed

- remove per-crate changelogs and configure single release
- *(locale)* handle nested Forms in role term extraction

### Other

- release v0.3.0
- add automated code versioning
- *(processor)* modularize document processing
- final clippy fixes and document processing polish
- *(cli)* use explicit short flags and fix annals citation
- *(cli)* csln-processor -> csln process
