#!/bin/bash
# scripts/lint-rendering.sh
# Flag common CSLN rendering errors (double spaces, punctuation glitches)

STYLE_FILE=$1
if [ -z "$STYLE_FILE" ]; then
    echo "Usage: $0 <style-yaml>"
    exit 1
fi

# Run the processor and capture output
OUTPUT=$(cargo run --bin csln-processor -- "$STYLE_FILE" 2>/dev/null)

echo "--- Rendering Lint Report for $(basename "$STYLE_FILE") ---"
ERRORS=0

# 1. Double Spaces (only within rendered content, ignoring ID prefix and leading indent)
# We use sed to strip leading indentation "  " and citation IDs like "[ITEM-1] "
CONTENT_ONLY=$(echo "$OUTPUT" | sed -E 's/^  (\[[^]]+\] )?//')
if echo "$CONTENT_ONLY" | grep -q "  "; then
    echo "❌ FAIL: Double spaces detected within content."
    echo "$CONTENT_ONLY" | grep --color=always "  "
    ERRORS=$((ERRORS + 1))
fi

# 2. Space before punctuation
if echo "$OUTPUT" | grep -qE " ([:;,\.\?\!])"; then
    echo "❌ FAIL: Space before punctuation detected."
    echo "$OUTPUT" | grep --color=always -E " ([:;,\.\?\!])"
    ERRORS=$((ERRORS + 1))
fi

# 3. Clashing punctuation (e.g. .., ,.)
# Note: we allow "et al." by checking if it's preceded by something other than "al"
if echo "$OUTPUT" | grep -v "et al" | grep -qE "(\.\.|\.,|,\.|\. \.)"; then
    echo "❌ FAIL: Clashing punctuation detected."
    echo "$OUTPUT" | grep -v "et al" | grep --color=always -E "(\.\.|\.,|,\.|\. \.)"
    ERRORS=$((ERRORS + 1))
fi

if [ $ERRORS -eq 0 ]; then
    echo "✅ PASS: No common formatting glitches detected."
    exit 0
else
    echo "--- Found $ERRORS formatting issues ---"
    exit 1
fi
