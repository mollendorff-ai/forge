//! Unit validator integration tests
//! ADR-004: 100% coverage required

use royalbit_forge::core::unit_validator::{UnitCategory, UnitWarning, WarningSeverity};

// ═══════════════════════════════════════════════════════════════════════════
// UNIT CATEGORY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_unit_category_parse_currencies() {
    // Test all currency formats
    assert!(matches!(
        UnitCategory::parse("CAD"),
        UnitCategory::Currency(_)
    ));
    assert!(matches!(
        UnitCategory::parse("cad"),
        UnitCategory::Currency(_)
    ));
    assert!(matches!(
        UnitCategory::parse("USD"),
        UnitCategory::Currency(_)
    ));
    assert!(matches!(
        UnitCategory::parse("EUR"),
        UnitCategory::Currency(_)
    ));
    assert!(matches!(
        UnitCategory::parse("GBP"),
        UnitCategory::Currency(_)
    ));
    assert!(matches!(
        UnitCategory::parse("JPY"),
        UnitCategory::Currency(_)
    ));
    assert!(matches!(
        UnitCategory::parse("CNY"),
        UnitCategory::Currency(_)
    ));
    assert!(matches!(
        UnitCategory::parse("$"),
        UnitCategory::Currency(_)
    ));
}

#[test]
fn test_unit_category_parse_percentage() {
    assert!(matches!(UnitCategory::parse("%"), UnitCategory::Percentage));
    assert!(matches!(
        UnitCategory::parse("percent"),
        UnitCategory::Percentage
    ));
    assert!(matches!(
        UnitCategory::parse("percentage"),
        UnitCategory::Percentage
    ));
    assert!(matches!(
        UnitCategory::parse("PERCENT"),
        UnitCategory::Percentage
    ));
}

#[test]
fn test_unit_category_parse_count() {
    assert!(matches!(UnitCategory::parse("count"), UnitCategory::Count));
    assert!(matches!(UnitCategory::parse("units"), UnitCategory::Count));
    assert!(matches!(UnitCategory::parse("items"), UnitCategory::Count));
    assert!(matches!(UnitCategory::parse("qty"), UnitCategory::Count));
    assert!(matches!(
        UnitCategory::parse("quantity"),
        UnitCategory::Count
    ));
    assert!(matches!(
        UnitCategory::parse("QUANTITY"),
        UnitCategory::Count
    ));
}

#[test]
fn test_unit_category_parse_time() {
    assert!(matches!(UnitCategory::parse("days"), UnitCategory::Time(_)));
    assert!(matches!(UnitCategory::parse("day"), UnitCategory::Time(_)));
    assert!(matches!(UnitCategory::parse("d"), UnitCategory::Time(_)));
    assert!(matches!(
        UnitCategory::parse("months"),
        UnitCategory::Time(_)
    ));
    assert!(matches!(
        UnitCategory::parse("month"),
        UnitCategory::Time(_)
    ));
    assert!(matches!(UnitCategory::parse("mo"), UnitCategory::Time(_)));
    assert!(matches!(
        UnitCategory::parse("years"),
        UnitCategory::Time(_)
    ));
    assert!(matches!(UnitCategory::parse("year"), UnitCategory::Time(_)));
    assert!(matches!(UnitCategory::parse("yr"), UnitCategory::Time(_)));
    assert!(matches!(
        UnitCategory::parse("hours"),
        UnitCategory::Time(_)
    ));
    assert!(matches!(UnitCategory::parse("hour"), UnitCategory::Time(_)));
    assert!(matches!(UnitCategory::parse("hr"), UnitCategory::Time(_)));
}

#[test]
fn test_unit_category_parse_ratio() {
    assert!(matches!(UnitCategory::parse("ratio"), UnitCategory::Ratio));
    assert!(matches!(UnitCategory::parse("factor"), UnitCategory::Ratio));
    assert!(matches!(
        UnitCategory::parse("multiplier"),
        UnitCategory::Ratio
    ));
    assert!(matches!(UnitCategory::parse("x"), UnitCategory::Ratio));
}

#[test]
fn test_unit_category_parse_unknown() {
    assert!(matches!(
        UnitCategory::parse("foobar"),
        UnitCategory::Unknown
    ));
    assert!(matches!(
        UnitCategory::parse("custom_unit"),
        UnitCategory::Unknown
    ));
    assert!(matches!(UnitCategory::parse("xyz"), UnitCategory::Unknown));
}

#[test]
fn test_unit_category_display() {
    assert_eq!(UnitCategory::Currency("CAD".to_string()).display(), "CAD");
    assert_eq!(UnitCategory::Percentage.display(), "%");
    assert_eq!(UnitCategory::Count.display(), "count");
    assert_eq!(UnitCategory::Time("days".to_string()).display(), "days");
    assert_eq!(UnitCategory::Ratio.display(), "ratio");
    assert_eq!(UnitCategory::Unknown.display(), "unknown");
}

#[test]
fn test_unit_category_eq() {
    let cad1 = UnitCategory::Currency("CAD".to_string());
    let cad2 = UnitCategory::Currency("CAD".to_string());
    let usd = UnitCategory::Currency("USD".to_string());

    assert_eq!(cad1, cad2);
    assert_ne!(cad1, usd);
    assert_eq!(UnitCategory::Percentage, UnitCategory::Percentage);
    assert_ne!(UnitCategory::Percentage, UnitCategory::Ratio);
}

#[test]
fn test_unit_category_clone() {
    let original = UnitCategory::Currency("EUR".to_string());
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn test_unit_category_hash() {
    use std::collections::HashMap;

    let mut map: HashMap<UnitCategory, i32> = HashMap::new();
    map.insert(UnitCategory::Currency("CAD".to_string()), 1);
    map.insert(UnitCategory::Percentage, 2);

    assert_eq!(
        map.get(&UnitCategory::Currency("CAD".to_string())),
        Some(&1)
    );
    assert_eq!(map.get(&UnitCategory::Percentage), Some(&2));
}

#[test]
fn test_unit_category_debug() {
    let category = UnitCategory::Currency("USD".to_string());
    let debug_str = format!("{:?}", category);
    assert!(debug_str.contains("Currency"));
    assert!(debug_str.contains("USD"));
}

// ═══════════════════════════════════════════════════════════════════════════
// WARNING SEVERITY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_warning_severity_eq() {
    assert_eq!(WarningSeverity::Warning, WarningSeverity::Warning);
    assert_eq!(WarningSeverity::Error, WarningSeverity::Error);
    assert_ne!(WarningSeverity::Warning, WarningSeverity::Error);
}

#[test]
fn test_warning_severity_clone() {
    let original = WarningSeverity::Warning;
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn test_warning_severity_debug() {
    let warning = WarningSeverity::Warning;
    let error = WarningSeverity::Error;
    assert!(format!("{:?}", warning).contains("Warning"));
    assert!(format!("{:?}", error).contains("Error"));
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT WARNING TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_unit_warning_display_warning() {
    let warning = UnitWarning {
        location: "table.column".to_string(),
        formula: "=price + rate".to_string(),
        message: "Mixing units".to_string(),
        severity: WarningSeverity::Warning,
    };

    let display = format!("{}", warning);
    assert!(display.contains("Warning"));
    assert!(display.contains("table.column"));
    assert!(display.contains("Mixing units"));
    assert!(display.contains("=price + rate"));
}

#[test]
fn test_unit_warning_display_error() {
    let warning = UnitWarning {
        location: "scalars.profit".to_string(),
        formula: "=revenue + cost".to_string(),
        message: "Type mismatch".to_string(),
        severity: WarningSeverity::Error,
    };

    let display = format!("{}", warning);
    assert!(display.contains("Error"));
    assert!(display.contains("scalars.profit"));
}

#[test]
fn test_unit_warning_clone() {
    let warning = UnitWarning {
        location: "test".to_string(),
        formula: "=x+y".to_string(),
        message: "test message".to_string(),
        severity: WarningSeverity::Warning,
    };

    let cloned = warning.clone();
    assert_eq!(cloned.location, "test");
    assert_eq!(cloned.formula, "=x+y");
    assert_eq!(cloned.message, "test message");
    assert_eq!(cloned.severity, WarningSeverity::Warning);
}

#[test]
fn test_unit_warning_debug() {
    let warning = UnitWarning {
        location: "loc".to_string(),
        formula: "=a".to_string(),
        message: "msg".to_string(),
        severity: WarningSeverity::Warning,
    };

    let debug_str = format!("{:?}", warning);
    assert!(debug_str.contains("UnitWarning"));
    assert!(debug_str.contains("loc"));
}
