"""Symmetric games of three to five players in the count form (ADR-073).

A case is a symmetric game written as one payoff for each of a player's own
strategies, over how many of the other players play each
(`g.others.<strategy>`). Forge expands it into the whole table. This suite
expands it again, here, and the rules, each a failure when broken, are:

  1. Every cell of Forge's table (`cells` in `forge game -o`) has the payoffs
     of this expansion: the expansion is right cell by cell.
  2. The pure equilibria are Gambit's (`enumpure_solve` on the table expanded
     here), and the ones a check of every cell here finds.
  3. Each equilibrium's payoffs are the payoffs of its cell.
  4. Mixed equilibria are looked for only in a game of two strategies, and
     only those in which every player plays alike (ADR-073, question 4;
     symmetric.py checks them against Gambit): the report says so, and lists
     no other.

The cases are the public goods game of five players (below, at and above
r = n, by its own formulas), and games drawn from a seed: three to five
players, two or three strategies, payoffs from a small range (ties on purpose)
or a wide one, each written as a sum of IF terms over the counts.
"""
import itertools
import random
from fractions import Fraction as F

import tables

EPS = 1e-9


def splits(others, k):
    """Every way `others` players can fall among k strategies, as counts."""
    return [c for c in itertools.product(range(others + 1), repeat=k) if sum(c) == others]


def expand(n, strategies, pay):
    """The table: each profile, the last player moving fastest, with each
    player's payoff, pay(own strategy index, counts of the others)."""
    k = len(strategies)
    table = {}
    for profile in itertools.product(range(k), repeat=n):
        row = []
        for player, own in enumerate(profile):
            counts = [0] * k
            for other, s in enumerate(profile):
                if other != player:
                    counts[s] += 1
            row.append(pay(own, tuple(counts)))
        table[profile] = row
    return table


def model(name, n, strategies, formulas, inputs=""):
    players = ", ".join(f"p{i + 1}" for i in range(n))
    lines = ['_forge_version: "5.5.0"', inputs.rstrip("\n"), "games:", f"  {name}:",
             f"    players: [{players}]", f"    strategies: [{', '.join(strategies)}]",
             "    symmetric: true", "    payoffs:"]
    lines += [f"      {s}: {formulas[s]}" for s in strategies]
    return "\n".join(line for line in lines if line) + "\n"


def drawn_formula(name, strategies, values):
    """A payoff over the counts as a sum of IF terms, one for each split."""
    terms = []
    for counts, v in values.items():
        test = ", ".join(f"{name}.others.{s} = {c}" for s, c in zip(strategies, counts))
        terms.append(f"IF(AND({test}), {tables.number(v)}, 0)")
    return '"=' + " + ".join(terms) + '"'


def draw(rng, index):
    n, k = rng.randint(3, 5), rng.randint(2, 3)
    strategies = ["x", "y", "z"][:k]
    small = rng.random() < 0.5
    values = [{c: (F(rng.randint(0, 2)) if small else F(rng.randint(-40, 40), 4)) for c in splits(n - 1, k)}
              for _ in strategies]
    name = f"g{index}"
    text = model(name, n, strategies, {s: drawn_formula(name, strategies, values[i])
                                       for i, s in enumerate(strategies)})
    kind = f"{n} players, {k} strategies, {'small' if small else 'wide'}"
    return name, kind, text, n, strategies, lambda own, counts: values[own][counts]


def public_goods(r):
    """kinship-protocol's game: e = 10, n = 5; contributing pays r e (k + 1) / n,
    keeping e + r e k / n, with k of the others contributing."""
    formulas = {"contribute": "=r * 10 * (pg.others.contribute + 1) / 5",
                "keep": "=10 + r * 10 * pg.others.contribute / 5"}
    text = model("pg", 5, ["contribute", "keep"], formulas, f"r: {tables.number(r)}")

    def pay(own, counts):
        k = counts[0]
        return r * 10 * (k + 1) / 5 if own == 0 else 10 + r * 10 * k / 5
    return "pg", f"public goods, r = {r}", text, 5, ["contribute", "keep"], pay


def pure_by_check(n, k, table):
    """Every cell where no player gains by changing alone."""
    found = set()
    for profile, pays in table.items():
        ok = True
        for player in range(n):
            for s in range(k):
                other = list(profile)
                other[player] = s
                if table[tuple(other)][player] > pays[player]:
                    ok = False
        if ok:
            found.add(profile)
    return found


def gambit_pure(n, k, table):
    import numpy
    import pygambit as gbt
    arrays = []
    for player in range(n):
        a = numpy.empty((k,) * n, dtype=object)
        for profile, pays in table.items():
            a[profile] = pays[player]
        arrays.append(a)
    game = gbt.Game.from_arrays(*arrays)
    found = set()
    for eq in gbt.nash.enumpure_solve(game).equilibria:
        found.add(tuple(next(i for i, s in enumerate(p.strategies) if F(str(eq[s])) == 1)
                        for p in game.players))
    return found


def check(report, n, strategies, table):
    """What is wrong with Forge's report of this game, and how many equilibria."""
    bad = []
    k = len(strategies)
    players = report["players"]
    index = {s: i for i, s in enumerate(strategies)}
    if len(report["cells"]) != len(table):
        bad.append(f"{len(report['cells'])} cells, not {len(table)}")
    for cell in report["cells"]:
        profile = tuple(index[s] for s in cell["strategies"])
        want = [float(v) for v in table[profile]]
        if any(abs(a - b) > EPS * max(1, abs(b)) for a, b in zip(cell["payoffs"], want)):
            bad.append(f"cell {cell['strategies']}: {cell['payoffs']}, expanded here {want}")
    forge = set()
    for eq in report["equilibria"]:
        if eq["kind"] != "pure":
            plays = [eq["players"][pl]["strategies"][strategies[0]] for pl in players]
            if k != 2 or max(plays) - min(plays) > 1e-9:
                bad.append(f"a {eq['kind']} equilibrium in a game of {n} players: {plays}")
            continue
        profile = tuple(next(index[s] for s, p in eq["players"][pl]["strategies"].items() if p == 1)
                        for pl in players)
        forge.add(profile)
        for pl, pay in zip(players, table[profile]):
            if abs(eq["players"][pl]["payoff"] - float(pay)) > EPS * max(1, abs(float(pay))):
                bad.append(f"equilibrium {profile}: {pl} paid {eq['players'][pl]['payoff']}, the cell {float(pay)}")
    gambit, here = gambit_pure(n, k, table), pure_by_check(n, k, table)
    if forge != gambit:
        bad.append(f"pure equilibria {sorted(forge)}, Gambit's {sorted(gambit)}")
    if gambit != here:
        bad.append(f"Gambit's pure equilibria {sorted(gambit)}, a check of every cell {sorted(here)}")
    if report["mixed_searched"]:
        bad.append("mixed equilibria reported as looked for")
    if report["symmetric_searched"] != (k == 2):
        bad.append(f"symmetric mixes looked for: {report['symmetric_searched']}, with {k} strategies")
    return bad, len(forge)


def cases(seed, count):
    rng = random.Random(seed)
    out = [public_goods(r) for r in (F(8, 5), F(5), F(6))]
    return out + [draw(rng, i) for i in range(count)]
