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

// ─────────────────────────────────────────────────────────────────────────────
// The rest of examples/bagel: one model per analytics command (roadmap T1-USE).
// Each expected value was computed by Python written independently of Forge;
// the computation is shown with the test.
// ─────────────────────────────────────────────────────────────────────────────

use mollendorff_forge::cli::{
    bayesian_core, bootstrap_core, compare_core, decision_tree_core, real_options_core,
    variance_core,
};

/// Plan vs what happened. Rent and staff hours went up, so they are
/// unfavorable; the NPV got worse, so its percentage is negative even though
/// the budget NPV is negative; price has no direction, so it is not judged.
#[test]
fn variance_judges_costs_as_costs() {
    let v = variance_core(
        Path::new(MODEL),
        Path::new("examples/bagel/actual.yaml"),
        10.0,
    )
    .unwrap();
    let get = |n: &str| v.results.iter().find(|r| r.name == n).unwrap();
    assert_eq!(get("assumptions.rent_per_month").is_favorable, Some(false));
    assert_eq!(get("assumptions.staff_hours_per_day").better, "lower");
    assert_eq!(
        get("assumptions.staff_hours_per_day").is_favorable,
        Some(false)
    );
    assert_eq!(get("assumptions.price_per_dozen").is_favorable, None);
    let npv = get("year.npv_first_year");
    // -123258.79 vs -95579.52: 27679.27 worse, 28.96% of the size of the budget
    assert!(
        close(npv.variance_pct, -28.959_419, 1e-5),
        "{}",
        npv.variance_pct
    );
    assert_eq!(npv.is_favorable, Some(false));
    assert!(v.not_compared.is_empty(), "{:?}", v.not_compared);
}

/// ```python
/// open_now = 0.45 * 420_000 + 0.55 * 90_000 - 180_000             # 58,500
/// busy  = max(0.8 * 400_000 + 0.2 * 85_000 - 190_000, 0)          # 147,000
/// quiet = max(0.1 * 400_000 + 0.9 * 85_000 - 190_000, 0)          # 0
/// popup = 0.5 * busy + 0.5 * quiet - 12_000                       # 61,500
/// ```
#[test]
fn decision_tree_prefers_the_popup() {
    let r = decision_tree_core(Path::new("examples/bagel/second_shop.yaml")).unwrap();
    assert!(close(r.root_expected_value, 61_500.0, 1e-9));
    let root = &r.node_results["root"];
    assert_eq!(root.optimal_choice.as_deref(), Some("popup_first"));
    assert!(root.tied_with.is_empty(), "{:?}", root.tied_with);
    assert!(close(root.branch_values["open_now"], 58_500.0, 1e-9));
    assert!(close(r.risk_profile.probability_positive, 0.4, 1e-12));
    assert!(close(r.risk_profile.worst_case, -117_000.0, 1e-9));
}

/// Cox-Ross-Rubinstein, 200 steps, American, written from the textbook:
/// S=240000, sigma=0.35, r=0.04, q=0.06, T=1.
/// Defer (call, K=180000) = 64604.68351396034; abandon (put, K=110000) = 279.7243076076951.
#[test]
fn real_options_match_an_independent_binomial_tree() {
    let r = real_options_core(Path::new("examples/bagel/wait_option.yaml")).unwrap();
    let defer = r.options["Wait up to a year"].value;
    let abandon = r.options["Sell the oven and fit-out if it fails"].value;
    assert!(close(defer, 64_604.683_513_960_34, 1e-6), "{defer}");
    assert!(close(abandon, 279.724_307_607_695_1, 1e-6), "{abandon}");
}

/// numpy, 200,000 resamples of the same 30 days: mean 50.7, standard error
/// 2.0162, 95% interval [46.8, 54.667]. Forge's own resampler (seed 3) must
/// agree within resampling noise.
#[test]
fn bootstrap_agrees_with_numpy() {
    let r = bootstrap_core(
        Path::new("examples/bagel/first_month.yaml"),
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(r.original_estimate, 50.7, 1e-12));
    assert!(
        close(r.bootstrap_std_error, 2.0162, 0.03),
        "{}",
        r.bootstrap_std_error
    );
    let ci95 = r
        .confidence_intervals
        .iter()
        .find(|c| close(c.level, 0.95, 1e-12))
        .unwrap();
    assert!(close(ci95.lower, 46.8, 0.15) && close(ci95.upper, 54.667, 0.15));
}

/// Enumerating the joint distribution of weather x roadwork x sales:
/// P(roadwork | slow) = 0.4; P(roadwork | slow, rainy) = 0.272727 (rain
/// explains the slow week, so roadwork becomes less likely).
#[test]
fn bayesian_two_parents_explain_away() {
    let file = Path::new("examples/bagel/slow_week.yaml");
    let p_yes = |evidence: &[&str]| {
        let ev: Vec<String> = evidence.iter().map(ToString::to_string).collect();
        let r = bayesian_core(file, Some("roadwork"), &ev).unwrap();
        let q = &r.queries["roadwork"];
        q.probabilities[q.states.iter().position(|s| s == "yes").unwrap()]
    };
    assert!(close(p_yes(&["sales=slow"]), 0.4, 1e-9));
    assert!(close(
        p_yes(&["sales=slow", "weather=rainy"]),
        3.0 / 11.0,
        1e-9
    ));
}

/// The same input gives the same output, in the same order (STABILITY.md 4).
#[test]
fn compare_and_real_options_output_is_the_same_every_time() {
    let scenarios = vec!["tourist_summer".to_string(), "base".to_string()];
    let a = serde_json::to_string(&compare_core(Path::new(MODEL), &scenarios).unwrap()).unwrap();
    for _ in 0..5 {
        let b =
            serde_json::to_string(&compare_core(Path::new(MODEL), &scenarios).unwrap()).unwrap();
        assert_eq!(a, b);
    }
    let file = Path::new("examples/bagel/wait_option.yaml");
    let a = serde_json::to_string(&real_options_core(file).unwrap()).unwrap();
    for _ in 0..5 {
        assert_eq!(
            a,
            serde_json::to_string(&real_options_core(file).unwrap()).unwrap()
        );
    }
}

/// `forge simulate` on the bagel model with uncertain demand, staffing, flour
/// price and wage (the last two in the included `costs_risk.yaml`).
///
/// Checked outside Forge: the model typed again below (and in numpy) gives
/// Forge's outputs from Forge's own input samples to 1e-9, and on those
/// samples numpy gives the threshold probabilities and
/// `scipy.stats.spearmanr` the rank correlations asserted here (1,000
/// iterations, seed 7).
#[test]
// The model is typed as in the Python check, row by row, so it reads the same
#[allow(clippy::needless_range_loop, clippy::suboptimal_flops)]
fn simulation_drivers_and_thresholds_match_scipy() {
    use mollendorff_forge::cli::{simulate_core, SimulateOptions};
    let options = SimulateOptions {
        iterations: Some(1000),
        ..SimulateOptions::default()
    };
    let result = simulate_core(Path::new("examples/bagel/bagel_simulate.yaml"), options).unwrap();
    let inputs = &result.input_samples;
    let profit = &result.outputs["year.profit"];

    // The model again, from bagel_simulate.yaml and costs.yaml
    let season = [
        0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00,
    ];
    for i in 0..1000 {
        let flour = inputs["@costs.ingredients.flour_per_kg"][i];
        let wage = inputs["@costs.labour.wage_per_hour"][i];
        let opening = inputs["assumptions.opening_dozens_per_day"][i];
        let staff = inputs["assumptions.staff_hours_per_day"][i];
        let mut total = 0.0;
        for (m, se) in season.iter().enumerate() {
            let dz = opening * 1.03_f64.powi(i32::try_from(m).unwrap()) * se * 30.0;
            let r = dz * 16.0;
            let c = dz * 12.0 / 9.0 * flour
                + dz * 12.0 * 6.0 / 1000.0 * 9.40
                + dz * 0.35
                + staff * 30.0 * wage * 1.15
                + 4800.0
                + 950.0
                + r * 0.026;
            total += r - c;
        }
        assert!(close(total, profit.samples[i], 1e-6), "iteration {i}");
    }

    let thresholds: Vec<(&str, f64)> = profit
        .threshold_probabilities
        .iter()
        .map(|(t, p)| (t.as_str(), *p))
        .collect();
    assert_eq!(
        thresholds,
        [("> 0", 0.349), ("> 20000", 0.196), ("< -50000", 0.205)]
    );
    let drivers: Vec<(&str, f64)> = profit
        .drivers
        .iter()
        .map(|d| (d.input.as_str(), d.rank_correlation))
        .collect();
    for ((name, rho), (want_name, want)) in drivers.iter().zip([
        ("assumptions.opening_dozens_per_day", 0.845_145_641_146),
        ("assumptions.staff_hours_per_day", -0.417_853_641_854),
        ("@costs.labour.wage_per_hour", -0.322_749_874_750),
        ("@costs.ingredients.flour_per_kg", -0.098_687_702_688),
    ]) {
        assert_eq!(*name, want_name);
        assert!(close(*rho, want, 1e-11), "{name}: {rho}");
    }
    assert_eq!(drivers.len(), 4);
    // Outputs in the order configured
    let names: Vec<&String> = result.outputs.keys().collect();
    assert_eq!(names, ["year.profit", "year.worst_month_profit"]);

    // The rent hike moves every sample by exactly 12 months of 800
    let hiked = simulate_core(
        Path::new("examples/bagel/bagel_simulate.yaml"),
        SimulateOptions {
            scenario: Some("rent_hike"),
            ..options
        },
    )
    .unwrap();
    for (a, b) in profit
        .samples
        .iter()
        .zip(&hiked.outputs["year.profit"].samples)
    {
        assert!(close(a - b, 9600.0, 1e-6), "{a} {b}");
    }
}
