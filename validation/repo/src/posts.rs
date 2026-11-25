//! The numbers in `docs/posts/` are what Forge prints (Session Prompt part 2).
//!
//! Each post quotes the output of commands on `examples/`. This runs those
//! commands and checks that the output has the figures the post is built on
//! and that the post says them, so a post cannot drift from the engine.

use crate::forge;

/// The text of `docs/posts/<name>.md`.
#[must_use]
pub fn post(name: &str) -> String {
    crate::read(&format!("docs/posts/{name}.md"))
}

/// Run `args`; every string of `output` must be in what it prints and every
/// string of `said` in the post.
///
/// # Panics
///
/// Panics, naming what is missing, if the command fails, does not print a
/// string of `output`, or the post does not say a string of `said`.
pub fn check(name: &str, args: &[&str], output: &[&str], said: &[&str]) {
    let printed = forge(args).printed();
    let text = post(name);
    for s in output {
        assert!(
            printed.contains(s),
            "forge {args:?} does not print {s:?}:\n{printed}"
        );
    }
    for s in said {
        assert!(text.contains(s), "docs/posts/{name}.md does not say {s:?}");
    }
}

/// The bagel shop's plan
pub const BAGEL: &str = "examples/bagel/bagel.yaml";
/// Its simulation
pub const SIM: &str = "examples/bagel/bagel_simulate.yaml";
/// Its slow week
pub const SLOW: &str = "examples/bagel/slow_week.yaml";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bagel_price() {
        check(
            "bagel-price",
            &["calculate", BAGEL, "--dry-run"],
            &[
                "year.profit = 786.320361712362",
                "year.revenue = 386827.449537828",
            ],
            &["786.32", "386,827.45", "0.2%"],
        );
        check(
            "bagel-price",
            &[
                "break-even",
                BAGEL,
                "--output",
                "year.profit",
                "--vary",
                "assumptions.price_per_dozen",
            ],
            &["assumptions.price_per_dozen = 15.97"],
            &["15.97"],
        );
        check(
            "bagel-price",
            &["simulate", SIM],
            &["P(year.profit > 0) = 34.76%"],
            &["34.76%"],
        );
        check(
            "bagel-price",
            &[
                "simulate",
                SIM,
                "--seek",
                "P(year.profit > 0) = 0.8",
                "--vary",
                "price_per_dozen",
                "--min",
                "14",
                "--max",
                "24",
            ],
            &["price_per_dozen = 18.32\n", "(2.92% at 14, 100.00% at 24)"],
            &["18.32", "2.92%", "2.32 more"],
        );
    }

    #[test]
    fn bagel_seasons() {
        let printed = forge(&["calculate", BAGEL, "--dry-run"]).printed();
        let text = post("bagel-seasons");
        let line = printed
            .lines()
            .find(|l| l.trim_start().starts_with("profit = ["))
            .unwrap();
        let profits: Vec<f64> = line
            .split(['[', ']'])
            .nth(1)
            .unwrap()
            .split(", ")
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(profits.len(), 12);
        // The table of the post is the month's profit to two decimals
        for p in &profits {
            let shown = format!("{:.2}", p.abs());
            let (whole, cents) = shown.split_once('.').unwrap();
            let mut grouped = String::new();
            for (i, c) in whole.chars().enumerate() {
                if i > 0 && (whole.len() - i) % 3 == 0 {
                    grouped.push(',');
                }
                grouped.push(c);
            }
            let cell = format!("{}{grouped}.{cents}", if *p < 0.0 { "-" } else { "" });
            assert!(
                text.contains(&format!("| {cell} |")),
                "no {cell} in the table"
            );
        }
        assert_eq!(profits.iter().filter(|p| **p < 0.0).count(), 5);
        // The shop is 28,652.05 down at the end of April (the first six months)
        let to_april: f64 = profits[..6].iter().sum();
        assert_eq!(format!("{:.2}", -to_april), "28652.05");
        assert!(text.contains("28,652.05 down"));
        check(
            "bagel-seasons",
            &["simulate", SIM],
            &[
                "48.72%", "98.30%", "90.00%", "56.92%", "4.86%", "2.96%", "24.08%", "28.52%",
            ],
            &["48.72%", "98.30%", "90.00%", "4.86%", "2.96%"],
        );
    }

    #[test]
    fn second_shop() {
        check(
            "second-shop",
            &["decision-tree", "examples/bagel/second_shop.yaml"],
            &[
                "Expected Value: $61,500.00",
                "$198,000.00 with probability 40.0%",
                "-$12,000.00 with probability 50.0%",
                "-$117,000.00 with probability 10.0%",
                "At \"open_after_busy\": choose \"open\"",
                "At \"open_after_quiet\": choose \"walk_away\"",
            ],
            &["61,500", "3,000 more", "-117,000", "12,000"],
        );
        // The same tree without the pop-up branch
        let tree = crate::read("examples/bagel/second_shop.yaml");
        let start = tree.find("      popup_first:").unwrap();
        let end = tree.find("      stay_one_shop:").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("second_shop_without_popup.yaml");
        std::fs::write(&file, format!("{}{}", &tree[..start], &tree[end..])).unwrap();
        check(
            "second-shop",
            &["decision-tree", file.to_str().unwrap()],
            &[
                "Expected Value: $58,500.00",
                "P(value > 0): 45.0%",
                "Worst case: -$90,000.00",
            ],
            &["58,500", "90,000", "second_shop_without_popup.yaml"],
        );
    }

    #[test]
    fn slow_week() {
        for (args, output, said) in [
            (vec!["-q", "roadwork"], "20.00%", "20%"),
            (vec!["-q", "sales"], "38.00%", "38%"),
            (vec!["-q", "roadwork", "-e", "sales=slow"], "40.00%", "40%"),
            (
                vec!["-q", "roadwork", "-e", "sales=slow", "-e", "weather=rainy"],
                "27.27%",
                "27.27%",
            ),
            (
                vec!["-q", "weather", "-e", "sales=slow"],
                "52.11%",
                "52.11%",
            ),
            (
                vec!["-q", "weather", "-e", "sales=slow", "-e", "roadwork=yes"],
                "35.53%",
                "35.53%",
            ),
        ] {
            let mut all = vec!["bayesian", SLOW];
            all.extend(args);
            check("slow-week", &all, &[output], &[said]);
        }
    }

    #[test]
    fn waiting() {
        check(
            "waiting",
            &["real-options", "examples/bagel/wait_option.yaml"],
            &[
                "Traditional NPV (invest now, no flexibility): $60,000.00",
                "Adds over the traditional NPV: $4,604.68",
                "Adds over the traditional NPV: $279.72",
                "exercise when value > $198,000",
                "exercise when value < $132,000",
            ],
            &["64,604.68", "279.72", "4,604.68", "198,000", "132,000"],
        );
    }

    #[test]
    fn first_month() {
        check(
            "first-month",
            &["bootstrap", "examples/bagel/first_month.yaml"],
            &[
                "Original statistic: 50.7000",
                "Bootstrap std error: 2.0163",
                "90% CI: [47.4333, 54.0667]",
                "95% CI: [46.8000, 54.7000]",
            ],
            &["50.7", "between 46.8 and 54.7", "47.4 to 54.1", "2.02"],
        );
        // The plan's first month: 1,402.5 dozens over 30 days is 46.75 a day
        check(
            "first-month",
            &["calculate", BAGEL, "--dry-run"],
            // The format's text (`#,##0`), then the full values (T2-CLI-FORMAT)
            &["dozens = [1,403; 2,124;", "[1402.5, 2124.375,"],
            &["1,402.5", "46.75"],
        );
    }

    #[test]
    fn mortgage() {
        let file = "examples/mortgage/mortgage.yaml";
        check(
            "mortgage",
            &["calculate", file, "--dry-run"],
            &[
                "terms.payment = 2563.68762956532",
                "as_agreed.total_interest = 319106.288869595",
                "saved.interest = 64596.8832216271",
                "saved.months = 53",
                "paid_off.payments = 247",
                "paid_off.last_payment = 42.2487749021918",
                "paid_off.year = 2047",
                "paid_off.month_of_year = 6",
                "year_one.interest = 21056.1731223496",
                "year_one.principal_paid = 13308.0784324342",
                "check.interest = 0",
            ],
            &[
                "2,563.69",
                "319,106.29",
                "64,596.88",
                "53 months",
                "247",
                "42.25",
                "21,056.17",
                "13,308.08",
            ],
        );
        check(
            "mortgage",
            &[
                "goal-seek",
                file,
                "--target",
                "saved.interest",
                "--value",
                "80000",
                "--vary",
                "loan.extra_per_month",
            ],
            &["loan.extra_per_month = 397.04 (397.044207903935) → saved.interest = 80,000.00"],
            &["397.04", "97.04", "15,403.12"],
        );
        // 80,000 less the 64,596.88 saved by 300 a month
        assert!((80_000.0_f64 - 64_596.88 - 15_403.12).abs() < 1e-9);
        // Dividing the annual rate by 12 gives a payment that is 12.20 too high
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("m.yaml");
        std::fs::write(
            &model,
            "_forge_version: \"5.1.0\"\nm:\n  naive: {formula: \"=-PMT(0.0479 / 12, 300, 450000)\"}\n  semi: {formula: \"=-PMT((1 + 0.0479 / 2) ^ (1 / 6) - 1, 300, 450000)\"}\n  diff: {formula: \"=naive - semi\"}\n",
        )
        .unwrap();
        check(
            "mortgage",
            &["calculate", model.to_str().unwrap(), "--dry-run"],
            // PMT keeps its digits since 2026-11-14: the exact payments for these
            // two rates are 2575.8919890757813 and 2563.6876295653160
            &["m.naive = 2575.89198907578", "m.semi = 2563.68762956532"],
            &["2,575.89", "12.20"],
        );
    }

    #[test]
    fn household() {
        let (budget, actual) = (
            "examples/household/budget.yaml",
            "examples/household/actual.yaml",
        );
        check(
            "household",
            &["calculate", budget, "--dry-run"],
            &[
                "expenses.total = 3100",
                "result.left_over = 2500",
                "result.savings_rate = 0.446428571428571",
            ],
            &["44.6%", "2,500"],
        );
        check(
            "household",
            &["calculate", actual, "--dry-run"],
            &[
                "expenses.total = 3305.85",
                "result.left_over = 2044.15",
                "result.savings_rate = 0.382084112149533",
            ],
            &["2,044.15", "38.2%"],
        );
        check(
            "household",
            &["variance", budget, actual, "--threshold", "10"],
            &[
                "income.side_work              400.00       150.00      -250.00     -62.5%",
                "expenses.groceries            700.00       812.43       112.43      16.1%",
                "expenses.transport            180.00        96.50       -83.50     -46.4%",
                "expenses.eating_out           250.00       388.75       138.75      55.5%",
                "Favorable: 1  ❌ Unfavorable: 9",
                "Alerts (>10%): 8",
                "expenses.total              3,100.00     3,305.85       205.85       6.6%",
            ],
            &["205.85", "6.6%", "55.5%", "83.50", "Alerts (>10%): 8"],
        );
    }
}
