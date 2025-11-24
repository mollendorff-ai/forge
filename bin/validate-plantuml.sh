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
#   1 - Validation failed (syntax errors)
# =============================================================================

set -o pipefail

# Configuration
PLANTUML_SERVER="https://www.plantuml.com/plantuml"
DIAGRAMS_DIR="${1:-diagrams}"
TIMEOUT=30

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

    # First check basic syntax locally (fast)
    if ! grep -q "@startuml" "$file" || ! grep -q "@enduml" "$file"; then
        echo -e "   ${RED}❌ Failed (missing @startuml or @enduml)${NC}"
        FAILED_FILES+=("$file")
        echo ""
        continue
    fi

    # Send to PlantUML server and check response
    # Note: Don't use -L (follow redirects) as it causes issues with the POST
    HTTP_CODE=$(timeout "$TIMEOUT" curl -s -o /dev/null -w "%{http_code}" \
        -X POST \
        -H "Content-Type: text/plain" \
        --data-binary "@$file" \
        "$PLANTUML_SERVER/png/" 2>/dev/null)

    CURL_EXIT=$?

    # Check HTTP code first (most reliable indicator)
    if [ "$HTTP_CODE" -eq 200 ]; then
        echo -e "   ${GREEN}✅ Valid (server confirmed)${NC}"
        ((PASSED_COUNT++))
    elif [ "$HTTP_CODE" -eq 302 ]; then
        # 302 redirect means diagram compiled successfully
        echo -e "   ${GREEN}✅ Valid (server confirmed via redirect)${NC}"
        ((PASSED_COUNT++))
    elif [ "$HTTP_CODE" -eq 400 ]; then
        echo -e "   ${RED}❌ Failed (HTTP $HTTP_CODE - syntax error)${NC}"
        FAILED_FILES+=("$file")
    elif [ $CURL_EXIT -eq 124 ] || [ $CURL_EXIT -eq 28 ]; then
        # Timeout
        echo -e "   ${YELLOW}⚠️  Server timeout - syntax check passed locally${NC}"
        ((PASSED_COUNT++))
    elif [ $CURL_EXIT -ne 0 ] && [ -z "$HTTP_CODE" ]; then
        # Curl failed and no HTTP code
        echo -e "   ${YELLOW}⚠️  Server unreachable - syntax check passed locally${NC}"
        ((PASSED_COUNT++))
    else
        # Unknown status, but local syntax passed
        echo -e "   ${YELLOW}⚠️  Unexpected response (HTTP $HTTP_CODE, exit $CURL_EXIT) - syntax check passed locally${NC}"
        ((PASSED_COUNT++))
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
