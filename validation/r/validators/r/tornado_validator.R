#!/usr/bin/env Rscript
# Tornado Chart Validator for forge-e2e
# Validates forge's tornado analysis against R calculations
#
# Usage:
#   Human-readable mode: Rscript tornado_validator.R
#   JSON API mode:       Rscript tornado_validator.R --json '<params>'
#
# JSON params format:
#   {
#     "base_value": 100,
#     "variables": [
#       {"name": "price", "low": 80, "high": 120, "base": 100},
#       {"name": "volume", "low": 900, "high": 1100, "base": 1000}
#     ],
#     "model": "linear",  # linear, multiplicative
#     "coefficients": {"price": 1.0, "volume": 0.1}
#   }
#
# Tornado analysis calculates how output changes when each variable
# moves from its base to low/high values (one at a time).

suppressPackageStartupMessages({
  library(jsonlite)
})

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

VERSION <- "1.0.0"
VALIDATOR_NAME <- "tornado_validator.R"

# ANSI color codes
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# ─────────────────────────────────────────────────────────────────────────────
# Tornado Calculation Functions
# ─────────────────────────────────────────────────────────────────────────────

# Calculate output for a linear model: sum(coef * var)
calc_linear <- function(variables, coefficients) {
  total <- 0
  for (var in variables) {
    coef <- coefficients[[var$name]]
    if (!is.null(coef)) {
      total <- total + coef * var$value
    }
  }
  return(total)
}

# Calculate output for a multiplicative model: product(var^coef)
calc_multiplicative <- function(variables, coefficients) {
  total <- 1
  for (var in variables) {
    coef <- coefficients[[var$name]]
    if (!is.null(coef)) {
      total <- total * (var$value ^ coef)
    }
  }
  return(total)
}

# Calculate tornado swings for each variable
calculate_tornado <- function(base_value, variables, model_type, coefficients) {
  results <- list()

  for (i in seq_along(variables)) {
    var <- variables[[i]]
    name <- var$name
    low <- var$low
    high <- var$high
    base <- var$base

    # Calculate output at low value (all others at base)
    vars_at_low <- lapply(variables, function(v) {
      if (v$name == name) {
        list(name = v$name, value = low)
      } else {
        list(name = v$name, value = v$base)
      }
    })

    # Calculate output at high value (all others at base)
    vars_at_high <- lapply(variables, function(v) {
      if (v$name == name) {
        list(name = v$name, value = high)
      } else {
        list(name = v$name, value = v$base)
      }
    })

    # Calculate outputs
    if (model_type == "multiplicative") {
      output_low <- calc_multiplicative(vars_at_low, coefficients)
      output_high <- calc_multiplicative(vars_at_high, coefficients)
    } else {
      output_low <- calc_linear(vars_at_low, coefficients)
      output_high <- calc_linear(vars_at_high, coefficients)
    }

    # Swing is the absolute range
    swing <- abs(output_high - output_low)

    results[[name]] <- list(
      name = name,
      low_value = low,
      high_value = high,
      base_value = base,
      output_at_low = output_low,
      output_at_high = output_high,
      swing = swing
    )
  }

  # Sort by swing (descending) to get ranking
  swings <- sapply(results, function(r) r$swing)
  sorted_names <- names(sort(swings, decreasing = TRUE))

  # Add ranking
  for (i in seq_along(sorted_names)) {
    results[[sorted_names[i]]]$rank <- i
  }

  return(results)
}

# ─────────────────────────────────────────────────────────────────────────────
# JSON API Mode
# ─────────────────────────────────────────────────────────────────────────────

run_json_mode <- function(params_json) {
  tryCatch({
    params <- fromJSON(params_json, simplifyVector = FALSE)

    base_value <- params$base_value
    variables <- params$variables
    model_type <- if (!is.null(params$model)) tolower(params$model) else "linear"
    coefficients <- params$coefficients

    if (is.null(variables) || length(variables) == 0) {
      stop("Tornado requires 'variables' array")
    }

    tornado_results <- calculate_tornado(base_value, variables, model_type, coefficients)

    # Build ordered results (by rank)
    ordered_results <- list()
    for (name in names(tornado_results)) {
      r <- tornado_results[[name]]
      ordered_results[[length(ordered_results) + 1]] <- list(
        rank = r$rank,
        name = r$name,
        swing = r$swing,
        output_at_low = r$output_at_low,
        output_at_high = r$output_at_high,
        low_value = r$low_value,
        high_value = r$high_value
      )
    }

    # Sort by rank
    ordered_results <- ordered_results[order(sapply(ordered_results, function(x) x$rank))]

    # Get ranking order
    rankings <- sapply(ordered_results, function(x) x$name)

    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = TRUE,
      results = list(
        base_value = base_value,
        model_type = model_type,
        rankings = rankings,
        variables = ordered_results
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
  cat(sprintf("\n%s=== Tornado Chart Validator ===%s\n\n", BLUE, RESET))

  # Example: Simple NPV model
  # NPV = price * volume - fixed_cost
  cat(sprintf("%sTest 1: Simple Linear Model%s\n", BLUE, RESET))
  cat("Model: output = price * volume - fixed_cost\n")
  cat("Base case: price=100, volume=1000, fixed_cost=50000\n\n")

  variables <- list(
    list(name = "price", low = 80, high = 120, base = 100),
    list(name = "volume", low = 800, high = 1200, base = 1000),
    list(name = "fixed_cost", low = 40000, high = 60000, base = 50000)
  )

  # For this model: output = price * volume - fixed_cost
  # At base: 100 * 1000 - 50000 = 50000
  base_value <- 100 * 1000 - 50000

  # Calculate manually for each variable
  cat("Manual calculations:\n")

  # Price: varies 80-120, others at base
  # Low: 80 * 1000 - 50000 = 30000
  # High: 120 * 1000 - 50000 = 70000
  # Swing: 40000
  cat(sprintf("  price: low=%.0f, high=%.0f, swing=%.0f\n", 30000, 70000, 40000))

  # Volume: varies 800-1200, others at base
  # Low: 100 * 800 - 50000 = 30000
  # High: 100 * 1200 - 50000 = 70000
  # Swing: 40000
  cat(sprintf("  volume: low=%.0f, high=%.0f, swing=%.0f\n", 30000, 70000, 40000))

  # Fixed_cost: varies 40000-60000, others at base
  # Low: 100 * 1000 - 40000 = 60000
  # High: 100 * 1000 - 60000 = 40000
  # Swing: 20000
  cat(sprintf("  fixed_cost: low=%.0f, high=%.0f, swing=%.0f\n", 60000, 40000, 20000))

  cat(sprintf("\nBase value: %.0f\n", base_value))
  cat("Expected ranking: price = volume > fixed_cost\n")

  # Test 2: Multiplicative model
  cat(sprintf("\n%sTest 2: Cobb-Douglas Production Function%s\n", BLUE, RESET))
  cat("Model: output = A * L^alpha * K^beta\n")
  cat("Parameters: A=1, alpha=0.7, beta=0.3\n")
  cat("Base case: L=100, K=200\n\n")

  # output = 1 * 100^0.7 * 200^0.3 = 25.12 * 4.79 = 120.3
  base_cd <- 1 * (100^0.7) * (200^0.3)
  cat(sprintf("Base value: %.4f\n", base_cd))

  # L varies 80-120
  output_l_low <- 1 * (80^0.7) * (200^0.3)
  output_l_high <- 1 * (120^0.7) * (200^0.3)
  swing_l <- abs(output_l_high - output_l_low)
  cat(sprintf("  L: low=%.4f, high=%.4f, swing=%.4f\n", output_l_low, output_l_high, swing_l))

  # K varies 160-240
  output_k_low <- 1 * (100^0.7) * (160^0.3)
  output_k_high <- 1 * (100^0.7) * (240^0.3)
  swing_k <- abs(output_k_high - output_k_low)
  cat(sprintf("  K: low=%.4f, high=%.4f, swing=%.4f\n", output_k_low, output_k_high, swing_k))

  if (swing_l > swing_k) {
    cat("Expected ranking: L > K (labor more sensitive due to higher exponent)\n")
  } else {
    cat("Expected ranking: K > L\n")
  }

  cat(sprintf("\n%s=== Tornado Validation Complete ===%s\n\n", BLUE, RESET))
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
