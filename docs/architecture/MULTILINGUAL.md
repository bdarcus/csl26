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

### 1.1 `Contributor` and `String` Fields

Currently, fields like `title` and `author` (via `Contributor`) primarily store single string values. We use a pattern to allow them to store complex objects without breaking the simple string ease-of-use.

**Schema (YAML) Examples:**

*Simple (Current Behavior):*
```yaml
title: "The Great Gatsby"
author: "Fitzgerald, F. Scott"
```

*Advanced (Multilingual Title):*
```yaml
title:
  original: "战争与和平"
  lang: "zh"
  transliterations:
    zh-Latn-pinyin: "Zhànzhēng yǔ Hépíng"
  translations:
    en: "War and Peace"
```

*Advanced (Multilingual Contributor):*
Names use a holistic multilingual approach where the entire name structure has parallel variants.

```yaml
author:
  original:
    family: "Tolstoy"
    given: "Leo"
  lang: "ru"
  transliterations:
    Latn:
      family: "Tolstoy"
      given: "Leo"
```

### 1.2 Internal Representation

We use Serde's `untagged` enum feature to seamlessly support both formats. This model incorporates feedback that alternate fields need explicit language and script tagging.

```rust
// For Titles and simple strings
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum MultilingualString {
    Simple(String),
    Complex(MultilingualComplex),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualComplex {
    pub original: String,
    pub lang: Option<LangID>,
    /// Transliterations/Transcriptions of the original text.
    /// Keys MUST be valid BCP 47 language tags including script and optional variant subtags.
    /// Script subtag specifies the writing system (Latn=Latin, Cyrl=Cyrillic, etc.).
    /// Variant subtag specifies the romanization method:
    ///   Japanese: "ja-Latn-hepburn" (Hepburn), "ja-Latn-kunrei" (Kunrei-shiki)
    ///   Chinese: "zh-Latn-pinyin" (Pinyin), "zh-Latn-wadegile" (Wade-Giles)
    ///   Russian: "ru-Latn-alalc97" (ALA-LC), "ru-Latn-bgn" (BGN/PCGN)
    /// Matching strategy: exact BCP 47 tag → script prefix → fallback to original.
    pub transliterations: HashMap<String, String>,
    pub translations: HashMap<LangID, String>,
}

// For Contributors
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum Contributor {
    SimpleName(SimpleName),
    StructuredName(StructuredName),
    Multilingual(MultilingualName), // Holistic parallel names
    ContributorList(ContributorList),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualName {
    pub original: StructuredName,
    pub lang: Option<LangID>,
    pub transliterations: HashMap<String, StructuredName>,
    pub translations: HashMap<LangID, StructuredName>,
}
```

### 1.3 Transliteration Methods

Transliteration keys use BCP 47 language tags with script and variant subtags to specify the exact romanization system used.

**Common transliteration variants:**
- Japanese: `ja-Latn-hepburn` (Hepburn), `ja-Latn-kunrei` (Kunrei-shiki)
- Chinese: `zh-Latn-pinyin` (Pinyin), `zh-Latn-wadegile` (Wade-Giles)
- Russian: `ru-Latn-alalc97` (ALA-LC), `ru-Latn-bgn` (BGN/PCGN)

**Matching strategy:**
1. Exact BCP 47 tag match (e.g., "ja-Latn-hepburn")
2. Prefix match on script (e.g., "ja-Latn" matches any Latin transliteration)
3. Fallback to `original` field

Future: `preferred-transliteration` style option will allow explicit method selection.
```

## 2. Style Configuration

A new global configuration section `multilingual` will be added to the CSLN style schema.

```yaml
options:
  multilingual:
    # Preferred view for titles:
    # - primary: Use original script
    # - transliterated: Prefer transliteration
    # - translated: Use translation matching style locale
    # - combined: "original [translation]" pattern
    title-mode: "transliterated [translated]" 
    
    # Preferred view for names:
    # - primary, transliterated, translated, combined
    name-mode: "transliterated"
    
    # Preferred script for transliterations (e.g., "Latn", "Cyrl")
    preferred-script: "Latn"

    # Script-specific behavior
    scripts:
      cjk:
        use-native-ordering: true # FamilyGiven for CJK
        delimiter: ""            # No space between Family/Given
```

## 3. Processor Logic

### 3.1 Value Resolution

... [existing resolution logic] ...

### 3.2 Script-Aware Ordering

For contributors, the processor must be script-aware to handle ordering (Given Family vs Family Given) and delimiters.

1.  **Detection**: Determine the script of the resolved name (e.g., Latin vs CJK).
2.  **Ordering**: 
    *   If CJK and `use-native-ordering` is true, use `FamilyGiven`.
    *   If Latin, use `Given Family` (unless `sort-order` is requested).
3.  **Delimiters**: Use script-appropriate delimiters for contributor lists (e.g., "・" for Japanese lists).

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

Citation disambiguation resolves surface-level ambiguity in written references for readers, not real-world identity verification.

### 5.1 Strategy

**Primary:** String matching on the displayed written form:
- If style displays `transliteration`, compare transliterated strings
- If style displays `original`, compare original script strings
- If style displays `translation`, compare translated strings

**Fallback:** If no exact match, use normalized comparison (Unicode NFC, case-folding)

### 5.2 PIDs Are Not Disambiguation Keys

Persistent identifiers (ORCID, DOI, ISBN) serve identity verification and linking purposes, but are NOT used for citation disambiguation. Two reasons:

1. **Scope mismatch**: PIDs identify entities globally, but disambiguation only needs to distinguish items within a single bibliography
2. **Display mismatch**: Readers see "Smith, J." vs "Smith, John" in text, not ORCIDs

PIDs remain valuable for metadata quality and cross-referencing, but disambiguation operates on rendered output strings.
