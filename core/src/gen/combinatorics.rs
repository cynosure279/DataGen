//! Combinatorics generators: permutations, combinations, derangements, partitions.

use rand::{Rng, RngExt};

// ---------------------------------------------------------------------------
// PermutationGen
// ---------------------------------------------------------------------------

/// Permutation generator using Fisher-Yates shuffle and SJT algorithm.
pub struct PermutationGen;

impl PermutationGen {
    /// Generate a random permutation of `[1..n]` using Fisher-Yates shuffle.
    ///
    /// Returns an empty vector when `n == 0`.
    pub fn fisher_yates(n: u32, rng: &mut impl Rng) -> Vec<u32> {
        if n == 0 {
            return vec![];
        }
        let mut v: Vec<u32> = (1..=n).collect();
        let len = n as usize;
        for i in (1..len).rev() {
            let j = rng.random_range(0..=i);
            v.swap(i, j);
        }
        v
    }

    /// Enumerate all permutations of `[1..n]` using the SJT algorithm.
    ///
    /// Yields `n!` permutations. Returns a single empty vector when `n == 0`.
    pub fn enumerate(n: u32) -> Box<dyn Iterator<Item = Vec<u32>>> {
        let n_usize = n as usize;
        if n_usize == 0 {
            return Box::new(std::iter::once(vec![]));
        }
        if n_usize == 1 {
            return Box::new(std::iter::once(vec![1u32]));
        }
        let total = ecgen::factorial(n_usize);
        let mut perm: Vec<u32> = (1..=n).collect();
        let mut count = 0usize;
        let mut swap_iter = ecgen::sjt_gen(n_usize).into_iter();

        Box::new(std::iter::from_fn(move || {
            if count == 0 {
                count = 1;
                Some(perm.clone())
            } else if count < total {
                if let Some(i) = swap_iter.next() {
                    perm.swap(i, i + 1);
                    count += 1;
                    Some(perm.clone())
                } else {
                    None
                }
            } else {
                None
            }
        }))
    }
}

// ---------------------------------------------------------------------------
// CombinationGen
// ---------------------------------------------------------------------------

/// Combination generator using revolving-door algorithm and manual binomial.
pub struct CombinationGen;

impl CombinationGen {
    /// Enumerate all `k`-combinations of `[1..n]`.
    ///
    /// Yields `C(n, k)` combinations. Returns a single empty vector when `k == 0`,
    /// and nothing when `k > n`.
    pub fn enumerate(n: u32, k: u32) -> Box<dyn Iterator<Item = Vec<u32>>> {
        let n_usize = n as usize;
        let k_usize = k as usize;

        if k == 0 {
            return Box::new(std::iter::once(vec![]));
        }
        if k > n {
            return Box::new(std::iter::empty());
        }

        let total = ecgen::comb(n_usize, k_usize);
        let mut bits: Vec<bool> = (0..n_usize).map(|i| i < k_usize).collect();
        let mut count = 0usize;
        let mut swap_iter = ecgen::emk_comb_gen(n_usize, k_usize).into_iter();

        Box::new(std::iter::from_fn(move || {
            if count == 0 {
                count = 1;
                Some(extract_combination(&bits))
            } else if count < total {
                if let Some((i, j)) = swap_iter.next() {
                    bits.swap(i, j);
                    count += 1;
                    Some(extract_combination(&bits))
                } else {
                    None
                }
            } else {
                None
            }
        }))
    }

    /// Compute the binomial coefficient `C(n, k)`.
    ///
    /// Returns `None` if the result exceeds `u64::MAX`.
    pub fn count(n: u32, k: u32) -> Option<u64> {
        if k > n {
            return Some(0);
        }
        let k = k.min(n - k);
        if k == 0 {
            return Some(1);
        }
        let n = n as u128;
        let k = k as u128;
        let mut result = 1u128;
        for i in 1..=k {
            result = result * (n - k + i) / i;
            if result > u64::MAX as u128 {
                return None;
            }
        }
        Some(result as u64)
    }
}

/// Extract 1-indexed positions of `true` bits.
fn extract_combination(bits: &[bool]) -> Vec<u32> {
    bits.iter()
        .enumerate()
        .filter(|(_, &b)| b)
        .map(|(i, _)| (i + 1) as u32)
        .collect()
}

// ---------------------------------------------------------------------------
// DerangementGen
// ---------------------------------------------------------------------------

/// Derangement generator using rejection sampling and the derangements crate.
pub struct DerangementGen;

impl DerangementGen {
    /// Generate a random derangement of `[1..n]` using rejection sampling.
    ///
    /// Repeatedly generates random permutations until a derangement is found.
    /// Returns an empty vector when `n == 0`.
    pub fn random_derangement(n: u32, rng: &mut impl Rng) -> Vec<u32> {
        if n == 0 {
            return vec![];
        }
        loop {
            let perm = PermutationGen::fisher_yates(n, rng);
            if is_derangement(&perm) {
                return perm;
            }
        }
    }

    /// Enumerate all derangements of `[1..n]`.
    ///
    /// Yields `!n` derangements. Returns a single empty vector when `n == 0`.
    pub fn enumerate(n: u32) -> Box<dyn Iterator<Item = Vec<u32>>> {
        let n_usize = n as usize;
        Box::new(
            derangements::derangements(0..n_usize, n_usize)
                .map(|v| v.into_iter().map(|x| (x + 1) as u32).collect()),
        )
    }

    /// Compute the number of derangements `!n` (subfactorial).
    pub fn count(n: u32) -> u64 {
        subfactorial(n)
    }
}

/// Check if a permutation is a derangement (no element at its original position).
fn is_derangement(perm: &[u32]) -> bool {
    perm.iter()
        .enumerate()
        .all(|(i, &val)| val != (i as u32 + 1))
}

/// Compute subfactorial `!n` using the recurrence `!n = (n-1) * (!(n-1) + !(n-2))`.
fn subfactorial(n: u32) -> u64 {
    match n {
        0 => 1,
        1 => 0,
        _ => {
            let mut a = 1u64; // !0
            let mut b = 0u64; // !1
            for i in 2..=n as u64 {
                let c = (i - 1) * (a + b);
                a = b;
                b = c;
            }
            b
        }
    }
}

// ---------------------------------------------------------------------------
// PartitionGen
// ---------------------------------------------------------------------------

/// Integer partition generator using Euler's pentagonal theorem and recursion.
pub struct PartitionGen;

impl PartitionGen {
    /// Compute the partition function `p(n)` using Euler's pentagonal number theorem.
    ///
    /// `p(0) = 1`, `p(1) = 1`, `p(2) = 2`, `p(3) = 3`, `p(4) = 5`, `p(5) = 7`, ...
    pub fn count_partitions(n: u32) -> u64 {
        if n == 0 {
            return 1;
        }
        let n = n as usize;
        let mut p = vec![0u64; n + 1];
        p[0] = 1;
        for i in 1..=n {
            let mut sum = 0i128;
            let mut k = 1u32;
            loop {
                let g1 = k * (3 * k - 1) / 2;
                let g2 = k * (3 * k + 1) / 2;

                if g1 > i as u32 && g2 > i as u32 {
                    break;
                }

                let sign = if k % 2 == 1 { 1i128 } else { -1i128 };

                if (g1 as usize) <= i {
                    sum += sign * p[i - g1 as usize] as i128;
                }
                if (g2 as usize) <= i {
                    sum += sign * p[i - g2 as usize] as i128;
                }

                k += 1;
            }
            p[i] = sum as u64;
        }
        p[n]
    }

    /// Enumerate all integer partitions of `n`.
    ///
    /// Returns partitions in lexicographic order (largest parts first).
    /// Returns a vector containing one empty vector when `n == 0`.
    pub fn enumerate_partitions(n: u32) -> Vec<Vec<u32>> {
        let mut result = Vec::new();
        let mut current = Vec::new();
        partition_helper(n, n, &mut current, &mut result);
        result
    }
}

/// Recursive helper for partition enumeration.
fn partition_helper(
    remaining: u32,
    max_part: u32,
    current: &mut Vec<u32>,
    result: &mut Vec<Vec<u32>>,
) {
    if remaining == 0 {
        result.push(current.clone());
        return;
    }
    let start = max_part.min(remaining);
    for part in (1..=start).rev() {
        current.push(part);
        partition_helper(remaining - part, part, current, result);
        current.pop();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    // --- PermutationGen ---

    #[test]
    fn test_fisher_yates_deterministic() {
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);
        let p1 = PermutationGen::fisher_yates(5, &mut rng1);
        let p2 = PermutationGen::fisher_yates(5, &mut rng2);
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_fisher_yates_valid() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let p = PermutationGen::fisher_yates(5, &mut rng);
        assert_eq!(p.len(), 5);
        let mut sorted = p.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_fisher_yates_zero() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let p = PermutationGen::fisher_yates(0, &mut rng);
        assert!(p.is_empty());
    }

    #[test]
    fn test_enumerate_permutations_count() {
        let perms: Vec<Vec<u32>> = PermutationGen::enumerate(4).collect();
        assert_eq!(perms.len(), 24); // 4! = 24
    }

    #[test]
    fn test_enumerate_permutations_zero() {
        let perms: Vec<Vec<u32>> = PermutationGen::enumerate(0).collect();
        assert_eq!(perms.len(), 1); // 0! = 1
        assert_eq!(perms[0], Vec::<u32>::new());
    }

    #[test]
    fn test_enumerate_permutations_one() {
        let perms: Vec<Vec<u32>> = PermutationGen::enumerate(1).collect();
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0], vec![1]);
    }

    // --- CombinationGen ---

    #[test]
    fn test_count_combinations_5_2() {
        assert_eq!(CombinationGen::count(5, 2), Some(10));
    }

    #[test]
    fn test_count_combinations_100_50_overflow() {
        // C(100, 50) ≈ 1.0089e29 exceeds u64::MAX ≈ 1.84e19
        assert_eq!(CombinationGen::count(100, 50), None);
    }

    #[test]
    fn test_count_combinations_200_100_overflow() {
        assert_eq!(CombinationGen::count(200, 100), None);
    }

    #[test]
    fn test_count_combinations_edge() {
        assert_eq!(CombinationGen::count(0, 0), Some(1));
        assert_eq!(CombinationGen::count(5, 0), Some(1));
        assert_eq!(CombinationGen::count(5, 6), Some(0));
        assert_eq!(CombinationGen::count(5, 5), Some(1));
        assert_eq!(CombinationGen::count(5, 1), Some(5));
    }

    #[test]
    fn test_enumerate_combinations_5_2() {
        let combs: Vec<Vec<u32>> = CombinationGen::enumerate(5, 2).collect();
        assert_eq!(combs.len(), 10);
        assert_eq!(combs[0], vec![1, 2]);
    }

    #[test]
    fn test_enumerate_combinations_k0() {
        let combs: Vec<Vec<u32>> = CombinationGen::enumerate(5, 0).collect();
        assert_eq!(combs.len(), 1);
        assert_eq!(combs[0], Vec::<u32>::new());
    }

    #[test]
    fn test_enumerate_combinations_kgt_n() {
        let combs: Vec<Vec<u32>> = CombinationGen::enumerate(5, 6).collect();
        assert_eq!(combs.len(), 0);
    }

    // --- DerangementGen ---

    #[test]
    fn test_random_derangement_4() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let d = DerangementGen::random_derangement(4, &mut rng);
        assert_eq!(d.len(), 4);
        for (i, &val) in d.iter().enumerate() {
            assert_ne!(val, i as u32 + 1, "Element {} at position {}", val, i);
        }
    }

    #[test]
    fn test_derangement_count_4() {
        assert_eq!(DerangementGen::count(4), 9);
    }

    #[test]
    fn test_derangement_count_1() {
        assert_eq!(DerangementGen::count(1), 0);
    }

    #[test]
    fn test_derangement_count_0() {
        assert_eq!(DerangementGen::count(0), 1);
    }

    #[test]
    fn test_enumerate_derangements_4() {
        let der: Vec<Vec<u32>> = DerangementGen::enumerate(4).collect();
        assert_eq!(der.len(), 9);
        for d in &der {
            assert_eq!(d.len(), 4);
            for (i, &val) in d.iter().enumerate() {
                assert_ne!(val, i as u32 + 1);
            }
        }
    }

    #[test]
    fn test_enumerate_derangements_0() {
        let der: Vec<Vec<u32>> = DerangementGen::enumerate(0).collect();
        assert_eq!(der.len(), 1);
        assert_eq!(der[0], Vec::<u32>::new());
    }

    // --- PartitionGen ---

    #[test]
    fn test_partition_count_5() {
        assert_eq!(PartitionGen::count_partitions(5), 7);
    }

    #[test]
    fn test_partition_count_10() {
        assert_eq!(PartitionGen::count_partitions(10), 42);
    }

    #[test]
    fn test_partition_count_0() {
        assert_eq!(PartitionGen::count_partitions(0), 1);
    }

    #[test]
    fn test_partition_count_1() {
        assert_eq!(PartitionGen::count_partitions(1), 1);
    }

    #[test]
    fn test_enumerate_partitions_3() {
        let partitions = PartitionGen::enumerate_partitions(3);
        assert_eq!(partitions, vec![vec![3], vec![2, 1], vec![1, 1, 1]]);
    }

    #[test]
    fn test_enumerate_partitions_0() {
        let partitions = PartitionGen::enumerate_partitions(0);
        assert_eq!(partitions, vec![Vec::<u32>::new()]);
    }

    #[test]
    fn test_enumerate_partitions_1() {
        let partitions = PartitionGen::enumerate_partitions(1);
        assert_eq!(partitions, vec![vec![1]]);
    }
}