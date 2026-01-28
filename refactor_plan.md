# Refactor Plan: Aligning CSLN with `csln-first.yaml`

## Objective
Refactor `csln_core` and `csln_migrate` (specifically `compressor.rs`) to target the data model demonstrated in `csln-first.yaml`. This model prioritizes a declarative, template-driven approach over the procedural/conditional tree of CSL 1.0.

## 1. Core Data Model Updates (`csln_core`)

The current `csln_core` is too close to CSL 1.0. We need to shift towards the `csln-first.yaml` schema.

### 1.1 New Structs
*   **`Options`**: A centralized configuration struct.
    *   `substitute`: Declarative substitution rules (e.g., `contributor_role_form`, `template` fallback).
    *   `processing`: High-level behavior (e.g., `author-date`).
    *   `titles`, `contributors`: Domain-specific default configs.
*   **`Contributor`**: Unified replacement for `Names`, `Label`, etc.
    *   Fields: `role` (author/editor), `form` (long/short/verb), `wrap`, `delimiter`.
*   **`Template`**: A named sequence of rendering instructions.
    *   Replaces generic `Macro`.
    *   Supports context-specific formatting (e.g., `title-apa`).

### 1.2 Deprecations
*   **`ConditionBlock`**: Remove explicit `if/else` logic from the runtime model. All logic must be resolved to **Overrides** or **Substitution Rules**.
*   **`NamesBlock`**: Replace with `Contributor`.

## 2. Compressor Refactor Logic (`csln_migrate/compressor.rs`)

The compressor's job is to transform the CSL 1.0 Tree -> CSLN Flat Templates.

### 2.1 Strategy: "Flatten and Feature"
1.  **Macro Expansion (Inlining)**: We are already doing this (or should be). Ensure all `text-macro` calls are inlined before compression.
2.  **Branch Merging (The Current Logic)**:
    *   *Current*: Merges `if variable=x` branches into `overrides`.
    *   *Upgrade*: Handle complex `group` merges. If a group contains conditionals, push the conditionals down or pull the common elements up.
3.  **Pattern Recognition (New)**:
    *   **Substitution**: Detect "If no Author, substitute Editor" patterns. Remove the nodes and populate `Options.substitute`.
    *   **Disambiguation**: Detect `year-suffix` logic and map to `Options.processing`.
4.  **Template Extraction**:
    *   Identify repeated sequences (even if not macros in the original).
    *   Promote them to named `Templates` (e.g., "standard-publisher-format").

## 3. Success Metrics

How do we quantify success?

1.  **Structural Similarity**:
    *   Target: `csln-first.yaml`
    *   Metric: Edit distance between the *manual* `csln-first.yaml` and the *generated* YAML from `apa.csl`.
2.  **Logic Reduction**:
    *   Metric: Count of `Condition` nodes remaining in the output. Goal is **0**.
3.  **Round-Trip Accuracy** (The "Anystyle Oracle"):
    *   Render `apa.csl` (Legacy) vs `generated_csln` (New Engine).
    *   *Note*: We don't have the New Engine fully built yet, so we rely on Structural Similarity first.

## 4. Implementation Steps

1.  **Modify `csln_core`**: Add `Options`, `Contributor`, update `CslnNode`.
2.  **Update `compressor.rs`**:
    *   Implement "Substitution Detector".
    *   Improve "Branch Merger" to handle `Groups`.
3.  **Run Migration**: Convert `styles/apa.csl`.
4.  **Compare**: Check output against `csln-first.yaml`.

## 5. Baseline Analysis (2026-01-28)

We ran the current migration tool on `apa.csl`.

### 5.1 Results
*   **Output Size**: 3,775 lines (YAML).
*   **Logic Depth**: Up to 15 levels of nested `Condition` and `Group` blocks.
*   **Condition Nodes**: Hundreds of conditional branches remain.

### 5.2 Failure Quantification
*   **Verbosity**: The output is ~75x larger than the target `csln-first.yaml` (approx 50-100 lines).
*   **Semantic Loss**: The current tool translates "what to do" (if this, print that) rather than "what it is" (this is the title, it behaves like X).
*   **Maintainability**: The generated YAML is as hard to read as the original CSL XML, failing the primary goal of the CSLN initiative.

### 5.3 Root Causes
1.  **Insufficient Merging**: `Compressor` only merges the simplest single-variable conditions.
2.  **Missing Substitutions**: Patterns like "Editor instead of Author" are kept as nested `if-else` rather than being promoted to `Options`.
3.  **No Template Promotion**: Common rendering sequences are not identified and deduplicated into `Templates`.

