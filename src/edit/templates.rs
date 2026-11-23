//! The blocks `insert` adds to a model (ADR-075 step 6; ADR-076, decision 6).
//!
//! Each template is a game, written as the examples write one, with its
//! numbers to change and a comment on what each part is. Every number is a
//! literal and every name is the template's own, so a template is valid in
//! any model of format 5.5.0 or later, whatever the model holds.
//! `{name}` is the name the game gets.

use serde::Serialize;

/// Where a block says the name of its game
const PLACEHOLDER: &str = "{name}";

/// A block of a model that can be inserted
pub struct Template {
    /// What `insert` names it by
    pub name: &'static str,
    /// What it is, in one line
    pub line: &'static str,
    /// The name its game gets when none is asked for
    pub default: &'static str,
    /// The entries of `games:`, indented as the file's first level, with
    /// `{name}` for the game's name
    pub yaml: &'static str,
}

/// The templates, in the order Forge Studio offers them
pub const TEMPLATES: [Template; 4] = [
    Template {
        name: "game_of_two",
        line: "A game of two players, played once",
        default: "new_game",
        yaml: "  {name}:
    description: Two players choose at once; each cell is [row's payoff, column's payoff]
    players: [row, column]
    strategies: [cooperate, defect]
    payoffs:
      cooperate: {cooperate: [3, 3], defect: [0, 5]}
      defect: {cooperate: [5, 0], defect: [1, 1]}
",
    },
    Template {
        name: "game_of_many",
        line: "A game of many players alike (the count form): each gets a payoff from its own strategy and how many of the others play each",
        default: "new_crowd",
        yaml: "  {name}:
    description: Five players alike each contribute 10 to a pot that pays back 1.6 times its size, shared equally, or keep it
    players: [a1, a2, a3, a4, a5]
    strategies: [contribute, keep]
    symmetric: true
    payoffs:                        # one player's payoff, for its own strategy
      contribute: =1.6 * 10 * ({name}.others.contribute + 1) / 5
      keep: =10 + 1.6 * 10 * {name}.others.contribute / 5
",
    },
    Template {
        name: "tournament",
        line: "A game played again and again by players who follow habits (a tournament)",
        default: "new_tournament",
        yaml: "  {name}_round:
    description: The prisoner's dilemma, one round
    players: [me, you]
    strategies: [cooperate, defect]   # a tournament reads the first as cooperate
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}

  {name}:
    description: The round, played again and again
    tournament:
      round: {name}_round
      players:
        nice: tit_for_tat             # starts by cooperating, then copies the other
        mean: always_defect
        trusting: always_cooperate
      rounds: 100
      self_play: true
",
    },
    Template {
        name: "dynamics",
        line: "A game whose strategies spread through a population, generation by generation",
        default: "new_population",
        yaml: "  {name}:
    description: A population of hunters, each meeting strangers
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true                 # a cell is the payoff of the row's player
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: 3, hare: 3}
    dynamics:
      start: {stag: 0.8, hare: 0.2}   # the share of each in the first generation
      generations: 100
",
    },
];

/// The template called `name`
#[must_use]
pub fn find(name: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.name == name)
}

impl Template {
    /// The block with the game called `name`
    #[must_use]
    pub fn block(&self, name: &str) -> String {
        self.yaml.replace(PLACEHOLDER, name)
    }

    /// The names the block gives its games: the game itself, and for a
    /// tournament the round it plays
    #[must_use]
    pub fn names(&self, name: &str) -> Vec<String> {
        self.block(name)
            .lines()
            .filter(|l| l.starts_with("  ") && !l.starts_with("   ") && l.ends_with(':'))
            .map(|l| l.trim().trim_end_matches(':').to_string())
            .collect()
    }
}

/// A template as the `templates` tool answers it
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Described {
    /// What `insert` names it by
    pub name: String,
    /// What it is, in one line
    pub line: String,
    /// The block, with the game called `default`
    pub yaml: String,
    /// The game's name when none is asked for
    pub default: String,
}

/// Every template, each with the block it inserts
#[must_use]
pub fn templates() -> Vec<Described> {
    TEMPLATES
        .iter()
        .map(|t| Described {
            name: t.name.into(),
            line: t.line.into(),
            yaml: t.block(t.default),
            default: t.default.into(),
        })
        .collect()
}
