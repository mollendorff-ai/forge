//! Utils tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI
#![allow(clippy::unreadable_literal)] // Test literals are intentionally exact

use super::super::*;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_format_number_integer() {
    assert_eq!(format_number(100.0), "100");
    assert_eq!(format_number(0.0), "0");
    assert_eq!(format_number(-50.0), "-50");
}

#[test]
fn test_format_number_decimal() {
    assert_eq!(format_number(3.14), "3.14");
    assert_eq!(format_number(0.5), "0.5");
    assert_eq!(format_number(-2.75), "-2.75");
}

#[test]
fn test_format_number_removes_trailing_zeros() {
    assert_eq!(format_number(1.10), "1.1");
    assert_eq!(format_number(2.500), "2.5");
    assert_eq!(format_number(10.000), "10");
}

#[test]
fn test_format_number_precision() {
    // Rounds to 6 decimal places
    assert_eq!(format_number(0.123456789), "0.123457");
    assert_eq!(format_number(1.0000001), "1");
}

#[test]
fn test_format_number_very_small() {
    assert_eq!(format_number(0.000001), "0.000001");
    assert_eq!(format_number(0.0000001), "0");
}

#[test]
fn test_format_number_large() {
    assert_eq!(format_number(1000000.0), "1000000");
    // 999999.999999 stays as-is since it's within 6 decimal precision
    assert_eq!(format_number(999999.999999), "999999.999999");
    // Very small differences beyond 6 decimals get rounded
    assert_eq!(format_number(1000000.0000001), "1000000");
}

#[test]
fn test_chrono_lite_timestamp_format() {
    let ts = chrono_lite_timestamp();
    // Format should be "HH:MM:SS UTC" (12 chars)
    assert_eq!(ts.len(), 12);
    assert!(ts.contains(':'));
    assert!(ts.ends_with(" UTC"));
}

#[test]
fn test_chrono_lite_timestamp_valid_time() {
    let ts = chrono_lite_timestamp();
    // Parse the HH:MM:SS part
    let time_part = &ts[..8];
    let parts: Vec<&str> = time_part.split(':').collect();
    assert_eq!(parts.len(), 3);

    let hours: u32 = parts[0].parse().unwrap();
    let minutes: u32 = parts[1].parse().unwrap();
    let seconds: u32 = parts[2].parse().unwrap();

    assert!(hours < 24);
    assert!(minutes < 60);
    assert!(seconds < 60);
}
