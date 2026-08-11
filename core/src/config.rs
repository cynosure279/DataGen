use crate::types::{TestCaseMode, TestConfig};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("TOML parse error: {0}")]
    ParseError(String),
    #[error("Config validation error: {0}")]
    ValidationError(String),
}

impl TestConfig {
    pub fn from_toml(input: &str) -> Result<Self, ConfigError> {
        toml::from_str(input).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.files_count > 1000 {
            return Err(ConfigError::ValidationError("files_count must be <= 1000".into()));
        }
        if let TestCaseMode::Fixed(t) = self.testcase_mode {
            if t > 1000 {
                return Err(ConfigError::ValidationError("testcase count must be <= 1000".into()));
            }
        }
        if let TestCaseMode::Random { range, .. } = &self.testcase_mode {
            if range.max > 1000 {
                return Err(ConfigError::ValidationError("testcase range max must be <= 1000".into()));
            }
        }
        if self.fields.is_empty() {
            return Err(ConfigError::ValidationError("at least one field is required".into()));
        }
        Ok(())
    }
}