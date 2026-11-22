"""The `dynamics:` key of a symmetric game, against egttools (ADR-073, question 3).

Forge moves each strategy's share by the discrete replicator equation,
x' = x (f + b) / (f_avg + b), where f is what the strategy pays against the
population, over every way the other players' strategies can fall, and b is
the baseline. egttools (0.2.1, a C++ core) gives the replicator gradient
x (f - f_avg) for a game of n players from its own payoff table. Since
x' = x + x (f - f_avg) / (f_avg + b), the rules, each a failure when broken,
are:

  1. In every generation, Forge's next shares are its shares plus egttools'
     gradient at them, over f_avg + b, within 1e-9.
  2. Forge's average payoff in every generation is the average worked out
     here, exactly (fractions) from the payoffs, within 1e-9 of its size.
  3. The shares add up to 1, and none is below 0, in every generation.
  4. Where the end is known by hand, the path reaches it: hawk-dove with
     V = 4 and C = 6 at a hawk share of V / C = 2/3, the stag hunt at all
     stag from above 3/4 and all hare from below, the volunteer's dilemma of
     five players at 1 - 0.4^(1/4), rock, paper, scissors of three players
     at its start when the start is the even mix.
  5. A baseline too low for a payoff is refused, with the strategy named.

The cases are those games, the shipped examples with their scenarios, and
games drawn from a seed: two to five players, two or three strategies, in
the count form, a start drawn on the simplex, and a baseline that keeps every
payoff above 0.
"""
import math
import os
import random
import subprocess
from fractions import Fraction as F

import counted
import tables

EPS = 1e-9


def payoff_matrix(n, k, pay):
    """egttools' table: a row per strategy, a column per group of n players
    in egttools' order (sample_simplex), the focal player counted in the
    group; a strategy absent from a group has no payoff there."""
    import egttools
    import numpy
    columns = egttools.calculate_nb_states(n, k)
    matrix = numpy.zeros((k, columns))
    for c in range(columns):
        group = [int(v) for v in egttools.sample_simplex(c, n, k)]
        for s in range(k):
            if group[s]:
                others = list(group)
                others[s] -= 1
                matrix[s, c] = float(pay(s, tuple(others)))
    return matrix


def fitness(n, k, pay, shares):
    """What each strategy pays against the population, exactly: the payoff
    over every split of the other n - 1, weighted by its chance."""
    out = []
    for s in range(k):
        total = F(0)
        for split in counted.splits(n - 1, k):
            orders = math.factorial(n - 1)
            for c in split:
                orders //= math.factorial(c)
            chance = F(orders)
            for x, c in zip(shares, split):
                chance *= x ** c
            total += chance * F(pay(s, split))
        out.append(total)
    return out


def check(report, n, strategies, pay, known=None):
    """What is wrong with Forge's path, and how many generations were compared."""
    import numpy
    from egttools.analytical import replicator_equation_n_player
    bad = []
    k = len(strategies)
    d = report.get("dynamics")
    if d is None:
        return ["no dynamics in the report"], 0
    b = d["baseline"]
    path = d["path"]
    if len(path) != d["generations"] + 1:
        bad.append(f"{len(path)} generations in the path, not {d['generations'] + 1}")
    matrix = payoff_matrix(n, k, pay)
    for t, generation in enumerate(path):
        x = [generation["shares"][s] for s in strategies]
        if abs(sum(x) - 1) > 1e-12 or min(x) < 0:
            bad.append(f"generation {t}: shares {x}")
        exact = fitness(n, k, pay, [F(v) for v in x])
        average = sum(F(v) * f for v, f in zip(x, exact))
        if abs(generation["payoff"] - float(average)) > EPS * max(1, abs(float(average))):
            bad.append(f"generation {t}: average payoff {generation['payoff']}, here {float(average)}")
        if t + 1 < len(path):
            gradient = replicator_equation_n_player(numpy.array(x), matrix, n)
            want = [v + g / (float(average) + b) for v, g in zip(x, gradient)]
            got = [path[t + 1]["shares"][s] for s in strategies]
            if any(abs(a - w) > EPS for a, w in zip(got, want)):
                bad.append(f"generation {t + 1}: shares {got}, egttools' step {want}")
    if known is not None:
        end = [path[-1]["shares"][s] for s in strategies]
        if any(abs(a - w) > 1e-9 for a, w in zip(end, known)):
            bad.append(f"the end {end}, by hand {known}")
    return bad[:5], len(path) - 1


def model(name, n, strategies, formulas, start, generations, baseline):
    text = counted.model(name, n, strategies, formulas)
    shares = ", ".join(f"{s}: {tables.number(x)}" for s, x in zip(strategies, start))
    return text + (f"    dynamics:\n      start: {{{shares}}}\n      generations: {generations}\n"
                   f"      baseline: {tables.number(baseline)}\n")


def hawk_dove(start, generations, baseline):
    """V = 4, C = 6: hawk pays -1 against hawk and 4 against dove, dove 0
    and 2. It goes to V / C = 2/3 from any start inside."""
    def pay(own, others):
        hawks, doves = others
        return F(-hawks + 4 * doves) if own == 0 else F(2 * doves)
    formulas = {"hawk": "=-hd.others.hawk + 4 * hd.others.dove", "dove": "=2 * hd.others.dove"}
    text = model("hd", 2, ["hawk", "dove"], formulas, start, generations, baseline)
    return "hd", f"hawk-dove from {start[0]}", text, 2, ["hawk", "dove"], pay, [F(2, 3), F(1, 3)]


def stag_hunt(start):
    """Stag 4 with a stag and 0 with a hare, hare 3: all stag from above 3/4."""
    def pay(own, others):
        return F(4 * others[0]) if own == 0 else F(3)
    formulas = {"stag": "=4 * sh.others.stag", "hare": "=3"}
    text = model("sh", 2, ["stag", "hare"], formulas, [start, 1 - start], 300, 0)
    end = [1, 0] if start > F(3, 4) else [0, 1]
    return "sh", f"stag hunt from {start}", text, 2, ["stag", "hare"], pay, end


def volunteers():
    """Five players, b = 10, c = 4: the mix 1 - (c / b)^(1/4)."""
    def pay(own, others):
        return F(6) if own == 0 else F(10 if others[0] > 0 else 0)
    formulas = {"volunteer": "=6", "stay_out": "=IF(vd.others.volunteer > 0, 10, 0)"}
    text = model("vd", 5, ["volunteer", "stay_out"], formulas, [F(9, 10), F(1, 10)], 200, 0)
    mix = 1 - 0.4 ** 0.25
    return "vd", "volunteer's dilemma of 5", text, 5, ["volunteer", "stay_out"], pay, [mix, 1 - mix]


def rock_paper_scissors(start, known):
    """Three players, each scored against both others: win 1, lose -1."""
    names = ["rock", "paper", "scissors"]

    def pay(own, others):
        beaten, beats = (own + 2) % 3, (own + 1) % 3
        return F(others[beaten] - others[beats])
    formulas = {s: f"=rps.others.{names[(i + 2) % 3]} - rps.others.{names[(i + 1) % 3]}"
                for i, s in enumerate(names)}
    text = model("rps", 3, names, formulas, start, 60, 3)
    return "rps", f"rock, paper, scissors from {start}", text, 3, names, pay, known


def draw(rng, index):
    n, k = rng.randint(2, 5), rng.randint(2, 3)
    strategies = ["x", "y", "z"][:k]
    values = [{c: F(rng.randint(-40, 40), 4) for c in counted.splits(n - 1, k)} for _ in strategies]
    least = min(v for table in values for v in table.values())
    baseline = 1 - least if least <= 0 else 0
    weights = [rng.randint(1, 9) for _ in strategies]
    start = [F(w, sum(weights)) for w in weights]
    name = f"d{index}"
    text = model(name, n, strategies, {s: counted.drawn_formula(name, strategies, values[i])
                                       for i, s in enumerate(strategies)}, start, 40, baseline)
    return name, f"{n} players, {k} strategies", text, n, strategies, lambda own, c: values[own][c], None


def examples():
    """The shipped examples, with what each scenario reaches by hand."""
    def hd(c):
        def pay(own, others):
            hawks, doves = others
            return F(4 - c, 2) * hawks + 4 * doves if own == 0 else F(2 * doves)
        return pay

    def sh(c):
        def pay(own, others):
            return F(4 * others[0]) if own == 0 else F(3) - c
        return pay
    yield "hawk-dove.yaml", "hawk_dove", None, 2, ["hawk", "dove"], hd(6), [F(2, 3), F(1, 3)]
    yield "hawk-dove.yaml", "hawk_dove", "costly", 2, ["hawk", "dove"], hd(8), [F(1, 2), F(1, 2)]
    yield "hawk-dove.yaml", "hawk_dove", "deadly", 2, ["hawk", "dove"], hd(12), [F(1, 3), F(2, 3)]
    yield "stag-hunt.yaml", "stag_hunt", None, 2, ["stag", "hare"], sh(0), [1, 0]
    yield "stag-hunt.yaml", "stag_hunt", "few_stags", 2, ["stag", "hare"], sh(0), [0, 1]
    yield "stag-hunt.yaml", "stag_hunt", "c3", 2, ["stag", "hare"], sh(F(3, 2)), [1, 0]
    # The bagel's price war across the city (examples/bagel/price_war.yaml):
    # a month's margins from the plan's numbers, exactly; no end by hand
    for scenario, share in ((None, F("0.70")), ("loyal_regulars", F("0.55"))):
        yield "../bagel/price_war.yaml", "price_war", scenario, 2, ["hold", "cut"], price_war(share), None


def price_war(share):
    """R, S, T and P of the bagel's price war, as its file works them out."""
    food = F(12, 9) * F("1.15") + F(12 * 6, 1000) * F("9.40") + F("0.35")
    hold, cut = 16 * F("0.974") - food, 14 * F("0.974") - food
    r, s = F(1, 2) * 110 * hold * 30, (1 - share) * 110 * hold * 30
    t, p = share * 110 * cut * 30, F(1, 2) * F("115.5") * cut * 30

    def pay(own, others):
        holds, cuts = others
        return (r * holds + s * cuts) if own == 0 else (t * holds + p * cuts)
    return pay


def cases(seed, count):
    rng = random.Random(seed)
    out = [hawk_dove([F(1, 10), F(9, 10)], 300, 5), hawk_dove([F(9, 10), F(1, 10)], 300, 5),
           stag_hunt(F(76, 100)), stag_hunt(F(74, 100)), volunteers(),
           rock_paper_scissors([F(1, 3)] * 3, [1 / 3] * 3),
           rock_paper_scissors([F(1, 2), F(3, 10), F(1, 5)], None)]
    return out + [draw(rng, i) for i in range(count)]


def refused(work):
    """Hawk-dove with baseline 0 from hawks only: hawk pays -1."""
    text = hawk_dove([F(1), F(0)], 5, 0)[2]
    path = os.path.join(work, "dynamics_refused.yaml")
    with open(path, "w") as f:
        f.write(text)
    done = subprocess.run([tables.FORGE, "game", path], capture_output=True, text=True)
    said = done.stdout + done.stderr
    want = "in generation 0 (0 is the start), hawk pays -1 and the baseline is 0"
    if done.returncode == 0 or want not in said or "a baseline above 1 is enough" not in said:
        return [f"a baseline of 0 with hawk paying -1: exit {done.returncode}, {said[-600:]}"]
    return []
