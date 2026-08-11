//! Graph theory generators.
//!
//! Provides generators for trees, random graphs, connected graphs,
//! DAGs, and bipartite graphs using petgraph for verification.
//!
//! All output uses 1-indexed vertices.

use crate::types::WeightConfig;
use petgraph::graph::{DiGraph, UnGraph};
#[cfg(test)]
use petgraph::algo;
use rand::{Rng, RngExt};
use rand::seq::SliceRandom;
use std::fmt::Write;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Decode a Prüfer sequence into edges of a tree.
///
/// The sequence has length `n-2` with values in `[1, n]`.
/// Returns `n-1` edges `(u, v)` forming a tree.
fn pruefer_decode(seq: &[u32]) -> Vec<(u32, u32)> {
    let n = seq.len() + 2; // usize
    if n <= 2 {
        // n == 1: no edges; n == 2: single edge (1,2)
        return if n == 2 { vec![(1, 2)] } else { vec![] };
    }

    let nu = n as u32;
    let mut degree = vec![1u32; n];
    for &v in seq {
        degree[v as usize - 1] += 1;
    }

    let mut edges = Vec::with_capacity(n - 1);
    for &v in seq {
        // Find smallest leaf (degree == 1)
        let leaf = (1..=nu)
            .find(|&i| degree[i as usize - 1] == 1)
            .expect("Prüfer: no leaf found — invariant violated");
        edges.push((leaf, v));
        degree[leaf as usize - 1] -= 1;
        degree[v as usize - 1] -= 1;
    }

    // Last two vertices with degree 1
    let remaining: Vec<u32> = (1..=nu).filter(|&i| degree[i as usize - 1] == 1).collect();
    debug_assert_eq!(remaining.len(), 2, "Prüfer: expected exactly 2 remaining leaves");
    edges.push((remaining[0], remaining[1]));
    edges
}

/// Build a petgraph `UnGraph` from edges for verification.
#[allow(dead_code)]
fn build_ungraph(n: u32, edges: &[(u32, u32)]) -> UnGraph<(), ()> {
    let mut g = UnGraph::new_undirected();
    let nodes: Vec<_> = (0..n).map(|_| g.add_node(())).collect();
    for &(u, v) in edges {
        g.add_edge(nodes[u as usize - 1], nodes[v as usize - 1], ());
    }
    g
}

/// Build a petgraph `DiGraph` from edges for verification.
#[allow(dead_code)]
fn build_digraph(n: u32, edges: &[(u32, u32)]) -> DiGraph<(), ()> {
    let mut g = DiGraph::new();
    let nodes: Vec<_> = (0..n).map(|_| g.add_node(())).collect();
    for &(u, v) in edges {
        g.add_edge(nodes[u as usize - 1], nodes[v as usize - 1], ());
    }
    g
}

/// Format edges as a string. First line is a header, then one edge per line.
fn format_output(header: &str, edges: &[(u32, u32)], weights: Option<&[f64]>) -> String {
    let mut out = String::new();
    writeln!(out, "{}", header).expect("write to string");
    for (i, &(u, v)) in edges.iter().enumerate() {
        if let Some(w) = weights {
            writeln!(out, "{} {} {}", u, v, w[i]).expect("write to string");
        } else {
            writeln!(out, "{} {}", u, v).expect("write to string");
        }
    }
    out
}

/// Generate random weights using the given config.
fn generate_weights(rng: &mut impl Rng, count: usize, config: &WeightConfig) -> Vec<f64> {
    let empty_pv = std::collections::HashMap::new();
    (0..count)
        .map(|_| match &config.range {
            crate::types::RangeValue::Static { min, max } => {
                let lo = min.eval(&empty_pv, rng);
                let hi = max.eval(&empty_pv, rng);
                rng.random_range(lo as f64..=hi as f64)
            }
            _ => 1.0,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// TreeGen
// ---------------------------------------------------------------------------

pub struct TreeGen;

impl TreeGen {
    /// Generate a random tree on `n` vertices using a Prüfer sequence.
    ///
    /// Output format:
    /// ```text
    /// n
    /// u1 v1
    /// u2 v2
    /// ...
    /// ```
    pub fn random_tree<R: Rng>(n: u32, rng: &mut R) -> String {
        Self::random_tree_weighted(n, rng, None)
    }

    /// Generate a random tree with optional edge weights.
    pub fn random_tree_weighted<R: Rng>(n: u32, rng: &mut R, weight: Option<&WeightConfig>) -> String {
        if n == 0 {
            return String::new();
        }
        if n == 1 {
            return "1\n".to_string();
        }

        let seq_len = n as usize - 2;
        let seq: Vec<u32> = (0..seq_len).map(|_| rng.random_range(1..=n)).collect();
        let edges = pruefer_decode(&seq);

        let weights = weight.map(|w| generate_weights(rng, edges.len(), w));
        format_output(&n.to_string(), &edges, weights.as_deref())
    }

    /// Generate a star tree: center vertex 1 connected to all others.
    pub fn star(n: u32) -> String {
        Self::star_weighted(n, None)
    }

    /// Generate a star tree with optional edge weights.
    pub fn star_weighted(n: u32, _weight: Option<&WeightConfig>) -> String {
        if n == 0 {
            return String::new();
        }
        if n == 1 {
            return "1\n".to_string();
        }

        let edges: Vec<(u32, u32)> = (2..=n).map(|v| (1, v)).collect();
        format_output(&n.to_string(), &edges, None)
    }

    /// Generate a chain (path) graph: 1-2-3-...-n.
    pub fn chain(n: u32) -> String {
        Self::chain_weighted(n, None)
    }

    /// Generate a chain graph with optional edge weights.
    pub fn chain_weighted(n: u32, _weight: Option<&WeightConfig>) -> String {
        if n == 0 {
            return String::new();
        }
        if n == 1 {
            return "1\n".to_string();
        }

        let edges: Vec<(u32, u32)> = (1..n).map(|v| (v, v + 1)).collect();
        format_output(&n.to_string(), &edges, None)
    }
}

// ---------------------------------------------------------------------------
// RandomGraphGen — G(n, m) Erdős–Rényi
// ---------------------------------------------------------------------------

pub struct RandomGraphGen;

impl RandomGraphGen {
    /// Generate a random graph with `n` vertices and `m` edges (G(n,m) model).
    ///
    /// No self-loops, no duplicate edges.
    pub fn random_graph<R: Rng>(n: u32, m: u32, rng: &mut R) -> String {
        Self::random_graph_weighted(n, m, rng, None)
    }

    /// Generate a random graph with optional edge weights.
    pub fn random_graph_weighted<R: Rng>(n: u32, m: u32, rng: &mut R, weight: Option<&WeightConfig>) -> String {
        if n == 0 {
            return format!("{} {}\n", n, 0);
        }
        assert!(m <= n * (n - 1) / 2, "RandomGraphGen: m exceeds max possible edges");
        if m == 0 {
            return format!("{} {}\n", n, 0);
        }

        // Generate all possible edges
        let mut all_edges: Vec<(u32, u32)> = Vec::with_capacity((n * (n - 1) / 2) as usize);
        for u in 1..=n {
            for v in (u + 1)..=n {
                all_edges.push((u, v));
            }
        }

        // Shuffle and take first m
        all_edges.shuffle(rng);
        all_edges.truncate(m as usize);

        let header = format!("{} {}", n, m);
        let weights = weight.map(|w| generate_weights(rng, all_edges.len(), w));
        format_output(&header, &all_edges, weights.as_deref())
    }
}

// ---------------------------------------------------------------------------
// ConnectedGraphGen — spanning tree + extra edges
// ---------------------------------------------------------------------------

pub struct ConnectedGraphGen;

impl ConnectedGraphGen {
    /// Generate a connected graph with `n` vertices and `m` edges.
    ///
    /// Guarantees connectivity by first generating a random spanning tree,
    /// then adding `m - (n-1)` random extra edges.
    pub fn connected_graph<R: Rng>(n: u32, m: u32, rng: &mut R) -> String {
        Self::connected_graph_weighted(n, m, rng, None)
    }

    /// Generate a connected graph with optional edge weights.
    pub fn connected_graph_weighted<R: Rng>(n: u32, m: u32, rng: &mut R, weight: Option<&WeightConfig>) -> String {
        assert!(n > 0, "ConnectedGraphGen: n must be > 0");
        assert!(m >= n - 1, "ConnectedGraphGen: m must be >= n-1 for connectivity");
        let max_edges = n * (n - 1) / 2;
        assert!(m <= max_edges, "ConnectedGraphGen: m exceeds max possible edges");

        if n == 1 {
            let header = format!("{} {}", n, 0);
            return format!("{}\n", header);
        }

        // Step 1: Generate random spanning tree via Prüfer
        let seq_len = n as usize - 2;
        let seq: Vec<u32> = (0..seq_len).map(|_| rng.random_range(1..=n)).collect();
        let mut edges = pruefer_decode(&seq);

        // Step 2: Add remaining random edges
        let extra = m - (n - 1);
        if extra > 0 {
            // Build set of existing edges for dedup
            let mut edge_set: std::collections::HashSet<(u32, u32)> = edges.iter().copied().collect();
            // Normalize: store (min, max)
            edge_set = edge_set.into_iter().map(|(a, b)| if a < b { (a, b) } else { (b, a) }).collect();

            // Generate all possible edges not in the tree
            let mut candidates: Vec<(u32, u32)> = Vec::new();
            for u in 1..=n {
                for v in (u + 1)..=n {
                    if !edge_set.contains(&(u, v)) {
                        candidates.push((u, v));
                    }
                }
            }

            candidates.shuffle(rng);
            candidates.truncate(extra as usize);
            edges.extend(candidates);
        }

        let header = format!("{} {}", n, m);
        let weights = weight.map(|w| generate_weights(rng, edges.len(), w));
        format_output(&header, &edges, weights.as_deref())
    }
}

// ---------------------------------------------------------------------------
// DAGGen — random directed acyclic graph
// ---------------------------------------------------------------------------

pub struct DAGGen;

impl DAGGen {
    /// Generate a random DAG with `n` vertices and `m` edges.
    ///
    /// Assigns a random topological order, then adds `m` forward edges
    /// (from lower order to higher order).
    pub fn random_dag<R: Rng>(n: u32, m: u32, rng: &mut R) -> String {
        Self::random_dag_weighted(n, m, rng, None)
    }

    /// Generate a random DAG with optional edge weights.
    pub fn random_dag_weighted<R: Rng>(n: u32, m: u32, rng: &mut R, weight: Option<&WeightConfig>) -> String {
        assert!(n > 0, "DAGGen: n must be > 0");
        let max_edges = n * (n - 1) / 2;
        assert!(m <= max_edges, "DAGGen: m exceeds max possible edges");

        if m == 0 {
            return format!("{} {}\n", n, 0);
        }

        // Assign random topological order (permutation of 0..n)
        let mut order: Vec<u32> = (0..n).collect();
        order.shuffle(rng);

        // Generate all possible forward edges
        let mut all_edges: Vec<(u32, u32)> = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                // 1-indexed output: order[i]+1, order[j]+1
                all_edges.push((order[i as usize] + 1, order[j as usize] + 1));
            }
        }

        all_edges.shuffle(rng);
        all_edges.truncate(m as usize);

        let header = format!("{} {}", n, m);
        let weights = weight.map(|w| generate_weights(rng, all_edges.len(), w));
        format_output(&header, &all_edges, weights.as_deref())
    }
}

// ---------------------------------------------------------------------------
// BipartiteGraphGen
// ---------------------------------------------------------------------------

pub struct BipartiteGraphGen;

impl BipartiteGraphGen {
    /// Generate a bipartite graph with `left` vertices on the left side,
    /// `right` vertices on the right side, and `m` crossing edges.
    ///
    /// Left vertices: 1..=left
    /// Right vertices: left+1..=left+right
    pub fn bipartite_graph<R: Rng>(left: u32, right: u32, m: u32, rng: &mut R) -> String {
        Self::bipartite_graph_weighted(left, right, m, rng, None)
    }

    /// Generate a bipartite graph with optional edge weights.
    pub fn bipartite_graph_weighted<R: Rng>(left: u32, right: u32, m: u32, rng: &mut R, weight: Option<&WeightConfig>) -> String {
        let max_edges = left * right;
        assert!(m <= max_edges, "BipartiteGraphGen: m exceeds max possible edges ({}*{}={})", left, right, max_edges);

        if left == 0 || right == 0 || m == 0 {
            return format!("{} {} {}\n", left, right, 0);
        }

        // Generate all possible crossing edges
        let mut all_edges: Vec<(u32, u32)> = Vec::with_capacity((left * right) as usize);
        for l in 1..=left {
            for r in (left + 1)..=(left + right) {
                all_edges.push((l, r));
            }
        }

        all_edges.shuffle(rng);
        all_edges.truncate(m as usize);

        let header = format!("{} {} {}", left, right, m);
        let weights = weight.map(|w| generate_weights(rng, all_edges.len(), w));
        format_output(&header, &all_edges, weights.as_deref())
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

    /// Helper: create a deterministic RNG with fixed seed.
    fn rng_fixed() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    /// Parse output string into (header, edges).
    fn parse_output(output: &str) -> (Vec<&str>, Vec<(u32, u32)>) {
        let mut lines = output.trim().lines();
        let header: Vec<&str> = lines.next().unwrap_or("").split_whitespace().collect();
        let edges: Vec<(u32, u32)> = lines
            .map(|line| {
                let parts: Vec<u32> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
                (parts[0], parts[1])
            })
            .collect();
        (header, edges)
    }

    // -----------------------------------------------------------------------
    // TreeGen tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_treegen_random_tree_has_n_minus_1_edges() {
        let out = TreeGen::random_tree(5, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), 4, "n=5 tree must have 4 edges");
    }

    #[test]
    fn test_treegen_random_tree_connected() {
        let n = 10;
        let out = TreeGen::random_tree(n, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), n as usize - 1);
        let g = build_ungraph(n, &edges);
        assert_eq!(algo::connected_components(&g), 1, "random tree must be connected");
    }

    #[test]
    fn test_treegen_star_center_in_all_edges() {
        let out = TreeGen::star(4);
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), 3);
        for &(u, v) in &edges {
            assert!(u == 1 || v == 1, "star: center 1 must appear in every edge");
        }
    }

    #[test]
    fn test_treegen_star_output() {
        let out = TreeGen::star(4);
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges, vec![(1, 2), (1, 3), (1, 4)]);
    }

    #[test]
    fn test_treegen_chain_output() {
        let out = TreeGen::chain(4);
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges, vec![(1, 2), (2, 3), (3, 4)]);
    }

    #[test]
    fn test_treegen_chain_connected() {
        let n = 10;
        let out = TreeGen::chain(n);
        let (_header, edges) = parse_output(&out);
        let g = build_ungraph(n, &edges);
        assert_eq!(algo::connected_components(&g), 1, "chain must be connected");
    }

    #[test]
    fn test_treegen_n1_empty() {
        let out = TreeGen::random_tree(1, &mut rng_fixed());
        assert_eq!(out.trim(), "1");
    }

    #[test]
    fn test_treegen_n1_star() {
        let out = TreeGen::star(1);
        assert_eq!(out.trim(), "1");
    }

    #[test]
    fn test_treegen_n1_chain() {
        let out = TreeGen::chain(1);
        assert_eq!(out.trim(), "1");
    }

    #[test]
    fn test_treegen_n2_random() {
        let out = TreeGen::random_tree(2, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), 1, "n=2 tree must have 1 edge");
        let g = build_ungraph(2, &edges);
        assert_eq!(algo::connected_components(&g), 1);
    }

    #[test]
    fn test_treegen_deterministic() {
        let out1 = TreeGen::random_tree(10, &mut rng_fixed());
        let out2 = TreeGen::random_tree(10, &mut rng_fixed());
        assert_eq!(out1, out2, "same seed must produce same output");
    }

    // -----------------------------------------------------------------------
    // RandomGraphGen tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_random_graph_has_m_edges() {
        let out = RandomGraphGen::random_graph(5, 7, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), 7, "must have exactly 7 edges");
    }

    #[test]
    fn test_random_graph_no_self_loops() {
        let out = RandomGraphGen::random_graph(5, 7, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        for &(u, v) in &edges {
            assert_ne!(u, v, "no self-loops allowed");
        }
    }

    #[test]
    fn test_random_graph_no_duplicates() {
        let out = RandomGraphGen::random_graph(10, 20, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        let mut seen = std::collections::HashSet::new();
        for &(u, v) in &edges {
            let key = if u < v { (u, v) } else { (v, u) };
            assert!(seen.insert(key), "duplicate edge ({}, {})", u, v);
        }
    }

    #[test]
    fn test_random_graph_header() {
        let out = RandomGraphGen::random_graph(5, 7, &mut rng_fixed());
        let (header, _edges) = parse_output(&out);
        assert_eq!(header, vec!["5", "7"]);
    }

    #[test]
    fn test_random_graph_deterministic() {
        let out1 = RandomGraphGen::random_graph(10, 15, &mut rng_fixed());
        let out2 = RandomGraphGen::random_graph(10, 15, &mut rng_fixed());
        assert_eq!(out1, out2);
    }

    #[test]
    fn test_random_graph_m0() {
        let out = RandomGraphGen::random_graph(5, 0, &mut rng_fixed());
        assert_eq!(out.trim(), "5 0");
    }

    // -----------------------------------------------------------------------
    // ConnectedGraphGen tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_connected_graph_is_connected() {
        let n = 5;
        let m = 7;
        let out = ConnectedGraphGen::connected_graph(n, m, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), m as usize);
        let g = build_ungraph(n, &edges);
        assert_eq!(algo::connected_components(&g), 1, "connected graph must be connected");
    }

    #[test]
    fn test_connected_graph_header() {
        let out = ConnectedGraphGen::connected_graph(5, 7, &mut rng_fixed());
        let (header, _edges) = parse_output(&out);
        assert_eq!(header, vec!["5", "7"]);
    }

    #[test]
    fn test_connected_graph_min_edges() {
        let n = 10;
        let m = n - 1; // tree
        let out = ConnectedGraphGen::connected_graph(n, m, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), m as usize);
        let g = build_ungraph(n, &edges);
        assert_eq!(algo::connected_components(&g), 1);
    }

    #[test]
    fn test_connected_graph_deterministic() {
        let out1 = ConnectedGraphGen::connected_graph(8, 12, &mut rng_fixed());
        let out2 = ConnectedGraphGen::connected_graph(8, 12, &mut rng_fixed());
        assert_eq!(out1, out2);
    }

    #[test]
    fn test_connected_graph_n1() {
        let out = ConnectedGraphGen::connected_graph(1, 0, &mut rng_fixed());
        assert_eq!(out.trim(), "1 0");
    }

    // -----------------------------------------------------------------------
    // DAGGen tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dag_is_acyclic() {
        let n = 10;
        let m = 20;
        let out = DAGGen::random_dag(n, m, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), m as usize);
        let g = build_digraph(n, &edges);
        assert!(!algo::is_cyclic_directed(&g), "DAG must be acyclic");
    }

    #[test]
    fn test_dag_header() {
        let out = DAGGen::random_dag(10, 20, &mut rng_fixed());
        let (header, _edges) = parse_output(&out);
        assert_eq!(header, vec!["10", "20"]);
    }

    #[test]
    fn test_dag_forward_edges() {
        let n = 10;
        let m = 20;
        let out = DAGGen::random_dag(n, m, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        let g = build_digraph(n, &edges);
        assert!(!algo::is_cyclic_directed(&g));
    }

    #[test]
    fn test_dag_deterministic() {
        let out1 = DAGGen::random_dag(10, 20, &mut rng_fixed());
        let out2 = DAGGen::random_dag(10, 20, &mut rng_fixed());
        assert_eq!(out1, out2);
    }

    #[test]
    fn test_dag_m0() {
        let out = DAGGen::random_dag(5, 0, &mut rng_fixed());
        assert_eq!(out.trim(), "5 0");
    }

    #[test]
    fn test_dag_no_self_loops() {
        let out = DAGGen::random_dag(10, 20, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        for &(u, v) in &edges {
            assert_ne!(u, v, "DAG: no self-loops");
        }
    }

    // -----------------------------------------------------------------------
    // BipartiteGraphGen tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bipartite_no_intra_side_edges() {
        let left = 3;
        let right = 4;
        let m = 8;
        let out = BipartiteGraphGen::bipartite_graph(left, right, m, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), m as usize);
        for &(u, v) in &edges {
            assert!(u <= left, "u={} must be in left side (1..{})", u, left);
            assert!(v > left, "v={} must be in right side ({}+1..{})", v, left, left + right);
        }
    }

    #[test]
    fn test_bipartite_header() {
        let out = BipartiteGraphGen::bipartite_graph(3, 4, 8, &mut rng_fixed());
        let (header, _edges) = parse_output(&out);
        assert_eq!(header, vec!["3", "4", "8"]);
    }

    #[test]
    fn test_bipartite_is_bipartite() {
        let left = 3;
        let right = 4;
        let m = 8;
        let out = BipartiteGraphGen::bipartite_graph(left, right, m, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        let n = left + right;
        let g = build_ungraph(n, &edges);
        let start = g.node_indices().next().unwrap();
        assert!(
            algo::is_bipartite_undirected(&g, start),
            "bipartite graph must be 2-colorable"
        );
    }

    #[test]
    fn test_bipartite_deterministic() {
        let out1 = BipartiteGraphGen::bipartite_graph(3, 4, 8, &mut rng_fixed());
        let out2 = BipartiteGraphGen::bipartite_graph(3, 4, 8, &mut rng_fixed());
        assert_eq!(out1, out2);
    }

    #[test]
    fn test_bipartite_m0() {
        let out = BipartiteGraphGen::bipartite_graph(3, 4, 0, &mut rng_fixed());
        assert_eq!(out.trim(), "3 4 0");
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_generators_n0() {
        let t = TreeGen::random_tree(0, &mut rng_fixed());
        assert!(t.is_empty() || t.trim().is_empty());

        let rg = RandomGraphGen::random_graph(0, 0, &mut rng_fixed());
        assert_eq!(rg.trim(), "0 0");
    }

    #[test]
    fn test_max_edges_random_graph() {
        // Complete graph K5 has 10 edges
        let out = RandomGraphGen::random_graph(5, 10, &mut rng_fixed());
        let (_header, edges) = parse_output(&out);
        assert_eq!(edges.len(), 10);
        let mut all: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        for u in 1..=5 {
            for v in (u + 1)..=5 {
                all.insert((u, v));
            }
        }
        let got: std::collections::HashSet<(u32, u32)> = edges.iter().map(|&(a, b)| if a < b { (a, b) } else { (b, a) }).collect();
        assert_eq!(got, all);
    }

    #[test]
    #[should_panic(expected = "m exceeds max")]
    fn test_random_graph_too_many_edges() {
        RandomGraphGen::random_graph(5, 11, &mut rng_fixed());
    }

    #[test]
    #[should_panic(expected = "m must be >= n-1")]
    fn test_connected_graph_too_few_edges() {
        ConnectedGraphGen::connected_graph(5, 3, &mut rng_fixed());
    }

    #[test]
    #[should_panic(expected = "m exceeds max")]
    fn test_dag_too_many_edges() {
        DAGGen::random_dag(5, 11, &mut rng_fixed());
    }

    #[test]
    #[should_panic(expected = "m exceeds max")]
    fn test_bipartite_too_many_edges() {
        BipartiteGraphGen::bipartite_graph(3, 4, 13, &mut rng_fixed());
    }
}