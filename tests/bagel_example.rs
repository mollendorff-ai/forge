//! The numbers in examples/bagel/README.md, as computed outside Forge.
//!
//! The Python that produced them (independent of Forge's evaluator):
//! ```python
//! season = [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00]
//! w = 18.5 * 1.15
//! def profit(flour=1.15, price=16.0):
//!     total = 0
//!     for m, se in enumerate(season, 1):
//!         dz = 55 * 1.03 ** (m - 1) * se * 30
//!         r = dz * price
//!         c = dz * 12 / 9 * flour + dz * 12 * 6 / 1000 * 9.40 + dz * 0.35 + 32 * 30 * w + 5750 + r * 0.026
//!         total += r - c
//!     return total
//! profit()            # 786.3203617
//! profit(flour=1.70)  # -16943.271075
//! ```

use mollendorff_forge::cli::{calculate_core, goal_seek_core, scenarios_core};
use std::path::Path;

const MODEL: &str = "examples/bagel/bagel.yaml";

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn year_profit_matches_the_independent_calculation() {
    let result = calculate_core(Path::new(MODEL), true, None).unwrap();
    let profit = result.scalars["year.profit"].unwrap();
    assert!(close(profit, 786.320_361_7, 1e-6), "{profit}");
    assert!(close(
        result.scalars["year.revenue"].unwrap(),
        386_827.449_537_8,
        1e-6
    ));
}

#[test]
fn flour_shock_changes_the_included_price() {
    let results = scenarios_core(Path::new(MODEL), Some("flour_shock")).unwrap();
    let shock = &results.scenarios[0];
    assert_eq!(shock.name, "flour_shock");
    assert!(close(shock.outputs["year.profit"], -16_943.271_075, 1e-6));
    // The expected value spans all four scenarios: 0.5*786.32 + 0.1*(-16943.27) + 0.2*(-71197.20) + 0.2*80811.24
    assert!(close(
        results.expected_values["year.profit"],
        621.641_286,
        1e-5
    ));
}

#[test]
fn break_even_price_and_staffing_for_sixty_thousand() {
    let model = Path::new(MODEL);
    let price = goal_seek_core(
        model,
        "year.profit",
        0.0,
        "assumptions.price_per_dozen",
        (Some(10.0), Some(30.0)),
        0.0001,
    )
    .unwrap();
    assert!(price.converged);
    assert!(close(price.solution, 15.966_6, 1e-3), "{}", price.solution);

    let hours = goal_seek_core(
        model,
        "year.profit",
        60_000.0,
        "assumptions.staff_hours_per_day",
        (Some(8.0), Some(40.0)),
        0.0001,
    )
    .unwrap();
    assert!(hours.converged);
    assert!(close(hours.solution, 24.268_7, 1e-3), "{}", hours.solution);
}
