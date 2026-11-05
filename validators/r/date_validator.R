#!/usr/bin/env Rscript
# Date Validator for forge-e2e
# Validates forge's date functions against R's base date functions
# Tolerance: exact match for date operations (deterministic)

script_dir <- dirname(sub("--file=", "", grep("--file=", commandArgs(FALSE), value = TRUE)[1]))
source(file.path(script_dir, "forge_helper.R"))

# ANSI color codes for output
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# Helper function to validate date results
validate_date <- function(test_name, forge_result, r_result) {
  if (is.na(forge_result) || is.na(r_result)) {
    cat(sprintf("%s[SKIP]%s %s (NA value)\n", YELLOW, RESET, test_name))
    return(FALSE)
  }

  if (forge_result == r_result) {
    cat(sprintf("%s[PASS]%s %s\n", GREEN, RESET, test_name))
    cat(sprintf("       Value: %s\n", as.character(r_result)))
    return(TRUE)
  } else {
    cat(sprintf("%s[FAIL]%s %s\n", RED, RESET, test_name))
    cat(sprintf("       Forge: %s, R: %s\n", as.character(forge_result), as.character(r_result)))
    return(FALSE)
  }
}

# Helper function to validate numeric results
validate_numeric <- function(test_name, forge_result, r_result, tolerance = 0) {
  diff <- abs(forge_result - r_result)

  if (is.na(forge_result) || is.na(r_result)) {
    cat(sprintf("%s[SKIP]%s %s (NA value)\n", YELLOW, RESET, test_name))
    return(FALSE)
  }

  if (diff <= tolerance) {
    cat(sprintf("%s[PASS]%s %s\n", GREEN, RESET, test_name))
    cat(sprintf("       Value: %d\n", r_result))
    return(TRUE)
  } else {
    cat(sprintf("%s[FAIL]%s %s\n", RED, RESET, test_name))
    cat(sprintf("       Forge: %d, R: %d\n", forge_result, r_result))
    return(FALSE)
  }
}

# Business day calculation helper functions
is_weekend <- function(date) {
  weekday <- as.POSIXlt(date)$wday
  return(weekday == 0 | weekday == 6)  # Sunday = 0, Saturday = 6
}

# Add workdays to a date (skip weekends)
add_workdays <- function(start_date, days) {
  if (days == 0) return(start_date)

  current_date <- start_date
  days_added <- 0
  direction <- if (days > 0) 1 else -1
  days_to_add <- abs(days)

  while (days_added < days_to_add) {
    current_date <- current_date + direction
    if (!is_weekend(current_date)) {
      days_added <- days_added + 1
    }
  }

  return(current_date)
}

# Count workdays between two dates (excluding weekends)
count_workdays <- function(start_date, end_date) {
  # Excel NETWORKDAYS: both ends count; a start after the end gives a negative count.
  if (start_date > end_date) {
    return(-count_workdays(end_date, start_date))
  }
  days <- seq(start_date, end_date, by = "day")
  sum(!is_weekend(days))
}

# Print header
cat(sprintf("\n%s=== Date Validator (R base date functions) ===%s\n\n", BLUE, RESET))

# Test counters
passed <- 0
failed <- 0
total <- 0

# ==============================================================================
# DATE Construction Tests
# ==============================================================================
cat(sprintf("%s--- DATE Construction Tests ---%s\n", BLUE, RESET))

# Test 1: Basic date construction
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 1, 15)")
r_result <- as.Date("2024-01-15")
if (validate_date("DATE(2024, 1, 15)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 2: End of month
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 1, 31)")
r_result <- as.Date("2024-01-31")
if (validate_date("DATE(2024, 1, 31)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 3: Leap year - Feb 29
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 2, 29)")
r_result <- as.Date("2024-02-29")
if (validate_date("DATE(2024, 2, 29) [leap year]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 4: Non-leap year Feb (28 days)
total <- total + 1
forge_result <- forge_eval_date("DATE(2023, 2, 28)")
r_result <- as.Date("2023-02-28")
if (validate_date("DATE(2023, 2, 28) [non-leap year]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 5: December 31
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 12, 31)")
r_result <- as.Date("2024-12-31")
if (validate_date("DATE(2024, 12, 31)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 6: January 1
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 1, 1)")
r_result <- as.Date("2024-01-01")
if (validate_date("DATE(2024, 1, 1)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 7: Century year (2000 - leap year)
total <- total + 1
forge_result <- forge_eval_date("DATE(2000, 2, 29)")
r_result <- as.Date("2000-02-29")
if (validate_date("DATE(2000, 2, 29) [century leap year]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 8: Century year (1900 - not leap year)
# Note: 1900 is not a leap year (divisible by 100 but not 400)
total <- total + 1
forge_result <- forge_eval_date("DATE(1900, 2, 28)")
r_result <- as.Date("1900-02-28")
if (validate_date("DATE(1900, 2, 28) [century non-leap]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# YEAR, MONTH, DAY Extraction Tests
# ==============================================================================
cat(sprintf("\n%s--- YEAR, MONTH, DAY Extraction Tests ---%s\n", BLUE, RESET))

# Test 9: Extract year
total <- total + 1
test_date <- as.Date("2024-03-15")
forge_result <- forge_eval("YEAR(DATE(2024, 3, 15))")
r_result <- as.numeric(format(test_date, "%Y"))
if (validate_numeric("YEAR(2024-03-15)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 10: Extract month
total <- total + 1
forge_result <- forge_eval("MONTH(DATE(2024, 3, 15))")
r_result <- as.numeric(format(test_date, "%m"))
if (validate_numeric("MONTH(2024-03-15)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 11: Extract day
total <- total + 1
forge_result <- forge_eval("DAY(DATE(2024, 3, 15))")
r_result <- as.numeric(format(test_date, "%d"))
if (validate_numeric("DAY(2024-03-15)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 12: Extract year from leap day
total <- total + 1
leap_date <- as.Date("2024-02-29")
forge_result <- forge_eval("YEAR(DATE(2024, 2, 29))")
r_result <- as.numeric(format(leap_date, "%Y"))
if (validate_numeric("YEAR(2024-02-29)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 13: Extract month from December
total <- total + 1
dec_date <- as.Date("2024-12-25")
forge_result <- forge_eval("MONTH(DATE(2024, 12, 25))")
r_result <- as.numeric(format(dec_date, "%m"))
if (validate_numeric("MONTH(2024-12-25)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 14: Extract day from end of month
total <- total + 1
eom_date <- as.Date("2024-01-31")
forge_result <- forge_eval("DAY(DATE(2024, 1, 31))")
r_result <- as.numeric(format(eom_date, "%d"))
if (validate_numeric("DAY(2024-01-31)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Date Arithmetic Tests
# ==============================================================================
cat(sprintf("\n%s--- Date Arithmetic Tests ---%s\n", BLUE, RESET))

# Test 15: Add days
total <- total + 1
start_date <- as.Date("2024-01-15")
forge_result <- forge_eval_date("DATE(2024, 1, 15) + 10")
r_result <- start_date + 10
if (validate_date("Add 10 days to 2024-01-15", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 16: Subtract days
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 1, 15) - 10")
r_result <- start_date - 10
if (validate_date("Subtract 10 days from 2024-01-15", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 17: Add days crossing month boundary
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 1, 25) + 11")
r_result <- as.Date("2024-01-25") + 11
if (validate_date("Add 11 days to 2024-01-25 [cross month]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 18: Add days crossing year boundary
total <- total + 1
forge_result <- forge_eval_date("DATE(2024, 12, 25) + 11")
r_result <- as.Date("2024-12-25") + 11
if (validate_date("Add 11 days to 2024-12-25 [cross year]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 19: Date difference in days
total <- total + 1
date1 <- as.Date("2024-01-01")
date2 <- as.Date("2024-12-31")
forge_result <- forge_eval("DATE(2024, 12, 31) - DATE(2024, 1, 1)")
r_result <- as.numeric(difftime(date2, date1, units = "days"))
if (validate_numeric("Days between 2024-01-01 and 2024-12-31", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 20: Date difference crossing leap day
total <- total + 1
date1 <- as.Date("2024-02-28")
date2 <- as.Date("2024-03-01")
forge_result <- forge_eval("DATE(2024, 3, 1) - DATE(2024, 2, 28)")
r_result <- as.numeric(difftime(date2, date1, units = "days"))
if (validate_numeric("Days between 2024-02-28 and 2024-03-01 [leap]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Leap Year Tests
# ==============================================================================
cat(sprintf("\n%s--- Leap Year Tests ---%s\n", BLUE, RESET))

# Test 21: 2024 is leap year (divisible by 4)
total <- total + 1
forge_result <- forge_eval("MONTH(DATE(2024, 2, 29))") == 2  # Feb 29 exists only in a leap year
# Check if Feb 29, 2024 is valid
r_result <- !is.na(as.Date("2024-02-29", format = "%Y-%m-%d", optional = FALSE))
if (isTRUE(forge_result == r_result)) {
  cat(sprintf("%s[PASS]%s 2024 is leap year\n", GREEN, RESET))
  passed <- passed + 1
} else {
  cat(sprintf("%s[FAIL]%s 2024 is leap year\n", RED, RESET))
  failed <- failed + 1
}

# Test 22: 2023 is not leap year
total <- total + 1
forge_result <- forge_eval("MONTH(DATE(2023, 2, 29))") == 2  # Feb 29 exists only in a leap year
# Check if Feb 29, 2023 is invalid (should be NA or error)
r_result <- !is.na(as.Date("2023-02-29", format = "%Y-%m-%d"))
if (isTRUE(forge_result == r_result)) {
  cat(sprintf("%s[PASS]%s 2023 is not leap year\n", GREEN, RESET))
  passed <- passed + 1
} else {
  cat(sprintf("%s[FAIL]%s 2023 is not leap year\n", RED, RESET))
  failed <- failed + 1
}

# Test 23: 2000 is leap year (divisible by 400)
total <- total + 1
forge_result <- forge_eval("MONTH(DATE(2000, 2, 29))") == 2  # Feb 29 exists only in a leap year
r_result <- !is.na(as.Date("2000-02-29", format = "%Y-%m-%d", optional = FALSE))
if (isTRUE(forge_result == r_result)) {
  cat(sprintf("%s[PASS]%s 2000 is leap year (divisible by 400)\n", GREEN, RESET))
  passed <- passed + 1
} else {
  cat(sprintf("%s[FAIL]%s 2000 is leap year (divisible by 400)\n", RED, RESET))
  failed <- failed + 1
}

# Test 24: 1900 is not leap year (divisible by 100 but not 400)
total <- total + 1
forge_result <- forge_eval("MONTH(DATE(1900, 2, 29))") == 2  # Feb 29 exists only in a leap year
r_result <- !is.na(as.Date("1900-02-29", format = "%Y-%m-%d"))
if (isTRUE(forge_result == r_result)) {
  cat(sprintf("%s[PASS]%s 1900 is not leap year (divisible by 100)\n", GREEN, RESET))
  passed <- passed + 1
} else {
  cat(sprintf("%s[FAIL]%s 1900 is not leap year (divisible by 100)\n", RED, RESET))
  failed <- failed + 1
}

# ==============================================================================
# WORKDAY Tests (Business Day Calculations)
# ==============================================================================
cat(sprintf("\n%s--- WORKDAY Tests ---%s\n", BLUE, RESET))

# Test 25: Add 1 workday from Monday
total <- total + 1
start <- as.Date("2024-01-15")  # Monday
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 1, 15), 1)")
r_result <- add_workdays(start, 1)
if (validate_date("WORKDAY(2024-01-15, 1) [Mon->Tue]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 26: Add 1 workday from Friday (skips weekend)
total <- total + 1
start <- as.Date("2024-01-19")  # Friday
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 1, 19), 1)")
r_result <- add_workdays(start, 1)
if (validate_date("WORKDAY(2024-01-19, 1) [Fri->Mon]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 27: Add 5 workdays (one full week)
total <- total + 1
start <- as.Date("2024-01-15")  # Monday
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 1, 15), 5)")
r_result <- add_workdays(start, 5)
if (validate_date("WORKDAY(2024-01-15, 5) [one week]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 28: Add 10 workdays (two full weeks)
total <- total + 1
start <- as.Date("2024-01-15")  # Monday
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 1, 15), 10)")
r_result <- add_workdays(start, 10)
if (validate_date("WORKDAY(2024-01-15, 10) [two weeks]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 29: Subtract 1 workday from Monday (goes to Friday)
total <- total + 1
start <- as.Date("2024-01-15")  # Monday
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 1, 15), -1)")
r_result <- add_workdays(start, -1)
if (validate_date("WORKDAY(2024-01-15, -1) [Mon->Fri]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 30: Add 0 workdays
total <- total + 1
start <- as.Date("2024-01-15")
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 1, 15), 0)")
r_result <- add_workdays(start, 0)
if (validate_date("WORKDAY(2024-01-15, 0)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# NETWORKDAYS Tests (Count Business Days)
# ==============================================================================
cat(sprintf("\n%s--- NETWORKDAYS Tests ---%s\n", BLUE, RESET))

# Test 31: Count workdays in one week (Mon-Fri)
total <- total + 1
start <- as.Date("2024-01-15")  # Monday
end <- as.Date("2024-01-19")    # Friday
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 1, 15), DATE(2024, 1, 19))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-01-15, 2024-01-19)", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 32: Count workdays crossing weekend
total <- total + 1
start <- as.Date("2024-01-15")  # Monday
end <- as.Date("2024-01-22")    # Next Monday
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 1, 15), DATE(2024, 1, 22))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-01-15, 2024-01-22) [cross weekend]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 33: Count workdays same day
total <- total + 1
start <- as.Date("2024-01-15")
end <- as.Date("2024-01-15")
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 1, 15), DATE(2024, 1, 15))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-01-15, 2024-01-15) [same day]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 34: Count workdays over two weeks
total <- total + 1
start <- as.Date("2024-01-15")  # Monday
end <- as.Date("2024-01-29")    # Monday two weeks later
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 1, 15), DATE(2024, 1, 29))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-01-15, 2024-01-29) [two weeks]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 35: Count workdays Friday to Monday (crossing weekend)
total <- total + 1
start <- as.Date("2024-01-19")  # Friday
end <- as.Date("2024-01-22")    # Monday
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 1, 19), DATE(2024, 1, 22))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-01-19, 2024-01-22) [Fri->Mon]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 36: Count workdays in reverse (negative)
total <- total + 1
start <- as.Date("2024-01-22")  # Monday
end <- as.Date("2024-01-15")    # Previous Monday
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 1, 22), DATE(2024, 1, 15))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-01-22, 2024-01-15) [reverse]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Edge Cases - Month End and Year Transitions
# ==============================================================================
cat(sprintf("\n%s--- Edge Cases - Month/Year Transitions ---%s\n", BLUE, RESET))

# Test 37: Workday crossing month boundary
total <- total + 1
start <- as.Date("2024-01-31")  # Wednesday, end of January
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 1, 31), 1)")
r_result <- add_workdays(start, 1)
if (validate_date("WORKDAY(2024-01-31, 1) [cross month]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 38: Workday crossing year boundary
total <- total + 1
start <- as.Date("2024-12-31")  # Tuesday, end of year
forge_result <- forge_eval_date("WORKDAY(DATE(2024, 12, 31), 1)")
r_result <- add_workdays(start, 1)
if (validate_date("WORKDAY(2024-12-31, 1) [cross year]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 39: Networkdays crossing month
total <- total + 1
start <- as.Date("2024-01-29")  # Monday
end <- as.Date("2024-02-02")    # Friday
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 1, 29), DATE(2024, 2, 2))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-01-29, 2024-02-02) [cross month]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# Test 40: Networkdays crossing year
total <- total + 1
start <- as.Date("2024-12-30")  # Monday
end <- as.Date("2025-01-03")    # Friday
forge_result <- forge_eval("NETWORKDAYS(DATE(2024, 12, 30), DATE(2025, 1, 3))")
r_result <- count_workdays(start, end)
if (validate_numeric("NETWORKDAYS(2024-12-30, 2025-01-03) [cross year]", forge_result, r_result)) {
  passed <- passed + 1
} else {
  failed <- failed + 1
}

# ==============================================================================
# Summary
# ==============================================================================
cat(sprintf("\n%s=== Date Validator Summary ===%s\n", BLUE, RESET))
cat(sprintf("Total tests:  %d\n", total))
cat(sprintf("%sPassed:       %d%s\n", GREEN, passed, RESET))

if (failed > 0) {
  cat(sprintf("%sFailed:       %d%s\n", RED, failed, RESET))
}

cat(sprintf("\nFunctions tested:\n"))
cat(sprintf("  - DATE (8 tests): construction, leap years, edge cases\n"))
cat(sprintf("  - YEAR/MONTH/DAY (6 tests): component extraction\n"))
cat(sprintf("  - Date Arithmetic (6 tests): addition, subtraction, differences\n"))
cat(sprintf("  - Leap Year (4 tests): 4-year, 100-year, 400-year rules\n"))
cat(sprintf("  - WORKDAY (6 tests): business day addition/subtraction\n"))
cat(sprintf("  - NETWORKDAYS (6 tests): business day counting\n"))
cat(sprintf("  - Edge Cases (4 tests): month/year boundary crossings\n"))

cat(sprintf("\nEdge cases covered:\n"))
cat(sprintf("  - Leap years (2024, 2000)\n"))
cat(sprintf("  - Non-leap years (2023, 1900)\n"))
cat(sprintf("  - Century leap year rules (400-year cycle)\n"))
cat(sprintf("  - Weekend handling (Saturday, Sunday)\n"))
cat(sprintf("  - Month boundaries (Jan 31 -> Feb 1)\n"))
cat(sprintf("  - Year boundaries (Dec 31 -> Jan 1)\n"))
cat(sprintf("  - Negative date differences\n"))
cat(sprintf("  - Business day calculations across weeks\n"))

cat(sprintf("\nNote: WORKDAY and NETWORKDAYS assume weekends only.\n"))
cat(sprintf("Holiday handling would require additional configuration.\n"))

# Exit with appropriate status
if (failed > 0) {
  cat(sprintf("\n%sValidation FAILED%s\n\n", RED, RESET))
  quit(status = 1)
} else {
  cat(sprintf("\n%sValidation PASSED%s\n\n", GREEN, RESET))
  quit(status = 0)
}
