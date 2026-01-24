#!/usr/bin/env bash
# Forge Release Script
# Usage: ./bin/release.sh <version> "<title>"
# Example: ./bin/release.sh 4.1.0 "Cross-file References"

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION="${1:-}"
TITLE="${2:-}"

if [[ -z "$VERSION" ]]; then
    echo -e "${RED}Error: Version required${NC}"
    echo "Usage: ./bin/release.sh <version> \"<title>\""
    echo "Example: ./bin/release.sh 4.1.0 \"Cross-file References\""
    exit 1
fi

if [[ -z "$TITLE" ]]; then
    echo -e "${RED}Error: Title required${NC}"
    echo "Usage: ./bin/release.sh <version> \"<title>\""
    exit 1
fi

BINARY_NAME="forge-${VERSION}-linux-x86_64"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Forge Release v${VERSION}: ${TITLE}${NC}"
echo -e "${BLUE}========================================${NC}"

# Step 1: Pre-flight checks
echo -e "\n${YELLOW}[1/8] Pre-flight checks...${NC}"
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${RED}Error: Working directory not clean. Commit or stash changes first.${NC}"
    git status --short
    exit 1
fi
echo -e "${GREEN}  Working directory clean${NC}"

# Step 2: Run tests
echo -e "\n${YELLOW}[2/8] Running tests...${NC}"
cargo test --release
echo -e "${GREEN}  All tests passed${NC}"

# Step 3: Clippy
echo -e "\n${YELLOW}[3/8] Running clippy...${NC}"
cargo clippy --release -- -D warnings
echo -e "${GREEN}  Zero warnings${NC}"

# Step 4: Build release
echo -e "\n${YELLOW}[4/8] Building release...${NC}"
cargo build --release
echo -e "${GREEN}  Build complete${NC}"

# Step 5: UPX compress
echo -e "\n${YELLOW}[5/8] UPX compressing...${NC}"
ORIGINAL_SIZE=$(ls -lh target/release/forge | awk '{print $5}')
upx --best --lzma target/release/forge -o "/tmp/${BINARY_NAME}" --force
COMPRESSED_SIZE=$(ls -lh "/tmp/${BINARY_NAME}" | awk '{print $5}')
echo -e "${GREEN}  Compressed: ${ORIGINAL_SIZE} -> ${COMPRESSED_SIZE}${NC}"

# Step 6: Install to /usr/local/bin
echo -e "\n${YELLOW}[6/8] Installing to /usr/local/bin...${NC}"
sudo cp "/tmp/${BINARY_NAME}" /usr/local/bin/forge
sudo chmod +x /usr/local/bin/forge
INSTALLED_VERSION=$(forge --version)
echo -e "${GREEN}  Installed: ${INSTALLED_VERSION}${NC}"

# Step 7: Create git tag
echo -e "\n${YELLOW}[7/8] Creating git tag...${NC}"
if git tag -l | grep -q "^v${VERSION}$"; then
    echo -e "${YELLOW}  Tag v${VERSION} already exists, skipping...${NC}"
else
    git tag -a "v${VERSION}" -m "v${VERSION}: ${TITLE}"
    git push origin "v${VERSION}"
    echo -e "${GREEN}  Tag v${VERSION} created and pushed${NC}"
fi

# Step 8: Create GitHub release
echo -e "\n${YELLOW}[8/8] Creating GitHub release...${NC}"
if gh release view "v${VERSION}" &>/dev/null; then
    echo -e "${YELLOW}  Release v${VERSION} already exists, skipping...${NC}"
else
    gh release create "v${VERSION}" "/tmp/${BINARY_NAME}" \
        --title "v${VERSION}: ${TITLE}" \
        --notes "$(cat <<EOF
## v${VERSION}: ${TITLE}

See [CHANGELOG.md](CHANGELOG.md) for full details.

### Binary

- \`${BINARY_NAME}\` - UPX compressed (${COMPRESSED_SIZE})

### Install

\`\`\`bash
# Download binary
curl -L https://github.com/mollendorff-ai/forge/releases/download/v${VERSION}/${BINARY_NAME} -o forge
chmod +x forge
sudo mv forge /usr/local/bin/

# Or build from source
git clone https://github.com/mollendorff-ai/forge
cd forge && cargo install --path .
\`\`\`

Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
    echo -e "${GREEN}  GitHub release created${NC}"
fi

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}Release v${VERSION} complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Checklist:"
echo "  [ ] Update CHANGELOG.md if not done"
echo "  [ ] Update README.md version table"
echo "  [ ] Update roadmap.yaml current version"
echo "  [ ] Commit and push doc changes"
echo ""
echo "Links:"
echo "  - GitHub: https://github.com/mollendorff-ai/forge/releases/tag/v${VERSION}"
