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
//! `418.33`. The calculation itself is not rounded.

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
}
