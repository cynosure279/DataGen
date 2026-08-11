//! Distribution-based random value generators.
//!
//! Each generator wraps a [`rand::distr`] or [`rand_distr`] distribution
//! and implements the [`Generator`] trait.

use rand_distr::Distribution;

use super::Generator;

// ---------------------------------------------------------------------------
// UniformIntGen
// ---------------------------------------------------------------------------

/// Generates uniformly distributed `i64` values in the inclusive range `[min, max]`.
pub struct UniformIntGen {
    dist: rand::distr::Uniform<i64>,
}

impl UniformIntGen {
    /// Create a new uniform integer generator.
    ///
    /// # Panics
    /// Panics if `min > max`.
    pub fn new(min: i64, max: i64) -> Self {
        assert!(min <= max, "UniformIntGen: min must be <= max, got {min} > {max}");
        Self {
            dist: rand::distr::Uniform::new_inclusive(min, max)
                .expect("Uniform::new_inclusive should succeed for min <= max"),
        }
    }
}

impl Generator for UniformIntGen {
    type Output = i64;

    fn generate(&mut self, rng: &mut impl rand::Rng) -> i64 {
        self.dist.sample(rng)
    }
}

// ---------------------------------------------------------------------------
// UniformFloatGen
// ---------------------------------------------------------------------------

/// Generates uniformly distributed `f64` values in the range `[min, max)`.
pub struct UniformFloatGen {
    dist: rand::distr::Uniform<f64>,
}

impl UniformFloatGen {
    /// Create a new uniform float generator.
    ///
    /// # Panics
    /// Panics if `min > max` or if the range is non-finite.
    pub fn new(min: f64, max: f64) -> Self {
        assert!(
            min.is_finite() && max.is_finite(),
            "UniformFloatGen: bounds must be finite"
        );
        Self {
            dist: rand::distr::Uniform::new(min, max)
                .expect("Uniform::new should succeed for valid bounds"),
        }
    }
}

impl Generator for UniformFloatGen {
    type Output = f64;

    fn generate(&mut self, rng: &mut impl rand::Rng) -> f64 {
        self.dist.sample(rng)
    }
}

// ---------------------------------------------------------------------------
// NormalGen
// ---------------------------------------------------------------------------

/// Generates normally distributed `f64` values with optional clamping.
pub struct NormalGen {
    dist: rand_distr::Normal<f64>,
    clamp_min: Option<f64>,
    clamp_max: Option<f64>,
}

impl NormalGen {
    /// Create a new normal (Gaussian) generator.
    ///
    /// # Panics
    /// Panics if `std_dev <= 0` or if `std_dev` is non-finite.
    pub fn new(mean: f64, std_dev: f64) -> Self {
        Self {
            dist: rand_distr::Normal::new(mean, std_dev)
                .expect("Normal::new should succeed for valid parameters"),
            clamp_min: None,
            clamp_max: None,
        }
    }

    /// Set a lower clamp bound. Generated values below this bound are clamped.
    pub fn with_clamp_min(mut self, min: f64) -> Self {
        self.clamp_min = Some(min);
        self
    }

    /// Set an upper clamp bound. Generated values above this bound are clamped.
    pub fn with_clamp_max(mut self, max: f64) -> Self {
        self.clamp_max = Some(max);
        self
    }
}

impl Generator for NormalGen {
    type Output = f64;

    fn generate(&mut self, rng: &mut impl rand::Rng) -> f64 {
        let mut val = self.dist.sample(rng);
        if let Some(min) = self.clamp_min {
            val = val.max(min);
        }
        if let Some(max) = self.clamp_max {
            val = val.min(max);
        }
        val
    }
}

// ---------------------------------------------------------------------------
// ExponentialGen
// ---------------------------------------------------------------------------

/// Generates exponentially distributed `f64` values.
///
/// The distribution has rate parameter `lambda` (mean = 1/lambda).
pub struct ExponentialGen {
    dist: rand_distr::Exp<f64>,
}

impl ExponentialGen {
    /// Create a new exponential generator.
    ///
    /// # Panics
    /// Panics if `lambda <= 0` or if `lambda` is non-finite.
    pub fn new(lambda: f64) -> Self {
        assert!(lambda > 0.0, "ExponentialGen: lambda must be > 0, got {lambda}");
        Self {
            dist: rand_distr::Exp::new(lambda)
                .expect("Exp::new should succeed for lambda > 0"),
        }
    }
}

impl Generator for ExponentialGen {
    type Output = f64;

    fn generate(&mut self, rng: &mut impl rand::Rng) -> f64 {
        self.dist.sample(rng)
    }
}

// ---------------------------------------------------------------------------
// PoissonGen
// ---------------------------------------------------------------------------

/// Generates Poisson-distributed `u64` values.
///
/// The distribution has mean `lambda`.
pub struct PoissonGen {
    dist: rand_distr::Poisson<f64>,
}

impl PoissonGen {
    /// Create a new Poisson generator.
    ///
    /// # Panics
    /// Panics if `lambda <= 0` or if `lambda` is non-finite.
    pub fn new(lambda: f64) -> Self {
        Self {
            dist: rand_distr::Poisson::new(lambda)
                .expect("Poisson::new should succeed for lambda > 0"),
        }
    }
}

impl Generator for PoissonGen {
    type Output = u64;

    fn generate(&mut self, rng: &mut impl rand::Rng) -> u64 {
        // Poisson<f64> samples f64; cast to u64 per spec
        self.dist.sample(rng) as u64
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// Helper: deterministic RNG with fixed seed.
    fn rng_fixed() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    /// Error function approximation (Abramowitz & Stegun).
    /// Maximum error ≈ 1.5e-7.
    fn erf(x: f64) -> f64 {
        let sign = if x >= 0.0 { 1.0 } else { -1.0 };
        let x = x.abs();
        let p = 0.3275911;
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
        sign * y
    }

    /// Standard normal CDF via the error function.
    fn normal_cdf(x: f64, mean: f64, std_dev: f64) -> f64 {
        0.5 * (1.0 + erf((x - mean) / (std_dev * std::f64::consts::SQRT_2)))
    }

    /// Two-sided Kolmogorov–Smirnov p-value approximation.
    fn ks_p_value(n: usize, d: f64) -> f64 {
        let nf = n as f64;
        let mut p = 0.0;
        for k in 1..100 {
            let term = (-2.0 * (k as f64).powi(2) * nf * d * d).exp();
            let sign = if k % 2 == 1 { 2.0 } else { -2.0 };
            p += sign * term;
            if term.abs() < 1e-15 {
                break;
            }
        }
        p.clamp(0.0, 1.0)
    }

    // -----------------------------------------------------------------------
    // UniformIntGen
    // -----------------------------------------------------------------------

    #[test]
    fn uniform_int_single_value() {
        // min == max should always produce that value
        let mut gen = UniformIntGen::new(5, 5);
        let mut rng = rng_fixed();
        for _ in 0..100 {
            assert_eq!(gen.generate(&mut rng), 5);
        }
    }

    #[test]
    fn uniform_int_deterministic_sequence() {
        let mut gen = UniformIntGen::new(1, 100);
        let mut rng1 = rng_fixed();
        let mut rng2 = rng_fixed();
        let seq1: Vec<i64> = (0..20).map(|_| gen.generate(&mut rng1)).collect();
        let seq2: Vec<i64> = (0..20).map(|_| gen.generate(&mut rng2)).collect();
        assert_eq!(seq1, seq2, "same seed must produce same sequence");
    }

    #[test]
    fn uniform_int_in_range() {
        let mut gen = UniformIntGen::new(-10, 10);
        let mut rng = rng_fixed();
        for _ in 0..1000 {
            let val = gen.generate(&mut rng);
            assert!(
                (-10..=10).contains(&val),
                "value {val} out of range [-10, 10]"
            );
        }
    }

    #[test]
    #[should_panic(expected = "min must be <= max")]
    fn uniform_int_min_greater_than_max() {
        UniformIntGen::new(10, 5);
    }

    // -----------------------------------------------------------------------
    // UniformFloatGen
    // -----------------------------------------------------------------------

    #[test]
    fn uniform_float_deterministic() {
        let mut gen = UniformFloatGen::new(0.0, 1.0);
        let mut rng1 = rng_fixed();
        let mut rng2 = rng_fixed();
        let seq1: Vec<f64> = (0..20).map(|_| gen.generate(&mut rng1)).collect();
        let seq2: Vec<f64> = (0..20).map(|_| gen.generate(&mut rng2)).collect();
        assert_eq!(seq1, seq2);
    }

    #[test]
    fn uniform_float_in_range() {
        let mut gen = UniformFloatGen::new(-5.0, 5.0);
        let mut rng = rng_fixed();
        for _ in 0..1000 {
            let val = gen.generate(&mut rng);
            assert!(val >= -5.0 && val < 5.0, "value {val} out of range [-5, 5)");
        }
    }

    // -----------------------------------------------------------------------
    // NormalGen — K-S test
    // -----------------------------------------------------------------------

    #[test]
    fn normal_ks_test() {
        let mut gen = NormalGen::new(0.0, 1.0);
        let mut rng = rng_fixed();
        let n = 10_000;
        let mut samples: Vec<f64> = (0..n).map(|_| gen.generate(&mut rng)).collect();
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Compute K-S statistic D
        let mut d = 0.0;
        for (i, &x) in samples.iter().enumerate() {
            let ecdf = (i as f64 + 1.0) / n as f64;
            let cdf = normal_cdf(x, 0.0, 1.0);
            let diff = (ecdf - cdf).abs();
            if diff > d {
                d = diff;
            }
        }

        let p = ks_p_value(n, d);
        assert!(
            p > 0.01,
            "K-S test p-value {p} <= 0.01 (D={d}), samples may not be normal"
        );
    }

    #[test]
    fn normal_deterministic() {
        let mut gen = NormalGen::new(5.0, 2.0);
        let mut rng1 = rng_fixed();
        let mut rng2 = rng_fixed();
        let seq1: Vec<f64> = (0..20).map(|_| gen.generate(&mut rng1)).collect();
        let seq2: Vec<f64> = (0..20).map(|_| gen.generate(&mut rng2)).collect();
        assert_eq!(seq1, seq2);
    }

    #[test]
    fn normal_clamp_bounds() {
        let mut gen = NormalGen::new(0.0, 10.0)
            .with_clamp_min(-1.0)
            .with_clamp_max(1.0);
        let mut rng = rng_fixed();
        for _ in 0..1000 {
            let val = gen.generate(&mut rng);
            assert!(val >= -1.0 && val <= 1.0, "clamped value {val} out of bounds");
        }
    }

    // -----------------------------------------------------------------------
    // ExponentialGen
    // -----------------------------------------------------------------------

    #[test]
    fn exponential_mean_approx_one() {
        let mut gen = ExponentialGen::new(1.0);
        let mut rng = rng_fixed();
        let n = 10_000;
        let sum: f64 = (0..n).map(|_| gen.generate(&mut rng)).sum();
        let mean = sum / n as f64;
        // Mean of Exp(1) is 1.0; allow 10% tolerance
        assert!(
            (mean - 1.0).abs() < 0.1,
            "mean {mean} too far from 1.0"
        );
    }

    #[test]
    fn exponential_deterministic() {
        let mut gen = ExponentialGen::new(2.0);
        let mut rng1 = rng_fixed();
        let mut rng2 = rng_fixed();
        let seq1: Vec<f64> = (0..20).map(|_| gen.generate(&mut rng1)).collect();
        let seq2: Vec<f64> = (0..20).map(|_| gen.generate(&mut rng2)).collect();
        assert_eq!(seq1, seq2);
    }

    #[test]
    #[should_panic(expected = "lambda must be > 0")]
    fn exponential_lambda_zero_panics() {
        ExponentialGen::new(0.0);
    }

    // -----------------------------------------------------------------------
    // PoissonGen
    // -----------------------------------------------------------------------

    #[test]
    fn poisson_mean_approx_five() {
        let mut gen = PoissonGen::new(5.0);
        let mut rng = rng_fixed();
        let n = 10_000;
        let sum: u64 = (0..n).map(|_| gen.generate(&mut rng)).sum();
        let mean = sum as f64 / n as f64;
        // Mean of Poisson(5) is 5.0; allow 10% tolerance
        assert!(
            (mean - 5.0).abs() < 0.5,
            "mean {mean} too far from 5.0"
        );
    }

    #[test]
    fn poisson_deterministic() {
        let mut gen = PoissonGen::new(3.0);
        let mut rng1 = rng_fixed();
        let mut rng2 = rng_fixed();
        let seq1: Vec<u64> = (0..20).map(|_| gen.generate(&mut rng1)).collect();
        let seq2: Vec<u64> = (0..20).map(|_| gen.generate(&mut rng2)).collect();
        assert_eq!(seq1, seq2);
    }

    #[test]
    fn poisson_output_type_u64() {
        let mut gen = PoissonGen::new(10.0);
        let mut rng = rng_fixed();
        let val = gen.generate(&mut rng);
        // Verify it's a sensible u64 value
        assert!(val < 100, "Poisson(10) sample {val} seems too large");
    }
}