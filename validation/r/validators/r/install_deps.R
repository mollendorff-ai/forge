#!/usr/bin/env Rscript
# Cross-platform R dependency installer for forge-e2e validators
#
# Usage:
#   Rscript install_deps.R           # Install all dependencies
#   Rscript install_deps.R --check   # Check which are missing
#
# Works on: Linux, macOS, Windows
# Requires: R 4.0+ with internet access

# ─────────────────────────────────────────────────────────────────────────────
# Required packages by phase
# ─────────────────────────────────────────────────────────────────────────────

PACKAGES <- list(
  # Core (always needed)
  core = c("jsonlite"),

  # Phase 2: Monte Carlo & Bootstrap
  phase2 = c("mc2d", "boot"),

  # Phase 3: Tornado & Sensitivity
  phase3 = c("tornado", "sensitivity"),

  # Phase 4: Decision Trees & Real Options
  phase4 = c("data.tree", "derivmkts"),

  # Phase 5: Bayesian
  phase5 = c("bnlearn")
)

ALL_PACKAGES <- unique(unlist(PACKAGES))

# ─────────────────────────────────────────────────────────────────────────────
# Functions
# ─────────────────────────────────────────────────────────────────────────────

check_packages <- function() {
  cat("Checking R packages for forge-e2e validators...\n\n")

  installed <- installed.packages()[, "Package"]

  for (phase in names(PACKAGES)) {
    cat(sprintf("=== %s ===\n", toupper(phase)))
    for (pkg in PACKAGES[[phase]]) {
      status <- if (pkg %in% installed) "\033[32m[OK]\033[0m" else "\033[31m[MISSING]\033[0m"
      cat(sprintf("  %s %s\n", status, pkg))
    }
    cat("\n")
  }

  missing <- setdiff(ALL_PACKAGES, installed)
  if (length(missing) == 0) {
    cat("\033[32mAll packages installed!\033[0m\n")
  } else {
    cat(sprintf("\033[33mMissing %d packages: %s\033[0m\n",
                length(missing), paste(missing, collapse = ", ")))
    cat("Run: Rscript install_deps.R\n")
  }

  invisible(missing)
}

install_packages <- function() {
  cat("Installing R packages for forge-e2e validators...\n")
  cat(sprintf("Platform: %s\n", R.version$platform))
  cat(sprintf("R version: %s\n\n", R.version.string))

  installed <- installed.packages()[, "Package"]
  missing <- setdiff(ALL_PACKAGES, installed)

  if (length(missing) == 0) {
    cat("\033[32mAll packages already installed!\033[0m\n")
    return(invisible(TRUE))
  }

  cat(sprintf("Installing %d packages: %s\n\n", length(missing), paste(missing, collapse = ", ")))

  # Use CRAN mirror
  repos <- "https://cloud.r-project.org"

  # Install missing packages
  for (pkg in missing) {
    cat(sprintf("Installing %s...\n", pkg))
    tryCatch({
      install.packages(pkg, repos = repos, quiet = TRUE)
      cat(sprintf("  \033[32m[OK]\033[0m %s installed\n", pkg))
    }, error = function(e) {
      cat(sprintf("  \033[31m[FAIL]\033[0m %s: %s\n", pkg, conditionMessage(e)))
    })
  }

  # Verify
  cat("\n=== Verification ===\n")
  installed <- installed.packages()[, "Package"]
  still_missing <- setdiff(ALL_PACKAGES, installed)

  if (length(still_missing) == 0) {
    cat("\033[32mAll packages installed successfully!\033[0m\n")
    return(invisible(TRUE))
  } else {
    cat(sprintf("\033[31mFailed to install: %s\033[0m\n", paste(still_missing, collapse = ", ")))
    cat("\nTry running with sudo (Linux/macOS) or as Administrator (Windows)\n")
    return(invisible(FALSE))
  }
}

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

args <- commandArgs(trailingOnly = TRUE)

if ("--check" %in% args) {
  check_packages()
} else if ("--help" %in% args || "-h" %in% args) {
  cat("Usage: Rscript install_deps.R [--check | --help]\n\n")
  cat("Options:\n")
  cat("  (none)    Install all missing packages\n")
  cat("  --check   Check which packages are missing\n")
  cat("  --help    Show this help\n")
} else {
  install_packages()
}
