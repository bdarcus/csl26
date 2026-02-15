# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/bdarcus/csl26/releases/tag/csln_analyze-v0.3.0) - 2026-02-15

### Added

- *(analyze)* add parent style ranking for dependent styles
- implement name and given-name disambiguation
- implement page-range-format (minimal, chicago, expanded)
- add csln_analyze tool for corpus analysis

### Fixed

- remove per-crate changelogs and configure single release

### Other

- release v0.3.0
- add automated code versioning
- rename styles directory to styles-legacy
- rename binaries from underscores to hyphens
- modularize core and processor crates
- update analysis tool with new known name attributes
