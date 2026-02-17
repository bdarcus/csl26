#!/bin/bash
# scripts/bench-check.sh: Run before/after comparison for performance changes.

# Exit on error
set -e

# Usage: ./scripts/bench-check.sh [baseline-name] [current-name]
# Example: ./scripts/bench-check.sh baseline

if [ "$#" -eq 1 ]; then
    TARGET_FILE=".bench-baselines/$1.txt"
    echo "--- Gathering Benchmarks for '$1' ---"
    echo "Results will be saved to $TARGET_FILE"
    echo "This may take a minute..."
    
    # Run and show minimal progress but save full output
    cargo bench --bench rendering > "$TARGET_FILE"
    cargo bench --bench formats >> "$TARGET_FILE"
    
    echo "Done. Captured $1."
    exit 0
fi

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
echo "Comparing against $BASELINE_NAME..."
cargo bench --bench rendering > "$CURRENT_FILE"
cargo bench --bench formats >> "$CURRENT_FILE"

if [ ! -f "$BASELINE_FILE" ]; then
    echo "Error: No baseline file found at $BASELINE_FILE"
    echo "To capture one first, run: ./scripts/bench-check.sh $BASELINE_NAME"
    exit 1
fi

echo ""
echo "--- Performance Delta ($BASELINE_NAME vs $CURRENT_NAME) ---"
critcmp "$BASELINE_FILE" "$CURRENT_FILE"
