#!/usr/bin/env bash
# Bump CSLN schema version and validate consistency
#
# Usage: ./scripts/bump-schema.sh <new-version>
# Example: ./scripts/bump-schema.sh 1.1.0

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Check arguments
if [ $# -ne 1 ]; then
    error "Usage: $0 <new-version>"
    error "Example: $0 1.1.0"
    exit 1
fi

NEW_VERSION="$1"

# Validate version format (SemVer)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    error "Invalid version format: $NEW_VERSION"
    error "Expected SemVer format: MAJOR.MINOR.PATCH (e.g., 1.1.0)"
    exit 1
fi

# Paths
CORE_LIB="crates/csln_core/src/lib.rs"
SCHEMA_DOC="docs/SCHEMA_VERSIONING.md"
STYLES_DIR="styles"

info "Bumping schema version to $NEW_VERSION"

# Step 1: Update default_version() in csln_core/src/lib.rs
info "Updating default_version() in $CORE_LIB"

if ! grep -q 'fn default_version()' "$CORE_LIB"; then
    error "Could not find default_version() function in $CORE_LIB"
    exit 1
fi

# Extract current version
CURRENT_VERSION=$(grep -A1 'fn default_version()' "$CORE_LIB" | grep -o '"[^"]*"' | tr -d '"')
info "Current schema version: $CURRENT_VERSION"

# Update the version
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS sed syntax
    sed -i '' "s/fn default_version() -> String {/fn default_version() -> String {/; /fn default_version() -> String {/,/}/ s/\"$CURRENT_VERSION\"/\"$NEW_VERSION\"/" "$CORE_LIB"
else
    # GNU sed syntax
    sed -i "s/fn default_version() -> String {/fn default_version() -> String {/; /fn default_version() -> String {/,/}/ s/\"$CURRENT_VERSION\"/\"$NEW_VERSION\"/" "$CORE_LIB"
fi

success "Updated default_version() to $NEW_VERSION"

# Step 2: Validate all styles parse correctly
info "Validating all styles in $STYLES_DIR"

if ! cargo test --quiet --lib 2>&1 | grep -q "test result: ok"; then
    error "Style validation failed. Run 'cargo test' for details."
    error "Reverting changes..."
    git checkout "$CORE_LIB"
    exit 1
fi

success "All styles parse correctly with schema $NEW_VERSION"

# Step 3: Update schema changelog in SCHEMA_VERSIONING.md
info "Updating schema changelog in $SCHEMA_DOC"

TIMESTAMP=$(date +%Y-%m-%d)
CHANGELOG_ENTRY="#### schema-v${NEW_VERSION} (${TIMESTAMP})\n- Schema version bumped to ${NEW_VERSION}\n"

# Insert changelog entry after "### Schema Changelog" header
if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "/### Schema Changelog/a\\
\\
$CHANGELOG_ENTRY" "$SCHEMA_DOC"
else
    sed -i "/### Schema Changelog/a\\$CHANGELOG_ENTRY" "$SCHEMA_DOC"
fi

success "Updated schema changelog"

# Step 4: Show diff
info "Review changes:"
git diff "$CORE_LIB" "$SCHEMA_DOC"

# Step 5: Prompt for commit
echo ""
read -p "Commit these changes? (y/N) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    git add "$CORE_LIB" "$SCHEMA_DOC"
    git commit -m "chore(schema): bump schema version to $NEW_VERSION

Schema version updated from $CURRENT_VERSION to $NEW_VERSION.
All styles validated successfully."

    success "Changes committed"

    # Step 6: Create git tag
    TAG_NAME="schema-v${NEW_VERSION}"
    info "Creating git tag: $TAG_NAME"

    git tag -a "$TAG_NAME" -m "Schema version $NEW_VERSION"
    success "Tag created: $TAG_NAME"

    echo ""
    info "Schema version bump complete!"
    info "Next steps:"
    echo "  1. Review the commit and tag"
    echo "  2. Push to remote: git push && git push --tags"
    echo "  3. Create GitHub Release for $TAG_NAME"
else
    warn "Changes not committed. Run 'git checkout $CORE_LIB $SCHEMA_DOC' to revert."
fi
