use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DataType { Int32, Int64, BigInt, Float32, Float64, Char, String }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Distribution { Uniform, Normal, Exponential, Poisson, Binomial, Geometric, LogNormal, Cauchy }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Op { Mul, Add, Sub, Min, Max }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ValueExpr {
    #[serde(rename = "const")]
    Const { value: i64 },
    #[serde(rename = "from_field")]
    FromField { name: String },
    #[serde(rename = "random")]
    Random { distribution: Distribution, lo: Box<ValueExpr>, hi: Box<ValueExpr> },
    #[serde(rename = "op")]
    Op { field: String, operator: Op, operand: i64 },
}

impl ValueExpr {
    pub fn eval(&self, pv: &HashMap<String, i64>, rng: &mut impl RngExt) -> i64 {
        match self {
            ValueExpr::Const { value } => *value,
            ValueExpr::FromField { name } => pv.get(name).copied().unwrap_or(0),
            ValueExpr::Random { lo, hi, .. } => {
                let lo = lo.eval(pv, rng);
                let hi = hi.eval(pv, rng);
                let lo = lo.min(hi);
                let hi = hi.max(lo);
                rng.random_range(lo..=hi)
            }
            ValueExpr::Op { field, operator, operand } => {
                let v = pv.get(field).copied().unwrap_or(0);
                match operator {
                    Op::Mul => v * operand,
                    Op::Add => v + operand,
                    Op::Sub => v - operand,
                    Op::Min => v.min(*operand),
                    Op::Max => v.max(*operand),
                }
            }
        }
    }

    pub fn dependencies(&self) -> Vec<String> {
        match self {
            ValueExpr::Const { .. } => vec![],
            ValueExpr::FromField { name } => vec![name.clone()],
            ValueExpr::Random { lo, hi, .. } => {
                let mut deps = lo.dependencies();
                deps.extend(hi.dependencies());
                deps
            }
            ValueExpr::Op { field, .. } => vec![field.clone()],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range<T: PartialOrd> { pub min: T, pub max: T }

impl<T: PartialOrd + std::fmt::Debug> Range<T> {
    pub fn new(min: T, max: T) -> Self {
        assert!(min <= max, "Range min ({:?}) must be <= max ({:?})", min, max);
        Self { min, max }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RangeValue {
    #[serde(rename = "static")]
    Static { min: ValueExpr, max: ValueExpr },
    #[serde(rename = "count_from")]
    CountFrom { from_field: String, elem_value: Box<ValueExpr> },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum FieldSeparator {
    #[default]
    Space,
    Newline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub data_type: DataType,
    pub distribution: Distribution,
    pub range: RangeValue,
    #[serde(default)]
    pub separator: FieldSeparator,
}

impl FieldDef {
    /// Collect all field names this field depends on from ValueExpr tree.
    pub fn dependencies(&self) -> Vec<String> {
        match &self.range {
            RangeValue::Static { min, max } => {
                let mut deps = min.dependencies();
                deps.extend(max.dependencies());
                deps
            }
            RangeValue::CountFrom { from_field, elem_value } => {
                let mut deps = vec![from_field.clone()];
                deps.extend(elem_value.dependencies());
                deps
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum TestCaseMode {
    #[default]
    Disabled,
    Fixed(u32),
    Random { distribution: Distribution, range: Range<u32> },
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    // --- Range tests ---

    #[test]
    fn test_range_valid() {
        let r = Range::new(1, 10);
        assert_eq!(r.min, 1);
        assert_eq!(r.max, 10);
    }

    #[test]
    #[should_panic]
    fn test_range_invalid() {
        Range::new(10, 1);
    }

    // --- Serde tests ---

    #[test]
    fn test_datatype_serde() {
        let dt = DataType::Int32;
        let json = serde_json::to_string(&dt).unwrap();
        let back: DataType = serde_json::from_str(&json).unwrap();
        assert_eq!(dt, back);
    }

    #[test]
    fn test_distribution_serde() {
        let d = Distribution::Normal;
        let json = serde_json::to_string(&d).unwrap();
        let back: Distribution = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn test_op_serde() {
        for op in [Op::Mul, Op::Add, Op::Sub, Op::Min, Op::Max] {
            let json = serde_json::to_string(&op).unwrap();
            let back: Op = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn test_value_expr_const_serde() {
        let expr = ValueExpr::Const { value: 42 };
        let json = serde_json::to_string(&expr).unwrap();
        let back: ValueExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, back);
    }

    #[test]
    fn test_value_expr_from_field_serde() {
        let expr = ValueExpr::FromField { name: "n".into() };
        let json = serde_json::to_string(&expr).unwrap();
        let back: ValueExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, back);
    }

    #[test]
    fn test_value_expr_random_serde() {
        let expr = ValueExpr::Random {
            distribution: Distribution::Uniform,
            lo: Box::new(ValueExpr::Const { value: 0 }),
            hi: Box::new(ValueExpr::Const { value: 100 }),
        };
        let json = serde_json::to_string(&expr).unwrap();
        let back: ValueExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, back);
    }

    #[test]
    fn test_value_expr_op_serde() {
        let expr = ValueExpr::Op { field: "n".into(), operator: Op::Mul, operand: 2 };
        let json = serde_json::to_string(&expr).unwrap();
        let back: ValueExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, back);
    }

    #[test]
    fn test_range_value_static_serde() {
        let rv = RangeValue::Static {
            min: ValueExpr::Const { value: 1 },
            max: ValueExpr::Const { value: 100 },
        };
        let json = serde_json::to_string(&rv).unwrap();
        let back: RangeValue = serde_json::from_str(&json).unwrap();
        assert_eq!(rv, back);
    }

    #[test]
    fn test_range_value_count_from_serde() {
        let rv = RangeValue::CountFrom {
            from_field: "n".into(),
            elem_value: Box::new(ValueExpr::Random {
                distribution: Distribution::Uniform,
                lo: Box::new(ValueExpr::Const { value: 1 }),
                hi: Box::new(ValueExpr::Const { value: 100 }),
            }),
        };
        let json = serde_json::to_string(&rv).unwrap();
        let back: RangeValue = serde_json::from_str(&json).unwrap();
        assert_eq!(rv, back);
    }

    // --- ValueExpr::eval tests ---

    #[test]
    fn test_value_expr_eval_const() {
        let pv = HashMap::new();
        let mut rng = test_rng();
        let expr = ValueExpr::Const { value: 42 };
        assert_eq!(expr.eval(&pv, &mut rng), 42);
    }

    #[test]
    fn test_value_expr_eval_from_field() {
        let mut pv = HashMap::new();
        pv.insert("n".into(), 10);
        let mut rng = test_rng();
        let expr = ValueExpr::FromField { name: "n".into() };
        assert_eq!(expr.eval(&pv, &mut rng), 10);
    }

    #[test]
    fn test_value_expr_eval_from_field_missing() {
        let pv = HashMap::new();
        let mut rng = test_rng();
        let expr = ValueExpr::FromField { name: "missing".into() };
        assert_eq!(expr.eval(&pv, &mut rng), 0);
    }

    #[test]
    fn test_value_expr_eval_random() {
        let pv = HashMap::new();
        let mut rng = test_rng();
        let expr = ValueExpr::Random {
            distribution: Distribution::Uniform,
            lo: Box::new(ValueExpr::Const { value: 10 }),
            hi: Box::new(ValueExpr::Const { value: 20 }),
        };
        let v = expr.eval(&pv, &mut rng);
        assert!(v >= 10 && v <= 20, "Random eval {} should be in [10,20]", v);
    }

    #[test]
    fn test_value_expr_eval_random_swapped() {
        // lo > hi should be swapped internally
        let pv = HashMap::new();
        let mut rng = test_rng();
        let expr = ValueExpr::Random {
            distribution: Distribution::Uniform,
            lo: Box::new(ValueExpr::Const { value: 100 }),
            hi: Box::new(ValueExpr::Const { value: 0 }),
        };
        let v = expr.eval(&pv, &mut rng);
        assert!(v >= 0 && v <= 100, "Random eval {} should be in [0,100] after swap", v);
    }

    #[test]
    fn test_value_expr_eval_op_mul() {
        let mut pv = HashMap::new();
        pv.insert("n".into(), 5);
        let mut rng = test_rng();
        let expr = ValueExpr::Op { field: "n".into(), operator: Op::Mul, operand: 3 };
        assert_eq!(expr.eval(&pv, &mut rng), 15);
    }

    #[test]
    fn test_value_expr_eval_op_add() {
        let mut pv = HashMap::new();
        pv.insert("n".into(), 5);
        let mut rng = test_rng();
        let expr = ValueExpr::Op { field: "n".into(), operator: Op::Add, operand: 3 };
        assert_eq!(expr.eval(&pv, &mut rng), 8);
    }

    #[test]
    fn test_value_expr_eval_op_sub() {
        let mut pv = HashMap::new();
        pv.insert("n".into(), 5);
        let mut rng = test_rng();
        let expr = ValueExpr::Op { field: "n".into(), operator: Op::Sub, operand: 3 };
        assert_eq!(expr.eval(&pv, &mut rng), 2);
    }

    #[test]
    fn test_value_expr_eval_op_min() {
        let mut pv = HashMap::new();
        pv.insert("n".into(), 5);
        let mut rng = test_rng();
        let expr = ValueExpr::Op { field: "n".into(), operator: Op::Min, operand: 10 };
        assert_eq!(expr.eval(&pv, &mut rng), 5);
        let expr2 = ValueExpr::Op { field: "n".into(), operator: Op::Min, operand: 3 };
        assert_eq!(expr2.eval(&pv, &mut rng), 3);
    }

    #[test]
    fn test_value_expr_eval_op_max() {
        let mut pv = HashMap::new();
        pv.insert("n".into(), 5);
        let mut rng = test_rng();
        let expr = ValueExpr::Op { field: "n".into(), operator: Op::Max, operand: 10 };
        assert_eq!(expr.eval(&pv, &mut rng), 10);
        let expr2 = ValueExpr::Op { field: "n".into(), operator: Op::Max, operand: 3 };
        assert_eq!(expr2.eval(&pv, &mut rng), 5);
    }

    #[test]
    fn test_value_expr_eval_op_missing_field() {
        let pv = HashMap::new();
        let mut rng = test_rng();
        let expr = ValueExpr::Op { field: "missing".into(), operator: Op::Mul, operand: 2 };
        assert_eq!(expr.eval(&pv, &mut rng), 0);
    }

    #[test]
    fn test_value_expr_eval_random_with_from_field() {
        let mut pv = HashMap::new();
        pv.insert("lo".into(), 5);
        pv.insert("hi".into(), 15);
        let mut rng = test_rng();
        let expr = ValueExpr::Random {
            distribution: Distribution::Uniform,
            lo: Box::new(ValueExpr::FromField { name: "lo".into() }),
            hi: Box::new(ValueExpr::FromField { name: "hi".into() }),
        };
        let v = expr.eval(&pv, &mut rng);
        assert!(v >= 5 && v <= 15, "Random eval {} should be in [5,15]", v);
    }

    #[test]
    fn test_value_expr_eval_nested_op() {
        let mut pv = HashMap::new();
        pv.insert("n".into(), 2);
        let mut rng = test_rng();
        // Op(Op(n, Mul, 3), Add, 1) = 2*3+1 = 7
        let _inner = ValueExpr::Op { field: "n".into(), operator: Op::Mul, operand: 3 };
        let outer = ValueExpr::Op { field: "n".into(), operator: Op::Add, operand: 1 };
        // Actually, Op doesn't nest - it reads from pv. So we need to insert intermediate.
        pv.insert("n".into(), 6); // simulate inner result
        assert_eq!(outer.eval(&pv, &mut rng), 7);
    }

    // --- ValueExpr::dependencies tests ---

    #[test]
    fn test_value_expr_deps_const() {
        let expr = ValueExpr::Const { value: 42 };
        assert!(expr.dependencies().is_empty());
    }

    #[test]
    fn test_value_expr_deps_from_field() {
        let expr = ValueExpr::FromField { name: "n".into() };
        assert_eq!(expr.dependencies(), vec!["n"]);
    }

    #[test]
    fn test_value_expr_deps_random() {
        let expr = ValueExpr::Random {
            distribution: Distribution::Uniform,
            lo: Box::new(ValueExpr::FromField { name: "a".into() }),
            hi: Box::new(ValueExpr::FromField { name: "b".into() }),
        };
        let deps = expr.dependencies();
        assert!(deps.contains(&"a".to_string()));
        assert!(deps.contains(&"b".to_string()));
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_value_expr_deps_op() {
        let expr = ValueExpr::Op { field: "n".into(), operator: Op::Mul, operand: 2 };
        assert_eq!(expr.dependencies(), vec!["n"]);
    }

    #[test]
    fn test_value_expr_deps_nested_random() {
        let expr = ValueExpr::Random {
            distribution: Distribution::Uniform,
            lo: Box::new(ValueExpr::Random {
                distribution: Distribution::Uniform,
                lo: Box::new(ValueExpr::FromField { name: "x".into() }),
                hi: Box::new(ValueExpr::Const { value: 10 }),
            }),
            hi: Box::new(ValueExpr::FromField { name: "y".into() }),
        };
        let deps = expr.dependencies();
        assert!(deps.contains(&"x".to_string()));
        assert!(deps.contains(&"y".to_string()));
    }

    // --- FieldDef::dependencies tests ---

    #[test]
    fn test_field_def_deps_static_const() {
        let f = FieldDef {
            name: "m".into(),
            data_type: DataType::Int32,
            distribution: Distribution::Uniform,
            range: RangeValue::Static {
                min: ValueExpr::Const { value: 1 },
                max: ValueExpr::Const { value: 100 },
            },
            separator: FieldSeparator::default(),
        };
        assert!(f.dependencies().is_empty());
    }

    #[test]
    fn test_field_def_deps_static_from_field() {
        let f = FieldDef {
            name: "m".into(),
            data_type: DataType::Int32,
            distribution: Distribution::Uniform,
            range: RangeValue::Static {
                min: ValueExpr::FromField { name: "n".into() },
                max: ValueExpr::Const { value: 100 },
            },
            separator: FieldSeparator::default(),
        };
        assert_eq!(f.dependencies(), vec!["n"]);
    }

    #[test]
    fn test_field_def_deps_count_from() {
        let f = FieldDef {
            name: "arr".into(),
            data_type: DataType::Int32,
            distribution: Distribution::Uniform,
            range: RangeValue::CountFrom {
                from_field: "n".into(),
                elem_value: Box::new(ValueExpr::Const { value: 42 }),
            },
            separator: FieldSeparator::default(),
        };
        assert_eq!(f.dependencies(), vec!["n"]);
    }

    #[test]
    fn test_field_def_deps_count_from_with_op() {
        let f = FieldDef {
            name: "arr".into(),
            data_type: DataType::Int32,
            distribution: Distribution::Uniform,
            range: RangeValue::CountFrom {
                from_field: "n".into(),
                elem_value: Box::new(ValueExpr::Op { field: "base".into(), operator: Op::Add, operand: 1 }),
            },
            separator: FieldSeparator::default(),
        };
        let deps = f.dependencies();
        assert!(deps.contains(&"n".to_string()));
        assert!(deps.contains(&"base".to_string()));
    }

    // --- TestCaseMode serde tests ---

    #[test]
    fn test_testcase_mode_disabled_serde() {
        let mode = TestCaseMode::Disabled;
        let json = serde_json::to_string(&mode).unwrap();
        let back: TestCaseMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }

    #[test]
    fn test_testcase_mode_fixed_serde() {
        let mode = TestCaseMode::Fixed(5);
        let json = serde_json::to_string(&mode).unwrap();
        let back: TestCaseMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }

    // --- TOML roundtrip test ---

    #[test]
    fn test_toml_roundtrip() {
        let config = TestConfig {
            files_count: 10,
            prefix: "test".into(),
            suffix: String::new(),
            testcase_mode: TestCaseMode::Fixed(3),
            fields: vec![FieldDef {
                name: "n".into(),
                data_type: DataType::Int32,
                distribution: Distribution::Uniform,
                range: RangeValue::Static {
                    min: ValueExpr::Const { value: 1 },
                    max: ValueExpr::Const { value: 1000 },
                },
                separator: FieldSeparator::default(),
            }],
            seed: Some(42),
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed = TestConfig::from_toml(&toml_str).unwrap();
        assert_eq!(config, parsed);
    }

    // --- Validation tests ---

    #[test]
    fn test_validate_files_count_too_large() {
        let config = TestConfig {
            files_count: 1001,
            prefix: "test".into(),
            suffix: String::new(),
            testcase_mode: TestCaseMode::Disabled,
            fields: vec![FieldDef {
                name: "n".into(),
                data_type: DataType::Int32,
                distribution: Distribution::Uniform,
                range: RangeValue::Static {
                    min: ValueExpr::Const { value: 1 },
                    max: ValueExpr::Const { value: 100 },
                },
                separator: FieldSeparator::default(),
            }],
            seed: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_no_fields() {
        let config = TestConfig {
            files_count: 10,
            prefix: "test".into(),
            suffix: String::new(),
            testcase_mode: TestCaseMode::Disabled,
            fields: vec![],
            seed: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_valid_config() {
        let config = TestConfig {
            files_count: 10,
            prefix: "test".into(),
            suffix: String::new(),
            testcase_mode: TestCaseMode::Fixed(5),
            fields: vec![FieldDef {
                name: "n".into(),
                data_type: DataType::Int32,
                distribution: Distribution::Uniform,
                range: RangeValue::Static {
                    min: ValueExpr::Const { value: 1 },
                    max: ValueExpr::Const { value: 100 },
                },
                separator: FieldSeparator::default(),
            }],
            seed: Some(42),
        };
        assert!(config.validate().is_ok());
    }
}