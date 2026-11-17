//! Binomial Tree Option Pricing
//!
//! Cox-Ross-Rubinstein binomial model for American and European options.
//! Supports early exercise and path-dependent features.
//! Unit tests check prices against `QuantLib` reference values.

/// Binomial tree model for option pricing
pub struct BinomialTree {
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
    /// Number of time steps
    pub steps: u16,
    /// Dividend yield (continuous)
    pub dividend_yield: f64,
}

/// Option style (American vs European)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionStyle {
    /// Can only exercise at expiry
    European,
    /// Can exercise anytime
    American,
}

impl BinomialTree {
    /// Create a new binomial tree model
    #[must_use]
    pub const fn new(
        spot: f64,
        strike: f64,
        rate: f64,
        volatility: f64,
        maturity: f64,
        steps: u16,
    ) -> Self {
        Self {
            spot,
            strike,
            rate,
            volatility,
            maturity,
            steps,
            dividend_yield: 0.0,
        }
    }

    /// Set dividend yield
    #[must_use]
    pub const fn with_dividend_yield(mut self, yield_rate: f64) -> Self {
        self.dividend_yield = yield_rate;
        self
    }

    /// Calculate time step
    fn dt(&self) -> f64 {
        self.maturity / f64::from(self.steps)
    }

    /// The price at a node `ups` moves up and `downs` moves down from now
    fn spot_at(&self, (u, d): (f64, f64), ups: u16, downs: u16) -> f64 {
        self.spot * u.powi(i32::from(ups)) * d.powi(i32::from(downs))
    }

    /// Calculate up factor
    fn up(&self) -> f64 {
        (self.volatility * self.dt().sqrt()).exp()
    }

    /// Calculate down factor
    fn down(&self) -> f64 {
        1.0 / self.up()
    }

    /// Calculate risk-neutral probability of up move
    fn prob_up(&self) -> f64 {
        let dt = self.dt();
        let u = self.up();
        let d = self.down();
        let growth = ((self.rate - self.dividend_yield) * dt).exp();
        (growth - d) / (u - d)
    }

    /// Calculate discount factor per step
    fn discount(&self) -> f64 {
        (-self.rate * self.dt()).exp()
    }

    /// Price a call option
    #[must_use]
    pub fn call_price(&self, style: OptionStyle) -> f64 {
        self.price_option(true, style)
    }

    /// Price a put option
    #[must_use]
    pub fn put_price(&self, style: OptionStyle) -> f64 {
        self.price_option(false, style)
    }

    /// General option pricing using backward induction
    fn price_option(&self, is_call: bool, style: OptionStyle) -> f64 {
        let n = self.steps;
        let moves = (self.up(), self.down());
        let p = self.prob_up();
        let disc = self.discount();

        // Build terminal payoffs
        let mut prices = vec![0.0; usize::from(n) + 1];
        for (i, price) in (0..=n).zip(prices.iter_mut()) {
            let spot_t = self.spot_at(moves, i, n - i);
            *price = if is_call {
                (spot_t - self.strike).max(0.0)
            } else {
                (self.strike - spot_t).max(0.0)
            };
        }

        // Backward induction
        for step in (0..n).rev() {
            for i in 0..=step {
                let at = usize::from(i);
                // Continuation value
                let hold = disc * p.mul_add(prices[at + 1], (1.0 - p) * prices[at]);

                if style == OptionStyle::American {
                    // Early exercise value
                    let spot_t = self.spot_at(moves, i, step - i);
                    let exercise = if is_call {
                        (spot_t - self.strike).max(0.0)
                    } else {
                        (self.strike - spot_t).max(0.0)
                    };
                    prices[at] = hold.max(exercise);
                } else {
                    prices[at] = hold;
                }
            }
        }

        prices[0]
    }

    /// Price a defer option (option to wait)
    #[must_use]
    pub fn defer_option_value(&self, max_deferral: f64, exercise_cost: f64) -> f64 {
        // The defer option is essentially a call option on the project
        // with the exercise cost as the strike
        let defer_tree = Self::new(
            self.spot,
            exercise_cost,
            self.rate,
            self.volatility,
            max_deferral.min(self.maturity),
            self.steps,
        )
        .with_dividend_yield(self.dividend_yield);

        defer_tree.call_price(OptionStyle::American)
    }

    /// Price an expand option
    #[must_use]
    pub fn expand_option_value(&self, expansion_factor: f64, exercise_cost: f64) -> f64 {
        // The expand option lets you scale up by expansion_factor
        // Value = Call on (expansion_factor - 1) * spot, strike = exercise_cost
        let additional_value = (expansion_factor - 1.0) * self.spot;

        let expand_tree = Self::new(
            additional_value,
            exercise_cost,
            self.rate,
            self.volatility,
            self.maturity,
            self.steps,
        )
        .with_dividend_yield(self.dividend_yield);

        expand_tree.call_price(OptionStyle::American)
    }

    /// Price an abandon option
    #[must_use]
    pub fn abandon_option_value(&self, salvage_value: f64) -> f64 {
        // The abandon option is a put option on the project
        // with salvage value as the strike
        let abandon_tree = Self::new(
            self.spot,
            salvage_value,
            self.rate,
            self.volatility,
            self.maturity,
            self.steps,
        )
        .with_dividend_yield(self.dividend_yield);

        abandon_tree.put_price(OptionStyle::American)
    }

    /// Price a contract option
    #[must_use]
    pub fn contract_option_value(&self, contraction_factor: f64, cost_savings: f64) -> f64 {
        // The contract option lets you scale down by contraction_factor
        // Value = Put on (1 - contraction_factor) * spot, strike = cost_savings
        let reduction = (1.0 - contraction_factor) * self.spot;

        let contract_tree = Self::new(
            reduction,
            cost_savings,
            self.rate,
            self.volatility,
            self.maturity,
            self.steps,
        )
        .with_dividend_yield(self.dividend_yield);

        contract_tree.put_price(OptionStyle::American)
    }

    /// Get early exercise boundary (for American options)
    #[must_use]
    pub fn early_exercise_boundary(&self, is_call: bool) -> Vec<(f64, f64)> {
        let n = self.steps;
        let dt = self.dt();
        let moves = (self.up(), self.down());
        let p = self.prob_up();
        let disc = self.discount();

        let mut boundary = Vec::new();

        // Build terminal payoffs
        let mut prices = vec![0.0; usize::from(n) + 1];
        for (i, price) in (0..=n).zip(prices.iter_mut()) {
            let spot_t = self.spot_at(moves, i, n - i);
            *price = if is_call {
                (spot_t - self.strike).max(0.0)
            } else {
                (self.strike - spot_t).max(0.0)
            };
        }

        // Backward induction with boundary tracking
        for step in (0..n).rev() {
            let time = f64::from(step) * dt;
            let mut exercise_at = None;

            for i in 0..=step {
                let at = usize::from(i);
                let hold = disc * p.mul_add(prices[at + 1], (1.0 - p) * prices[at]);
                let spot_t = self.spot_at(moves, i, step - i);
                let exercise = if is_call {
                    (spot_t - self.strike).max(0.0)
                } else {
                    (self.strike - spot_t).max(0.0)
                };

                if exercise > hold && exercise_at.is_none() {
                    exercise_at = Some(spot_t);
                }

                prices[at] = hold.max(exercise);
            }

            if let Some(spot_boundary) = exercise_at {
                boundary.push((time, spot_boundary));
            }
        }

        boundary
    }
}

#[cfg(test)]
mod binomial_tests {
    use super::*;

    /// Test convergence to Black-Scholes for European options
    #[test]
    fn test_european_convergence() {
        // With enough steps, binomial should converge to Black-Scholes
        // BS call ≈ 10.4506 for S=100, K=100, r=5%, σ=20%, T=1
        let tree = BinomialTree::new(100.0, 100.0, 0.05, 0.20, 1.0, 200);
        let call = tree.call_price(OptionStyle::European);

        assert!(
            (call - 10.4506).abs() < 0.1,
            "European call should converge to BS: got {call}"
        );
    }

    #[test]
    fn test_american_premium() {
        // American put should be worth more than European put
        let tree = BinomialTree::new(100.0, 100.0, 0.05, 0.20, 1.0, 100);
        let euro_put = tree.put_price(OptionStyle::European);
        let amer_put = tree.put_price(OptionStyle::American);

        assert!(
            amer_put >= euro_put,
            "American put should be >= European put"
        );
    }

    #[test]
    fn test_put_call_parity_european() {
        let tree = BinomialTree::new(100.0, 100.0, 0.05, 0.20, 1.0, 100);
        let call = tree.call_price(OptionStyle::European);
        let put = tree.put_price(OptionStyle::European);

        let lhs = call - put;
        let rhs = 100.0f64.mul_add(-(-0.05_f64).exp(), 100.0);

        assert!((lhs - rhs).abs() < 0.5, "Put-call parity: {lhs} != {rhs}");
    }

    #[test]
    fn test_defer_option() {
        let tree = BinomialTree::new(10_000_000.0, 10_000_000.0, 0.05, 0.30, 3.0, 100);
        let defer_value = tree.defer_option_value(2.0, 8_000_000.0);

        // Should have positive value (option to wait has value)
        assert!(defer_value > 0.0, "Defer option should have positive value");

        // For a call with S=10M, K=8M, T=2, σ=30%:
        // Intrinsic value = 2M (minimum for in-the-money American call)
        // With time value, should be between intrinsic and spot price
        assert!(
            defer_value >= 2_000_000.0,
            "Defer value should be at least intrinsic value (2M): {defer_value}"
        );
        assert!(
            defer_value < 10_000_000.0,
            "Defer value should be less than spot price (10M): {defer_value}"
        );
    }

    #[test]
    fn test_abandon_option() {
        let tree = BinomialTree::new(10_000_000.0, 10_000_000.0, 0.05, 0.30, 3.0, 100);
        let abandon_value = tree.abandon_option_value(3_000_000.0);

        // Should have positive value
        assert!(
            abandon_value > 0.0,
            "Abandon option should have positive value"
        );
    }

    #[test]
    fn test_expand_option() {
        let tree = BinomialTree::new(10_000_000.0, 10_000_000.0, 0.05, 0.30, 3.0, 100);
        let expand_value = tree.expand_option_value(1.5, 4_000_000.0);

        // Should have positive value
        assert!(
            expand_value > 0.0,
            "Expand option should have positive value"
        );
    }

    /// Checked against `QuantLib` reference values (loose tolerance)
    #[test]
    fn test_quantlib_equivalence() {
        // QuantLib binomial tree (CRR) reference values:
        // S=100, K=100, r=5%, σ=20%, T=1, N=100
        // European call ≈ 10.44
        // American put ≈ 6.08 (not 5.63 - that's European put)

        let tree = BinomialTree::new(100.0, 100.0, 0.05, 0.20, 1.0, 100);

        let euro_call = tree.call_price(OptionStyle::European);
        assert!(
            (euro_call - 10.44).abs() < 0.2,
            "Euro call should match QuantLib: {euro_call}"
        );

        let amer_put = tree.put_price(OptionStyle::American);
        assert!(
            (amer_put - 6.08).abs() < 0.2,
            "American put should match QuantLib: {amer_put}"
        );
    }
}
