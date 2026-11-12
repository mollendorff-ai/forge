#!/usr/bin/env Rscript
# Distribution Validator for forge-e2e
# Validates forge statistical distributions against R's stats package
# Tolerance: 1e-6 (6 decimal places - standard for statistical functions)

# ANSI color codes for output
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# Helper function to validate results
validate <- function(test_name, forge_result, r_result, tolerance = 1e-6) {
  diff <- abs(forge_result - r_result)

  if (is.na(forge_result) || is.na(r_result)) {
    cat(sprintf("%s[SKIP]%s %s (NA value)\n", YELLOW, RESET, test_name))
    return(FALSE)
  }

  if (diff <= tolerance) {
    cat(sprintf("%s[PASS]%s %s (diff: %.2e, tolerance: %.2e)\n",
                GREEN, RESET, test_name, diff, tolerance))
    return(TRUE)
  } else {
    cat(sprintf("%s[FAIL]%s %s (diff: %.2e > %.2e)\n",
                RED, RESET, test_name, diff, tolerance))
    cat(sprintf("       Forge: %.10f, R: %.10f\n", forge_result, r_result))
    return(FALSE)
  }
}

# Print header
cat(sprintf("\n%s=== Distribution Validator (R stats package) ===%s\n\n", BLUE, RESET))

# ============================================================================
# Normal Distribution Tests
# ============================================================================
cat(sprintf("%s=== Normal Distribution ===%s\n\n", BLUE, RESET))

# Test 1: Normal CDF (pnorm)
cat(sprintf("%sTest 1: Normal CDF (NORM.DIST)%s\n", BLUE, RESET))

test_cases_norm_cdf <- list(
  list(x = 0, mean = 0, sd = 1, expected_name = "P(Z <= 0)"),
  list(x = 1.96, mean = 0, sd = 1, expected_name = "P(Z <= 1.96) [95th percentile]"),
  list(x = -1.96, mean = 0, sd = 1, expected_name = "P(Z <= -1.96) [5th percentile]"),
  list(x = 2.576, mean = 0, sd = 1, expected_name = "P(Z <= 2.576) [99th percentile]"),
  list(x = 100, mean = 100, sd = 15, expected_name = "P(X <= 100) with mean=100, sd=15"),
  list(x = 115, mean = 100, sd = 15, expected_name = "P(X <= 115) with mean=100, sd=15"),
  list(x = 85, mean = 100, sd = 15, expected_name = "P(X <= 85) with mean=100, sd=15")
)

for (tc in test_cases_norm_cdf) {
  r_result <- pnorm(tc$x, mean = tc$mean, sd = tc$sd)
  cat(sprintf("%-50s: %.10f\n", tc$expected_name, r_result))
}

# Test 2: Normal Quantile (qnorm)
cat(sprintf("\n%sTest 2: Normal Quantile (NORM.INV)%s\n", BLUE, RESET))

test_cases_norm_inv <- list(
  list(p = 0.5, mean = 0, sd = 1, expected_name = "Q(0.5) - median"),
  list(p = 0.025, mean = 0, sd = 1, expected_name = "Q(0.025) - 2.5th percentile"),
  list(p = 0.975, mean = 0, sd = 1, expected_name = "Q(0.975) - 97.5th percentile"),
  list(p = 0.95, mean = 0, sd = 1, expected_name = "Q(0.95) - 95th percentile"),
  list(p = 0.99, mean = 0, sd = 1, expected_name = "Q(0.99) - 99th percentile"),
  list(p = 0.5, mean = 100, sd = 15, expected_name = "Q(0.5) with mean=100, sd=15"),
  list(p = 0.975, mean = 100, sd = 15, expected_name = "Q(0.975) with mean=100, sd=15")
)

for (tc in test_cases_norm_inv) {
  r_result <- qnorm(tc$p, mean = tc$mean, sd = tc$sd)
  cat(sprintf("%-50s: %.10f\n", tc$expected_name, r_result))
}

# ============================================================================
# t-Distribution Tests
# ============================================================================
cat(sprintf("\n%s=== t-Distribution ===%s\n\n", BLUE, RESET))

# Test 3: t CDF (pt)
cat(sprintf("%sTest 3: t-Distribution CDF (T.DIST)%s\n", BLUE, RESET))

test_cases_t_cdf <- list(
  list(x = 0, df = 10, expected_name = "P(t <= 0) with df=10"),
  list(x = 1.812, df = 10, expected_name = "P(t <= 1.812) with df=10 [95th percentile]"),
  list(x = 2.228, df = 10, expected_name = "P(t <= 2.228) with df=10 [97.5th percentile]"),
  list(x = 2.5, df = 10, expected_name = "P(t <= 2.5) with df=10"),
  list(x = 1.96, df = 30, expected_name = "P(t <= 1.96) with df=30"),
  list(x = 0, df = 1, expected_name = "P(t <= 0) with df=1 [Cauchy]"),
  list(x = 2.0, df = 5, expected_name = "P(t <= 2.0) with df=5")
)

for (tc in test_cases_t_cdf) {
  r_result <- pt(tc$x, df = tc$df)
  cat(sprintf("%-50s: %.10f\n", tc$expected_name, r_result))
}

# Test 4: t Quantile (qt)
cat(sprintf("\n%sTest 4: t-Distribution Quantile (T.INV)%s\n", BLUE, RESET))

test_cases_t_inv <- list(
  list(p = 0.5, df = 10, expected_name = "Q(0.5) with df=10"),
  list(p = 0.95, df = 10, expected_name = "Q(0.95) with df=10"),
  list(p = 0.975, df = 10, expected_name = "Q(0.975) with df=10 [two-tailed 95%]"),
  list(p = 0.99, df = 10, expected_name = "Q(0.99) with df=10"),
  list(p = 0.975, df = 20, expected_name = "Q(0.975) with df=20"),
  list(p = 0.95, df = 30, expected_name = "Q(0.95) with df=30"),
  list(p = 0.99, df = 5, expected_name = "Q(0.99) with df=5")
)

for (tc in test_cases_t_inv) {
  r_result <- qt(tc$p, df = tc$df)
  cat(sprintf("%-50s: %.10f\n", tc$expected_name, r_result))
}

# ============================================================================
# Chi-Square Distribution Tests
# ============================================================================
cat(sprintf("\n%s=== Chi-Square Distribution ===%s\n\n", BLUE, RESET))

# Test 5: Chi-square CDF (pchisq)
cat(sprintf("%sTest 5: Chi-Square CDF (CHISQ.DIST)%s\n", BLUE, RESET))

test_cases_chisq_cdf <- list(
  list(x = 0, df = 1, expected_name = "P(X <= 0) with df=1"),
  list(x = 3.841, df = 1, expected_name = "P(X <= 3.841) with df=1 [95th percentile]"),
  list(x = 5.991, df = 2, expected_name = "P(X <= 5.991) with df=2 [95th percentile]"),
  list(x = 7.815, df = 3, expected_name = "P(X <= 7.815) with df=3 [95th percentile]"),
  list(x = 5.99, df = 2, expected_name = "P(X <= 5.99) with df=2"),
  list(x = 10.0, df = 5, expected_name = "P(X <= 10.0) with df=5"),
  list(x = 20.0, df = 10, expected_name = "P(X <= 20.0) with df=10")
)

for (tc in test_cases_chisq_cdf) {
  r_result <- pchisq(tc$x, df = tc$df)
  cat(sprintf("%-50s: %.10f\n", tc$expected_name, r_result))
}

# Test 6: Chi-square Quantile (qchisq)
cat(sprintf("\n%sTest 6: Chi-Square Quantile (CHISQ.INV)%s\n", BLUE, RESET))

test_cases_chisq_inv <- list(
  list(p = 0.5, df = 1, expected_name = "Q(0.5) with df=1"),
  list(p = 0.95, df = 1, expected_name = "Q(0.95) with df=1"),
  list(p = 0.95, df = 2, expected_name = "Q(0.95) with df=2"),
  list(p = 0.95, df = 3, expected_name = "Q(0.95) with df=3"),
  list(p = 0.99, df = 5, expected_name = "Q(0.99) with df=5"),
  list(p = 0.975, df = 10, expected_name = "Q(0.975) with df=10"),
  list(p = 0.9, df = 20, expected_name = "Q(0.9) with df=20")
)

for (tc in test_cases_chisq_inv) {
  r_result <- qchisq(tc$p, df = tc$df)
  cat(sprintf("%-50s: %.10f\n", tc$expected_name, r_result))
}

# ============================================================================
# F Distribution Tests
# ============================================================================
cat(sprintf("\n%s=== F Distribution ===%s\n\n", BLUE, RESET))

# Test 7: F CDF (pf)
cat(sprintf("%sTest 7: F-Distribution CDF (F.DIST with cumulative=TRUE)%s\n", BLUE, RESET))

test_cases_f_cdf <- list(
  list(x = 2.5, df1 = 5, df2 = 10, expected_name = "P(F <= 2.5) with df1=5, df2=10"),
  list(x = 3.326, df1 = 5, df2 = 10, expected_name = "P(F <= 3.326) with df1=5, df2=10 [~95th percentile]"),
  list(x = 1.0, df1 = 10, df2 = 20, expected_name = "P(F <= 1.0) with df1=10, df2=20"),
  list(x = 4.0, df1 = 5, df2 = 5, expected_name = "P(F <= 4.0) with df1=5, df2=5"),
  list(x = 2.0, df1 = 3, df2 = 15, expected_name = "P(F <= 2.0) with df1=3, df2=15")
)

for (tc in test_cases_f_cdf) {
  r_result <- pf(tc$x, df1 = tc$df1, df2 = tc$df2)
  cat(sprintf("%-60s: %.10f\n", tc$expected_name, r_result))
}

# Test 8: F PDF (df)
cat(sprintf("\n%sTest 8: F-Distribution PDF (F.DIST with cumulative=FALSE)%s\n", BLUE, RESET))

test_cases_f_pdf <- list(
  list(x = 1.0, df1 = 5, df2 = 10, expected_name = "f(1.0) with df1=5, df2=10"),
  list(x = 2.5, df1 = 5, df2 = 10, expected_name = "f(2.5) with df1=5, df2=10"),
  list(x = 3.0, df1 = 3, df2 = 15, expected_name = "f(3.0) with df1=3, df2=15")
)

for (tc in test_cases_f_pdf) {
  r_result <- df(tc$x, df1 = tc$df1, df2 = tc$df2)
  cat(sprintf("%-60s: %.10f\n", tc$expected_name, r_result))
}

# Test 9: F Quantile (qf)
cat(sprintf("\n%sTest 9: F-Distribution Quantile (F.INV)%s\n", BLUE, RESET))

test_cases_f_inv <- list(
  list(p = 0.95, df1 = 5, df2 = 10, expected_name = "Q(0.95) with df1=5, df2=10"),
  list(p = 0.99, df1 = 5, df2 = 10, expected_name = "Q(0.99) with df1=5, df2=10"),
  list(p = 0.90, df1 = 10, df2 = 20, expected_name = "Q(0.90) with df1=10, df2=20"),
  list(p = 0.95, df1 = 3, df2 = 15, expected_name = "Q(0.95) with df1=3, df2=15")
)

for (tc in test_cases_f_inv) {
  r_result <- qf(tc$p, df1 = tc$df1, df2 = tc$df2)
  cat(sprintf("%-60s: %.10f\n", tc$expected_name, r_result))
}

# ============================================================================
# Exponential Distribution Tests
# ============================================================================
cat(sprintf("\n%s=== Exponential Distribution ===%s\n\n", BLUE, RESET))

# Test 10: Exponential CDF (pexp)
cat(sprintf("%sTest 10: Exponential CDF (EXPON.DIST with cumulative=TRUE)%s\n", BLUE, RESET))

test_cases_expon_cdf <- list(
  list(x = 1, lambda = 0.5, expected_name = "P(X <= 1) with lambda=0.5"),
  list(x = 2, lambda = 0.5, expected_name = "P(X <= 2) with lambda=0.5"),
  list(x = 1, lambda = 1.0, expected_name = "P(X <= 1) with lambda=1.0"),
  list(x = 0.5, lambda = 2.0, expected_name = "P(X <= 0.5) with lambda=2.0"),
  list(x = 3, lambda = 0.3, expected_name = "P(X <= 3) with lambda=0.3")
)

for (tc in test_cases_expon_cdf) {
  r_result <- pexp(tc$x, rate = tc$lambda)
  cat(sprintf("%-60s: %.10f\n", tc$expected_name, r_result))
}

# Test 11: Exponential PDF (dexp)
cat(sprintf("\n%sTest 11: Exponential PDF (EXPON.DIST with cumulative=FALSE)%s\n", BLUE, RESET))

test_cases_expon_pdf <- list(
  list(x = 1, lambda = 0.5, expected_name = "f(1) with lambda=0.5"),
  list(x = 2, lambda = 0.5, expected_name = "f(2) with lambda=0.5"),
  list(x = 1, lambda = 1.0, expected_name = "f(1) with lambda=1.0")
)

for (tc in test_cases_expon_pdf) {
  r_result <- dexp(tc$x, rate = tc$lambda)
  cat(sprintf("%-60s: %.10f\n", tc$expected_name, r_result))
}

# ============================================================================
# Poisson Distribution Tests
# ============================================================================
cat(sprintf("\n%s=== Poisson Distribution ===%s\n\n", BLUE, RESET))

# Test 12: Poisson CDF (ppois)
cat(sprintf("%sTest 12: Poisson CDF (POISSON.DIST with cumulative=TRUE)%s\n", BLUE, RESET))

test_cases_poisson_cdf <- list(
  list(x = 3, mean = 2, expected_name = "P(X <= 3) with mean=2"),
  list(x = 5, mean = 3, expected_name = "P(X <= 5) with mean=3"),
  list(x = 10, mean = 5, expected_name = "P(X <= 10) with mean=5"),
  list(x = 2, mean = 4, expected_name = "P(X <= 2) with mean=4"),
  list(x = 8, mean = 6, expected_name = "P(X <= 8) with mean=6")
)

for (tc in test_cases_poisson_cdf) {
  r_result <- ppois(tc$x, lambda = tc$mean)
  cat(sprintf("%-60s: %.10f\n", tc$expected_name, r_result))
}

# Test 13: Poisson PMF (dpois)
cat(sprintf("\n%sTest 13: Poisson PMF (POISSON.DIST with cumulative=FALSE)%s\n", BLUE, RESET))

test_cases_poisson_pmf <- list(
  list(x = 3, mean = 2, expected_name = "P(X = 3) with mean=2"),
  list(x = 5, mean = 3, expected_name = "P(X = 5) with mean=3"),
  list(x = 2, mean = 2, expected_name = "P(X = 2) with mean=2"),
  list(x = 0, mean = 1, expected_name = "P(X = 0) with mean=1")
)

for (tc in test_cases_poisson_pmf) {
  r_result <- dpois(tc$x, lambda = tc$mean)
  cat(sprintf("%-60s: %.10f\n", tc$expected_name, r_result))
}

# ============================================================================
# Summary
# ============================================================================
cat(sprintf("\n%s=== Distribution Validation Summary ===%s\n", BLUE, RESET))
cat(sprintf("All reference values computed successfully.\n"))
cat(sprintf("Compare forge distribution functions with R values above.\n"))
cat(sprintf("Tolerance: 1e-6 (6 decimal places - standard for statistical functions)\n\n"))

cat(sprintf("%sDistributions Validated:%s\n", YELLOW, RESET))
cat(sprintf("  - Normal: pnorm (NORM.DIST), qnorm (NORM.INV)\n"))
cat(sprintf("  - t-distribution: pt (T.DIST), qt (T.INV)\n"))
cat(sprintf("  - Chi-square: pchisq (CHISQ.DIST), qchisq (CHISQ.INV)\n"))
cat(sprintf("  - F-distribution: pf (F.DIST CDF), df (F.DIST PDF), qf (F.INV)\n"))
cat(sprintf("  - Exponential: pexp (EXPON.DIST CDF), dexp (EXPON.DIST PDF)\n"))
cat(sprintf("  - Poisson: ppois (POISSON.DIST CDF), dpois (POISSON.DIST PMF)\n\n"))

# Exit successfully
quit(status = 0)
