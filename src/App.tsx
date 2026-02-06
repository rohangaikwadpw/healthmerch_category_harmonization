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

interface ApiConfig {
  api_key: string | null;
  api_base_url: string | null;
}

function App() {
  const [hasApiKey, setHasApiKey] = useState(false);
  const [useCustomUrl, setUseCustomUrl] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [customUrlInput, setCustomUrlInput] = useState("");
  const [apiKeyError, setApiKeyError] = useState<string | null>(null);
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
    checkApiKey();
  }, []);

  async function checkApiKey() {
    try {
      const existingConfig = await invoke<ApiConfig | null>("get_api_key");
      if (existingConfig && existingConfig.api_key) {
        setHasApiKey(true);
        await initializeApp();
      }
    } catch (e) {
      console.error("Failed to check API key:", e);
    }
  }

  async function handleApiKeySubmit(e: React.FormEvent) {
    e.preventDefault();
    
    // Validate based on mode
    if (useCustomUrl) {
      if (!customUrlInput.trim()) {
        setApiKeyError("Please enter a custom endpoint URL");
        return;
      }
      if (!customUrlInput.startsWith("http")) {
        setApiKeyError("Custom URL must start with http:// or https://");
        return;
      }
      // API key is optional for custom endpoints
      const key = apiKeyInput.trim() || "not-required";
      try {
        setApiKeyError(null);
        await invoke("save_api_key", { apiKey: key, apiBaseUrl: customUrlInput });
        setHasApiKey(true);
        setApiKeyInput("");
        setCustomUrlInput("");
        await initializeApp();
      } catch (e) {
        setApiKeyError(String(e));
      }
    } else {
      // OpenAI mode - API key is required
      if (!apiKeyInput.trim()) {
        setApiKeyError("Please enter an API key");
        return;
      }
      if (!apiKeyInput.startsWith("sk-")) {
        setApiKeyError("Invalid API key format. OpenAI API keys start with 'sk-'");
        return;
      }
      try {
        setApiKeyError(null);
        await invoke("save_api_key", { apiKey: apiKeyInput, apiBaseUrl: null });
        setHasApiKey(true);
        setApiKeyInput("");
        await initializeApp();
      } catch (e) {
        setApiKeyError(String(e));
      }
    }
  }

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
      {/* API Key Input Screen */}
      {!hasApiKey ? (
        <div className="api-key-screen">
          <div className="header">
            <div className="icon-badge">🔐</div>
            <h1>AI API Configuration</h1>
            <p className="subtitle">{useCustomUrl ? "Configure Custom AI Endpoint" : "Enter your OpenAI API key"}</p>
          </div>
          
          <form onSubmit={handleApiKeySubmit} className="api-key-form">
            <div className="toggle-container">
              <button
                type="button"
                className={`toggle-btn ${!useCustomUrl ? 'active' : ''}`}
                onClick={() => setUseCustomUrl(false)}
              >
                OpenAI API Key
              </button>
              <button
                type="button"
                className={`toggle-btn ${useCustomUrl ? 'active' : ''}`}
                onClick={() => setUseCustomUrl(true)}
              >
                Custom Endpoint (LM Studio, etc.)
              </button>
            </div>

            {useCustomUrl ? (
              <>
                <div className="input-group">
                  <label htmlFor="customUrl">Custom API Endpoint URL</label>
                  <input
                    id="customUrl"
                    type="text"
                    value={customUrlInput}
                    onChange={(e) => setCustomUrlInput(e.target.value)}
                    placeholder="http://localhost:1234/v1"
                    className="api-key-input"
                  />
                  <p className="input-hint">
                    Enter the base URL ending with /v1 (e.g., http://localhost:1234/v1 for LM Studio)
                  </p>
                </div>
                <div className="input-group">
                  <label htmlFor="apiKey">API Key (if required)</label>
                  <input
                    id="apiKey"
                    type="password"
                    value={apiKeyInput}
                    onChange={(e) => setApiKeyInput(e.target.value)}
                    placeholder="API key (or leave blank if not needed)"
                    className="api-key-input"
                    autoFocus
                  />
                  <p className="input-hint">
                    Some endpoints require an API key, others don't. Enter one if needed.
                  </p>
                </div>
              </>
            ) : (
              <div className="input-group">
                <label htmlFor="apiKey">OpenAI API Key</label>
                <input
                  id="apiKey"
                  type="password"
                  value={apiKeyInput}
                  onChange={(e) => setApiKeyInput(e.target.value)}
                  placeholder="sk-..."
                  className="api-key-input"
                  autoFocus
                />
                <p className="input-hint">
                  Your API key will be stored temporarily and deleted when you close the application.
                </p>
              </div>
            )}
            
            {apiKeyError && (
              <div className="error-box">
                {apiKeyError}
              </div>
            )}
            
            <button type="submit" className="submit-button">
              Continue →
            </button>
          </form>
          
          <div className="info-box">
            <h3>🔒 Privacy & Security</h3>
            <ul>
              <li>Your credentials are stored only in a local .env file</li>
              <li>All data is automatically deleted when you close the app</li>
              <li>You'll need to re-enter credentials each time you open the app</li>
              {!useCustomUrl && (
                <li>Get your API key at <a href="https://platform.openai.com/api-keys" target="_blank">platform.openai.com</a></li>
              )}
              {useCustomUrl && (
                <li>Compatible with LM Studio, LocalAI, and other OpenAI-compatible APIs</li>
              )}
              {useCustomUrl && (
                <li><strong>LM Studio users:</strong> Make sure the server is running on the specified port</li>
              )}
            </ul>
          </div>
        </div>
      ) : (
        <>
      <div className="header">
        <div className="icon-badge">🎯</div>
        <h1>Category Harmonization Tool</h1>
        <p className="subtitle">AI-Powered Product Category Mapping</p>
      </div>

      {/* Status Bar */}
      <div className="status-bar">
        <div className={`status-item ${configReady ? "active" : ""}`}>
          <span className="status-icon">{configReady ? "✅" : "⚙️"}</span>
          <div className="status-text">
            <div className="status-label">Configuration</div>
            <div className="status-value">{configReady ? "Ready" : "Not Ready"}</div>
          </div>
        </div>
        <div className={`status-item ${taxonomyLoaded ? "active" : ""}`}>
          <span className="status-icon">{taxonomyLoaded ? "📚" : "📋"}</span>
          <div className="status-text">
            <div className="status-label">Taxonomy</div>
            <div className="status-value">{taxonomyLoaded ? `${taxonomyCount} entries` : "Not Loaded"}</div>
          </div>
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
        <div className="workflow">
          <h2 className="workflow-title">Workflow Steps</h2>
          <div className="actions">
            <button className="step-button" onClick={handleLoadTaxonomy} disabled={loading}>
              <span className="step-number">1</span>
              <span className="step-icon">📚</span>
              <span className="step-text">Load Taxonomy CSV</span>
            </button>
            <button
              className="step-button"
              onClick={handleLoadCSV}
              disabled={loading || !taxonomyLoaded}
            >
              <span className="step-number">2</span>
              <span className="step-icon">📄</span>
              <span className="step-text">Load Products CSV</span>
            </button>
            <button
              className="step-button"
              onClick={handleFetchProducts}
              disabled={loading || stage !== "csv_loaded"}
            >
              <span className="step-number">3</span>
              <span className="step-icon">💾</span>
              <span className="step-text">Fetch from Database</span>
              {productInputs.length > 0 && (
                <span className="step-badge">{productInputs.length}</span>
              )}
            </button>
            <button
              className="step-button"
              onClick={handleHarmonize}
              disabled={loading || stage !== "fetched"}
            >
              <span className="step-number">4</span>
              <span className="step-icon">🤖</span>
              <span className="step-text">Harmonize Categories</span>
            </button>
            <button
              className="step-button"
              onClick={handleExport}
              disabled={loading || stage !== "harmonized"}
            >
              <span className="step-number">5</span>
              <span className="step-icon">📥</span>
              <span className="step-text">Export CSV</span>
            </button>
          </div>
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
        </>
      )}
    </div>
  );
}

export default App;
