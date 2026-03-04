mod config;
mod db;
mod openai;

use config::Config;
use db::{ProductData, ProductInput};
use openai::{CategoryMapping, OpenAIClient, ProductInfo};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use std::fs;

// Application state
pub struct AppState {
    pub config: Mutex<Option<Config>>,
    pub taxonomy: Mutex<String>,
    pub products: Mutex<Vec<ProductData>>,
    pub mappings: Mutex<HashMap<String, CategoryMapping>>,
    pub api_key: Mutex<Option<String>>,
    pub api_base_url: Mutex<Option<String>>,
    pub token_usage: Mutex<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost: f64,
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
    pub description: String,
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
async fn load_config(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    // Try multiple possible config locations
    let mut possible_paths = vec![
        PathBuf::from("config.json"),
        PathBuf::from("../config.json"),
        PathBuf::from("src-tauri/config.json"),
    ];

    // Add resource directory path for bundled app
    if let Ok(resource_path) = app.path().resource_dir() {
        let resource_config = resource_path.join("config.json");
        possible_paths.insert(0, resource_config);
    }

    for config_path in &possible_paths {
        match Config::load(config_path) {
            Ok(config) => {
                println!("Loaded config from: {:?}", config_path);
                let is_placeholder = config.is_placeholder();
                *state.config.lock().unwrap() = Some(config);

                if is_placeholder {
                    return Ok(false); // Config exists but has placeholder values
                } else {
                    return Ok(true); // Config is ready
                }
            }
            Err(_e) => {
                // Continue trying other paths
                continue;
            }
        }
    }

    // No config file found in any location - use default config
    println!("No config file found, using default configuration");
    *state.config.lock().unwrap() = Some(Config::default());
    Ok(false)
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
async fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
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

    println!("=== Database Connection Attempt ===");
    println!("Connection URL: postgresql://[user]:***@{}", 
        config.postgres.connection_url.split('@').nth(1).unwrap_or("unknown"));
    println!("Number of products to fetch: {}", inputs.len());
    
    let client = db::connect(&config.postgres.connection_url)
        .await
        .map_err(|e| {
            eprintln!("=== DATABASE CONNECTION FAILED ===");
            eprintln!("Error type: {:?}", e);
            eprintln!("Error message: {}", e);
            eprintln!("Possible causes:");
            eprintln!("  1. Database user doesn't have LOGIN permission");
            eprintln!("  2. Windows Firewall is blocking the connection");
            eprintln!("  3. Antivirus software is blocking network access");
            eprintln!("  4. Network connectivity issue");
            eprintln!("=================================");
            format!("Database connection failed: {}", e)
        })?;

    println!("✓ Successfully connected to database");
    let products = db::fetch_products(&client, &inputs)
        .await
        .map_err(|e| {
            eprintln!("Failed to fetch products: {}", e);
            format!("Failed to fetch products: {}", e)
        })?;

    println!("✓ Successfully fetched {} products", products.len());
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

    // Validate API key is not a placeholder
    if config.openai.api_key.is_empty() 
        || config.openai.api_key.contains("your-openai-api-key")
        || config.openai.api_key.contains("will-be-provided-via-ui") {
        return Err("OpenAI API key not configured. Please enter your API key in the Settings.".to_string());
    }

    // Get unique products with their information for AI categorization
    let unique_products: Vec<ProductInfo> = {
        let mut seen = HashSet::new();
        products
            .iter()
            .filter(|p| !p.raw_category.is_empty())
            .filter_map(|p| {
                if seen.insert(p.raw_category.clone()) {
                    Some(ProductInfo {
                        product_name: p.product_name.clone(),
                        description: p.description.clone(),
                        raw_category: p.raw_category.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    };

    if unique_products.is_empty() {
        return Err("No products with categories to harmonize".to_string());
    }

    // Get API base URL from state (if provided), otherwise from config
    let api_base_url = state.api_base_url.lock().unwrap().clone()
        .or_else(|| config.openai.base_url.clone());

    println!("Harmonizing {} unique products using model: {}", unique_products.len(), config.openai.model);
    println!("API base URL: {}", api_base_url.as_deref().unwrap_or("https://api.openai.com/v1"));

    // Call OpenAI to map categories
    let openai_client = OpenAIClient::new(
        &config.openai.api_key,
        &config.openai.model,
        api_base_url.as_deref(),
    );

    let (mappings, token_usage) = openai_client
        .map_categories(&unique_products, &taxonomy)
        .await
        .map_err(|e| format!("OpenAI mapping failed: {}", e))?;

    *state.mappings.lock().unwrap() = mappings.clone();

    // Calculate estimated cost (GPT-4o-mini pricing: $0.15 per 1M input tokens, $0.60 per 1M output tokens)
    let estimated_cost = (token_usage.input_tokens as f64 / 1_000_000.0 * 0.15) 
        + (token_usage.output_tokens as f64 / 1_000_000.0 * 0.60);

    // Store token usage in state
    *state.token_usage.lock().unwrap() = TokenUsage {
        input_tokens: token_usage.input_tokens,
        output_tokens: token_usage.output_tokens,
        total_tokens: token_usage.total_tokens,
        estimated_cost,
    };

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
                description: p.description.clone(),
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
fn export_to_excel(
    products: Vec<HarmonizedProduct>, 
    output_path: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    use rust_xlsxwriter::*;

    // Ensure the output path has .xlsx extension
    let output_path = if output_path.ends_with(".csv") {
        output_path.replace(".csv", ".xlsx")
    } else if !output_path.ends_with(".xlsx") {
        format!("{}.xlsx", output_path)
    } else {
        output_path
    };

    let mut workbook = Workbook::new();

    // Sheet 1: Harmonized Products
    let sheet1 = workbook.add_worksheet().set_name("Harmonized Products")
        .map_err(|e| format!("Failed to create worksheet: {}", e))?;

    // Write headers for Sheet 1
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::Blue)
        .set_font_color(Color::White);

    let headers = vec![
        "ID", "Product ID", "Supplier ID", "Product Name", 
        "Raw Category", "Main Category", "Sub Category", 
        "Sub-Sub Category", "Confidence"
    ];

    for (col, header) in headers.iter().enumerate() {
        sheet1.write_string_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| format!("Failed to write header: {}", e))?;
    }

    // Write product data
    for (row, product) in products.iter().enumerate() {
        let row_num = (row + 1) as u32;
        sheet1.write_string(row_num, 0, &product.id)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 1, &product.product_id)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 2, &product.supplier_id)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 3, &product.product_name)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 4, &product.raw_category)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 5, &product.main_category)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 6, &product.sub_category)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 7, &product.sub_sub_category)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sheet1.write_string(row_num, 8, &product.confidence)
            .map_err(|e| format!("Failed to write data: {}", e))?;
    }

    // Auto-fit columns
    for col in 0..9 {
        sheet1.set_column_width(col, 15)
            .map_err(|e| format!("Failed to set column width: {}", e))?;
    }

    // Sheet 2: Token Usage Statistics
    let sheet2 = workbook.add_worksheet().set_name("Token Usage Statistics")
        .map_err(|e| format!("Failed to create worksheet: {}", e))?;

    let token_usage = state.token_usage.lock().unwrap().clone();

    // Write headers for Sheet 2
    sheet2.write_string_with_format(0, 0, "Metric", &header_format)
        .map_err(|e| format!("Failed to write header: {}", e))?;
    sheet2.write_string_with_format(0, 1, "Value", &header_format)
        .map_err(|e| format!("Failed to write header: {}", e))?;

    // Write token usage data
    sheet2.write_string(1, 0, "Input Tokens")
        .map_err(|e| format!("Failed to write data: {}", e))?;
    sheet2.write_number(1, 1, token_usage.input_tokens as f64)
        .map_err(|e| format!("Failed to write data: {}", e))?;

    sheet2.write_string(2, 0, "Output Tokens")
        .map_err(|e| format!("Failed to write data: {}", e))?;
    sheet2.write_number(2, 1, token_usage.output_tokens as f64)
        .map_err(|e| format!("Failed to write data: {}", e))?;

    sheet2.write_string(3, 0, "Total Tokens")
        .map_err(|e| format!("Failed to write data: {}", e))?;
    sheet2.write_number(3, 1, token_usage.total_tokens as f64)
        .map_err(|e| format!("Failed to write data: {}", e))?;

    sheet2.write_string(4, 0, "Estimated Cost (USD)")
        .map_err(|e| format!("Failed to write data: {}", e))?;
    
    let currency_format = Format::new().set_num_format("$0.0000");
    sheet2.write_number_with_format(4, 1, token_usage.estimated_cost, &currency_format)
        .map_err(|e| format!("Failed to write data: {}", e))?;

    // Set column widths for Sheet 2
    sheet2.set_column_width(0, 25)
        .map_err(|e| format!("Failed to set column width: {}", e))?;
    sheet2.set_column_width(1, 20)
        .map_err(|e| format!("Failed to set column width: {}", e))?;

    // Save the workbook
    workbook.save(&output_path)
        .map_err(|e| format!("Failed to save Excel file: {}", e))?;

    Ok(format!("Exported {} products with token usage statistics to {}", products.len(), output_path))
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
            token_usage: Mutex::new(TokenUsage::default()),
        })
        .invoke_handler(tauri::generate_handler![
            save_api_key,
            get_api_key,
            delete_api_key,
            load_config,
            load_taxonomy,
            load_taxonomy_content,
            write_file,
            parse_input_csv,
            fetch_products_from_db,
            harmonize_categories,
            export_to_excel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
