#!/bin/bash
# scripts/prep-migration.sh
# Preparation script for @styleauthor migration workflow

STYLE_PATH=$1
AGENT_MODE=false

if [[ "$*" == *"--agent"* ]]; then
    AGENT_MODE=true
fi

if [ "$STYLE_PATH" == "--help" ] || [ -z "$STYLE_PATH" ]; then
    echo "Usage: $0 <path-to-legacy-csl> [--agent]"
    echo ""
    echo "Prepares for @styleauthor migration by generating:"
    echo "1. Target rendering (citeproc-js)"
    echo "2. Baseline CSLN config (csln-migrate)"
    echo "3. Agent-ready prompt (or JSON with --agent)"
    exit 0
fi

STYLE_NAME=$(basename "$STYLE_PATH" .csl)

if [ "$AGENT_MODE" = false ]; then
    echo "--- 🚀 MIGRATION PREPARATION FOR: $STYLE_NAME ---"
    echo ""
fi

if [ "$AGENT_MODE" = false ]; then
    # === BEANS TASK TRACKING ===
    if command -v beans &> /dev/null; then
        echo "📦 Creating beans task for migration tracking..."
        TASK_OUTPUT=$(beans create "Migrate: $STYLE_NAME" \
            --type feature \
            --priority normal \
            --body "Auto-created by prep-migration.sh

Style: $STYLE_PATH
Workflow: Hybrid migration (Tier 1 options + Tier 3 templates)

Progress:
- [ ] Phase 1: Generate baseline (csln-migrate)
- [ ] Phase 2: Infer templates (output-driven)
- [ ] Phase 3: Merge and validate
- [ ] Phase 4: Agent refinement
- [ ] Phase 5: Final verification
" \
            --status in-progress 2>&1)

        TASK_ID=$(echo "$TASK_OUTPUT" | grep -oE 'csl26-[a-z0-9]{4}' | head -1)

        if [ -n "$TASK_ID" ]; then
            echo "✅ Created task: $TASK_ID"
            echo "   Track: /beans show $TASK_ID"
            echo "   Update: /beans update $TASK_ID --status <STATUS>"
            echo ""

            # Store for later use
            echo "$TASK_ID" > ".migration-task-$STYLE_NAME.txt"
        else
            echo "⚠️  Failed to create beans task (continuing anyway)"
        fi
    else
        echo "⚠️  beans CLI not found - skipping task tracking"
    fi
    echo ""
fi

# 1. Run Automation Pipeline
if [ "$AGENT_MODE" = false ]; then
    echo "=== 🏗️  PHASE 1: AUTOMATED MIGRATION ==="
fi

TEMP_DIR=".tmp_migration"
mkdir -p "$TEMP_DIR"
BASE_YAML="$TEMP_DIR/base.yaml"
CITE_JSON="$TEMP_DIR/citation.json"
BIB_JSON="$TEMP_DIR/bibliography.json"

if [ "$AGENT_MODE" = false ]; then echo "-> Extracting base options (csln-migrate)..."; fi
cargo run -q --bin csln-migrate -- "$STYLE_PATH" > "$BASE_YAML"

if [ "$AGENT_MODE" = false ]; then echo "-> Inferring citation template..."; fi
node scripts/infer-template.js "$STYLE_PATH" --section=citation --fragment > "$CITE_JSON"

if [ "$AGENT_MODE" = false ]; then echo "-> Inferring bibliography template..."; fi
node scripts/infer-template.js "$STYLE_PATH" --section=bibliography --fragment > "$BIB_JSON"

if [ "$AGENT_MODE" = false ]; then echo "-> Merging into CSLN style..."; fi
node scripts/merge-migration.js "$STYLE_NAME" "$BASE_YAML" "$CITE_JSON" "$BIB_JSON" > /dev/null

# Phase 2: Coverage & Validation
if [ "$AGENT_MODE" = false ]; then
    node scripts/check-coverage.js "$STYLE_NAME"
    node scripts/validate-migration.js "styles/$STYLE_NAME.yaml"
    echo ""
fi

# Cleanup
rm -rf "$TEMP_DIR"

if [ "$AGENT_MODE" = false ]; then
    echo "✨ Created: styles/$STYLE_NAME.yaml"
    echo ""
fi

# === UPDATE BEANS TASK ===
TASK_FILE=".migration-task-$STYLE_NAME.txt"
if [ -f "$TASK_FILE" ]; then
    TASK_ID=$(cat "$TASK_FILE")
    beans update "$TASK_ID" --body "Migration prep completed ✅

Style: styles/$STYLE_NAME.yaml
Next: Agent refinement (Phase 4)

Auto-generated baseline:
- Options: csln-migrate (Rust)
- Templates: infer-template.js (output-driven)

Validation: Run \`node scripts/oracle-migration.js $STYLE_PATH\`
" 2>/dev/null

    rm "$TASK_FILE"  # Cleanup temp file
fi

# 2. Generate Verification Prompt
if [ "$AGENT_MODE" = true ]; then
    # Output machine-readable JSON
    ORACLE_OUTPUT=$(node scripts/oracle-migration.js "$STYLE_PATH" --json 2>/dev/null || echo "{}")
    cat <<EOF
{
  "action": "migrate",
  "style": "$STYLE_NAME",
  "path": "styles/$STYLE_NAME.yaml",
  "legacy_path": "$STYLE_PATH",
  "context": {
    "oracle_results": $ORACLE_OUTPUT
  },
  "recommended_path": "simple"
}
EOF
else
    echo "=== 📝 PHASE 2: AGENT PROMPT ==="
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
fi
