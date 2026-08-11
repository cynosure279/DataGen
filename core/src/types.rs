use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType { Int32, Int64, BigInt, Float32, Float64, Char, String }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Distribution { Uniform, Normal, Exponential, Poisson }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range<T: PartialOrd> { pub min: T, pub max: T }

impl<T: PartialOrd + std::fmt::Debug> Range<T> {
    pub fn new(min: T, max: T) -> Self {
        assert!(min <= max, "Range min ({:?}) must be <= max ({:?})", min, max);
        Self { min, max }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RangeValue {
    Int32(Range<i32>),
    Int64(Range<i64>),
    Float32(Range<f32>),
    Float64(Range<f64>),
    Char(Range<char>),
    StringLen(Range<usize>),
    /// Array/string: count = generated value of from_field.
    /// Each element value in [elem_min, elem_max].
    /// E.g. "N followed by N numbers": field "n" Int32 1..100,
    /// then field "arr" with CountFrom { from_field: "n", elem_min: 1, elem_max: 1000000 }
    CountFrom { from_field: String, elem_min: i64, elem_max: i64 },
    /// Single value: max = parent_value * multiplier (as i64), min = 1.
    /// E.g. "M where M ≤ 2*N": ValueFrom { from_field: "n", multiplier: 2.0 }
    ValueFrom { from_field: String, multiplier: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FieldSeparator {
    Space,
    Newline,
}

impl Default for FieldSeparator {
    fn default() -> Self { Self::Space }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub data_type: DataType,
    pub distribution: Distribution,
    pub range: RangeValue,
    #[serde(default)]
    pub depends_on: Option<String>,
    #[serde(default)]
    pub separator: FieldSeparator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TestCaseMode {
    Disabled,
    Fixed(u32),
    Random { distribution: Distribution, range: Range<u32> },
}

impl Default for TestCaseMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestConfig {
    pub files_count: u32,
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub testcase_mode: TestCaseMode,
    pub fields: Vec<FieldDef>,
    pub seed: Option<u64>,
}

fn default_prefix() -> String { "test".to_string() }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenFile { pub filename: String, pub content: String }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenMetadata {
    pub seed: u64,
    pub generated_at: String,
    pub config_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenResult { pub files: Vec<GenFile>, pub metadata: GenMetadata }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GraphType { Tree, Random, Connected, DAG, Bipartite }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightConfig { pub distribution: Distribution, pub range: RangeValue }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphConfig {
    pub graph_type: GraphType,
    pub nodes: u32,
    pub edges: Option<u32>,
    pub weighted: Option<WeightConfig>,
    pub left_nodes: Option<u32>,
    pub right_nodes: Option<u32>,
}