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
    #[allow(clippy::float_cmp)] // the exact numbers are the point
    fn rounds_the_noise_away() {
        assert_eq!(to_15_digits(0.1 + 0.2), 0.3);
        assert_eq!(to_15_digits(418.329_999_999_999_9).to_string(), "418.33");
        assert_eq!(to_15_digits(2_170.970_000_000_000_3).to_string(), "2170.97");
        assert_eq!(
            to_15_digits(-7_935.533_406_400_002).to_string(),
            "-7935.5334064"
        );
        assert_eq!(to_15_digits(65_268.000_000_000_01).to_string(), "65268");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn keeps_fifteen_digits_and_everything_shorter() {
        assert_eq!(
            to_15_digits(0.604_651_162_790_697_6).to_string(),
            "0.604651162790698"
        );
        for v in [1.0, -2.5, 1234.5678, 1e-300, 6.02e23, 123_456_789_012_345.0] {
            assert_eq!(to_15_digits(v), v);
        }
        assert_eq!(to_15_digits(0.0), 0.0);
        assert!(to_15_digits(f64::NAN).is_nan());
        assert_eq!(to_15_digits(f64::INFINITY), f64::INFINITY);
        assert_eq!(to_15_digits(f64::NEG_INFINITY), f64::NEG_INFINITY);
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
