//! The examples `forge examples` lists and runs, embedded at compile time.

/// Embedded example YAML files (compile-time)
const EXAMPLE_STARTER: &str = include_str!("../../../../examples/starter.yaml");
const EXAMPLE_MONTE_CARLO: &str = include_str!("../../../../examples/monte-carlo.yaml");
const EXAMPLE_SCENARIOS: &str = include_str!("../../../../examples/scenarios.yaml");
const EXAMPLE_DECISION_TREE: &str = include_str!("../../../../examples/decision-tree.yaml");
const EXAMPLE_REAL_OPTIONS: &str = include_str!("../../../../examples/real-options.yaml");
const EXAMPLE_TORNADO: &str = include_str!("../../../../examples/tornado.yaml");
const EXAMPLE_BOOTSTRAP: &str = include_str!("../../../../examples/bootstrap.yaml");
const EXAMPLE_BAYESIAN: &str = include_str!("../../../../examples/bayesian.yaml");
const EXAMPLE_VARIANCE: &str = include_str!("../../../../examples/variance.yaml");
const EXAMPLE_BREAKEVEN: &str = include_str!("../../../../examples/breakeven.yaml");

// Games (ADR-068), each run by `forge game`
const EXAMPLE_STAG_HUNT: &str = include_str!("../../../../examples/games/stag-hunt.yaml");
const EXAMPLE_CHICKEN: &str = include_str!("../../../../examples/games/chicken.yaml");
const EXAMPLE_AXELROD: &str = include_str!("../../../../examples/games/axelrod.yaml");
const EXAMPLE_INSPECTION: &str = include_str!("../../../../examples/games/inspection.yaml");
const EXAMPLE_NOISY_TOURNAMENT: &str =
    include_str!("../../../../examples/games/noisy-tournament.yaml");
const EXAMPLE_BATTLE_OF_THE_SEXES: &str =
    include_str!("../../../../examples/games/battle-of-the-sexes.yaml");
const EXAMPLE_CONTINUATION: &str = include_str!("../../../../examples/games/continuation.yaml");

/// Example metadata
pub(super) struct Example {
    pub(super) name: &'static str,
    pub(super) description: &'static str,
    pub(super) command: &'static str,
    pub(super) content: &'static str,
}

/// All available examples
pub(super) const EXAMPLES: &[Example] = &[
    Example {
        name: "starter",
        description: "A first model: a group of scalars and a table",
        command: "forge calculate",
        content: EXAMPLE_STARTER,
    },
    Example {
        name: "monte-carlo",
        description: "Probabilistic simulation with distributions",
        command: "forge simulate",
        content: EXAMPLE_MONTE_CARLO,
    },
    Example {
        name: "scenarios",
        description: "Probability-weighted scenario analysis",
        command: "forge scenarios",
        content: EXAMPLE_SCENARIOS,
    },
    Example {
        name: "decision-tree",
        description: "Sequential decisions with backward induction",
        command: "forge decision-tree",
        content: EXAMPLE_DECISION_TREE,
    },
    Example {
        name: "real-options",
        description: "Option pricing for managerial flexibility",
        command: "forge real-options",
        content: EXAMPLE_REAL_OPTIONS,
    },
    Example {
        name: "tornado",
        description: "One-at-a-time sensitivity analysis",
        command: "forge tornado",
        content: EXAMPLE_TORNADO,
    },
    Example {
        name: "bootstrap",
        description: "Non-parametric confidence intervals",
        command: "forge bootstrap",
        content: EXAMPLE_BOOTSTRAP,
    },
    Example {
        name: "bayesian",
        description: "Probabilistic graphical models",
        command: "forge bayesian",
        content: EXAMPLE_BAYESIAN,
    },
    Example {
        name: "variance",
        description: "Budget vs actual analysis",
        command: "forge calculate",
        content: EXAMPLE_VARIANCE,
    },
    Example {
        name: "breakeven",
        description: "Break-even calculations",
        command: "forge calculate",
        content: EXAMPLE_BREAKEVEN,
    },
    Example {
        name: "stag-hunt",
        description: "kinship-protocol's stag hunt: two pure equilibria and a mixed one",
        command: "forge game",
        content: EXAMPLE_STAG_HUNT,
    },
    Example {
        name: "chicken",
        description: "Chicken: who swerves, and the cost at which swerving wins",
        command: "forge game",
        content: EXAMPLE_CHICKEN,
    },
    Example {
        name: "axelrod",
        description: "Axelrod's tournament of five strategies, played exactly",
        command: "forge game",
        content: EXAMPLE_AXELROD,
    },
    Example {
        name: "inspection",
        description: "An AI cheats or complies, a human audits: a penalty's effect",
        command: "forge game",
        content: EXAMPLE_INSPECTION,
    },
    Example {
        name: "noisy-tournament",
        description: "A tournament with noise, where a forgiving strategy wins",
        command: "forge game",
        content: EXAMPLE_NOISY_TOURNAMENT,
    },
    Example {
        name: "battle-of-the-sexes",
        description: "Two pure equilibria and a mixed one, for two tastes",
        command: "forge game",
        content: EXAMPLE_BATTLE_OF_THE_SEXES,
    },
    Example {
        name: "continuation",
        description: "The continuation at which cooperation holds",
        command: "forge game",
        content: EXAMPLE_CONTINUATION,
    },
];
