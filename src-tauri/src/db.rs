use anyhow::Result;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInput {
    pub product_id: String,
    pub supplier_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductData {
    pub id: String, // UUID as string for JSON serialization
    pub product_id: String,
    pub supplier_id: String,
    pub product_name: String,
    pub category_array: Vec<String>,
    pub raw_category: String, // Joined category for display/grouping
}

pub async fn connect(connection_url: &str) -> Result<Client> {
    // Validate connection URL
    if connection_url.is_empty() 
        || connection_url.contains("YOUR_POSTGRES_URL_HERE")
        || connection_url.contains("username:password") {
        return Err(anyhow::anyhow!("invalid configuration"));
    }

    // Create TLS connector for AWS RDS
    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let connector = MakeTlsConnector::new(tls_connector);

    let (client, connection) = tokio_postgres::connect(connection_url, connector).await?;

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    Ok(client)
}

pub async fn fetch_products(
    client: &Client,
    product_inputs: &[ProductInput],
) -> Result<Vec<ProductData>> {
    let mut products = Vec::new();

    for input in product_inputs {
        // Parse supplier_id as UUID
        let supplier_uuid = Uuid::parse_str(&input.supplier_id)
            .map_err(|e| anyhow::anyhow!("Invalid supplier_id UUID '{}': {}", input.supplier_id, e))?;

        let rows = client
            .query(
                r#"
                SELECT
                    id,
                    product_id,
                    supplier_id,
                    product_data->>'productName' as product_name,
                    product_data->'ProductCategoryArray' as category_array
                FROM products_ps_jsondump
                WHERE product_id = $1 AND supplier_id = $2
                "#,
                &[&input.product_id, &supplier_uuid],
            )
            .await?;

        for row in rows {
            let id: Uuid = row.get("id");
            let product_id: String = row.get("product_id");
            let supplier_id: Uuid = row.get("supplier_id");
            let product_name: Option<String> = row.get("product_name");
            let category_json: Option<serde_json::Value> = row.get("category_array");

            // Parse category array from JSON - it's an array of {"category": "value"} objects
            let category_array: Vec<String> = category_json
                .and_then(|v| {
                    if let serde_json::Value::Array(arr) = v {
                        Some(
                            arr.iter()
                                .filter_map(|item| {
                                    item.get("category")
                                        .and_then(|c| c.as_str())
                                        .map(|s| s.to_string())
                                })
                                .collect()
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let raw_category = category_array.join(" > ");

            products.push(ProductData {
                id: id.to_string(),
                product_id,
                supplier_id: supplier_id.to_string(),
                product_name: product_name.unwrap_or_default(),
                category_array,
                raw_category,
            });
        }
    }

    Ok(products)
}
