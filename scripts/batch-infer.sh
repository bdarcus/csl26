#!/bin/bash
# Batch template inference: runs infer-template.js for both citation and
# bibliography sections, then caches section-specific fragments for Rust.
#
# Usage:
#   ./scripts/batch-infer.sh                    # All parent styles
#   ./scripts/batch-infer.sh --top 10           # Top 10 by dependent count
#   ./scripts/batch-infer.sh --styles "apa elsevier-harvard"  # Specific styles
#   ./scripts/batch-infer.sh --force            # Regenerate existing cache

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(dirname "$SCRIPT_DIR")"
STYLES_DIR="$WORKSPACE_ROOT/styles-legacy"
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
FORCE=false

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
        --force)
            FORCE=true
            shift
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--top N] [--styles \"style1 style2\"] [--force]" >&2
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
    # All parent styles (files directly in styles-legacy/ that aren't in dependent/)
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

    bib_cache_path="$CACHE_DIR/$style_name.bibliography.json"
    cit_cache_path="$CACHE_DIR/$style_name.citation.json"
    if [[ "$FORCE" = false && -f "$bib_cache_path" && -f "$cit_cache_path" ]]; then
        echo "  CACHE $style_name"
        SKIP=$((SKIP + 1))
        continue
    fi

    if bib_output=$(node "$INFERRER" "$style_path" --section=bibliography --fragment 2>/dev/null) \
      && cit_output=$(node "$INFERRER" "$style_path" --section=citation --fragment 2>/dev/null); then
        echo "$bib_output" > "$bib_cache_path"
        echo "$cit_output" > "$cit_cache_path"
        echo "  OK    $style_name"
        SUCCESS=$((SUCCESS + 1))
    else
        echo "  FAIL  $style_name"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "Done: $SUCCESS inferred, $SKIP skipped, $FAIL failed (of $TOTAL)"
