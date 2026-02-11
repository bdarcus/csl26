# CSL Next Multilingual Support Design

**Status**: Draft
**Authors**: @dstyleplan
**Date**: 2026-02-11

## Overview

This document outlines the architectural design for adding "elegant" multilingual support to CSL Next (CSLN). The goal is to move away from procedural macros and toward a declarative, type-safe system that handles parallel metadata for high-fidelity citations.

## core Principles

1.  **High-Fidelity Data**: Store original, transliterated, and translated versions of metadata fields side-by-side.
2.  **Declarative Style**: Styles request a specific "view" of the data (e.g., "transliterated [translated]") rather than implementing complex logic.
3.  **Graceful Degradation**: Simple use cases (monolingual data) must remain simple. The complexity of multilingual support should only be incurred when necessary.
4.  **Performance Check**: Heavy dependencies (like ICU4X for sorting) must be optional via feature flags.

## 1. Data Model

The core data model in `csln_core` will be updated to support **Parallel Metadata**.

### 1.1 `Contrbutor` and `String` Fields

Currently, fields like `title` and `author` (via `Contributor`) primarily store single string values. We will introduce a pattern to allow them to store complex objects without breaking the simple string ease-of-use.

**Schema (YAML) Examples:**

*Simple (Current Behavior):*
```yaml
title: "The Great Gatsby"
author: "Fitzgerald, F. Scott"
```

*Advanced (Multilingual):*
```yaml
title:
  original: "战争与和平"
  transliteration: "Zhànzhēng yǔ Hépíng"
  translation: "War and Peace"
author:
  family:
    original: "Tolstoy"
    transliteration: "Tolstoy"
  given: "Leo"
```

### 1.2 Internal Representation

We will use Serde's `untagged` enum feature to seamlessly support both formats.

```rust
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum MultilingualString {
    Simple(String),
    Complex {
        original: Option<String>,
        transliteration: Option<String>,
        translation: Option<String>,
        lang: Option<String>, // ISO language code for the original
    }
}
```

## 2. Style Configuration

A new global configuration section `multilingual` will be added to the CSLN style schema.

```yaml
options:
  multilingual:
    # How to render titles:
    # - primary: Use original script
    # - transliterated: Prefer transliteration
    # - translated: Use translation
    # - combined: "original [translation]" pattern
    title-mode: "transliterated [translated]" 
    
    # How to render names:
    name-mode: "transliterated"
    
    # Transliteration standard (informational for now, potential validation later)
    transliteration-standard: "ala-lc"
```

## 3. Processor Logic

The `csln_processor` will implement the view logic.

### 3.1 Value Resolution

When the template requests a variable (e.g., `title`), the processor will:

1.  Check the `MultilingualOptions` for the current style.
2.  Resolves the value based on the mode:
    *   **Primary**: Returns `original` ?? `Simple` string.
    *   **Transliterated**: Returns `transliteration` ?? `original`.
    *   **Translated**: Returns `translation` ?? `original`.
    *   **Combined**: Formats the string using the specified pattern (e.g., `"{transliteration} [{translation}]"`). 

### 3.2 Locale Separation

The processor must distinguish between:
*   **Data Language**: The language of the source metadata (e.g., Russian).
*   **Style Locale**: The language of the citation style (e.g., English for "edited by").

Labels ("Ed.", "vol.") will always use the **Style Locale**. Data fields will use the script determined by the **Data Language** and **Multilingual Mode**.

## 4. Sorting & Transliteration

Sorting mixed scripts (e.g., Hanzi vs. Latin) requires Unicode Collation Algorithm (UCA) support.

### 4.1 Implementation

*   **Library**: Use `icu_collation` (ICU4X) for robust, locale-aware sorting.
*   **Logic**: 
    *   If a sort key is `author` or `title`, the processor should prefer the `transliteration` variant if available, even if the bibliography displays the `original` script. This ensures that "Tolstoy" (Cyrillic) sorts near "Tolstoy" (Latin) in an English bibliography.
    
### 4.2 Performance & Feature Flags

To avoid bloating the binary size for users who only need English/Simple citation support, all ICU4X dependencies will be gated.

```toml
[features]
default = []
multilingual = ["dep:icu_collation", "dep:icu_locid", "dep:icu_properties"]
```

## 5. Disambiguation

When names appear in multiple scripts, simple string matching fails to identify them as the same person.

*   **Strategy**: Use Persistent Identifiers (ORCID, DOI, etc.) as the primary key for disambiguation grouping.
*   **Fallback**: If no PID is present, fall back to string comparison of the `transliteration` field if available.
