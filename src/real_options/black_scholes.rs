//! Black-Scholes Option Pricing
//!
//! Closed-form solution for European options.
//! Unit tests check prices against `QuantLib` reference values.

use std::f64::consts::PI;

/// Black-Scholes model for European options
pub struct BlackScholes {
    /// Spot price (current value)
    pub spot: f64,
    /// Strike price (exercise cost)
    pub strike: f64,
    /// Risk-free rate (annual)
    pub rate: f64,
    /// Volatility (annual)
    pub volatility: f64,
    /// Time to maturity (years)
    pub maturity: f64,
    /// Dividend yield (continuous)
    pub dividend_yield: f64,
}

impl BlackScholes {
    /// Create a new Black-Scholes model
    #[must_use]
    pub const fn new(spot: f64, strike: f64, rate: f64, volatility: f64, maturity: f64) -> Self {
        Self {
            spot,
            strike,
            rate,
            volatility,
            maturity,
            dividend_yield: 0.0,
        }
    }

    /// Set dividend yield
    #[must_use]
    pub const fn with_dividend_yield(mut self, yield_rate: f64) -> Self {
        self.dividend_yield = yield_rate;
        self
    }

    /// Calculate d1 parameter
    fn d1(&self) -> f64 {
        let numerator = 0.5f64
            .mul_add(self.volatility.powi(2), self.rate - self.dividend_yield)
            .mul_add(self.maturity, (self.spot / self.strike).ln());
        let denominator = self.volatility * self.maturity.sqrt();
        numerator / denominator
    }

    /// Calculate d2 parameter
    fn d2(&self) -> f64 {
        self.volatility.mul_add(-self.maturity.sqrt(), self.d1())
    }

    /// Standard normal CDF (cumulative distribution function)
    /// The standard normal CDF that NORM.S.DIST uses, right to about 3 units
    /// in the last place. The Numerical Recipes approximation of `erf` used
    /// before was off by up to 1.2e-7, which moved a price on a $5M project by
    /// about $0.09; `statrs`'s is right to only 11 digits.
    fn norm_cdf(x: f64) -> f64 {
        crate::core::array_calculator::evaluator::standard_cdf(x)
    }

    /// Calculate European call option price
    #[must_use]
    pub fn call_price(&self) -> f64 {
        let d1 = self.d1();
        let d2 = self.d2();

        let discount_factor = (-self.rate * self.maturity).exp();
        let dividend_factor = (-self.dividend_yield * self.maturity).exp();

        (self.spot * dividend_factor).mul_add(
            Self::norm_cdf(d1),
            -(self.strike * discount_factor * Self::norm_cdf(d2)),
        )
    }

    /// Calculate European put option price
    #[must_use]
    pub fn put_price(&self) -> f64 {
        let d1 = self.d1();
        let d2 = self.d2();

        let discount_factor = (-self.rate * self.maturity).exp();
        let dividend_factor = (-self.dividend_yield * self.maturity).exp();

        (self.strike * discount_factor).mul_add(
            Self::norm_cdf(-d2),
            -(self.spot * dividend_factor * Self::norm_cdf(-d1)),
        )
    }

    /// Calculate option delta (sensitivity to spot price)
    #[must_use]
    pub fn delta_call(&self) -> f64 {
        let dividend_factor = (-self.dividend_yield * self.maturity).exp();
        dividend_factor * Self::norm_cdf(self.d1())
    }

    /// Calculate option gamma (sensitivity of delta)
    #[must_use]
    pub fn gamma(&self) -> f64 {
        let dividend_factor = (-self.dividend_yield * self.maturity).exp();
        let d1 = self.d1();
        dividend_factor * Self::norm_pdf(d1) / (self.spot * self.volatility * self.maturity.sqrt())
    }

    /// Calculate option vega (sensitivity to volatility)
    #[must_use]
    pub fn vega(&self) -> f64 {
        let dividend_factor = (-self.dividend_yield * self.maturity).exp();
        let d1 = self.d1();
        self.spot * dividend_factor * Self::norm_pdf(d1) * self.maturity.sqrt()
    }

    /// Calculate option theta (time decay) for call
    #[must_use]
    pub fn theta_call(&self) -> f64 {
        let d1 = self.d1();
        let d2 = self.d2();
        let discount_factor = (-self.rate * self.maturity).exp();
        let dividend_factor = (-self.dividend_yield * self.maturity).exp();

        let term1 = -self.spot * dividend_factor * Self::norm_pdf(d1) * self.volatility
            / (2.0 * self.maturity.sqrt());
        let term2 = -self.rate * self.strike * discount_factor * Self::norm_cdf(d2);
        let term3 = self.dividend_yield * self.spot * dividend_factor * Self::norm_cdf(d1);

        term1 + term2 + term3
    }

    /// Calculate option rho (sensitivity to interest rate) for call
    #[must_use]
    pub fn rho_call(&self) -> f64 {
        let d2 = self.d2();
        let discount_factor = (-self.rate * self.maturity).exp();
        self.strike * self.maturity * discount_factor * Self::norm_cdf(d2)
    }

    /// Standard normal PDF
    fn norm_pdf(x: f64) -> f64 {
        (-0.5 * x.powi(2)).exp() / (2.0 * PI).sqrt()
    }
}

#[cfg(test)]
mod black_scholes_tests {
    use super::*;

    /// Theta and rho of a call against mpmath at 40 digits, where they equal
    /// the derivatives of the price in time and in the rate: S = K = 100,
    /// r = 5%, volatility 20%, one year, a 2% dividend yield
    #[test]
    fn theta_and_rho_are_the_derivatives_of_the_price() {
        let bs = BlackScholes::new(100.0, 100.0, 0.05, 0.2, 1.0).with_dividend_yield(0.02);
        let (theta, rho) = (bs.theta_call(), bs.rho_call());
        assert_eq!(
            (
                (theta + 5.089_318_913_998_332).abs() <= 1e-12,
                (rho - 49.458_109_105_322_35).abs() <= 1e-12
            ),
            (true, true),
            "{theta} {rho}"
        );
    }

    /// Calls and puts against scipy 1.18.1 (`norm.cdf` in the Merton
    /// formula with a dividend yield), to within 1e-12 of the spot. The
    /// approximation of `erf` used before was off by up to 4.6e-8 of it.
    #[test]
    fn prices_are_scipys() {
        let cases = [
            (
                (100.0, 100.0, 0.05, 0.2, 1.0, 0.0),
                10.450_583_572_185_565,
                5.573_526_022_256_971,
            ),
            (
                (150.0, 100.0, 0.05, 0.2, 1.0, 0.0),
                54.970_140_137_999_195,
                0.093_082_588_070_597_44,
            ),
            (
                (50.0, 100.0, 0.05, 0.2, 1.0, 0.0),
                0.002_399_417_553_309_732_3,
                45.125_341_867_624_72,
            ),
            (
                (100.0, 100.0, 0.05, 0.2, 1.0, 0.02),
                9.227_005_508_154_036,
                6.330_080_627_549_918,
            ),
            (
                (5e6, 4e6, 0.05, 0.3, 3.0, 0.0),
                1_850_180.738_209_715_6,
                293_012.643_909_946_8,
            ),
            (
                (1e7, 3e6, 0.05, 0.3, 3.0, 0.0),
                7_421_565.413_701_064,
                3_689.342_976_236_708_6,
            ),
            (
                (2e6, 1e6, 0.05, 0.3, 3.0, 0.01),
                1_096_717.282_454_456_7,
                16_534.191_782_498_05,
            ),
        ];
        for ((spot, strike, rate, vol, years, dividend), call, put) in cases {
            let bs =
                BlackScholes::new(spot, strike, rate, vol, years).with_dividend_yield(dividend);
            let (c, p) = (bs.call_price(), bs.put_price());
            let close = |got: f64, want: f64| (got - want).abs() <= 1e-12 * spot;
            assert_eq!(
                (close(c, call), close(p, put)),
                (true, true),
                "{spot} {strike}: {c} {p}"
            );
        }
    }

    /// Test against known Black-Scholes values
    /// Checked against a `QuantLib` reference value
    #[test]
    fn test_call_price() {
        // Standard test case:
        // S=100, K=100, r=5%, σ=20%, T=1 year
        // Expected call price ≈ 10.4506 (QuantLib reference)
        let bs = BlackScholes::new(100.0, 100.0, 0.05, 0.20, 1.0);
        let call = bs.call_price();

        assert!(
            (call - 10.4506).abs() < 0.01,
            "Call price should be ~10.4506, got {call}"
        );
    }

    #[test]
    fn test_put_price() {
        // Same parameters, put price ≈ 5.5735
        let bs = BlackScholes::new(100.0, 100.0, 0.05, 0.20, 1.0);
        let put = bs.put_price();

        assert!(
            (put - 5.5735).abs() < 0.01,
            "Put price should be ~5.5735, got {put}"
        );
    }

    #[test]
    fn test_put_call_parity() {
        // Put-Call Parity: C - P = S*e^(-qT) - K*e^(-rT)
        let bs = BlackScholes::new(100.0, 100.0, 0.05, 0.20, 1.0);
        let call = bs.call_price();
        let put = bs.put_price();

        let lhs = call - put;
        let rhs = 100.0f64.mul_add(-(-0.05_f64).exp(), 100.0);

        assert!(
            (lhs - rhs).abs() < 0.0001,
            "Put-call parity violated: {lhs} != {rhs}"
        );
    }

    #[test]
    fn test_greeks() {
        let bs = BlackScholes::new(100.0, 100.0, 0.05, 0.20, 1.0);

        // Delta should be between 0 and 1 for call
        let delta = bs.delta_call();
        assert!(delta > 0.0 && delta < 1.0);
        assert!((delta - 0.6368).abs() < 0.01); // Expected ~0.6368

        // Gamma should be positive
        let gamma = bs.gamma();
        assert!(gamma > 0.0);

        // Vega should be positive
        let vega = bs.vega();
        assert!(vega > 0.0);
    }

    #[test]
    fn test_deep_itm_call() {
        // Deep in-the-money call: S=150, K=100
        // Should be close to intrinsic value plus time value
        let bs = BlackScholes::new(150.0, 100.0, 0.05, 0.20, 1.0);
        let call = bs.call_price();

        // Intrinsic value = 50
        assert!(call > 50.0, "ITM call should be > intrinsic value");
        assert!(call < 60.0, "ITM call should be reasonable");
    }

    #[test]
    fn test_deep_otm_call() {
        // Deep out-of-the-money call: S=50, K=100
        let bs = BlackScholes::new(50.0, 100.0, 0.05, 0.20, 1.0);
        let call = bs.call_price();

        // Should be small but positive
        assert!(call > 0.0 && call < 1.0);
    }

    #[test]
    fn test_with_dividend() {
        // Dividend-paying stock
        let bs = BlackScholes::new(100.0, 100.0, 0.05, 0.20, 1.0).with_dividend_yield(0.02);
        let call = bs.call_price();

        // Dividend reduces call value
        let bs_no_div = BlackScholes::new(100.0, 100.0, 0.05, 0.20, 1.0);
        let call_no_div = bs_no_div.call_price();

        assert!(call < call_no_div, "Dividend should reduce call value");
    }
}
