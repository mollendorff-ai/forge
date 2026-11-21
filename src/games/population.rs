//! The symmetric equilibria of a symmetric game of two strategies, for any
//! number of players, and whether a population returns to each (ADR-073,
//! question 4).
//!
//! When each of the other players plays the first strategy, A, with
//! probability x, A pays more than the second, B, by
//!
//! g(x) = Σₖ C(m, k) xᵏ (1 − x)ᵐ⁻ᵏ dₖ, with m = n − 1 others,
//!
//! where dₖ is what A pays less what B pays when k of the others play A. A mix
//! x in (0, 1) is an equilibrium when g(x) = 0. Playing B for certain is one
//! when d₀ ≤ 0, and A for certain when dₘ ≥ 0. The replicator dynamics move
//! the share of A up where g > 0 and down where g < 0, so a population returns
//! to an equilibrium when g is positive just below it and negative just above.
//!
//! The roots are found with `polycool`, so the list is complete:
//! - a root at either end, where d₀ or dₘ is a tie, is divided out exactly
//!   first (in Bernstein form, g = (1 − x) h with h's coefficients
//!   dₖ m / (m − k)), so none is found just inside the interval;
//! - every sign change in (0, 1) is a root;
//! - a point where g touches 0 without changing sign is a root of g′ where g
//!   is within the tie;
//! - two roots between which g stays within the tie are one.
//!
//! Checked against the exact roots (sympy's real-root isolation) of 31,500
//! polynomials with integer coefficients, of two to eight players: the same
//! roots, and the same number of them, in every one.

use super::table::Table;
use crate::core::precision::count;
use polycool::PolyDyn;

/// How close a root is narrowed down.
const NARROW: f64 = 1e-14;

/// Shares closer than this are the same equilibrium.
const SAME: f64 = 1e-6;

/// A symmetric equilibrium: the probability with which every player plays
/// the first strategy, and whether the dynamics return to it after a small
/// change in the shares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rest {
    pub share: f64,
    pub stable: bool,
}

/// What a population of players of a symmetric game of two strategies gets.
#[derive(Debug, Clone, PartialEq)]
pub struct Population {
    /// What the first strategy pays with k of the others playing it, for k
    /// from 0 to n − 1
    pub first: Vec<f64>,
    /// What the first strategy pays less what the second pays, the same way
    pub advantage: Vec<f64>,
    /// Whether the two strategies pay the same against every mix: then every
    /// mix is an equilibrium, and `rests` is empty
    pub degenerate: bool,
    /// Every symmetric equilibrium, by increasing share: playing the second
    /// strategy for certain (share 0), the mixes, then the first (share 1)
    pub rests: Vec<Rest>,
}

/// The population of `table`, if each player has two strategies. The table
/// must be symmetric: the first player's payoffs are read.
#[must_use]
pub fn population(table: &Table) -> Option<Population> {
    if table.strategies[0].len() != 2 {
        return None;
    }
    let others = table.players.len() - 1;
    let pays = |own: usize, k: usize| {
        let mut profile = vec![own];
        profile.extend(std::iter::repeat_n(0, k));
        profile.extend(std::iter::repeat_n(1, others - k));
        table.payoff(&profile, 0)
    };
    let first: Vec<f64> = (0..=others).map(|k| pays(0, k)).collect();
    let advantage: Vec<f64> = (0..=others).map(|k| first[k] - pays(1, k)).collect();
    let tie = table.tie();
    if advantage.iter().all(|d| d.abs() <= tie) {
        return Some(Population {
            first,
            advantage,
            degenerate: true,
            rests: Vec::new(),
        });
    }
    let mixes = roots(&advantage, tie);
    // The sign of g between each pair of neighbouring points of 0, the
    // mixes and 1
    let mut points = vec![0.0];
    points.extend(&mixes);
    points.push(1.0);
    let rising: Vec<bool> = points
        .windows(2)
        .map(|w| bernstein(&advantage, f64::midpoint(w[0], w[1])) > 0.0)
        .collect();
    let mut rests = Vec::new();
    if advantage[0] <= tie {
        rests.push(Rest {
            share: 0.0,
            stable: !rising[0],
        });
    }
    for (i, &share) in mixes.iter().enumerate() {
        rests.push(Rest {
            share,
            stable: rising[i] && !rising[i + 1],
        });
    }
    if advantage[others] >= -tie {
        rests.push(Rest {
            share: 1.0,
            stable: rising[mixes.len()],
        });
    }
    Some(Population {
        first,
        advantage,
        degenerate: false,
        rests,
    })
}

impl Population {
    /// What every player expects when each plays the first strategy with
    /// probability `share`.
    #[must_use]
    pub fn payoff(&self, share: f64) -> f64 {
        // At an equilibrium both strategies pay the same, so the first's
        // payoff is everyone's
        bernstein(&self.first, share)
    }

    /// The equilibrium at `share`, if there is one.
    #[must_use]
    pub fn rest_at(&self, share: f64) -> Option<Rest> {
        self.rests
            .iter()
            .find(|r| (r.share - share).abs() < SAME)
            .copied()
    }
}

/// The value at `x` of the polynomial with Bernstein coefficients `b`, by de
/// Casteljau's algorithm.
fn bernstein(b: &[f64], x: f64) -> f64 {
    let mut level = b.to_vec();
    for size in (1..level.len()).rev() {
        for i in 0..size {
            level[i] = (1.0 - x).mul_add(level[i], x * level[i + 1]);
        }
    }
    level[0]
}

/// The coefficients of 1, x, x², … of the polynomial with Bernstein
/// coefficients `b`.
fn power(b: &[f64]) -> Vec<f64> {
    let m = b.len() - 1;
    let mut c = vec![0.0; m + 1];
    for (k, &bk) in b.iter().enumerate() {
        // C(m, k) xᵏ (1 − x)ᵐ⁻ᵏ = Σⱼ C(m, k) C(m − k, j) (−1)ʲ xᵏ⁺ʲ
        let outer = choose(m, k) * bk;
        for j in 0..=m - k {
            let sign = if j % 2 == 0 { 1.0 } else { -1.0 };
            c[k + j] = (sign * outer).mul_add(choose(m - k, j), c[k + j]);
        }
    }
    c
}

/// The binomial coefficient C(m, k).
fn choose(m: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |c, i| c * count(m - i) / count(i + 1))
}

/// The roots in (0, 1) of the polynomial with Bernstein coefficients `d`, not
/// all within `tie` of 0, by increasing value (see the module's description).
fn roots(d: &[f64], tie: f64) -> Vec<f64> {
    let mut d = d.to_vec();
    loop {
        let m = d.len() - 1;
        if m > 0 && d[m].abs() <= tie {
            // A root at 1: g = (1 − x) h
            d = (0..m).map(|k| d[k] * count(m) / count(m - k)).collect();
        } else if m > 0 && d[0].abs() <= tie {
            // A root at 0: g = x h
            d = (0..m).map(|j| d[j + 1] * count(m) / count(j + 1)).collect();
        } else {
            break;
        }
    }
    let polynomial = PolyDyn::new(power(&d));
    let mut found = polynomial.roots_between(0.0, 1.0, NARROW);
    for at in polynomial.deriv().roots_between(0.0, 1.0, NARROW) {
        if bernstein(&d, at).abs() <= tie {
            found.push(at);
        }
    }
    found.retain(|&x| x > 0.0 && x < 1.0);
    found.sort_by(f64::total_cmp);
    let mut merged: Vec<f64> = Vec::new();
    for x in found {
        match merged.last_mut() {
            Some(last) if bernstein(&d, f64::midpoint(*last, x)).abs() <= tie => {
                *last = f64::midpoint(*last, x);
            },
            _ => merged.push(x),
        }
    }
    merged
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A symmetric table of `n` players and strategies a and b, in which a
    /// player gets `pays(own, k)` with k of the others playing a.
    pub fn symmetric(n: usize, pays: impl Fn(usize, usize) -> f64) -> Table {
        let strategies = vec![vec!["a".to_string(), "b".to_string()]; n];
        let mut table = Table {
            players: (1..=n).map(|p| format!("p{p}")).collect(),
            strategies,
            payoffs: Vec::new(),
        };
        table.payoffs = table
            .profiles()
            .iter()
            .map(|profile| {
                (0..n)
                    .map(|p| {
                        let k = (0..n).filter(|&o| o != p && profile[o] == 0).count();
                        pays(profile[p], k)
                    })
                    .collect()
            })
            .collect();
        table
    }

    fn shares(p: &Population) -> Vec<f64> {
        p.rests.iter().map(|r| r.share).collect()
    }

    fn close(a: &[f64], b: &[f64], within: f64) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < within)
    }

    #[test]
    fn the_volunteers_dilemma_has_the_closed_form_mix() {
        // Each volunteers (a) at cost c, and all get b if anyone does. By
        // hand, a player who does not volunteer is indifferent when the
        // others all stay out with probability q, q^(n − 1) b = b − c, so
        // q = (c / b)^(1 / (n − 1)), and the share of a is 1 − q. Neither
        // pure symmetric profile is an equilibrium for n ≥ 2: alone, a
        // player volunteers; with everyone volunteering, a player stays out.
        let (b, c) = (10.0, 4.0);
        for n in 2..=8 {
            let table = symmetric(n, |own, k| match (own, k) {
                (0, _) => b - c,
                (_, 0) => 0.0,
                _ => b,
            });
            let p = population(&table).unwrap();
            let q = (c / b).powf(1.0 / count(n - 1));
            assert!(close(&shares(&p), &[1.0 - q], 1e-12), "n = {n}: {p:?}");
            // Below the mix volunteering pays more, above it less: the
            // dynamics return to it
            assert!(p.rests[0].stable);
            assert!((p.payoff(1.0 - q) - (b - c)).abs() < 1e-12);
            assert!(!p.degenerate);
        }
    }

    #[test]
    fn the_stag_hunt_has_two_stable_pure_rests_and_an_unstable_mix() {
        // Two players, stag (a) 4 against stag and 0 against hare, hare 3
        // either way: g(x) = 4x − 3, a root at 3/4, below which the share
        // of stag falls and above which it rises.
        let table = symmetric(2, |own, k| match (own, k) {
            (0, 1) => 4.0,
            (0, _) => 0.0,
            _ => 3.0,
        });
        let p = population(&table).unwrap();
        assert!(close(&shares(&p), &[0.0, 0.75, 1.0], 1e-12), "{p:?}");
        let stable: Vec<bool> = p.rests.iter().map(|r| r.stable).collect();
        assert_eq!(stable, [true, false, true]);
        assert_eq!(p.rest_at(0.75 + 1e-8), Some(p.rests[1]));
        assert_eq!(p.rest_at(0.5), None);
        assert!((p.payoff(0.75) - 3.0).abs() < 1e-12);
        assert_eq!(p.advantage, [-3.0, 1.0]);
    }

    #[test]
    fn the_public_goods_game_at_r_equal_to_n_is_degenerate() {
        // By hand: contributing pays r e (k + 1) / n and keeping e + r e k / n,
        // so contributing pays e (r / n − 1) more, 0 at r = n
        let table = symmetric(5, |own, k| {
            let k = count(k);
            if own == 0 {
                5.0 * 10.0 * (k + 1.0) / 5.0
            } else {
                10.0 + 5.0 * 10.0 * k / 5.0
            }
        });
        let p = population(&table).unwrap();
        assert!(p.degenerate);
        assert!(p.rests.is_empty());
        assert_eq!(p.rest_at(0.0), None);
    }

    #[test]
    fn three_strategies_have_no_population() {
        let mut table = symmetric(2, |_, _| 0.0);
        table.strategies = vec![vec!["a".into(), "b".into(), "c".into()]; 2];
        assert_eq!(population(&table), None);
    }

    /// The roots in (0, 1) of g with Bernstein coefficients `d`, as a game of
    /// `d.len()` players in which a pays dₖ and b pays 0.
    fn mixes(d: &[f64]) -> Vec<f64> {
        let table = symmetric(d.len(), |own, k| if own == 0 { d[k] } else { 0.0 });
        let p = population(&table).unwrap();
        p.rests
            .iter()
            .map(|r| r.share)
            .filter(|&s| s > 0.0 && s < 1.0)
            .collect()
    }

    #[test]
    fn roots_agree_with_their_exact_values() {
        // Each exact value from sympy's real_roots of the polynomial
        // Σ C(m, k) xᵏ (1 − x)ᵐ⁻ᵏ dₖ
        // (2x − 1)² (7x − 2): 1/2 touches 0 without a sign change
        assert!(close(
            &mixes(&[-2.0, 3.0, -4.0, 5.0]),
            &[2.0 / 7.0, 0.5],
            1e-7
        ));
        // A double root at 0 divided out: 3/7 only, none just above 0
        assert!(close(&mixes(&[0.0, 0.0, 1.0, -4.0]), &[3.0 / 7.0], 1e-12));
        // A double root at 1 divided out: (6 ± √19) / 17
        let s = 19.0_f64.sqrt();
        assert!(close(
            &mixes(&[-2.0, 5.0, -2.0, 0.0, 0.0]),
            &[(6.0 - s) / 17.0, (6.0 + s) / 17.0],
            1e-12
        ));
        // A double root at 1/2 found as two sign changes 1e-8 apart: one
        assert!(close(&mixes(&[-1.0, 0.0, 1.0, -2.0]), &[0.5], 1e-7));
        // Five roots, of Π (x − rᵢ) written in Bernstein form: a game of six
        // players
        let r = [1.0 / 6.0, 1.0 / 3.0, 0.5, 2.0 / 3.0, 5.0 / 6.0];
        let found = mixes(&to_bernstein(&power_of_roots(&r)));
        assert!(close(&found, &r, 1e-9), "{found:?}");
        // A constant, nonzero g: no mix
        assert!(mixes(&[1.0, 1.0, 1.0]).is_empty());
        // A root at both ends, divided out down to a constant
        assert!(mixes(&[0.0, 2.0, 0.0]).is_empty());
    }

    /// The coefficients of 1, x, x², … of Π (x − rᵢ).
    fn power_of_roots(r: &[f64]) -> Vec<f64> {
        r.iter().fold(vec![1.0], |c, ri| {
            let mut next = vec![0.0; c.len() + 1];
            for (i, ci) in c.iter().enumerate() {
                next[i + 1] += ci;
                next[i] -= ri * ci;
            }
            next
        })
    }

    /// The Bernstein coefficients of a polynomial given by its coefficients
    /// of 1, x, x², …: bₖ = Σ_{i ≤ k} C(k, i) / C(m, i) cᵢ.
    fn to_bernstein(c: &[f64]) -> Vec<f64> {
        let m = c.len() - 1;
        (0..=m)
            .map(|k| (0..=k).map(|i| choose(k, i) / choose(m, i) * c[i]).sum())
            .collect()
    }

    #[test]
    fn the_power_form_and_the_bernstein_form_agree() {
        let b = [3.0, -1.0, 4.0, -1.5, 5.0];
        let c = power(&b);
        assert!(close(&to_bernstein(&c), &b, 1e-12));
        for x in [0.0, 0.3, 0.5, 0.9, 1.0] {
            let direct: f64 = c.iter().rev().fold(0.0, |v, ci| v.mul_add(x, *ci));
            assert!((direct - bernstein(&b, x)).abs() < 1e-12);
        }
        assert!((choose(7, 3) - 35.0).abs() < 1e-12);
    }

    #[test]
    fn a_pure_rest_on_a_tie_is_listed_and_its_stability_read_from_inside() {
        // d = (0, 1, 1): playing b for certain is an equilibrium (a pays the
        // same against all b), and g = 2x (1 − x) + x² > 0 inside, so a
        // population near all-b moves away from it: not stable. d₂ = 1 > 0:
        // all-a is an equilibrium, and stable.
        let table = symmetric(3, |own, k| if own == 0 { [0.0, 1.0, 1.0][k] } else { 0.0 });
        let p = population(&table).unwrap();
        assert!(close(&shares(&p), &[0.0, 1.0], 1e-12), "{p:?}");
        assert!(!p.rests[0].stable);
        assert!(p.rests[1].stable);
    }
}
