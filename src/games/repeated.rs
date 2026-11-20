//! A match of two built-in strategies, played exactly (ADR-068).
//!
//! The match is a joint Markov chain: a state for each pair of the
//! machines' states, at most nine. Its expected
//! payoffs and cooperation are exact: the chain is stepped round by round
//! for a fixed number of rounds, or, for a match that goes on after each
//! round with a fixed chance, solved as one linear system.

use super::linear::solve;
use super::strategy::Machine;

/// How long a match is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// Exactly this many rounds
    Rounds(u64),
    /// After each round the match goes on with this chance, below 1; the
    /// expected number of rounds is 1 / (1 - chance)
    Continuation(f64),
}

/// What one side of a match expects, per round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Side {
    /// The expected payoff per round: the expected total over the expected
    /// number of rounds
    pub payoff: f64,
    /// The expected share of its moves that are cooperation
    pub cooperation: f64,
}

/// The chance that a player whose state cooperates with `cooperate` plays
/// cooperate, when each move is flipped with `noise`.
fn played(cooperate: f64, noise: f64) -> f64 {
    cooperate.mul_add(1.0 - noise, (1.0 - cooperate) * noise)
}

/// Play `a` against `b`, and return what each expects.
///
/// `round[own][other]` is the payoff to a player
/// from its own move and the other's, 0 for cooperate and 1 for defect; the
/// round is symmetric, so it gives both sides' payoffs. Each move is flipped
/// with the chance `noise`, and the machines see the moves as played.
///
/// # Panics
///
/// Never in practice: with a continuation below 1, the system solved is
/// strictly diagonally dominant, so it has one solution.
#[must_use]
pub fn play(
    a: &Machine,
    b: &Machine,
    round: &[[f64; 2]; 2],
    length: Length,
    noise: f64,
) -> [Side; 2] {
    let width = b.cooperate.len();
    let states = a.cooperate.len() * width;
    // For each joint state: where it goes, and what each side expects from
    // the round played in it (a's payoff, b's, a's cooperation, b's)
    let mut moves = vec![vec![0.0; states]; states];
    let mut gain = vec![[0.0; 4]; states];
    for (x, &first) in a.cooperate.iter().enumerate() {
        for (y, &second) in b.cooperate.iter().enumerate() {
            let at = x * width + y;
            let (pa, pb) = (played(first, noise), played(second, noise));
            for (ca, wa) in [(true, pa), (false, 1.0 - pa)] {
                for (cb, wb) in [(true, pb), (false, 1.0 - pb)] {
                    let chance = wa * wb;
                    let (ia, ib) = (usize::from(!ca), usize::from(!cb));
                    let pays = [
                        round[ia][ib],
                        round[ib][ia],
                        f64::from(u8::from(ca)),
                        f64::from(u8::from(cb)),
                    ];
                    for (g, p) in gain[at].iter_mut().zip(pays) {
                        *g = chance.mul_add(p, *g);
                    }
                    let to = (a.next)(x, ca, cb) * width + (b.next)(y, cb, ca);
                    moves[at][to] += chance;
                }
            }
        }
    }
    let start = a.start * width + b.start;
    let mean = match length {
        Length::Rounds(rounds) => stepped(&moves, &gain, start, rounds),
        Length::Continuation(w) => solved(&moves, &gain, start, w),
    };
    [
        Side {
            payoff: mean[0],
            cooperation: mean[2],
        },
        Side {
            payoff: mean[1],
            cooperation: mean[3],
        },
    ]
}

/// The mean per round over exactly `rounds` rounds, stepping the chain.
fn stepped(moves: &[Vec<f64>], gain: &[[f64; 4]], start: usize, rounds: u64) -> [f64; 4] {
    let mut at: Vec<f64> = vec![0.0; gain.len()];
    at[start] = 1.0;
    let mut total = [0.0; 4];
    for _ in 0..rounds {
        let mut next = vec![0.0; gain.len()];
        for ((&p, g), row) in at.iter().zip(gain).zip(moves) {
            for (t, v) in total.iter_mut().zip(g) {
                *t = p.mul_add(*v, *t);
            }
            for (n, m) in next.iter_mut().zip(row) {
                *n = p.mul_add(*m, *n);
            }
        }
        at = next;
    }
    let count = crate::core::precision::unsigned(rounds);
    total.map(|t| t / count)
}

/// The expected total over the expected number of rounds, 1 / (1 - w), when
/// the match goes on after each round with the chance `w`.
///
/// Round t (from 0) is played with the chance w^t, so the expected visits
/// to each state are v = start (I - w P)^-1, and the mean is (1 - w) v . gain.
fn solved(moves: &[Vec<f64>], gain: &[[f64; 4]], start: usize, w: f64) -> [f64; 4] {
    let n = gain.len();
    let one = |i: usize, j: usize| f64::from(u8::from(i == j));
    // The transpose of I - w P, so that v is the solution of a x = start
    let a: Vec<Vec<f64>> = (0..n)
        .map(|j| (0..n).map(|i| w.mul_add(-moves[i][j], one(i, j))).collect())
        .collect();
    let b: Vec<f64> = (0..n).map(|i| one(i, start)).collect();
    let visits = solve(a, b).expect("I - w P is diagonally dominant when w < 1");
    let mut mean = [0.0; 4];
    for (v, g) in visits.iter().zip(gain) {
        for (m, value) in mean.iter_mut().zip(g) {
            *m = ((1.0 - w) * v).mul_add(*value, *m);
        }
    }
    mean
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::strategy::Plays;

    /// The prisoner's dilemma of Axelrod's tournaments: R 3, S 0, T 5, P 1.
    const DILEMMA: [[f64; 2]; 2] = [[3.0, 0.0], [5.0, 1.0]];

    fn side(a: Plays, b: Plays, length: Length, noise: f64) -> [Side; 2] {
        play(
            &a.machine(a.parameter().map_or(0.0, |p| p.1)),
            &b.machine(b.parameter().map_or(0.0, |p| p.1)),
            &DILEMMA,
            length,
            noise,
        )
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn tit_for_tat_against_always_defect_loses_only_the_first_round() {
        // By hand: (C, D) pays 0 and 5, then (D, D) pays 1 each for 199
        // rounds; TFT cooperates once in 200
        let [tft, ad] = side(
            Plays::TitForTat,
            Plays::AlwaysDefect,
            Length::Rounds(200),
            0.0,
        );
        assert!(close(tft.payoff, 199.0 / 200.0), "{}", tft.payoff);
        assert!(close(ad.payoff, (5.0 + 199.0) / 200.0), "{}", ad.payoff);
        assert!(close(tft.cooperation, 1.0 / 200.0));
        assert!(close(ad.cooperation, 0.0));
    }

    #[test]
    fn grim_trigger_against_random_is_worked_out_in_closed_form() {
        // By hand: Grim cooperates in round t + 1 only if Random cooperated
        // in all t rounds before it, a chance of 2^-t. In such a round Grim
        // expects (3 + 0) / 2; once triggered, (5 + 1) / 2. Over 200 rounds
        // Grim's total is 1.5 sum(2^-t) + 3 (200 - sum(2^-t)) with the sum
        // over t = 0..199, which is 2 - 2^-199: 600 - 1.5 (2 - 2^-199)
        let [grim, random] = side(Plays::GrimTrigger, Plays::Random, Length::Rounds(200), 0.0);
        let sum = 2.0 - 2.0_f64.powi(-199);
        assert!(
            close(grim.payoff, 1.5f64.mul_add(-sum, 600.0) / 200.0),
            "{}",
            grim.payoff
        );
        assert!(close(grim.cooperation, sum / 200.0));
        assert!(close(random.cooperation, 0.5));
    }

    #[test]
    fn noise_flips_each_move() {
        // By hand: two AlwaysCooperate players with noise 0.1 each play C
        // with 0.9, so each expects 0.81 * 3 + 0.09 * 0 + 0.09 * 5 + 0.01 * 1
        let [a, b] = side(
            Plays::AlwaysCooperate,
            Plays::AlwaysCooperate,
            Length::Rounds(7),
            0.1,
        );
        let expected = 0.81f64.mul_add(3.0, 0.09f64.mul_add(5.0, 0.01));
        assert!(close(a.payoff, expected) && close(b.payoff, expected));
        assert!(close(a.cooperation, 0.9));
    }

    #[test]
    fn a_match_that_ends_at_random_is_solved() {
        // By hand: AlwaysDefect against Grim with continuation w gets 5 in
        // the first round and 1 after, so its mean is (1 - w) 5 + w 1; at
        // w = 0.5 that is 3, Grim's R against itself: the threshold
        // (T - R) / (T - P) of ADR-068
        for w in [0.0, 0.5, 0.9] {
            let [ad, grim] = side(
                Plays::AlwaysDefect,
                Plays::GrimTrigger,
                Length::Continuation(w),
                0.0,
            );
            assert!(
                close(ad.payoff, (1.0 - w).mul_add(5.0, w)),
                "w {w}: {}",
                ad.payoff
            );
            assert!(close(grim.cooperation, 1.0 - w), "w {w}");
        }
        // Self-play of TFT never leaves mutual cooperation
        let [tft, _] = side(
            Plays::TitForTat,
            Plays::TitForTat,
            Length::Continuation(0.99),
            0.0,
        );
        assert!(close(tft.payoff, 3.0));
    }

    // axelrod 4.14.0, axl.Match one at a time with seeds 0 to 999: a's
    // payoff is its total over the turns, both summed across the runs
    // (prob_end 0.1 for a continuation of 0.9), and the standard error is
    // the delta method's. Every strategy is a and b once in each length.
    // All 81 pairs of the nine strategies were also compared with 400 runs
    // each, within 2.4 standard errors; here the tolerance is 3.
    const AC: Plays = Plays::AlwaysCooperate;
    const AD: Plays = Plays::AlwaysDefect;
    const TFT: Plays = Plays::TitForTat;
    const STFT: Plays = Plays::SuspiciousTitForTat;
    const TF2T: Plays = Plays::TitForTwoTats;
    const GRIM: Plays = Plays::GrimTrigger;
    const WSLS: Plays = Plays::WinStayLoseShift;
    const GTFT: Plays = Plays::GenerousTitForTat;
    const RAND: Plays = Plays::Random;
    const R200: Length = Length::Rounds(200);
    const W09: Length = Length::Continuation(0.9);
    /// a, b, length, noise, axelrod's mean for a, its standard error
    const AXELROD: [(Plays, Plays, Length, f64, f64, f64); 18] = [
        (TFT, AD, R200, 0.05, 1.09645, 0.00148),
        (AD, TFT, R200, 0.05, 1.34543, 0.00231),
        (STFT, WSLS, R200, 0.05, 2.25401, 0.00377),
        (WSLS, GTFT, R200, 0.05, 2.78414, 0.00259),
        (GTFT, TF2T, R200, 0.05, 2.99392, 0.00148),
        (TF2T, GRIM, R200, 0.05, 1.22776, 0.00529),
        (GRIM, RAND, R200, 0.05, 2.91371, 0.00455),
        (RAND, AC, R200, 0.05, 3.82292, 0.00286),
        (AC, STFT, R200, 0.05, 2.79489, 0.00156),
        (AD, GRIM, W09, 0.02, 1.46959, 0.01404),
        (GRIM, TF2T, W09, 0.02, 2.82835, 0.02346),
        (TF2T, WSLS, W09, 0.02, 2.62858, 0.02349),
        (WSLS, STFT, W09, 0.02, 2.04058, 0.01213),
        (STFT, GTFT, W09, 0.02, 3.00581, 0.00843),
        (GTFT, RAND, W09, 0.02, 1.96171, 0.01380),
        (RAND, TFT, W09, 0.02, 2.43877, 0.01150),
        (TFT, AC, W09, 0.02, 3.01411, 0.00383),
        (AC, AD, W09, 0.02, 0.08095, 0.00453),
    ];

    #[test]
    fn exact_values_agree_with_axelrods_average_over_seeded_runs() {
        for (a, b, length, noise, mean, se) in AXELROD {
            let [got, _] = side(a, b, length, noise);
            assert!(
                (got.payoff - mean).abs() < 3.0 * se,
                "{a:?} v {b:?}: {} against {mean}",
                got.payoff
            );
        }
    }
}
