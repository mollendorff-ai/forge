#!/usr/bin/env Rscript
# Real Options Validator for forge-e2e
# Validates forge's real options analysis against R derivmkts calculations
#
# Usage:
#   Human-readable mode: Rscript real_options_validator.R
#   JSON API mode:       Rscript real_options_validator.R --json '<params>'
#
# JSON params format:
#   {
#     "option_type": "call|put",
#     "model": "black_scholes|binomial",
#     "S": 100,           # Current asset value
#     "K": 100,           # Strike price (investment cost)
#     "r": 0.05,          # Risk-free rate
#     "sigma": 0.3,       # Volatility
#     "T": 1,             # Time to expiration (years)
#     "n": 100,           # Steps (binomial only)
#     "q": 0              # Dividend yield (optional)
#   }

suppressPackageStartupMessages({
  library(jsonlite)
})

# derivmkts is optional: when installed, its prices are reported next to the
# hand-written Black-Scholes formulas below, so the two can be compared.
HAVE_DERIVMKTS <- requireNamespace("derivmkts", quietly = TRUE)

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

VERSION <- "1.0.0"
VALIDATOR_NAME <- "real_options_validator.R"

# ANSI color codes
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# ─────────────────────────────────────────────────────────────────────────────
# Black-Scholes Model
# ─────────────────────────────────────────────────────────────────────────────

# Black-Scholes call option price
bs_call <- function(S, K, r, sigma, T, q = 0) {
  if (T <= 0) return(max(S - K, 0))

  d1 <- (log(S / K) + (r - q + sigma^2 / 2) * T) / (sigma * sqrt(T))
  d2 <- d1 - sigma * sqrt(T)

  S * exp(-q * T) * pnorm(d1) - K * exp(-r * T) * pnorm(d2)
}

# Black-Scholes put option price
bs_put <- function(S, K, r, sigma, T, q = 0) {
  if (T <= 0) return(max(K - S, 0))

  d1 <- (log(S / K) + (r - q + sigma^2 / 2) * T) / (sigma * sqrt(T))
  d2 <- d1 - sigma * sqrt(T)

  K * exp(-r * T) * pnorm(-d2) - S * exp(-q * T) * pnorm(-d1)
}

# Calculate Greeks
calculate_greeks <- function(S, K, r, sigma, T, q = 0, option_type = "call") {
  if (T <= 0) {
    return(list(delta = NA, gamma = NA, theta = NA, vega = NA, rho = NA))
  }

  d1 <- (log(S / K) + (r - q + sigma^2 / 2) * T) / (sigma * sqrt(T))
  d2 <- d1 - sigma * sqrt(T)

  # Common terms
  nd1 <- dnorm(d1)
  Nd1 <- pnorm(d1)
  Nd2 <- pnorm(d2)

  if (option_type == "call") {
    delta <- exp(-q * T) * Nd1
    theta <- (-(S * sigma * exp(-q * T) * nd1) / (2 * sqrt(T))
              - r * K * exp(-r * T) * Nd2
              + q * S * exp(-q * T) * Nd1)
    rho <- K * T * exp(-r * T) * Nd2
  } else {
    delta <- exp(-q * T) * (Nd1 - 1)
    theta <- (-(S * sigma * exp(-q * T) * nd1) / (2 * sqrt(T))
              + r * K * exp(-r * T) * pnorm(-d2)
              - q * S * exp(-q * T) * pnorm(-d1))
    rho <- -K * T * exp(-r * T) * pnorm(-d2)
  }

  gamma <- exp(-q * T) * nd1 / (S * sigma * sqrt(T))
  vega <- S * exp(-q * T) * sqrt(T) * nd1

  # Scale theta to per-day
  theta_per_day <- theta / 365

  list(
    delta = delta,
    gamma = gamma,
    theta = theta_per_day,
    vega = vega / 100,  # per 1% change in vol
    rho = rho / 100     # per 1% change in rate
  )
}

# ─────────────────────────────────────────────────────────────────────────────
# Binomial Model
# ─────────────────────────────────────────────────────────────────────────────

# CRR Binomial Tree
binomial_option <- function(S, K, r, sigma, T, n, option_type = "call", american = FALSE, q = 0) {
  dt <- T / n
  u <- exp(sigma * sqrt(dt))
  d <- 1 / u
  p <- (exp((r - q) * dt) - d) / (u - d)

  # Discount factor
  disc <- exp(-r * dt)

  # Build terminal payoffs
  asset_prices <- S * d^(n:0) * u^(0:n)

  if (option_type == "call") {
    option_values <- pmax(asset_prices - K, 0)
  } else {
    option_values <- pmax(K - asset_prices, 0)
  }

  # Backward induction
  for (i in (n - 1):0) {
    asset_prices <- S * d^(i:0) * u^(0:i)
    option_values <- disc * (p * option_values[2:(i + 2)] + (1 - p) * option_values[1:(i + 1)])

    if (american) {
      if (option_type == "call") {
        intrinsic <- pmax(asset_prices - K, 0)
      } else {
        intrinsic <- pmax(K - asset_prices, 0)
      }
      option_values <- pmax(option_values, intrinsic)
    }
  }

  # Return option value and tree parameters
  list(
    price = option_values[1],
    u = u,
    d = d,
    p = p,
    dt = dt,
    n = n
  )
}

# ─────────────────────────────────────────────────────────────────────────────
# Real Options Specific Functions
# ─────────────────────────────────────────────────────────────────────────────

# Option to delay (call option on project)
option_to_delay <- function(V, I, r, sigma, T, q = 0) {
  # V = present value of project cash flows
  # I = investment cost (strike)
  value <- bs_call(V, I, r, sigma, T, q)
  npv_now <- V - I

  list(
    option_value = value,
    npv_if_invest_now = npv_now,
    value_of_waiting = value - max(npv_now, 0)
  )
}

# Option to expand
option_to_expand <- function(V, expansion_cost, expansion_factor, r, sigma, T, q = 0) {
  # Value of expansion option = call on (expansion_factor - 1) * V
  additional_value <- V * (expansion_factor - 1)
  value <- bs_call(additional_value, expansion_cost, r, sigma, T, q)

  list(
    option_value = value,
    additional_capacity_value = additional_value,
    expansion_cost = expansion_cost
  )
}

# Option to abandon (put option)
option_to_abandon <- function(V, salvage_value, r, sigma, T, q = 0) {
  value <- bs_put(V, salvage_value, r, sigma, T, q)

  list(
    option_value = value,
    salvage_value = salvage_value,
    current_project_value = V
  )
}

# ─────────────────────────────────────────────────────────────────────────────
# JSON API Mode
# ─────────────────────────────────────────────────────────────────────────────

run_json_mode <- function(params_json) {
  tryCatch({
    params <- fromJSON(params_json, simplifyVector = FALSE)

    # Extract common parameters
    S <- params$S
    K <- params$K
    r <- if (!is.null(params$r)) params$r else 0.05
    sigma <- if (!is.null(params$sigma)) params$sigma else 0.3
    T <- if (!is.null(params$T)) params$T else 1
    q <- if (!is.null(params$q)) params$q else 0
    option_type <- if (!is.null(params$option_type)) tolower(params$option_type) else "call"
    model <- if (!is.null(params$model)) tolower(params$model) else "black_scholes"

    if (is.null(S) || is.null(K)) {
      stop("Real options require 'S' (asset value) and 'K' (strike/investment cost)")
    }

    results <- list()

    if (model == "binomial") {
      n <- if (!is.null(params$n)) params$n else 100
      american <- if (!is.null(params$american)) params$american else FALSE

      bin_result <- binomial_option(S, K, r, sigma, T, n, option_type, american, q)
      results$price <- bin_result$price
      results$model <- "binomial"
      results$steps <- n
      results$american <- american
      results$u <- bin_result$u
      results$d <- bin_result$d
      results$p <- bin_result$p

    } else {
      # Black-Scholes
      if (option_type == "call") {
        price <- bs_call(S, K, r, sigma, T, q)
      } else {
        price <- bs_put(S, K, r, sigma, T, q)
      }

      greeks <- calculate_greeks(S, K, r, sigma, T, q, option_type)

      results$price <- price
      if (HAVE_DERIVMKTS) {
        results$price_derivmkts <- tryCatch(
          if (option_type == "call") derivmkts::bscall(S, K, sigma, r, T, q)
          else derivmkts::bsput(S, K, sigma, r, T, q),
          error = function(e) NULL)
      }
      results$model <- "black_scholes"
      results$greeks <- greeks
    }

    # Add common outputs
    results$option_type <- option_type
    results$inputs <- list(S = S, K = K, r = r, sigma = sigma, T = T, q = q)

    # Intrinsic value
    if (option_type == "call") {
      results$intrinsic <- max(S - K, 0)
    } else {
      results$intrinsic <- max(K - S, 0)
    }
    results$time_value <- results$price - results$intrinsic

    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = TRUE,
      results = results
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
  cat(sprintf("\n%s=== Real Options Validator ===%s\n\n", BLUE, RESET))

  # Example parameters
  S <- 100      # Current project value
  K <- 100      # Investment cost
  r <- 0.05     # Risk-free rate
  sigma <- 0.30 # Volatility
  T <- 1        # Time to expiration

  cat(sprintf("%sExample: Option to Delay Investment%s\n", BLUE, RESET))
  cat(sprintf("Project value (S): $%d\n", S))
  cat(sprintf("Investment cost (K): $%d\n", K))
  cat(sprintf("Risk-free rate (r): %.1f%%\n", r * 100))
  cat(sprintf("Volatility (sigma): %.1f%%\n", sigma * 100))
  cat(sprintf("Time to decide (T): %d year\n\n", T))

  # Black-Scholes
  call_price <- bs_call(S, K, r, sigma, T)
  put_price <- bs_put(S, K, r, sigma, T)

  cat(sprintf("%sBlack-Scholes Results:%s\n", BLUE, RESET))
  cat(sprintf("  Call (option to invest): $%.4f\n", call_price))
  cat(sprintf("  Put (option to abandon): $%.4f\n", put_price))

  # Greeks
  greeks <- calculate_greeks(S, K, r, sigma, T, 0, "call")
  cat(sprintf("\n%sGreeks (Call):%s\n", BLUE, RESET))
  cat(sprintf("  Delta: %.4f\n", greeks$delta))
  cat(sprintf("  Gamma: %.6f\n", greeks$gamma))
  cat(sprintf("  Theta: $%.4f/day\n", greeks$theta))
  cat(sprintf("  Vega: $%.4f/1%% vol\n", greeks$vega))
  cat(sprintf("  Rho: $%.4f/1%% rate\n", greeks$rho))

  # Binomial comparison
  cat(sprintf("\n%sBinomial Model Comparison:%s\n", BLUE, RESET))
  for (n in c(10, 50, 100, 500)) {
    bin <- binomial_option(S, K, r, sigma, T, n, "call")
    diff <- abs(bin$price - call_price)
    cat(sprintf("  n=%3d: $%.4f (diff: $%.6f)\n", n, bin$price, diff))
  }

  # Real options interpretation
  cat(sprintf("\n%sReal Options Interpretation:%s\n", BLUE, RESET))
  npv_now <- S - K
  cat(sprintf("  NPV if invest now: $%.2f\n", npv_now))
  cat(sprintf("  Value of option to delay: $%.4f\n", call_price))
  cat(sprintf("  Value of waiting: $%.4f\n", call_price - max(npv_now, 0)))

  if (call_price > max(npv_now, 0)) {
    cat(sprintf("\n%sRecommendation: WAIT - option value exceeds NPV%s\n", GREEN, RESET))
  } else {
    cat(sprintf("\n%sRecommendation: INVEST NOW - NPV exceeds option value%s\n", GREEN, RESET))
  }

  cat(sprintf("\n%s=== Real Options Validation Complete ===%s\n\n", BLUE, RESET))
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
