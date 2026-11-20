"""Games drawn from a seed, 2 by 2 up to 6 by 6, with ties on purpose.

Some games draw small integer payoffs, so that two strategies pay the same
against something and the game is degenerate; others copy a strategy, hold a
row or column constant, or are zero-sum. The rest (more than half) draw payoffs
from 81 values, mostly without a tie, where the equilibria are isolated and
Forge and Gambit must find every one, and the game must not be reported as
degenerate. The same seed and count give the same games.
"""
import random
from fractions import Fraction as F

import tables

KINDS = ["distinct"] * 4 + ["symmetric"] * 2 + ["small", "small", "ties", "ties", "copied", "constant", "zero-sum"]


def matrix(rng, m, n, low, high, step):
    return [[F(rng.randint(low, high), step) for _ in range(n)] for _ in range(m)]


def draw(rng):
    """One game as (kind, a, b)."""
    kind = rng.choice(KINDS)
    m, n = rng.randint(2, 6), rng.randint(2, 6)
    if kind == "symmetric":
        n = m
    if kind in ("distinct", "symmetric"):
        a, b = matrix(rng, m, n, -40, 40, 4), matrix(rng, m, n, -40, 40, 4)
    elif kind == "small":
        a, b = matrix(rng, m, n, 0, 2, 1), matrix(rng, m, n, 0, 2, 1)
    else:
        a, b = matrix(rng, m, n, 0, 3, 1), matrix(rng, m, n, 0, 3, 1)
    if kind == "copied":
        # a row and a column pay as another does, for both players
        i, k = rng.sample(range(m), 2) if m > 1 else (0, 0)
        j, l = rng.sample(range(n), 2) if n > 1 else (0, 0)
        a[k], b[k] = list(a[i]), list(b[i])
        for r in range(m):
            a[r][l], b[r][l] = a[r][j], b[r][j]
    elif kind == "constant":
        j = rng.randrange(n)
        for r in range(m):
            a[r][j], b[r][j] = F(1), F(1)
        i = rng.randrange(m)
        a[i] = [F(2)] * n
    elif kind == "zero-sum":
        b = [[-v for v in row] for row in a]
    elif kind == "symmetric":
        b = [[a[j][i] for j in range(m)] for i in range(m)]
    return kind, a, b


def cases(seed, count):
    rng = random.Random(seed)
    out = []
    for k in range(count):
        kind, a, b = draw(rng)
        out.append((f"g{k}", kind, a, b))
    return out


def model_of(batch):
    """One model holding every game of a batch."""
    text = ['_forge_version: "5.5.0"', "games:"]
    for name, _, a, b in batch:
        body = tables.model(name, a, b).split("games:\n", 1)[1]
        text.append(body.rstrip("\n"))
    return "\n".join(text) + "\n"


def show(a, b):
    return "\n".join("   " + "  ".join(f"({tables.number(a[i][j])}, {tables.number(b[i][j])})"
                                       for j in range(len(a[0]))) for i in range(len(a)))
