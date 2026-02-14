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

# 1. Get Target Output (Oracle) - Validation Only
echo "=== PHASE 1: TARGET OUTPUT (Oracle) ==="
# We run this just to show the user/LLM the goal state, and to warm up the cache
node scripts/oracle-migration.js "$STYLE_PATH" || true
echo ""

# 2. Run Automation Pipeline
echo "=== PHASE 2: AUTOMATED MIGRATION ==="
TEMP_DIR=".tmp_migration"
mkdir -p "$TEMP_DIR"
BASE_YAML="$TEMP_DIR/base.yaml"
CITE_JSON="$TEMP_DIR/citation.json"
BIB_JSON="$TEMP_DIR/bibliography.json"

echo "-> Extracting base options (csln-migrate)..."
# Capture output to file instead of stdout? 
# csln-migrate currently prints to stdout. We need to capture it.
cargo run -q --bin csln-migrate -- "$STYLE_PATH" > "$BASE_YAML"

echo "-> Inferring citation template..."
node scripts/infer-template.js "$STYLE_PATH" --section=citation --fragment > "$CITE_JSON"

echo "-> Inferring bibliography template..."
node scripts/infer-template.js "$STYLE_PATH" --section=bibliography --fragment > "$BIB_JSON"

echo "-> Merging into CSLN style..."
node scripts/merge-migration.js "$STYLE_NAME" "$BASE_YAML" "$CITE_JSON" "$BIB_JSON"

# Cleanup
rm -rf "$TEMP_DIR"

echo "Created: styles/$STYLE_NAME.yaml"
echo ""

# 3. Generate Verification Prompt
echo "=== PHASE 3: AGENT PROMPT ==="
cat <<EOF
I have auto-generated the CSLN style file "styles/$STYLE_NAME.yaml" using the new output-driven migration workflow.

TASK:
1. Review the generated file "styles/$STYLE_NAME.yaml".
   - It combines global options extracted by Rust with templates inferred from citeproc-js output.
   - It is likely 80-90% correct but may need refinement for edge cases.

2. Verify the output:
   - Run: \`node scripts/oracle-migration.js "$STYLE_PATH"\`
   - Compare the CSLN output against the Oracle output.

3. Iterate & Fix:
   - If match rate is < 100%, analyze the mismatches.
   - Edit "styles/$STYLE_NAME.yaml" to fix formatting issues.
   - Repeat verification until passing.

4. Final Polish:
   - Ensure all lints pass: \`cargo clippy --all-targets -- -D warnings\`
   - Commit the final result.
EOF
