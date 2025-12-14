#!/usr/bin/env python3
"""
Monte Carlo Distribution Validation Script

This script validates Monte Carlo distributions against scipy/numpy by:
1. Generating samples from specified distributions
2. Computing descriptive statistics (mean, std, percentiles)
3. Performing Kolmogorov-Smirnov tests for goodness-of-fit

Usage:
    python validate_mc.py <distribution_type> <params_json> <n_samples>

Example:
    python validate_mc.py normal '{"mu": 100, "sigma": 15}' 10000
"""

import argparse
import json
import sys
from typing import Dict, Any, List

import numpy as np
from scipy import stats


class DistributionValidator:
    """Validates Monte Carlo distributions against reference implementations."""

    def __init__(self, dist_type: str, params: Dict[str, Any], n_samples: int):
        """
        Initialize the validator.

        Args:
            dist_type: Type of distribution (normal, triangular, uniform, pert, lognormal, discrete)
            params: Distribution parameters as a dictionary
            n_samples: Number of samples to generate
        """
        self.dist_type = dist_type.lower()
        self.params = params
        self.n_samples = n_samples
        self.samples = None

    def generate_samples(self) -> np.ndarray:
        """
        Generate samples from the specified distribution.

        Returns:
            Array of samples

        Raises:
            ValueError: If distribution type is unknown or parameters are invalid
        """
        if self.dist_type == "normal":
            return self._generate_normal()
        elif self.dist_type == "triangular":
            return self._generate_triangular()
        elif self.dist_type == "uniform":
            return self._generate_uniform()
        elif self.dist_type == "pert":
            return self._generate_pert()
        elif self.dist_type == "lognormal":
            return self._generate_lognormal()
        elif self.dist_type == "discrete":
            return self._generate_discrete()
        else:
            raise ValueError(
                f"Unknown distribution type: {self.dist_type}. "
                f"Supported types: normal, triangular, uniform, pert, lognormal, discrete"
            )

    def _generate_normal(self) -> np.ndarray:
        """Generate samples from a normal distribution."""
        try:
            mu = self.params["mu"]
            sigma = self.params["sigma"]
            if sigma <= 0:
                raise ValueError("sigma must be positive")
            return np.random.normal(mu, sigma, self.n_samples)
        except KeyError as e:
            raise ValueError(f"Missing parameter for normal distribution: {e}")

    def _generate_triangular(self) -> np.ndarray:
        """Generate samples from a triangular distribution."""
        try:
            a = self.params["a"]  # minimum
            m = self.params["m"]  # mode
            b = self.params["b"]  # maximum
            if not (a <= m <= b):
                raise ValueError("Triangular distribution requires a <= m <= b")
            return np.random.triangular(a, m, b, self.n_samples)
        except KeyError as e:
            raise ValueError(f"Missing parameter for triangular distribution: {e}")

    def _generate_uniform(self) -> np.ndarray:
        """Generate samples from a uniform distribution."""
        try:
            a = self.params["a"]  # minimum
            b = self.params["b"]  # maximum
            if a >= b:
                raise ValueError("Uniform distribution requires a < b")
            return np.random.uniform(a, b, self.n_samples)
        except KeyError as e:
            raise ValueError(f"Missing parameter for uniform distribution: {e}")

    def _generate_pert(self) -> np.ndarray:
        """
        Generate samples from a PERT distribution.

        PERT (Program Evaluation and Review Technique) distribution uses
        Beta distribution with special parameterization:
            alpha = 1 + 4 * (mode - min) / (max - min)
            beta = 1 + 4 * (max - mode) / (max - min)

        The Beta distribution is then scaled from [0, 1] to [min, max].
        """
        try:
            a = self.params["a"]  # minimum
            m = self.params["m"]  # mode
            b = self.params["b"]  # maximum

            if not (a < m < b):
                raise ValueError("PERT distribution requires a < m < b")

            # Calculate Beta distribution parameters
            range_val = b - a
            alpha = 1 + 4 * (m - a) / range_val
            beta_param = 1 + 4 * (b - m) / range_val

            # Generate samples from Beta(alpha, beta) and scale to [a, b]
            beta_samples = np.random.beta(alpha, beta_param, self.n_samples)
            return a + beta_samples * range_val

        except KeyError as e:
            raise ValueError(f"Missing parameter for PERT distribution: {e}")

    def _generate_lognormal(self) -> np.ndarray:
        """Generate samples from a lognormal distribution."""
        try:
            mu = self.params["mu"]
            sigma = self.params["sigma"]
            if sigma <= 0:
                raise ValueError("sigma must be positive")
            return np.random.lognormal(mu, sigma, self.n_samples)
        except KeyError as e:
            raise ValueError(f"Missing parameter for lognormal distribution: {e}")

    def _generate_discrete(self) -> np.ndarray:
        """Generate samples from a discrete distribution."""
        try:
            values = self.params["values"]
            probs = self.params["probs"]

            if len(values) != len(probs):
                raise ValueError("Length of values and probs must match")

            if not np.isclose(sum(probs), 1.0):
                raise ValueError(f"Probabilities must sum to 1.0, got {sum(probs)}")

            if any(p < 0 for p in probs):
                raise ValueError("Probabilities must be non-negative")

            return np.random.choice(values, size=self.n_samples, p=probs)

        except KeyError as e:
            raise ValueError(f"Missing parameter for discrete distribution: {e}")

    def compute_statistics(self) -> Dict[str, float]:
        """
        Compute descriptive statistics for the generated samples.

        Returns:
            Dictionary containing mean, std, and percentiles (p10, p50, p90)
        """
        if self.samples is None:
            self.samples = self.generate_samples()

        return {
            "mean": float(np.mean(self.samples)),
            "std": float(np.std(self.samples, ddof=1)),  # Sample standard deviation
            "p10": float(np.percentile(self.samples, 10)),
            "p50": float(np.percentile(self.samples, 50)),
            "p90": float(np.percentile(self.samples, 90)),
        }

    def compute_ks_test(self) -> float:
        """
        Perform Kolmogorov-Smirnov test for normal distribution.

        Returns:
            KS test p-value, or None if not applicable

        Note:
            Only performs KS test for normal distribution as specified.
        """
        if self.dist_type != "normal":
            return None

        if self.samples is None:
            self.samples = self.generate_samples()

        mu = self.params["mu"]
        sigma = self.params["sigma"]

        # Perform KS test against theoretical normal distribution
        ks_statistic, p_value = stats.kstest(
            self.samples, lambda x: stats.norm.cdf(x, mu, sigma)
        )

        return float(p_value)

    def validate(self) -> Dict[str, Any]:
        """
        Validate the distribution by generating samples and computing statistics.

        Returns:
            Dictionary containing statistics and KS test results (if applicable)
        """
        # Generate samples once
        self.samples = self.generate_samples()

        # Compute statistics
        result = self.compute_statistics()

        # Add KS test for normal distribution
        ks_p_value = self.compute_ks_test()
        if ks_p_value is not None:
            result["ks_p_value"] = ks_p_value

        return result


def main():
    """Main entry point for the validation script."""
    parser = argparse.ArgumentParser(
        description="Validate Monte Carlo distributions against scipy/numpy",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python validate_mc.py normal '{"mu": 100, "sigma": 15}' 10000
  python validate_mc.py triangular '{"a": 50, "m": 100, "b": 150}' 10000
  python validate_mc.py uniform '{"a": 50, "b": 150}' 10000
  python validate_mc.py pert '{"a": 50, "m": 100, "b": 150}' 10000
  python validate_mc.py lognormal '{"mu": 4.6, "sigma": 0.5}' 10000
  python validate_mc.py discrete '{"values": [1,2,3], "probs": [0.2,0.5,0.3]}' 10000
        """,
    )

    parser.add_argument(
        "distribution_type",
        type=str,
        help="Type of distribution (normal, triangular, uniform, pert, lognormal, discrete)",
    )

    parser.add_argument(
        "params_json",
        type=str,
        help="Distribution parameters as JSON string",
    )

    parser.add_argument(
        "n_samples",
        type=int,
        help="Number of samples to generate",
    )

    args = parser.parse_args()

    try:
        # Parse parameters
        params = json.loads(args.params_json)

        # Validate n_samples
        if args.n_samples <= 0:
            raise ValueError("n_samples must be positive")

        # Create validator and run validation
        validator = DistributionValidator(args.distribution_type, params, args.n_samples)
        results = validator.validate()

        # Output results as JSON
        print(json.dumps(results, indent=2))

    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in params_json: {e}", file=sys.stderr)
        sys.exit(1)

    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    except Exception as e:
        print(f"Error: Unexpected error occurred: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
