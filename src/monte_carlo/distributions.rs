//! Monte Carlo Distribution Types
//!
//! Provides distribution sampling for FP&A modeling:
//! - MC.Normal(mean, stdev) - Symmetric variability
//! - MC.Triangular(min, mode, max) - Expert estimates
//! - MC.Uniform(min, max) - Complete uncertainty
//! - MC.PERT(min, mode, max) - Smooth project estimates
//! - MC.Lognormal(mean, stdev) - Non-negative values (Phase 4)

use statrs::distribution::{Beta, ContinuousCDF, Normal as StandardNormal};
use std::fmt;

/// Probabilities passed to an inverse CDF are kept this far from 0 and 1, so
/// a uniform draw of exactly 0 cannot map to an infinite value.
const P_EPSILON: f64 = 1e-12;

/// Quantile of the standard normal distribution.
pub(crate) fn standard_normal_quantile(p: f64) -> f64 {
    StandardNormal::standard().inverse_cdf(p.clamp(P_EPSILON, 1.0 - P_EPSILON))
}

/// Cumulative probability of the standard normal distribution.
pub(crate) fn standard_normal_cdf(z: f64) -> f64 {
    StandardNormal::standard().cdf(z)
}

/// A parameter must be a finite number. `1E+400`, `inf` and `NaN` are read
/// as numbers, and without this check `MC.PERT(0, 1, 1E+400)` ended the
/// process on a panic while `MC.Triangular(0, 1, 1E+400)` printed NaN
/// percentiles with exit 0 (roadmap T1-NONFINITE).
fn finite(distribution: &str, parameter: &str, value: f64) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!(
            "{distribution} {parameter} must be a finite number, not {value}"
        ))
    }
}

/// `max - min` must be finite too: `MC.PERT(-1E+308, 1E+308, 1E+308)` has
/// finite arguments, and an infinite range makes its shape `inf / inf`.
fn finite_range(distribution: &str, min: f64, max: f64) -> Result<(), String> {
    if (max - min).is_finite() {
        Ok(())
    } else {
        Err(format!(
            "{distribution} max - min is too large to calculate with ({min:e} to {max:e})"
        ))
    }
}

/// Distribution type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum DistributionType {
    /// Normal (Gaussian) distribution: MC.Normal(mean, stdev)
    Normal { mean: f64, stdev: f64 },

    /// Triangular distribution: MC.Triangular(min, mode, max)
    Triangular { min: f64, mode: f64, max: f64 },

    /// Uniform distribution: MC.Uniform(min, max)
    Uniform { min: f64, max: f64 },

    /// PERT distribution: MC.PERT(min, mode, max)
    /// Beta distribution shaped by min/mode/max, smoother than triangular
    PERT { min: f64, mode: f64, max: f64 },

    /// Lognormal distribution: MC.Lognormal(mean, stdev)
    /// For non-negative values like prices, revenue
    Lognormal { mean: f64, stdev: f64 },

    /// Discrete distribution: MC.Discrete({value: prob, ...})
    /// For scenario probabilities
    Discrete {
        values: Vec<f64>,
        probabilities: Vec<f64>,
    },
}

impl fmt::Display for DistributionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal { mean, stdev } => {
                write!(f, "MC.Normal({mean}, {stdev})")
            },
            Self::Triangular { min, mode, max } => {
                write!(f, "MC.Triangular({min}, {mode}, {max})")
            },
            Self::Uniform { min, max } => {
                write!(f, "MC.Uniform({min}, {max})")
            },
            Self::PERT { min, mode, max } => {
                write!(f, "MC.PERT({min}, {mode}, {max})")
            },
            Self::Lognormal { mean, stdev } => {
                write!(f, "MC.Lognormal({mean}, {stdev})")
            },
            Self::Discrete {
                values,
                probabilities,
            } => {
                let list = |numbers: &[f64]| {
                    numbers
                        .iter()
                        .map(f64::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                write!(
                    f,
                    "MC.Discrete({{{}}}, {{{}}})",
                    list(values),
                    list(probabilities)
                )
            },
        }
    }
}

/// A distribution the engine draws from through [`Distribution::inverse_cdf`].
///
/// Only the checked constructors build one, so its parameters are finite,
/// `min < max` with `max - min` finite, and its draws are finite.
#[derive(Debug, Clone)]
pub struct Distribution {
    dist_type: DistributionType,
}

impl Distribution {
    /// The kind of distribution and its parameters
    #[must_use]
    pub const fn dist_type(&self) -> &DistributionType {
        &self.dist_type
    }

    /// Create a Normal distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `stdev` is not positive.
    pub fn normal(mean: f64, stdev: f64) -> Result<Self, String> {
        finite("MC.Normal", "mean", mean)?;
        finite("MC.Normal", "stdev", stdev)?;
        if stdev <= 0.0 {
            return Err("Normal distribution stdev must be > 0".to_string());
        }
        Self {
            dist_type: DistributionType::Normal { mean, stdev },
        }
        .with_finite_draws("MC.Normal")
    }

    /// Create a Triangular distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `min >= max` or `mode` is outside `[min, max]`.
    pub fn triangular(min: f64, mode: f64, max: f64) -> Result<Self, String> {
        finite("MC.Triangular", "min", min)?;
        finite("MC.Triangular", "mode", mode)?;
        finite("MC.Triangular", "max", max)?;
        if min >= max {
            return Err("Triangular distribution requires min < max".to_string());
        }
        if mode < min || mode > max {
            return Err("Triangular distribution requires min <= mode <= max".to_string());
        }
        finite_range("MC.Triangular", min, max)?;
        Self {
            dist_type: DistributionType::Triangular { min, mode, max },
        }
        .with_finite_draws("MC.Triangular")
    }

    /// Create a Uniform distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `min >= max`.
    pub fn uniform(min: f64, max: f64) -> Result<Self, String> {
        finite("MC.Uniform", "min", min)?;
        finite("MC.Uniform", "max", max)?;
        if min >= max {
            return Err("Uniform distribution requires min < max".to_string());
        }
        finite_range("MC.Uniform", min, max)?;
        Self {
            dist_type: DistributionType::Uniform { min, max },
        }
        .with_finite_draws("MC.Uniform")
    }

    /// Create a PERT distribution (Beta-PERT)
    ///
    /// # Errors
    ///
    /// Returns an error if `min >= max` or `mode` is outside `[min, max]`.
    pub fn pert(min: f64, mode: f64, max: f64) -> Result<Self, String> {
        finite("MC.PERT", "min", min)?;
        finite("MC.PERT", "mode", mode)?;
        finite("MC.PERT", "max", max)?;
        if min >= max {
            return Err("PERT distribution requires min < max".to_string());
        }
        if mode < min || mode > max {
            return Err("PERT distribution requires min <= mode <= max".to_string());
        }
        finite_range("MC.PERT", min, max)?;
        Self {
            dist_type: DistributionType::PERT { min, mode, max },
        }
        .with_finite_draws("MC.PERT")
    }

    /// Create a Lognormal distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `stdev` is not positive or `mean` is not positive.
    pub fn lognormal(mean: f64, stdev: f64) -> Result<Self, String> {
        finite("MC.Lognormal", "mean", mean)?;
        finite("MC.Lognormal", "stdev", stdev)?;
        if stdev <= 0.0 {
            return Err("Lognormal distribution stdev must be > 0".to_string());
        }
        if mean <= 0.0 {
            return Err("Lognormal distribution mean must be > 0".to_string());
        }
        Self {
            dist_type: DistributionType::Lognormal { mean, stdev },
        }
        .with_finite_draws("MC.Lognormal")
    }

    /// Create a Discrete distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `values` and `probabilities` have different lengths,
    /// are empty, probabilities don't sum to 1.0, or any probability is negative.
    pub fn discrete(values: Vec<f64>, probabilities: Vec<f64>) -> Result<Self, String> {
        if values.len() != probabilities.len() {
            return Err(
                "Discrete distribution requires equal number of values and probabilities"
                    .to_string(),
            );
        }
        if values.is_empty() {
            return Err("Discrete distribution requires at least one value".to_string());
        }
        for &value in &values {
            finite("MC.Discrete", "value", value)?;
        }
        for &probability in &probabilities {
            finite("MC.Discrete", "probability", probability)?;
        }
        let sum: f64 = probabilities.iter().sum();
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!(
                "Discrete distribution probabilities must sum to 1.0 (got {sum})"
            ));
        }
        if probabilities.iter().any(|&p| p < 0.0) {
            return Err("Discrete distribution probabilities must be >= 0".to_string());
        }
        Ok(Self {
            dist_type: DistributionType::Discrete {
                values,
                probabilities,
            },
        })
    }

    /// The engine draws through `inverse_cdf`, whose probability stays in
    /// `[1e-12, 1 - 1e-12]`, so every draw lies between these two quantiles.
    /// With finite parameters they can still overflow, as in
    /// `MC.Normal(0, 1E+308)` or `MC.Lognormal(1, 1E+200)`; such a
    /// distribution is refused here rather than giving infinite or NaN draws.
    fn with_finite_draws(self, distribution: &str) -> Result<Self, String> {
        let low = self.inverse_cdf(0.0);
        let high = self.inverse_cdf(1.0);
        if low.is_finite() && high.is_finite() {
            Ok(self)
        } else {
            Err(format!(
                "{distribution}: its arguments are too large to calculate with; its draws \
                 would reach {low:e} to {high:e}"
            ))
        }
    }

    /// Value at cumulative probability `p`: the inverse CDF, or quantile function.
    ///
    /// Mapping uniform draws through this function gives samples with exactly
    /// this distribution, which lets the engine use stratified (Latin
    /// hypercube) or correlated uniforms. `p` is clamped to `[1e-12, 1 - 1e-12]`.
    ///
    /// # Panics
    ///
    /// Cannot panic for a distribution built by the checked constructors: the
    /// PERT shape parameters are always at least 1.
    #[must_use]
    pub fn inverse_cdf(&self, p: f64) -> f64 {
        let p = p.clamp(P_EPSILON, 1.0 - P_EPSILON);
        match &self.dist_type {
            DistributionType::Normal { mean, stdev } => {
                stdev.mul_add(standard_normal_quantile(p), *mean)
            },
            DistributionType::Triangular { min, mode, max } => {
                triangular_quantile(*min, *mode, *max, p)
            },
            DistributionType::Uniform { min, max } => p.mul_add(max - min, *min),
            DistributionType::PERT { min, mode, max } => {
                let range = max - min;
                let (alpha, beta) = pert_shape(*min, *mode, *max);
                let x = Beta::new(alpha, beta)
                    .expect("PERT shape parameters are at least 1")
                    .inverse_cdf(p);
                x.mul_add(range, *min)
            },
            DistributionType::Lognormal { mean, stdev } => {
                let (mu, sigma) = lognormal_params(*mean, *stdev);
                sigma.mul_add(standard_normal_quantile(p), mu).exp()
            },
            DistributionType::Discrete {
                values,
                probabilities,
            } => discrete_quantile(values, probabilities, p),
        }
    }

    /// The value to plan with when the model gives none beside the formula,
    /// with what it is: the mode of a triangular or PERT, the mean of a normal
    /// or lognormal as typed, the middle of a uniform (it has no likeliest
    /// value) and the likeliest value of a discrete distribution.
    #[must_use]
    pub fn most_likely(&self) -> (f64, &'static str) {
        match &self.dist_type {
            DistributionType::Normal { mean, .. } | DistributionType::Lognormal { mean, .. } => {
                (*mean, "mean")
            },
            DistributionType::Triangular { mode, .. } | DistributionType::PERT { mode, .. } => {
                (*mode, "mode")
            },
            DistributionType::Uniform { min, max } => (min.midpoint(*max), "midpoint"),
            DistributionType::Discrete {
                values,
                probabilities,
            } => {
                // The first of the likeliest, as written
                let mut best = 0;
                for (i, p) in probabilities.iter().enumerate() {
                    if *p > probabilities[best] {
                        best = i;
                    }
                }
                (values[best], "likeliest")
            },
        }
    }
}

/// Beta shape parameters of a PERT distribution with `max > min`
fn pert_shape(min: f64, mode: f64, max: f64) -> (f64, f64) {
    let range = max - min;
    // Divide before multiplying by 4: 4 * (max - mode) overflows when the
    // range is near the largest finite number. Multiplying by 4 is exact, so
    // the result is the same for every range that does not overflow.
    (
        4.0f64.mul_add((mode - min) / range, 1.0),
        4.0f64.mul_add((max - mode) / range, 1.0),
    )
}

/// Quantile of the triangular distribution, in closed form
fn triangular_quantile(min: f64, mode: f64, max: f64, p: f64) -> f64 {
    let range = max - min;
    let mode_probability = (mode - min) / range;
    if p < mode_probability {
        min + sqrt_of_product(p, range, mode - min)
    } else {
        max - sqrt_of_product(1.0 - p, range, max - mode)
    }
}

/// `sqrt(a * b * c)`, which is between 0 and the range when `a <= 1` and
/// `c <= b`. The product can overflow when the range is wide, as for
/// `MC.Triangular(-1E+307, 0, 1E+307)`; then each factor's root is taken
/// first. A product that fits keeps the one-root form, so a model that
/// calculated before draws exactly the same values.
fn sqrt_of_product(a: f64, b: f64, c: f64) -> f64 {
    let product = a * b * c;
    if product.is_finite() {
        product.sqrt()
    } else {
        a.sqrt() * b.sqrt() * c.sqrt()
    }
}

/// Parameters `(mu, sigma)` of the normal distribution underlying a
/// lognormal distribution with the given mean and standard deviation
fn lognormal_params(mean: f64, stdev: f64) -> (f64, f64) {
    let variance = stdev * stdev;
    let mu = (mean * mean / mean.mul_add(mean, variance).sqrt()).ln();
    let sigma = (variance / (mean * mean)).ln_1p().sqrt();
    (mu, sigma)
}

/// Value of a discrete distribution at cumulative probability `p`
fn discrete_quantile(values: &[f64], probabilities: &[f64], p: f64) -> f64 {
    let mut cumulative = 0.0;
    for (value, prob) in values.iter().zip(probabilities.iter()) {
        cumulative += prob;
        if p <= cumulative {
            return *value;
        }
    }
    // Rounding can leave the total just below 1
    *values.last().unwrap_or(&0.0)
}

/// Parse a distribution from formula string (e.g., "`MC.Normal(100, 15)`")
///
/// # Errors
///
/// Returns an error if the formula is malformed, uses an unknown distribution type,
/// has the wrong number of arguments, or has invalid parameter values.
pub fn parse_distribution(formula: &str) -> Result<Distribution, String> {
    let formula = formula.trim();

    // Check for MC. prefix
    if !formula.starts_with("MC.") {
        return Err(format!(
            "Distribution must start with 'MC.' prefix: {formula}"
        ));
    }

    let without_prefix = &formula[3..];

    // Parse function name and arguments
    let paren_pos = without_prefix
        .find('(')
        .ok_or_else(|| format!("Missing opening parenthesis: {formula}"))?;

    let func_name = &without_prefix[..paren_pos];
    let args_str = without_prefix[paren_pos + 1..]
        .strip_suffix(')')
        .ok_or_else(|| format!("Missing closing parenthesis: {formula}"))?;

    if func_name.eq_ignore_ascii_case("Discrete") {
        return parse_discrete(args_str);
    }

    // Parse arguments
    let args: Vec<f64> = args_str
        .split(',')
        .map(|s| s.trim().parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Invalid argument: {e}"))?;

    match func_name.to_uppercase().as_str() {
        "NORMAL" => {
            if args.len() != 2 {
                return Err("MC.Normal requires 2 arguments: (mean, stdev)".to_string());
            }
            Distribution::normal(args[0], args[1])
        },
        "TRIANGULAR" => {
            if args.len() != 3 {
                return Err("MC.Triangular requires 3 arguments: (min, mode, max)".to_string());
            }
            Distribution::triangular(args[0], args[1], args[2])
        },
        "UNIFORM" => {
            if args.len() != 2 {
                return Err("MC.Uniform requires 2 arguments: (min, max)".to_string());
            }
            Distribution::uniform(args[0], args[1])
        },
        "PERT" => {
            if args.len() != 3 {
                return Err("MC.PERT requires 3 arguments: (min, mode, max)".to_string());
            }
            Distribution::pert(args[0], args[1], args[2])
        },
        "LOGNORMAL" => {
            if args.len() != 2 {
                return Err("MC.Lognormal requires 2 arguments: (mean, stdev)".to_string());
            }
            Distribution::lognormal(args[0], args[1])
        },
        _ => Err(format!("Unknown distribution type: {func_name}")),
    }
}

const DISCRETE_FORM: &str =
    "MC.Discrete takes two arrays, the values and their probabilities: MC.Discrete({0, 0.3}, {0.9, 0.1})";

/// The arguments of `MC.Discrete({values}, {probabilities})`: two array
/// constants (ADR-064), each a row `{0, 0.3}` or a column `{0; 0.3}`.
fn parse_discrete(args: &str) -> Result<Distribution, String> {
    let (values, rest) = array_constant(args.trim())?;
    let rest = rest
        .trim_start()
        .strip_prefix(',')
        .ok_or_else(|| DISCRETE_FORM.to_string())?;
    let (probabilities, rest) = array_constant(rest.trim_start())?;
    if !rest.trim().is_empty() {
        return Err(DISCRETE_FORM.to_string());
    }
    Distribution::discrete(
        numbers(values, "value")?,
        numbers(probabilities, "probability")?,
    )
}

/// The inside of the array constant at the start of `text`, and what follows it.
fn array_constant(text: &str) -> Result<(&str, &str), String> {
    let inner = text
        .strip_prefix('{')
        .ok_or_else(|| DISCRETE_FORM.to_string())?;
    let end = inner.find('}').ok_or_else(|| DISCRETE_FORM.to_string())?;
    Ok((&inner[..end], &inner[end + 1..]))
}

/// The numbers of one array constant: a row or a column, not a table.
fn numbers(items: &str, what: &str) -> Result<Vec<f64>, String> {
    if items.contains(',') && items.contains(';') {
        return Err(format!(
            "MC.Discrete {what} array must be one row {{0, 0.3}} or one column {{0; 0.3}}, not a table: {{{items}}}"
        ));
    }
    items
        .split([',', ';'])
        .map(|item| {
            let item = item.trim();
            item.parse::<f64>()
                .map_err(|_| format!("MC.Discrete {what} '{item}' is not a number"))
        })
        .collect()
}

// Financial math: exact float comparison validated against Excel/Gnumeric/R
#[cfg(test)]
mod tests {
    use super::*;

    /// The quantiles the engine draws through, at five probabilities, against
    /// scipy 1.18.1 (`stats.norm`, `triang`, `uniform`, `beta` scaled to the
    /// range for PERT, `lognorm` with the mean and stdev turned into `mu` and
    /// `sigma`), to within 1e-9 of each value.
    #[test]
    fn each_quantile_is_scipys() {
        let probabilities = [0.025, 0.1, 0.5, 0.9, 0.975];
        let cases: [(&str, [f64; 5]); 6] = [
            (
                "MC.Normal(100, 15)",
                [
                    70.600_540_231_899_17,
                    80.776_726_516_830_99,
                    100.0,
                    119.223_273_483_169_01,
                    129.399_459_768_100_83,
                ],
            ),
            (
                "MC.Triangular(0, 5, 10)",
                [
                    1.118_033_988_749_895,
                    2.236_067_977_499_79,
                    5.0,
                    7.763_932_022_500_21,
                    8.881_966_011_250_103,
                ],
            ),
            (
                "MC.Triangular(0, 2, 10)",
                [
                    0.707_106_781_186_547_7,
                    1.414_213_562_373_095_4,
                    3.675_444_679_663_241,
                    7.171_572_875_253_81,
                    8.585_786_437_626_904,
                ],
            ),
            ("MC.Uniform(10, 20)", [10.25, 11.0, 15.0, 19.0, 19.75]),
            (
                "MC.PERT(0, 3, 10)",
                [
                    0.680_956_639_218_361,
                    1.361_999_625_636_714_2,
                    3.509_994_849_491_181_2,
                    6.207_544_856_651_202,
                    7.475_513_729_654_873,
                ],
            ),
            (
                "MC.Lognormal(2000, 500)",
                [
                    1_197.518_154_290_320_1,
                    1_415.227_613_115_077_5,
                    1_940.285_000_290_663,
                    2_660.141_624_898_338,
                    3_143.756_834_804_729,
                ],
            ),
        ];
        for (formula, expected) in cases {
            let dist = parse_distribution(formula).unwrap();
            let got = probabilities.map(|p| dist.inverse_cdf(p));
            let close = got
                .iter()
                .zip(expected)
                .all(|(g, e)| (g - e).abs() <= 1e-9 * e.abs().max(1.0));
            assert_eq!((formula, close), (formula, true), "{got:?}");
        }
    }

    /// A discrete distribution's probabilities may sum to 1 within 0.001; a
    /// draw above their total is the last value.
    #[test]
    fn a_draw_above_a_discrete_total_just_below_one_is_the_last_value() {
        let dist = Distribution::discrete(vec![1.0, 2.0], vec![0.5, 0.4995]).unwrap();
        assert_exact!(dist.inverse_cdf(0.9999), 2.0);
        assert_exact!(dist.inverse_cdf(0.5), 1.0);
        assert_exact!(dist.inverse_cdf(0.5001), 2.0);
    }

    /// Each parameter check, with its message
    #[test]
    fn each_distribution_names_the_parameters_it_refuses() {
        for (formula, expected) in [
            (
                "MC.Triangular(0, 11, 10)",
                "Triangular distribution requires min <= mode <= max",
            ),
            (
                "MC.Triangular(0, -1, 10)",
                "Triangular distribution requires min <= mode <= max",
            ),
            (
                "MC.Triangular(10, 5, 10)",
                "Triangular distribution requires min < max",
            ),
            (
                "MC.Uniform(20, 10)",
                "Uniform distribution requires min < max",
            ),
            (
                "MC.Uniform(10, 10)",
                "Uniform distribution requires min < max",
            ),
            (
                "MC.PERT(10, 10, 10)",
                "PERT distribution requires min < max",
            ),
            (
                "MC.PERT(0, 11, 10)",
                "PERT distribution requires min <= mode <= max",
            ),
            (
                "MC.PERT(0, -1, 10)",
                "PERT distribution requires min <= mode <= max",
            ),
            (
                "MC.Lognormal(10, 0)",
                "Lognormal distribution stdev must be > 0",
            ),
            (
                "MC.Lognormal(0, 1)",
                "Lognormal distribution mean must be > 0",
            ),
            (
                "MC.Lognormal(-5, 1)",
                "Lognormal distribution mean must be > 0",
            ),
            (
                "MC.Normal(100)",
                "MC.Normal requires 2 arguments: (mean, stdev)",
            ),
            (
                "MC.Triangular(0, 5)",
                "MC.Triangular requires 3 arguments: (min, mode, max)",
            ),
            (
                "MC.Uniform(1, 2, 3)",
                "MC.Uniform requires 2 arguments: (min, max)",
            ),
            (
                "MC.PERT(0, 5)",
                "MC.PERT requires 3 arguments: (min, mode, max)",
            ),
            (
                "MC.Lognormal(5)",
                "MC.Lognormal requires 2 arguments: (mean, stdev)",
            ),
            ("MC.Unknown(1, 2)", "Unknown distribution type: Unknown"),
            (
                "Normal(100, 15)",
                "Distribution must start with 'MC.' prefix: Normal(100, 15)",
            ),
            (
                "MC.Normal 100, 15",
                "Missing opening parenthesis: MC.Normal 100, 15",
            ),
            (
                "MC.Normal(100, 15",
                "Missing closing parenthesis: MC.Normal(100, 15",
            ),
            (
                "MC.Normal(100, x)",
                "Invalid argument: invalid float literal",
            ),
        ] {
            assert_eq!(
                parse_distribution(formula).err(),
                Some(expected.to_string()),
                "{formula}"
            );
        }
        assert_eq!(
            Distribution::discrete(vec![], vec![]).err(),
            Some("Discrete distribution requires at least one value".to_string())
        );
        assert_eq!(
            Distribution::discrete(vec![1.0, 2.0], vec![1.5, -0.5]).err(),
            Some("Discrete distribution probabilities must be >= 0".to_string())
        );
    }

    #[test]
    fn test_parse_distribution() {
        for (formula, expected) in [
            (
                "MC.Normal(100, 15)",
                DistributionType::Normal {
                    mean: 100.0,
                    stdev: 15.0,
                },
            ),
            (
                "MC.Triangular(0, 5, 10)",
                DistributionType::Triangular {
                    min: 0.0,
                    mode: 5.0,
                    max: 10.0,
                },
            ),
            (
                "MC.Uniform(10, 20)",
                DistributionType::Uniform {
                    min: 10.0,
                    max: 20.0,
                },
            ),
            (
                "MC.PERT(0, 3, 10)",
                DistributionType::PERT {
                    min: 0.0,
                    mode: 3.0,
                    max: 10.0,
                },
            ),
            (
                "MC.lognormal(2000, 500)",
                DistributionType::Lognormal {
                    mean: 2000.0,
                    stdev: 500.0,
                },
            ),
        ] {
            assert_eq!(parse_distribution(formula).unwrap().dist_type(), &expected);
        }
    }

    #[test]
    fn test_parse_distribution_errors() {
        assert!(parse_distribution("Normal(100, 15)").is_err()); // Missing MC. prefix
        assert!(parse_distribution("MC.Normal(100)").is_err()); // Wrong arg count
        assert!(parse_distribution("MC.Normal(100, -5)").is_err()); // Negative stdev
        assert!(parse_distribution("MC.Triangular(10, 5, 0)").is_err()); // min > max
        assert!(parse_distribution("MC.Unknown(1, 2)").is_err()); // Unknown type
    }

    /// T1-DISCRETE: MC.Discrete was documented and gave "Unknown distribution
    /// type: Discrete". Its two arguments are array constants (ADR-064).
    #[test]
    fn discrete_takes_its_values_and_probabilities_as_two_arrays() {
        let row = parse_distribution("MC.Discrete({0, 0.3}, {0.9, 0.1})").unwrap();
        let column = parse_distribution("MC.Discrete( {0; 0.3} , {0.9; 0.1} )").unwrap();
        let expected = DistributionType::Discrete {
            values: vec![0.0, 0.3],
            probabilities: vec![0.9, 0.1],
        };
        assert_eq!(
            (row.dist_type(), column.dist_type()),
            (&expected, &expected)
        );
        // The first 90% of the probability is no frost, the last 10% a frost
        assert_exact!(row.inverse_cdf(0.9), 0.0);
        assert_exact!(row.inverse_cdf(0.900_001), 0.3);
        assert_eq!(row.most_likely(), (0.0, "likeliest"));
        let certain = parse_distribution("MC.discrete({1}, {1})").unwrap();
        assert_exact!(certain.inverse_cdf(0.5), 1.0);
    }

    #[test]
    fn discrete_written_any_other_way_is_refused_with_its_form() {
        let form = DISCRETE_FORM;
        for formula in [
            "MC.Discrete(0, 0.3, 0.9, 0.1)",
            "MC.Discrete({0, 0.3})",
            "MC.Discrete({0, 0.3} {0.9, 0.1})",
            "MC.Discrete({0, 0.3}, 0.9)",
            "MC.Discrete({0, 0.3}, {0.9, 0.1}, {1})",
            "MC.Discrete({0, 0.3, {0.9, 0.1})",
            "MC.Discrete({0, 0.3)",
        ] {
            refused_with(parse_distribution(formula), form);
        }
        refused_with(
            parse_distribution("MC.Discrete({0, 1; 2, 3}, {0.25, 0.25, 0.25, 0.25})"),
            "MC.Discrete value array must be one row {0, 0.3} or one column {0; 0.3}, not a table: {0, 1; 2, 3}",
        );
        refused_with(
            parse_distribution("MC.Discrete({0, 0.3}, {0.9, ten})"),
            "MC.Discrete probability 'ten' is not a number",
        );
        refused_with(
            parse_distribution("MC.Discrete({0, 0.3}, {0.9, 0.2})"),
            "probabilities must sum to 1.0",
        );
        refused_with(
            parse_distribution("MC.Discrete({0, 0.3, 1}, {0.9, 0.1})"),
            "equal number of values and probabilities",
        );
    }

    /// Each distribution is shown as the formula that gives it again.
    #[test]
    fn each_distribution_is_shown_as_its_formula() {
        for formula in [
            "MC.Normal(100, 15)",
            "MC.Triangular(0, 5, 10)",
            "MC.Uniform(10, 20)",
            "MC.PERT(0, 3, 10)",
            "MC.Lognormal(2000, 500)",
            "MC.Discrete({0, 0.3}, {0.9, 0.1})",
        ] {
            let dist = parse_distribution(formula).unwrap();
            assert_eq!(dist.dist_type.to_string(), formula);
        }
    }

    /// T1-SIM-MEAN-VS-PLAN: the value to plan with when the model gives none
    #[test]
    fn most_likely_is_the_value_each_distribution_is_written_around() {
        let cases = [
            ("MC.Normal(100, 15)", (100.0, "mean")),
            ("MC.Lognormal(50, 5)", (50.0, "mean")),
            ("MC.Triangular(28, 32, 38)", (32.0, "mode")),
            ("MC.PERT(40, 55, 70)", (55.0, "mode")),
            ("MC.Uniform(17.5, 20.5)", (19.0, "midpoint")),
        ];
        for (formula, expected) in cases {
            assert_eq!(
                parse_distribution(formula).unwrap().most_likely(),
                expected,
                "{formula}"
            );
        }
        // The likeliest value; on a tie, the first as written
        let discrete = Distribution::discrete(vec![1.0, 2.0, 3.0], vec![0.25, 0.5, 0.25]).unwrap();
        assert_eq!(discrete.most_likely(), (2.0, "likeliest"));
        let tie = Distribution::discrete(vec![7.0, 9.0], vec![0.5, 0.5]).unwrap();
        assert_eq!(tie.most_likely(), (7.0, "likeliest"));
    }

    /// A PERT whose range is finite but near the largest number: its shape
    /// multiplied 4 by `max - mode` before dividing, which overflowed.
    #[test]
    fn a_pert_near_the_largest_number_draws_finite_values() {
        let dist = parse_distribution("MC.PERT(0, 1, 1E+308)").unwrap();
        let x = dist.inverse_cdf(0.5);
        assert_eq!(
            (x.is_finite(), (0.0..=1e308).contains(&x)),
            (true, true),
            "{x}"
        );
    }

    // Non-finite parameters (roadmap T1-NONFINITE)

    const NOT_FINITE: [f64; 3] = [f64::INFINITY, f64::NEG_INFINITY, f64::NAN];

    fn refused_with(result: Result<Distribution, String>, expected: &str) {
        let message = result.err().unwrap();
        assert!(
            message.contains(expected),
            "{message}; expected: {expected}"
        );
    }

    /// The draws the engine can take: the quantiles at both ends and in between
    fn draws_are_finite(dist: &Distribution) {
        for p in [0.0, 1e-12, 0.001, 0.5, 0.999, 1.0 - 1e-12, 1.0] {
            let x = dist.inverse_cdf(p);
            assert!(x.is_finite(), "{} at {p}: {x}", dist.dist_type);
        }
    }

    #[test]
    fn every_parameter_must_be_finite() {
        for bad in NOT_FINITE {
            let text = format!("not {bad}");
            refused_with(
                Distribution::normal(bad, 1.0),
                &format!("MC.Normal mean must be a finite number, {text}"),
            );
            refused_with(
                Distribution::normal(0.0, bad),
                &format!("MC.Normal stdev must be a finite number, {text}"),
            );
            refused_with(
                Distribution::triangular(bad, 1.0, 2.0),
                &format!("MC.Triangular min must be a finite number, {text}"),
            );
            refused_with(
                Distribution::triangular(0.0, bad, 2.0),
                &format!("MC.Triangular mode must be a finite number, {text}"),
            );
            refused_with(
                Distribution::triangular(0.0, 1.0, bad),
                &format!("MC.Triangular max must be a finite number, {text}"),
            );
            refused_with(
                Distribution::uniform(bad, 2.0),
                &format!("MC.Uniform min must be a finite number, {text}"),
            );
            refused_with(
                Distribution::uniform(0.0, bad),
                &format!("MC.Uniform max must be a finite number, {text}"),
            );
            refused_with(
                Distribution::pert(bad, 1.0, 2.0),
                &format!("MC.PERT min must be a finite number, {text}"),
            );
            refused_with(
                Distribution::pert(0.0, bad, 2.0),
                &format!("MC.PERT mode must be a finite number, {text}"),
            );
            refused_with(
                Distribution::pert(0.0, 1.0, bad),
                &format!("MC.PERT max must be a finite number, {text}"),
            );
            refused_with(
                Distribution::lognormal(bad, 1.0),
                &format!("MC.Lognormal mean must be a finite number, {text}"),
            );
            refused_with(
                Distribution::lognormal(1.0, bad),
                &format!("MC.Lognormal stdev must be a finite number, {text}"),
            );
            refused_with(
                Distribution::discrete(vec![1.0, bad], vec![0.5, 0.5]),
                &format!("MC.Discrete value must be a finite number, {text}"),
            );
        }
        // A NaN probability passed the sum check, as NaN compares false with everything
        refused_with(
            Distribution::discrete(vec![1.0, 2.0], vec![1.0, f64::NAN]),
            "MC.Discrete probability must be a finite number, not NaN",
        );
    }

    #[test]
    fn the_audits_formulas_are_refused_naming_the_argument() {
        refused_with(
            parse_distribution("MC.PERT(0, 1, inf)"),
            "MC.PERT max must be a finite number, not inf",
        );
        refused_with(
            parse_distribution("MC.PERT(0, 1, 1E+400)"),
            "MC.PERT max must be a finite number, not inf",
        );
        refused_with(
            parse_distribution("MC.Normal(0, nan)"),
            "MC.Normal stdev must be a finite number, not NaN",
        );
        refused_with(
            parse_distribution("MC.Triangular(0, 1, inf)"),
            "MC.Triangular max must be a finite number, not inf",
        );
        refused_with(
            parse_distribution("MC.Lognormal(1, inf)"),
            "MC.Lognormal stdev must be a finite number, not inf",
        );
        refused_with(
            parse_distribution("MC.Uniform(-inf, 1)"),
            "MC.Uniform min must be a finite number, not -inf",
        );
    }

    #[test]
    fn a_range_too_wide_to_subtract_is_refused() {
        // Finite arguments, an infinite max - min: PERT's shape was inf / inf
        let expected = "max - min is too large to calculate with (-1e308 to 1e308)";
        refused_with(Distribution::pert(-1e308, 1e308, 1e308), expected);
        refused_with(Distribution::pert(-1e308, 0.0, 1e308), expected);
        refused_with(Distribution::triangular(-1e308, 0.0, 1e308), expected);
        refused_with(Distribution::uniform(-1e308, 1e308), expected);
    }

    #[test]
    fn draws_that_would_overflow_are_refused() {
        let expected = "its arguments are too large to calculate with";
        // About 7 standard deviations either side, at the engine's 1e-12 and 1 - 1e-12
        refused_with(Distribution::normal(0.0, 1e308), expected);
        refused_with(Distribution::normal(1.5e308, 1e307), expected);
        refused_with(Distribution::lognormal(1.0, 1e200), expected);
    }

    #[test]
    fn large_but_finite_distributions_are_kept_and_draw_finite_values() {
        for dist in [
            Distribution::normal(0.0, 1e300).unwrap(),
            Distribution::normal(-1e307, 1e306).unwrap(),
            // range * (mode - min) overflows; the draws are within the range
            Distribution::triangular(-1e307, 0.0, 1e307).unwrap(),
            Distribution::triangular(0.0, 1e200, 1e200).unwrap(),
            Distribution::triangular(-1e200, -1e200, 0.0).unwrap(),
            // The largest a draw can be is 1e308 + 7 * 1e307, below f64::MAX
            Distribution::normal(1e308, 1e307).unwrap(),
            Distribution::uniform(-1e307, 1e307).unwrap(),
            Distribution::pert(-1e307, 1e307, 1e307).unwrap(),
            Distribution::lognormal(1e150, 1e150).unwrap(),
            Distribution::lognormal(1.0, 1e100).unwrap(),
            Distribution::discrete(vec![-1e308, 1e308], vec![0.5, 0.5]).unwrap(),
        ] {
            draws_are_finite(&dist);
        }
    }

    #[test]
    fn the_engine_names_the_scalar_of_a_refused_distribution() {
        use crate::monte_carlo::config::MonteCarloConfig;
        use crate::monte_carlo::engine::MonteCarloEngine;
        let model = crate::parser::parse_model_text(
            "m.yaml",
            "_forge_version: \"5.1.0\"\nm:\n  x:\n    formula: \"=MC.PERT(0, 1, 1E+400)\"\n",
            &std::collections::HashMap::<String, String>::new(),
        )
        .unwrap();
        let mut engine = MonteCarloEngine::new(MonteCarloConfig::default()).unwrap();
        let message = engine.parse_distributions_from_model(&model).unwrap_err();
        assert_eq!(message, "m.x: MC.PERT max must be a finite number, not inf");
    }
    #[test]
    fn a_wide_triangular_draws_the_narrow_one_scaled() {
        // The quantile scales with the range: the overflow path agrees with
        // the one-root path of the same shape, a factor of 1e307 apart
        let narrow = Distribution::triangular(-1.0, 0.25, 1.0).unwrap();
        let wide = Distribution::triangular(-1e307, 0.25e307, 1e307).unwrap();
        for p in [1e-12, 0.01, 0.2, 0.5, 0.625, 0.9, 1.0 - 1e-12] {
            let expected = narrow.inverse_cdf(p) * 1e307;
            let got = wide.inverse_cdf(p);
            assert!(
                (got - expected).abs() <= 1e-12 * 1e307,
                "at {p}: {got:e}, expected {expected:e}"
            );
        }
    }

    // The command line and the MCP server, which need `cli`
    #[cfg(feature = "cli")]
    mod refused_everywhere {
        //! A distribution with a parameter that is not a finite number is refused,
        //! naming the scalar and the argument (roadmap T1-NONFINITE). `1E+400`, `inf`
        //! and `NaN` are read as numbers: before this, `MC.PERT(0, 1, 1E+400)` ended
        //! `forge simulate` on a panic (exit 134, as the release build aborts on a
        //! panic) and a `forge mcp` server on the same call, while
        //! `MC.Triangular(0, 1, 1E+400)` and `MC.Lognormal(1, 1E+400)` printed NaN
        //! percentiles with exit 0.

        use crate::cli::forge;
        use crate::mcp::Session;

        /// Each distribution of the audit, and the start of the message it gives
        const REFUSED: [(&str, &str); 8] = [
            (
                "MC.PERT(0, 1, 1E+400)",
                "MC.PERT max must be a finite number, not inf",
            ),
            (
                "MC.Normal(0, nan)",
                "MC.Normal stdev must be a finite number, not NaN",
            ),
            (
                "MC.Triangular(0, 1, 1E+400)",
                "MC.Triangular max must be a finite number, not inf",
            ),
            (
                "MC.Lognormal(1, 1E+400)",
                "MC.Lognormal stdev must be a finite number, not inf",
            ),
            (
                "MC.Uniform(0, inf)",
                "MC.Uniform max must be a finite number, not inf",
            ),
            (
                "MC.PERT(-1E+308, 1E+308, 1E+308)",
                "MC.PERT max - min is too large to calculate with",
            ),
            (
                "MC.Normal(0, 1E+308)",
                "MC.Normal: its arguments are too large to calculate with",
            ),
            (
                "MC.Lognormal(1, 1E+200)",
                "MC.Lognormal: its arguments are too large to calculate with",
            ),
        ];

        fn model(distribution: &str) -> String {
            format!(
                "_forge_version: \"5.1.0\"\nmonte_carlo:\n  iterations: 200\n  seed: 1\n  outputs:\n    \
         - variable: m.out\n      percentiles: [50]\nm:\n  x:\n    formula: \"={distribution}\"\n  \
         out:\n    formula: \"=x * 2\"\n"
            )
        }

        #[test]
        fn forge_simulate_exits_1_naming_the_scalar_and_the_argument() {
            let dir = tempfile::tempdir().unwrap();
            for (distribution, message) in REFUSED {
                let path = dir.path().join("m.yaml");
                std::fs::write(&path, model(distribution)).unwrap();
                let ran = forge(&["simulate", path.to_str().unwrap()]);
                assert_eq!(ran.code, 1, "{distribution}: {}", ran.err);
                assert!(
                    ran.err.contains(&format!("m.x: {message}")),
                    "{distribution}: {}",
                    ran.err
                );
                assert!(!ran.out.contains("NaN"));
            }
        }

        #[test]
        fn a_wide_but_finite_distribution_still_simulates() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, model("MC.Triangular(-1E+307, 0, 1E+307)")).unwrap();
            let ran = forge(&["simulate", path.to_str().unwrap()]);
            assert_eq!(ran.code, 0, "{}", ran.err);
            let stdout = ran.out;
            // The draws, the mean, the percentiles and the extremes are finite. The
            // standard deviation of values near 1e307 overflows when squared, as for
            // any output above about 1e154: roadmap T1-HUGE-STATS, not this item.
            assert!(!stdout.contains("NaN"), "{stdout}");
            let infinite: Vec<&str> = stdout.lines().filter(|l| l.contains("inf")).collect();
            assert_eq!(infinite.len(), 1, "{infinite:?}");
            assert!(
                infinite[0].trim_start().starts_with("Std Dev:"),
                "{infinite:?}"
            );
        }

        #[test]
        fn forge_mcp_answers_each_with_an_error_and_then_simulates() {
            let mut session = Session::start(None);
            let mut call = |id: u64, distribution: &str| {
                session.call(
                    id,
                    "forge_simulate",
                    &serde_json::json!({"content": model(distribution)}),
                )
            };
            for ((distribution, message), id) in REFUSED.into_iter().zip(2..) {
                let answer = call(id, distribution);
                assert!(
                    answer.contains("\"isError\":true"),
                    "{distribution}: {answer}"
                );
                assert!(
                    answer.contains(&format!("m.x: {message}")),
                    "{distribution}: {answer}"
                );
            }
            let answer = call(100, "MC.PERT(0, 5, 10)");
            assert!(answer.contains("\"isError\":false"), "{answer}");
            assert!(answer.contains("monte_carlo_results"), "{answer}");
            assert!(session.is_serving(), "the server is alive");
        }
    }
}
