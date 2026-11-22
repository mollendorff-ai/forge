//! Strategies that spread: the `dynamics:` key of a symmetric game and the
//! discrete replicator equation (ADR-073, question 3).
//!
//! ```yaml
//! hawk_dove:
//!   players: [me, you]
//!   strategies: [hawk, dove]
//!   symmetric: true
//!   payoffs: ...
//!   dynamics:
//!     start: {hawk: 0.1, dove: 0.9}   # every strategy, adding up to 1
//!     generations: 50
//!     baseline: 5                     # added to every payoff, 0 if not given
//! ```
//!
//! Generation by generation, each strategy's share xₛ becomes
//! xₛ (fₛ + b) / (f̄ + b), where fₛ is what s pays against the population,
//! averaged over every way the other players' strategies can fall, f̄ is the
//! average of the fₛ weighted by the shares, and b is the baseline. With m
//! other players and counts k of them playing each strategy,
//!
//! fₛ = Σₖ m! / Π kⱼ! · Π xⱼ^kⱼ · f(s, k),
//!
//! which for two strategies is the Bernstein sum of `population.rs`. The
//! arithmetic is exact: no step size and no solver.
//!
//! Every strategy that is played must pay more than minus the baseline, or
//! its share would fall below 0. A strategy whose share is 0 stays at 0
//! whatever it pays, so it is not checked: in the stag hunt with baseline 0,
//! stag pays 4x at a share x, and x can reach 0 as stag dies out.

use super::model::{describe, error, number_or_formula, Cell, Payoff};
use super::table::Table;
use crate::core::precision::count;
use crate::error::{ForgeError, ForgeResult};
use indexmap::IndexMap;
use num_traits::ToPrimitive;
use serde::Serialize;
use yaml_serde::{Mapping, Value};

/// The most generations a run may have.
pub const MOST_GENERATIONS: u64 = 100_000;

/// How far the start's shares may add up away from 1, for a start worked
/// out by formulas.
const ADDS_UP: f64 = 1e-9;

const KEYS: [&str; 3] = ["start", "generations", "baseline"];

/// The `dynamics:` key as written.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dynamics {
    /// The first generation's share of each strategy, in the game's order
    pub start: Vec<Cell>,
    pub generations: Cell,
    pub baseline: Cell,
}

/// The settings worked out.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub start: Vec<f64>,
    pub generations: usize,
    pub baseline: f64,
}

/// One generation of the path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Generation {
    /// Each strategy's share
    pub shares: IndexMap<String, f64>,
    /// The average payoff, without the baseline
    pub payoff: f64,
}

/// What the dynamics give: every generation, from the start.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DynamicsReport {
    pub generations: usize,
    pub baseline: f64,
    /// Generation 0 (the start) to the last, `generations + 1` in all
    pub path: Vec<Generation>,
}

impl DynamicsReport {
    /// The last generation.
    ///
    /// # Panics
    ///
    /// Never: a path has the start at least.
    #[must_use]
    pub fn end(&self) -> &Generation {
        self.path.last().expect("a path has its start")
    }
}

impl Dynamics {
    /// Every formula of the key, once each.
    #[must_use]
    pub fn formulas(&self) -> Vec<&str> {
        let mut all: Vec<&str> = Vec::new();
        for cell in self.start.iter().chain([&self.generations, &self.baseline]) {
            if let Payoff::Formula(f) = &cell.payoff {
                if !all.contains(&f.as_str()) {
                    all.push(f);
                }
            }
        }
        all
    }

    /// The settings, with every formula worked out by `value`, which is
    /// given the formula and where it is written.
    ///
    /// # Errors
    ///
    /// Returns the error `value` gives for the first formula that fails, or
    /// one naming a number out of its range: a share from 0 to 1, the shares
    /// adding up to 1, a whole number of generations from 1 to
    /// [`MOST_GENERATIONS`], and a baseline that is a number.
    pub fn work_out(
        &self,
        game: &str,
        mut value: impl FnMut(&str, &str) -> ForgeResult<f64>,
    ) -> ForgeResult<Settings> {
        let mut number = |cell: &Cell| match &cell.payoff {
            Payoff::Number(n) => Ok(*n),
            Payoff::Formula(f) => value(f, &cell.at),
        };
        let wrong = |cell: &Cell, got: f64, rule: &str| {
            ForgeError::Eval(format!("game '{game}': {} is {got}; {rule}", cell.at))
        };
        let mut start = Vec::with_capacity(self.start.len());
        for cell in &self.start {
            let share = number(cell)?;
            if !(0.0..=1.0).contains(&share) {
                return Err(wrong(cell, share, "a share is from 0 to 1"));
            }
            start.push(share);
        }
        let total: f64 = start.iter().sum();
        if (total - 1.0).abs() > ADDS_UP {
            let at = self.start[0].at.rsplit_once('.').map_or("", |(at, _)| at);
            return Err(ForgeError::Eval(format!(
                "game '{game}': the shares of {at} add up to {total}; the shares of every \
                 strategy add up to 1"
            )));
        }
        let got = number(&self.generations)?;
        let generations = got
            .to_u64()
            .filter(|g| got.fract() == 0.0 && (1..=MOST_GENERATIONS).contains(g))
            .and_then(|g| g.to_usize())
            .ok_or_else(|| {
                wrong(
                    &self.generations,
                    got,
                    &format!("it is a whole number of generations, from 1 to {MOST_GENERATIONS}"),
                )
            })?;
        let baseline = number(&self.baseline)?;
        if !baseline.is_finite() {
            return Err(wrong(
                &self.baseline,
                baseline,
                "it is added to every payoff",
            ));
        }
        Ok(Settings {
            start: start.iter().map(|s| s / total).collect(),
            generations,
            baseline,
        })
    }
}

/// Read `games.<name>.dynamics`, for a game with these strategies.
pub(super) fn parse(
    name: &str,
    value: &Value,
    symmetric: bool,
    strategies: &[String],
) -> ForgeResult<Dynamics> {
    let at = format!("games.{name}.dynamics");
    if !symmetric {
        return Err(error(format!(
            "'{at}': the dynamics follow one population whose members all play the same \
             game, so the game must be marked symmetric: true, and games.{name} is not"
        )));
    }
    let Value::Mapping(map) = value else {
        return Err(error(format!(
            "'{at}' must be a mapping with start, generations, and baseline if wanted"
        )));
    };
    if let Some(key) = map
        .keys()
        .find(|k| !k.as_str().is_some_and(|k| KEYS.contains(&k)))
    {
        return Err(error(format!(
            "'{at}' has {}, which the dynamics do not have: they have {}",
            describe(key),
            KEYS.join(", ")
        )));
    }
    let setting = |key: &str, value: &Value| {
        let place = format!("{at}.{key}");
        number_or_formula(value)
            .map(|payoff| Cell {
                payoff,
                at: place.clone(),
                others: Vec::new(),
            })
            .ok_or_else(|| {
                error(format!(
                    "'{place}' is {}: it is a number or a formula starting with =",
                    describe(value)
                ))
            })
    };
    let start = parse_start(&at, map, strategies, setting)?;
    let generations = map.get("generations").map_or_else(
        || {
            Err(error(format!(
                "'{at}' has no generations: how many times the shares change"
            )))
        },
        |g| setting("generations", g),
    )?;
    let baseline = map.get("baseline").map_or_else(
        || {
            Ok(Cell {
                payoff: Payoff::Number(0.0),
                at: format!("{at}.baseline"),
                others: Vec::new(),
            })
        },
        |b| setting("baseline", b),
    )?;
    Ok(Dynamics {
        start,
        generations,
        baseline,
    })
}

fn parse_start(
    at: &str,
    map: &Mapping,
    strategies: &[String],
    setting: impl Fn(&str, &Value) -> ForgeResult<Cell>,
) -> ForgeResult<Vec<Cell>> {
    let list = strategies.join(", ");
    let Some(Value::Mapping(start)) = map.get("start") else {
        return Err(error(format!(
            "'{at}.start' must be a mapping of every strategy ({list}) to its share in the \
             first generation"
        )));
    };
    if let Some(key) = start.keys().find(|k| {
        !k.as_str()
            .is_some_and(|k| strategies.iter().any(|s| s == k))
    }) {
        return Err(error(format!(
            "'{at}.start' has {}, which is not a strategy of the game ({list})",
            describe(key)
        )));
    }
    strategies
        .iter()
        .map(|s| {
            start.get(s.as_str()).map_or_else(
                || {
                    Err(error(format!(
                        "'{at}.start' has no share for {s}: give every strategy ({list}), \
                         0 for one that is not played"
                    )))
                },
                |share| setting(&format!("start.{s}"), share),
            )
        })
        .collect()
}

/// One way the other players' strategies can fall: how many play each, the
/// number of orders it has (m! / Π kⱼ!), and what each strategy pays then.
struct Split {
    counts: Vec<i32>,
    orders: f64,
    pays: Vec<f64>,
}

/// Every way `others` players can fall among `strategies` strategies, as
/// counts in lexicographic order.
fn splits(others: usize, strategies: usize) -> Vec<Vec<usize>> {
    if strategies == 1 {
        return vec![vec![others]];
    }
    let mut all = Vec::new();
    for first in (0..=others).rev() {
        for mut rest in splits(others - first, strategies - 1) {
            rest.insert(0, first);
            all.push(rest);
        }
    }
    all
}

/// m! / Π kⱼ!, the orders of a split of m = Σ kⱼ.
fn orders(counts: &[usize]) -> f64 {
    let mut left = 0;
    counts.iter().fold(1.0, |orders, &k| {
        // C(left + k, k): the places, among those filled so far, of this
        // strategy's k players
        left += k;
        (0..k).fold(orders, |c, i| c * count(left - i) / count(k - i))
    })
}

/// What each strategy pays against the population `shares`, the first
/// player's payoffs of `table` read for each split.
fn payoffs(splits: &[Split], shares: &[f64]) -> Vec<f64> {
    let mut pays = vec![0.0; shares.len()];
    for split in splits {
        let weight = split
            .counts
            .iter()
            .zip(shares)
            .fold(split.orders, |w, (&k, x)| w * x.powi(k));
        for (pay, &own) in pays.iter_mut().zip(&split.pays) {
            *pay = weight.mul_add(own, *pay);
        }
    }
    pays
}

/// The path of the shares of symmetric game `game`, whose `table` is
/// symmetric (the first player's payoffs are read).
///
/// # Errors
///
/// Returns an error naming the strategy, the generation and the baseline
/// when a strategy that is played (its share above 0) pays no more than
/// minus the baseline, which would take its share below 0 or divide by 0.
///
/// # Panics
///
/// Never in practice: a table has at most [`super::model::MOST_CELLS`]
/// cells, so a count fits in an `i32`.
pub fn run(game: &str, table: &Table, settings: &Settings) -> ForgeResult<DynamicsReport> {
    let names = &table.strategies[0];
    let others = table.players.len() - 1;
    let splits: Vec<Split> = splits(others, names.len())
        .into_iter()
        .map(|counts| {
            let mut profile = vec![0];
            for (s, &k) in counts.iter().enumerate() {
                profile.extend(std::iter::repeat_n(s, k));
            }
            let pays = (0..names.len())
                .map(|own| {
                    profile[0] = own;
                    table.payoff(&profile, 0)
                })
                .collect();
            Split {
                orders: orders(&counts),
                counts: counts
                    .iter()
                    .map(|&k| i32::try_from(k).expect("a count of players fits"))
                    .collect(),
                pays,
            }
        })
        .collect();
    let least = table
        .payoffs
        .iter()
        .map(|cell| cell[0])
        .fold(f64::INFINITY, f64::min);
    let b = settings.baseline;
    let generation = |shares: &[f64], pays: &[f64]| Generation {
        shares: names.iter().cloned().zip(shares.iter().copied()).collect(),
        payoff: shares.iter().zip(pays).map(|(x, f)| x * f).sum(),
    };
    let mut shares = settings.start.clone();
    let mut path = Vec::with_capacity(settings.generations + 1);
    for k in 0..=settings.generations {
        let pays = payoffs(&splits, &shares);
        let now = generation(&shares, &pays);
        if k == settings.generations {
            path.push(now);
            break;
        }
        // A share of 0 stays 0 whatever its strategy pays
        let played = names.iter().zip(&pays).zip(&shares);
        if let Some(((s, f), _)) = played
            .into_iter()
            .find(|((_, f), x)| **x > 0.0 && **f + b <= 0.0)
        {
            return Err(ForgeError::Eval(format!(
                "game '{game}': in generation {k} (0 is the start), {s} pays {f} and the \
                 baseline is {b}, so {s}'s payoff plus the baseline is {}, not above 0, and \
                 a share cannot fall below 0; a baseline above {} is enough in every \
                 generation, as no payoff in the table is below {least}",
                f + b,
                0.0 - least
            )));
        }
        // Each share's part of the next generation, over their sum: the
        // average plus the baseline when the shares add up to 1, and kept on
        // the simplex when a rounding moves them off it. Divided by the
        // average plus the baseline instead, such a rounding grows by
        // b / (f̄ + b) each generation, which is above 1 when f̄ < 0.
        let parts: Vec<f64> = shares
            .iter()
            .zip(&pays)
            .map(|(&x, f)| if x > 0.0 { x * (f + b) } else { 0.0 })
            .collect();
        let total: f64 = parts.iter().sum();
        shares = parts.iter().map(|part| part / total).collect();
        path.push(now);
    }
    Ok(DynamicsReport {
        generations: settings.generations,
        baseline: b,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::population::{bernstein, population};
    use crate::games::table::tests::two;

    /// The hawk-dove game with value `v` and cost `c`.
    fn hawk_dove(v: f64, c: f64) -> Table {
        two(
            ["me", "you"],
            &["hawk", "dove"],
            &["hawk", "dove"],
            &[
                [(v - c) / 2.0, (v - c) / 2.0],
                [v, 0.0],
                [0.0, v],
                [v / 2.0, v / 2.0],
            ],
        )
    }

    fn settings(start: &[f64], generations: usize, baseline: f64) -> Settings {
        Settings {
            start: start.to_vec(),
            generations,
            baseline,
        }
    }

    fn share(report: &DynamicsReport, generation: usize, strategy: &str) -> f64 {
        report.path[generation].shares[strategy]
    }

    #[test]
    fn hawk_dove_goes_to_a_hawk_share_of_v_over_c() {
        // By hand, V = 4, C = 6: with hawk share x, hawk pays -x + 4 (1 - x)
        // and dove 2 (1 - x); they are equal at x = V / C = 2/3, where the
        // average payoff is 2 (1 - 2/3) = 2/3
        let report = run("hd", &hawk_dove(4.0, 6.0), &settings(&[0.1, 0.9], 500, 5.0)).unwrap();
        assert_eq!(report.path.len(), 501);
        assert!((share(&report, 500, "hawk") - 2.0 / 3.0).abs() < 1e-12);
        assert!((share(&report, 500, "dove") - 1.0 / 3.0).abs() < 1e-12);
        assert!((report.end().payoff - 2.0 / 3.0).abs() < 1e-12);
        // The first step by hand: hawk pays -0.1 + 3.6 = 3.5, dove 1.8, the
        // average is 0.1 * 3.5 + 0.9 * 1.8 = 1.97, and hawk's share becomes
        // 0.1 * (3.5 + 5) / (1.97 + 5) = 0.85 / 6.97
        assert!((share(&report, 1, "hawk") - 0.85 / 6.97).abs() < 1e-15);
        assert!((report.path[0].payoff - 1.97).abs() < 1e-15);
        assert!((report.path[0].shares["hawk"] - 0.1).abs() < f64::EPSILON);
        // From the other side too, and it stays where it is
        let above = run("hd", &hawk_dove(4.0, 6.0), &settings(&[0.9, 0.1], 500, 5.0)).unwrap();
        assert!((share(&above, 500, "hawk") - 2.0 / 3.0).abs() < 1e-12);
        let rest = run(
            "hd",
            &hawk_dove(4.0, 6.0),
            &settings(&[2.0 / 3.0, 1.0 / 3.0], 3, 5.0),
        )
        .unwrap();
        assert!((share(&rest, 3, "hawk") - 2.0 / 3.0).abs() < 1e-15);
    }

    #[test]
    fn the_stag_hunt_goes_to_the_side_its_start_is_on() {
        // Stag 4 against stag and 0 against hare, hare 3 against both: stag
        // pays more once its share is above 3/4
        let hunt = two(
            ["me", "you"],
            &["stag", "hare"],
            &["stag", "hare"],
            &[[4.0, 4.0], [0.0, 3.0], [3.0, 0.0], [3.0, 3.0]],
        );
        let up = run("sh", &hunt, &settings(&[0.76, 0.24], 2000, 0.0)).unwrap();
        assert!((share(&up, 2000, "stag") - 1.0).abs() < 1e-12);
        assert!((up.end().payoff - 4.0).abs() < 1e-11);
        // Stag dies out: its share falls as its square, and reaches 0
        let down = run("sh", &hunt, &settings(&[0.74, 0.26], 2000, 0.0)).unwrap();
        assert_eq!(share(&down, 2000, "stag").to_bits(), 0);
        assert!((down.end().payoff - 3.0).abs() < 1e-11);
        // The shares go one way all along
        assert!(up
            .path
            .windows(2)
            .all(|w| w[1].shares["stag"] >= w[0].shares["stag"]));
        assert!(down
            .path
            .windows(2)
            .all(|w| w[1].shares["stag"] <= w[0].shares["stag"]));
    }

    #[test]
    fn a_payoff_below_minus_the_baseline_is_refused() {
        // Hawk against hawk pays -1: with baseline 0, a population of
        // hawks only would lose more than all of its share
        let err = run("hd", &hawk_dove(4.0, 6.0), &settings(&[1.0, 0.0], 5, 0.0))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "game 'hd': in generation 0 (0 is the start), hawk pays -1 and the baseline \
                 is 0, so hawk's payoff plus the baseline is -1, not above 0"
            ) && err.contains("a baseline above 1 is enough in every generation, as no payoff in the table is below -1"),
            "{err}"
        );
        // Later: a stag hunt where stag pays -1 against hare, so 5x - 1 at a
        // stag share x. By hand from 0.3: stag pays 0.5, hare 3, so the
        // share becomes 0.15 / 2.25 = 1/15, where stag pays -2/3
        let costly = two(
            ["me", "you"],
            &["stag", "hare"],
            &["stag", "hare"],
            &[[4.0, 4.0], [-1.0, 3.0], [3.0, -1.0], [3.0, 3.0]],
        );
        let err = run("sh", &costly, &settings(&[0.3, 0.7], 50, 0.0))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("game 'sh': in generation 1 (0 is the start), stag pays -0.666666")
                && err.contains("as no payoff in the table is below -1"),
            "{err}"
        );
        // A strategy that is not played is not checked: its share stays 0,
        // a positive 0, though stag pays -1 against a population of hare
        let none = run("sh", &costly, &settings(&[0.0, 1.0], 3, 0.0)).unwrap();
        assert_eq!(none.end().shares["stag"].to_bits(), 0);
        assert!((none.end().payoff - 3.0).abs() < f64::EPSILON);
        // A payoff of exactly minus the baseline is refused too
        let err = run("hd", &hawk_dove(4.0, 6.0), &settings(&[1.0, 0.0], 5, 1.0))
            .unwrap_err()
            .to_string();
        assert!(err.contains("plus the baseline is 0, not above 0"), "{err}");
    }

    /// A symmetric game of `n` players from a payoff for each own strategy
    /// over the counts of the others.
    fn counted(n: usize, names: &[&str], pay: impl Fn(usize, &[usize]) -> f64) -> Table {
        let strategies: Vec<String> = names.iter().map(ToString::to_string).collect();
        let blank = Table {
            players: (0..n).map(|p| format!("p{p}")).collect(),
            strategies: vec![strategies; n],
            payoffs: Vec::new(),
        };
        let payoffs = blank
            .profiles()
            .iter()
            .map(|profile| {
                (0..n)
                    .map(|p| {
                        let mut others = vec![0; names.len()];
                        for (q, &s) in profile.iter().enumerate() {
                            if q != p {
                                others[s] += 1;
                            }
                        }
                        pay(profile[p], &others)
                    })
                    .collect()
            })
            .collect();
        Table { payoffs, ..blank }
    }

    #[test]
    fn the_shares_add_up_to_1_in_every_generation() {
        // validation/games/dynamics.py, seed 1, game d17: three players and
        // an average payoff of -7.75 with baseline 10.5. Dividing by the
        // average plus the baseline, a rounding in the sum of the shares
        // grew by 10.5 / 2.75 each generation, and the share of y passed 1
        // in generation 23 (1.05 in generation 31); egttools' step stays
        // on the simplex. x dies out: y pays more once x is below 0.47.
        let table = counted(3, &["x", "y"], |own, others| {
            let x: [[f64; 3]; 2] = [[-9.5, -2.25, 2.75], [-7.75, -1.5, 7.5]];
            x[own][others[0]]
        });
        let report = run("d17", &table, &settings(&[0.5, 0.5], 1000, 10.5)).unwrap();
        for (k, g) in report.path.iter().enumerate() {
            let total: f64 = g.shares.values().sum();
            assert!((total - 1.0).abs() < 1e-14, "generation {k}: {total}");
            assert!(
                g.shares.values().all(|&x| (0.0..=1.0).contains(&x)),
                "generation {k}"
            );
        }
        assert!((share(&report, 1000, "y") - 1.0).abs() < 1e-15);
        assert!((report.end().payoff + 7.75).abs() < 1e-12);
    }

    #[test]
    fn two_strategies_average_as_the_population_function_does() {
        // f_A is the Bernstein sum of population.rs, over every split of
        // four others; the game is volunteer's dilemma b = 10, c = 4 (a
        // volunteer gets b - c, a bystander b if anyone volunteers)
        let n = 5;
        let table = crate::games::table::Table {
            players: (0..n).map(|p| format!("p{p}")).collect(),
            strategies: vec![vec!["volunteer".to_string(), "wait".to_string()]; n],
            payoffs: crate::games::table::Table {
                players: Vec::new(),
                strategies: vec![vec![String::new(); 2]; n],
                payoffs: Vec::new(),
            }
            .profiles()
            .iter()
            .map(|profile| {
                let anyone = |p: usize| profile.iter().enumerate().any(|(q, &s)| q != p && s == 0);
                (0..n)
                    .map(|p| match (profile[p], anyone(p)) {
                        (0, _) => 6.0,
                        (_, true) => 10.0,
                        _ => 0.0,
                    })
                    .collect()
            })
            .collect(),
        };
        let people = population(&table).unwrap();
        let mut shares = [0.3, 0.7];
        for _ in 0..20 {
            let report = run("vd", &table, &settings(&shares, 1, 2.0)).unwrap();
            let x = shares[0];
            let first = people.payoff(x);
            let second = first - bernstein(&people.advantage, x);
            let average = x.mul_add(first, (1.0 - x) * second);
            assert!((report.path[0].payoff - average).abs() < 1e-12);
            let next = x * (first + 2.0) / (average + 2.0);
            assert!((share(&report, 1, "volunteer") - next).abs() < 1e-14);
            shares = [next, 1.0 - next];
        }
        // The rest point is the mix 1 - (c / b)^(1 / (n - 1)) = 0.2047...
        let long = run("vd", &table, &settings(&[0.9, 0.1], 400, 0.0)).unwrap();
        assert!((share(&long, 400, "volunteer") - 0.204_729_271_232_949).abs() < 1e-12);
    }

    #[test]
    fn three_strategies_and_three_players_average_over_every_split() {
        // Rock, paper, scissors for three players, each scored against both
        // others: win 1, lose -1. With shares x, rock pays 2 (xₛ - x_p) by
        // linearity, so with baseline 3 one step from (0.5, 0.3, 0.2) by
        // hand: rock 2 (0.2 - 0.3) = -0.2, paper 2 (0.5 - 0.2) = 0.6,
        // scissors 2 (0.3 - 0.5) = -0.4
        let beats = |a: usize, b: usize| match (a + 3 - b) % 3 {
            0 => 0.0,
            1 => 1.0,
            _ => -1.0,
        };
        let shapes = ["rock", "paper", "scissors"];
        let blank = crate::games::table::Table {
            players: Vec::new(),
            strategies: vec![shapes.iter().map(ToString::to_string).collect(); 3],
            payoffs: Vec::new(),
        };
        let table = crate::games::table::Table {
            players: vec!["a".into(), "b".into(), "c".into()],
            payoffs: blank
                .profiles()
                .iter()
                .map(|p| {
                    (0..3)
                        .map(|i| (0..3).filter(|&j| j != i).map(|j| beats(p[i], p[j])).sum())
                        .collect()
                })
                .collect(),
            ..blank
        };
        let report = run("rps", &table, &settings(&[0.5, 0.3, 0.2], 1, 3.0)).unwrap();
        let average = 0.5f64.mul_add(-0.2, 0.3f64.mul_add(0.6, 0.2 * -0.4));
        assert!((report.path[0].payoff - average).abs() < 1e-15);
        for (k, (x, f)) in [(0.5, -0.2), (0.3, 0.6), (0.2, -0.4)]
            .into_iter()
            .enumerate()
        {
            let next = x * (f + 3.0) / (average + 3.0);
            let shape = shapes[k];
            assert!((share(&report, 1, shape) - next).abs() < 1e-15, "{shape}");
        }
        // The shares always add up to 1
        let long = run("rps", &table, &settings(&[0.5, 0.3, 0.2], 300, 3.0)).unwrap();
        for g in &long.path {
            assert!((g.shares.values().sum::<f64>() - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn splits_and_their_orders() {
        assert_eq!(splits(2, 2), [vec![2, 0], vec![1, 1], vec![0, 2]]);
        assert_eq!(
            splits(2, 3),
            [
                vec![2, 0, 0],
                vec![1, 1, 0],
                vec![1, 0, 1],
                vec![0, 2, 0],
                vec![0, 1, 1],
                vec![0, 0, 2]
            ]
        );
        assert_eq!(splits(0, 3), [vec![0, 0, 0]]);
        // 4! / (2! 1! 1!) = 12, 12! / (12!) = 1, C(12, 5) = 792
        assert!((orders(&[2, 1, 1]) - 12.0).abs() < f64::EPSILON);
        assert!((orders(&[12, 0]) - 1.0).abs() < f64::EPSILON);
        assert!((orders(&[5, 7]) - 792.0).abs() < f64::EPSILON);
    }

    fn cell(payoff: Payoff, at: &str) -> Cell {
        Cell {
            payoff,
            at: at.to_string(),
            others: Vec::new(),
        }
    }

    fn written(start: [Payoff; 2], generations: Payoff, baseline: Payoff) -> Dynamics {
        let [a, b] = start;
        Dynamics {
            start: vec![
                cell(a, "games.hd.dynamics.start.hawk"),
                cell(b, "games.hd.dynamics.start.dove"),
            ],
            generations: cell(generations, "games.hd.dynamics.generations"),
            baseline: cell(baseline, "games.hd.dynamics.baseline"),
        }
    }

    #[test]
    fn settings_are_worked_out_and_checked() {
        use Payoff::{Formula, Number};
        let f = |s: &str| Formula(s.to_string());
        let d = written([f("=x"), f("=1 - x")], f("=g"), f("=x"));
        assert_eq!(d.formulas(), ["=x", "=1 - x", "=g"]);
        let values = |formula: &str, _: &str| match formula {
            "=x" => Ok(0.25),
            "=1 - x" => Ok(0.75),
            "=g" => Ok(40.0),
            _ => Err(ForgeError::Eval("nope".to_string())),
        };
        let got = d.work_out("hd", values).unwrap();
        assert_eq!(got, settings(&[0.25, 0.75], 40, 0.25));
        // A formula that fails gives its error
        let nope = ForgeError::Eval("nope".to_string()).to_string();
        for bad in [
            written([f("=y"), Number(0.5)], Number(1.0), Number(0.0)),
            written([Number(0.5), Number(0.5)], f("=y"), Number(0.0)),
            written([Number(0.5), Number(0.5)], Number(1.0), f("=y")),
        ] {
            assert_eq!(bad.work_out("hd", values).unwrap_err().to_string(), nope);
        }
        let check = |d: Dynamics| d.work_out("hd", values).unwrap_err().to_string();
        let numbers = |a: f64, b: f64, g: f64, base: f64| {
            written([Number(a), Number(b)], Number(g), Number(base))
        };
        assert!(check(numbers(1.5, -0.5, 1.0, 0.0))
            .contains("game 'hd': games.hd.dynamics.start.hawk is 1.5; a share is from 0 to 1"));
        assert!(check(numbers(0.5, 0.4, 1.0, 0.0)).contains(
            "game 'hd': the shares of games.hd.dynamics.start add up to 0.9; the shares of \
             every strategy add up to 1"
        ));
        for g in [0.0, 2.5, 100_001.0] {
            assert!(
                check(numbers(0.5, 0.5, g, 0.0)).contains(&format!(
                    "games.hd.dynamics.generations is {g}; it is a whole number of generations, \
                     from 1 to 100000"
                )),
                "{g}"
            );
        }
        assert!(check(numbers(0.5, 0.5, 1.0, f64::NAN))
            .contains("games.hd.dynamics.baseline is NaN; it is added to every payoff"));
        // Shares a rounding away from 1 are scaled to add up to 1
        let near = numbers(0.5, 0.5 + 1e-12, 100_000.0, 0.0)
            .work_out("hd", values)
            .unwrap();
        assert!((near.start.iter().sum::<f64>() - 1.0).abs() < 1e-15);
        assert_eq!(near.generations, 100_000);
    }

    fn parsed(yaml: &str, symmetric: bool) -> ForgeResult<Dynamics> {
        let value: Value = yaml_serde::from_str(yaml).unwrap();
        parse("hd", &value, symmetric, &["hawk".into(), "dove".into()])
    }

    #[test]
    fn the_key_is_read_with_its_place() {
        let d = parsed("{start: {dove: 0.9, hawk: =x}, generations: 50}", true).unwrap();
        assert_eq!(
            d,
            Dynamics {
                start: vec![
                    cell(
                        Payoff::Formula("=x".to_string()),
                        "games.hd.dynamics.start.hawk"
                    ),
                    cell(Payoff::Number(0.9), "games.hd.dynamics.start.dove"),
                ],
                generations: cell(Payoff::Number(50.0), "games.hd.dynamics.generations"),
                baseline: cell(Payoff::Number(0.0), "games.hd.dynamics.baseline"),
            }
        );
        let with = parsed(
            "{start: {dove: 1, hawk: 0}, generations: 5, baseline: =b}",
            true,
        )
        .unwrap();
        assert_eq!(with.baseline.payoff, Payoff::Formula("=b".to_string()));
    }

    #[test]
    fn what_cannot_be_read_is_named() {
        let err = |yaml: &str, symmetric: bool| parsed(yaml, symmetric).unwrap_err().to_string();
        let start = "{start: {hawk: 0.5, dove: 0.5}, generations: 5}";
        assert!(err(start, false).contains(
            "'games.hd.dynamics': the dynamics follow one population whose members all play \
             the same game, so the game must be marked symmetric: true, and games.hd is not"
        ));
        assert!(err("[1]", true).contains(
            "'games.hd.dynamics' must be a mapping with start, generations, and baseline if wanted"
        ));
        assert!(err(
            "{start: {hawk: 1, dove: 0}, generations: 5, steps: 3}",
            true
        )
        .contains(
            "'games.hd.dynamics' has \"steps\", which the dynamics do not have: they have \
             start, generations, baseline"
        ));
        assert!(err("{generations: 5}", true).contains(
            "'games.hd.dynamics.start' must be a mapping of every strategy (hawk, dove) to its \
             share in the first generation"
        ));
        assert!(err("{start: {hawk: 1, dove: 0, owl: 0}, generations: 5}", true).contains(
            "'games.hd.dynamics.start' has \"owl\", which is not a strategy of the game (hawk, dove)"
        ));
        assert!(err("{start: {hawk: 1}, generations: 5}", true).contains(
            "'games.hd.dynamics.start' has no share for dove: give every strategy (hawk, dove), \
             0 for one that is not played"
        ));
        assert!(err("{start: {hawk: 1, dove: high}, generations: 5}", true).contains(
            "'games.hd.dynamics.start.dove' is \"high\": it is a number or a formula starting with ="
        ));
        assert!(err("{start: {hawk: 1, dove: 0}}", true)
            .contains("'games.hd.dynamics' has no generations: how many times the shares change"));
        assert!(err("{start: {hawk: 1, dove: 0}, generations: [5]}", true)
            .contains("'games.hd.dynamics.generations' is a list"));
        assert!(err(
            "{start: {hawk: 1, dove: 0}, generations: 5, baseline: true}",
            true
        )
        .contains("'games.hd.dynamics.baseline' is true"));
    }
}
