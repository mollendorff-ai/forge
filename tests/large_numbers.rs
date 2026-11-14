//! A result that is a whole number too large for a 64-bit integer
//! (from about 9.2e18) is written back to the model so that it reads as the
//! same number. `forge calculate` wrote it as digits alone, which YAML reads
//! as an integer: the edited file did not parse, and Forge refused to write
//! any result ("results not written, the file is unchanged"). Found by
//! tests/properties.rs (`calculating_again_changes_nothing`).

use mollendorff_forge::cli::calculate_core;

#[test]
#[allow(clippy::float_cmp)] // the written number must read back exactly
fn a_whole_number_past_i64_is_written_and_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.yaml");
    std::fs::write(
        &path,
        "_forge_version: \"5.1.0\"\ns:\n  x: 10000000000\n  big: {formula: \"=s.x * s.x\"}\n  neg: {formula: \"=-s.x * 999999999.999999\"}\n",
    )
    .unwrap();
    let first = calculate_core(&path, false, None).unwrap();
    assert_eq!(first.scalars["s.big"], Some(1e20));
    let written = std::fs::read_to_string(&path).unwrap();
    // The written file calculates again to the same numbers
    let again = calculate_core(&path, true, None).unwrap();
    assert_eq!(again.scalars["s.big"], Some(1e20), "{written}");
    assert_eq!(again.scalars["s.neg"], first.scalars["s.neg"], "{written}");
}
