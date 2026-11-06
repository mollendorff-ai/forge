#!/usr/bin/env Rscript
# Bayesian Network Validator for forge-e2e
# Validates forge's Bayesian network inference against R bnlearn calculations
#
# Usage:
#   Human-readable mode: Rscript bayesian_validator.R
#   JSON API mode:       Rscript bayesian_validator.R --json '<params>'
#
# JSON params format:
#   {
#     "network": {
#       "nodes": ["Rain", "Sprinkler", "Grass"],
#       "arcs": [["Rain", "Grass"], ["Sprinkler", "Grass"]],
#       "cpts": {
#         "Rain": {"probs": [0.8, 0.2], "levels": ["no", "yes"]},
#         "Sprinkler": {"probs": [0.6, 0.4], "levels": ["off", "on"]},
#         "Grass": {
#           "probs": [[0.9, 0.1], [0.2, 0.8], [0.1, 0.9], [0.01, 0.99]],
#           "parents": ["Rain", "Sprinkler"],
#           "levels": ["dry", "wet"]
#         }
#       }
#     },
#     "query": {
#       "target": "Rain",
#       "evidence": {"Grass": "wet"}
#     }
#   }

suppressPackageStartupMessages({
  library(jsonlite)
  library(bnlearn)
})

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

VERSION <- "1.0.0"
VALIDATOR_NAME <- "bayesian_validator.R"

# ANSI color codes
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# ─────────────────────────────────────────────────────────────────────────────
# Network Building Functions
# ─────────────────────────────────────────────────────────────────────────────

# Build a Bayesian network from JSON specification
build_network <- function(network_spec) {
  nodes <- unlist(network_spec$nodes)
  arcs_list <- network_spec$arcs
  cpts <- network_spec$cpts

  # Create empty network with nodes
  dag <- empty.graph(nodes)

  # Add arcs
  if (!is.null(arcs_list) && length(arcs_list) > 0) {
    arcs_matrix <- do.call(rbind, lapply(arcs_list, function(a) c(a[[1]], a[[2]])))
    arcs(dag) <- arcs_matrix
  }

  # Build CPT list for bn.fit
  cpt_list <- list()

  for (node in nodes) {
    cpt_spec <- cpts[[node]]
    levels <- unlist(cpt_spec$levels)
    probs <- unlist(cpt_spec$probs)
    parents <- unlist(cpt_spec$parents)

    if (is.null(parents) || length(parents) == 0) {
      # Root node: simple probability table
      cpt <- array(probs, dim = length(probs), dimnames = list(levels))
    } else {
      # Node with parents: conditional probability table
      parent_levels <- lapply(parents, function(p) unlist(cpts[[p]]$levels))
      names(parent_levels) <- parents

      # Dimensions: [node levels, parent1 levels, parent2 levels, ...]
      dims <- c(length(levels), sapply(parent_levels, length))
      dim_names <- c(list(levels), parent_levels)
      names(dim_names) <- c(node, parents)

      # The specs list each table row by row: one row per state of this node,
      # one column per combination of parent states (first parent varies
      # fastest). R fills arrays column by column, so lay the rows out first.
      cpt <- array(matrix(probs, nrow = length(levels), byrow = TRUE),
                   dim = dims, dimnames = dim_names)
    }

    cpt_list[[node]] <- cpt
  }

  # Create fitted network
  fitted <- custom.fit(dag, cpt_list)

  return(fitted)
}

# ─────────────────────────────────────────────────────────────────────────────
# Inference Functions
# ─────────────────────────────────────────────────────────────────────────────

# Inference using rejection sampling
query_network <- function(fitted, target, evidence = NULL, n_samples = 100000) {
  # Generate samples from the network
  samples <- rbn(fitted, n = n_samples)

  target_levels <- dimnames(fitted[[target]]$prob)[[1]]

  if (is.null(evidence) || length(evidence) == 0) {
    # No evidence: return marginal distribution
    probs <- table(samples[[target]]) / nrow(samples)
    result <- as.vector(probs[target_levels])
    names(result) <- target_levels
    return(result)
  }

  # With evidence: filter samples
  mask <- rep(TRUE, nrow(samples))
  for (var_name in names(evidence)) {
    mask <- mask & (samples[[var_name]] == evidence[[var_name]])
  }
  filtered <- samples[mask, ]

  if (nrow(filtered) == 0) {
    warning("No samples match evidence - returning uniform distribution")
    probs <- rep(1/length(target_levels), length(target_levels))
    names(probs) <- target_levels
    return(probs)
  }

  # Calculate posterior
  probs <- table(filtered[[target]]) / nrow(filtered)
  result <- numeric(length(target_levels))
  names(result) <- target_levels
  for (level in target_levels) {
    if (level %in% names(probs)) {
      result[level] <- probs[level]
    } else {
      result[level] <- 0
    }
  }

  return(result)
}

# Calculate joint probability P(X=x, Y=y, ...)
joint_probability <- function(fitted, values, n_samples = 100000) {
  # Generate samples from the network
  samples <- rbn(fitted, n = n_samples)

  # Count matching samples
  mask <- rep(TRUE, nrow(samples))
  for (var_name in names(values)) {
    mask <- mask & (samples[[var_name]] == values[[var_name]])
  }

  return(sum(mask) / nrow(samples))
}

# Calculate marginal probability for all nodes
marginal_probabilities <- function(fitted, n_samples = 100000) {
  # Generate samples from the network
  samples <- rbn(fitted, n = n_samples)

  marginals <- list()

  for (node in names(fitted)) {
    levels <- dimnames(fitted[[node]]$prob)[[1]]
    counts <- table(samples[[node]])
    probs <- numeric(length(levels))
    names(probs) <- levels

    for (level in levels) {
      if (level %in% names(counts)) {
        probs[level] <- counts[level] / nrow(samples)
      } else {
        probs[level] <- 0
      }
    }

    marginals[[node]] <- as.list(probs)
  }

  return(marginals)
}

# ─────────────────────────────────────────────────────────────────────────────
# JSON API Mode
# ─────────────────────────────────────────────────────────────────────────────

run_json_mode <- function(params_json) {
  set.seed(42)  # the same samples every run
  tryCatch({
    params <- fromJSON(params_json, simplifyVector = FALSE)

    if (is.null(params$network)) {
      stop("Bayesian network requires 'network' specification")
    }

    # Build network
    fitted <- build_network(params$network)

    results <- list()

    # If query specified, compute posterior
    if (!is.null(params$query)) {
      target <- params$query$target
      evidence <- params$query$evidence

      posterior <- query_network(fitted, target, evidence)
      results$posterior <- as.list(posterior)
      results$target <- target
      results$evidence <- evidence
    }

    # Compute marginals if requested or no query
    if (is.null(params$query) || isTRUE(params$compute_marginals)) {
      results$marginals <- marginal_probabilities(fitted)
    }

    # Joint probability if specified
    if (!is.null(params$joint)) {
      results$joint_probability <- joint_probability(fitted, params$joint)
    }

    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = TRUE,
      results = results
    )

    cat(toJSON(result, auto_unbox = TRUE, digits = 6))

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
  cat(sprintf("\n%s=== Bayesian Network Validator ===%s\n\n", BLUE, RESET))

  # Classic Rain-Sprinkler-Grass example
  cat(sprintf("%sExample: Rain-Sprinkler-Grass Network%s\n", BLUE, RESET))
  cat("Structure:\n")
  cat("  Rain -> Grass <- Sprinkler\n\n")

  # Build network manually
  dag <- empty.graph(c("Rain", "Sprinkler", "Grass"))
  arcs(dag) <- matrix(c("Rain", "Grass", "Sprinkler", "Grass"),
                      ncol = 2, byrow = TRUE)

  cat(sprintf("%sPrior Probabilities:%s\n", BLUE, RESET))
  cat("  P(Rain=yes) = 0.2\n")
  cat("  P(Sprinkler=on) = 0.4\n\n")

  cat(sprintf("%sConditional Probability Table for Grass:%s\n", BLUE, RESET))
  cat("  P(Grass=wet | Rain=no,  Sprinkler=off) = 0.0\n")
  cat("  P(Grass=wet | Rain=yes, Sprinkler=off) = 0.8\n")
  cat("  P(Grass=wet | Rain=no,  Sprinkler=on)  = 0.9\n")
  cat("  P(Grass=wet | Rain=yes, Sprinkler=on)  = 0.99\n\n")

  # Create fitted network
  cpt_rain <- array(c(0.8, 0.2), dim = 2, dimnames = list(c("no", "yes")))
  cpt_sprinkler <- array(c(0.6, 0.4), dim = 2, dimnames = list(c("off", "on")))
  cpt_grass <- array(c(1.0, 0.0, 0.2, 0.8, 0.1, 0.9, 0.01, 0.99),
                     dim = c(2, 2, 2),
                     dimnames = list(c("dry", "wet"), c("no", "yes"), c("off", "on")))

  fitted <- custom.fit(dag, list(Rain = cpt_rain,
                                 Sprinkler = cpt_sprinkler,
                                 Grass = cpt_grass))

  cat(sprintf("%sBayesian Inference:%s\n", BLUE, RESET))
  cat("Query: P(Rain | Grass=wet)\n")
  cat("Using Bayes' theorem:\n")
  cat("  P(Rain=yes | Grass=wet) = P(Grass=wet | Rain=yes) * P(Rain=yes) / P(Grass=wet)\n\n")

  # Generate samples from the network for inference
  set.seed(42)
  samples <- rbn(fitted, n = 100000)

  # Filter for evidence: Grass == "wet"
  wet_samples <- samples[samples$Grass == "wet", ]

  # Calculate posterior
  p_rain_given_wet <- mean(wet_samples$Rain == "yes")

  cat(sprintf("%sResult (rejection sampling with 100K samples):%s\n", GREEN, RESET))
  cat(sprintf("  P(Rain=yes | Grass=wet) = %.4f\n", p_rain_given_wet))
  cat(sprintf("  P(Rain=no  | Grass=wet) = %.4f\n", 1 - p_rain_given_wet))

  # Prior comparison
  cat(sprintf("\n%sInterpretation:%s\n", BLUE, RESET))
  cat(sprintf("  Prior P(Rain=yes) = 0.20\n"))
  cat(sprintf("  Posterior P(Rain=yes | Grass=wet) = %.2f\n", p_rain_given_wet))
  cat("  Observing wet grass increases our belief that it rained.\n")

  cat(sprintf("\n%s=== Bayesian Validation Complete ===%s\n\n", BLUE, RESET))
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
