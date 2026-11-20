"""Tournaments: Forge's exact match values against the axelrod library.

Forge works a match out exactly (a Markov chain over the strategies' states).
axelrod plays it. So each pair of strategies plays `runs` seeded matches, one
at a time with axl.Match, and the average is the expected total over the
expected length: the payoffs summed over the runs, over the lengths summed
over the runs. (The mean of each run's payoff per round is a different number
when a match ends at random, and is not what a tournament reports.) The
tolerance is `SIGMAS` standard errors of that ratio, by the delta method:
the standard error of sum(T) / sum(L) is sqrt(sum((T - r L)^2)) / sum(L),
with r the ratio and n / (n - 1) on the variance. A cooperation share is an
average over the opponents, and the runs of every pair share their seeds, so
the average's error adds each run's influence across the opponents before it
squares them.

With 45 pairs of the nine strategies, two sides each, in each setting,
several hundred values are compared, so a tolerance of 4.5 standard errors
keeps a false failure at about one run in a few hundred even if the seeds
change. The seeds do not change: the same seed, runs and library give the
same numbers.
"""
import math

import tables

SIGMAS = 4.5
FLOOR = 1e-6  # for matches that are not random at all: standard error 0

# What Forge calls each strategy, and the axelrod class that plays the same
# (GTFT forgives with 1/3, Random cooperates with 1/2: Forge's defaults)
STRATEGIES = ["always_cooperate", "always_defect", "tit_for_tat", "suspicious_tit_for_tat",
              "tit_for_two_tats", "grim_trigger", "win_stay_lose_shift", "generous_tit_for_tat", "random"]

# (rounds, continuation, noise): the settings every pair is checked in. The
# examples' own settings are added as they come.
SETTINGS = [(200, None, 0.05), (None, 0.9, 0.02), (200, None, 0.0)]


def player(name):
    import axelrod as axl
    return {"always_cooperate": axl.Cooperator, "always_defect": axl.Defector,
            "tit_for_tat": axl.TitForTat, "suspicious_tit_for_tat": axl.SuspiciousTitForTat,
            "tit_for_two_tats": axl.TitFor2Tats, "grim_trigger": axl.Grudger,
            "win_stay_lose_shift": axl.WinStayLoseShift,
            "generous_tit_for_tat": lambda: axl.GTFT(p=1 / 3),
            "random": lambda: axl.Random(0.5)}[name]()


def influence(totals, lengths):
    """sum(T) / sum(L), and what each run adds to it: (T - r L) / sum(L).
    The standard error is the square root of the squared influences, times
    n / (n - 1)."""
    total = sum(lengths)
    r = sum(totals) / total
    return r, [(t - r * length) / total for t, length in zip(totals, lengths)]


def error(influences):
    n = len(influences)
    return math.sqrt(sum(i * i for i in influences) * n / (n - 1))


_cache = {}


def pair(setting, a, b, runs):
    """axelrod's average payoff per round of `a` and of `b` when they play,
    and how often each cooperates, each as (value, influence of each run)."""
    key = (setting, a, b, runs)
    if key in _cache:
        return _cache[key]
    import axelrod as axl
    rounds, continuation, noise = setting
    pays = ([], [])
    coops = ([], [])
    lengths = []
    for seed in range(runs):
        players = (player(a), player(b))
        if continuation is None:
            match = axl.Match(players, turns=rounds, noise=noise, seed=seed)
        else:
            match = axl.Match(players, prob_end=1 - continuation, noise=noise, seed=seed)
        match.play()
        scores = match.scores()
        lengths.append(len(scores))
        for side in (0, 1):
            pays[side].append(sum(s[side] for s in scores))
            coops[side].append(sum(1 for turn in match.result if str(turn[side]) == "C"))
    out = (tuple(influence(pays[s], lengths) for s in (0, 1)), tuple(influence(coops[s], lengths) for s in (0, 1)))
    _cache[key] = out
    return out


def value(setting, a, b, runs, kind=0):
    """a's average against b (kind 0: payoff, 1: cooperation), and what each
    run adds to it. Against its own kind the two sides are the same, so they
    are pooled."""
    sides = pair(setting, *sorted((a, b)), runs)[kind]
    if a == b:
        return (sides[0][0] + sides[1][0]) / 2, [(x + y) / 2 for x, y in zip(sides[0][1], sides[1][1])]
    return sides[0] if a <= b else sides[1]


def model(rounds, continuation, noise, kinds):
    """A model with the dilemma and one tournament of the given strategies,
    each named by its kind."""
    length = f"rounds: {rounds}" if continuation is None else f"continuation: {continuation}"
    plays = []
    for k in kinds:
        body = "{plays: random, cooperate: 0.5}" if k == "random" else k
        plays.append(f"        {k}: {body}")
    return "\n".join(['_forge_version: "5.5.0"', "games:", "  dilemma:", "    players: [me, you]",
                      "    strategies: [cooperate, defect]", "    symmetric: true", "    payoffs:",
                      "      cooperate: {cooperate: 3, defect: 0}", "      defect: {cooperate: 5, defect: 1}",
                      "  field:", "    tournament:", "      round: dilemma", "      players:", *plays,
                      f"      {length}", f"      noise: {noise}", "      self_play: true"]) + "\n"


def compare(report, runs, label):
    """The failures and the number of values compared for one tournament report."""
    setting = (report["rounds"], report["continuation"], report["noise"])
    plays = {s["player"]: s["plays"] for s in report["ranking"]}
    names = list(plays)
    failures, compared = [], 0
    for a in names:
        for b in names:
            if a == b and not report["self_play"]:
                continue
            want, parts = value(setting, plays[a], plays[b], runs)
            se = error(parts)
            got = report["matches"][a][b]
            compared += 1
            if abs(got - want) > SIGMAS * se + FLOOR:
                failures.append(f"{label}: {a} ({plays[a]}) against {b} ({plays[b]}) pays {got} a round, "
                                f"axelrod {want:.5f} (standard error {se:.5f}, {runs} runs)")
    # The share of moves that cooperate: the average over the opponents
    for a in names:
        opponents = [b for b in names if b != a or report["self_play"]]
        got = next(s["cooperation"] for s in report["ranking"] if s["player"] == a)
        # The runs share their seeds from one pair to the next, so the errors
        # of the opponents are added run by run, not as independent ones
        parts = [value(setting, plays[a], plays[b], runs, 1) for b in opponents]
        want = sum(p[0] for p in parts) / len(parts)
        se = error([sum(p[1][i] for p in parts) / len(parts) for i in range(runs)])
        compared += 1
        if abs(got - want) > SIGMAS * se + FLOOR:
            failures.append(f"{label}: {a} ({plays[a]}) cooperates {got}, axelrod {want:.5f} "
                            f"(standard error {se:.5f})")
    failures += consistency(report, label)
    return failures, compared


def consistency(report, label):
    """Forge's score, rank and stable list from its own match table, worked out again."""
    names = [s["player"] for s in report["ranking"]]
    matches, bad = report["matches"], []
    for s in report["ranking"]:
        opponents = [b for b in names if b != s["player"] or report["self_play"]]
        mean = sum(matches[s["player"]][b] for b in opponents) / len(opponents)
        if abs(mean - s["score"]) > 1e-9:
            bad.append(f"{label}: {s['player']} scores {s['score']}, its matches average {mean}")
        better = sum(1 for o in report["ranking"] if o["score"] > s["score"] + 1e-9)
        if s["rank"] != better + 1:
            bad.append(f"{label}: {s['player']} is ranked {s['rank']}, {better} score more")
    for s in names:
        stable = all(matches[t][s] <= matches[s][s] + 1e-9 for t in names if t != s)
        if stable != (s in report["stable"]):
            bad.append(f"{label}: {s} stable is {s in report['stable']}, its match table says {stable}")
    return bad
