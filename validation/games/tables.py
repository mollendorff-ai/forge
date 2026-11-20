"""A game as a payoff table: Forge's report against Gambit.

A case is two payoff matrices, `a` for the row player and `b` for the column
player (lists of lists of Fractions). It is written as a Forge model, run
through `forge game -o`, and solved by Gambit (pygambit's pure and mixed
solvers, which are not Forge's support enumeration). The rules, each a
failure when broken:

  1. Every equilibrium Forge lists is one: no player gains by a change, and
     the payoffs it reports are the payoffs of the profile.
  2. The pure equilibria are the same as Gambit's, whatever the ties: Forge's
     list of them is complete.
  3. Every mixed equilibrium Gambit finds, Forge finds, unless Forge reports
     the game as degenerate (`ties` is not empty). A degenerate game can have
     a line of equilibria, and the report says its list may be incomplete.
  4. Every equilibrium Forge lists, Gambit lists, unless the game is reported
     as degenerate (Gambit lists the corners of a line, Forge may list a point
     inside it).
  5. A game not reported as degenerate has an odd number of equilibria (a
     known property of nondegenerate two-player games).
  6. The strategies removed as strictly dominated, and those reported as
     weakly dominated, are the ones an independent check finds.
  7. The payoff against an even mix is the mean over the opponent's strategies.
"""
import json
import os
import subprocess
import tempfile
from fractions import Fraction
from fractions import Fraction as F

FORGE = os.environ.get("FORGE", "forge")
EPS = 1e-7


def number(x):
    """A payoff as the text of a YAML number: an integer or a decimal."""
    x = Fraction(x)
    if x.denominator == 1:
        return str(x.numerator)
    return repr(float(x))


def model(name, a, b, rows=None, cols=None):
    """The Forge model of a two-player game, in format 5.5.0."""
    rows = rows or [f"r{i + 1}" for i in range(len(a))]
    cols = cols or [f"c{j + 1}" for j in range(len(a[0]))]
    out = ['_forge_version: "5.5.0"', "games:", f"  {name}:", "    players: [row, col]",
           "    strategies:", f"      row: [{', '.join(rows)}]", f"      col: [{', '.join(cols)}]",
           "    payoffs:"]
    for i, r in enumerate(rows):
        cells = ", ".join(f"{c}: [{number(a[i][j])}, {number(b[i][j])}]" for j, c in enumerate(cols))
        out.append(f"      {r}: {{{cells}}}")
    return "\n".join(out) + "\n"


def run_forge(path, game=None, scenario=None):
    """The reports `forge game -o` writes for a file, one for each game."""
    handle, out = tempfile.mkstemp(suffix=".json")
    os.close(handle)
    cmd = [FORGE, "game", path, "-o", out]
    if game:
        cmd += ["-g", game]
    if scenario:
        cmd += ["-s", scenario]
    done = subprocess.run(cmd, capture_output=True, text=True)
    if done.returncode:
        raise SystemExit(f"forge game failed on {path}:\n{done.stdout[-1500:]}\n{done.stderr[-1500:]}")
    with open(out) as f:
        reports = json.load(f)
    os.remove(out)
    return reports


def gambit_solutions(a, b):
    """Gambit's pure equilibria and its mixed ones, as (row mix, column mix)
    pairs of Fraction lists. The mixed list holds every extreme equilibrium
    and includes the pure ones; a pure equilibrium is a mix with one 1."""
    import numpy
    import pygambit as gbt
    game = gbt.Game.from_arrays(numpy.array(a, dtype=object), numpy.array(b, dtype=object))
    row, col = list(game.players)

    def profile(eq):
        return ([F(str(eq[s])) for s in row.strategies], [F(str(eq[s])) for s in col.strategies])

    pure = [profile(eq) for eq in gbt.nash.enumpure_solve(game).equilibria]
    mixed = [profile(eq) for eq in gbt.nash.enummixed_solve(game, rational=True).equilibria]
    return pure, mixed


def forge_profile(report, eq):
    """An equilibrium of the report as (row mix, column mix, payoffs)."""
    first, second = report["players"]
    rows, cols = report["strategies"][first], report["strategies"][second]
    return ([eq["players"][first]["strategies"][s] for s in rows],
            [eq["players"][second]["strategies"][s] for s in cols],
            [eq["players"][first]["payoff"], eq["players"][second]["payoff"]])


def close(p, q):
    return all(abs(float(x) - float(y)) < EPS for u, v in zip(p, q) for x, y in zip(u, v))


def certified_degenerate(a, b, found):
    """Whether the game is degenerate, with a proof: a pure strategy with two
    best replies, or an equilibrium (one of those in `found`, as Fractions)
    where a player has more best replies than the other has strategies in
    play. Exact. (A game can be degenerate without either; that is not
    shown here, so a game that is not certified is not shown to be nondegenerate.)"""
    rows, cols = len(a), len(a[0])
    for j in range(cols):
        column = [a[i][j] for i in range(rows)]
        if column.count(max(column)) > 1:
            return True
    for i in range(rows):
        if b[i].count(max(b[i])) > 1:
            return True
    for x, y in found:
        pays = [sum(a[i][j] * y[j] for j in range(cols)) for i in range(rows)]
        if pays.count(max(pays)) > sum(1 for v in x if v):
            return True
        pays = [sum(b[i][j] * x[i] for i in range(rows)) for j in range(cols)]
        if pays.count(max(pays)) > sum(1 for v in y if v):
            return True
    return False


def payoffs_of(a, b, x, y):
    """The expected payoff of each player at the mixes x and y, in floats."""
    fa = [[float(v) for v in r] for r in a]
    fb = [[float(v) for v in r] for r in b]
    row = sum(x[i] * y[j] * fa[i][j] for i in range(len(x)) for j in range(len(y)))
    col = sum(x[i] * y[j] * fb[i][j] for i in range(len(x)) for j in range(len(y)))
    return row, col


def verify(a, b, x, y, reported):
    """Why (x, y) is not an equilibrium paying `reported`, or None."""
    if abs(sum(x) - 1) > EPS or abs(sum(y) - 1) > EPS or min(x + y) < -EPS:
        return "the mixes are not probabilities"
    fa = [[float(v) for v in r] for r in a]
    fb = [[float(v) for v in r] for r in b]
    row, col = payoffs_of(a, b, x, y)
    if max(sum(fa[i][j] * y[j] for j in range(len(y))) for i in range(len(x))) > row + EPS:
        return "the row player gains by changing strategy"
    if max(sum(fb[i][j] * x[i] for i in range(len(x))) for j in range(len(y))) > col + EPS:
        return "the column player gains by changing strategy"
    if abs(reported[0] - row) > EPS or abs(reported[1] - col) > EPS:
        return f"payoffs reported {reported}, worked out {(row, col)}"
    return None


def strictly_dominates(table, by, strategy, others):
    """Whether row `by` pays more than `strategy` against every one of `others`.
    `table(i, j)` is the payoff of the player's strategy i against j."""
    return all(table(by, o) > table(strategy, o) for o in others)


def dominance_failures(a, b, report):
    """Rule 6, for two players: the steps are valid in order, nothing
    strictly dominated is left, and the weak list is the independent one."""
    first, second = report["players"]
    names = {first: report["strategies"][first], second: report["strategies"][second]}
    alive = {p: list(range(len(n))) for p, n in names.items()}
    pay = {first: lambda i, j: a[i][j], second: lambda j, i: b[i][j]}
    other = {first: second, second: first}
    bad = []
    for step in report["dominated"]:
        p = step["player"]
        s, by = names[p].index(step["strategy"]), names[p].index(step["by"])
        if s not in alive[p] or by not in alive[p] or not strictly_dominates(pay[p], by, s, alive[other[p]]):
            bad.append(f"step {step} is not a valid removal")
        else:
            alive[p].remove(s)
    for p in (first, second):
        for s in alive[p]:
            for by in alive[p]:
                if by != s and strictly_dominates(pay[p], by, s, alive[other[p]]):
                    bad.append(f"{p}'s {names[p][s]} is still dominated by {names[p][by]}")
    weak = set()
    for p in (first, second):
        every = range(len(names[p]))
        for s in every:
            for by in every:
                rest = range(len(names[other[p]]))
                if by != s and all(pay[p](by, o) >= pay[p](s, o) for o in rest) \
                        and any(pay[p](by, o) > pay[p](s, o) for o in rest):
                    weak.add((p, names[p][s]))
                    break
    reported = {(w["player"], w["strategy"]) for w in report["weakly_dominated"]}
    if weak != reported:
        bad.append(f"weakly dominated: Forge {sorted(reported)}, independent {sorted(weak)}")
    return bad


def even_mix_failures(a, b, report):
    first, second = report["players"]
    rows, cols = report["strategies"][first], report["strategies"][second]
    bad = []
    for i, r in enumerate(rows):
        want = float(sum(a[i]) / len(cols))
        if abs(report["even_mix"][first][r] - want) > EPS:
            bad.append(f"even mix of {first}'s {r}: {report['even_mix'][first][r]} against {want}")
    for j, c in enumerate(cols):
        want = float(sum(b[i][j] for i in range(len(rows))) / len(rows))
        if abs(report["even_mix"][second][c] - want) > EPS:
            bad.append(f"even mix of {second}'s {c}: {report['even_mix'][second][c]} against {want}")
    return bad


def check(a, b, report):
    """Every rule against one Forge report. Returns (failures, facts), the
    facts being counters for the summary."""
    failures = []
    degenerate = bool(report["ties"])
    forge = [(e["kind"], forge_profile(report, e)) for e in report["equilibria"]]
    for kind, (x, y, pays) in forge:
        why = verify(a, b, x, y, pays)
        if why:
            failures.append(f"Forge's {kind} equilibrium {x} {y} is not one: {why}")
    pure_gambit, mixed_gambit = gambit_solutions(a, b)
    pure_forge = [p[:2] for k, p in forge if k == "pure"]
    if len(pure_forge) != len(pure_gambit) or not all(any(close(p, q) for q in pure_gambit) for p in pure_forge):
        failures.append(f"pure equilibria: Forge {pure_forge}, Gambit {pure_gambit}")
    missed = [g for g in mixed_gambit if not any(close(g, p[:2]) for _, p in forge)]
    extra = [p[:2] for _, p in forge if not any(close(p[:2], g) for g in mixed_gambit)]
    if (missed or extra) and not degenerate:
        failures.append(f"not reported as degenerate, but Forge misses {missed} and Gambit misses {extra}")
    if not degenerate and len(forge) % 2 == 0:
        failures.append(f"a game not reported as degenerate has {len(forge)} equilibria, an even number")
    failures += dominance_failures(a, b, report)
    failures += even_mix_failures(a, b, report)
    proved = certified_degenerate(a, b, mixed_gambit)
    if degenerate and not proved:
        failures.append("reported as degenerate (" + report["ties"][0] + "), which the check cannot show: "
                        "no pure strategy has two best replies and no equilibrium has a spare best reply")
    if proved and not degenerate:
        failures.append("degenerate (a tie at a pure strategy or an equilibrium), and not reported as such")
    facts = {"equilibria": len(forge), "degenerate": degenerate,
             "missed": len(missed) if degenerate else 0, "extra": len(extra) if degenerate else 0}
    return failures, facts
