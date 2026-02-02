import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import "./App.css";

interface ProductInput {
  product_id: string;
  supplier_id: string;
}

interface ProductData {
  id: string;
  product_id: string;
  supplier_id: string;
  product_name: string;
  category_array: string[];
  raw_category: string;
}

interface HarmonizedProduct {
  id: string;
  product_id: string;
  supplier_id: string;
  product_name: string;
  raw_category: string;
  main_category: string;
  sub_category: string;
  sub_sub_category: string;
  confidence: string;
}

type Stage = "init" | "taxonomy_loaded" | "csv_loaded" | "fetched" | "harmonized" | "error";

function App() {
  const [stage, setStage] = useState<Stage>("init");
  const [configReady, setConfigReady] = useState(false);
  const [taxonomyLoaded, setTaxonomyLoaded] = useState(false);
  const [taxonomyCount, setTaxonomyCount] = useState(0);
  const [productInputs, setProductInputs] = useState<ProductInput[]>([]);
  const [products, setProducts] = useState<ProductData[]>([]);
  const [harmonizedProducts, setHarmonizedProducts] = useState<HarmonizedProduct[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadingMessage, setLoadingMessage] = useState("");

  useEffect(() => {
    initializeApp();
  }, []);

  async function initializeApp() {
    try {
      setLoadingMessage("Loading configuration...");
      const isReady = await invoke<boolean>("load_config");
      setConfigReady(isReady);

      if (!isReady) {
        setError("Please update config.json with your Postgres and OpenAI credentials, then restart the app.");
        setStage("error");
        return;
      }

      setStage("init");
      setError(null);
    } catch (e) {
      setError(String(e));
      setStage("error");
    }
  }

  async function handleLoadTaxonomy() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "CSV", extensions: ["csv"] }],
        title: "Select Taxonomy CSV",
      });

      if (!selected) return;

      setLoading(true);
      setLoadingMessage("Loading taxonomy...");

      const content = await readTextFile(selected);
      const count = await invoke<number>("load_taxonomy_content", {
        content: content,
      });

      setTaxonomyLoaded(true);
      setTaxonomyCount(count);
      setStage("taxonomy_loaded");
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleLoadCSV() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "CSV", extensions: ["csv"] }],
        title: "Select Products CSV",
      });

      if (!selected) return;

      setLoading(true);
      setLoadingMessage("Parsing CSV...");

      const content = await readTextFile(selected);
      const inputs = await invoke<ProductInput[]>("parse_input_csv", {
        csvContent: content,
      });

      setProductInputs(inputs);
      setStage("csv_loaded");
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleFetchProducts() {
    try {
      setLoading(true);
      setLoadingMessage(`Fetching ${productInputs.length} products from database...`);

      const fetched = await invoke<ProductData[]>("fetch_products_from_db", {
        inputs: productInputs,
      });

      setProducts(fetched);
      setStage("fetched");
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleHarmonize() {
    try {
      setLoading(true);
      const uniqueCategories = new Set(products.map((p) => p.raw_category).filter(Boolean));
      setLoadingMessage(`Harmonizing ${uniqueCategories.size} unique categories via OpenAI...`);

      const harmonized = await invoke<HarmonizedProduct[]>("harmonize_categories");

      setHarmonizedProducts(harmonized);
      setStage("harmonized");
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleExport() {
    try {
      const savePath = await save({
        filters: [{ name: "CSV", extensions: ["csv"] }],
        defaultPath: "harmonized_products.csv",
      });

      if (!savePath) return;

      setLoading(true);
      setLoadingMessage("Exporting CSV...");

      const result = await invoke<string>("export_to_csv", {
        products: harmonizedProducts,
        outputPath: savePath,
      });

      alert(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  function getConfidenceColor(confidence: string): string {
    switch (confidence) {
      case "high":
        return "#22c55e";
      case "medium":
        return "#eab308";
      case "low":
        return "#ef4444";
      default:
        return "#6b7280";
    }
  }

  return (
    <div className="container">
      <h1>Category Harmonization Tool</h1>

      {/* Status Bar */}
      <div className="status-bar">
        <div className={`status-item ${configReady ? "active" : ""}`}>
          <span className="status-dot"></span>
          Config {configReady ? "Ready" : "Not Ready"}
        </div>
        <div className={`status-item ${taxonomyLoaded ? "active" : ""}`}>
          <span className="status-dot"></span>
          Taxonomy {taxonomyLoaded ? `(${taxonomyCount} entries)` : "Not Loaded"}
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="error-box">
          <strong>Error:</strong> {error}
        </div>
      )}

      {/* Loading Indicator */}
      {loading && (
        <div className="loading-box">
          <div className="spinner"></div>
          <span>{loadingMessage}</span>
        </div>
      )}

      {/* Main Actions */}
      {stage !== "error" && (
        <div className="actions">
          <button onClick={handleLoadTaxonomy} disabled={loading}>
            1. Load Taxonomy CSV
          </button>
          <button
            onClick={handleLoadCSV}
            disabled={loading || !taxonomyLoaded}
          >
            2. Load Products CSV
          </button>
          <button
            onClick={handleFetchProducts}
            disabled={loading || stage !== "csv_loaded"}
          >
            3. Fetch from Database ({productInputs.length} products)
          </button>
          <button
            onClick={handleHarmonize}
            disabled={loading || stage !== "fetched"}
          >
            4. Harmonize Categories
          </button>
          <button
            onClick={handleExport}
            disabled={loading || stage !== "harmonized"}
          >
            5. Export CSV
          </button>
        </div>
      )}

      {/* Products Preview */}
      {stage === "csv_loaded" && productInputs.length > 0 && (
        <div className="preview">
          <h3>CSV Preview ({productInputs.length} products)</h3>
          <table>
            <thead>
              <tr>
                <th>Product ID</th>
                <th>Supplier ID</th>
              </tr>
            </thead>
            <tbody>
              {productInputs.slice(0, 10).map((p, i) => (
                <tr key={i}>
                  <td>{p.product_id}</td>
                  <td>{p.supplier_id}</td>
                </tr>
              ))}
              {productInputs.length > 10 && (
                <tr>
                  <td colSpan={2} className="more">
                    ... and {productInputs.length - 10} more
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {/* Fetched Products */}
      {stage === "fetched" && products.length > 0 && (
        <div className="preview">
          <h3>Fetched Products ({products.length} found)</h3>
          <table>
            <thead>
              <tr>
                <th>Product ID</th>
                <th>Name</th>
                <th>Raw Category</th>
              </tr>
            </thead>
            <tbody>
              {products.slice(0, 10).map((p, i) => (
                <tr key={i}>
                  <td>{p.product_id}</td>
                  <td>{p.product_name}</td>
                  <td>{p.raw_category}</td>
                </tr>
              ))}
              {products.length > 10 && (
                <tr>
                  <td colSpan={3} className="more">
                    ... and {products.length - 10} more
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {/* Harmonized Results */}
      {stage === "harmonized" && harmonizedProducts.length > 0 && (
        <div className="preview results">
          <h3>Harmonized Results ({harmonizedProducts.length} products)</h3>
          <table>
            <thead>
              <tr>
                <th>Product Name</th>
                <th>Raw Category</th>
                <th>→</th>
                <th>Main</th>
                <th>Sub</th>
                <th>Sub-Sub</th>
                <th>Conf</th>
              </tr>
            </thead>
            <tbody>
              {harmonizedProducts.slice(0, 20).map((p, i) => (
                <tr key={i}>
                  <td>{p.product_name}</td>
                  <td className="raw-cat">{p.raw_category}</td>
                  <td>→</td>
                  <td>{p.main_category}</td>
                  <td>{p.sub_category}</td>
                  <td>{p.sub_sub_category}</td>
                  <td>
                    <span
                      className="confidence"
                      style={{ backgroundColor: getConfidenceColor(p.confidence) }}
                    >
                      {p.confidence}
                    </span>
                  </td>
                </tr>
              ))}
              {harmonizedProducts.length > 20 && (
                <tr>
                  <td colSpan={7} className="more">
                    ... and {harmonizedProducts.length - 20} more
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export default App;
