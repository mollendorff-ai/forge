# The closed forms of a 2 x 2 game, against Forge's report.
#
# R has no package that finds the equilibria of a game, so R checks the one
# case with a formula. For payoffs a (the row player's) and b (the column
# player's), rows 1 and 2, columns 1 and 2:
#   a mixed equilibrium with both players mixing exists when
#     x = (b22 - b21) / (b11 - b12 - b21 + b22)   the chance of row 1
#     y = (a22 - a12) / (a11 - a12 - a21 + a22)   the chance of column 1
#   are both strictly between 0 and 1; each player's payoff there is
#     (a11 a22 - a12 a21) / (a11 - a12 - a21 + a22), and the same with b;
#   a pure equilibrium is a cell where neither player gains by changing.
# Forge must list exactly the pure equilibria, and the mixed one when it
# exists (and none that mixes both players when it does not, unless the game
# is reported as degenerate).
#
# Usage: Rscript closed_forms.R games.json
#   games.json: a list of {name, a, b, report}: the payoffs as 2 x 2 arrays
#   and the report `forge game -o` wrote for that game.
library(jsonlite)

eps <- 1e-7
args <- commandArgs(trailingOnly = TRUE)
games <- fromJSON(args[1], simplifyVector = FALSE)

as_matrix <- function(rows) matrix(unlist(rows), nrow = 2, byrow = TRUE)

failures <- character(0)
checked <- 0
mixed_found <- 0

for (g in games) {
  a <- as_matrix(g$a)
  b <- as_matrix(g$b)
  report <- g$report
  players <- unlist(report$players)
  rows <- unlist(report$strategies[[players[1]]])
  cols <- unlist(report$strategies[[players[2]]])
  fail <- function(why) failures <<- c(failures, paste0(g$name, ": ", why))
  checked <- checked + 1

  # What Forge lists, as (chance of row 1, chance of column 1)
  pure_forge <- list()
  mixed_forge <- list()
  for (e in report$equilibria) {
    x <- e$players[[players[1]]]$strategies[[rows[1]]]
    y <- e$players[[players[2]]]$strategies[[cols[1]]]
    if (e$kind == "pure") pure_forge[[length(pure_forge) + 1]] <- c(x, y)
    else mixed_forge[[length(mixed_forge) + 1]] <- c(x, y)
  }

  # Pure equilibria, cell by cell
  pure_r <- list()
  for (i in 1:2) for (j in 1:2) {
    if (a[i, j] >= a[3 - i, j] - eps && b[i, j] >= b[i, 3 - j] - eps) {
      pure_r[[length(pure_r) + 1]] <- c(if (i == 1) 1 else 0, if (j == 1) 1 else 0)
    }
  }
  same <- function(p, q) all(abs(p - q) < eps)
  in_list <- function(p, list) any(vapply(list, function(q) same(p, q), logical(1)))
  if (length(pure_forge) != length(pure_r) ||
      !all(vapply(pure_r, in_list, logical(1), list = pure_forge))) {
    fail(paste0("pure equilibria: Forge lists ", length(pure_forge), ", the cells give ", length(pure_r)))
  }

  # The mixed equilibrium of the closed form
  da <- a[1, 1] - a[1, 2] - a[2, 1] + a[2, 2]
  db <- b[1, 1] - b[1, 2] - b[2, 1] + b[2, 2]
  interior <- FALSE
  if (abs(da) > eps && abs(db) > eps) {
    x <- (b[2, 2] - b[2, 1]) / db
    y <- (a[2, 2] - a[1, 2]) / da
    interior <- x > eps && x < 1 - eps && y > eps && y < 1 - eps
  }
  if (interior) {
    mixed_found <- mixed_found + 1
    hit <- Filter(function(q) same(c(x, y), q), mixed_forge)
    if (length(hit) != 1) {
      fail(sprintf("the closed form gives x = %.6f, y = %.6f; Forge's mixed equilibria: %s", x, y,
                   paste(vapply(mixed_forge, function(q) sprintf("(%.6f, %.6f)", q[1], q[2]), ""), collapse = ", ")))
    } else {
      want <- c((a[1, 1] * a[2, 2] - a[1, 2] * a[2, 1]) / da, (b[1, 1] * b[2, 2] - b[1, 2] * b[2, 1]) / db)
      for (e in report$equilibria) {
        if (e$kind == "mixed" && same(c(e$players[[players[1]]]$strategies[[rows[1]]],
                                         e$players[[players[2]]]$strategies[[cols[1]]]), c(x, y))) {
          got <- c(e$players[[players[1]]]$payoff, e$players[[players[2]]]$payoff)
          if (!same(got, want)) fail(sprintf("payoffs %.6f, %.6f; the closed form gives %.6f, %.6f", got[1], got[2], want[1], want[2]))
        }
      }
    }
  } else if (length(report$ties) == 0 && length(mixed_forge) > 0) {
    fail("Forge lists a mixed equilibrium, and the closed form has none")
  }
}

cat(sprintf("R: %d 2 x 2 games, %d with a mixed equilibrium by the closed form, %d failed\n",
            checked, mixed_found, length(failures)))
for (f in failures) cat("FAIL ", f, "\n", sep = "")
if (length(failures) > 0) quit(status = 1)
