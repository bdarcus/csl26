#!/bin/bash
# scripts/prep-migration.sh
# Preparation script for @styleauthor migration workflow

STYLE_PATH=$1

if [ "$STYLE_PATH" == "--help" ] || [ -z "$STYLE_PATH" ]; then
    echo "Usage: $0 <path-to-legacy-csl>"
    echo ""
    echo "Prepares for @styleauthor migration by generating:"
    echo "1. Target rendering (citeproc-js)"
    echo "2. Baseline CSLN config (csln-migrate)"
    echo "3. Agent-ready prompt"
    exit 0
fi

STYLE_NAME=$(basename "$STYLE_PATH" .csl)

echo "--- MIGRATION PREPARATION FOR: $STYLE_NAME ---"
echo ""

# 1. Get Target Output (Oracle)
echo "=== PHASE 1: TARGET OUTPUT (citeproc-js) ==="
node scripts/oracle-simple.js "$STYLE_PATH"
echo ""

# 2. Get Baseline CSLN (Migration Tool)
echo "=== PHASE 2: BASELINE CSLN (Migration Tool) ==="
cargo run -q --bin csln-migrate -- "$STYLE_PATH"
echo ""

# 3. Generate Agent Prompt
echo "=== PHASE 3: AGENT PROMPT ==="
cat <<EOF
I am migrating the CSL 1.0 style "$STYLE_NAME" to CSLN.

TARGET RENDERING:
Reference the "PHASE 1" output above for exactly how citations and bibliography entries should look.

BASELINE OPTIONS:
Reference the "PHASE 2" output above for the initial extraction of global options (contributors, dates, etc.).

TASK:
1. Use the /styleauthor skill to create "styles/$STYLE_NAME.yaml".
2. Use the "Baseline CSLN" for the 'options' block.
3. Hand-author the 'template' blocks in 'citation' and 'bibliography' to match the "Target Rendering".
4. Follow the iterative author-test-verify loop until 100% match is achieved.
EOF
