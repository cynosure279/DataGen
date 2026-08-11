//! Number theory generators: primes, factorization, modular arithmetic, CRT, primitive roots.

use num_modular::{ModularCoreOps, ModularPow, ModularUnaryOps};
use num_prime::nt_funcs::{factorize64, is_prime64};

use crate::gen::Generator;

// ---------------------------------------------------------------------------
// PrimeGen — generate primes in a range
// ---------------------------------------------------------------------------

/// Generate all primes in a given range.
///
/// Uses Sieve of Eratosthenes when the upper bound ≤ 1_000_000,
/// otherwise falls back to deterministic Miller–Rabin (`is_prime64`).
pub struct PrimeGen {
    range: std::ops::RangeInclusive<u64>,
}

impl PrimeGen {
    pub fn new(range: std::ops::RangeInclusive<u64>) -> Self {
        Self { range }
    }
}

impl Generator for PrimeGen {
    type Output = Vec<u64>;

    fn generate(&mut self, _rng: &mut impl rand::Rng) -> Vec<u64> {
        let (lo, hi) = self.range.clone().into_inner();
        if lo > hi || hi < 2 {
            return vec![];
        }
        let lo = lo.max(2);

        if hi <= 1_000_000 {
            sieve_range(lo, hi)
        } else {
            (lo..=hi).filter(|&n| is_prime64(n)).collect()
        }
    }
}

/// Sieve of Eratosthenes for `[lo, hi]` where `hi ≤ 1_000_000`.
fn sieve_range(lo: u64, hi: u64) -> Vec<u64> {
    let n = hi as usize;
    let mut is_prime = vec![true; n + 1];
    if n >= 1 {
        is_prime[1] = false;
    }
    is_prime[0] = false;

    let limit = (n as f64).sqrt() as usize;
    for i in 2..=limit {
        if is_prime[i] {
            let mut j = i * i;
            while j <= n {
                is_prime[j] = false;
                j += i;
            }
        }
    }

    is_prime[lo as usize..=n]
        .iter()
        .enumerate()
        .filter(|(_, &p)| p)
        .map(|(idx, _)| lo + idx as u64)
        .collect()
}

// ---------------------------------------------------------------------------
// FactorGen — factorize a number
// ---------------------------------------------------------------------------

/// Factorize a positive integer into its prime factors (with multiplicity).
///
/// Uses trial division for `n ≤ 10^6`, otherwise delegates to
/// `num_prime::nt_funcs::factorize64` (Pollard–Rho etc.).
pub struct FactorGen {
    n: u64,
}

impl FactorGen {
    pub fn new(n: u64) -> Self {
        Self { n }
    }
}

impl Generator for FactorGen {
    type Output = Vec<u64>;

    fn generate(&mut self, _rng: &mut impl rand::Rng) -> Vec<u64> {
        factorize(self.n)
    }
}

/// Internal factorization routine.
fn factorize(n: u64) -> Vec<u64> {
    if n <= 1 {
        return vec![];
    }

    if n <= 1_000_000 {
        trial_division(n)
    } else {
        let map = factorize64(n);
        let mut result = Vec::new();
        for (p, k) in &map {
            for _ in 0..*k {
                result.push(*p);
            }
        }
        result.sort_unstable();
        result
    }
}

/// Trial division for `n ≤ 10^6`.
fn trial_division(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();
    while n.is_multiple_of(2) {
        factors.push(2);
        n /= 2;
    }
    let mut d = 3;
    while d * d <= n {
        while n.is_multiple_of(d) {
            factors.push(d);
            n /= d;
        }
        d += 2;
    }
    if n > 1 {
        factors.push(n);
    }
    factors
}

// ---------------------------------------------------------------------------
// ModularGen — modular arithmetic operations
// ---------------------------------------------------------------------------

/// Input selector for a single modular operation.
#[derive(Debug, Clone, Copy)]
pub enum ModularOp {
    Add(u64, u64, u64),
    Sub(u64, u64, u64),
    Mul(u64, u64, u64),
    Pow(u64, u64, u64),
    Inv(u64, u64),
}

/// Perform one modular arithmetic operation.
pub struct ModularGen {
    op: ModularOp,
}

impl ModularGen {
    pub fn new(op: ModularOp) -> Self {
        Self { op }
    }

    /// `(a + b) mod m`
    pub fn add(a: u64, b: u64, m: u64) -> u64 {
        a.addm(b, &m)
    }

    /// `(a - b) mod m`  (result normalized to `[0, m)`)
    pub fn sub(a: u64, b: u64, m: u64) -> u64 {
        a.subm(b, &m)
    }

    /// `(a * b) mod m`
    pub fn mul(a: u64, b: u64, m: u64) -> u64 {
        a.mulm(b, &m)
    }

    /// `(base ^ exp) mod m`
    pub fn pow(base: u64, exp: u64, m: u64) -> u64 {
        base.powm(exp, &m)
    }

    /// Modular inverse of `a` modulo `m`.
    ///
    /// Returns `Err(reason)` when `a` and `m` are not coprime.
    pub fn inv(a: u64, m: u64) -> Result<u64, String> {
        a.invm(&m)
            .ok_or_else(|| format!("{} is not invertible modulo {}", a, m))
    }
}

impl Generator for ModularGen {
    type Output = Result<u64, String>;

    fn generate(&mut self, _rng: &mut impl rand::Rng) -> Result<u64, String> {
        match self.op {
            ModularOp::Add(a, b, m) => Ok(Self::add(a, b, m)),
            ModularOp::Sub(a, b, m) => Ok(Self::sub(a, b, m)),
            ModularOp::Mul(a, b, m) => Ok(Self::mul(a, b, m)),
            ModularOp::Pow(b, e, m) => Ok(Self::pow(b, e, m)),
            ModularOp::Inv(a, m) => Self::inv(a, m),
        }
    }
}

// ---------------------------------------------------------------------------
// CRTGen — Chinese Remainder Theorem
// ---------------------------------------------------------------------------

/// Solve a system of congruences via the Chinese Remainder Theorem.
///
/// Returns `None` if the moduli are not pairwise coprime.
pub struct CRTGen {
    remainders: Vec<u64>,
    moduli: Vec<u64>,
}

impl CRTGen {
    pub fn new(remainders: Vec<u64>, moduli: Vec<u64>) -> Self {
        Self {
            remainders,
            moduli,
        }
    }

    /// Solve `x ≡ remainders[i] (mod moduli[i])` for all `i`.
    ///
    /// Returns `None` when the moduli are not pairwise coprime or inputs are empty.
    pub fn solve(remainders: &[u64], moduli: &[u64]) -> Option<u64> {
        crt(remainders, moduli)
    }
}

impl Generator for CRTGen {
    type Output = Option<u64>;

    fn generate(&mut self, _rng: &mut impl rand::Rng) -> Option<u64> {
        crt(&self.remainders, &self.moduli)
    }
}

fn crt(remainders: &[u64], moduli: &[u64]) -> Option<u64> {
    if remainders.len() != moduli.len() || remainders.is_empty() {
        return None;
    }

    // Check pairwise coprime
    for i in 0..moduli.len() {
        for j in (i + 1)..moduli.len() {
            if gcd(moduli[i], moduli[j]) != 1 {
                return None;
            }
        }
    }

    // Compute product using u128 to avoid overflow
    let prod: u128 = moduli.iter().map(|&m| m as u128).product();
    let mut result: u128 = 0;

    for i in 0..remainders.len() {
        let ni = prod / moduli[i] as u128;
        let inv = mod_inv((ni % moduli[i] as u128) as u64, moduli[i])?;
        let term = (remainders[i] as u128) * ni * (inv as u128);
        result += term;
    }

    Some((result % prod) as u64)
}

// ---------------------------------------------------------------------------
// PrimitiveRootGen — find primitive roots modulo a prime
// ---------------------------------------------------------------------------

/// Find the smallest primitive root modulo a prime `p`.
pub struct PrimitiveRootGen {
    p: u64,
}

impl PrimitiveRootGen {
    pub fn new(p: u64) -> Self {
        Self { p }
    }

    /// Returns the smallest primitive root modulo `p`, or `None` if `p` is not prime.
    pub fn find_root(p: u64) -> Option<u64> {
        primitive_root(p)
    }
}

impl Generator for PrimitiveRootGen {
    type Output = Option<u64>;

    fn generate(&mut self, _rng: &mut impl rand::Rng) -> Option<u64> {
        primitive_root(self.p)
    }
}

fn primitive_root(p: u64) -> Option<u64> {
    if !is_prime64(p) || p < 2 {
        return None;
    }
    if p == 2 {
        return Some(1);
    }

    let phi = p - 1;
    let factors = unique_prime_factors(phi);

    'candidate: for g in 2..p {
        for &q in &factors {
            if ModularGen::pow(g, phi / q, p) == 1 {
                continue 'candidate;
            }
        }
        return Some(g);
    }
    None
}

/// Return the unique prime factors of `n` (no multiplicity).
fn unique_prime_factors(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();
    if n.is_multiple_of(2) {
        factors.push(2);
        while n.is_multiple_of(2) {
            n /= 2;
        }
    }
    let mut d = 3;
    while d * d <= n {
        if n.is_multiple_of(d) {
            factors.push(d);
            while n.is_multiple_of(d) {
                n /= d;
            }
        }
        d += 2;
    }
    if n > 1 {
        factors.push(n);
    }
    factors
}

/// Extended Euclidean algorithm — returns `(gcd, x, y)` where `ax + by = gcd`.
fn egcd(a: i128, b: i128) -> (i128, i128, i128) {
    if b == 0 {
        (a, 1, 0)
    } else {
        let (g, x, y) = egcd(b, a % b);
        (g, y, x - (a / b) * y)
    }
}

/// Modular inverse using extended Euclidean algorithm.
fn mod_inv(a: u64, m: u64) -> Option<u64> {
    let (g, x, _) = egcd(a as i128, m as i128);
    if g != 1 {
        return None;
    }
    Some(((x % m as i128 + m as i128) % m as i128) as u64)
}

/// Greatest common divisor.
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;

    fn test_rng() -> impl rand::Rng {
        rand::rngs::StdRng::seed_from_u64(42)
    }

    // --- PrimeGen ---

    #[test]
    fn test_prime_gen_small_range() {
        let mut gen = PrimeGen::new(1u64..=100);
        let primes = gen.generate(&mut test_rng());
        assert_eq!(primes.len(), 25);
        assert_eq!(primes[0], 2);
        assert_eq!(primes[1], 3);
        assert_eq!(primes[2], 5);
        assert_eq!(primes[3], 7);
        assert_eq!(primes[primes.len() - 1], 97);
    }

    #[test]
    fn test_prime_gen_empty() {
        let mut gen = PrimeGen::new(10u64..=1);
        let primes = gen.generate(&mut test_rng());
        assert!(primes.is_empty());
    }

    #[test]
    fn test_prime_gen_below_two() {
        let mut gen = PrimeGen::new(0u64..=1);
        let primes = gen.generate(&mut test_rng());
        assert!(primes.is_empty());
    }

    // --- FactorGen ---

    #[test]
    fn test_factor_84() {
        let mut gen = FactorGen::new(84);
        let mut factors = gen.generate(&mut test_rng());
        factors.sort_unstable();
        assert_eq!(factors, vec![2, 2, 3, 7]);
    }

    #[test]
    fn test_factor_prime() {
        let mut gen = FactorGen::new(97);
        let factors = gen.generate(&mut test_rng());
        assert_eq!(factors, vec![97]);
    }

    #[test]
    fn test_factor_zero() {
        let mut gen = FactorGen::new(0);
        let factors = gen.generate(&mut test_rng());
        assert!(factors.is_empty());
    }

    #[test]
    fn test_factor_one() {
        let mut gen = FactorGen::new(1);
        let factors = gen.generate(&mut test_rng());
        assert!(factors.is_empty());
    }

    #[test]
    fn test_factor_large_composite() {
        let n = 1_000_003u64 * 1_000_007;
        let mut gen = FactorGen::new(n);
        let mut factors = gen.generate(&mut test_rng());
        factors.sort_unstable();
        assert_eq!(factors, vec![29, 34483, 1_000_003]);
    }

    // --- ModularGen ---

    #[test]
    fn test_modular_add() {
        assert_eq!(ModularGen::add(3, 4, 7), 0);
    }

    #[test]
    fn test_modular_sub() {
        assert_eq!(ModularGen::sub(3, 4, 7), 6);
    }

    #[test]
    fn test_modular_mul() {
        assert_eq!(ModularGen::mul(3, 4, 7), 5);
    }

    #[test]
    fn test_modular_pow() {
        assert_eq!(ModularGen::pow(2, 10, 1000), 24);
    }

    #[test]
    fn test_modular_inv_ok() {
        assert_eq!(ModularGen::inv(3, 7), Ok(5));
    }

    #[test]
    fn test_modular_inv_err() {
        assert!(ModularGen::inv(2, 4).is_err());
    }

    #[test]
    fn test_modular_gen_trait() {
        let mut gen = ModularGen::new(ModularOp::Add(3, 4, 7));
        assert_eq!(gen.generate(&mut test_rng()), Ok(0));

        let mut gen = ModularGen::new(ModularOp::Sub(3, 4, 7));
        assert_eq!(gen.generate(&mut test_rng()), Ok(6));

        let mut gen = ModularGen::new(ModularOp::Mul(3, 4, 7));
        assert_eq!(gen.generate(&mut test_rng()), Ok(5));

        let mut gen = ModularGen::new(ModularOp::Pow(2, 10, 1000));
        assert_eq!(gen.generate(&mut test_rng()), Ok(24));

        let mut gen = ModularGen::new(ModularOp::Inv(3, 7));
        assert_eq!(gen.generate(&mut test_rng()), Ok(5));

        let mut gen = ModularGen::new(ModularOp::Inv(2, 4));
        assert!(gen.generate(&mut test_rng()).is_err());
    }

    // --- CRTGen ---

    #[test]
    fn test_crt_valid() {
        let result = CRTGen::solve(&[2, 3, 1], &[3, 5, 7]);
        // 8 ≡ 2 (mod 3), 8 ≡ 3 (mod 5), 8 ≡ 1 (mod 7)
        assert_eq!(result, Some(8));
    }

    #[test]
    fn test_crt_not_coprime() {
        let result = CRTGen::solve(&[1, 2], &[4, 6]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_crt_empty() {
        let result = CRTGen::solve(&[], &[]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_crt_mismatched_lengths() {
        let result = CRTGen::solve(&[1, 2], &[3]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_crt_gen_trait() {
        let mut gen = CRTGen::new(vec![2, 3, 1], vec![3, 5, 7]);
        // 8 ≡ 2 (mod 3), 8 ≡ 3 (mod 5), 8 ≡ 1 (mod 7)
        assert_eq!(gen.generate(&mut test_rng()), Some(8));

        let mut gen = CRTGen::new(vec![1, 2], vec![4, 6]);
        assert_eq!(gen.generate(&mut test_rng()), None);
    }

    // --- PrimitiveRootGen ---

    #[test]
    fn test_primitive_root_7() {
        let root = PrimitiveRootGen::find_root(7);
        assert!(root == Some(3) || root == Some(5));
    }

    #[test]
    fn test_primitive_root_not_prime() {
        let root = PrimitiveRootGen::find_root(4);
        assert_eq!(root, None);
    }

    #[test]
    fn test_primitive_root_two() {
        let root = PrimitiveRootGen::find_root(2);
        assert_eq!(root, Some(1));
    }

    #[test]
    fn test_primitive_root_below_two() {
        let root = PrimitiveRootGen::find_root(1);
        assert_eq!(root, None);
    }

    #[test]
    fn test_primitive_root_gen_trait() {
        let mut gen = PrimitiveRootGen::new(7);
        let root = gen.generate(&mut test_rng());
        assert!(root == Some(3) || root == Some(5));

        let mut gen = PrimitiveRootGen::new(4);
        assert_eq!(gen.generate(&mut test_rng()), None);
    }

    // --- Helper tests ---

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(17, 5), 1);
        assert_eq!(gcd(0, 5), 5);
    }

    #[test]
    fn test_mod_inv() {
        assert_eq!(mod_inv(3, 7), Some(5));
        assert_eq!(mod_inv(2, 4), None);
    }

    #[test]
    fn test_unique_prime_factors() {
        let mut f = unique_prime_factors(12);
        f.sort_unstable();
        assert_eq!(f, vec![2, 3]);
        assert_eq!(unique_prime_factors(97), vec![97]);
    }
}