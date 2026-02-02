use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMapping {
    pub raw_category: String,
    pub main_category: String,
    pub sub_category: String,
    pub sub_sub_category: String,
    pub confidence: String,
}

pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAIClient {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn map_categories(
        &self,
        raw_categories: &[String],
        taxonomy: &str,
    ) -> Result<HashMap<String, CategoryMapping>> {
        let mut mappings = HashMap::new();

        // Process in batches of 10 to avoid token limits
        for chunk in raw_categories.chunks(10) {
            let batch_mappings = self.map_category_batch(chunk, taxonomy).await?;
            mappings.extend(batch_mappings);
        }

        Ok(mappings)
    }

    async fn map_category_batch(
        &self,
        raw_categories: &[String],
        taxonomy: &str,
    ) -> Result<HashMap<String, CategoryMapping>> {
        let categories_list = raw_categories
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{}. {}", i + 1, c))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"You are a product categorization expert. Map the following raw product categories to the standardized taxonomy provided.

TAXONOMY:
{}

RAW CATEGORIES TO MAP:
{}

For each raw category, respond with a JSON array where each object has:
- "raw_category": the original category string
- "main_category": the matched Main Category from taxonomy
- "sub_category": the matched Sub-Category from taxonomy (or empty string if none)
- "sub_sub_category": the matched Sub-Sub-Category from taxonomy (or empty string if none)
- "confidence": "high", "medium", or "low"

Respond ONLY with the JSON array, no other text."#,
            taxonomy, categories_list
        );

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }

        let chat_response: ChatResponse = response.json().await?;
        let content = &chat_response.choices[0].message.content;

        // Parse the JSON response
        let parsed: Vec<CategoryMapping> = serde_json::from_str(content.trim())?;

        let mut mappings = HashMap::new();
        for mapping in parsed {
            mappings.insert(mapping.raw_category.clone(), mapping);
        }

        Ok(mappings)
    }
}
