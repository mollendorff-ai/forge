//! Tests for `forge compare`'s table and formats.

use super::*;

/// Eight scenarios with long names, as in a consumer-proposal model
fn eight() -> ComparisonResult {
    let scenarios: Vec<String> = [1100, 1300, 1500, 1700]
        .iter()
        .flat_map(|rent| {
            [
                format!("rent_{rent}_prosupport_recognized"),
                format!("rent_{rent}_prosupport_not_recognized"),
            ]
        })
        .collect();
    let variables = vec![
        "bankruptcy.monthly_surplus_payment".to_string(),
        "living.months".to_string(),
        "living.rate".to_string(),
    ];
    let mut values = indexmap::IndexMap::new();
    for (i, variable) in variables.iter().enumerate() {
        let mut row = indexmap::IndexMap::new();
        let surplus = [
            774.75, 724.75, 674.75, 624.75, 574.75, 524.75, 474.75, 424.75,
        ];
        for (s, scenario) in scenarios.iter().enumerate() {
            let value = match i {
                0 => surplus[s],
                1 => 60.0,
                _ => 0.149_75,
            };
            row.insert(scenario.clone(), Some(value));
        }
        values.insert(variable.clone(), row);
    }
    ComparisonResult {
        scenarios,
        variables,
        values,
        descriptions: indexmap::IndexMap::new(),
    }
}

fn inputs() -> Vec<String> {
    vec!["living.months".to_string(), "living.rate".to_string()]
}

#[test]
fn every_column_has_room_and_two_spaces_between() {
    let text = render_comparison(&eight(), &inputs(), ReportFormat::Table, None);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "{text}");
    // Names never run together
    assert!(
        lines[0].contains("rent_1100_prosupport_recognized  rent_1100_prosupport_not_recognized")
    );
    // Every row has the width of the header: values stay in their columns
    let width = lines[0].chars().count();
    for line in &lines {
        assert_eq!(line.chars().count(), width, "{text}");
    }
    // Numbers: report's rule, whole numbers whole, an input as written
    assert!(
        lines[1].starts_with("bankruptcy.monthly_surplus_payment  "),
        "{text}"
    );
    assert!(lines[1].contains("774.75") && lines[1].contains("424.75"));
    assert!(lines[2].ends_with(" 60"), "{text}");
    assert!(lines[3].ends_with(" 0.14975"), "{text}");
}

#[test]
fn a_table_wider_than_the_terminal_is_printed_in_blocks() {
    let text = render_comparison(&eight(), &inputs(), ReportFormat::Table, Some(80));
    let blocks: Vec<&str> = text.split("\n\n").collect();
    assert!(blocks.len() > 1, "{text}");
    for line in text.lines() {
        assert!(line.chars().count() <= 80, "{line}");
    }
    // Long scenario names and few scalars: one row per scenario, so every
    // block lists the eight scenarios and each scalar is in one block
    for block in &blocks {
        assert!(block.starts_with("scenario "), "{text}");
        for scenario in &eight().scenarios {
            assert!(
                block.lines().any(|l| l.starts_with(scenario.as_str())),
                "{text}"
            );
        }
    }
    for variable in &eight().variables {
        let hits = blocks
            .iter()
            .filter(|b| {
                b.lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .any(|w| w == variable)
            })
            .count();
        assert_eq!(hits, 1, "{variable}");
    }
}

#[test]
fn many_scalars_and_short_scenario_names_keep_one_row_per_scalar() {
    let scenarios: Vec<String> = (1..=8).map(|i| format!("s{i}")).collect();
    let variables: Vec<String> = (1..=30).map(|i| format!("group.value_{i}")).collect();
    let mut values = indexmap::IndexMap::new();
    for v in &variables {
        values.insert(
            v.clone(),
            scenarios
                .iter()
                .map(|s| (s.clone(), Some(1234.5)))
                .collect(),
        );
    }
    let result = ComparisonResult {
        scenarios,
        variables,
        values,
        descriptions: indexmap::IndexMap::new(),
    };
    let text = render_comparison(&result, &[], ReportFormat::Table, Some(60));
    assert!(text.starts_with("variable "), "{text}");
    assert!(text.split("\n\n").count() > 1, "{text}");
    for line in text.lines() {
        assert!(line.chars().count() <= 60, "{line}");
    }
}

#[test]
fn one_column_wider_than_the_terminal_still_gets_a_block() {
    let text = render_comparison(&eight(), &inputs(), ReportFormat::Table, Some(10));
    // One row per scalar: eight blocks; one row per scenario: three
    assert_eq!(text.split("\n\n").count(), 3, "{text}");
}

#[test]
fn csv_and_json_keep_full_precision() {
    let mut result = eight();
    result.values["living.rate"][0] = Some(0.149_753_000_1);
    result.values["living.rate"][1] = None;
    let csv = render_comparison(&result, &inputs(), ReportFormat::Csv, None);
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines[0].starts_with("variable,rent_1100_prosupport_recognized,"));
    assert!(
        lines[3].starts_with("living.rate,0.1497530001,,0.14975"),
        "{csv}"
    );

    let json = render_comparison(&result, &inputs(), ReportFormat::Json, Some(10));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["scenarios"].as_array().unwrap().len(), 8);
    assert_eq!(
        parsed["values"]["living.rate"]["rent_1100_prosupport_recognized"],
        0.149_753_000_1
    );
    assert!(parsed["values"]["living.rate"]["rent_1100_prosupport_not_recognized"].is_null());
}

#[test]
fn markdown_is_one_pipe_table_whatever_the_width() {
    let text = render_comparison(&eight(), &inputs(), ReportFormat::Markdown, Some(20));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[1], format!("| --- |{}", " ---: |".repeat(8)));
    assert!(lines[2].starts_with("| bankruptcy.monthly_surplus_payment | 774.75 |"));
    assert!(lines[4].contains("| 0.14975 |"));
}

#[test]
fn descriptions_are_a_legend_below_the_table() {
    let mut result = eight();
    result.descriptions.insert(
        "rent_1500_prosupport_recognized".to_string(),
        "Studio at $1,500, support recognized".to_string(),
    );
    let table = render_comparison(&result, &inputs(), ReportFormat::Table, None);
    assert!(
        table
            .ends_with("\nrent_1500_prosupport_recognized: Studio at $1,500, support recognized\n"),
        "{table}"
    );
    let markdown = render_comparison(&result, &inputs(), ReportFormat::Markdown, None);
    assert!(
        markdown.ends_with(
            "\n- **rent_1500_prosupport_recognized**: Studio at $1,500, support recognized\n"
        ),
        "{markdown}"
    );
    // Data formats carry the descriptions only in JSON, and the CSV stays a table
    let csv = render_comparison(&result, &inputs(), ReportFormat::Csv, None);
    assert!(!csv.contains("Studio"));
    let json = render_comparison(&result, &inputs(), ReportFormat::Json, None);
    assert!(json.contains("\"descriptions\""));
    // No description, no legend and no field
    let plain = render_comparison(&eight(), &inputs(), ReportFormat::Json, None);
    assert!(!plain.contains("descriptions"));
}
