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
info "Validating production styles in $STYLES_DIR (excluding experimental)"

# Validate only top-level styles/*.yaml; experimental drafts live under styles/experimental/
STYLE_FILES=("$STYLES_DIR"/*.yaml)
STYLE_COUNT=${#STYLE_FILES[@]}
info "Found $STYLE_COUNT production style files"

FAILED=0
for style in "${STYLE_FILES[@]}"; do
    if ! cargo run --quiet --bin csln -- check -s "$style" >/dev/null 2>&1; then
        error "Style failed schema validation: $style"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    success "All $STYLE_COUNT production styles parse correctly with schema $SCHEMA_VERSION"
    exit 0
fi

error "Style validation failed"
exit 1
