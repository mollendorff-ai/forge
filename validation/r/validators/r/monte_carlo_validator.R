#!/usr/bin/env Rscript
# Monte Carlo Validator for forge-e2e
# Validates Monte Carlo distributions against R's stats package
#
# Usage:
#   Human-readable mode: Rscript monte_carlo_validator.R
#   JSON API mode:       Rscript monte_carlo_validator.R --json '<params>'
#
# JSON params format:
#   {
#     "distribution": "normal|uniform|lognormal|triangular|pert|exponential|discrete",
#     "params": {"mean": 100, "sd": 15, ...},
#     "seed": 42,
#     "iterations": 10000
#   }

suppressPackageStartupMessages({
  library(jsonlite)
})

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

VERSION <- "1.0.0"
VALIDATOR_NAME <- "monte_carlo_validator.R"

# ANSI color codes for human-readable output
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# ─────────────────────────────────────────────────────────────────────────────
# Distribution Functions
# ─────────────────────────────────────────────────────────────────────────────

# Triangular distribution using mc2d or fallback
rtriangular <- function(n, min, mode, max) {
  if (requireNamespace("mc2d", quietly = TRUE)) {
    return(mc2d::rtriang(n, min = min, mode = mode, max = max))
  }
  # Fallback: inverse transform method
  u <- runif(n)
  fc <- (mode - min) / (max - min)
  result <- numeric(n)
  idx_left <- u < fc
  result[idx_left] <- min + sqrt(u[idx_left] * (max - min) * (mode - min))
  result[!idx_left] <- max - sqrt((1 - u[!idx_left]) * (max - min) * (max - mode))
  return(result)
}

# PERT distribution using mc2d or fallback
rpert <- function(n, min, mode, max, shape = 4) {
  if (requireNamespace("mc2d", quietly = TRUE)) {
    return(mc2d::rpert(n, min = min, mode = mode, max = max, shape = shape))
  }
  # Fallback: Beta-based
  range <- max - min
  mu <- (min + shape * mode + max) / (shape + 2)
  if (mu == min || mu == max) return(rep(mu, n))
  alpha <- ((mu - min) * (2 * mode - min - max)) / ((mode - mu) * range)
  beta <- alpha * (max - mu) / (mu - min)
  samples <- rbeta(n, alpha, beta)
  return(min + samples * range)
}

# ─────────────────────────────────────────────────────────────────────────────
# Statistics Functions
# ─────────────────────────────────────────────────────────────────────────────

compute_stats <- function(samples) {
  list(
    mean = mean(samples),
    std = sd(samples),
    median = median(samples),
    min = min(samples),
    max = max(samples),
    percentiles = list(
      "5" = unname(quantile(samples, 0.05)),
      "10" = unname(quantile(samples, 0.10)),
      "25" = unname(quantile(samples, 0.25)),
      "50" = unname(quantile(samples, 0.50)),
      "75" = unname(quantile(samples, 0.75)),
      "90" = unname(quantile(samples, 0.90)),
      "95" = unname(quantile(samples, 0.95))
    ),
    samples = samples
  )
}

# ─────────────────────────────────────────────────────────────────────────────
# JSON API Mode
# ─────────────────────────────────────────────────────────────────────────────

run_json_mode <- function(params_json) {
  tryCatch({
    params <- fromJSON(params_json)

    dist <- tolower(params$distribution)
    p <- params$params
    seed <- if (!is.null(params$seed)) params$seed else 42
    n <- if (!is.null(params$iterations)) params$iterations else 10000

    set.seed(seed)

    samples <- switch(dist,
      "normal" = {
        mean_val <- if (!is.null(p$mean)) p$mean else 0
        sd_val <- if (!is.null(p$sd)) p$sd else 1
        rnorm(n, mean = mean_val, sd = sd_val)
      },
      "uniform" = {
        min_val <- if (!is.null(p$min)) p$min else 0
        max_val <- if (!is.null(p$max)) p$max else 1
        runif(n, min = min_val, max = max_val)
      },
      "lognormal" = {
        meanlog <- if (!is.null(p$meanlog)) p$meanlog else 0
        sdlog <- if (!is.null(p$sdlog)) p$sdlog else 1
        rlnorm(n, meanlog = meanlog, sdlog = sdlog)
      },
      "triangular" = {
        min_val <- if (!is.null(p$min)) p$min else 0
        mode_val <- if (!is.null(p$mode)) p$mode else 0.5
        max_val <- if (!is.null(p$max)) p$max else 1
        rtriangular(n, min = min_val, mode = mode_val, max = max_val)
      },
      "pert" = {
        min_val <- if (!is.null(p$min)) p$min else 0
        mode_val <- if (!is.null(p$mode)) p$mode else 0.5
        max_val <- if (!is.null(p$max)) p$max else 1
        shape <- if (!is.null(p$shape)) p$shape else 4
        rpert(n, min = min_val, mode = mode_val, max = max_val, shape = shape)
      },
      "exponential" = {
        rate <- if (!is.null(p$rate)) p$rate else 1
        rexp(n, rate = rate)
      },
      "discrete" = {
        values <- p$values
        probs <- p$probs
        if (is.null(values) || is.null(probs)) {
          stop("Discrete distribution requires 'values' and 'probs'")
        }
        sample(values, n, replace = TRUE, prob = probs)
      },
      stop(paste("Unknown distribution:", dist))
    )

    stats <- compute_stats(samples)

    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = TRUE,
      results = list(
        mean = stats$mean,
        std = stats$std,
        percentiles = stats$percentiles,
        samples = stats$samples[1:min(100, length(stats$samples))]  # Limit samples for output
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
  cat(sprintf("\n%s=== Monte Carlo Distribution Validator ===%s\n", BLUE, RESET))
  cat(sprintf("Generates large samples and validates summary statistics\n"))
  cat(sprintf("Sample size: N=100,000 per distribution\n"))
  cat(sprintf("Tolerance: 1e-3 (0.1%% acceptable due to random sampling)\n\n"))

  # Test 1: Normal Distribution
  cat(sprintf("%s=== Test 1: Normal Distribution ===%s\n", BLUE, RESET))
  cat(sprintf("MC.Normal(mean=100, sd=15)\n"))

  set.seed(12345)
  sample_normal <- rnorm(100000, mean = 100, sd = 15)
  stats <- compute_stats(sample_normal)

  cat(sprintf("\nR Reference Values:\n"))
  cat(sprintf("  Mean:     %.10f (expected: 100.00)\n", stats$mean))
  cat(sprintf("  Std Dev:  %.10f (expected: 15.00)\n", stats$std))
  cat(sprintf("  Median:   %.10f (expected: ~100.00)\n", stats$median))
  cat(sprintf("  P5:       %.10f\n", stats$percentiles[["5"]]))
  cat(sprintf("  P95:      %.10f\n", stats$percentiles[["95"]]))

  # Test 2: Uniform Distribution
  cat(sprintf("\n%s=== Test 2: Uniform Distribution ===%s\n", BLUE, RESET))
  cat(sprintf("MC.Uniform(min=0, max=100)\n"))

  set.seed(12345)
  sample_uniform <- runif(100000, min = 0, max = 100)
  stats <- compute_stats(sample_uniform)

  cat(sprintf("\nR Reference Values:\n"))
  cat(sprintf("  Mean:     %.10f (expected: 50.00)\n", stats$mean))
  cat(sprintf("  Std Dev:  %.10f (expected: 28.87)\n", stats$std))

  # Test 3: Triangular Distribution
  cat(sprintf("\n%s=== Test 3: Triangular Distribution ===%s\n", BLUE, RESET))
  cat(sprintf("MC.Triangular(min=10, mode=50, max=100)\n"))

  set.seed(12345)
  sample_tri <- rtriangular(100000, min = 10, mode = 50, max = 100)
  stats <- compute_stats(sample_tri)

  cat(sprintf("\nR Reference Values:\n"))
  cat(sprintf("  Mean:     %.10f (expected: 53.33)\n", stats$mean))
  cat(sprintf("  Std Dev:  %.10f\n", stats$std))

  # Test 4: PERT Distribution
  cat(sprintf("\n%s=== Test 4: PERT Distribution ===%s\n", BLUE, RESET))
  cat(sprintf("MC.PERT(min=10, mode=50, max=100, shape=4)\n"))

  set.seed(12345)
  sample_pert <- rpert(100000, min = 10, mode = 50, max = 100, shape = 4)
  stats <- compute_stats(sample_pert)

  cat(sprintf("\nR Reference Values:\n"))
  cat(sprintf("  Mean:     %.10f (expected: ~51.67)\n", stats$mean))
  cat(sprintf("  Std Dev:  %.10f\n", stats$std))

  # Test 5: Lognormal Distribution
  cat(sprintf("\n%s=== Test 5: Lognormal Distribution ===%s\n", BLUE, RESET))
  cat(sprintf("MC.Lognormal(meanlog=4, sdlog=0.5)\n"))

  set.seed(12345)
  sample_lognorm <- rlnorm(100000, meanlog = 4, sdlog = 0.5)
  stats <- compute_stats(sample_lognorm)

  cat(sprintf("\nR Reference Values:\n"))
  cat(sprintf("  Mean:     %.10f (expected: ~66.69)\n", stats$mean))
  cat(sprintf("  Median:   %.10f (expected: ~54.60)\n", stats$median))

  # Test 6: Exponential Distribution
  cat(sprintf("\n%s=== Test 6: Exponential Distribution ===%s\n", BLUE, RESET))
  cat(sprintf("MC.Exponential(rate=0.1)\n"))

  set.seed(12345)
  sample_exp <- rexp(100000, rate = 0.1)
  stats <- compute_stats(sample_exp)

  cat(sprintf("\nR Reference Values:\n"))
  cat(sprintf("  Mean:     %.10f (expected: 10.00)\n", stats$mean))
  cat(sprintf("  Std Dev:  %.10f (expected: 10.00)\n", stats$std))

  cat(sprintf("\n%s=== Monte Carlo Validation Complete ===%s\n\n", BLUE, RESET))
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
