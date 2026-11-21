"""The symmetric mixed equilibria of symmetric games of two strategies
(ADR-073, question 4).

When every other player plays the first strategy with probability x, it pays
more than the second by g(x) = sum_k C(n-1, k) x^k (1-x)^(n-1-k) d_k, where
d_k is the difference with k of the others playing it. Forge lists each root
of g in (0, 1) as an equilibrium in which every player plays alike, after the
pure ones, and says whether a population returns to it. The rules, each a
failure when broken:

  1. The equilibria in which every player mixes alike are the roots of g in
     (0, 1), share for share, found exactly by sympy's real-root isolation
     of g with its fractions.
  2. Every such equilibrium Gambit finds (`enumpoly_solve` on the whole
     table, keeping those in which every player's mix is the same and not
     pure) is one of Forge's. Gambit's method is numerical and misses some
     near the ends of the interval, which the exact roots then confirm; the
     run counts them.
  3. Each one's payoff is what the first strategy pays against it, worked
     out here with exact fractions.
  4. `stable` is 1 exactly when g falls through 0 there (g' < 0, from the
     exact coefficients); for a pure one in which all play the second
     strategy, when d_0 < 0, and the first, when d_(n-1) > 0. An equilibrium
     in which players play differently has no `stable`, nor has any in a
     game where every d_k is 0, in which a population stands still.
  5. The report says the symmetric mixes were looked for (or, for two
     players, that every mixed equilibrium was).

The cases are the volunteer's dilemma of two to eight players, whose mix has
the closed form 1 - (c / b)^(1 / (n - 1)); the public goods game at r = n,
where every mix is an equilibrium and none is listed; and games drawn from a
seed: three to five players, each difference a fraction from a wide range.
"""
import random
from fractions import Fraction as F
from math import comb

import counted

EPS = 1e-6


def volunteers(n, b=10, c=4):
    """Volunteering pays b - c; staying out b if anyone volunteers, else 0."""
    formulas = {"volunteer": f"={b} - {c}",
                "stay_out": f"=IF(vd.others.volunteer > 0, {b}, 0)"}
    text = counted.model("vd", n, ["volunteer", "stay_out"], formulas)

    def pay(own, counts):
        return F(b - c) if own == 0 else (F(b) if counts[0] > 0 else F(0))
    share = 1 - (c / b) ** (1 / (n - 1))
    return "vd", f"volunteer's dilemma, {n} players", text, n, ["volunteer", "stay_out"], pay, share


def public_goods():
    name, kind, text, n, strategies, pay = counted.public_goods(F(5))
    return name, kind, text, n, strategies, pay, None


def draw(rng, index):
    n = rng.randint(3, 5)
    strategies = ["x", "y"]
    values = [{c: F(rng.randint(-400, 400), 40) for c in counted.splits(n - 1, 2)}
              for _ in strategies]
    name = f"s{index}"
    text = counted.model(name, n, strategies, {s: counted.drawn_formula(name, strategies, values[i])
                                               for i, s in enumerate(strategies)})
    return (name, f"{n} players", text, n, strategies,
            lambda own, counts: values[own][counts], None)


def advantage(n, pay):
    """d_k for k = 0 to n - 1, exactly."""
    return [pay(0, (k, n - 1 - k)) - pay(1, (k, n - 1 - k)) for k in range(n)]


def bernstein(coefficients, x):
    m = len(coefficients) - 1
    return sum(comb(m, k) * x ** k * (1 - x) ** (m - k) * c for k, c in enumerate(coefficients))


def slope(d, x):
    """g'(x): (n-1) times the Bernstein form of the differences of d."""
    m = len(d) - 1
    return m * bernstein([d[k + 1] - d[k] for k in range(m)], x)


def exact_roots(d):
    """The roots of g in (0, 1), by sympy's real-root isolation."""
    from sympy import Poly, Rational, real_roots, symbols
    x = symbols("x")
    m = len(d) - 1
    g = sum(comb(m, k) * x ** k * (1 - x) ** (m - k) * Rational(v.numerator, v.denominator)
            for k, v in enumerate(d))
    return sorted({float(r.evalf(30)) for r in real_roots(Poly(g, x)) if 0 < r < 1})


def gambit_alike(n, table):
    """Gambit's equilibria in which every player mixes the same, not pure:
    the share of the first strategy of each."""
    import numpy
    import pygambit as gbt
    arrays = []
    for player in range(n):
        a = numpy.empty((2,) * n, dtype=object)
        for profile, pays in table.items():
            a[profile] = pays[player]
        arrays.append(a)
    game = gbt.Game.from_arrays(*arrays)
    shares = []
    for eq in gbt.nash.enumpoly_solve(game).equilibria:
        first = [float(eq[list(p.strategies)[0]]) for p in game.players]
        if max(first) - min(first) < EPS and EPS < first[0] < 1 - EPS:
            shares.append(first[0])
    shares.sort()
    # Gambit can find one equilibrium from two supports
    out = []
    for s in shares:
        if not out or abs(s - out[-1]) > EPS:
            out.append(s)
    return out


def check(report, n, strategies, pay, closed):
    """What is wrong with Forge's report of this game, how many mixes it
    lists, and how many of them Gambit missed."""
    bad = []
    players = report["players"]
    first = strategies[0]
    d = advantage(n, pay)
    degenerate = all(v == 0 for v in d)
    mixes = []
    for k, eq in enumerate(report["equilibria"], 1):
        plays = [eq["players"][pl]["strategies"][first] for pl in players]
        alike = max(plays) - min(plays) < 1e-9
        x = plays[0]
        if eq["kind"] == "mixed":
            if not alike:
                bad.append(f"equilibrium_{k}: players mix differently, {plays}")
                continue
            mixes.append(x)
            want = float(bernstein([pay(0, (j, n - 1 - j)) for j in range(n)], F(x)))
            got = eq["players"][players[0]]["payoff"]
            if abs(got - want) > EPS * max(1, abs(want)):
                bad.append(f"equilibrium_{k}: payoff {got}, worked out here {want}")
            stable = float(slope(d, F(x))) < 0
        elif alike and not degenerate:
            stable = d[0] < 0 if x == 0 else d[-1] > 0
        else:
            stable = None
        if eq.get("stable") != stable:
            bad.append(f"equilibrium_{k} at {x}: stable {eq.get('stable')}, here {stable}")
    if degenerate:
        if mixes or not report["ties"]:
            bad.append(f"degenerate (every d is 0): mixes {mixes}, ties {report['ties']}")
        return bad, 0, 0
    if mixes != sorted(mixes):
        bad.append(f"mixes not by increasing share: {mixes}")
    exact = exact_roots(d)
    if len(exact) != len(mixes) or any(abs(a - b) > EPS for a, b in zip(mixes, exact)):
        bad.append(f"mixes {mixes}, the exact roots {exact}")
    gambit = gambit_alike(n, counted.expand(n, strategies, pay))
    if any(all(abs(g - x) > EPS for x in mixes) for g in gambit):
        bad.append(f"mixes {mixes}, Gambit's {gambit}")
    if closed is not None and (len(mixes) != 1 or abs(mixes[0] - closed) > 1e-12):
        bad.append(f"mixes {mixes}, the closed form {closed}")
    searched = report["mixed_searched"] if n == 2 else report["symmetric_searched"]
    if not searched:
        bad.append("the report does not say the symmetric mixes were looked for")
    return bad, len(mixes), max(0, len(mixes) - len(gambit))


def cases(seed, count):
    rng = random.Random(seed)
    out = [volunteers(n) for n in range(2, 9)] + [public_goods()]
    return out + [draw(rng, i) for i in range(count)]

