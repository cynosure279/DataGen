use crate::config::ConfigError;
use crate::gen::{
    BinomialGen, CauchyGen, GeometricGen, Generator, LogNormalGen,
};
use crate::types::*;
use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate(config: &TestConfig) -> Result<GenResult, ConfigError> {
    config.validate()?;
    let seed = config.seed.unwrap_or_else(|| rand::rng().random());
    let mut rng = StdRng::seed_from_u64(seed);
    let sorted = topo_sort_fields(&config.fields).map_err(|e| {
        ConfigError::ValidationError(format!("dependency error: {}", e))
    })?;
    let mut files = Vec::with_capacity(config.files_count as usize);
    for i in 0..config.files_count {
        let t = tc_count(&config.testcase_mode, &mut rng);
        let mut content = String::new();
        if t > 1 { content.push_str(&format!("{t}\n")); }
        for _ in 0..t {
            let mut pv: HashMap<String, i64> = HashMap::new();
            let mut first_field = true;
            for f in &sorted {
                let field_parts = gen_field(f, &mut pv, &mut rng);
                if field_parts.is_empty() { continue; }
                if first_field { first_field = false; }
                else { content.push(' '); }
                for (j, part) in field_parts.iter().enumerate() {
                    if j > 0 { content.push(' '); }
                    content.push_str(part);
                }
                if f.separator == FieldSeparator::Newline { content.push('\n'); }
            }
            if !content.ends_with('\n') { content.push('\n'); }
        }
        let sfx = if config.suffix.is_empty() { String::new() } else { format!("_{}", config.suffix) };
        let name = if config.files_count == 1 { format!("{}{}.in", config.prefix, sfx) } else { format!("{}{}{}.in", config.prefix, i+1, sfx) };
        files.push(GenFile { filename: name, content });
    }
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().to_string();
    let hash = toml::to_string(config).unwrap_or_default();
    Ok(GenResult { files, metadata: GenMetadata { seed, generated_at: ts, config_hash: hash } })
}

fn tc_count(mode: &TestCaseMode, rng: &mut impl Rng) -> u32 {
    match mode { TestCaseMode::Disabled => 1, TestCaseMode::Fixed(n) => *n, TestCaseMode::Random { range, .. } => rng.random_range(range.min..=range.max) }
}

fn topo_sort_fields(fields: &[FieldDef]) -> Result<Vec<&FieldDef>, String> {
    let names: HashSet<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let mut sorted = Vec::new();
    let mut visiting = HashSet::new();
    let mut done = HashSet::new();
    fn visit<'a>(f: &'a FieldDef, all: &'a [FieldDef], names: &HashSet<&str>, visiting: &mut HashSet<&'a str>, done: &mut HashSet<&'a str>, sorted: &mut Vec<&'a FieldDef>) -> Result<(), String> {
        let n = f.name.as_str();
        if done.contains(n) { return Ok(()); }
        if visiting.contains(n) { return Err(format!("circular dependency: '{}'", n)); }
        visiting.insert(n);
        for dep in f.dependencies() {
            if !names.contains(dep.as_str()) { return Err(format!("'{}' depends on unknown '{}'", n, dep)); }
            let parent = all.iter().find(|pf| pf.name == dep).unwrap();
            visit(parent, all, names, visiting, done, sorted)?;
        }
        done.insert(n);
        sorted.push(f);
        Ok(())
    }
    for f in fields { visit(f, fields, &names, &mut visiting, &mut done, &mut sorted)?; }
    Ok(sorted)
}

fn gen_field(field: &FieldDef, pv: &mut HashMap<String, i64>, rng: &mut impl Rng) -> Vec<String> {
    match &field.range {
        RangeValue::CountFrom { from_field, elem_value } => {
            let cnt = pv.get(from_field).copied().unwrap_or(0).max(0);
            pv.insert(field.name.clone(), cnt);
            if cnt == 0 { return vec![]; }
            (0..cnt).map(|_| {
                let v = elem_value.eval(pv, rng);
                v.to_string()
            }).collect()
        }
        RangeValue::Static { min, max } => {
            let lo = min.eval(pv, rng);
            let hi = max.eval(pv, rng);
            let lo = lo.min(hi);
            let hi = hi.max(lo);
            let (num, s) = gen_scalar(field, lo, hi, rng);
            pv.insert(field.name.clone(), num);
            vec![s]
        }
    }
}

fn gen_scalar(field: &FieldDef, lo: i64, hi: i64, rng: &mut impl Rng) -> (i64, String) {
    // Distribution-aware generation for non-uniform distributions
    match field.distribution {
        Distribution::Binomial => {
            let n = lo.max(1) as u64;
            let mut gen = BinomialGen::new(n, 0.5);
            let v = gen.generate(rng);
            return (v as i64, v.to_string());
        }
        Distribution::Geometric => {
            let mut gen = GeometricGen::new(0.5);
            let v = gen.generate(rng);
            return (v as i64, v.to_string());
        }
        Distribution::LogNormal => {
            let mu = lo as f64;
            let sigma = (hi as f64).max(0.1);
            let mut gen = LogNormalGen::new(mu, sigma);
            let v = gen.generate(rng);
            return (v as i64, format!("{:.6}", v));
        }
        Distribution::Cauchy => {
            let median = lo as f64;
            let scale = (hi as f64).max(0.1);
            let mut gen = CauchyGen::new(median, scale);
            let v = gen.generate(rng);
            return (v as i64, format!("{:.6}", v));
        }
        _ => {} // Uniform, Normal, Exponential, Poisson fall through to default
    }

    // Default: uniform random_range based on data_type
    match field.data_type {
        DataType::Int32 => { let v = rng.random_range((lo as i32)..=(hi as i32)); (v as i64, v.to_string()) }
        DataType::Int64 => { let v = rng.random_range(lo..=hi); (v, v.to_string()) }
        DataType::Float32 => { let v: f32 = rng.random_range((lo as f32)..(hi as f32)); (v as i64, format!("{:.6}", v)) }
        DataType::Float64 => { let v: f64 = rng.random_range(lo as f64..hi as f64); (v as i64, format!("{:.6}", v)) }
        DataType::Char => {
            let idx = rng.random_range(0..52usize);
            let c = if idx < 26 { (b'a' + idx as u8) as char } else { (b'A' + (idx - 26) as u8) as char };
            (c as i64, c.to_string())
        }
        DataType::String => {
            let len = rng.random_range((lo as usize)..=(hi as usize));
            let s: String = (0..len).map(|_| {
                let i = rng.random_range(0..52usize);
                if i < 26 { (b'a' + i as u8) as char } else { (b'A' + (i - 26) as u8) as char }
            }).collect();
            (len as i64, s)
        }
        DataType::BigInt => (0, "0".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int_field(name: &str, min: i32, max: i32) -> FieldDef {
        FieldDef {
            name: name.into(),
            data_type: DataType::Int32,
            distribution: Distribution::Uniform,
            range: RangeValue::Static {
                min: ValueExpr::Const { value: min as i64 },
                max: ValueExpr::Const { value: max as i64 },
            },
            separator: FieldSeparator::default(),
        }
    }

    fn const_expr(v: i64) -> ValueExpr { ValueExpr::Const { value: v } }
    fn from_field_expr(name: &str) -> ValueExpr { ValueExpr::FromField { name: name.into() } }
    fn random_expr(lo: i64, hi: i64) -> ValueExpr {
        ValueExpr::Random {
            distribution: Distribution::Uniform,
            lo: Box::new(ValueExpr::Const { value: lo }),
            hi: Box::new(ValueExpr::Const { value: hi }),
        }
    }
    fn op_expr(field: &str, operator: Op, operand: i64) -> ValueExpr {
        ValueExpr::Op { field: field.into(), operator, operand }
    }

    #[test] fn single_file_int() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![int_field("n",1,100)], seed: Some(42) };
        let r = generate(&c).unwrap();
        assert_eq!(r.files.len(), 1);
    }

    #[test] fn count_from_parent() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 3, 3),
            FieldDef { name: "arr".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::CountFrom { from_field: "n".into(), elem_value: Box::new(random_expr(1, 100)) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 4, "n=3 + 3 values = 4 parts, got {:?}", parts);
        assert_eq!(parts[0], "3");
    }

    #[test] fn count_from_with_op_expr() {
        // n=3, arr uses Op(n, Mul, 10) as elem_value → each element = n*10 = 30
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 3, 3),
            FieldDef { name: "arr".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::CountFrom { from_field: "n".into(), elem_value: Box::new(op_expr("n", Op::Mul, 10)) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "3");
        for i in 1..4 {
            assert_eq!(parts[i], "30", "element {} should be 30 (n*10)", i);
        }
    }

    #[test] fn count_from_with_random_expr() {
        // n=3, arr uses Random(10,20) as elem_value
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 3, 3),
            FieldDef { name: "arr".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::CountFrom { from_field: "n".into(), elem_value: Box::new(random_expr(10, 20)) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "3");
        for i in 1..4 {
            let v: i64 = parts[i].parse().unwrap();
            assert!(v >= 10 && v <= 20, "element {} = {} should be in [10,20]", i, v);
        }
    }

    #[test] fn static_from_field_min_fixed_max() {
        // n=5 fixed, m has Static{min:FromField(\"n\"), max:Const(100)} → m in [5,100]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 5, 5),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: from_field_expr("n"), max: const_expr(100) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "5");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 5 && m <= 100, "Static: m={} should be in [5,100]", m);
    }

    #[test] fn static_fixed_min_from_field_max() {
        // n=100 fixed, m has Static{min:Const(1), max:FromField(\"n\")} → m in [1,100]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 100, 100),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(1), max: from_field_expr("n") }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "100");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 1 && m <= 100, "Static: m={} should be in [1,100]", m);
    }

    #[test] fn static_random_bound() {
        // n=10 fixed, m has Static{min:Const(1), max:Random{1..50}} → m in [1,50]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 10, 10),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(1), max: random_expr(1, 50) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "10");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 1 && m <= 50, "Static Random: m={} should be in [1,50]", m);
    }

    #[test] fn static_both_from_field() {
        // n=5, p=10, m has Static{min:FromField(\"n\"), max:FromField(\"p\")} → m in [5,10]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 5, 5),
            int_field("p", 10, 10),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: from_field_expr("n"), max: from_field_expr("p") }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "5");
        assert_eq!(parts[1], "10");
        let m: i64 = parts[2].parse().unwrap();
        assert!(m >= 5 && m <= 10, "Static both FromField: m={} should be in [5,10]", m);
    }

    #[test] fn static_with_op_min() {
        // n=10, m has Static{min:Const(1), max:Op(n, Min, 50)} → m in [1, min(10,50)=10]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 10, 10),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(1), max: op_expr("n", Op::Min, 50) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "10");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 1 && m <= 10, "Op Min: m={} should be in [1,10]", m);
    }

    #[test] fn static_with_op_max() {
        // n=5, m has Static{min:Op(n, Max, 10), max:Const(100)} → m in [max(5,10)=10, 100]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 5, 5),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: op_expr("n", Op::Max, 10), max: const_expr(100) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "5");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 10 && m <= 100, "Op Max: m={} should be in [10,100]", m);
    }

    #[test] fn static_with_op_mul() {
        // n=5, m has Static{min:Const(1), max:Op(n, Mul, 10)} → m in [1, 50]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 5, 5),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(1), max: op_expr("n", Op::Mul, 10) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "5");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 1 && m <= 50, "Op Mul: m={} should be in [1,50]", m);
    }

    #[test] fn static_with_op_add() {
        // n=5, m has Static{min:Const(1), max:Op(n, Add, 10)} → m in [1, 15]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 5, 5),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(1), max: op_expr("n", Op::Add, 10) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "5");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 1 && m <= 15, "Op Add: m={} should be in [1,15]", m);
    }

    #[test] fn static_with_op_sub() {
        // n=20, m has Static{min:Const(1), max:Op(n, Sub, 5)} → m in [1, 15]
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 20, 20),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(1), max: op_expr("n", Op::Sub, 5) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "20");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 1 && m <= 15, "Op Sub: m={} should be in [1,15]", m);
    }

    #[test] fn same_seed_identical() {
        let c = TestConfig { files_count: 2, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(3), fields: vec![int_field("n",1,1000)], seed: Some(12345) };
        assert_eq!(generate(&c).unwrap().files, generate(&c).unwrap().files);
    }

    #[test] fn files_count_err() { assert!(generate(&TestConfig { files_count: 1001, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![int_field("n",1,100)], seed: Some(42) }).is_err()); }
    #[test] fn empty_fields_err() { assert!(generate(&TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![], seed: Some(42) }).is_err()); }
    #[test] fn circular_err() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "a".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: from_field_expr("b"), max: const_expr(10) }, separator: FieldSeparator::default()},
            FieldDef { name: "b".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: from_field_expr("a"), max: const_expr(10) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        assert!(generate(&c).is_err());
    }

    #[test] fn unknown_dep_err() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "a".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: from_field_expr("nonexistent"), max: const_expr(10) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        assert!(generate(&c).is_err());
    }

    #[test] fn nested_random_expr() {
        // min = Random(1, Random(10, 20)), max = Const(100)
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static {
                min: ValueExpr::Random {
                    distribution: Distribution::Uniform,
                    lo: Box::new(ValueExpr::Const { value: 1 }),
                    hi: Box::new(ValueExpr::Random {
                        distribution: Distribution::Uniform,
                        lo: Box::new(ValueExpr::Const { value: 10 }),
                        hi: Box::new(ValueExpr::Const { value: 20 }),
                    }),
                },
                max: const_expr(100),
            }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: i64 = r.files[0].content.trim().parse().unwrap();
        assert!(v >= 1 && v <= 100, "Nested Random: x={} should be in [1,100]", v);
    }

    #[test] fn multiple_files_with_expr() {
        let c = TestConfig { files_count: 3, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 1, 10),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Static { min: from_field_expr("n"), max: const_expr(100) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        assert_eq!(r.files.len(), 3);
    }

    #[test] fn float32_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Float32, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(10), max: const_expr(20) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: f32 = r.files[0].content.trim().parse().unwrap();
        assert!(v >= 10.0 && v < 20.0, "Float32: x={} should be in [10,20)", v);
    }

    #[test] fn float64_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Float64, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(0), max: const_expr(1) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: f64 = r.files[0].content.trim().parse().unwrap();
        assert!(v >= 0.0 && v < 1.0, "Float64: x={} should be in [0,1)", v);
    }

    #[test] fn string_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "s".into(), data_type: DataType::String, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(3), max: const_expr(5) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let s = r.files[0].content.trim();
        assert!(s.len() >= 3 && s.len() <= 5, "String: len={} should be in [3,5]", s.len());
    }

    #[test] fn char_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "c".into(), data_type: DataType::Char, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(0), max: const_expr(100) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let c = r.files[0].content.trim().chars().next().unwrap();
        assert!(c.is_ascii_alphabetic(), "Char: should be a letter, got {:?}", c);
    }

    #[test] fn int64_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Int64, distribution: Distribution::Uniform, range: RangeValue::Static { min: const_expr(1000000), max: const_expr(2000000) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: i64 = r.files[0].content.trim().parse().unwrap();
        assert!(v >= 1000000 && v <= 2000000, "Int64: x={} should be in [1000000,2000000]", v);
    }

    #[test] fn binomial_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Int32, distribution: Distribution::Binomial, range: RangeValue::Static { min: const_expr(10), max: const_expr(10) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: i64 = r.files[0].content.trim().parse().unwrap();
        assert!(v >= 0 && v <= 10, "Binomial: x={} should be in [0,10]", v);
    }

    #[test] fn geometric_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Int32, distribution: Distribution::Geometric, range: RangeValue::Static { min: const_expr(0), max: const_expr(100) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: i64 = r.files[0].content.trim().parse().unwrap();
        assert!(v >= 0, "Geometric: x={} should be >= 0", v);
    }

    #[test] fn lognormal_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Float64, distribution: Distribution::LogNormal, range: RangeValue::Static { min: const_expr(0), max: const_expr(1) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: f64 = r.files[0].content.trim().parse().unwrap();
        assert!(v > 0.0, "LogNormal: x={} should be > 0", v);
    }

    #[test] fn cauchy_with_expr() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "x".into(), data_type: DataType::Float64, distribution: Distribution::Cauchy, range: RangeValue::Static { min: const_expr(0), max: const_expr(1) }, separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let v: f64 = r.files[0].content.trim().parse().unwrap();
        assert!(v.is_finite(), "Cauchy: x={} should be finite", v);
    }
}