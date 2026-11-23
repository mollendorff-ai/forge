"""Forge's games against Gambit, axelrod and R (ADR-068, T2-GAME-THEORY-E2E).

  python3 -I run.py [--only specs tournaments generated r counted symmetric dynamics] [--seed N]
                    [--games N] [--runs N] [--work DIR] [--fail]

specs        named games (specs.py) and the shipped examples, against Gambit
tournaments  every pair of the nine strategies in three settings, and the
             tournaments of the examples, against axelrod
generated    seeded random games, 2 x 2 to 6 x 6, ties included, against
             Gambit; a game Gambit and Forge disagree on fails unless Forge
             reports it as degenerate
r            the closed forms of 2 x 2 games, against R; and the games of many
             players worked out again in R (many_players.R): every generation
             of the dynamics, the symmetric mixed equilibria as polynomial roots
             with their stability, the volunteer's dilemma and public goods
             closed forms
counted      symmetric games of three to five players in the count form
             (ADR-073): the public goods game and seeded games, the table
             expanded cell by cell and the pure equilibria, against Gambit
symmetric    the mixed equilibria in which every player plays alike, of
             symmetric games of two strategies (ADR-073): the volunteer's
             dilemma of 2 to 8 players against its closed form, and seeded
             games of 3 to 5 players against Gambit's enumpoly_solve
dynamics     the dynamics: key (ADR-073): every generation of the discrete
             replicator equation against egttools' gradient, in hawk-dove,
             the stag hunt, the volunteer's dilemma, rock, paper, scissors,
             the examples and seeded games of 2 to 5 players

Needs `forge` (FORGE, default `forge` on the path), pygambit 16.7.0, the
axelrod library and egttools for the Python parts, and Rscript with jsonlite for `r`.
"""
import argparse
import json
import os
import random
import shutil
import subprocess
import sys
from fractions import Fraction as F

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)  # python3 -I leaves the script's folder off the path

import counted
import dynamics
import generated
import symmetric
import specs
import tables
import tournaments

BATCH = 100


def write(work, name, text):
    path = os.path.join(work, name)
    with open(path, "w") as f:
        f.write(text)
    return path


def only(reports, game):
    return next(r for r in reports if r["game"] == game)


def run_specs(work):
    failures, equilibria, degenerate = [], 0, 0
    runs = 0
    for k, spec in enumerate(specs.SPECS):
        path = write(work, f"spec{k}.yaml", tables.model("g", spec.a, spec.b, spec.rows, spec.cols))
        reports = [(spec.name, only(tables.run_forge(path), "g"))]
        if spec.example:
            file, game, scenario = spec.example
            shown = os.path.relpath(os.path.join(HERE, file), os.path.join(HERE, "..", ".."))
            reports.append((f"{spec.name} ({shown})",
                            only(tables.run_forge(os.path.join(HERE, file), game, scenario), game)))
        for label, report in reports:
            bad, facts = tables.check(spec.a, spec.b, report)
            runs += 1
            equilibria += facts["equilibria"]
            degenerate += facts["degenerate"]
            failures += [f"{label}: {b}" for b in bad]
    print(f"specs: {len(specs.SPECS)} games, {runs} runs of Forge ({runs - len(specs.SPECS)} through the examples), "
          f"{equilibria} equilibria, {degenerate} degenerate, {len(failures)} failed")
    return failures


def run_generated(work, seed, count):
    failures = []
    totals = {"equilibria": 0, "degenerate": 0, "missed": 0, "extra": 0}
    kinds = {}
    drawn = generated.cases(seed, count)
    for start in range(0, len(drawn), BATCH):
        batch = drawn[start:start + BATCH]
        path = write(work, f"gen{seed}_{start}.yaml", generated.model_of(batch))
        reports = {r["game"]: r for r in tables.run_forge(path)}
        for name, kind, a, b in batch:
            bad, facts = tables.check(a, b, reports[name])
            for key in totals:
                totals[key] += facts[key]
            kinds[kind] = kinds.get(kind, 0) + 1
            if bad:
                failures.append(f"{name} ({kind}, {len(a)} x {len(a[0])}): " + "; ".join(bad)
                                + "\n" + generated.show(a, b))
    print(f"generated: seed {seed}, {count} games ({', '.join(f'{n} {k}' for k, n in sorted(kinds.items()))}), "
          f"{totals['equilibria']} equilibria, {totals['degenerate']} games reported as degenerate "
          f"(where Gambit finds {totals['missed']} that Forge does not, and Forge {totals['extra']} that Gambit does not, "
          f"all in games reported as degenerate), {len(failures)} failed")
    if failures:
        print(f"replay: python3 -I run.py --only generated --seed {seed} --games {count}")
    return failures


def run_counted(work, seed, count):
    failures, equilibria, kinds = [], 0, {}
    for name, kind, text, n, strategies, pay in counted.cases(seed, count):
        path = write(work, f"counted_{name}_{len(kinds)}.yaml", text)
        report = only(tables.run_forge(path), name)
        table = counted.expand(n, strategies, pay)
        bad, found = counted.check(report, n, strategies, table)
        equilibria += found
        group = kind if kind.startswith("public goods") else kind.split(",")[0]
        kinds[group] = kinds.get(group, 0) + 1
        failures += [f"{name} ({kind}): {b}" for b in bad]
    # The bagel's festival of five shops, from its file, with its scenario
    for scenario, match in ((None, 1), ("borough_matches", 2)):
        report = only(tables.run_forge(os.path.join(HERE, specs.EXAMPLES, "../bagel/street_fund.yaml"), "street",
                                       scenario), "street")
        strategies = ["pay", "keep"]
        bad, found = counted.check(report, 5, strategies, counted.expand(5, strategies, counted.street_fund(match)))
        equilibria += found
        kinds["bagel street fund"] = kinds.get("bagel street fund", 0) + 1
        failures += [f"examples/bagel/street_fund.yaml ({scenario or 'no scenario'}): {b}" for b in bad]
    print(f"counted: seed {seed}, {sum(kinds.values())} games "
          f"({', '.join(f'{n} {k}' for k, n in sorted(kinds.items()))}), "
          f"{equilibria} pure equilibria, every cell compared, {len(failures)} failed")
    return failures


def run_symmetric(work, seed, count):
    failures, mixes, missed, games = [], 0, 0, 0
    for name, kind, text, n, strategies, pay, closed in symmetric.cases(seed, count):
        path = write(work, f"symmetric_{name}_{games}.yaml", text)
        report = only(tables.run_forge(path), name)
        bad, found, gambit_missed = symmetric.check(report, n, strategies, pay, closed)
        mixes += found
        missed += gambit_missed
        games += 1
        failures += [f"{name} ({kind}): {b}" for b in bad]
    print(f"symmetric: seed {seed}, {games} games (the volunteer's dilemma of 2 to 8 players, "
          f"the public goods game at r = 5, {count} drawn), {mixes} mixes in which all play alike, "
          f"compared with the exact roots; Gambit's enumpoly_solve found {mixes - missed} of them; "
          f"{len(failures)} failed")
    if failures:
        print(f"replay: python3 -I run.py --only symmetric --seed {seed} --games {count * 10}")
    return failures


def run_dynamics(work, seed, count):
    failures, steps, games = [], 0, 0
    for name, kind, text, n, strategies, pay, known in dynamics.cases(seed, count):
        path = write(work, f"dynamics_{name}_{games}.yaml", text)
        bad, compared = dynamics.check(only(tables.run_forge(path), name), n, strategies, pay, known)
        steps += compared
        games += 1
        failures += [f"{name} ({kind}): {b}" for b in bad]
    for file, game, scenario, n, strategies, pay, known in dynamics.examples():
        report = only(tables.run_forge(os.path.join(HERE, specs.EXAMPLES, file), game, scenario), game)
        bad, compared = dynamics.check(report, n, strategies, pay, known)
        steps += compared
        games += 1
        failures += [f"examples/games/{file} ({scenario or 'no scenario'}): {b}" for b in bad]
    failures += dynamics.refused(work)
    print(f"dynamics: seed {seed}, {games} games (7 named, {count} drawn, 8 runs of the examples, the bagel's price war among them), {steps} generations "
          f"compared with egttools' replicator gradient, one refusal, {len(failures)} failed")
    if failures:
        print(f"replay: python3 -I run.py --only dynamics --seed {seed} --games {count * 10}")
    return failures


def run_tournaments(work, runs):
    failures, compared = [], 0
    for k, (rounds, continuation, noise) in enumerate(tournaments.SETTINGS):
        path = write(work, f"field{k}.yaml", tournaments.model(rounds, continuation, noise, tournaments.STRATEGIES))
        report = only(tables.run_forge(path), "field")
        label = f"all nine, {'rounds ' + str(rounds) if continuation is None else 'continuation ' + str(continuation)}, noise {noise}"
        bad, n = tournaments.compare(report, runs, label)
        failures += bad
        compared += n
    for file, games in (("axelrod.yaml", ["axelrod"]), ("noisy-tournament.yaml", ["clean", "noisy"]),
                        ("continuation.yaml", ["long_run"])):
        reports = tables.run_forge(os.path.join(HERE, specs.EXAMPLES, file))
        for game in games:
            bad, n = tournaments.compare(only(reports, game), runs, f"examples/games/{file}, {game}")
            failures += bad
            compared += n
    print(f"tournaments: {compared} values compared with axelrod ({runs} seeded runs of each match, "
          f"{tournaments.SIGMAS} standard errors), {len(failures)} failed")
    return failures


def run_closed_forms(work, seed, count):
    rng = random.Random(seed)
    batch = []
    for k in range(count):
        a = [[F(rng.randint(-40, 40), 4) for _ in range(2)] for _ in range(2)]
        b = [[F(rng.randint(-40, 40), 4) for _ in range(2)] for _ in range(2)]
        batch.append((f"g{k}", "2 x 2", a, b))
    path = write(work, f"closed{seed}.yaml", generated.model_of(batch))
    reports = {r["game"]: r for r in tables.run_forge(path)}
    games = [{"name": name, "a": [[float(v) for v in row] for row in a],
              "b": [[float(v) for v in row] for row in b], "report": reports[name]}
             for name, _, a, b in batch]
    data = write(work, f"closed{seed}.json", json.dumps(games))
    done = subprocess.run(["Rscript", os.path.join(HERE, "closed_forms.R"), data], capture_output=True, text=True)
    print(done.stdout.strip() or done.stderr.strip())
    return [] if done.returncode == 0 else [f"R: {done.stdout[-800:]}{done.stderr[-800:]}"]


def run_many_players(work, seed, count):
    """The games of many players and the dynamics, worked out again in R
    (many_players.R): the symmetric games, the dynamics' cases and the
    examples, from the same seed as symmetric.py and dynamics.py."""
    def table(n, strategies, pay):
        return [[{"split": list(c), "value": float(pay(s, c))} for c in counted.splits(n - 1, len(strategies))]
                for s in range(len(strategies))]
    out = []
    for name, kind, text, n, strategies, pay, closed in symmetric.cases(seed, count):
        report = only(tables.run_forge(write(work, f"many_{name}_{len(out)}.yaml", text)), name)
        case = {"name": name, "kind": kind, "n": n, "strategies": strategies, "pay": table(n, strategies, pay),
                "report": report, "roots": True}
        if name == "vd":
            case["volunteer"] = {"b": 10, "c": 4}
        out.append(case)
    for r in (F(8, 5), F(5), F(6)):
        name, kind, text, n, strategies, pay = counted.public_goods(r)
        report = only(tables.run_forge(write(work, f"many_pg_{len(out)}.yaml", text)), name)
        out.append({"name": name, "kind": kind, "n": n, "strategies": strategies, "pay": table(n, strategies, pay),
                    "report": report, "roots": True, "public_goods": {"r": float(r)}})
    for name, kind, text, n, strategies, pay, known in dynamics.cases(seed, count):
        report = only(tables.run_forge(write(work, f"many_{name}_{len(out)}.yaml", text)), name)
        out.append({"name": name, "kind": kind, "n": n, "strategies": strategies, "pay": table(n, strategies, pay),
                    "report": report, "roots": len(strategies) == 2,
                    "known": None if known is None else [float(v) for v in known]})
    for file, game, scenario, n, strategies, pay, known in dynamics.examples():
        report = only(tables.run_forge(os.path.join(HERE, specs.EXAMPLES, file), game, scenario), game)
        out.append({"name": f"examples/games/{file}", "kind": scenario or "no scenario", "n": n,
                    "strategies": strategies, "pay": table(n, strategies, pay), "report": report, "roots": True,
                    "known": None if known is None else [float(v) for v in known]})
    data = write(work, f"many{seed}.json", json.dumps(out))
    done = subprocess.run(["Rscript", os.path.join(HERE, "many_players.R"), data], capture_output=True, text=True)
    print(done.stdout.strip() or done.stderr.strip())
    if done.returncode == 0:
        return []
    print(f"replay: python3 -I run.py --only r --seed {seed} --games {count * 10}")
    return [f"R: {done.stdout[-800:]}{done.stderr[-800:]}"]


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--only", nargs="*", choices=["specs", "tournaments", "generated", "r", "counted", "symmetric",
                                                 "dynamics"])
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--games", type=int, default=300, help="generated games")
    p.add_argument("--runs", type=int, default=200, help="seeded axelrod matches of each pair")
    p.add_argument("--work", default="work")
    p.add_argument("--fail", action="store_true", help="exit 1 when anything fails (CI)")
    a = p.parse_args()
    parts = a.only or ["specs", "tournaments", "generated", "r", "counted", "symmetric", "dynamics"]
    os.makedirs(a.work, exist_ok=True)
    failures = []
    if "specs" in parts:
        failures += run_specs(a.work)
    if "generated" in parts:
        failures += run_generated(a.work, a.seed, a.games)
    if "counted" in parts:
        failures += run_counted(a.work, a.seed, max(a.games // 10, 10))
    if "symmetric" in parts:
        # Gambit's enumpoly_solve takes seconds a game of five players: at most 100
        failures += run_symmetric(a.work, a.seed, min(max(a.games // 10, 10), 100))
    if "dynamics" in parts:
        failures += run_dynamics(a.work, a.seed, max(a.games // 10, 10))
    if "tournaments" in parts:
        failures += run_tournaments(a.work, a.runs)
    if "r" in parts:
        failures += run_closed_forms(a.work, a.seed, a.games)
        failures += run_many_players(a.work, a.seed, max(a.games // 10, 10))
    for f in failures:
        print("FAIL", f)
    if failures and a.fail:
        sys.exit(1)
    shutil.rmtree(a.work, ignore_errors=True)


if __name__ == "__main__":
    main()
