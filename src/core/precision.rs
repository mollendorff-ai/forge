//! The precision of the numbers Forge writes and prints.
//!
//! The engine calculates in full `f64` precision, about 17 significant digits,
//! and keeps it between formulas. The last one or two of those digits are
//! binary rounding noise: `0.1 + 0.2` is `0.30000000000000004`, and a sum of
//! prices like `418.33` comes out as `418.3299999999999`. Excel calculates the
//! same way and shows at most 15 significant digits, which hides the noise.
//!
//! Forge does the same at its edges: a number written to a file or printed is
//! rounded to 15 significant digits, so `418.3299999999999` is stored as
//! `418.33`. The calculation itself is not rounded, except where Excel rounds
//! it: a formula's last `+` or `-`, and the last addition of `SUM`, are 0 when
//! they nearly cancel ([`final_sum`], ADR-066).

use crate::core::array_calculator::evaluator::text::number_format::rounded_digits;
use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};

/// The significant digits a written or printed number keeps, as in Excel.
pub const SIGNIFICANT_DIGITS: usize = 15;

/// Whether `a` and `b` are the same number, compared exactly as Excel does.
///
/// It is IEEE 754 equality: `0` and `-0` are equal, and a NaN is equal to
/// nothing. For where exactness is the rule and a tolerance would
/// change the answer: `MODE` counts equal values, a line on budget has no
/// variance, a depreciation period is the first or the last.
#[must_use]
pub fn same(a: f64, b: f64) -> bool {
    a.partial_cmp(&b) == Some(std::cmp::Ordering::Equal)
}

/// 2^32, the weight of the high half of a 64-bit whole number
const HALF: f64 = 4_294_967_296.0;

/// A whole number of 64 bits as a number: each 32-bit half is exact, and the
/// one rounding is the sum, to nearest with ties to even, as `n as f64`.
fn from_u64(n: u64) -> f64 {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = n.to_le_bytes();
    f64::from(u32::from_le_bytes([b4, b5, b6, b7]))
        .mul_add(HALF, f64::from(u32::from_le_bytes([b0, b1, b2, b3])))
}

/// A count (a length, a place in an array, a number of draws) as a number.
///
/// Exact up to 2^53, about 9e15, which no array in memory reaches; above that
/// the nearest number, ties to even, as IEEE 754 rounds. It is the same number
/// `count as f64` gives, built from conversions that cannot lose anything, so
/// that the one place where a count becomes a number is named and tested.
#[must_use]
pub fn count(n: usize) -> f64 {
    let bytes = n.to_le_bytes();
    let mut wide = [0_u8; 8];
    wide[..bytes.len()].copy_from_slice(&bytes);
    from_u64(u64::from_le_bytes(wide))
}

/// A whole number of 64 bits (a day count, a random integer) as a number:
/// exact up to 2^53, the nearest number above it, as [`count`] does.
#[must_use]
pub fn whole(n: i64) -> f64 {
    let magnitude = from_u64(n.unsigned_abs());
    if n < 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// An unsigned whole number of 64 bits as a number, as [`whole`] does.
#[must_use]
pub fn unsigned(n: u64) -> f64 {
    from_u64(n)
}

/// How Excel's comparison operators order two numbers: each rounded to 15
/// significant digits first (ADR-065).
///
/// So `0.1 + 0.2 = 0.3` and `100 * 1.1 <= 110` hold, while `1 + 1E-14 > 1`
/// and `-1E-12 < 0` do too. The rounding is of each number, relative to its
/// own size: there is no tolerance near zero. `None` when either is a NaN.
///
/// For `=`, `<>`, `<`, `<=`, `>`, `>=`, `SWITCH` and the criteria of
/// `COUNTIF` and its family. An exact lookup (`MATCH` 0, `XLOOKUP`) compares
/// exactly instead, with [`same`].
#[must_use]
pub fn excel_order(a: f64, b: f64) -> Option<std::cmp::Ordering> {
    let exact = a.partial_cmp(&b);
    // Rounding moves a number by at most half a unit in its 15th digit, under
    // 5E-15 of it, and never past another, so two numbers further apart than
    // 1E-13 of the larger keep their order; only nearer ones are rounded
    if exact == Some(std::cmp::Ordering::Equal) || (a - b).abs() > 1e-13 * a.abs().max(b.abs()) {
        return exact;
    }
    to_15_digits(a).partial_cmp(&to_15_digits(b))
}

/// Whether `a` and `b` are equal as Excel's `=` finds them: [`excel_order`].
#[must_use]
pub fn excel_equal(a: f64, b: f64) -> bool {
    excel_order(a, b) == Some(std::cmp::Ordering::Equal)
}

/// The result of a formula's last `+` or `-`, as Excel gives it (ADR-066).
///
/// It is 0 when `result` is less than 8 units in the last place of the larger
/// of the two operands `a` and `b`, which can only happen when they nearly
/// cancel. `0.1 + 0.2 - 0.3` is 5.55E-17, a unit in the last place of
/// 0.30000000000000004, and so 0; `1E15 + 1 - 1E15` is 1, 8 units of
/// 1000000000000001, and stays 1. Excel for iPhone draws the line there on
/// every case checked, near 1E-10, 1, 512 and 1E15 (2026-11-18).
///
/// The unit is the larger operand's: 2^e for the power of two at or below
/// it, times 2^-52. Adding 0, or numbers of one sign, never falls below 8 of
/// them; a subnormal operand has no such unit, and nothing is changed.
#[must_use]
pub fn final_sum(a: f64, b: f64, result: f64) -> f64 {
    // The power of two at or below the larger operand: its exponent bits alone
    let larger = a.abs().max(b.abs());
    let power = f64::from_bits(larger.to_bits() & 0x7ff0_0000_0000_0000);
    // 8 units in the last place of `power` is power * 2^-49
    let line = if larger.is_normal() {
        power * EIGHT_UNITS
    } else {
        0.0
    };
    if result.abs() < line {
        0.0
    } else {
        result
    }
}

/// 2^-49: 8 units in the last place of 1
const EIGHT_UNITS: f64 = 1.0 / 562_949_953_421_312.0;

/// `value` rounded to 15 significant digits: the number Forge writes and
/// prints. Zero, infinities and NaN are returned as they are.
#[must_use]
pub fn to_15_digits(value: f64) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    // `{:.14e}` is correctly rounded: one digit before the point and fourteen
    // after it. The nearest `f64` to that decimal prints back as it.
    format!("{value:.14e}").parse().unwrap_or(value)
}

/// The number in `text` to its first 15 significant digits, the rest dropped.
///
/// It is how Excel reads a number typed in a formula or turned from text
/// (`VALUE`), not rounded: `1234567890123456` is `1234567890123450`, and
/// `0.1234567890123456` is `0.123456789012345` (Excel for iPhone,
/// 2026-11-24). A value stored in a workbook keeps every digit, in Excel as
/// in Forge; this is only for numbers read from text.
///
/// # Errors
///
/// Returns `f64`'s parse error when `text` is no number.
pub fn read_typed(text: &str) -> Result<f64, std::num::ParseFloatError> {
    let mut significant = 0;
    let mut exponent = false;
    let digits: String = text
        .chars()
        .map(|c| {
            exponent = exponent || matches!(c, 'e' | 'E');
            // Zeros before the first other digit do not count
            if exponent || !c.is_ascii_digit() || significant == 0 && c == '0' {
                return c;
            }
            significant += 1;
            if significant > SIGNIFICANT_DIGITS {
                '0'
            } else {
                c
            }
        })
        .collect();
    digits.parse()
}

/// An amount of money as the commands print it: `$61,500.00`.
///
/// A loss is `-$117,000.00`. Two decimals, thousands separators, the sign in
/// front of the `$`, and the digits a spreadsheet shows (15 significant,
/// half away from zero).
#[must_use]
pub fn money(amount: f64) -> String {
    money_with(amount, 2)
}

/// [`money`] in whole dollars, for a rule of thumb or a sentence:
/// `$198,000`.
#[must_use]
pub fn money_whole(amount: f64) -> String {
    money_with(amount, 0)
}

fn money_with(amount: f64, decimals: usize) -> String {
    if !amount.is_finite() {
        return format!("${amount}");
    }
    let (whole, fraction) = rounded_digits(amount.abs(), decimals);
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let zero = whole == "0" && fraction.bytes().all(|b| b == b'0');
    let sign = if amount < 0.0 && !zero { "-" } else { "" };
    if decimals == 0 {
        format!("{sign}${grouped}")
    } else {
        format!("{sign}${grouped}.{fraction}")
    }
}

/// A value serialized with every `f64` in it, at any depth, rounded to 15
/// significant digits.
///
/// `serde_json::to_string_pretty(&Rounded(&result))`. Nothing else changes (keys, their order, integers, text), so a result written to a
/// JSON or YAML file reads as it did, without the noise.
pub struct Rounded<'a, T: ?Sized>(pub &'a T);

impl<T: Serialize + ?Sized> Serialize for Rounded<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(RoundingSerializer(serializer))
    }
}

/// For `#[serde(serialize_with = "...")]`: the field serialized as
/// [`Rounded`].
///
/// # Errors
///
/// Returns the serializer's error.
pub fn serialize_rounded<T: Serialize + ?Sized, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    Rounded(value).serialize(serializer)
}

/// A serializer that rounds each `f64` and passes everything else on.
struct RoundingSerializer<S>(S);

/// A compound (sequence, map, struct...) whose elements are [`Rounded`].
struct Compound<C>(C);

impl<S: Serializer> Serializer for RoundingSerializer<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Compound<S::SerializeSeq>;
    type SerializeTuple = Compound<S::SerializeTuple>;
    type SerializeTupleStruct = Compound<S::SerializeTupleStruct>;
    type SerializeTupleVariant = Compound<S::SerializeTupleVariant>;
    type SerializeMap = Compound<S::SerializeMap>;
    type SerializeStruct = Compound<S::SerializeStruct>;
    type SerializeStructVariant = Compound<S::SerializeStructVariant>;

    fn serialize_f64(self, v: f64) -> Result<S::Ok, S::Error> {
        self.0.serialize_f64(to_15_digits(v))
    }
    fn serialize_f32(self, v: f32) -> Result<S::Ok, S::Error> {
        self.0.serialize_f32(v)
    }
    fn serialize_bool(self, v: bool) -> Result<S::Ok, S::Error> {
        self.0.serialize_bool(v)
    }
    fn serialize_i8(self, v: i8) -> Result<S::Ok, S::Error> {
        self.0.serialize_i8(v)
    }
    fn serialize_i16(self, v: i16) -> Result<S::Ok, S::Error> {
        self.0.serialize_i16(v)
    }
    fn serialize_i32(self, v: i32) -> Result<S::Ok, S::Error> {
        self.0.serialize_i32(v)
    }
    fn serialize_i64(self, v: i64) -> Result<S::Ok, S::Error> {
        self.0.serialize_i64(v)
    }
    fn serialize_i128(self, v: i128) -> Result<S::Ok, S::Error> {
        self.0.serialize_i128(v)
    }
    fn serialize_u8(self, v: u8) -> Result<S::Ok, S::Error> {
        self.0.serialize_u8(v)
    }
    fn serialize_u16(self, v: u16) -> Result<S::Ok, S::Error> {
        self.0.serialize_u16(v)
    }
    fn serialize_u32(self, v: u32) -> Result<S::Ok, S::Error> {
        self.0.serialize_u32(v)
    }
    fn serialize_u64(self, v: u64) -> Result<S::Ok, S::Error> {
        self.0.serialize_u64(v)
    }
    fn serialize_u128(self, v: u128) -> Result<S::Ok, S::Error> {
        self.0.serialize_u128(v)
    }
    fn serialize_char(self, v: char) -> Result<S::Ok, S::Error> {
        self.0.serialize_char(v)
    }
    fn serialize_str(self, v: &str) -> Result<S::Ok, S::Error> {
        self.0.serialize_str(v)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<S::Ok, S::Error> {
        self.0.serialize_bytes(v)
    }
    fn serialize_none(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_none()
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_some(&Rounded(value))
    }
    fn serialize_unit(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit()
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_struct(name)
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_variant(name, index, variant)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_newtype_struct(name, &Rounded(value))
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<S::Ok, S::Error> {
        self.0
            .serialize_newtype_variant(name, index, variant, &Rounded(value))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, S::Error> {
        self.0.serialize_seq(len).map(Compound)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, S::Error> {
        self.0.serialize_tuple(len).map(Compound)
    }
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, S::Error> {
        self.0.serialize_tuple_struct(name, len).map(Compound)
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, S::Error> {
        self.0
            .serialize_tuple_variant(name, index, variant, len)
            .map(Compound)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, S::Error> {
        self.0.serialize_map(len).map(Compound)
    }
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, S::Error> {
        self.0.serialize_struct(name, len).map(Compound)
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, S::Error> {
        self.0
            .serialize_struct_variant(name, index, variant, len)
            .map(Compound)
    }
    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

impl<C: SerializeSeq> SerializeSeq for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_element(&Rounded(value))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeTuple> SerializeTuple for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_element(&Rounded(value))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeTupleStruct> SerializeTupleStruct for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_field(&Rounded(value))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeTupleVariant> SerializeTupleVariant for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_field(&Rounded(value))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeMap> SerializeMap for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), C::Error> {
        self.0.serialize_key(&Rounded(key))
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_value(&Rounded(value))
    }
    fn serialize_entry<K: Serialize + ?Sized, V: Serialize + ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), C::Error> {
        self.0.serialize_entry(&Rounded(key), &Rounded(value))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeStruct> SerializeStruct for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), C::Error> {
        self.0.serialize_field(key, &Rounded(value))
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), C::Error> {
        self.0.skip_field(key)
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeStructVariant> SerializeStructVariant for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), C::Error> {
        self.0.serialize_field(key, &Rounded(value))
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), C::Error> {
        self.0.skip_field(key)
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering::{Equal, Greater, Less};

    /// Excel's last addition (ADR-066): 0 under 8 units in the last place of
    /// the larger operand, the result itself from 8 on
    #[test]
    fn a_last_sum_under_eight_units_is_zero() {
        let unit = 2f64.powi(-52);
        let (two, seven, eight) = (2.0 * unit, 7.0 * unit, 8.0 * unit);
        assert_exact!(final_sum(1.0 + seven, -1.0, seven), 0.0);
        assert_exact!(final_sum(1.0 + eight, -1.0, eight), eight);
        // Below zero alike
        assert_exact!(final_sum(-1.0 - two, 1.0, -two), 0.0);
        // The larger operand's unit: 1E15 + 1 is 8 units of 1000000000000001
        assert_exact!(final_sum(1e15 + 1.0, -1e15, 1.0), 1.0);
        assert_exact!(final_sum(1e15 + 0.5, -1e15, 0.5), 0.0);
        // Nothing cancels: adding 0, or numbers of one sign
        assert_exact!(final_sum(5e-17, 0.0, 5e-17), 5e-17);
        assert_exact!(final_sum(1.0, unit, 1.0 + unit), 1.0 + unit);
        assert_exact!(final_sum(0.0, 0.0, 0.0), 0.0);
        // A subnormal operand has no such unit, and an infinity is kept
        let tiny = f64::MIN_POSITIVE / 4.0;
        assert_exact!(final_sum(tiny, -tiny / 2.0, tiny / 2.0), tiny / 2.0);
        assert!(final_sum(f64::INFINITY, 1.0, f64::INFINITY).is_infinite());
    }

    /// A number read from text keeps its first 15 significant digits and
    /// drops the rest, in the whole part, the fraction and before an
    /// exponent, as Excel reads one typed in a formula (2026-11-24)
    #[test]
    fn a_number_read_from_text_keeps_fifteen_significant_digits() {
        for (text, excel) in [
            ("1234567890123456", 1_234_567_890_123_450.0),
            ("1234567890123459.9", 1_234_567_890_123_450.0),
            ("-1234567890123456", -1_234_567_890_123_450.0),
            ("12345678901234567890", 12_345_678_901_234_500_000.0),
            ("0.1234567890123456", 0.123_456_789_012_345),
            // Zeros before the first other digit are not significant
            ("0.000001234567890123456", 0.000_001_234_567_890_123_45),
            ("1.2345678901234567E20", 1.234_567_890_123_45E20),
            ("1.2345678901234567e-5", 1.234_567_890_123_45e-5),
            // Fifteen digits or fewer are read as they are
            ("123456789012345", 123_456_789_012_345.0),
            ("0.1", 0.1),
            ("1E15", 1e15),
            ("007", 7.0),
        ] {
            assert_exact!(read_typed(text).unwrap(), excel);
        }
        assert!(read_typed("1.2.3").is_err());
        assert!(read_typed("").is_err());
    }

    /// A count becomes the number IEEE 754 gives it, at every edge
    #[test]
    fn a_whole_number_becomes_a_number_as_ieee_rounds_it() {
        assert_exact!(count(0), 0.0);
        assert_exact!(count(1), 1.0);
        assert_exact!(count(4_294_967_295), 4_294_967_295.0);
        assert_exact!(count(4_294_967_296), 4_294_967_296.0);
        assert_exact!(unsigned(9_007_199_254_740_991), 9_007_199_254_740_991.0);
        assert_exact!(unsigned(9_007_199_254_740_992), 9_007_199_254_740_992.0);
        // Above 2^53 the spacing is 2: a tie goes to the even neighbour
        assert_exact!(unsigned(9_007_199_254_740_993), 9_007_199_254_740_992.0);
        assert_exact!(unsigned(9_007_199_254_740_995), 9_007_199_254_740_996.0);
        // Not a tie: the nearer neighbour
        assert_exact!(unsigned(9_007_199_254_740_994), 9_007_199_254_740_994.0);
        assert_exact!(unsigned(u64::MAX), 18_446_744_073_709_551_616.0);
        // A count is as wide as a usize, which fills 64 bits or fewer
        assert_exact!(count(usize::MAX), unsigned(u64::MAX >> (64 - usize::BITS)));
        assert_exact!(count(123_456_789), 123_456_789.0);
    }

    /// A signed whole number keeps its sign, and the rounding is symmetric
    #[test]
    fn a_signed_whole_number_becomes_a_number_with_its_sign() {
        assert_exact!(whole(0), 0.0);
        assert_exact!(whole(45_000), 45_000.0);
        assert_exact!(whole(-45_000), -45_000.0);
        assert_exact!(whole(-9_007_199_254_740_993), -9_007_199_254_740_992.0);
        assert_exact!(whole(9_007_199_254_740_995), 9_007_199_254_740_996.0);
        assert_exact!(whole(i64::MAX), 9_223_372_036_854_775_808.0);
        assert_exact!(whole(i64::MIN), -9_223_372_036_854_775_808.0);
        // Zero has no sign to keep
        assert!(whole(0).is_sign_positive());
    }

    /// Excel's comparison: each number to 15 significant digits (ADR-065)
    #[test]
    fn excel_order_rounds_each_number_to_fifteen_digits() {
        // Exactly equal, and zero of either sign
        assert_eq!(excel_order(2.5, 2.5), Some(Equal));
        assert_eq!(excel_order(0.0, -0.0), Some(Equal));
        // Binary noise in the 17th digit
        assert_eq!(excel_order(0.1 + 0.2, 0.3), Some(Equal));
        assert_eq!(excel_order(0.3, 0.1 + 0.2), Some(Equal));
        // The 15th digit counts, at every size
        assert_eq!(excel_order(1.0 + 1e-14, 1.0), Some(Greater));
        assert_eq!(excel_order(1e-300, 1.000_000_000_000_01e-300), Some(Less));
        assert_eq!(excel_order(1e300, 1.000_000_000_000_01e300), Some(Less));
        // No tolerance near zero, and far apart is the plain order
        assert_eq!(excel_order(-1e-12, 0.0), Some(Less));
        assert_eq!(excel_order(f64::MIN_POSITIVE, 0.0), Some(Greater));
        assert_eq!(excel_order(-3.0, 2.0), Some(Less));
        // Two numbers one binary step apart on either side of a 15th digit's
        // halfway point (1.0000000000000049 and 1.000000000000005) are
        // unequal: the rule rounds each, it does not measure the gap
        assert_eq!(
            excel_order(1.000_000_000_000_004_9, 1.000_000_000_000_005),
            Some(Less)
        );
        // Infinities and NaN
        assert_eq!(excel_order(f64::INFINITY, f64::INFINITY), Some(Equal));
        assert_eq!(excel_order(f64::INFINITY, 1e308), Some(Greater));
        assert_eq!(excel_order(f64::NAN, 1.0), None);
        assert!(!excel_equal(f64::NAN, f64::NAN));
        assert!(excel_equal(100.0 * 1.1, 110.0));
    }

    /// The shortcut for numbers far apart gives what rounding both would
    #[test]
    fn excel_order_shortcut_agrees_with_rounding() {
        let values = [
            0.0,
            1e-300,
            1e-15,
            0.1,
            0.3,
            0.1 + 0.2,
            1.0,
            1.0 + 1e-14,
            1.0 + 1e-13,
            99.99,
            110.0,
            100.0 * 1.1,
            1e15,
            1e15 + 1.0,
            1e15 + 10.0,
            1e300,
        ];
        for &a in &values {
            for &b in &values {
                for (x, y) in [(a, b), (-a, b), (a, -b), (-a, -b)] {
                    assert_eq!(
                        excel_order(x, y),
                        to_15_digits(x).partial_cmp(&to_15_digits(y)),
                        "{x} and {y}"
                    );
                }
            }
        }
    }

    #[test]
    fn same_is_ieee_equality_on_every_edge() {
        let edges = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.1 + 0.2,
            0.3,
            f64::MIN_POSITIVE,
            f64::MIN_POSITIVE / 4.0,
            f64::EPSILON,
            1.0 + f64::EPSILON,
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        for a in edges {
            for b in edges {
                // IEEE equality, from the bits: the same number and not a NaN, or
                // both zeros (0 and -0 differ in their sign bit)
                let ieee = (!a.is_nan() && a.to_bits() == b.to_bits()) || (a == 0.0 && b == 0.0);
                assert_eq!(same(a, b), ieee, "{a:?} and {b:?}");
            }
        }
        assert!(same(0.0, -0.0));
        assert!(!same(f64::NAN, f64::NAN));
        assert!(!same(0.1 + 0.2, 0.3));
        assert!(same(f64::INFINITY, f64::INFINITY));
    }

    #[test]
    fn money_has_separators_two_decimals_and_the_sign_first() {
        assert_eq!(money(61_500.0), "$61,500.00");
        assert_eq!(money(-117_000.0), "-$117,000.00");
        assert_eq!(money(64_604.68), "$64,604.68");
        assert_eq!(money(279.719_999_999_999_9), "$279.72");
        assert_eq!(money(0.5), "$0.50");
        assert_eq!(money(0.0), "$0.00");
        assert_eq!(money(-0.001), "$0.00");
        assert_eq!(money(1_234_567.891), "$1,234,567.89");
        assert_eq!(money(f64::INFINITY), "$inf");
        assert_eq!(money_whole(198_000.0), "$198,000");
        assert_eq!(money_whole(64_604.68), "$64,605");
        assert_eq!(money_whole(-4_604.68), "-$4,605");
        assert_eq!(money_whole(999.5), "$1,000");
    }

    #[test]
    fn rounds_the_noise_away() {
        assert_exact!(to_15_digits(0.1 + 0.2), 0.3);
        assert_eq!(to_15_digits(418.329_999_999_999_9).to_string(), "418.33");
        assert_eq!(to_15_digits(2_170.970_000_000_000_3).to_string(), "2170.97");
        assert_eq!(
            to_15_digits(-7_935.533_406_400_002).to_string(),
            "-7935.5334064"
        );
        assert_eq!(to_15_digits(65_268.000_000_000_01).to_string(), "65268");
    }

    #[test]
    fn keeps_fifteen_digits_and_everything_shorter() {
        assert_eq!(
            to_15_digits(0.604_651_162_790_697_6).to_string(),
            "0.604651162790698"
        );
        for v in [1.0, -2.5, 1234.5678, 1e-300, 6.02e23, 123_456_789_012_345.0] {
            assert_exact!(to_15_digits(v), v);
        }
        assert_exact!(to_15_digits(0.0), 0.0);
        assert!(to_15_digits(f64::NAN).is_nan());
        assert_exact!(to_15_digits(f64::INFINITY), f64::INFINITY);
        assert_exact!(to_15_digits(f64::NEG_INFINITY), f64::NEG_INFINITY);
    }

    #[test]
    fn rounded_serializes_every_float_and_keeps_the_rest() {
        #[derive(serde::Serialize)]
        struct Inner {
            share: f64,
            count: u32,
        }
        #[derive(serde::Serialize)]
        struct Outer {
            name: String,
            total: f64,
            maybe: Option<f64>,
            list: Vec<f64>,
            map: indexmap::IndexMap<String, f64>,
            inner: Inner,
            pair: (f64, i64),
        }
        let mut map = indexmap::IndexMap::new();
        map.insert("z".to_string(), 0.1 + 0.2);
        map.insert("a".to_string(), 1.5);
        let value = Outer {
            name: "0.30000000000000004".to_string(),
            total: 418.329_999_999_999_9,
            maybe: Some(2_170.970_000_000_000_3),
            list: vec![0.1 + 0.2, 7.0],
            map,
            inner: Inner {
                share: 0.604_651_162_790_697_6,
                count: 3,
            },
            pair: (-7_935.533_406_400_002, 12),
        };
        assert_eq!(
            serde_json::to_string(&Rounded(&value)).unwrap(),
            r#"{"name":"0.30000000000000004","total":418.33,"maybe":2170.97,"list":[0.3,7.0],"map":{"z":0.3,"a":1.5},"inner":{"share":0.604651162790698,"count":3},"pair":[-7935.5334064,12]}"#
        );
        let yaml = yaml_serde::to_string(&Rounded(&value)).unwrap();
        assert!(yaml.contains("total: 418.33\n"), "{yaml}");
        assert!(yaml.contains("  z: 0.3\n  a: 1.5\n"), "{yaml}");
        let json_value = serde_json::json!({"x": 0.1 + 0.2, "n": 2});
        assert_eq!(
            serde_json::to_string(&Rounded(&json_value)).unwrap(),
            r#"{"n":2,"x":0.3}"#
        );
    }

    #[test]
    fn never_prints_more_than_fifteen_digits() {
        let mut x = 0.1_f64;
        for _ in 0..1000 {
            x = x.mul_add(1.37, 0.011) % 10_000.0;
            let shown = to_15_digits(x).to_string();
            let digits = shown
                .trim_start_matches('-')
                .split('e')
                .next()
                .unwrap()
                .replace('.', "")
                .trim_start_matches('0')
                .len();
            assert!(digits <= SIGNIFICANT_DIGITS, "{x} -> {shown}");
        }
    }

    mod rounded_serialization {
        //! `Rounded` passes every kind of value through, rounding each `f64`
        //! at any depth and changing nothing else.

        use super::super::Rounded;
        use serde::{Serialize, Serializer};

        #[derive(Serialize)]
        struct Marker;

        #[derive(Serialize)]
        struct Wrapped(f64);

        #[derive(Serialize)]
        struct Pair(f64, i8);

        #[derive(Serialize)]
        enum Shape {
            Newtype(f64),
            Tuple(f64, u8),
            Record {
                a: f64,
                #[serde(skip_serializing_if = "Option::is_none")]
                b: Option<f64>,
            },
        }

        struct Raw;

        impl Serialize for Raw {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_bytes(&[1, 2])
            }
        }

        /// A map written key by key and value by value
        struct Entries;

        impl Serialize for Entries {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1)).unwrap();
                map.serialize_key("k").unwrap();
                map.serialize_value(&(0.1 + 0.2)).unwrap();
                map.end()
            }
        }

        struct Readable;

        impl Serialize for Readable {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let readable = serializer.is_human_readable();
                serializer.serialize_bool(readable)
            }
        }

        #[derive(Serialize)]
        struct Every {
            small: (i8, i16, i128, u8, u16, u128),
            single: f32,
            letter: char,
            marker: Marker,
            wrapped: Wrapped,
            pair: Pair,
            raw: Raw,
            readable: Readable,
            entries: Entries,
            newtype: Shape,
            tuple: Shape,
            record: Shape,
            skipped: Shape,
            map: std::collections::BTreeMap<String, f64>,
            some: Option<f64>,
            none: Option<f64>,
            noisy: f64,
        }

        #[test]
        fn every_kind_of_value_comes_through_and_only_floats_change() {
            let value = Every {
                small: (-1, -2, -3, 4, 5, 6),
                single: 0.5,
                letter: 'x',
                marker: Marker,
                wrapped: Wrapped(0.1 + 0.2),
                pair: Pair(0.1 + 0.2, 7),
                raw: Raw,
                readable: Readable,
                entries: Entries,
                newtype: Shape::Newtype(0.1 + 0.2),
                tuple: Shape::Tuple(0.1 + 0.2, 8),
                record: Shape::Record {
                    a: 0.1 + 0.2,
                    b: Some(0.5),
                },
                skipped: Shape::Record { a: 1.0, b: None },
                map: std::iter::once(("k".to_string(), 0.1 + 0.2)).collect(),
                some: Some(0.1 + 0.2),
                none: None,
                noisy: 0.300_000_000_000_000_04,
            };
            let text = serde_json::to_string(&Rounded(&value)).unwrap();
            assert_eq!(
                text,
                concat!(
                    r#"{"small":[-1,-2,-3,4,5,6],"single":0.5,"letter":"x","marker":null,"#,
                    r#""wrapped":0.3,"pair":[0.3,7],"raw":[1,2],"readable":true,"entries":{"k":0.3},"#,
                    r#""newtype":{"Newtype":0.3},"tuple":{"Tuple":[0.3,8]},"#,
                    r#""record":{"Record":{"a":0.3,"b":0.5}},"skipped":{"Record":{"a":1.0}},"#,
                    r#""map":{"k":0.3},"some":0.3,"none":null,"noisy":0.3}"#
                )
            );
        }
    }
}
