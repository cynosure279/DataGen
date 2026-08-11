use crate::config::ConfigError;
use crate::gen::{
    BinomialGen, CauchyGen, GeometricGen, Generator, LogNormalGen,
};
use crate::types::*;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
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
                // Prefix: separator from the PREVIOUS field
                if first_field { first_field = false; }
                else { content.push(' '); } // default inter-field separator is space
                // Within-field parts: space-separated
                for (j, part) in field_parts.iter().enumerate() {
                    if j > 0 { content.push(' '); }
                    content.push_str(part);
                }
                // After-field separator
                if f.separator == FieldSeparator::Newline { content.push('\n'); }
            }
            // Ensure each testcase line ends with newline (unless already there)
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

fn tc_count(mode: &TestCaseMode, rng: &mut impl RngExt) -> u32 {
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
        if let Some(p) = &f.depends_on {
            if !names.contains(p.as_str()) { return Err(format!("'{}' depends on unknown '{}'", n, p)); }
            let parent = all.iter().find(|pf| pf.name == *p).unwrap();
            visit(parent, all, names, visiting, done, sorted)?;
        }
        done.insert(n);
        sorted.push(f);
        Ok(())
    }
    for f in fields { visit(f, fields, &names, &mut visiting, &mut done, &mut sorted)?; }
    Ok(sorted)
}

fn gen_field(field: &FieldDef, pv: &mut HashMap<String, i64>, rng: &mut impl RngExt) -> Vec<String> {
    match &field.range {
        RangeValue::CountFrom { from_field, elem_min, elem_max } => {
            let cnt = pv.get(from_field).copied().unwrap_or(0).max(0);
            pv.insert(field.name.clone(), cnt);
            if cnt == 0 { return vec![]; }
            (0..cnt).map(|_| rng.random_range(*elem_min..=*elem_max).to_string()).collect()
        }
        RangeValue::ValueFrom { from_field, multiplier } => {
            let p = pv.get(from_field).copied().unwrap_or(1) as f64;
            let max = (p * multiplier).max(1.0) as i64;
            let v = rng.random_range(1..=max);
            pv.insert(field.name.clone(), v);
            vec![v.to_string()]
        }
        RangeValue::RangeFrom { from_field, min_mult, max_mult } => {
            let p = pv.get(from_field).copied().unwrap_or(1) as f64;
            let min = (p * min_mult).max(1.0) as i64;
            let max = (p * max_mult).max(min as f64) as i64;
            let v = rng.random_range(min..=max);
            pv.insert(field.name.clone(), v);
            vec![v.to_string()]
        }
        _ => {
            let (num, s) = gen_scalar(field, rng);
            pv.insert(field.name.clone(), num);
            vec![s]
        }
    }
}

/// Extract (min, max) as f64 from a RangeValue, falling back to defaults.
fn extract_float_range(range: &RangeValue, default_min: f64, default_max: f64) -> (f64, f64) {
    match range {
        RangeValue::Float32(r) => (r.min as f64, r.max as f64),
        RangeValue::Float64(r) => (r.min, r.max),
        RangeValue::Int32(r) => (r.min as f64, r.max as f64),
        RangeValue::Int64(r) => (r.min as f64, r.max as f64),
        _ => (default_min, default_max),
    }
}

fn gen_scalar(field: &FieldDef, rng: &mut impl RngExt) -> (i64, String) {
    // Distribution-aware generation for non-uniform distributions
    match field.distribution {
        Distribution::Binomial => {
            let n = match &field.range {
                RangeValue::Int32(r) => r.min.max(1) as u64,
                RangeValue::Int64(r) => r.min.max(1) as u64,
                _ => 1,
            };
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
            let (mu, sigma) = extract_float_range(&field.range, 0.0, 1.0);
            let mut gen = LogNormalGen::new(mu, sigma.max(0.1));
            let v = gen.generate(rng);
            return (v as i64, format!("{:.6}", v));
        }
        Distribution::Cauchy => {
            let (median, scale) = extract_float_range(&field.range, 0.0, 1.0);
            let mut gen = CauchyGen::new(median, scale.max(0.1));
            let v = gen.generate(rng);
            return (v as i64, format!("{:.6}", v));
        }
        _ => {} // Uniform, Normal, Exponential, Poisson fall through to default
    }

    // Default: uniform random_range (existing behavior)
    match (&field.data_type, &field.range) {
        (DataType::Int32, RangeValue::Int32(r)) => { let v = rng.random_range(r.min..=r.max); (v as i64, v.to_string()) }
        (DataType::Int64, RangeValue::Int64(r)) => { let v = rng.random_range(r.min..=r.max); (v, v.to_string()) }
        (DataType::Float32, RangeValue::Float32(r)) => { let v: f32 = rng.random_range(r.min..r.max); (v as i64, format!("{:.6}", v)) }
        (DataType::Float64, RangeValue::Float64(r)) => { let v: f64 = rng.random_range(r.min..r.max); (v as i64, format!("{:.6}", v)) }
        (DataType::Char, RangeValue::Char(_)) => {
            let idx = rng.random_range(0..52usize);
            let c = if idx < 26 { (b'a' + idx as u8) as char } else { (b'A' + (idx - 26) as u8) as char };
            (c as i64, c.to_string())
        }
        (DataType::String, RangeValue::StringLen(r)) => {
            let len = rng.random_range(r.min..=r.max);
            let s: String = (0..len).map(|_| {
                let i = rng.random_range(0..52usize);
                if i < 26 { (b'a' + i as u8) as char } else { (b'A' + (i - 26) as u8) as char }
            }).collect();
            (len as i64, s)
        }
        _ => (0, "0".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn int_field(name: &str, min: i32, max: i32) -> FieldDef {
        FieldDef { name: name.into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Int32(Range::new(min, max)), depends_on: None, separator: FieldSeparator::default() }
    }

    #[test] fn single_file_int() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![int_field("n",1,100)], seed: Some(42) };
        let r = generate(&c).unwrap();
        assert_eq!(r.files.len(), 1);
    }

    #[test] fn count_from_parent() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 3, 3),
            FieldDef { name: "arr".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::CountFrom { from_field: "n".into(), elem_min: 1, elem_max: 100 }, depends_on: Some("n".into()) , separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 4, "n=3 + 3 values = 4 parts, got {:?}", parts);
        assert_eq!(parts[0], "3");
    }

    #[test] fn value_from_parent() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 10, 10),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::ValueFrom { from_field: "n".into(), multiplier: 1.0 }, depends_on: Some("n".into()) , separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "10");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 1 && m <= 10);
    }

    #[test] fn range_from_parent() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            int_field("n", 10, 10),
            FieldDef { name: "m".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::RangeFrom { from_field: "n".into(), min_mult: 0.5, max_mult: 2.0 }, depends_on: Some("n".into()) , separator: FieldSeparator::default()},
        ], seed: Some(42) };
        let r = generate(&c).unwrap();
        let parts: Vec<_> = r.files[0].content.trim().split(' ').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "10");
        let m: i64 = parts[1].parse().unwrap();
        assert!(m >= 5 && m <= 20, "m={} should be in [5,20]", m);
    }

    #[test] fn same_seed_identical() {
        let c = TestConfig { files_count: 2, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(3), fields: vec![int_field("n",1,1000)], seed: Some(12345) };
        assert_eq!(generate(&c).unwrap().files, generate(&c).unwrap().files);
    }

    #[test] fn files_count_err() { assert!(generate(&TestConfig { files_count: 1001, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![int_field("n",1,100)], seed: Some(42) }).is_err()); }
    #[test] fn empty_fields_err() { assert!(generate(&TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![], seed: Some(42) }).is_err()); }
    #[test] fn circular_err() {
        let c = TestConfig { files_count: 1, prefix: "t".into(), suffix: String::new(), testcase_mode: TestCaseMode::Fixed(1), fields: vec![
            FieldDef { name: "a".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Int32(Range::new(1,10)), depends_on: Some("b".into()) , separator: FieldSeparator::default()},
            FieldDef { name: "b".into(), data_type: DataType::Int32, distribution: Distribution::Uniform, range: RangeValue::Int32(Range::new(1,10)), depends_on: Some("a".into()) , separator: FieldSeparator::default()},
        ], seed: Some(42) };
        assert!(generate(&c).is_err());
    }
}
