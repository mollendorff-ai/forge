#!/usr/bin/env Rscript
# Bootstrap Validator for forge-e2e
# Validates forge's PERCENTILE.BOOT against R's boot package
#
# Usage:
#   Human-readable mode: Rscript bootstrap_validator.R
#   JSON API mode:       Rscript bootstrap_validator.R --json '<params>'
#
# JSON params format:
#   {
#     "data": [1, 2, 3, 4, 5],
#     "statistic": "mean|median|std|var",
#     "method": "percentile|bca|basic",
#     "confidence_levels": [0.90, 0.95, 0.99],
#     "seed": 42,
#     "iterations": 10000
#   }

suppressPackageStartupMessages({
  library(jsonlite)
  library(boot)
})

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

VERSION <- "1.0.0"
VALIDATOR_NAME <- "bootstrap_validator.R"

# ANSI color codes for human-readable output
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# ─────────────────────────────────────────────────────────────────────────────
# Bootstrap Functions
# ─────────────────────────────────────────────────────────────────────────────

get_statistic_function <- function(statistic) {
  switch(tolower(statistic),
    "mean" = function(d, i) mean(d[i]),
    "median" = function(d, i) median(d[i]),
    "std" = function(d, i) sd(d[i]),
    "sd" = function(d, i) sd(d[i]),
    "var" = function(d, i) var(d[i]),
    "variance" = function(d, i) var(d[i]),
    stop(paste("Unknown statistic:", statistic))
  )
}

# ─────────────────────────────────────────────────────────────────────────────
# JSON API Mode
# ─────────────────────────────────────────────────────────────────────────────

run_json_mode <- function(params_json) {
  tryCatch({
    params <- fromJSON(params_json)

    data <- params$data
    statistic <- if (!is.null(params$statistic)) params$statistic else "mean"
    method <- if (!is.null(params$method)) tolower(params$method) else "percentile"
    confidence_levels <- if (!is.null(params$confidence_levels)) params$confidence_levels else c(0.95)
    seed <- if (!is.null(params$seed)) params$seed else 42
    n <- if (!is.null(params$iterations)) params$iterations else 10000

    if (is.null(data) || length(data) == 0) {
      stop("Bootstrap requires 'data' array")
    }

    set.seed(seed)

    stat_func <- get_statistic_function(statistic)
    boot_results <- boot(data, stat_func, R = n)

    # Calculate original estimate
    original_estimate <- stat_func(data, seq_along(data))
    bootstrap_mean <- mean(boot_results$t)
    bootstrap_std_error <- sd(boot_results$t)
    bias <- bootstrap_mean - original_estimate

    # Calculate confidence intervals for each level
    ci_list <- list()
    ci_type <- switch(method,
      "percentile" = "perc",
      "perc" = "perc",
      "bca" = "bca",
      "basic" = "basic",
      "perc"  # default
    )

    for (conf in confidence_levels) {
      # boot.ci prints a message to stdout when every resample is equal,
      # which would corrupt the JSON output, so its output is discarded.
      ci <- NULL
      invisible(capture.output(ci <- tryCatch(
        boot.ci(boot_results, type = ci_type, conf = conf),
        error = function(e) NULL
      )))

      conf_key <- as.character(conf)

      if (is.null(ci) && ci_type == "perc" && length(unique(boot_results$t)) == 1) {
        # No variation: the percentile interval is that single value.
        v <- boot_results$t[1]
        ci_list[[conf_key]] <- list(lower = v, upper = v)
      } else if (!is.null(ci)) {
        if (ci_type == "perc") {
          ci_list[[conf_key]] <- list(
            lower = ci$percent[4],
            upper = ci$percent[5]
          )
        } else if (ci_type == "bca") {
          ci_list[[conf_key]] <- list(
            lower = ci$bca[4],
            upper = ci$bca[5]
          )
        } else if (ci_type == "basic") {
          ci_list[[conf_key]] <- list(
            lower = ci$basic[4],
            upper = ci$basic[5]
          )
        }
      }
    }

    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = TRUE,
      results = list(
        mean = bootstrap_mean,
        std = bootstrap_std_error,
        original_estimate = original_estimate,
        bias = bias,
        bias_corrected = original_estimate - bias,
        confidence_intervals = ci_list,
        samples = as.vector(boot_results$t)[1:min(100, length(boot_results$t))]
      )
    )

    cat(toJSON(result, auto_unbox = TRUE, digits = 10))

  }, error = function(e) {
    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = FALSE,
      results = NULL,
      error = conditionMessage(e)
    )
    cat(toJSON(result, auto_unbox = TRUE))
    quit(status = 1)
  })
}

# ─────────────────────────────────────────────────────────────────────────────
# Human-Readable Mode
# ─────────────────────────────────────────────────────────────────────────────

run_human_mode <- function() {
  cat(sprintf("\n%s=== Bootstrap Validator (R boot package) ===%s\n\n", BLUE, RESET))

  # Test 1: Basic bootstrap with mean statistic
  cat(sprintf("%sTest 1: Bootstrap Mean (10,000 iterations)%s\n", BLUE, RESET))

  data1 <- c(0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04)
  mean_func <- function(d, indices) mean(d[indices])

  set.seed(12345)
  results1 <- boot(data1, mean_func, R = 10000)

  original_mean <- mean(data1)
  bootstrap_mean <- mean(results1$t)
  bootstrap_std_error <- sd(results1$t)
  bias <- bootstrap_mean - original_mean

  cat(sprintf("Original mean:        %.10f\n", original_mean))
  cat(sprintf("Bootstrap mean:       %.10f\n", bootstrap_mean))
  cat(sprintf("Bootstrap std error:  %.10f\n", bootstrap_std_error))
  cat(sprintf("Bias:                 %.10f\n", bias))

  # Confidence intervals
  ci_90 <- boot.ci(results1, type = "perc", conf = 0.90)
  ci_95 <- boot.ci(results1, type = "perc", conf = 0.95)
  ci_99 <- boot.ci(results1, type = "perc", conf = 0.99)

  cat(sprintf("\n90%% CI: [%.10f, %.10f]\n", ci_90$percent[4], ci_90$percent[5]))
  cat(sprintf("95%% CI: [%.10f, %.10f]\n", ci_95$percent[4], ci_95$percent[5]))
  cat(sprintf("99%% CI: [%.10f, %.10f]\n", ci_99$percent[4], ci_99$percent[5]))

  # Test 2: BCa vs Percentile
  cat(sprintf("\n%sTest 2: BCa vs Percentile Confidence Intervals%s\n", BLUE, RESET))

  ci_perc <- boot.ci(results1, type = "perc", conf = 0.95)
  ci_bca <- boot.ci(results1, type = "bca", conf = 0.95)

  cat(sprintf("Percentile 95%% CI:   [%.10f, %.10f]\n", ci_perc$percent[4], ci_perc$percent[5]))
  cat(sprintf("BCa 95%% CI:           [%.10f, %.10f]\n", ci_bca$bca[4], ci_bca$bca[5]))

  # Test 3: Bootstrap median
  cat(sprintf("\n%sTest 3: Bootstrap Median%s\n", BLUE, RESET))

  median_func <- function(d, indices) median(d[indices])
  set.seed(12345)
  results3 <- boot(data1, median_func, R = 10000)

  original_median <- median(data1)
  bootstrap_median <- mean(results3$t)

  cat(sprintf("Original median:      %.10f\n", original_median))
  cat(sprintf("Bootstrap median:     %.10f\n", bootstrap_median))

  ci_95_med <- boot.ci(results3, type = "perc", conf = 0.95)
  cat(sprintf("95%% CI:               [%.10f, %.10f]\n", ci_95_med$percent[4], ci_95_med$percent[5]))

  # Test 4: Bootstrap std dev
  cat(sprintf("\n%sTest 4: Bootstrap Standard Deviation%s\n", BLUE, RESET))

  sd_func <- function(d, indices) sd(d[indices])
  set.seed(12345)
  results4 <- boot(data1, sd_func, R = 10000)

  original_sd <- sd(data1)
  bootstrap_sd <- mean(results4$t)

  cat(sprintf("Original sd:          %.10f\n", original_sd))
  cat(sprintf("Bootstrap sd:         %.10f\n", bootstrap_sd))

  ci_95_sd <- boot.ci(results4, type = "perc", conf = 0.95)
  cat(sprintf("95%% CI:               [%.10f, %.10f]\n", ci_95_sd$percent[4], ci_95_sd$percent[5]))

  cat(sprintf("\n%s=== Bootstrap Validation Complete ===%s\n\n", BLUE, RESET))
}

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

args <- commandArgs(trailingOnly = TRUE)

if (length(args) >= 2 && args[1] == "--json") {
  run_json_mode(args[2])
} else {
  run_human_mode()
}
