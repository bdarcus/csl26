#!/usr/bin/env bash
# Validate all CSLN style files parse correctly with current schema version
#
# Usage: ./scripts/validate-schema.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

STYLES_DIR="styles"
CORE_LIB="crates/csln_core/src/lib.rs"

# Extract current schema version
SCHEMA_VERSION=$(grep -A1 'fn default_version()' "$CORE_LIB" | grep -o '"[^"]*"' | tr -d '"')

info "Current schema version: $SCHEMA_VERSION"
info "Validating all styles in $STYLES_DIR"

# Count style files
STYLE_COUNT=$(find "$STYLES_DIR" -name "*.yaml" | wc -l | tr -d ' ')
info "Found $STYLE_COUNT style files"

# Run library tests (includes style parsing tests)
if cargo test --quiet --lib 2>&1 | grep -q "test result: ok"; then
    success "All $STYLE_COUNT styles parse correctly with schema $SCHEMA_VERSION"
    exit 0
else
    error "Style validation failed"
    error "Run 'cargo test --lib' for detailed error messages"
    exit 1
fi
