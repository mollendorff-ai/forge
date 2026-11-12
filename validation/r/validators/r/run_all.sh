#!/bin/bash
# run_all.sh - Execute all R validators for forge-e2e
# Part of v0.4.0: R validator scripts and CI/CD workflow

set -e  # Exit on error

# ANSI color codes
GREEN='\033[32m'
RED='\033[31m'
YELLOW='\033[33m'
BLUE='\033[34m'
BOLD='\033[1m'
RESET='\033[0m'

# Get script directory (resolves symlinks)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Print header
echo -e "\n${BOLD}${BLUE}========================================${RESET}"
echo -e "${BOLD}${BLUE}  forge-e2e R Validators v0.10.0${RESET}"
echo -e "${BOLD}${BLUE}========================================${RESET}\n"

# Check if R is installed
if ! command -v R &> /dev/null; then
    echo -e "${RED}[ERROR]${RESET} R is not installed"
    echo -e "${YELLOW}Install R:${RESET}"
    echo -e "  macOS:        brew install r"
    echo -e "  Ubuntu/Debian: sudo apt install r-base"
    echo -e "  Fedora/RHEL:   sudo dnf install R"
    exit 1
fi

echo -e "${GREEN}[OK]${RESET} R is installed: $(R --version | head -n1)"

# Check if boot package is installed
echo -e "\n${BLUE}Checking R packages...${RESET}"
if ! R --quiet --no-save -e 'library(boot)' &> /dev/null; then
    echo -e "${YELLOW}[WARN]${RESET} R boot package not found, installing..."
    R --quiet --no-save -e 'install.packages("boot", repos="https://cloud.r-project.org/")'
    echo -e "${GREEN}[OK]${RESET} boot package installed"
else
    echo -e "${GREEN}[OK]${RESET} boot package is installed"
fi

# Track overall success
TOTAL_VALIDATORS=6
PASSED_VALIDATORS=0
FAILED_VALIDATORS=0

# Create temp file for all validator output
VALIDATOR_OUTPUT=$(mktemp)

# Function to run a validator
run_validator() {
    local validator_name=$1
    local validator_script=$2

    echo -e "\n${BOLD}${BLUE}================================================${RESET}"
    echo -e "${BOLD}${BLUE}Running: ${validator_name}${RESET}"
    echo -e "${BOLD}${BLUE}================================================${RESET}"

    if [ ! -f "$validator_script" ]; then
        echo -e "${RED}[ERROR]${RESET} Validator script not found: $validator_script"
        return 1
    fi

    # Make script executable
    chmod +x "$validator_script"

    # Run the R script and capture output
    local validator_temp=$(mktemp)
    if Rscript "$validator_script" 2>&1 | tee "$validator_temp"; then
        cat "$validator_temp" >> "$VALIDATOR_OUTPUT"
        rm -f "$validator_temp"
        echo -e "\n${GREEN}[PASS]${RESET} ${validator_name} completed successfully"
        return 0
    else
        cat "$validator_temp" >> "$VALIDATOR_OUTPUT"
        rm -f "$validator_temp"
        echo -e "\n${RED}[FAIL]${RESET} ${validator_name} failed"
        return 1
    fi
}

# Run validators
echo -e "\n${BOLD}Running R Validators...${RESET}\n"

if run_validator "Bootstrap Validator" "$SCRIPT_DIR/bootstrap_validator.R"; then
    PASSED_VALIDATORS=$((PASSED_VALIDATORS + 1))
else
    FAILED_VALIDATORS=$((FAILED_VALIDATORS + 1))
fi

if run_validator "Distribution Validator" "$SCRIPT_DIR/distribution_validator.R"; then
    PASSED_VALIDATORS=$((PASSED_VALIDATORS + 1))
else
    FAILED_VALIDATORS=$((FAILED_VALIDATORS + 1))
fi

if run_validator "Financial Validator" "$SCRIPT_DIR/financial_validator.R"; then
    PASSED_VALIDATORS=$((PASSED_VALIDATORS + 1))
else
    FAILED_VALIDATORS=$((FAILED_VALIDATORS + 1))
fi

if run_validator "Monte Carlo Validator" "$SCRIPT_DIR/monte_carlo_validator.R"; then
    PASSED_VALIDATORS=$((PASSED_VALIDATORS + 1))
else
    FAILED_VALIDATORS=$((FAILED_VALIDATORS + 1))
fi

if run_validator "Math Validator" "$SCRIPT_DIR/math_validator.R"; then
    PASSED_VALIDATORS=$((PASSED_VALIDATORS + 1))
else
    FAILED_VALIDATORS=$((FAILED_VALIDATORS + 1))
fi

if run_validator "Date Validator" "$SCRIPT_DIR/date_validator.R"; then
    PASSED_VALIDATORS=$((PASSED_VALIDATORS + 1))
else
    FAILED_VALIDATORS=$((FAILED_VALIDATORS + 1))
fi

# Count individual test results from validator output
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

if [ -f "$VALIDATOR_OUTPUT" ]; then
    # Strip ANSI color codes for easier parsing
    VALIDATOR_OUTPUT_CLEAN=$(mktemp)
    sed 's/\x1b\[[0-9;]*m//g' "$VALIDATOR_OUTPUT" > "$VALIDATOR_OUTPUT_CLEAN"

    # Extract actual test counts from each validator's summary
    # Use sed for portable regex (works on both macOS and Linux)

    # Validators that output "Total tests: X" and "Passed: Y" (Financial, Math, Date)
    # Extract all instances and sum them
    VALIDATOR_TOTALS=$(grep "^Total tests:" "$VALIDATOR_OUTPUT_CLEAN" 2>/dev/null | sed -n 's/^Total tests:[[:space:]]*\([0-9]*\).*/\1/p')
    VALIDATOR_TOTAL=0
    for total in $VALIDATOR_TOTALS; do
        VALIDATOR_TOTAL=$(( VALIDATOR_TOTAL + total ))
    done

    VALIDATOR_PASSES=$(grep "^Passed:" "$VALIDATOR_OUTPUT_CLEAN" 2>/dev/null | sed -n 's/^Passed:[[:space:]]*\([0-9]*\).*/\1/p')
    VALIDATOR_PASSED=0
    for passed in $VALIDATOR_PASSES; do
        VALIDATOR_PASSED=$(( VALIDATOR_PASSED + passed ))
    done

    # Bootstrap validator outputs "Tests completed: 15"
    BOOTSTRAP_TESTS=$(grep "Tests completed:" "$VALIDATOR_OUTPUT_CLEAN" 2>/dev/null | sed -n 's/.*Tests completed:[[:space:]]*\([0-9]*\).*/\1/p' | head -1)
    BOOTSTRAP_TESTS=${BOOTSTRAP_TESTS:-0}
    # Ensure it's a number
    BOOTSTRAP_TESTS=$(echo "$BOOTSTRAP_TESTS" | tr -d '\n' | grep -o '[0-9]*' | head -1)
    BOOTSTRAP_TESTS=${BOOTSTRAP_TESTS:-0}

    # Monte Carlo validator outputs "Tests Completed: 19 Total"
    MONTE_CARLO_TESTS=$(grep "Tests Completed:" "$VALIDATOR_OUTPUT_CLEAN" 2>/dev/null | sed -n 's/.*Tests Completed:[[:space:]]*\([0-9]*\).*/\1/p' | head -1)
    MONTE_CARLO_TESTS=${MONTE_CARLO_TESTS:-0}
    # Ensure it's a number
    MONTE_CARLO_TESTS=$(echo "$MONTE_CARLO_TESTS" | tr -d '\n' | grep -o '[0-9]*' | head -1)
    MONTE_CARLO_TESTS=${MONTE_CARLO_TESTS:-0}

    # Distribution validator: count test sections (Test 1-13)
    DISTRIBUTION_TESTS=$(grep -c "^Test [0-9][0-9]*:" "$VALIDATOR_OUTPUT_CLEAN" 2>/dev/null || echo "0")
    # Ensure it's a number
    DISTRIBUTION_TESTS=$(echo "$DISTRIBUTION_TESTS" | tr -d '\n' | grep -o '[0-9]*' | head -1)
    DISTRIBUTION_TESTS=${DISTRIBUTION_TESTS:-0}

    # Clean up temp file
    rm -f "$VALIDATOR_OUTPUT_CLEAN"

    # Calculate totals
    # Validators with explicit pass/fail tracking
    FAILED_TESTS=$(( VALIDATOR_TOTAL - VALIDATOR_PASSED ))

    # Sum up all tests
    REFERENCE_TESTS=$(( BOOTSTRAP_TESTS + MONTE_CARLO_TESTS + DISTRIBUTION_TESTS ))
    if [ $PASSED_VALIDATORS -eq 6 ]; then
        # All validators passed, so count reference tests as passed
        PASSED_TESTS=$(( VALIDATOR_PASSED + REFERENCE_TESTS ))
    elif [ $PASSED_VALIDATORS -ge 3 ]; then
        # If most passed, count what we can
        PASSED_TESTS=$(( VALIDATOR_PASSED + REFERENCE_TESTS ))
        # Adjust if some validators failed
        if [ $FAILED_VALIDATORS -gt 0 ]; then
            # Conservative estimate - only count what explicitly passed
            PASSED_TESTS=$(( VALIDATOR_PASSED ))
        fi
    else
        # Partial success - be conservative
        PASSED_TESTS=$(( VALIDATOR_PASSED ))
    fi

    TOTAL_TESTS=$(( PASSED_TESTS + FAILED_TESTS ))
fi

# Print summary
echo -e "\n${BOLD}${BLUE}========================================${RESET}"
echo -e "${BOLD}${BLUE}  Validation Summary${RESET}"
echo -e "${BOLD}${BLUE}========================================${RESET}\n"

echo -e "Validators:        ${PASSED_VALIDATORS}/${TOTAL_VALIDATORS} passed"
if [ $TOTAL_TESTS -gt 0 ]; then
    echo -e "Individual Tests:  ${PASSED_TESTS} passed, ${FAILED_TESTS} failed"
fi

# Cleanup temp file
rm -f "$VALIDATOR_OUTPUT"

if [ $FAILED_VALIDATORS -eq 0 ]; then
    echo -e "\n${GREEN}${BOLD}✓ All validators passed!${RESET}\n"
    exit 0
else
    echo -e "\n${RED}${BOLD}✗ Some validators failed${RESET}\n"
    exit 1
fi
