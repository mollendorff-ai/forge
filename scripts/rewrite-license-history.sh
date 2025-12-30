#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
# Forge License History Rewrite Script
# ═══════════════════════════════════════════════════════════════════════════════
#
# Purpose: Rewrite git history to have Elastic-2.0 LICENSE from commit #1
#          and remove LICENSE-DOCS from all commits
#
# Uses: git filter-repo (faster, safer than filter-branch)
# Install: pip install git-filter-repo  OR  brew install git-filter-repo
#
# IMPORTANT: Run this BEFORE first public push. Solo project, no external clones.
#
# ═══════════════════════════════════════════════════════════════════════════════

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  FORGE LICENSE HISTORY REWRITE${NC}"
echo -e "${CYAN}  Elastic License 2.0 | Using git filter-repo${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LICENSE_FILE="$PROJECT_ROOT/LICENSE"

cd "$PROJECT_ROOT"

# Check for git-filter-repo
if ! command -v git-filter-repo &> /dev/null; then
    echo -e "${RED}ERROR: git-filter-repo not found${NC}"
    echo ""
    echo "Install with one of:"
    echo "  pip install git-filter-repo"
    echo "  brew install git-filter-repo"
    echo ""
    exit 1
fi
echo -e "${GREEN}[OK] git-filter-repo found${NC}"

# Verify LICENSE file
if [ ! -f "$LICENSE_FILE" ]; then
    echo -e "${RED}ERROR: LICENSE file not found at $LICENSE_FILE${NC}"
    exit 1
fi

if ! grep -q "Elastic License 2.0" "$LICENSE_FILE"; then
    echo -e "${RED}ERROR: LICENSE file does not contain Elastic License 2.0${NC}"
    exit 1
fi
echo -e "${GREEN}[OK] Found Elastic-2.0 LICENSE file${NC}"

# Check for LICENSE-DOCS
if [ -f "$PROJECT_ROOT/LICENSE-DOCS" ]; then
    echo -e "${YELLOW}[!] LICENSE-DOCS exists and will be removed from history${NC}"
fi

# Show current state
echo ""
echo -e "${YELLOW}Current git state:${NC}"
COMMIT_COUNT=$(git rev-list --count HEAD)
FIRST_COMMIT=$(git rev-list --max-parents=0 HEAD)
echo "  Total commits: $COMMIT_COUNT"
echo "  First commit:  ${FIRST_COMMIT:0:8}"
echo ""

# Confirm
echo -e "${YELLOW}This script will:${NC}"
echo "  1. Remove LICENSE-DOCS from every commit in history"
echo "  2. Add LICENSE (Elastic-2.0) to every commit from the beginning"
echo "  3. Rewrite all $COMMIT_COUNT commits"
echo ""
echo -e "${RED}WARNING: This rewrites ALL commit hashes.${NC}"
echo -e "${RED}         Only run this before first public push.${NC}"
echo -e "${RED}         Ensure no external clones exist.${NC}"
echo ""
read -p "Type 'rewrite' to confirm: " CONFIRM

if [ "$CONFIRM" != "rewrite" ]; then
    echo "Aborted."
    exit 0
fi

# Create backup
echo ""
echo -e "${YELLOW}Creating backup...${NC}"
BACKUP_DIR="$PROJECT_ROOT/../forge-backup-$(date +%Y%m%d-%H%M%S)"
cp -r "$PROJECT_ROOT" "$BACKUP_DIR"
echo -e "${GREEN}[OK] Full backup created: $BACKUP_DIR${NC}"

# Create temp directory with LICENSE
TEMP_DIR=$(mktemp -d)
cp "$LICENSE_FILE" "$TEMP_DIR/LICENSE"
echo -e "${GREEN}[OK] LICENSE copied to temp: $TEMP_DIR${NC}"

# Step 1: Remove LICENSE-DOCS from history
echo ""
echo -e "${YELLOW}Step 1: Removing LICENSE-DOCS from history...${NC}"
git filter-repo --invert-paths --path LICENSE-DOCS --force

# Step 2: Add LICENSE to all commits using blob callback
echo ""
echo -e "${YELLOW}Step 2: Adding LICENSE to all commits...${NC}"

# Read LICENSE content and escape for Python
LICENSE_CONTENT=$(cat "$TEMP_DIR/LICENSE" | python3 -c "import sys; print(repr(sys.stdin.read()))")

# Create Python callback script
CALLBACK_SCRIPT="$TEMP_DIR/add_license.py"
cat > "$CALLBACK_SCRIPT" << 'PYTHON_EOF'
import sys

# LICENSE content will be inserted here
LICENSE_CONTENT = LICENSE_PLACEHOLDER

license_blob_id = None

def blob_callback(blob, callback_metadata):
    global license_blob_id
    if blob.original_id is None and license_blob_id is None:
        # This is our inserted LICENSE blob
        license_blob_id = blob.id

def commit_callback(commit, callback_metadata):
    global license_blob_id, LICENSE_CONTENT

    # Check if LICENSE already exists in this commit
    has_license = False
    for change in commit.file_changes:
        if change.filename == b'LICENSE':
            has_license = True
            break

    # If no LICENSE, add it
    if not has_license:
        from git_filter_repo import FileChange, Blob

        # Create blob for LICENSE content
        license_bytes = LICENSE_CONTENT.encode('utf-8')
        blob = Blob(license_bytes)
        callback_metadata['output'].write(blob.dump())

        # Add file change
        file_change = FileChange(b'M', b'LICENSE', blob.id, b'100644')
        commit.file_changes.append(file_change)
PYTHON_EOF

# Replace placeholder with actual content
sed -i.bak "s/LICENSE_PLACEHOLDER/$LICENSE_CONTENT/" "$CALLBACK_SCRIPT"

# Run filter-repo with callback
# Actually, filter-repo's callback system is complex. Let's use a simpler approach.

# Simpler approach: use --blob-callback to insert LICENSE
echo ""
echo -e "${YELLOW}Using simplified approach with tree-filter equivalent...${NC}"

# git filter-repo doesn't have a direct equivalent to tree-filter for adding files
# We need to use a different approach: create a commit with LICENSE and graft it

# Alternative: Use git filter-branch just for adding LICENSE (it's simpler for this case)
# Or use filter-repo's --replace-text for the blob

# Actually, the cleanest way is:
# 1. Already removed LICENSE-DOCS with filter-repo
# 2. Use filter-branch just to add LICENSE (single operation)

echo -e "${YELLOW}Adding LICENSE to all commits...${NC}"

git filter-branch --force --tree-filter "
    cp '$TEMP_DIR/LICENSE' ./LICENSE
" --tag-name-filter cat -- --all

# Cleanup
rm -rf "$TEMP_DIR"
rm -rf .git/refs/original/ 2>/dev/null || true

# Garbage collect
echo ""
echo -e "${YELLOW}Running garbage collection...${NC}"
git reflog expire --expire=now --all
git gc --prune=now --aggressive

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  HISTORY REWRITE COMPLETE${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════════════════════${NC}"
echo ""

# Verification
NEW_FIRST_COMMIT=$(git rev-list --max-parents=0 HEAD)
echo -e "${CYAN}Verification:${NC}"
echo ""

# Check LICENSE in first commit
echo -n "  LICENSE in first commit: "
if git show "$NEW_FIRST_COMMIT":LICENSE 2>/dev/null | grep -q "Elastic License 2.0"; then
    echo -e "${GREEN}YES (Elastic-2.0)${NC}"
else
    echo -e "${RED}PROBLEM - check manually${NC}"
fi

# Check LICENSE-DOCS anywhere
echo -n "  LICENSE-DOCS in history: "
if git log --all --full-history -- LICENSE-DOCS 2>/dev/null | grep -q commit; then
    echo -e "${RED}FOUND (problem!)${NC}"
else
    echo -e "${GREEN}NOT FOUND (good!)${NC}"
fi

# Count commits
NEW_COMMIT_COUNT=$(git rev-list --count HEAD)
echo "  Total commits: $NEW_COMMIT_COUNT"

echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "  1. Review: git log --oneline | head -10"
echo "  2. Verify first commit: git show ${NEW_FIRST_COMMIT:0:8}:LICENSE | head -3"
echo "  3. Force push: git push --force --all"
echo ""
echo -e "${YELLOW}Backup location: $BACKUP_DIR${NC}"
echo ""
