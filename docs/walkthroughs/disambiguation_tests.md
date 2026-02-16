# Walkthrough: Implementing Disambiguation Test Coverage

This walkthrough details the steps taken to implement disambiguation test coverage from the CSL test suite into the CSLN project.

## 1. Dependency Updates
Added `csln_migrate` and `roxmltree` to `crates/csln_processor/Cargo.toml` as dev-dependencies. This allows the test suite to:
- Parse legacy CSL 1.0 XML styles (`roxmltree`, `csl_legacy`).
- Migrate them to CSLN format on-the-fly (`csln_migrate`).
- Run the CSLN Processor against the migrated style.

## 2. Test Generation Infrastructure
Created a Python script `tests/fixtures/update_disambiguation_tests.py` that:
1.  Targeted 11 representative CSL test cases for disambiguation (e.g., year-suffix, add-names).
2.  Parses the CSL test file format (mixed XML/JSON/Plaintext).
3.  **Sanitizes Input Data**: Converts string years (e.g., `"1990"`) to integers (`1990`) to satisfy `csl_legacy`'s strict type requirements.
4.  Generates a Rust integration test file `crates/csln_processor/tests/disambiguation_csl.rs`.

## 3. Generated Rust Tests
The generated file `crates/csln_processor/tests/disambiguation_csl.rs` contains:
- A helper function `compile_style_from_xml` that replicates the `csln_migrate` pipeline (inliner -> upsampler -> compressor -> compiler).
- A helper `run_test_case` that:
    - Compiles the style.
    - Loads the bibliography.
    - Runs `processor.process_citation` for each citation item in the test.
    - Asserts that the output matches the expected CSL test result.
- 11 `#[test]` functions, currently marked `#[ignore]`.

## 4. Current Status & Findings
The tests confirm significant gaps in the current implementation:
- **Year Suffix Assignment**: Suffixes reset or cycle incorrectly (`a, b, c, a, b...`) instead of being globally unique (`a..m`). This indicates disambiguation context scope issues.
- **Variable Formatting**: Attributes like `suffix="!"` on `year-suffix` are lost or ignored.
- **Name Disambiguation**: Complex add-names logic is failing or producing empty output.

Detailed analysis is available in `docs/DISAMBIGUATION.md`.

## How to Run Tests
To regenerate tests (if source files change):
```bash
PYTHONPATH=tests/fixtures python3 tests/fixtures/update_disambiguation_tests.py
```

To run the ignored tests:
```bash
cargo test --test disambiguation_csl -p csln_processor -- --ignored
```
