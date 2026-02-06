mod config;
mod db;
mod openai;

use config::Config;
use db::{ProductData, ProductInput};
use openai::{CategoryMapping, OpenAIClient};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use std::fs;

// Application state
pub struct AppState {
    pub config: Mutex<Option<Config>>,
    pub taxonomy: Mutex<String>,
    pub products: Mutex<Vec<ProductData>>,
    pub mappings: Mutex<HashMap<String, CategoryMapping>>,
    pub api_key: Mutex<Option<String>>,
    pub api_base_url: Mutex<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStatus {
    pub stage: String,
    pub progress: f32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonizedProduct {
    pub id: String, // UUID as string
    pub product_id: String,
    pub supplier_id: String,
    pub product_name: String,
    pub raw_category: String,
    pub main_category: String,
    pub sub_category: String,
    pub sub_sub_category: String,
    pub confidence: String,
}

// ============ TAURI COMMANDS ============

#[tauri::command]
async fn save_api_key(api_key: String, api_base_url: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let env_path = PathBuf::from(".env");
    
    // Store in memory
    *state.api_key.lock().unwrap() = Some(api_key.clone());
    *state.api_base_url.lock().unwrap() = api_base_url.clone();
    
    // Write to .env file
    let mut content = format!("OPENAI_API_KEY={}\n", api_key);
    if let Some(url) = &api_base_url {
        content.push_str(&format!("OPENAI_API_BASE={}", url));
    }
    
    fs::write(&env_path, content)
        .map_err(|e| format!("Failed to write .env file: {}", e))?;
    
    // Reload environment
    dotenvy::from_path(&env_path)
        .map_err(|e| format!("Failed to load .env: {}", e))?;
    
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiConfig {
    api_key: Option<String>,
    api_base_url: Option<String>,
}

#[tauri::command]
async fn get_api_key(state: State<'_, AppState>) -> Result<Option<ApiConfig>, String> {
    // First check memory
    let api_key = state.api_key.lock().unwrap().clone();
    let api_base_url = state.api_base_url.lock().unwrap().clone();
    
    if api_key.is_some() {
        return Ok(Some(ApiConfig { api_key, api_base_url }));
    }
    
    // Try to read from .env file
    let env_path = PathBuf::from(".env");
    if env_path.exists() {
        let content = fs::read_to_string(&env_path)
            .map_err(|e| format!("Failed to read .env file: {}", e))?;
        
        let mut key = None;
        let mut base_url = None;
        
        for line in content.lines() {
            if line.starts_with("OPENAI_API_KEY=") {
                key = Some(line.split('=').nth(1).unwrap_or("").to_string());
            } else if line.starts_with("OPENAI_API_BASE=") {
                base_url = Some(line.split('=').nth(1).unwrap_or("").to_string());
            }
        }
        
        if key.is_some() {
            *state.api_key.lock().unwrap() = key.clone();
            *state.api_base_url.lock().unwrap() = base_url.clone();
            return Ok(Some(ApiConfig { api_key: key, api_base_url: base_url }));
        }
    }
    
    Ok(None)
}

#[tauri::command]
async fn delete_api_key(state: State<'_, AppState>) -> Result<(), String> {
    // Clear from memory
    *state.api_key.lock().unwrap() = None;
    *state.api_base_url.lock().unwrap() = None;
    
    // Delete .env file if it exists
    let env_path = PathBuf::from(".env");
    if env_path.exists() {
        fs::remove_file(&env_path)
            .map_err(|e| format!("Failed to delete .env file: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
async fn load_config(state: State<'_, AppState>) -> Result<bool, String> {
    // Try multiple possible config locations
    let possible_paths = [
        PathBuf::from("config.json"),
        PathBuf::from("../config.json"),
        PathBuf::from("src-tauri/config.json"),
    ];

    let mut last_error = String::new();

    for config_path in &possible_paths {
        match Config::load(config_path) {
            Ok(config) => {
                let is_placeholder = config.is_placeholder();
                *state.config.lock().unwrap() = Some(config);

                if is_placeholder {
                    return Ok(false); // Config exists but has placeholder values
                } else {
                    return Ok(true); // Config is ready
                }
            }
            Err(e) => {
                last_error = format!("{}", e);
                continue;
            }
        }
    }

    Err(format!("Failed to load config from any location: {}", last_error))
}

#[tauri::command]
async fn load_taxonomy(state: State<'_, AppState>, taxonomy_path: String) -> Result<usize, String> {
    let content = std::fs::read_to_string(&taxonomy_path)
        .map_err(|e| format!("Failed to read taxonomy file: {}", e))?;

    let line_count = content.lines().count();
    *state.taxonomy.lock().unwrap() = content;

    Ok(line_count)
}

#[tauri::command]
async fn load_taxonomy_content(state: State<'_, AppState>, content: String) -> Result<usize, String> {
    let line_count = content.lines().count();
    *state.taxonomy.lock().unwrap() = content;

    Ok(line_count)
}

#[tauri::command]
async fn parse_input_csv(csv_content: String) -> Result<Vec<ProductInput>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_content.as_bytes());

    let mut products = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| format!("CSV parse error: {}", e))?;

        if record.len() >= 2 {
            products.push(ProductInput {
                product_id: record.get(0).unwrap_or("").trim().to_string(),
                supplier_id: record.get(1).unwrap_or("").trim().to_string(),
            });
        }
    }

    Ok(products)
}

#[tauri::command]
async fn fetch_products_from_db(
    state: State<'_, AppState>,
    inputs: Vec<ProductInput>,
) -> Result<Vec<ProductData>, String> {
    let config = state.config.lock().unwrap().clone()
        .ok_or("Config not loaded")?;

    let client = db::connect(&config.postgres.connection_url)
        .await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let products = db::fetch_products(&client, &inputs)
        .await
        .map_err(|e| format!("Failed to fetch products: {}", e))?;

    *state.products.lock().unwrap() = products.clone();

    Ok(products)
}

#[tauri::command]
async fn harmonize_categories(
    state: State<'_, AppState>,
) -> Result<Vec<HarmonizedProduct>, String> {
    let config = state.config.lock().unwrap().clone()
        .ok_or("Config not loaded")?;
    let taxonomy = state.taxonomy.lock().unwrap().clone();
    let products = state.products.lock().unwrap().clone();

    if taxonomy.is_empty() {
        return Err("Taxonomy not loaded".to_string());
    }

    if products.is_empty() {
        return Err("No products loaded".to_string());
    }

    // Get unique raw categories
    let unique_categories: Vec<String> = products
        .iter()
        .map(|p| p.raw_category.clone())
        .filter(|c| !c.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Get API base URL from state (if provided), otherwise from config
    let api_base_url = state.api_base_url.lock().unwrap().clone()
        .or_else(|| config.openai.base_url.clone());

    // Call OpenAI to map categories
    let openai_client = OpenAIClient::new(
        &config.openai.api_key,
        &config.openai.model,
        api_base_url.as_deref(),
    );

    let mappings = openai_client
        .map_categories(&unique_categories, &taxonomy)
        .await
        .map_err(|e| format!("OpenAI mapping failed: {}", e))?;

    *state.mappings.lock().unwrap() = mappings.clone();

    // Apply mappings to products
    let harmonized: Vec<HarmonizedProduct> = products
        .iter()
        .map(|p| {
            let mapping = mappings.get(&p.raw_category);
            HarmonizedProduct {
                id: p.id.clone(),
                product_id: p.product_id.clone(),
                supplier_id: p.supplier_id.clone(),
                product_name: p.product_name.clone(),
                raw_category: p.raw_category.clone(),
                main_category: mapping.map(|m| m.main_category.clone()).unwrap_or_default(),
                sub_category: mapping.map(|m| m.sub_category.clone()).unwrap_or_default(),
                sub_sub_category: mapping.map(|m| m.sub_sub_category.clone()).unwrap_or_default(),
                confidence: mapping.map(|m| m.confidence.clone()).unwrap_or("none".to_string()),
            }
        })
        .collect();

    Ok(harmonized)
}

#[tauri::command]
fn export_to_csv(products: Vec<HarmonizedProduct>, output_path: String) -> Result<String, String> {
    let mut writer = csv::Writer::from_path(&output_path)
        .map_err(|e| format!("Failed to create CSV file: {}", e))?;

    // Write header
    writer
        .write_record(&[
            "id",
            "product_id",
            "supplier_id",
            "product_name",
            "raw_category",
            "main_category",
            "sub_category",
            "sub_sub_category",
            "confidence",
        ])
        .map_err(|e| format!("Failed to write header: {}", e))?;

    // Write records
    for p in &products {
        writer
            .write_record(&[
                &p.id,
                &p.product_id,
                &p.supplier_id,
                &p.product_name,
                &p.raw_category,
                &p.main_category,
                &p.sub_category,
                &p.sub_sub_category,
                &p.confidence,
            ])
            .map_err(|e| format!("Failed to write record: {}", e))?;
    }

    writer.flush().map_err(|e| format!("Failed to flush: {}", e))?;

    Ok(format!("Exported {} products to {}", products.len(), output_path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            config: Mutex::new(None),
            taxonomy: Mutex::new(String::new()),
            products: Mutex::new(Vec::new()),
            mappings: Mutex::new(HashMap::new()),
            api_key: Mutex::new(None),
            api_base_url: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            save_api_key,
            get_api_key,
            delete_api_key,
            load_config,
            load_taxonomy,
            load_taxonomy_content,
            parse_input_csv,
            fetch_products_from_db,
            harmonize_categories,
            export_to_csv,
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Delete API key when window closes
                let _ = std::fs::remove_file(".env");
            }
        })
        .setup(|_app| {
            // Cleanup .env file on startup to ensure fresh start
            let _ = std::fs::remove_file(".env");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
