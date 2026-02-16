#!/usr/bin/env bash
# Convert CSL Test Suite disambiguation tests to CSLN format
#
# Usage: ./convert_csl_tests.sh [test_category]
#
# Categories:
#   - disambiguate_YearSuffix
#   - disambiguate_AddNames
#   - disambiguate_AddGivenname
#   - disambiguate_Combined

set -euo pipefail

CSL_TEST_DIR="../../tests/csl-test-suite/processor-tests/humans"
OUTPUT_DIR="../disambiguation"

# TODO: Parse CSL YAML test files
# TODO: Extract input items (bibliography entries)
# TODO: Extract expected output
# TODO: Convert to CSLN test format (JSON fixtures + assertions)
# TODO: Write to disambiguation/*.rs test files

echo "CSL test converter scaffold created"
echo "Next steps:"
echo "1. Implement YAML parsing for CSL test format"
echo "2. Map CSL input items to references-expanded.json format"
echo "3. Generate Rust test case templates"
