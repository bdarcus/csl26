#!/bin/bash
# scripts/bench-check.sh: Run before/after comparison for performance changes.

# Exit on error
set -e

# Usage: ./scripts/bench-check.sh [baseline-name] [current-name]
# Example: ./scripts/bench-check.sh main refactor

BASELINE_NAME=${1:-"baseline"}
CURRENT_NAME=${2:-"current"}
BASELINE_FILE=".bench-baselines/$BASELINE_NAME.txt"
CURRENT_FILE=".bench-baselines/$CURRENT_NAME.txt"

# Ensure critcmp is installed
if ! command -v critcmp &> /dev/null; then
    echo "Error: 'critcmp' not found. Please install it: cargo install critcmp"
    exit 1
fi

echo "--- Benchmarking Current State ($CURRENT_NAME) ---"
# Run rendering benchmarks (processor hot path)
# We use --output to redirect stdout to a file for critcmp
cargo bench --bench rendering > "$CURRENT_FILE" 2>/dev/null
cargo bench --bench formats >> "$CURRENT_FILE" 2>/dev/null

if [ ! -f "$BASELINE_FILE" ]; then
    echo "Warning: No baseline file found at $BASELINE_FILE"
    echo "To set a baseline, run: ./scripts/bench-check.sh $BASELINE_NAME"
    echo "Or rename your current run: mv $CURRENT_FILE $BASELINE_FILE"
    exit 0
fi

echo "--- Performance Delta ($BASELINE_NAME vs $CURRENT_NAME) ---"
critcmp "$BASELINE_FILE" "$CURRENT_FILE"
