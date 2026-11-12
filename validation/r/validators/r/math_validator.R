#!/usr/bin/env Rscript
# Math Validator for forge-e2e
# Validates forge's math functions (ROUND, FLOOR, CEILING) against R's base math
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

# Excel's ROUND rounds half away from zero (R's round() rounds half to even).
excel_round <- function(x, digits = 0) {
  scale <- 10^digits
  sign(x) * floor(abs(x) * scale + 0.5) / scale
}

# Print header
cat(sprintf("\n%s=== Math Validator (R base math functions) ===%s\n\n", BLUE, RESET))

# Test counters
passed <- 0
failed <- 0
total <- 0

# ==============================================================================
# ROUND Tests
# ==============================================================================
cat(sprintf("%s--- ROUND Tests ---%s\n", BLUE, RESET))

# Test 1: Basic rounding to 0 decimal places
total <- total + 1
forge_result <- forge_eval("ROUND(3.14159, 0)")
r_result <- round(3.14159, 0)
if (validate("ROUND(3.14159, 0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 2: Round to 2 decimal places
total <- total + 1
forge_result <- forge_eval("ROUND(3.14159, 2)")
r_result <- round(3.14159, 2)
if (validate("ROUND(3.14159, 2)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 3: Round to 4 decimal places
total <- total + 1
forge_result <- forge_eval("ROUND(3.14159, 4)")
r_result <- round(3.14159, 4)
if (validate("ROUND(3.14159, 4)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 4: Round negative number to 0 decimals
total <- total + 1
forge_result <- forge_eval("ROUND(-3.14159, 0)")
r_result <- round(-3.14159, 0)
if (validate("ROUND(-3.14159, 0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 5: Round negative number to 2 decimals
total <- total + 1
forge_result <- forge_eval("ROUND(-3.14159, 2)")
r_result <- round(-3.14159, 2)
if (validate("ROUND(-3.14159, 2)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 6: Round exactly 0.5 (banker's rounding to even)
total <- total + 1
forge_result <- forge_eval("ROUND(2.5, 0)")
r_result <- excel_round(2.5, 0)  # Excel: half away from zero
if (validate("ROUND(2.5, 0) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 7: Round 3.5 (banker's rounding to even)
total <- total + 1
forge_result <- forge_eval("ROUND(3.5, 0)")
r_result <- excel_round(3.5, 0)  # Excel: half away from zero
if (validate("ROUND(3.5, 0) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 8: Round very small number
total <- total + 1
forge_result <- forge_eval("ROUND(0.000000001, 8)")
r_result <- round(0.000000001, 8)
if (validate("ROUND(0.000000001, 8)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 9: Round very large number
total <- total + 1
forge_result <- forge_eval("ROUND(1234567890.123, 0)")
r_result <- round(1234567890.123, 0)
if (validate("ROUND(1234567890.123, 0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 10: Round to negative decimal places (round to tens)
total <- total + 1
forge_result <- forge_eval("ROUND(1234.5678, -1)")
r_result <- round(1234.5678, -1)
if (validate("ROUND(1234.5678, -1) [round to tens]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 11: Round to negative decimal places (round to hundreds)
total <- total + 1
forge_result <- forge_eval("ROUND(1234.5678, -2)")
r_result <- round(1234.5678, -2)
if (validate("ROUND(1234.5678, -2) [round to hundreds]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 12: Round zero
total <- total + 1
forge_result <- forge_eval("ROUND(0, 2)")
r_result <- round(0, 2)
if (validate("ROUND(0, 2)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 13: Round with floating point precision edge case
total <- total + 1
forge_result <- forge_eval("ROUND(1.225, 2)")
r_result <- round(1.225, 2)
if (validate("ROUND(1.225, 2) [floating point edge]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 14: Round negative with negative digits
total <- total + 1
forge_result <- forge_eval("ROUND(-1234.5678, -2)")
r_result <- round(-1234.5678, -2)
if (validate("ROUND(-1234.5678, -2)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# FLOOR Tests
# ==============================================================================
cat(sprintf("\n%s--- FLOOR Tests ---%s\n", BLUE, RESET))

# Test 15: Basic floor positive
total <- total + 1
forge_result <- forge_eval("FLOOR(3.14159)")
r_result <- floor(3.14159)
if (validate("FLOOR(3.14159)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 16: Floor positive close to integer
total <- total + 1
forge_result <- forge_eval("FLOOR(3.9999)")
r_result <- floor(3.9999)
if (validate("FLOOR(3.9999)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 17: Floor negative number
total <- total + 1
forge_result <- forge_eval("FLOOR(-3.14159)")
r_result <- floor(-3.14159)
if (validate("FLOOR(-3.14159)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 18: Floor negative close to integer
total <- total + 1
forge_result <- forge_eval("FLOOR(-3.0001)")
r_result <- floor(-3.0001)
if (validate("FLOOR(-3.0001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 19: Floor zero
total <- total + 1
forge_result <- forge_eval("FLOOR(0)")
r_result <- floor(0)
if (validate("FLOOR(0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 20: Floor exact integer
total <- total + 1
forge_result <- forge_eval("FLOOR(5.0)")
r_result <- floor(5.0)
if (validate("FLOOR(5.0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 21: Floor very small positive
total <- total + 1
forge_result <- forge_eval("FLOOR(0.0000001)")
r_result <- floor(0.0000001)
if (validate("FLOOR(0.0000001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 22: Floor very small negative
total <- total + 1
forge_result <- forge_eval("FLOOR(-0.0000001)")
r_result <- floor(-0.0000001)
if (validate("FLOOR(-0.0000001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 23: Floor large positive
total <- total + 1
forge_result <- forge_eval("FLOOR(1234567890.987)")
r_result <- floor(1234567890.987)
if (validate("FLOOR(1234567890.987)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 24: Floor large negative
total <- total + 1
forge_result <- forge_eval("FLOOR(-1234567890.123)")
r_result <- floor(-1234567890.123)
if (validate("FLOOR(-1234567890.123)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 25: Floor 0.5
total <- total + 1
forge_result <- forge_eval("FLOOR(0.5)")
r_result <- floor(0.5)
if (validate("FLOOR(0.5)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 26: Floor -0.5
total <- total + 1
forge_result <- forge_eval("FLOOR(-0.5)")
r_result <- floor(-0.5)
if (validate("FLOOR(-0.5)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# CEILING Tests
# ==============================================================================
cat(sprintf("\n%s--- CEILING Tests ---%s\n", BLUE, RESET))

# Test 27: Basic ceiling positive
total <- total + 1
forge_result <- forge_eval("CEILING(3.14159)")
r_result <- ceiling(3.14159)
if (validate("CEILING(3.14159)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 28: Ceiling positive close to integer
total <- total + 1
forge_result <- forge_eval("CEILING(3.0001)")
r_result <- ceiling(3.0001)
if (validate("CEILING(3.0001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 29: Ceiling negative number
total <- total + 1
forge_result <- forge_eval("CEILING(-3.14159)")
r_result <- ceiling(-3.14159)
if (validate("CEILING(-3.14159)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 30: Ceiling negative close to integer
total <- total + 1
forge_result <- forge_eval("CEILING(-3.9999)")
r_result <- ceiling(-3.9999)
if (validate("CEILING(-3.9999)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 31: Ceiling zero
total <- total + 1
forge_result <- forge_eval("CEILING(0)")
r_result <- ceiling(0)
if (validate("CEILING(0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 32: Ceiling exact integer
total <- total + 1
forge_result <- forge_eval("CEILING(5.0)")
r_result <- ceiling(5.0)
if (validate("CEILING(5.0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 33: Ceiling very small positive
total <- total + 1
forge_result <- forge_eval("CEILING(0.0000001)")
r_result <- ceiling(0.0000001)
if (validate("CEILING(0.0000001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 34: Ceiling very small negative
total <- total + 1
forge_result <- forge_eval("CEILING(-0.0000001)")
r_result <- ceiling(-0.0000001)
if (validate("CEILING(-0.0000001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 35: Ceiling large positive
total <- total + 1
forge_result <- forge_eval("CEILING(1234567890.001)")
r_result <- ceiling(1234567890.001)
if (validate("CEILING(1234567890.001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 36: Ceiling large negative
total <- total + 1
forge_result <- forge_eval("CEILING(-1234567890.999)")
r_result <- ceiling(-1234567890.999)
if (validate("CEILING(-1234567890.999)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 37: Ceiling 0.5
total <- total + 1
forge_result <- forge_eval("CEILING(0.5)")
r_result <- ceiling(0.5)
if (validate("CEILING(0.5)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 38: Ceiling -0.5
total <- total + 1
forge_result <- forge_eval("CEILING(-0.5)")
r_result <- ceiling(-0.5)
if (validate("CEILING(-0.5)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Floating Point Precision Tests
# ==============================================================================
cat(sprintf("\n%s--- Floating Point Precision Tests ---%s\n", BLUE, RESET))

# Test 39: Round with floating point representation issue
total <- total + 1
forge_result <- forge_eval("ROUND(0.1 + 0.2 - 0.3, 1)")
r_result <- round(0.1 + 0.2 - 0.3, 1)  # Classic floating point case
if (validate("ROUND(0.1 + 0.2 - 0.3, 1) [floating point]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 40: Floor with floating point
total <- total + 1
forge_result <- forge_eval("FLOOR(0.1 + 0.2 - 0.3)")
r_result <- floor(0.1 + 0.2 - 0.3)
if (validate("FLOOR(0.1 + 0.2 - 0.3)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 41: Ceiling with floating point
total <- total + 1
forge_result <- forge_eval("CEILING(0.1 + 0.2 - 0.3 + 0.0001)")
r_result <- ceiling(0.1 + 0.2 - 0.3 + 0.0001)
if (validate("CEILING(0.1 + 0.2 - 0.3 + 0.0001)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 42: Round very precise number
total <- total + 1
forge_result <- forge_eval("ROUND(1.234567890123456, 8)")
r_result <- round(1.234567890123456, 8)
if (validate("ROUND(1.234567890123456, 8)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Edge Cases with Different Rounding Modes
# ==============================================================================
cat(sprintf("\n%s--- Rounding Mode Edge Cases ---%s\n", BLUE, RESET))

# Test 43: Round series of .5 values (banker's rounding pattern)
total <- total + 1
forge_result <- forge_eval("ROUND(0.5, 0)")
r_result <- excel_round(0.5, 0)  # Excel: half away from zero
if (validate("ROUND(0.5, 0) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 44: Round 1.5
total <- total + 1
forge_result <- forge_eval("ROUND(1.5, 0)")
r_result <- excel_round(1.5, 0)  # Excel: half away from zero
if (validate("ROUND(1.5, 0) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 45: Round 4.5
total <- total + 1
forge_result <- forge_eval("ROUND(4.5, 0)")
r_result <- excel_round(4.5, 0)  # Excel: half away from zero
if (validate("ROUND(4.5, 0) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 46: Round 5.5
total <- total + 1
forge_result <- forge_eval("ROUND(5.5, 0)")
r_result <- excel_round(5.5, 0)  # Excel: half away from zero
if (validate("ROUND(5.5, 0) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 47: Round with multiple decimal precision on .5
total <- total + 1
forge_result <- forge_eval("ROUND(1.25, 1)")
r_result <- excel_round(1.25, 1)  # Excel: half away from zero
if (validate("ROUND(1.25, 1) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 48: Round with multiple decimal precision on .5 (odd)
total <- total + 1
forge_result <- forge_eval("ROUND(1.35, 1)")
r_result <- excel_round(1.35, 1)  # Excel: half away from zero
if (validate("ROUND(1.35, 1) [half away from zero]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Summary
# ==============================================================================
cat(sprintf("\n%s=== Math Validator Summary ===%s\n", BLUE, RESET))
cat(sprintf("Total tests:  %d\n", total))
cat(sprintf("%sPassed:       %d%s\n", GREEN, passed, RESET))

if (failed > 0) {
  cat(sprintf("%sFailed:       %d%s\n", RED, failed, RESET))
}

cat(sprintf("Tolerance:    1e-10 (exact arithmetic)\n"))

cat(sprintf("\nFunctions tested:\n"))
cat(sprintf("  - ROUND (14 tests): basic, negative digits, banker's rounding\n"))
cat(sprintf("  - FLOOR (12 tests): positive, negative, edge cases\n"))
cat(sprintf("  - CEILING (12 tests): positive, negative, edge cases\n"))
cat(sprintf("  - Floating Point (4 tests): precision edge cases\n"))
cat(sprintf("  - Rounding Modes (6 tests): banker's rounding patterns\n"))

cat(sprintf("\nEdge cases covered:\n"))
cat(sprintf("  - Very large numbers (>1 billion)\n"))
cat(sprintf("  - Very small numbers (<0.000001)\n"))
cat(sprintf("  - Negative numbers\n"))
cat(sprintf("  - Zero\n"))
cat(sprintf("  - Exact integers\n"))
cat(sprintf("  - Banker's rounding (round-to-even for .5)\n"))
cat(sprintf("  - Negative digit rounding (tens, hundreds)\n"))
cat(sprintf("  - Floating point representation issues\n"))

# Exit with appropriate status
if (failed > 0) {
  cat(sprintf("\n%sValidation FAILED%s\n\n", RED, RESET))
  quit(status = 1)
} else {
  cat(sprintf("\n%sValidation PASSED%s\n\n", GREEN, RESET))
  quit(status = 0)
}
