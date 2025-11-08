//! Configuration file support for batch analysis

use crate::contact::ContactCriteria;
use crate::error::{ContactDetectorError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for a single contact pair analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactPairConfig {
    /// Name of the first surface/part
    pub surface_a: String,

    /// Name of the second surface/part
    pub surface_b: String,

    /// Contact detection criteria
    #[serde(default)]
    pub criteria: ContactCriteria,

    /// Output filename (optional, will be auto-generated if not specified)
    pub output_file: Option<String>,
}

/// Top-level configuration for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Input Exodus file path
    pub input_file: String,

    /// Output directory for results
    pub output_dir: String,

    /// List of contact pairs to analyze
    pub contact_pairs: Vec<ContactPairConfig>,

    /// Global contact criteria (can be overridden per pair)
    #[serde(default)]
    pub default_criteria: ContactCriteria,
}

impl AnalysisConfig {
    /// Load configuration from a JSON file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ContactDetectorError::ConfigError(format!("Failed to read config file: {}", e))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            ContactDetectorError::ConfigError(format!("Failed to parse config file: {}", e))
        })
    }

    /// Save configuration to a JSON file
    pub fn to_file(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            ContactDetectorError::ConfigError(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(path, content).map_err(|e| {
            ContactDetectorError::ConfigError(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    /// Parse contact pairs from command-line string
    /// Format: "PartA:PartB,PartC:PartD"
    pub fn from_pairs_string(
        input_file: String,
        output_dir: String,
        pairs_str: &str,
        default_criteria: ContactCriteria,
    ) -> Result<Self> {
        let mut contact_pairs = Vec::new();

        for pair in pairs_str.split(',') {
            let parts: Vec<&str> = pair.trim().split(':').collect();
            if parts.len() != 2 {
                return Err(ContactDetectorError::ConfigError(format!(
                    "Invalid pair format: '{}'. Expected 'PartA:PartB'",
                    pair
                )));
            }

            contact_pairs.push(ContactPairConfig {
                surface_a: parts[0].trim().to_string(),
                surface_b: parts[1].trim().to_string(),
                criteria: default_criteria.clone(),
                output_file: None,
            });
        }

        Ok(AnalysisConfig {
            input_file,
            output_dir,
            contact_pairs,
            default_criteria,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pairs_string() {
        let config = AnalysisConfig::from_pairs_string(
            "test.exo".to_string(),
            "output".to_string(),
            "Block1:Block2, Block3:Block4",
            ContactCriteria::default(),
        )
        .unwrap();

        assert_eq!(config.contact_pairs.len(), 2);
        assert_eq!(config.contact_pairs[0].surface_a, "Block1");
        assert_eq!(config.contact_pairs[0].surface_b, "Block2");
        assert_eq!(config.contact_pairs[1].surface_a, "Block3");
        assert_eq!(config.contact_pairs[1].surface_b, "Block4");
    }

    #[test]
    fn test_invalid_pairs_string() {
        let result = AnalysisConfig::from_pairs_string(
            "test.exo".to_string(),
            "output".to_string(),
            "Block1:Block2:Block3",
            ContactCriteria::default(),
        );

        assert!(result.is_err());
    }
}
