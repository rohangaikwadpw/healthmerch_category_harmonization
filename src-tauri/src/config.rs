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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            postgres: PostgresConfig {
                connection_url: std::env::var("POSTGRES_URL")
                    .unwrap_or_else(|_| "postgres://username:password@localhost:5432/your_database".to_string()),
            },
            openai: OpenAIConfig {
                api_key: std::env::var("OPENAI_API_KEY")
                    .unwrap_or_else(|_| "sk-your-openai-api-key-here".to_string()),
                model: std::env::var("OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4".to_string()),
                base_url: std::env::var("OPENAI_API_BASE").ok(),
            },
        }
    }
}

impl Config {
    pub fn load(config_path: &Path) -> anyhow::Result<Self> {
        // First try to load from .env file
        let env_path = std::path::PathBuf::from(".env");
        if env_path.exists() {
            let _ = dotenvy::from_path(&env_path);
        } else {
            let _ = dotenvy::dotenv();
        }
        
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let mut config: Config = serde_json::from_str(&content)?;
            
            // Override with environment variables if available
            if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                if !api_key.is_empty() && !api_key.contains("your-openai-api-key") {
                    config.openai.api_key = api_key;
                }
            }
            if let Ok(base_url) = std::env::var("OPENAI_API_BASE") {
                if !base_url.is_empty() {
                    config.openai.base_url = Some(base_url);
                }
            }
            
            Ok(config)
        } else {
            // Try loading from environment variables
            let config = Config::default();
            
            // Only create config.json if env vars are not set
            if config.is_placeholder() {
                let content = serde_json::to_string_pretty(&config)?;
                fs::write(config_path, content)?;
            }
            
            Ok(config)
        }
    }

    pub fn is_placeholder(&self) -> bool {
        self.postgres.connection_url.is_empty()
            || self.postgres.connection_url.contains("username:password")
            || self.postgres.connection_url.contains("YOUR_POSTGRES_URL_HERE")
            || self.openai.api_key.is_empty()
            || self.openai.api_key.contains("your-openai-api-key")
            || self.openai.api_key.contains("YOUR_OPENAI_API_KEY_HERE")
    }
}
