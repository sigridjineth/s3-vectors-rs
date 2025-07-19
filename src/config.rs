use std::fmt;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_region")]
    pub aws_region: String,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_session_token: Option<String>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("aws_region", &self.aws_region)
            .field(
                "aws_access_key_id",
                &self.aws_access_key_id.as_ref().map(|_| "***REDACTED***"),
            )
            .field(
                "aws_secret_access_key",
                &self
                    .aws_secret_access_key
                    .as_ref()
                    .map(|_| "***REDACTED***"),
            )
            .field(
                "aws_session_token",
                &self.aws_session_token.as_ref().map(|_| "***REDACTED***"),
            )
            .finish()
    }
}

impl Config {
    pub fn has_credentials(&self) -> bool {
        self.aws_access_key_id.is_some() && self.aws_secret_access_key.is_some()
    }
}

fn default_region() -> String {
    "us-east-1".to_string()
}

// Store the result of loading config, not the config itself
pub static CONFIG: LazyLock<Result<Config>> = LazyLock::new(|| {
    envy::from_env::<Config>()
        .context("Failed to load AWS configuration from environment variables")
});

// Helper function to get config or use defaults
pub fn get_config() -> Config {
    match CONFIG.as_ref() {
        Ok(config) => config.clone(),
        Err(_) => Config {
            aws_region: default_region(),
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_debug_redacts_credentials() {
        let config = Config {
            aws_region: "us-east-1".to_string(),
            aws_access_key_id: Some("AKIAXXXXXXXX".to_string()),
            aws_secret_access_key: Some("secret123".to_string()),
            aws_session_token: Some("token456".to_string()),
        };

        let debug_output = format!("{:?}", config);

        // These assertions will fail until we implement custom Debug
        assert!(
            !debug_output.contains("secret123"),
            "Secret key should be redacted"
        );
        assert!(
            !debug_output.contains("token456"),
            "Session token should be redacted"
        );
        assert!(
            debug_output.contains("***REDACTED***"),
            "Should show REDACTED for sensitive fields"
        );
        assert!(
            debug_output.contains("us-east-1"),
            "Region should be visible"
        );
    }

    #[test]
    fn test_get_config_returns_defaults_on_error() {
        // Test that get_config handles both scenarios:
        // 1. When env vars are not set (returns defaults)
        // 2. When env vars are set (returns actual values)
        let config = get_config();

        // Region should always have a value (either from env or default)
        assert!(!config.aws_region.is_empty());

        // If CONFIG loaded successfully from env, it might have credentials
        // If it failed to load from env, it should return defaults with no credentials
        match CONFIG.as_ref() {
            Ok(_) => {
                // Config loaded from env - credentials may or may not be present
                // This is valid behavior
            }
            Err(_) => {
                // Config failed to load - should return defaults
                assert_eq!(config.aws_region, "us-east-1");
                assert!(config.aws_access_key_id.is_none());
                assert!(config.aws_secret_access_key.is_none());
                assert!(config.aws_session_token.is_none());
            }
        }
    }
}
