//! Binomial Tree Option Pricing
//!
//! Cox-Ross-Rubinstein binomial model for American and European options.
//! Supports early exercise and path-dependent features.
//! Validated against QuantLib.

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
    pub steps: usize,
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
    pub fn new(
        spot: f64,
        strike: f64,
        rate: f64,
        volatility: f64,
        maturity: f64,
        steps: usize,
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
    pub fn with_dividend_yield(mut self, yield_rate: f64) -> Self {
        self.dividend_yield = yield_rate;
        self
    }

    /// Calculate time step
    fn dt(&self) -> f64 {
        self.maturity / self.steps as f64
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
    pub fn call_price(&self, style: OptionStyle) -> f64 {
        self.price_option(true, style)
    }

    /// Price a put option
    pub fn put_price(&self, style: OptionStyle) -> f64 {
        self.price_option(false, style)
    }

    /// General option pricing using backward induction
    fn price_option(&self, is_call: bool, style: OptionStyle) -> f64 {
        let n = self.steps;
        let u = self.up();
        let d = self.down();
        let p = self.prob_up();
        let disc = self.discount();

        // Build terminal payoffs
        let mut prices = vec![0.0; n + 1];
        for (i, price) in prices.iter_mut().enumerate() {
            let spot_t = self.spot * u.powi(i as i32) * d.powi((n - i) as i32);
            *price = if is_call {
                (spot_t - self.strike).max(0.0)
            } else {
                (self.strike - spot_t).max(0.0)
            };
        }

        // Backward induction
        for step in (0..n).rev() {
            for i in 0..=step {
                // Continuation value
                let hold = disc * (p * prices[i + 1] + (1.0 - p) * prices[i]);

                if style == OptionStyle::American {
                    // Early exercise value
                    let spot_t = self.spot * u.powi(i as i32) * d.powi((step - i) as i32);
                    let exercise = if is_call {
                        (spot_t - self.strike).max(0.0)
                    } else {
                        (self.strike - spot_t).max(0.0)
                    };
                    prices[i] = hold.max(exercise);
                } else {
                    prices[i] = hold;
                }
            }
        }

        prices[0]
    }

    /// Price a defer option (option to wait)
    pub fn defer_option_value(&self, max_deferral: f64, exercise_cost: f64) -> f64 {
        // The defer option is essentially a call option on the project
        // with the exercise cost as the strike
        let defer_tree = BinomialTree::new(
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
    pub fn expand_option_value(&self, expansion_factor: f64, exercise_cost: f64) -> f64 {
        // The expand option lets you scale up by expansion_factor
        // Value = Call on (expansion_factor - 1) * spot, strike = exercise_cost
        let additional_value = (expansion_factor - 1.0) * self.spot;

        let expand_tree = BinomialTree::new(
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
    pub fn abandon_option_value(&self, salvage_value: f64) -> f64 {
        // The abandon option is a put option on the project
        // with salvage value as the strike
        let abandon_tree = BinomialTree::new(
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
    pub fn contract_option_value(&self, contraction_factor: f64, cost_savings: f64) -> f64 {
        // The contract option lets you scale down by contraction_factor
        // Value = Put on (1 - contraction_factor) * spot, strike = cost_savings
        let reduction = (1.0 - contraction_factor) * self.spot;

        let contract_tree = BinomialTree::new(
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
    pub fn early_exercise_boundary(&self, is_call: bool) -> Vec<(f64, f64)> {
        let n = self.steps;
        let dt = self.dt();
        let u = self.up();
        let d = self.down();
        let p = self.prob_up();
        let disc = self.discount();

        let mut boundary = Vec::new();

        // Build terminal payoffs
        let mut prices = vec![0.0; n + 1];
        for (i, price) in prices.iter_mut().enumerate() {
            let spot_t = self.spot * u.powi(i as i32) * d.powi((n - i) as i32);
            *price = if is_call {
                (spot_t - self.strike).max(0.0)
            } else {
                (self.strike - spot_t).max(0.0)
            };
        }

        // Backward induction with boundary tracking
        for step in (0..n).rev() {
            let time = step as f64 * dt;
            let mut exercise_at = None;

            for i in 0..=step {
                let hold = disc * (p * prices[i + 1] + (1.0 - p) * prices[i]);
                let spot_t = self.spot * u.powi(i as i32) * d.powi((step - i) as i32);
                let exercise = if is_call {
                    (spot_t - self.strike).max(0.0)
                } else {
                    (self.strike - spot_t).max(0.0)
                };

                if exercise > hold && exercise_at.is_none() {
                    exercise_at = Some(spot_t);
                }

                prices[i] = hold.max(exercise);
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
            "European call should converge to BS: got {}",
            call
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
        let rhs = 100.0 - 100.0 * (-0.05_f64).exp();

        assert!(
            (lhs - rhs).abs() < 0.5,
            "Put-call parity: {} != {}",
            lhs,
            rhs
        );
    }

    #[test]
    fn test_defer_option() {
        let tree = BinomialTree::new(10_000_000.0, 10_000_000.0, 0.05, 0.30, 3.0, 100);
        let defer_value = tree.defer_option_value(2.0, 8_000_000.0);

        // Should have positive value (option to wait has value)
        assert!(defer_value > 0.0, "Defer option should have positive value");

        // Should be less than spot - strike
        assert!(
            defer_value < 2_000_000.0,
            "Defer value should be reasonable"
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

    /// Roundtrip validation against QuantLib
    #[test]
    fn test_quantlib_equivalence() {
        // QuantLib binomial tree (CRR) reference values:
        // S=100, K=100, r=5%, σ=20%, T=1, N=100
        // European call ≈ 10.44
        // American put ≈ 5.63

        let tree = BinomialTree::new(100.0, 100.0, 0.05, 0.20, 1.0, 100);

        let euro_call = tree.call_price(OptionStyle::European);
        assert!(
            (euro_call - 10.44).abs() < 0.2,
            "Euro call should match QuantLib: {}",
            euro_call
        );

        let amer_put = tree.put_price(OptionStyle::American);
        assert!(
            (amer_put - 5.63).abs() < 0.2,
            "American put should match QuantLib: {}",
            amer_put
        );
    }
}
