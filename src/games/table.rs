//! A game played once, with numbers in its cells: who plays, what each can
//! play, and what each gets in every cell (ADR-068).

/// Payoffs closer than this, relative to the largest payoff in the table (or
/// 1 when they are all smaller), are a tie.
pub const TIE: f64 = 1e-9;

/// A payoff table with a number for every player in every cell.
///
/// A cell is a profile: one strategy for each player, by index. The cells are
/// stored with the first player's strategy outermost, as the model writes them.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub players: Vec<String>,
    /// Each player's strategies, in the order of `players`
    pub strategies: Vec<Vec<String>>,
    /// For each cell, in profile order, one payoff per player
    pub payoffs: Vec<Vec<f64>>,
}

impl Table {
    /// How many cells the table has: the product of the numbers of strategies.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.strategies.iter().map(Vec::len).product()
    }

    /// The position of a profile in `payoffs`.
    #[must_use]
    pub fn index(&self, profile: &[usize]) -> usize {
        profile
            .iter()
            .zip(&self.strategies)
            .fold(0, |at, (&s, list)| at * list.len() + s)
    }

    /// What `player` gets in the cell of `profile`.
    #[must_use]
    pub fn payoff(&self, profile: &[usize], player: usize) -> f64 {
        self.payoffs[self.index(profile)][player]
    }

    /// Every profile, in table order.
    #[must_use]
    pub fn profiles(&self) -> Vec<Vec<usize>> {
        let sizes: Vec<usize> = self.strategies.iter().map(Vec::len).collect();
        let mut all = Vec::with_capacity(self.cell_count());
        let mut profile = vec![0; sizes.len()];
        loop {
            all.push(profile.clone());
            // The last player's strategy moves fastest
            let mut player = sizes.len();
            loop {
                if player == 0 {
                    return all;
                }
                player -= 1;
                profile[player] += 1;
                if profile[player] < sizes[player] {
                    break;
                }
                profile[player] = 0;
            }
        }
    }

    /// The largest payoff in absolute value, or 1 when it is smaller: the
    /// scale against which two payoffs are a tie.
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.payoffs
            .iter()
            .flatten()
            .fold(1.0_f64, |most, p| most.max(p.abs()))
    }

    /// The tolerance for a tie in this table.
    #[must_use]
    pub fn tie(&self) -> f64 {
        TIE * self.scale()
    }

    /// A profile written with the strategies' names: `stag, hare`.
    #[must_use]
    pub fn profile_name(&self, profile: &[usize]) -> String {
        profile
            .iter()
            .zip(&self.strategies)
            .map(|(&s, list)| list[s].as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A table from rows of `[a, b]` payoffs, two players.
    pub fn two(players: [&str; 2], rows: &[&str], columns: &[&str], cells: &[[f64; 2]]) -> Table {
        assert_eq!(cells.len(), rows.len() * columns.len());
        Table {
            players: players.iter().map(|p| (*p).to_string()).collect(),
            strategies: vec![
                rows.iter().map(|s| (*s).to_string()).collect(),
                columns.iter().map(|s| (*s).to_string()).collect(),
            ],
            payoffs: cells.iter().map(|c| c.to_vec()).collect(),
        }
    }

    #[test]
    fn profiles_run_with_the_last_player_fastest() {
        let table = Table {
            players: vec!["a".into(), "b".into(), "c".into()],
            strategies: vec![
                vec!["x".into(), "y".into()],
                vec!["p".into()],
                vec!["l".into(), "m".into(), "n".into()],
            ],
            payoffs: vec![vec![0.0; 3]; 6],
        };
        let profiles = table.profiles();
        assert_eq!(
            profiles,
            vec![
                vec![0, 0, 0],
                vec![0, 0, 1],
                vec![0, 0, 2],
                vec![1, 0, 0],
                vec![1, 0, 1],
                vec![1, 0, 2],
            ]
        );
        for (at, profile) in profiles.iter().enumerate() {
            assert_eq!(table.index(profile), at);
        }
        assert_eq!(table.cell_count(), 6);
        assert_eq!(table.profile_name(&[1, 0, 2]), "y, p, n");
    }

    #[test]
    fn a_tie_is_relative_to_the_largest_payoff() {
        let small = two(["a", "b"], &["x"], &["y"], &[[0.5, -0.25]]);
        assert!((small.scale() - 1.0).abs() < f64::EPSILON);
        assert!((small.tie() - TIE).abs() < f64::EPSILON);
        let large = two(["a", "b"], &["x"], &["y"], &[[-2000.0, 10.0]]);
        assert!((large.scale() - 2000.0).abs() < f64::EPSILON);
        assert!((large.payoff(&[0, 0], 1) - 10.0).abs() < f64::EPSILON);
    }
}
