"""The named games, each checked against Gambit.

A spec is a payoff table written here, as numbers, so that Gambit and Forge
read the same game. It is run as a model generated from the table and, where a
file in examples/games holds the same game, through that file as well, so the
examples that ship are checked against Gambit too.
"""
from fractions import Fraction as F

EXAMPLES = "../../examples/games"


def symmetric(m):
    """A symmetric game: the column player's payoff is the mirror cell."""
    return m, [[m[j][i] for j in range(len(m))] for i in range(len(m))]


class Spec:
    def __init__(self, name, a, b, rows=None, cols=None, example=None):
        self.name, self.a, self.b, self.rows, self.cols = name, a, b, rows, cols
        # (file in examples/games, game, scenario) when an example is the same game
        self.example = example


def stag_hunt(c, scenario):
    a, b = symmetric([[F(4), F(0)], [F(3) - c, F(3) - c]])
    return Spec(f"stag hunt, c = {c}", a, b, ["stag", "hare"], ["stag", "hare"],
                (f"{EXAMPLES}/stag-hunt.yaml", "stag_hunt", scenario))


def chicken(c, scenario):
    a, b = symmetric([[F(3), F(1)], [F(5) - c, -c]])
    return Spec(f"chicken, c = {c}", a, b, ["swerve", "straight"], ["swerve", "straight"],
                (f"{EXAMPLES}/chicken.yaml", "chicken", scenario))


def dilemma(c, scenario):
    a, b = symmetric([[F(3), F(0)], [F(5) - c, F(1) - c]])
    return Spec(f"prisoner's dilemma, c = {c}", a, b, ["cooperate", "defect"], ["cooperate", "defect"],
                (f"{EXAMPLES}/axelrod.yaml", "dilemma", scenario))


def inspection(penalty, example):
    # rows comply and cheat, columns audit and trust; the AI's payoff, then the human's
    a = [[F(0), F(0)], [-penalty, F(2)]]
    b = [[F(-1), F(0)], [F(-1), F(-3)]]
    return Spec(f"inspection, penalty {penalty}", a, b, ["comply", "cheat"], ["audit", "trust"],
                (f"{EXAMPLES}/inspection.yaml", "inspection", None) if example else None)


SPECS = [
    stag_hunt(F(0), None), stag_hunt(F(1, 2), "c1"), stag_hunt(F(1), "c2"), stag_hunt(F(3, 2), "c3"),
    chicken(F(0), None), chicken(F(1, 4), "c1"), chicken(F(1, 2), "c2"), chicken(F(1), "c3"),
    dilemma(F(0), None), dilemma(F(1, 10), "c1"), dilemma(F(1, 5), "c2"), dilemma(F(3, 10), "c3"),
    inspection(F(4), True), inspection(F(8), False), inspection(F(2), False),
    Spec("battle of the sexes", [[F(3), F(0)], [F(0), F(2)]], [[F(2), F(0)], [F(0), F(3)]],
         ["opera", "football"], ["opera", "football"],
         (f"{EXAMPLES}/battle-of-the-sexes.yaml", "evening", None)),
    Spec("matching pennies", [[F(1), F(-1)], [F(-1), F(1)]], [[F(-1), F(1)], [F(1), F(-1)]]),
    Spec("rock, paper, scissors",
         [[F(0), F(-1), F(1)], [F(1), F(0), F(-1)], [F(-1), F(1), F(0)]],
         [[F(0), F(1), F(-1)], [F(-1), F(0), F(1)], [F(1), F(-1), F(0)]]),
    Spec("coordination on three", *symmetric([[F(3), F(0), F(0)], [F(0), F(2), F(0)], [F(0), F(0), F(1)]])),
    Spec("three steps of dominance",
         [[F(4), F(3), F(0)], [F(3), F(2), F(1)], [F(1), F(1), F(0)]],
         [[F(1), F(2), F(0)], [F(0), F(3), F(1)], [F(2), F(1), F(0)]]),
    Spec("two by three, no pure equilibrium",
         [[F(3), F(0), F(1)], [F(0), F(3), F(1)]], [[F(0), F(3), F(1)], [F(3), F(0), F(1)]]),
    # Ties: the second row pays the same as the first against the first column
    Spec("degenerate: a tie against one column",
         [[F(1), F(1)], [F(0), F(2)], [F(2), F(0)]], [[F(1), F(1)], [F(2), F(0)], [F(0), F(2)]]),
    Spec("degenerate: equal strategies", *symmetric([[F(2), F(2), F(0)], [F(2), F(2), F(0)], [F(0), F(0), F(1)]])),
]
