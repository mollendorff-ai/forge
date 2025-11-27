#!/usr/bin/env bash
# Forge Session End Checklist
# Usage: ./bin/session-end.sh
# Run before ending any development session to ensure pristine state

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Forge Session End Checklist${NC}"
echo -e "${BLUE}========================================${NC}"

FAILED=0

# Code Quality
echo -e "\n${YELLOW}[Code Quality]${NC}"

echo -n "  cargo test --release... "
if cargo test --release --quiet 2>/dev/null; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILED=1
fi

echo -n "  cargo clippy... "
if cargo clippy --release -- -D warnings 2>/dev/null; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILED=1
fi

echo -n "  cargo fmt --check... "
if cargo fmt -- --check 2>/dev/null; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILED=1
fi

echo -n "  cargo build --release... "
if cargo build --release 2>/dev/null; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILED=1
fi

# Git State
echo -e "\n${YELLOW}[Git State]${NC}"

echo -n "  Working directory clean... "
if [[ -z $(git status --porcelain) ]]; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    echo -e "    Uncommitted changes:"
    git status --short | sed 's/^/      /'
    FAILED=1
fi

echo -n "  Pushed to remote... "
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse @{u} 2>/dev/null || echo "none")
if [[ "$LOCAL" == "$REMOTE" ]]; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${YELLOW}UNPUSHED${NC}"
    echo "    Run: git push"
fi

# Documentation
echo -e "\n${YELLOW}[Documentation]${NC}"

echo -n "  CHANGELOG.md exists... "
if [[ -f CHANGELOG.md ]]; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILED=1
fi

echo -n "  README.md exists... "
if [[ -f README.md ]]; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILED=1
fi

# Summary
echo -e "\n${BLUE}========================================${NC}"
if [[ $FAILED -eq 0 ]]; then
    echo -e "${GREEN}All checks passed! Safe to end session.${NC}"
else
    echo -e "${RED}Some checks failed. Fix before ending session.${NC}"
    exit 1
fi
echo -e "${BLUE}========================================${NC}"
