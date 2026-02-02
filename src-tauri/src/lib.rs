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

// Application state
pub struct AppState {
    pub config: Mutex<Option<Config>>,
    pub taxonomy: Mutex<String>,
    pub products: Mutex<Vec<ProductData>>,
    pub mappings: Mutex<HashMap<String, CategoryMapping>>,
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

    // Call OpenAI to map categories
    let openai_client = OpenAIClient::new(&config.openai.api_key, &config.openai.model);

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
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            load_taxonomy,
            load_taxonomy_content,
            parse_input_csv,
            fetch_products_from_db,
            harmonize_categories,
            export_to_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
