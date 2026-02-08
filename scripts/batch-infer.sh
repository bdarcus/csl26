#!/bin/bash
# Batch template inference: runs infer-template.js --fragment for parent styles
# and caches results to templates/inferred/{style-name}.json.
#
# Usage:
#   ./scripts/batch-infer.sh                    # All parent styles
#   ./scripts/batch-infer.sh --top 10           # Top 10 by dependent count
#   ./scripts/batch-infer.sh --styles "apa elsevier-harvard"  # Specific styles

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(dirname "$SCRIPT_DIR")"
STYLES_DIR="$WORKSPACE_ROOT/styles"
CACHE_DIR="$WORKSPACE_ROOT/templates/inferred"
INFERRER="$SCRIPT_DIR/infer-template.js"

# Top parent styles by dependent count (from STYLE_PRIORITY.md)
TOP_PARENTS=(
    apa
    elsevier-with-titles
    elsevier-harvard
    springer-basic-author-date
    ieee
    american-medical-association
    vancouver-superscript
    chicago-author-date
    harvard-cite-them-right
    taylor-and-francis-national-library-of-medicine
)

# Parse arguments
TOP_N=0
SPECIFIC_STYLES=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --top)
            TOP_N="$2"
            shift 2
            ;;
        --styles)
            SPECIFIC_STYLES="$2"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--top N] [--styles \"style1 style2\"]" >&2
            exit 1
            ;;
    esac
done

# Ensure cache directory exists
mkdir -p "$CACHE_DIR"

# Build style list
if [[ -n "$SPECIFIC_STYLES" ]]; then
    IFS=' ' read -ra STYLES <<< "$SPECIFIC_STYLES"
elif [[ "$TOP_N" -gt 0 ]]; then
    STYLES=("${TOP_PARENTS[@]:0:$TOP_N}")
else
    # All parent styles (files directly in styles/ that aren't in dependent/)
    STYLES=()
    for f in "$STYLES_DIR"/*.csl; do
        name="$(basename "$f" .csl)"
        STYLES+=("$name")
    done
fi

# Run inference
SUCCESS=0
FAIL=0
SKIP=0
TOTAL=${#STYLES[@]}

echo "Batch inference: $TOTAL styles → $CACHE_DIR"
echo ""

for style_name in "${STYLES[@]}"; do
    style_path="$STYLES_DIR/$style_name.csl"

    if [[ ! -f "$style_path" ]]; then
        echo "  SKIP  $style_name (file not found)"
        SKIP=$((SKIP + 1))
        continue
    fi

    cache_path="$CACHE_DIR/$style_name.json"
    if [[ -f "$cache_path" ]]; then
        echo "  CACHE $style_name"
        SKIP=$((SKIP + 1))
        continue
    fi

    if output=$(node "$INFERRER" "$style_path" --fragment 2>/dev/null); then
        echo "$output" > "$cache_path"
        echo "  OK    $style_name"
        SUCCESS=$((SUCCESS + 1))
    else
        echo "  FAIL  $style_name"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "Done: $SUCCESS inferred, $SKIP skipped, $FAIL failed (of $TOTAL)"
