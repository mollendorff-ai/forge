use super::*;
use std::io::Write as _;

fn model_of(yaml: &str) -> ParsedModel {
    let mut file = tempfile::NamedTempFile::new().expect("temp file");
    file.write_all(yaml.as_bytes()).expect("write");
    parser::parse_model(file.path()).expect("parse")
}

fn report_of(yaml: &str, options: &ReportOptions) -> Report {
    build_report(&model_of(yaml), options).expect("report")
}

const OPTIONS: &str = r#"_forge_version: "5.1.0"
oak_suite:
  rent: 1450
  utilities: 60
  year1_monthly: {formula: "=oak_suite.rent + oak_suite.utilities", value: 1510}
elm_loft:
  utilities: 90
  rent: 1390
  year1_monthly: {formula: "=elm_loft.rent + elm_loft.utilities", value: 1480}
assumptions:
  rate: 0.0479
"#;

#[test]
fn numbers_have_two_decimals_and_thousands_separators() {
    assert_eq!(display_number(1450.0), "1,450.00");
    assert_eq!(display_number(1_234_567.891), "1,234,567.89");
    assert_eq!(display_number(-1234.5), "-1,234.50");
    assert_eq!(display_number(999.995), "1,000.00");
    assert_eq!(display_number(100.0), "100.00");
    assert_eq!(display_number(0.0), "0.00");
}

#[test]
fn numbers_under_one_keep_four_significant_digits() {
    assert_eq!(display_number(0.0479), "0.0479");
    assert_eq!(display_number(0.5), "0.50");
    assert_eq!(display_number(0.25), "0.25");
    assert_eq!(display_number(0.123_456), "0.1235");
    assert_eq!(display_number(-0.0479), "-0.0479");
    assert_eq!(display_number(0.99996), "1.00");
    assert_eq!(display_number(0.000_123_4), "0.000123");
}

#[test]
fn rounding_is_half_away_from_zero_on_the_shown_digits() {
    // The nearest doubles to these are a little under the tie.
    assert_eq!(display_number(2.675), "2.68");
    assert_eq!(display_number(1.005), "1.01");
    assert_eq!(display_number(-2.675), "-2.68");
    assert_eq!(display_number(1_234.565), "1,234.57");
}

#[test]
fn a_residue_near_zero_is_zero_without_a_sign() {
    assert_eq!(display_number(-1.2e-10), "0.00");
    assert_eq!(display_number(4e-9), "0.00");
    assert_eq!(display_number(-0.0), "0.00");
}

#[test]
fn numbers_that_are_not_finite_say_so() {
    assert_eq!(display_number(f64::NAN), "NaN");
    assert_eq!(display_number(f64::INFINITY), "inf");
}

#[test]
fn groups_with_the_same_keys_share_a_table_aligned_by_key() {
    let report = report_of(OPTIONS, &ReportOptions::default());
    assert_eq!(report.sections.len(), 2);
    let first = &report.sections[0];
    assert_eq!(first.keys, ["rent", "utilities", "year1_monthly"]);
    assert_eq!(first.rows[0].group, "oak_suite");
    // elm_loft lists utilities before rent; its cells follow the section's keys.
    assert_eq!(
        first.rows[1].cells,
        [Cell::Value(1390.0), Cell::Value(90.0), Cell::Value(1480.0)]
    );
    assert_eq!(report.sections[1].keys, ["rate"]);
    assert_eq!(report.sections[1].rows[0].group, "assumptions");
}

#[test]
fn groups_with_other_keys_get_their_own_table_in_file_order() {
    let yaml = r#"_forge_version: "5.1.0"
a:
  x: 1
  y: 2
b:
  z: 3
c:
  y: 4
  x: 5
d:
  x: 6
"#;
    let report = report_of(yaml, &ReportOptions::default());
    let shape: Vec<(Vec<String>, Vec<String>)> = report
        .sections
        .iter()
        .map(|s| {
            (
                s.keys.clone(),
                s.rows.iter().map(|r| r.group.clone()).collect(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            (vec!["x".into(), "y".into()], vec!["a".into(), "c".into()]),
            (vec!["z".into()], vec!["b".into()]),
            (vec!["x".into()], vec!["d".into()]),
        ]
    );
}

#[test]
fn inputs_come_before_formulas() {
    let yaml = r#"_forge_version: "5.1.0"
a:
  total: {formula: "=a.x + a.y", value: 3}
  x: 1
  y: 2
"#;
    let report = report_of(yaml, &ReportOptions::default());
    assert_eq!(report.sections[0].keys, ["x", "y", "total"]);
}

#[test]
fn no_inputs_leaves_only_formulas() {
    let report = report_of(
        OPTIONS,
        &ReportOptions {
            no_inputs: true,
            ..ReportOptions::default()
        },
    );
    // assumptions has only an input, so it has no row left.
    assert_eq!(report.sections.len(), 1);
    assert_eq!(report.sections[0].keys, ["year1_monthly"]);
    assert_eq!(report.sections[0].rows.len(), 2);
}

#[test]
fn chosen_groups_are_shown_in_file_order() {
    let report = report_of(
        OPTIONS,
        &ReportOptions {
            groups: vec!["elm_loft".into(), "oak_suite".into()],
            ..ReportOptions::default()
        },
    );
    let groups: Vec<&str> = report.sections[0]
        .rows
        .iter()
        .map(|r| r.group.as_str())
        .collect();
    assert_eq!(groups, ["oak_suite", "elm_loft"]);
    assert_eq!(report.tables_not_shown, Vec::<String>::new());
}

#[test]
fn an_unknown_group_is_an_error_that_lists_the_groups() {
    let err = build_report(
        &model_of(OPTIONS),
        &ReportOptions {
            groups: vec!["unit_807".into()],
            ..ReportOptions::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("'unit_807' is not a group"), "{err}");
    assert!(err.contains("oak_suite, elm_loft, assumptions"), "{err}");
}

#[test]
fn chosen_keys_make_one_table_in_their_order() {
    let report = report_of(
        OPTIONS,
        &ReportOptions {
            keys: vec!["year1_monthly".into(), "rent".into(), "rate".into()],
            ..ReportOptions::default()
        },
    );
    assert_eq!(report.sections.len(), 1);
    let section = &report.sections[0];
    assert_eq!(section.keys, ["year1_monthly", "rent", "rate"]);
    assert_eq!(
        section.rows[0].cells,
        [Cell::Value(1510.0), Cell::Value(1450.0), Cell::Absent]
    );
    assert_eq!(
        section.rows[2].cells,
        [Cell::Absent, Cell::Absent, Cell::Value(0.0479)]
    );
}

#[test]
fn an_unknown_key_is_an_error_that_lists_the_keys() {
    let err = build_report(
        &model_of(OPTIONS),
        &ReportOptions {
            keys: vec!["deposit".into()],
            ..ReportOptions::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("'deposit' is not a key"), "{err}");
    assert!(
        err.contains("rent, utilities, year1_monthly, rate"),
        "{err}"
    );
}

const UNCALCULATED: &str = r#"_forge_version: "5.1.0"
a:
  x: 1
  total: {formula: "=a.x * 2"}
summary:
  rate: 0.05
  t: {formula: "=summary.rate * 2"}
items:
  amount: [1, 2]
"#;

#[test]
fn formulas_without_a_value_are_named() {
    let report = report_of(UNCALCULATED, &ReportOptions::default());
    assert_eq!(report.sections[0].rows[0].cells[1], Cell::Uncalculated);
    assert_eq!(report.uncalculated, ["a.total", "summary.t"]);
    assert_eq!(report.tables_not_shown, ["items"]);
    let notes = notes(&report, Path::new("m.yaml"));
    assert_eq!(
        notes,
        [
            "Note: 2 formulas have no value yet and are shown as —; run `forge calculate m.yaml` first (a.total, summary.t)",
            "Note: tables are not shown (items); `forge export` writes them to Excel",
        ]
    );
}

#[test]
fn top_level_scalars_are_one_row() {
    let yaml = r#"_forge_version: "5.1.0"
price: 16
quantity: 3
"#;
    let report = report_of(yaml, &ReportOptions::default());
    assert_eq!(report.sections[0].rows[0].group, "");
    assert!(render(&report, ReportFormat::Table)
        .starts_with("group        price  quantity\n(top level)"));
}

#[test]
fn a_file_with_no_scalars_says_so() {
    let yaml = r#"_forge_version: "5.1.0"
items:
  amount: [1, 2]
"#;
    let report = report_of(yaml, &ReportOptions::default());
    assert_eq!(report.sections, Vec::<ReportSection>::new());
    assert_eq!(render(&report, ReportFormat::Table), "");
    assert_eq!(
        notes(&report, Path::new("m.yaml"))[0],
        "No groups of scalars to report in m.yaml"
    );
}

#[test]
fn table_format() {
    let report = report_of(OPTIONS, &ReportOptions::default());
    assert_eq!(
        render(&report, ReportFormat::Table),
        "\
group          rent  utilities  year1_monthly
oak_suite  1,450.00      60.00       1,510.00
elm_loft   1,390.00      90.00       1,480.00

group          rate
assumptions  0.0479
"
    );
}

#[test]
fn markdown_format() {
    let report = report_of(OPTIONS, &ReportOptions::default());
    assert_eq!(
        render(&report, ReportFormat::Markdown),
        "\
| group | rent | utilities | year1_monthly |
| --- | ---: | ---: | ---: |
| oak_suite | 1,450.00 | 60.00 | 1,510.00 |
| elm_loft | 1,390.00 | 90.00 | 1,480.00 |

| group | rate |
| --- | ---: |
| assumptions | 0.0479 |
"
    );
}

#[test]
fn markdown_is_one_pipe_table_per_key_set() {
    let text = render(
        &report_of(UNCALCULATED, &ReportOptions::default()),
        ReportFormat::Markdown,
    );
    let tables: Vec<&str> = text.trim_end().split("\n\n").collect();
    assert_eq!(tables.len(), 2);
    for table in tables {
        let lines: Vec<&str> = table.lines().collect();
        let cells = |line: &str| line.matches(" | ").count();
        assert!(lines.len() >= 3, "{table}");
        assert!(lines[1].starts_with("| --- |"), "{table}");
        for line in &lines {
            assert!(line.starts_with("| ") && line.ends_with(" |"), "{line}");
            assert_eq!(cells(line), cells(lines[0]), "{table}");
        }
    }
    assert!(text.contains("| a | 1.00 | — |"), "{text}");
}

#[test]
fn csv_format_has_full_precision_and_no_separators() {
    let yaml = r#"_forge_version: "5.1.0"
"a,b":
  big: 1234567.891
  rate: 0.0479
  total: {formula: "=1"}
"#;
    let report = report_of(yaml, &ReportOptions::default());
    assert_eq!(
        render(&report, ReportFormat::Csv),
        "group,big,rate,total\n\"a,b\",1234567.891,0.0479,\n"
    );
}

#[test]
fn csv_sections_are_separated_by_a_blank_line() {
    let report = report_of(OPTIONS, &ReportOptions::default());
    assert_eq!(
        render(&report, ReportFormat::Csv),
        "\
group,rent,utilities,year1_monthly
oak_suite,1450,60,1510
elm_loft,1390,90,1480

group,rate
assumptions,0.0479
"
    );
}

#[test]
fn json_format() {
    let report = report_of(UNCALCULATED, &ReportOptions::default());
    let value: serde_json::Value =
        serde_json::from_str(&render(&report, ReportFormat::Json)).expect("json");
    assert_eq!(
        value,
        serde_json::json!({
            "sections": [
                {"keys": ["x", "total"], "rows": [{"group": "a", "values": {"x": 1.0, "total": null}}]},
                {"keys": ["rate", "t"], "rows": [{"group": "summary", "values": {"rate": 0.05, "t": null}}]},
            ],
            "uncalculated": ["a.total", "summary.t"],
            "tables_not_shown": ["items"],
        })
    );
}

#[test]
fn json_leaves_out_keys_a_group_does_not_have() {
    let report = report_of(
        OPTIONS,
        &ReportOptions {
            keys: vec!["rent".into(), "rate".into()],
            ..ReportOptions::default()
        },
    );
    let value: serde_json::Value =
        serde_json::from_str(&render(&report, ReportFormat::Json)).expect("json");
    assert_eq!(
        value["sections"][0]["rows"][2],
        serde_json::json!({"group": "assumptions", "values": {"rate": 0.0479}})
    );
}

#[test]
fn formats_are_read_by_name() {
    assert_eq!(
        "markdown".parse::<ReportFormat>().unwrap(),
        ReportFormat::Markdown
    );
    assert!("html".parse::<ReportFormat>().is_err());
}
