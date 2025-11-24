#!/usr/bin/env bash
# =============================================================================
# PlantUML Diagram Validation Script
# =============================================================================
# Purpose: Validate .puml/.plantuml files compile without errors
# Server: https://www.plantuml.com/plantuml (public server)
# Usage: ./bin/validate-plantuml.sh [directory]
#
# Exit codes:
#   0 - All diagrams valid
#   1 - Validation failed (syntax errors or server unreachable)
# =============================================================================

set -e

# Configuration
PLANTUML_SERVER="https://www.plantuml.com/plantuml"
DIAGRAMS_DIR="${1:-diagrams}"
TIMEOUT=10

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🎨 Validating PlantUML diagrams..."
echo ""

# Check if diagrams directory exists
if [ ! -d "$DIAGRAMS_DIR" ]; then
    echo -e "${YELLOW}ℹ️  No diagrams directory found (skipping)${NC}"
    exit 0
fi

# Find all PlantUML files
PUML_FILES=$(find "$DIAGRAMS_DIR" -type f \( -name "*.puml" -o -name "*.plantuml" \) 2>/dev/null)

if [ -z "$PUML_FILES" ]; then
    echo -e "${YELLOW}ℹ️  No PlantUML files found in $DIAGRAMS_DIR (skipping)${NC}"
    exit 0
fi

# Count files
FILE_COUNT=$(echo "$PUML_FILES" | wc -l)
echo "Found $FILE_COUNT PlantUML file(s) to validate"
echo ""

# Check if PlantUML server is accessible
echo "🌐 Checking PlantUML server accessibility..."
if ! curl -s --head --max-time "$TIMEOUT" "$PLANTUML_SERVER/png/" >/dev/null 2>&1; then
    echo -e "${RED}❌ PlantUML server unreachable: $PLANTUML_SERVER${NC}"
    echo "   Please check your internet connection or try again later"
    exit 1
fi
echo -e "${GREEN}✅ PlantUML server accessible${NC}"
echo ""

# Validate each file
FAILED_FILES=()
PASSED_COUNT=0

while IFS= read -r file; do
    echo "📄 Validating: $file"

    # Encode diagram to base64 (PlantUML text encoding format)
    # Use the /png/ endpoint to just check if it compiles
    ENCODED=$(cat "$file" | base64 -w 0)

    # Send to PlantUML server and check response
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time "$TIMEOUT" \
        -X POST \
        -H "Content-Type: text/plain" \
        --data-binary "@$file" \
        "$PLANTUML_SERVER/png/")

    if [ "$HTTP_CODE" -eq 200 ]; then
        echo -e "   ${GREEN}✅ Valid${NC}"
        ((PASSED_COUNT++))
    else
        echo -e "   ${RED}❌ Failed (HTTP $HTTP_CODE)${NC}"
        FAILED_FILES+=("$file")
    fi
    echo ""
done <<< "$PUML_FILES"

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ ${#FAILED_FILES[@]} -eq 0 ]; then
    echo -e "${GREEN}✅ All $FILE_COUNT diagram(s) validated successfully!${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    echo -e "${RED}❌ Validation failed for ${#FAILED_FILES[@]} file(s):${NC}"
    for file in "${FAILED_FILES[@]}"; do
        echo "   - $file"
    done
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
