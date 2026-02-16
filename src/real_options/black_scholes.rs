//! Black-Scholes Option Pricing
//!
//! Closed-form solution for European options.
//! Validated against `QuantLib`.

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
    fn norm_cdf(x: f64) -> f64 {
        // Approximation using error function
        0.5 * (1.0 + erf(x / 2.0_f64.sqrt()))
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

/// Error function approximation (for normal CDF)
fn erf(x: f64) -> f64 {
    // Abramowitz and Stegun approximation
    let t = 1.0 / 0.5f64.mul_add(x.abs(), 1.0);

    let tau = t * 0.170_872_77_f64
        .mul_add(
            t.powi(9),
            0.822_152_23_f64.mul_add(
                -t.powi(8),
                1.488_515_87_f64.mul_add(
                    t.powi(7),
                    1.135_203_98_f64.mul_add(
                        -t.powi(6),
                        0.278_868_07_f64.mul_add(
                            t.powi(5),
                            0.186_288_06_f64.mul_add(
                                -t.powi(4),
                                0.096_784_18_f64.mul_add(
                                    t.powi(3),
                                    0.374_091_96_f64.mul_add(
                                        t.powi(2),
                                        1.000_023_68_f64.mul_add(t, -x.powi(2) - 1.265_512_23),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            ),
        )
        .exp();

    if x >= 0.0 {
        1.0 - tau
    } else {
        tau - 1.0
    }
}

#[cfg(test)]
mod black_scholes_tests {
    use super::*;

    /// Test against known Black-Scholes values
    /// Validated against `QuantLib`
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
