use std::sync::LazyLock;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_region")]
    pub aws_region: String,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_session_token: Option<String>,
}

impl Config {
    pub fn has_credentials(&self) -> bool {
        self.aws_access_key_id.is_some() && self.aws_secret_access_key.is_some()
    }
}

fn default_region() -> String {
    "us-east-1".to_string()
}

pub static CONFIG: LazyLock<Config> =
    LazyLock::new(|| envy::from_env().expect("failed to get config"));