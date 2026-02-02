use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub postgres: PostgresConfig,
    pub openai: OpenAIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    pub connection_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            postgres: PostgresConfig {
                // PLACEHOLDER: Replace with your Postgres connection URL
                // Format: postgres://user:password@host:port/database
                connection_url: "postgres://username:password@localhost:5432/your_database".to_string(),
            },
            openai: OpenAIConfig {
                // PLACEHOLDER: Replace with your OpenAI API key
                api_key: "sk-your-openai-api-key-here".to_string(),
                model: "gpt-4o-mini".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load(config_path: &Path) -> anyhow::Result<Self> {
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config file for user to fill in
            let config = Config::default();
            let content = serde_json::to_string_pretty(&config)?;
            fs::write(config_path, content)?;
            Ok(config)
        }
    }

    pub fn is_placeholder(&self) -> bool {
        self.postgres.connection_url.contains("username:password")
            || self.openai.api_key.contains("your-openai-api-key")
    }
}
