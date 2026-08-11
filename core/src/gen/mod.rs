// Data generation engine

pub mod combinatorics;
pub mod distribution;
pub mod graph;
pub mod numtheory;

pub use distribution::{
    BinomialGen, CauchyGen, ExponentialGen, GeometricGen, LogNormalGen, NormalGen, PoissonGen,
    UniformFloatGen, UniformIntGen,
};

/// Trait for all data generators.
///
/// Implementors produce values of `Output` using a random number source.
pub trait Generator {
    type Output;
    fn generate(&mut self, rng: &mut impl rand::Rng) -> Self::Output;
}