pub mod types;
pub mod config;
pub mod gen;
pub mod orchestrator;

#[cfg(test)]
mod tests {
    use super::types::*;
    use super::config::ConfigError;

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
                range: RangeValue::Int32(Range::new(1, 1000)),
                depends_on: None, separator: FieldSeparator::default(),
            }],
            seed: Some(42),
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed = TestConfig::from_toml(&toml_str).unwrap();
        assert_eq!(config, parsed);
    }

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
                range: RangeValue::Int32(Range::new(1, 100)),
                depends_on: None, separator: FieldSeparator::default(),
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
                range: RangeValue::Int32(Range::new(1, 100)),
                depends_on: None, separator: FieldSeparator::default(),
            }],
            seed: Some(42),
        };
        assert!(config.validate().is_ok());
    }
}