# Symmetric games of many players (ADR-073), against Forge's report, worked
# out again in R from the payoffs alone: no code of Forge's or of the Python
# suite's checkers.
#
# For each case, run.py gives the game's payoffs in the count form, pay(s, c)
# for strategy s and each split c of the other n - 1 players, and the report
# `forge game -o` wrote for it. R checks:
#
#   1. The dynamics (when the report has them). What strategy s pays against
#      the population x is f_s = sum over splits c of
#      (n - 1)! / prod(c_i!) * prod(x_i^c_i) * pay(s, c), and one generation
#      is x_s' = x_s (f_s + b) / sum_t x_t (f_t + b). From each of Forge's
#      generations, R's next shares and the average payoff sum_s x_s f_s are
#      Forge's within 1e-9. Where the end is known by hand, R's own path from
#      the start, generation by generation, ends there within 1e-9, and so
#      does Forge's.
#   2. The mixed equilibria of a game of two strategies in which all play
#      alike. They are the roots in (0, 1) of
#        g(x) = sum_k C(n - 1, k) x^k (1 - x)^(n - 1 - k) d_k,
#      d_k = pay(1, (k, n - 1 - k)) - pay(2, (k, n - 1 - k)), found here by
#      R's polyroot (Jenkins-Traub) on g in powers of x and polished by
#      Newton's method; one is stable when g' < 0 there. Forge's list must be
#      the same roots within 1e-6, each with that stability, and its payoff
#      f_1 at the root. A game with a tie (Forge names it) is not compared,
#      nor one whose g has a root of multiplicity two or more, which polyroot
#      places only to the square root of the precision: the Python suite
#      checks those against the exact roots (symmetric.py).
#   3. The closed forms: the volunteer's dilemma volunteers with
#      1 - (c / b)^(1 / (n - 1)); the public goods game of n players has one
#      pure equilibrium, all keeping, for r < n, every one of the 2^n
#      profiles at r = n, and one, all contributing, for r > n.
#
# Usage: Rscript many_players.R cases.json
library(jsonlite)

args <- commandArgs(trailingOnly = TRUE)
cases <- fromJSON(args[1], simplifyVector = FALSE)

failures <- character(0)
counts <- c(generations = 0, games = 0, dynamics = 0, roots = 0, skipped = 0, closed = 0)

# What each strategy pays against the shares x
fitness <- function(x, pay, n) {
  vapply(pay, function(table) {
    total <- 0
    for (cell in table) {
      split <- unlist(cell$split)
      total <- total + factorial(n - 1) / prod(factorial(split)) * prod(x^split) * cell$value
    }
    total
  }, numeric(1))
}
step <- function(x, pay, n, b) {
  part <- x * (fitness(x, pay, n) + b)
  part / sum(part)
}
# The payoff of strategy s with k of the others playing the first
pay2 <- function(pay, s, k) {
  for (cell in pay[[s]]) if (unlist(cell$split)[1] == k) return(cell$value)
  stop("no split")
}

for (case in cases) {
  report <- case$report
  n <- case$n
  strategies <- unlist(case$strategies)
  pay <- case$pay
  fail <- function(why) failures <<- c(failures, paste0(case$name, " (", case$kind, "): ", why))
  counts["games"] <- counts["games"] + 1

  # 1. The dynamics
  d <- report$dynamics
  if (!is.null(d)) {
    counts["dynamics"] <- counts["dynamics"] + 1
    b <- d$baseline
    path <- lapply(d$path, function(g) unlist(g$shares[strategies]))
    if (length(path) != d$generations + 1) fail(sprintf("%d generations in the path, not %d", length(path), d$generations + 1))
    bad <- 0
    for (t in seq_along(path)) {
      x <- path[[t]]
      f <- fitness(x, pay, n)
      average <- sum(x * f)
      if (abs(d$path[[t]]$payoff - average) > 1e-9 * max(1, abs(average))) {
        bad <- bad + 1
        if (bad <= 3) fail(sprintf("generation %d: average payoff %.12g, R %.12g", t - 1, d$path[[t]]$payoff, average))
      }
      if (t < length(path)) {
        want <- step(x, pay, n, b)
        if (any(abs(path[[t + 1]] - want) > 1e-9)) {
          bad <- bad + 1
          if (bad <= 3) fail(sprintf("generation %d: shares %s, R's step %s", t,
                                     paste(signif(path[[t + 1]], 12), collapse = ", "), paste(signif(want, 12), collapse = ", ")))
        }
      }
      counts["generations"] <- counts["generations"] + 1
    }
    if (!is.null(case$known)) {
      known <- unlist(case$known)
      own <- path[[1]]
      for (t in seq_len(d$generations)) own <- step(own, pay, n, b)
      if (any(abs(own - known) > 1e-9)) fail(sprintf("R's own path ends at %s, by hand %s", paste(own, collapse = ", "), paste(known, collapse = ", ")))
      if (any(abs(path[[length(path)]] - known) > 1e-9)) fail(sprintf("Forge ends at %s, by hand %s", paste(path[[length(path)]], collapse = ", "), paste(known, collapse = ", ")))
    }
  }

  # 2. The mixed equilibria in which all play alike, two strategies
  if (length(strategies) == 2 && isTRUE(case$roots) && length(report$ties) == 0) {
    m <- n - 1
    dk <- vapply(0:m, function(k) pay2(pay, 1, k) - pay2(pay, 2, k), numeric(1))
    power <- numeric(m + 1)
    for (k in 0:m) for (j in 0:(m - k)) {
      power[k + j + 1] <- power[k + j + 1] + choose(m, k) * dk[k + 1] * choose(m - k, j) * (-1)^j
    }
    full <- power
    g <- function(x) sum(full * x^(0:m))
    dg <- function(x) if (m == 0) 0 else sum(full[-1] * (1:m) * x^(0:(m - 1)))
    while (length(power) > 1 && abs(power[length(power)]) < 1e-12) power <- power[-length(power)]
    candidates <- if (length(power) > 1) polyroot(power) else complex(0)
    roots <- numeric(0)
    multiple <- FALSE
    for (z in candidates) {
      if (abs(Im(z)) > 1e-3 || Re(z) < -1e-3 || Re(z) > 1 + 1e-3) next
      x <- Re(z)
      for (i in 1:50) { s <- dg(x); if (s == 0) break; x <- x - g(x) / s }
      # A root near the axis that is not on it, or where g' vanishes, is
      # a root of multiplicity two or more
      if (abs(Im(z)) > 1e-7 || abs(dg(x)) < 1e-6 * max(1, max(abs(power)))) { multiple <- TRUE; next }
      if (x > 1e-9 && x < 1 - 1e-9) roots <- c(roots, x)
    }
    roots <- sort(roots)
    if (multiple) {
      counts["skipped"] <- counts["skipped"] + 1
    } else {
      forge <- Filter(function(e) e$kind == "mixed", report$equilibria)
      first <- unlist(report$players)[1]
      got <- sort(vapply(forge, function(e) e$players[[first]]$strategies[[strategies[1]]], numeric(1)))
      if (length(got) != length(roots) || any(abs(got - roots) > 1e-6)) {
        fail(sprintf("Forge's mixes %s, R's roots %s", paste(signif(got, 10), collapse = ", "), paste(signif(roots, 10), collapse = ", ")))
      } else {
        counts["roots"] <- counts["roots"] + length(roots)
        for (e in forge) {
          x <- e$players[[first]]$strategies[[strategies[1]]]
          r <- roots[which.min(abs(roots - x))]
          if (!is.null(e$stable) && e$stable != (dg(r) < 0)) fail(sprintf("the mix at %.8f is %s in Forge; g' there is %.3g", x, if (e$stable) "stable" else "unstable", dg(r)))
          f1 <- fitness(c(r, 1 - r), pay, n)[1]
          for (p in e$players) {
            if (abs(p$payoff - f1) > 1e-6 * max(1, abs(f1))) fail(sprintf("the mix at %.8f pays %.10g in Forge, %.10g here", x, p$payoff, f1))
          }
        }
      }
    }
  }

  # 3. The closed forms
  if (!is.null(case$volunteer)) {
    counts["closed"] <- counts["closed"] + 1
    want <- 1 - (case$volunteer$c / case$volunteer$b)^(1 / (n - 1))
    forge <- Filter(function(e) e$kind == "mixed", report$equilibria)
    first <- unlist(report$players)[1]
    got <- vapply(forge, function(e) e$players[[first]]$strategies[["volunteer"]], numeric(1))
    if (length(got) != 1 || abs(got - want) > 1e-7) fail(sprintf("volunteers with %s, the closed form %.12f", paste(got, collapse = ", "), want))
  }
  if (!is.null(case$public_goods)) {
    counts["closed"] <- counts["closed"] + 1
    r <- case$public_goods$r
    pure <- Filter(function(e) e$kind == "pure", report$equilibria)
    want <- if (r == n) 2^n else 1
    if (length(pure) != want) fail(sprintf("r = %g: %d pure equilibria, the closed form %d", r, length(pure), want))
    if (r != n && length(pure) == 1) {
      everyone <- if (r < n) "keep" else "contribute"
      plays <- vapply(pure[[1]]$players, function(p) p$strategies[[everyone]], numeric(1))
      if (any(plays != 1)) fail(sprintf("r = %g: the equilibrium is not everyone playing %s", r, everyone))
    }
  }
}

cat(sprintf(paste0("R, games of many players: %d games, %d with dynamics (%d generations stepped again), ",
                   "%d mixed equilibria found as roots (%d games with a multiple root left to symmetric.py), ",
                   "%d closed forms, %d failed\n"),
            counts["games"], counts["dynamics"], counts["generations"], counts["roots"], counts["skipped"],
            counts["closed"], length(failures)))
for (f in failures) cat("FAIL ", f, "\n", sep = "")
if (length(failures) > 0) quit(status = 1)
