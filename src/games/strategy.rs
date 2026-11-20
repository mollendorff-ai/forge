//! The built-in strategies of a tournament (ADR-068, section 1).
//!
//! Each is a small machine: at most three states, a chance of cooperating in each, and
//! the state it moves to after a round, from the moves that were played.
//!
//! They behave as the strategies of the Python `axelrod` library with the
//! same names, which is how they are checked (`axl.Grudger` for
//! `grim_trigger`, `axl.GTFT` for `generous_tit_for_tat`).

use serde::Serialize;

/// A built-in strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plays {
    AlwaysCooperate,
    AlwaysDefect,
    TitForTat,
    SuspiciousTitForTat,
    TitForTwoTats,
    GrimTrigger,
    WinStayLoseShift,
    GenerousTitForTat,
    Random,
}

/// The state after a round, from the state and whether this player and
/// the other cooperated in it.
pub type Next = fn(usize, bool, bool) -> usize;

/// A strategy as a machine.
#[derive(Debug, Clone)]
pub struct Machine {
    /// The chance of cooperating in each state
    pub cooperate: Vec<f64>,
    /// The state of the first round
    pub start: usize,
    /// The state after a round, from the state and whether this player and
    /// the other cooperated in it (the moves played, after any noise)
    pub next: Next,
}

/// Stays in its one state.
const fn same(_: usize, _: bool, _: bool) -> usize {
    0
}

/// State 0 after the other cooperated, 1 after it defected.
const fn answer(_: usize, _: bool, other: bool) -> usize {
    if other {
        0
    } else {
        1
    }
}

impl Plays {
    /// Every built-in strategy, in the order the ADR lists them.
    pub const ALL: [Self; 9] = [
        Self::AlwaysCooperate,
        Self::AlwaysDefect,
        Self::TitForTat,
        Self::SuspiciousTitForTat,
        Self::TitForTwoTats,
        Self::GrimTrigger,
        Self::WinStayLoseShift,
        Self::GenerousTitForTat,
        Self::Random,
    ];

    /// The name a model gives the strategy.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AlwaysCooperate => "always_cooperate",
            Self::AlwaysDefect => "always_defect",
            Self::TitForTat => "tit_for_tat",
            Self::SuspiciousTitForTat => "suspicious_tit_for_tat",
            Self::TitForTwoTats => "tit_for_two_tats",
            Self::GrimTrigger => "grim_trigger",
            Self::WinStayLoseShift => "win_stay_lose_shift",
            Self::GenerousTitForTat => "generous_tit_for_tat",
            Self::Random => "random",
        }
    }

    /// The strategy a model names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|plays| plays.name() == name)
    }

    /// The parameter the strategy takes, a chance, and its value when the
    /// model does not give one.
    #[must_use]
    pub const fn parameter(self) -> Option<(&'static str, f64)> {
        match self {
            // axelrod's GTFT works its default out from the payoffs; 1/3 is
            // what it gives for 3, 0, 5, 1, and Forge's default for any
            Self::GenerousTitForTat => Some(("forgive", 1.0 / 3.0)),
            Self::Random => Some(("cooperate", 0.5)),
            _ => None,
        }
    }

    /// The strategy as a machine. `chance` is its parameter's value; a
    /// strategy without a parameter does not read it.
    #[must_use]
    pub fn machine(self, chance: f64) -> Machine {
        let (cooperate, start, next): (Vec<f64>, usize, Next) = match self {
            Self::AlwaysCooperate => (vec![1.0], 0, same),
            Self::AlwaysDefect => (vec![0.0], 0, same),
            Self::Random => (vec![chance], 0, same),
            Self::TitForTat => (vec![1.0, 0.0], 0, answer),
            Self::SuspiciousTitForTat => (vec![1.0, 0.0], 1, answer),
            // Answers a defection with cooperation with the chance `forgive`
            Self::GenerousTitForTat => (vec![1.0, chance], 0, answer),
            // Defects for good once the other has defected; its own moves,
            // flipped by noise, do not set it off (as axl.Grudger)
            Self::GrimTrigger => (vec![1.0, 0.0], 0, |state, _, other| {
                usize::from(state == 1 || !other)
            }),
            // Cooperates when both played the same last round: stays
            // after the other cooperated, shifts after it defected
            Self::WinStayLoseShift => {
                (vec![1.0, 0.0], 0, |_, own, other| usize::from(own != other))
            },
            // Defects only after two defections in a row by the other:
            // state 1 is one defection, 2 is two or more
            Self::TitForTwoTats => (vec![1.0, 1.0, 0.0], 0, |state, _, other| {
                if other {
                    0
                } else {
                    (state + 1).min(2)
                }
            }),
        };
        Machine {
            cooperate,
            start,
            next,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The states a machine goes through against a list of the other's
    /// moves, when it plays what its state says (no noise, no chance).
    fn states(plays: Plays, other: &[bool]) -> Vec<usize> {
        let machine = plays.machine(0.5);
        let mut state = machine.start;
        let mut seen = vec![state];
        for &theirs in other {
            let own = machine.cooperate[state] > 0.5;
            state = (machine.next)(state, own, theirs);
            seen.push(state);
        }
        seen
    }

    #[test]
    fn every_name_is_read_back() {
        for plays in Plays::ALL {
            assert_eq!(Plays::from_name(plays.name()), Some(plays));
        }
        assert_eq!(Plays::from_name("tit_for_three_tats"), None);
        assert_eq!(
            Plays::GenerousTitForTat.parameter(),
            Some(("forgive", 1.0 / 3.0))
        );
        assert_eq!(Plays::Random.parameter(), Some(("cooperate", 0.5)));
        assert_eq!(Plays::TitForTat.parameter(), None);
    }

    #[test]
    fn each_machine_moves_as_its_axelrod_namesake() {
        let (c, d) = (true, false);
        // By hand, from axelrod 4.14.0's source for each strategy
        assert_eq!(states(Plays::AlwaysCooperate, &[d, c]), [0, 0, 0]);
        assert_eq!(states(Plays::AlwaysDefect, &[d]), [0, 0]);
        assert_eq!(states(Plays::Random, &[d]), [0, 0]);
        // Copies the other's last move; the suspicious one starts with D
        assert_eq!(states(Plays::TitForTat, &[d, c, d]), [0, 1, 0, 1]);
        assert_eq!(states(Plays::SuspiciousTitForTat, &[c, d]), [1, 0, 1]);
        assert_eq!(states(Plays::GenerousTitForTat, &[d, c]), [0, 1, 0]);
        // Never forgives
        assert_eq!(states(Plays::GrimTrigger, &[c, d, c, c]), [0, 0, 1, 1, 1]);
        // (C, C) stays at C; (C, D) shifts to D; (D, C) stays at D; (D, D)
        // shifts back to C
        assert_eq!(
            states(Plays::WinStayLoseShift, &[c, d, c, d]),
            [0, 0, 1, 1, 0]
        );
        // Two defections in a row before it defects, and one C resets it
        assert_eq!(
            states(Plays::TitForTwoTats, &[d, c, d, d, d, c]),
            [0, 1, 0, 1, 2, 2, 0]
        );
        let generous = Plays::GenerousTitForTat.machine(0.25);
        assert_eq!(generous.cooperate, vec![1.0, 0.25]);
        assert_eq!(Plays::Random.machine(0.75).cooperate, vec![0.75]);
        assert_eq!(Plays::TitForTwoTats.machine(0.5).cooperate, [1.0, 1.0, 0.0]);
        assert_eq!(Plays::AlwaysDefect.machine(0.5).cooperate, [0.0]);
    }
}
