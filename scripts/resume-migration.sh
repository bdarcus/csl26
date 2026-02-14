#!/bin/bash
# scripts/resume-migration.sh
#
# Resumes migration workflow from a checkpoint.
#
# Usage:
#   ./scripts/resume-migration.sh <style-name>

STYLE_NAME=$1

if [ -z "$STYLE_NAME" ]; then
    echo "Usage: $0 <style-name>"
    exit 1
fi

CHECKPOINT_DIR=".migration-checkpoints"
LATEST_YAML=$(ls -t "$CHECKPOINT_DIR/$STYLE_NAME"*.yaml 2>/dev/null | head -1)

if [ -z "$LATEST_YAML" ]; then
    echo "❌ No checkpoints found for $STYLE_NAME"
    exit 1
fi

echo "✅ Resuming from checkpoint: $LATEST_YAML"
cp "$LATEST_YAML" "styles/$STYLE_NAME.yaml"

echo "Next Step: Run @styleauthor to continue refinement."
echo "Validation: node scripts/oracle-migration.js styles-legacy/$STYLE_NAME.csl"
