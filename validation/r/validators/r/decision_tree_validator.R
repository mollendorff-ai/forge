#!/usr/bin/env Rscript
# Decision Tree Validator for forge-e2e
# Validates forge's decision tree analysis against R data.tree calculations
#
# Usage:
#   Human-readable mode: Rscript decision_tree_validator.R
#   JSON API mode:       Rscript decision_tree_validator.R --json '<params>'
#
# JSON params format:
#   {
#     "tree": {
#       "name": "Investment Decision",
#       "type": "decision",
#       "children": [
#         {
#           "name": "Invest",
#           "type": "chance",
#           "cost": 100000,
#           "children": [
#             {"name": "Success", "type": "terminal", "probability": 0.7, "payoff": 300000},
#             {"name": "Failure", "type": "terminal", "probability": 0.3, "payoff": 50000}
#           ]
#         },
#         {"name": "Don't Invest", "type": "terminal", "payoff": 0}
#       ]
#     }
#   }

suppressPackageStartupMessages({
  library(jsonlite)
  library(data.tree)
})

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

VERSION <- "1.0.0"
VALIDATOR_NAME <- "decision_tree_validator.R"

# ANSI color codes
GREEN <- "\033[32m"
RED <- "\033[31m"
YELLOW <- "\033[33m"
BLUE <- "\033[34m"
RESET <- "\033[0m"

# ─────────────────────────────────────────────────────────────────────────────
# Tree Building Functions
# ─────────────────────────────────────────────────────────────────────────────

# Build data.tree from JSON structure
build_tree <- function(node_spec, parent = NULL) {
  if (is.null(parent)) {
    # Root node
    tree <- Node$new(node_spec$name)
    tree$type <- if (!is.null(node_spec$type)) node_spec$type else "decision"
  } else {
    tree <- parent$AddChild(node_spec$name)
    tree$type <- if (!is.null(node_spec$type)) node_spec$type else "terminal"
  }

  # Set node attributes
  if (!is.null(node_spec$probability)) tree$probability <- node_spec$probability
  if (!is.null(node_spec$payoff)) tree$payoff <- node_spec$payoff
  if (!is.null(node_spec$cost)) tree$cost <- node_spec$cost

  # Recursively add children
  if (!is.null(node_spec$children)) {
    for (child_spec in node_spec$children) {
      build_tree(child_spec, tree)
    }
  }

  return(tree)
}

# ─────────────────────────────────────────────────────────────────────────────
# EMV Calculation (Backward Induction)
# ─────────────────────────────────────────────────────────────────────────────

# Calculate EMV using rollback/backward induction
calculate_emv <- function(node) {
  # Terminal node: payoff minus the cost of reaching it. The cost was ignored,
  # which gave 250000 instead of 130000 for sequential_decisions and 0 instead
  # of -50000 for negative_payoffs.
  if (node$type == "terminal" || node$isLeaf) {
    payoff <- if (!is.null(node$payoff)) node$payoff else 0
    cost <- if (!is.null(node$cost)) node$cost else 0
    node$emv <- payoff - cost
    return(payoff - cost)
  }

  # Get cost at this node (if any)
  cost <- if (!is.null(node$cost)) node$cost else 0

  # Calculate EMV for all children first
  child_emvs <- sapply(node$children, calculate_emv)

  if (node$type == "chance") {
    # Chance node: weighted average by probability
    probs <- sapply(node$children, function(c) {
      if (!is.null(c$probability)) c$probability else 1/length(node$children)
    })
    emv <- sum(child_emvs * probs) - cost
    node$emv <- emv
    node$decision <- NA
  } else if (node$type == "decision") {
    # Decision node: max of children
    emv <- max(child_emvs) - cost
    node$emv <- emv
    # Store optimal decision
    best_idx <- which.max(child_emvs)
    node$decision <- node$children[[best_idx]]$name
  } else {
    # Default: treat as decision node
    emv <- max(child_emvs) - cost
    node$emv <- emv
  }

  return(emv)
}

# Extract decision path (optimal choices)
get_decision_path <- function(node, path = c()) {
  if (node$isLeaf) {
    return(path)
  }

  if (node$type == "decision" && !is.null(node$decision)) {
    path <- c(path, node$decision)
    # Find the child with this name
    for (child in node$children) {
      if (child$name == node$decision) {
        return(get_decision_path(child, path))
      }
    }
  } else if (node$type == "chance") {
    # For chance nodes, we don't make decisions, follow highest EMV path for display
    if (length(node$children) > 0) {
      best_child <- node$children[[which.max(sapply(node$children, function(c) c$emv))]]
      return(get_decision_path(best_child, path))
    }
  }

  return(path)
}

# Get tree structure with EMV values
get_tree_structure <- function(node) {
  result <- list(
    name = node$name,
    type = node$type,
    emv = node$emv
  )

  if (!is.null(node$probability)) result$probability <- node$probability
  if (!is.null(node$payoff)) result$payoff <- node$payoff
  if (!is.null(node$cost)) result$cost <- node$cost
  if (!is.null(node$decision) && !is.na(node$decision)) result$decision <- node$decision

  if (!node$isLeaf) {
    result$children <- lapply(node$children, get_tree_structure)
  }

  return(result)
}

# ─────────────────────────────────────────────────────────────────────────────
# Risk Analysis
# ─────────────────────────────────────────────────────────────────────────────

# Calculate risk profile for a decision alternative
calculate_risk_profile <- function(node) {
  outcomes <- list()

  collect_outcomes <- function(n, prob_so_far = 1) {
    if (n$isLeaf || n$type == "terminal") {
      payoff <- if (!is.null(n$payoff)) n$payoff else n$emv
      outcomes[[length(outcomes) + 1]] <<- list(
        name = n$name,
        probability = prob_so_far,
        payoff = payoff
      )
    } else if (n$type == "chance") {
      for (child in n$children) {
        child_prob <- if (!is.null(child$probability)) child$probability else 1/length(n$children)
        collect_outcomes(child, prob_so_far * child_prob)
      }
    } else {
      # For decision nodes in risk profile, follow optimal path
      if (!is.null(n$decision)) {
        for (child in n$children) {
          if (child$name == n$decision) {
            collect_outcomes(child, prob_so_far)
            break
          }
        }
      }
    }
  }

  collect_outcomes(node)
  return(outcomes)
}

# ─────────────────────────────────────────────────────────────────────────────
# JSON API Mode
# ─────────────────────────────────────────────────────────────────────────────

run_json_mode <- function(params_json) {
  tryCatch({
    params <- fromJSON(params_json, simplifyVector = FALSE)

    if (is.null(params$tree)) {
      stop("Decision tree requires 'tree' specification")
    }

    # Build tree
    tree <- build_tree(params$tree)

    # Calculate EMV (modifies tree in place)
    root_emv <- calculate_emv(tree)

    # Get optimal decision path
    decision_path <- get_decision_path(tree)

    # Get risk profile for root decision alternatives
    risk_profiles <- list()
    if (!tree$isLeaf && tree$type == "decision") {
      for (child in tree$children) {
        risk_profiles[[child$name]] <- calculate_risk_profile(child)
      }
    }

    # Get full tree structure with EMVs
    tree_with_emv <- get_tree_structure(tree)

    result <- list(
      validator = VALIDATOR_NAME,
      version = VERSION,
      success = TRUE,
      results = list(
        root_emv = root_emv,
        optimal_decision = if (length(decision_path) > 0) decision_path[1] else NA,
        decision_path = decision_path,
        tree = tree_with_emv,
        risk_profiles = risk_profiles
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
  cat(sprintf("\n%s=== Decision Tree Validator ===%s\n\n", BLUE, RESET))

  # Example: Investment decision
  cat(sprintf("%sExample: Investment Decision%s\n", BLUE, RESET))
  cat("Should we invest $100,000?\n")
  cat("  If invest: 70% chance of $300,000, 30% chance of $50,000\n")
  cat("  If don't invest: $0\n\n")

  # Build the tree
  invest <- Node$new("Investment Decision")
  invest$type <- "decision"

  do_invest <- invest$AddChild("Invest")
  do_invest$type <- "chance"
  do_invest$cost <- 100000

  success <- do_invest$AddChild("Success")
  success$type <- "terminal"
  success$probability <- 0.7
  success$payoff <- 300000

  failure <- do_invest$AddChild("Failure")
  failure$type <- "terminal"
  failure$probability <- 0.3
  failure$payoff <- 50000

  dont_invest <- invest$AddChild("Don't Invest")
  dont_invest$type <- "terminal"
  dont_invest$payoff <- 0

  # Calculate EMV
  root_emv <- calculate_emv(invest)

  cat("EMV Calculation (Backward Induction):\n")
  cat(sprintf("  Success payoff: $%s\n", format(success$payoff, big.mark = ",")))
  cat(sprintf("  Failure payoff: $%s\n", format(failure$payoff, big.mark = ",")))
  cat(sprintf("  Invest EMV: 0.7 * $300,000 + 0.3 * $50,000 - $100,000 = $%s\n",
              format(do_invest$emv, big.mark = ",")))
  cat(sprintf("  Don't Invest EMV: $%s\n", format(dont_invest$emv, big.mark = ",")))
  cat(sprintf("\n%sOptimal Decision: %s%s\n", GREEN, invest$decision, RESET))
  cat(sprintf("%sRoot EMV: $%s%s\n", GREEN, format(root_emv, big.mark = ","), RESET))

  cat("\nTree Structure:\n")
  print(invest, "type", "probability", "payoff", "cost", "emv")

  cat(sprintf("\n%s=== Decision Tree Validation Complete ===%s\n\n", BLUE, RESET))
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
