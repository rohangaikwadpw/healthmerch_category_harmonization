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
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInfo {
    pub product_name: String,
    pub description: String,
    pub raw_category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsageStats {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: &str, model: &str, custom_base_url: Option<&str>) -> Self {
        let base_url = custom_base_url
            .unwrap_or("https://api.openai.com/v1")
            .trim_end_matches('/')
            .trim_end_matches("/chat/completions")
            .to_string();
        
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            base_url,
        }
    }

    pub async fn map_categories(
        &self,
        products: &[ProductInfo],
        taxonomy: &str,
    ) -> Result<(HashMap<String, CategoryMapping>, TokenUsageStats)> {
        let mut mappings = HashMap::new();
        let mut total_usage = TokenUsageStats::default();

        // Process in batches of 10 to avoid token limits
        for chunk in products.chunks(10) {
            let (batch_mappings, batch_usage) = self.map_category_batch(chunk, taxonomy).await?;
            mappings.extend(batch_mappings);
            total_usage.input_tokens += batch_usage.input_tokens;
            total_usage.output_tokens += batch_usage.output_tokens;
            total_usage.total_tokens += batch_usage.total_tokens;
        }

        Ok((mappings, total_usage))
    }

    async fn map_category_batch(
        &self,
        products: &[ProductInfo],
        taxonomy: &str,
    ) -> Result<(HashMap<String, CategoryMapping>, TokenUsageStats)> {
        let products_list = products
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "{}. Product Name: {}\n   Description: {}\n   Current Category: {}",
                    i + 1,
                    p.product_name,
                    if p.description.is_empty() { "N/A" } else { &p.description },
                    p.raw_category
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
             r#"You are a product categorization expert. Map the following products to the standardized taxonomy provided.

TAXONOMY:
{}

PRODUCTS TO CATEGORIZE:
{}

For each product, use the product name, description, and current category to determine the best matching category from the taxonomy. Respond with a JSON array where each object has:
- "raw_category": the original category string from the product
- "main_category": the matched Main Category from taxonomy
- "sub_category": the matched Sub-Category from taxonomy (or empty string if none)
- "sub_sub_category": the matched Sub-Sub-Category from taxonomy (or empty string if none)
- "confidence": "high", "medium", or "low" based on how well the product matches the category

Consider the product name and description to make more accurate categorization decisions. Respond ONLY with the JSON array, no other text."#,
            taxonomy, products_list
        );

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        println!("Sending request to: {}/chat/completions", self.base_url);
        println!("Using model: {}", self.model);
        println!("Processing {} products", products.len());

        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json");
        
        // Only add Authorization header if API key is actually provided and not a placeholder
        if !self.api_key.is_empty() 
            && self.api_key != "not-required" 
            && !self.api_key.contains("your-openai-api-key") {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
            println!("API key configured (length: {})", self.api_key.len());
        } else {
            println!("WARNING: No valid API key configured");
        }
        
        let response = req
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    anyhow::anyhow!(
                        "Failed to connect to {}. Make sure your AI server is running and accessible.",
                        self.base_url
                    )
                } else if e.is_timeout() {
                    anyhow::anyhow!("Request timed out. Your AI server might be overloaded or not responding.")
                } else {
                    anyhow::anyhow!("Network error: {}", e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            println!("API Error (status {}): {}", status, error_text);
            anyhow::bail!("OpenAI API error (status {}): {}", status, error_text);
        }

        println!("Received successful response from API");

        let chat_response: ChatResponse = response.json().await
            .map_err(|e| {
                println!("Failed to parse API response as JSON: {}", e);
                anyhow::anyhow!("Failed to parse API response as JSON: {}. Make sure the API endpoint is correct.", e)
            })?;
        
        if chat_response.choices.is_empty() {
            println!("API returned no choices in response");
            anyhow::bail!("API returned no choices in response");
        }
        
        let content = &chat_response.choices[0].message.content;
        println!("Raw AI response (first 200 chars): {}", &content.chars().take(200).collect::<String>());

        // Clean the response - remove markdown code blocks if present
        let cleaned_content = content.trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if cleaned_content.is_empty() {
            println!("API returned empty content after cleaning");
            anyhow::bail!("API returned empty content");
        }

        println!("Cleaned content (first 200 chars): {}", &cleaned_content.chars().take(200).collect::<String>());

        // Parse the JSON response
        let parsed: Vec<CategoryMapping> = serde_json::from_str(cleaned_content)
            .map_err(|e| {
                println!("JSON parse error: {}", e);
                anyhow::anyhow!(
                    "Failed to parse AI response as category mappings. Error: {}\n\nAI Response:\n{}\n\nCleaned:\n{}",
                    e,
                    content,
                    cleaned_content
                )
            })?;

        println!("Successfully parsed {} category mappings", parsed.len());

        let mut mappings = HashMap::new();
        for mapping in parsed {
            mappings.insert(mapping.raw_category.clone(), mapping);
        }

        // Extract token usage
        let usage = if let Some(usage_data) = chat_response.usage {
            TokenUsageStats {
                input_tokens: usage_data.prompt_tokens,
                output_tokens: usage_data.completion_tokens,
                total_tokens: usage_data.total_tokens,
            }
        } else {
            TokenUsageStats::default()
        };

        Ok((mappings, usage))
    }
}
