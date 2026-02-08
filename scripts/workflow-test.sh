#!/bin/bash
#
# Workflow Test Script
#
# Runs structured oracle comparison followed by batch analysis
# to show both detailed failures and broader impact across styles.
#
# Usage:
#   ./scripts/workflow-test.sh styles/apa.csl
#   ./scripts/workflow-test.sh styles/apa.csl --json
#   ./scripts/workflow-test.sh styles/apa.csl --top 20
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STYLE_PATH="$1"
BATCH_COUNT="${BATCH_COUNT:-10}"
JSON_OUTPUT=false
VERBOSE=false

# Parse arguments
shift
while [[ $# -gt 0 ]]; do
  case $1 in
    --json)
      JSON_OUTPUT=true
      shift
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --top)
      BATCH_COUNT="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: workflow-test.sh <style.csl> [--json] [--verbose] [--top N]"
      exit 1
      ;;
  esac
done

if [ -z "$STYLE_PATH" ]; then
  echo "Usage: workflow-test.sh <style.csl> [--json] [--verbose] [--top N]"
  exit 1
fi

STYLE_NAME=$(basename "$STYLE_PATH" .csl)

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "CSLN Workflow Test: $STYLE_NAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Step 1: Run structured oracle for this style
echo "┌─ Step 1: Structured Oracle Analysis"
echo "│"

if [ "$JSON_OUTPUT" = true ]; then
  node "$SCRIPT_DIR/oracle.js" "$STYLE_PATH" --json
else
  if [ "$VERBOSE" = true ]; then
    node "$SCRIPT_DIR/oracle.js" "$STYLE_PATH" --verbose
  else
    node "$SCRIPT_DIR/oracle.js" "$STYLE_PATH"
  fi
fi

echo "│"
echo "└─ Structured oracle complete"
echo

# Step 2: Run batch analysis to show broader impact
echo "┌─ Step 2: Batch Impact Analysis (Top $BATCH_COUNT Styles)"
echo "│"

if [ "$JSON_OUTPUT" = true ]; then
  node "$SCRIPT_DIR/oracle-batch-aggregate.js" styles-legacy/ --top "$BATCH_COUNT" --json
else
  echo "│  Running batch analysis across top $BATCH_COUNT priority styles..."
  echo "│  This shows whether issues are specific to $STYLE_NAME or systemic."
  echo "│"
  node "$SCRIPT_DIR/oracle-batch-aggregate.js" styles-legacy/ --top "$BATCH_COUNT"
fi

echo "│"
echo "└─ Batch analysis complete"
echo

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Workflow test complete!"
echo
echo "Next steps:"
echo "  - Review component failures in structured oracle output"
echo "  - Check if issues appear in batch analysis (systemic)"
echo "  - Fix high-impact issues affecting multiple styles first"
echo "  - Re-run this script to verify fixes"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
