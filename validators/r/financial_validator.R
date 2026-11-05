#!/usr/bin/env Rscript
# Financial Analytics Validator for forge-e2e
# Validates forge's financial functions against R's exact arithmetic
# Author: forge-e2e team
# Tolerance: 1e-10 (exact arithmetic - deterministic calculations)

script_dir <- dirname(sub("--file=", "", grep("--file=", commandArgs(FALSE), value = TRUE)[1]))
source(file.path(script_dir, "forge_helper.R"))

# ANSI color codes for output
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# Helper function to validate results
validate <- function(test_name, forge_result, r_result, tolerance = 1e-10) {
  diff <- abs(forge_result - r_result)

  if (is.na(forge_result) || is.na(r_result)) {
    cat(sprintf("%s[SKIP]%s %s (NA value)\n", YELLOW, RESET, test_name))
    return(FALSE)
  }

  if (is.infinite(forge_result) || is.infinite(r_result)) {
    if (forge_result == r_result) {
      cat(sprintf("%s[PASS]%s %s (both Inf)\n", GREEN, RESET, test_name))
      return(TRUE)
    } else {
      cat(sprintf("%s[FAIL]%s %s (Inf mismatch)\n", RED, RESET, test_name))
      return(FALSE)
    }
  }

  if (diff <= tolerance) {
    cat(sprintf("%s[PASS]%s %s (diff: %.2e, tolerance: %.2e)\n",
                GREEN, RESET, test_name, diff, tolerance))
    return(TRUE)
  } else {
    cat(sprintf("%s[FAIL]%s %s (diff: %.2e > %.2e)\n",
                RED, RESET, test_name, diff, tolerance))
    cat(sprintf("       Forge: %.15f, R: %.15f\n", forge_result, r_result))
    return(FALSE)
  }
}

# R implementations of financial functions
breakeven_units <- function(fixed_costs, price, variable_cost) {
  return(fixed_costs / (price - variable_cost))
}

breakeven_revenue <- function(fixed_costs, margin_pct) {
  return(fixed_costs / margin_pct)
}

variance <- function(actual, budget) {
  return(actual - budget)
}

variance_pct <- function(actual, budget) {
  return((actual - budget) / budget)
}

variance_status <- function(actual, budget, threshold) {
  var_pct <- variance_pct(actual, budget)

  if (var_pct > threshold) {
    return(1)   # favorable
  } else if (var_pct < -threshold) {
    return(-1)  # unfavorable
  } else {
    return(0)   # on target
  }
}

# Print header
cat(sprintf("\n%s=== Financial Analytics Validator ===%s\n\n", BLUE, RESET))

# Test counters
passed <- 0
failed <- 0
total <- 0

# ==============================================================================
# BREAKEVEN_UNITS Tests
# ==============================================================================
cat(sprintf("%s--- BREAKEVEN_UNITS Tests ---%s\n", BLUE, RESET))

# Test 1: Normal case
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_UNITS(10000, 100, 60)")
r_result <- breakeven_units(10000, 100, 60)
if (validate("BREAKEVEN_UNITS(10000, 100, 60)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 2: Small fixed costs
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_UNITS(1000, 200, 100)")
r_result <- breakeven_units(1000, 200, 100)
if (validate("BREAKEVEN_UNITS(1000, 200, 100)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 3: Large fixed costs
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_UNITS(1000000, 50, 40)")
r_result <- breakeven_units(1000000, 50, 40)
if (validate("BREAKEVEN_UNITS(1000000, 50, 40)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 4: High margin (large price difference)
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_UNITS(50000, 600, 100)")
r_result <- breakeven_units(50000, 600, 100)
if (validate("BREAKEVEN_UNITS(50000, 600, 100)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 5: Low margin (small price difference)
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_UNITS(50000, 101, 100)")
r_result <- breakeven_units(50000, 101, 100)
if (validate("BREAKEVEN_UNITS(50000, 101, 100)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 6: Zero variable cost
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_UNITS(10000, 100, 0)")
r_result <- breakeven_units(10000, 100, 0)
if (validate("BREAKEVEN_UNITS(10000, 100, 0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# BREAKEVEN_REVENUE Tests
# ==============================================================================
cat(sprintf("\n%s--- BREAKEVEN_REVENUE Tests ---%s\n", BLUE, RESET))

# Test 7: Normal case (40% margin)
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_REVENUE(10000, 0.40)")
r_result <- breakeven_revenue(10000, 0.40)
if (validate("BREAKEVEN_REVENUE(10000, 0.40)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 8: High margin (80%)
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_REVENUE(10000, 0.80)")
r_result <- breakeven_revenue(10000, 0.80)
if (validate("BREAKEVEN_REVENUE(10000, 0.80)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 9: Low margin (10%)
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_REVENUE(10000, 0.10)")
r_result <- breakeven_revenue(10000, 0.10)
if (validate("BREAKEVEN_REVENUE(10000, 0.10)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 10: Large fixed costs
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_REVENUE(1000000, 0.40)")
r_result <- breakeven_revenue(1000000, 0.40)
if (validate("BREAKEVEN_REVENUE(1000000, 0.40)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 11: Small fixed costs
total <- total + 1
forge_result <- forge_eval("BREAKEVEN_REVENUE(500, 0.25)")
r_result <- breakeven_revenue(500, 0.25)
if (validate("BREAKEVEN_REVENUE(500, 0.25)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# VARIANCE Tests
# ==============================================================================
cat(sprintf("\n%s--- VARIANCE Tests ---%s\n", BLUE, RESET))

# Test 12: Positive variance
total <- total + 1
forge_result <- forge_eval("VARIANCE(150000, 100000)")
r_result <- variance(150000, 100000)
if (validate("VARIANCE(150000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 13: Negative variance
total <- total + 1
forge_result <- forge_eval("VARIANCE(75000, 100000)")
r_result <- variance(75000, 100000)
if (validate("VARIANCE(75000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 14: Zero variance
total <- total + 1
forge_result <- forge_eval("VARIANCE(100000, 100000)")
r_result <- variance(100000, 100000)
if (validate("VARIANCE(100000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 15: Large numbers
total <- total + 1
forge_result <- forge_eval("VARIANCE(15000000, 10000000)")
r_result <- variance(15000000, 10000000)
if (validate("VARIANCE(15000000, 10000000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 16: Small numbers
total <- total + 1
forge_result <- forge_eval("VARIANCE(200, 150)")
r_result <- variance(200, 150)
if (validate("VARIANCE(200, 150)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# VARIANCE_PCT Tests
# ==============================================================================
cat(sprintf("\n%s--- VARIANCE_PCT Tests ---%s\n", BLUE, RESET))

# Test 17: Positive variance 20%
total <- total + 1
forge_result <- forge_eval("VARIANCE_PCT(120000, 100000)")
r_result <- variance_pct(120000, 100000)
if (validate("VARIANCE_PCT(120000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 18: Negative variance -25%
total <- total + 1
forge_result <- forge_eval("VARIANCE_PCT(75000, 100000)")
r_result <- variance_pct(75000, 100000)
if (validate("VARIANCE_PCT(75000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 19: Zero variance
total <- total + 1
forge_result <- forge_eval("VARIANCE_PCT(100000, 100000)")
r_result <- variance_pct(100000, 100000)
if (validate("VARIANCE_PCT(100000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 20: Large positive variance 100%
total <- total + 1
forge_result <- forge_eval("VARIANCE_PCT(200000, 100000)")
r_result <- variance_pct(200000, 100000)
if (validate("VARIANCE_PCT(200000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 21: Small variance 5%
total <- total + 1
forge_result <- forge_eval("VARIANCE_PCT(105000, 100000)")
r_result <- variance_pct(105000, 100000)
if (validate("VARIANCE_PCT(105000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 22: Large negative variance -50%
total <- total + 1
forge_result <- forge_eval("VARIANCE_PCT(50000, 100000)")
r_result <- variance_pct(50000, 100000)
if (validate("VARIANCE_PCT(50000, 100000)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# VARIANCE_STATUS Tests
# ==============================================================================
cat(sprintf("\n%s--- VARIANCE_STATUS Tests ---%s\n", BLUE, RESET))

# Test 23: Favorable variance (above threshold)
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(120000, 100000, 0.10)")
r_result <- variance_status(120000, 100000, 0.10)
if (validate("VARIANCE_STATUS(120000, 100000, 0.10)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 24: Unfavorable variance (below negative threshold)
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(75000, 100000, 0.10)")
r_result <- variance_status(75000, 100000, 0.10)
if (validate("VARIANCE_STATUS(75000, 100000, 0.10)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 25: On target (within threshold)
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(105000, 100000, 0.10)")
r_result <- variance_status(105000, 100000, 0.10)
if (validate("VARIANCE_STATUS(105000, 100000, 0.10)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 26: On target (negative within threshold)
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(96000, 100000, 0.10)")
r_result <- variance_status(96000, 100000, 0.10)
if (validate("VARIANCE_STATUS(96000, 100000, 0.10)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 27: Exactly at threshold (should be on target)
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(110000, 100000, 0.10)")
r_result <- variance_status(110000, 100000, 0.10)
if (validate("VARIANCE_STATUS(110000, 100000, 0.10)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 28: Exactly at negative threshold (should be on target)
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(90000, 100000, 0.10)")
r_result <- variance_status(90000, 100000, 0.10)
if (validate("VARIANCE_STATUS(90000, 100000, 0.10)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 29: Large favorable variance with small threshold
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(200000, 100000, 0.01)")
r_result <- variance_status(200000, 100000, 0.01)
if (validate("VARIANCE_STATUS(200000, 100000, 0.01)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 30: Large unfavorable variance with small threshold
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(50000, 100000, 0.01)")
r_result <- variance_status(50000, 100000, 0.01)
if (validate("VARIANCE_STATUS(50000, 100000, 0.01)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 31: Tight threshold test (2%)
total <- total + 1
forge_result <- forge_eval("VARIANCE_STATUS(101500, 100000, 0.02)")
r_result <- variance_status(101500, 100000, 0.02)
if (validate("VARIANCE_STATUS(101500, 100000, 0.02)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Summary
# ==============================================================================
cat(sprintf("\n%s=== Financial Analytics Validation Summary ===%s\n", BLUE, RESET))
cat(sprintf("Total tests:  %d\n", total))
cat(sprintf("%sPassed:       %d%s\n", GREEN, passed, RESET))

if (failed > 0) {
  cat(sprintf("%sFailed:       %d%s\n", RED, failed, RESET))
}

cat(sprintf("Tolerance:    1e-10 (exact arithmetic)\n"))

# Exit with appropriate status
if (failed > 0) {
  cat(sprintf("\n%sValidation FAILED%s\n\n", RED, RESET))
  quit(status = 1)
} else {
  cat(sprintf("\n%sValidation PASSED%s\n\n", GREEN, RESET))
  quit(status = 0)
}
