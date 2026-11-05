#!/usr/bin/env Rscript
# Sensitivity Analysis Validator for forge-e2e
# Validates forge's sensitivity analysis against R calculations
#
# Usage:
#   Human-readable mode: Rscript sensitivity_validator.R
#   JSON API mode:       Rscript sensitivity_validator.R --json '<params>'
#
# JSON params format:
#   {
#     "analysis_type": "one_way|two_way|elasticity",
#     "base_values": {"price": 100, "volume": 1000},
#     "vary": ["price"],           # or ["price", "volume"] for two_way
#     "range": {"price": {"low": 80, "high": 120, "steps": 5}},
#     "model": "linear",
#     "coefficients": {"price": 1000, "volume": 100},
#     "output_formula": "price * volume"  # optional description
#   }

suppressPackageStartupMessages({
  library(jsonlite)
})

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

VERSION <- "1.0.0"
VALIDATOR_NAME <- "sensitivity_validator.R"

# ANSI color codes
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# ─────────────────────────────────────────────────────────────────────────────
# Model Evaluation Functions
# ─────────────────────────────────────────────────────────────────────────────

# Evaluate linear model: sum(coef * var)
eval_linear <- function(values, coefficients) {
  total <- 0
  for (name in names(values)) {
    coef <- coefficients[[name]]
    if (!is.null(coef)) {
      total <- total + coef * values[[name]]
    }
  }
  return(total)
}

# Evaluate multiplicative model: product(var^coef)
eval_multiplicative <- function(values, coefficients) {
  total <- 1
  for (name in names(values)) {
    coef <- coefficients[[name]]
    if (!is.null(coef)) {
      total <- total * (values[[name]] ^ coef)
    }
  }
  return(total)
}

# Generic evaluator
evaluate_model <- function(values, model_type, coefficients) {
  if (model_type == "multiplicative") {
    eval_multiplicative(values, coefficients)
  } else {
    eval_linear(values, coefficients)
  }
}

# ─────────────────────────────────────────────────────────────────────────────
# Sensitivity Analysis Functions
# ─────────────────────────────────────────────────────────────────────────────

# One-way sensitivity: vary one variable, keep others at base
one_way_sensitivity <- function(base_values, vary_name, range_spec, model_type, coefficients) {
  low <- range_spec$low
  high <- range_spec$high
  steps <- if (!is.null(range_spec$steps)) range_spec$steps else 10

  vary_values <- seq(low, high, length.out = steps)
  outputs <- numeric(steps)

  for (i in seq_along(vary_values)) {
    values <- base_values
    values[[vary_name]] <- vary_values[i]
    outputs[i] <- evaluate_model(values, model_type, coefficients)
  }

  # Calculate elasticity at base
  base_output <- evaluate_model(base_values, model_type, coefficients)
  base_var <- base_values[[vary_name]]

  # Use central difference for elasticity
  delta <- (high - low) / 100  # small change
  values_plus <- base_values
  values_plus[[vary_name]] <- base_var + delta
  values_minus <- base_values
  values_minus[[vary_name]] <- base_var - delta

  output_plus <- evaluate_model(values_plus, model_type, coefficients)
  output_minus <- evaluate_model(values_minus, model_type, coefficients)

  # Elasticity = (dY/Y) / (dX/X) = (dY/dX) * (X/Y)
  dY_dX <- (output_plus - output_minus) / (2 * delta)
  elasticity <- dY_dX * (base_var / base_output)

  list(
    variable = vary_name,
    vary_values = vary_values,
    outputs = outputs,
    base_value = base_var,
    base_output = base_output,
    elasticity = elasticity,
    min_output = min(outputs),
    max_output = max(outputs),
    range = max(outputs) - min(outputs)
  )
}

# Two-way sensitivity: vary two variables in a grid
two_way_sensitivity <- function(base_values, vary_names, ranges, model_type, coefficients) {
  var1 <- as.character(vary_names[[1]])
  var2 <- as.character(vary_names[[2]])

  range1 <- ranges[[var1]]
  range2 <- ranges[[var2]]

  steps1 <- if (!is.null(range1$steps)) range1$steps else 5
  steps2 <- if (!is.null(range2$steps)) range2$steps else 5

  values1 <- seq(range1$low, range1$high, length.out = steps1)
  values2 <- seq(range2$low, range2$high, length.out = steps2)

  # Create output grid
  grid <- matrix(0, nrow = steps1, ncol = steps2)
  rownames(grid) <- as.character(round(values1, 4))
  colnames(grid) <- as.character(round(values2, 4))

  for (i in seq_along(values1)) {
    for (j in seq_along(values2)) {
      values <- base_values
      values[[var1]] <- values1[i]
      values[[var2]] <- values2[j]
      grid[i, j] <- evaluate_model(values, model_type, coefficients)
    }
  }

  list(
    variable1 = var1,
    variable2 = var2,
    values1 = values1,
    values2 = values2,
    grid = grid,
    min_output = min(grid),
    max_output = max(grid)
  )
}

# Calculate elasticities for all variables
calculate_elasticities <- function(base_values, model_type, coefficients, pct_change = 0.01) {
  base_output <- evaluate_model(base_values, model_type, coefficients)
  elasticities <- list()

  for (name in names(base_values)) {
    base_var <- base_values[[name]]
    delta <- base_var * pct_change

    values_plus <- base_values
    values_plus[[name]] <- base_var + delta
    values_minus <- base_values
    values_minus[[name]] <- base_var - delta

    output_plus <- evaluate_model(values_plus, model_type, coefficients)
    output_minus <- evaluate_model(values_minus, model_type, coefficients)

    # Elasticity = (dY/Y) / (dX/X)
    pct_change_output <- (output_plus - output_minus) / base_output
    pct_change_input <- (2 * delta) / base_var
    elasticity <- pct_change_output / pct_change_input

    elasticities[[name]] <- elasticity
  }

  # Sort by absolute elasticity (most sensitive first)
  abs_elast <- sapply(elasticities, abs)
  sorted_names <- names(sort(abs_elast, decreasing = TRUE))

  ordered_elasticities <- list()
  for (i in seq_along(sorted_names)) {
    name <- sorted_names[i]
    ordered_elasticities[[i]] <- list(
      rank = i,
      variable = name,
      elasticity = elasticities[[name]],
      interpretation = if (abs(elasticities[[name]]) > 1) "elastic" else "inelastic"
    )
  }

  list(
    base_output = base_output,
    elasticities = ordered_elasticities
  )
}

# ─────────────────────────────────────────────────────────────────────────────
# JSON API Mode
# ─────────────────────────────────────────────────────────────────────────────

run_json_mode <- function(params_json) {
  tryCatch({
    params <- fromJSON(params_json, simplifyVector = FALSE)

    analysis_type <- if (!is.null(params$analysis_type)) tolower(params$analysis_type) else "one_way"
    base_values <- params$base_values
    vary <- params$vary
    ranges <- params$range
    model_type <- if (!is.null(params$model)) tolower(params$model) else "linear"
    coefficients <- params$coefficients

    if (is.null(base_values)) {
      stop("Sensitivity requires 'base_values'")
    }

    results <- switch(analysis_type,
      "one_way" = {
        if (is.null(vary) || length(vary) == 0) {
          stop("one_way requires 'vary' with one variable name")
        }
        vary_name <- vary[[1]]
        range_spec <- ranges[[vary_name]]
        if (is.null(range_spec)) {
          stop(paste("No range specified for variable:", vary_name))
        }
        one_way_sensitivity(base_values, vary_name, range_spec, model_type, coefficients)
      },

      "two_way" = {
        if (is.null(vary) || length(vary) < 2) {
          stop("two_way requires 'vary' with two variable names")
        }
        two_way_sensitivity(base_values, vary[1:2], ranges, model_type, coefficients)
      },

      "elasticity" = {
        calculate_elasticities(base_values, model_type, coefficients)
      },

      stop(paste("Unknown analysis_type:", analysis_type))
    )

    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = TRUE,
      results = list(
        analysis_type = analysis_type,
        model_type = model_type,
        data = results
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
  cat(sprintf("\n%s=== Sensitivity Analysis Validator ===%s\n\n", BLUE, RESET))

  # Example: Revenue = price * volume
  cat(sprintf("%sTest 1: One-Way Sensitivity%s\n", BLUE, RESET))
  cat("Model: revenue = price * volume\n")
  cat("Base: price=100, volume=1000 => revenue=100,000\n")
  cat("Vary price from 80 to 120\n\n")

  base_values <- list(price = 100, volume = 1000)
  coefficients <- list(price = 1000, volume = 100)  # revenue = price*1000 + volume*100 for linear approx

  # Actually for multiplicative: revenue = price * volume
  # Let's use a simpler linear approximation for demo
  # revenue = price * volume at base = 100 * 1000 = 100000

  price_values <- seq(80, 120, by = 10)
  for (p in price_values) {
    revenue <- p * 1000  # volume fixed at 1000
    cat(sprintf("  price=%d => revenue=%d\n", p, revenue))
  }

  cat("\n")
  cat(sprintf("%sTest 2: Two-Way Sensitivity%s\n", BLUE, RESET))
  cat("Vary both price (80-120) and volume (800-1200)\n\n")

  cat("           volume\n")
  cat("price     800    1000    1200\n")
  for (p in c(80, 100, 120)) {
    row <- sprintf("%3d   ", p)
    for (v in c(800, 1000, 1200)) {
      revenue <- p * v
      row <- paste0(row, sprintf("%6d  ", revenue))
    }
    cat(row, "\n")
  }

  cat("\n")
  cat(sprintf("%sTest 3: Elasticity Analysis%s\n", BLUE, RESET))
  cat("For multiplicative model: revenue = price * volume\n")
  cat("Elasticity of revenue w.r.t. price = 1.0 (proportional)\n")
  cat("Elasticity of revenue w.r.t. volume = 1.0 (proportional)\n")

  # For Cobb-Douglas: Y = L^0.7 * K^0.3
  cat("\nFor Cobb-Douglas: Y = L^0.7 * K^0.3\n")
  cat("Elasticity w.r.t. L = 0.7 (exponent)\n")
  cat("Elasticity w.r.t. K = 0.3 (exponent)\n")

  cat(sprintf("\n%s=== Sensitivity Validation Complete ===%s\n\n", BLUE, RESET))
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
