//! The headline numbers of examples/mortgage and examples/household, as
//! computed outside Forge.
//!
//! The mortgage, as a month-by-month amortisation loop (no closed forms):
//! ```python
//! r = (1 + 0.0479 / 2) ** (1 / 6) - 1        # semi-annual compounding
//! P, n = 450000, 300
//! pay = P * r / (1 - (1 + r) ** -n)            # 2563.6876295653
//! def run(pmt):
//!     bal, months, interest, y1i, y1p = P, 0, 0.0, 0.0, 0.0
//!     while bal > 1e-9:
//!         i = bal * r; p = min(pmt, bal + i)
//!         bal = bal + i - p; months += 1; interest += i
//!         if months <= 12: y1i += i; y1p += p - i
//!     return months, p, interest, y1i, y1p
//! run(pay)        # interest 319106.2888696
//! # the extra payment that saves 80,000 of interest, as in the README
//! brentq(lambda e: run(pay)[2] - run(pay + e)[2] - 80000, 1, 2000)  # 397.044208
//! run(pay + 300)  # 247 payments, the last 42.248775, interest 254509.4056479,
//!                 # first year interest 21056.173122, principal 13308.078432
//! ```
//! The `schedule` table of the example is that loop, written with PREV; the
//! balances the loop prints for months 1, 12, 60, 120, 246 and 247 are in the
//! test below.
//!
//! examples/periods, in Python:
//! ```python
//! # monthly_plan.yaml
//! season = [0.85, 0.80, 0.95, 1.00, 1.05, 1.10, 0.90, 0.85, 1.00, 1.05, 1.15, 1.30]
//! trend, cash, sales, profit, low = 18000, 12000, 0, 0, 1e9
//! for m in range(12):
//!     if m: trend *= 1.01
//!     s = trend * season[m]; p = s * (1 - 0.42) - 9500
//!     cash += p; sales += s; profit += p; low = min(low, cash)
//! # sales 229045.2297709736, profit 18846.23326716473, cash 30846.23326716473,
//! # lowest 10309.52, two months at a loss
//! # dcf.yaml
//! rev, pv = 4200000, 0
//! for t in range(1, 6):
//!     rev *= 1.08; f = rev * 0.18 * (1 - 0.265) * (1 - 0.30)
//!     df = 1 / 1.095 ** t; pv += f * df
//! tv = f * 1.025 / (0.095 - 0.025)
//! # pv 1866331.1790654135, tv 8368580.101335552, tv * df 5315953.599495626,
//! # equity 5682284.77856104
//! ```
//!
//! examples/options/compare-options.yaml, in Python:
//! ```python
//! T, cap = 4800, 0.35
//! for rent, util, net, transit, move_in, inc in options:
//!     mo = rent + util + net + transit
//!     row = (mo, mo * 12 + move_in, rent * (1 + inc) + util + net + transit,
//!            T - mo, mo / T, T * cap - mo)
//! # maple_studio    1582 19884 1622.5             3218 0.32958333333333334  98
//! # harbour_one_bed 1650 21000 1689.8749999999998 3150 0.34375              30
//! # park_loft       1752 21674 1811.2             3048 0.365               -72
//! # river_studio    1542 20004 1587.1499999999999 3258 0.32125             138
//! # garden_one_bed  1660 20320 1736.25            3140 0.3458333333333333   20
//! # lowest monthly 1542, lowest first year 19884
//! ```
//!
//! The household: income 5200 + 400 = 5600 budgeted, 5200 + 150 = 5350
//! actual; expenses 3100 budgeted, 1650 + 812.43 + 96.50 + 163.20 + 120 +
//! 388.75 + 74.97 = 3305.85 actual; left over 2500 and 2044.15.
//!
//! examples/rolling-forecast, in Python:
//! ```python
//! su, sp = [985, 1032, 1001, 1047, 1052, 1038, 1066], [50, 50, 49.5, 49.5, 49.8, 49.8, 50.1]
//! pu, pp = [214, 220, 231, 228, 240, 244, 251], [118, 118, 119, 119, 119.5, 119.5, 120]
//! for _ in range(12):   # November 2026 to October 2027, grown from the month before
//!     su.append(su[-1] * 1.008); sp.append(sp[-1] * 1.002)
//!     pu.append(pu[-1] * 1.015); pp.append(pp[-1] * 1.003)
//! rev = [su[i] * sp[i] + pu[i] * pp[i] for i in range(19)]
//! fy27 = sum(rev[:12])                                    # 987621.9306764038
//! bs, bp = sum(1000 + 10 * i for i in range(12)), sum(200 + 5 * i for i in range(12))
//! budget = bs * 50 + bp * 120                             # 960600
//! quarters = [sum(rev[k:k + 3]) for k in (0, 3, 6, 9)]   # 229100.5, 240878.5,
//! #                                   253830.2732159771, 263812.6574604267
//! next_12 = sum(rev[7:])                                  # 1090944.3557895026
//! ls, lp = sum(su[:12]), sum(pu[:12]); L, B = ls + lp, bs + bp
//! volume = (L - B) * budget / B                          # 14412.743557430787
//! mix = L * ((ls / L - bs / B) * 50 + (lp / L - bp / B) * 120)   # 11875.965095038919
//! price = sum(su[i] * sp[i] - su[i] * 50 + pu[i] * pp[i] - pu[i] * 120 for i in range(12))
//! #                                                       # 733.22202393...
//! ```

use mollendorff_forge::cli::commands::{render, report_core, Cell, ReportFormat, ReportOptions};
use mollendorff_forge::cli::{calculate_core, goal_seek_core, variance_core};
use mollendorff_forge::core::ArrayCalculator;
use mollendorff_forge::ColumnValue;
use std::path::Path;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn the_mortgage_matches_an_amortisation_loop() {
    let result = calculate_core(Path::new("examples/mortgage/mortgage.yaml"), true, None).unwrap();
    let get = |name: &str| result.scalars[name].unwrap();
    for (name, expected) in [
        ("terms.payment", 2_563.687_629_565_3),
        ("as_agreed.total_interest", 319_106.288_869_6),
        ("with_extra.last_payment", 42.248_774_9),
        ("with_extra.total_interest", 254_509.405_647_9),
        ("saved.interest", 64_596.883_221_6),
        ("saved.months", 53.0),
        ("year_one.interest", 21_056.173_122_35),
        ("year_one.principal_paid", 13_308.078_432_43),
    ] {
        assert!(close(get(name), expected, 1e-5), "{name}: {}", get(name));
    }
    // 246 full payments and a smaller last one: 247, as in the loop
    assert!(close(get("with_extra.full_months"), 246.0, 0.0));
    // The schedule and the closed forms agree
    for name in ["check.interest", "check.last_payment", "check.payments"] {
        assert!(close(get(name), 0.0, 0.0), "{name}: {}", get(name));
    }
    assert!(close(get("paid_off.payments"), 247.0, 0.0));
    assert!(close(get("paid_off.year"), 2047.0, 0.0));
    assert!(close(get("paid_off.month_of_year"), 6.0, 0.0));
}

fn numbers(values: &ColumnValue) -> &[f64] {
    match values {
        ColumnValue::Number(v) => v,
        other => panic!("not numbers: {other:?}"),
    }
}

/// T2-TIME: the schedule, written with PREV, is the Python loop month by
/// month, to well under a cent.
#[test]
fn the_mortgage_schedule_matches_the_loop_month_by_month() {
    let model =
        mollendorff_forge::parser::parse_model(Path::new("examples/mortgage/mortgage.yaml"))
            .unwrap();
    let model = ArrayCalculator::new(model).calculate_all().unwrap();
    let schedule = &model.tables["schedule"];
    let column = |name: &str| numbers(&schedule.columns[name].values);
    assert_eq!(column("closing").len(), 300);
    // (month, opening, interest, payment, closing) from the loop
    for (month, opening, interest, payment, closing) in [
        (
            1,
            450_000.0,
            1_778.582_904_770_109_5,
            2_863.687_629_565_315_5,
            448_914.895_275_204_8,
        ),
        (
            12,
            437_825.146_262_861,
            1_730.462_934_270_216_5,
            2_863.687_629_565_315_5,
            436_691.921_567_565_9,
        ),
        (
            60,
            378_057.801_437_760_8,
            1_494.238_094_782_608_9,
            2_863.687_629_565_315_5,
            376_688.351_902_978_15,
        ),
        (
            120,
            285_535.300_994_710_65,
            1_128.551_566_794_622_2,
            2_863.687_629_565_315_5,
            283_800.164_931_94,
        ),
        (
            246,
            2_894.330_506_949_832,
            11.439_570_578_701_28,
            2_863.687_629_565_315_5,
            42.082_447_963_217_874,
        ),
        (
            247,
            42.082_447_963_217_874,
            0.166_326_938_973_904_52,
            42.248_774_902_191_78,
            0.0,
        ),
        (248, 0.0, 0.0, 0.0, 0.0),
        (300, 0.0, 0.0, 0.0, 0.0),
    ] {
        for (name, expected) in [
            ("opening", opening),
            ("interest", interest),
            ("payment", payment),
            ("closing", closing),
        ] {
            let got = column(name)[month - 1];
            assert!(close(got, expected, 1e-6), "month {month} {name}: {got}");
        }
    }
    let total = |name: &str| column(name).iter().sum::<f64>();
    assert!(close(total("interest"), 254_509.405_647_968_52, 1e-5));
    assert!(close(total("payment"), 704_509.405_647_969_8, 1e-5));
    match &schedule.columns["month"].values {
        ColumnValue::Date(dates) => {
            assert_eq!(dates[0], "2026-12-01");
            assert_eq!(dates[246], "2047-06-01");
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_period_examples_match_python() {
    let plan = calculate_core(Path::new("examples/periods/monthly_plan.yaml"), true, None).unwrap();
    for (name, expected) in [
        ("year.sales", 229_045.229_770_973_6),
        ("year.profit", 18_846.233_267_164_73),
        ("year.closing_cash", 30_846.233_267_164_73),
        ("year.lowest_cash", 10_309.52),
        ("year.months_at_a_loss", 2.0),
    ] {
        let got = plan.scalars[name].unwrap();
        assert!(close(got, expected, 1e-8), "{name}: {got}");
    }
    let dcf = calculate_core(Path::new("examples/periods/dcf.yaml"), true, None).unwrap();
    for (name, expected) in [
        ("valuation.forecast_years", 1_866_331.179_065_413_5),
        ("valuation.terminal_value", 8_368_580.101_335_552),
        ("valuation.terminal_value_today", 5_315_953.599_495_626),
        ("valuation.equity_value", 5_682_284.778_561_04),
        ("valuation.check", 0.0),
    ] {
        let got = dcf.scalars[name].unwrap();
        assert!(close(got, expected, 1e-6), "{name}: {got}");
    }
}

#[test]
fn the_extra_payment_that_saves_eighty_thousand() {
    let result = goal_seek_core(
        Path::new("examples/mortgage/mortgage.yaml"),
        "saved.interest",
        80_000.0,
        "loan.extra_per_month",
        (None, None),
        0.0001,
    )
    .unwrap();
    assert!(result.converged);
    assert!(
        close(result.solution, 397.044_208, 1e-4),
        "{}",
        result.solution
    );
}

#[test]
fn the_household_variance_has_the_right_directions_and_totals() {
    let report = variance_core(
        Path::new("examples/household/budget.yaml"),
        Path::new("examples/household/actual.yaml"),
        10.0,
    )
    .unwrap();
    let get = |name: &str| {
        report
            .results
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    for (name, budget, actual, better, favourable) in [
        ("income.salary", 5200.0, 5200.0, "higher", Some(true)),
        ("income.side_work", 400.0, 150.0, "higher", Some(false)),
        ("income.total", 5600.0, 5350.0, "higher", Some(false)),
        ("expenses.transport", 180.0, 96.5, "lower", Some(true)),
        ("expenses.eating_out", 250.0, 388.75, "lower", Some(false)),
        ("expenses.total", 3100.0, 3305.85, "lower", Some(false)),
        ("result.left_over", 2500.0, 2044.15, "higher", Some(false)),
    ] {
        let r = get(name);
        assert!(close(r.budget, budget, 1e-9), "{name} budget {}", r.budget);
        assert!(close(r.actual, actual, 1e-9), "{name} actual {}", r.actual);
        assert_eq!(r.better, better, "{name}");
        assert_eq!(r.is_favorable, favourable, "{name}");
    }
    // Over the 10% threshold: side work, groceries, transport, utilities,
    // eating out, subscriptions, left over and the savings rate
    let alerts: Vec<&str> = report
        .results
        .iter()
        .filter(|r| r.exceeds_threshold)
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        alerts,
        [
            "income.side_work",
            "expenses.groceries",
            "expenses.transport",
            "expenses.utilities",
            "expenses.eating_out",
            "expenses.subscriptions",
            "result.left_over",
            "result.savings_rate",
        ]
    );
}

#[test]
fn the_rolling_forecast_matches_python() {
    let model = calculate_core(
        Path::new("examples/rolling-forecast/forecast.yaml"),
        true,
        None,
    )
    .unwrap();
    for (name, expected) in [
        ("fy2027.revenue", 987_621.930_676_403_8),
        ("fy2027.budget", 960_600.0),
        ("fy2027.variance", 27_021.930_676_403_8),
        ("fy2027.actual_so_far", 553_505.6),
        ("next_12_months.revenue", 1_090_944.355_789_502_6),
        ("pvm.volume", 14_412.743_557_430_787),
        ("pvm.mix", 11_875.965_095_038_919),
        ("pvm.price", 733.222_023_934),
        ("pvm.check", 0.0),
    ] {
        let got = model.scalars[name].unwrap();
        assert!(close(got, expected, 1e-6), "{name}: {got}");
    }
    // The table of quarters (ADR-058): the quarters of the same Python, and
    // the budget's months by quarter, by hand (Q1: 3,030 x 50 + 615 x 120)
    let quarter = |column: &str| -> Vec<f64> {
        model.tables["quarters"].values[column]
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    };
    let revenue = [
        229_100.5,
        240_878.5,
        253_830.273_215_977_1,
        263_812.657_460_426_7,
    ];
    let budget = [225_300.0, 235_200.0, 245_100.0, 255_000.0];
    for (i, (r, b)) in revenue.iter().zip(&budget).enumerate() {
        assert!(close(quarter("revenue")[i], *r, 1e-6), "quarter {}", i + 1);
        assert!(close(quarter("budget")[i], *b, 1e-9), "quarter {}", i + 1);
        assert!(
            close(quarter("variance")[i], r - b, 1e-6),
            "quarter {}",
            i + 1
        );
        let share = r / 987_621.930_676_403_8;
        assert!(
            close(quarter("share_of_year")[i], share, 1e-9),
            "quarter {}",
            i + 1
        );
    }
}

#[test]
fn each_demo_calculates_in_under_a_second() {
    for model in [
        "examples/mortgage/mortgage.yaml",
        "examples/household/budget.yaml",
        "examples/household/actual.yaml",
        "examples/bagel/bagel.yaml",
        "examples/periods/monthly_plan.yaml",
        "examples/periods/dcf.yaml",
        "examples/options/compare-options.yaml",
        "examples/rolling-forecast/forecast.yaml",
    ] {
        let start = std::time::Instant::now();
        calculate_core(Path::new(model), true, None).unwrap();
        let took = start.elapsed();
        // A debug build is slower than a release build; a second is still ample
        assert!(took.as_secs_f64() < 1.0, "{model}: {took:?}");
    }
}

/// Every model under examples/ passes the schema of its format (the parser
/// checks it before anything else) and parses.
#[test]
fn every_example_passes_its_schema() {
    fn models(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                models(&path, out);
            } else if path.extension().is_some_and(|e| e == "yaml") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    models(Path::new("examples"), &mut files);
    assert!(files.len() >= 20, "{files:?}");
    for file in files {
        if let Err(e) = mollendorff_forge::parser::parse_model(&file) {
            panic!("{}: {e}", file.display());
        }
    }
}

/// T2-COMPARE-EXAMPLE: forge calculate, then forge report, gives Python's
/// numbers, the five options in one table and the summary in its own.
#[test]
fn the_comparison_example_matches_python_through_report() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("compare-options.yaml");
    std::fs::copy("examples/options/compare-options.yaml", &file).unwrap();
    calculate_core(&file, false, None).unwrap();
    let before = std::fs::read_to_string(&file).unwrap();
    let report = report_core(
        &file,
        &ReportOptions {
            no_inputs: true,
            ..ReportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);

    let expected: [(&str, [f64; 6]); 5] = [
        (
            "maple_studio",
            [
                1582.0,
                19884.0,
                1622.5,
                3218.0,
                0.329_583_333_333_333_34,
                98.0,
            ],
        ),
        (
            "harbour_one_bed",
            [
                1650.0,
                21000.0,
                1_689.874_999_999_999_8,
                3150.0,
                0.34375,
                30.0,
            ],
        ),
        ("park_loft", [1752.0, 21674.0, 1811.2, 3048.0, 0.365, -72.0]),
        (
            "river_studio",
            [
                1542.0,
                20004.0,
                1_587.149_999_999_999_9,
                3258.0,
                0.32125,
                138.0,
            ],
        ),
        (
            "garden_one_bed",
            [
                1660.0,
                20320.0,
                1736.25,
                3140.0,
                0.345_833_333_333_333_3,
                20.0,
            ],
        ),
    ];
    assert_eq!(report.sections.len(), 2, "{report:?}");
    let options = &report.sections[0];
    assert_eq!(
        options.keys,
        [
            "monthly",
            "first_year",
            "second_year_monthly",
            "left_each_month",
            "share_of_pay",
            "room_under_cap"
        ]
    );
    assert_eq!(options.rows.len(), 5);
    for (row, (group, values)) in options.rows.iter().zip(expected) {
        assert_eq!(row.group, group);
        for (cell, want) in row.cells.iter().zip(values) {
            let Cell::Value(got) = cell else {
                panic!("{group}: {cell:?}")
            };
            assert!(close(*got, want, 1e-9), "{group}: {got} vs {want}");
        }
    }
    let summary = &report.sections[1];
    assert_eq!(summary.rows[0].group, "summary");
    assert_eq!(
        summary.rows[0].cells,
        [Cell::Value(1542.0), Cell::Value(19884.0)]
    );
    assert_eq!(report.uncalculated, Vec::<String>::new());

    // docs/REPORT.md shows this report as Markdown, as it is printed.
    let doc = std::fs::read_to_string("docs/REPORT.md").unwrap();
    let markdown = render(&report, ReportFormat::Markdown);
    assert!(
        doc.contains(&markdown),
        "docs/REPORT.md must show:\n{markdown}"
    );
}

/// examples/capex-plan against a month-by-month loan and a least-squares fit.
/// In Python: the loan loop gives payment 2347.9377862474407, year-2 interest
/// 5775.928291150727 and principal 22399.32514381856; with numpy,
/// `polyfit(x, y, 1)` is [6300, 34500] and `(66000 / 41000) ** .25 - 1` is
/// 0.12639319927790216.
#[test]
fn the_capex_plan_matches_python() {
    let result = calculate_core(Path::new("examples/capex-plan/capex.yaml"), true, None).unwrap();
    let value = |name: &str| result.scalars[name].unwrap();
    assert!(close(value("bank.payment"), 2_347.937_786_247_440_7, 1e-9));
    assert!(close(
        value("bank.year2_interest"),
        5_775.928_291_150_727,
        1e-9
    ));
    assert!(close(
        value("bank.year2_principal"),
        22_399.325_143_818_56,
        1e-9
    ));
    assert!(value("bank.check").abs() < 1e-9);
    assert!(close(value("writedown.straight_line"), 13_500.0, 1e-12));
    assert!(close(value("writedown.sum_of_years"), 18_000.0, 1e-12));
    assert!(close(value("writedown.double_declining"), 16_875.0, 1e-12));
    assert!(close(value("writedown.total"), 108_000.0, 1e-12));
    assert!(close(value("growth.cagr"), 0.126_393_199_277_902_16, 1e-9));
    assert!(close(value("growth.slope"), 6_300.0, 1e-9));
    assert!(close(value("growth.year6"), 72_300.0, 1e-9));
}
