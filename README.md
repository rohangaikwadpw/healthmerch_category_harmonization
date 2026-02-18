# HealthMerch Category Harmonization Tool

A powerful desktop application built with Tauri, React, and Rust for automated product category mapping using AI. This tool helps standardize product categories from raw data to a predefined taxonomy using OpenAI or custom AI endpoints like LM Studio.

## 🎯 Features

- **AI-Powered Category Mapping**: Automatically map product categories to standardized taxonomy using OpenAI GPT or local AI models
- **Secure API Key Management**: Local credential storage that persists across app restarts for seamless workflow
- **Multiple AI Provider Support**:
  - OpenAI API (GPT-4, GPT-3.5, etc.)
  - Local AI servers (LM Studio, LocalAI, and other OpenAI-compatible APIs)
- **Database Integration**: Fetch product data directly from PostgreSQL database
- **Batch Processing**: Process multiple products efficiently with automatic batching
- **Confidence Scoring**: Get confidence levels (high/medium/low) for each category mapping
- **CSV Import/Export**: Load products from CSV and export harmonized results
- **Real-time Progress Tracking**: Visual workflow with step-by-step progress indicators
- **Beautiful UI**: Modern, responsive interface with dark mode support

## 🏗️ Architecture

- **Frontend**: React + TypeScript + Vite
- **Backend**: Rust (Tauri)
- **Database**: PostgreSQL
- **AI Integration**: OpenAI API / Custom endpoints

## 📋 Prerequisites

Before you begin, ensure you have the following installed:

- **Node.js** (v18 or higher) - [Download](https://nodejs.org/)
- **Rust** (latest stable) - [Install Rust](https://www.rust-lang.org/tools/install)
- **Yarn** package manager - `npm install -g yarn`
- **PostgreSQL** database with product data
- **OpenAI API Key** or **Local AI Server** (e.g., LM Studio)

## 🚀 Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/rohangaikwadpw/healthmerch_category_harmonization.git
   cd healthmerch_category_harmonization
   ```

2. **Install dependencies**
   ```bash
   yarn install
   ```

3. **Configure database connection**
   
   Create/edit `config.json` in the root directory:
   ```json
   {
     "postgres": {
       "connection_url": "postgres://username:password@localhost:5432/your_database"
     },
     "openai": {
       "api_key": "your-api-key-here",
       "model": "gpt-4"
     }
   }
   ```
   
   **Note**: The API key in config.json is optional. You can enter it directly in the app on startup.

4. **Run the application**
   ```bash
   yarn tauri dev
   ```

## 📖 Usage Guide

### Step 1: Configure AI Provider

When you launch the application, you'll see the API configuration screen with two options:

#### Option A: OpenAI API
1. Select **"OpenAI API Key"** tab
2. Enter your OpenAI API key (starts with `sk-`)
3. Click **"Continue"**

#### Option B: Custom Endpoint (LM Studio, LocalAI, etc.)
1. Select **"Custom Endpoint"** tab
2. Enter your endpoint URL (e.g., `http://localhost:1234/v1`)
3. Optionally enter an API key if required by your endpoint
4. Click **"Continue"**

**🔒 Security Note**: Your API credentials are stored locally in a `.env` file and will persist across app restarts. The credentials are never sent anywhere except to your configured AI endpoint. To remove credentials, simply delete the `.env` file from the application directory.

### Step 2: Load Taxonomy

1. Click **"Load Taxonomy CSV"**
2. Select your taxonomy file containing:
   - Main Category
   - Sub-Category
   - Sub-Sub-Category
3. The app will show the number of taxonomy entries loaded

### Step 3: Load Products CSV

1. Click **"Load Products CSV"**
2. Select a CSV file with columns:
   - `product_id`
   - `supplier_id`
3. Preview the loaded products

### Step 4: Fetch from Database

1. Click **"Fetch from Database"**
2. The app will query PostgreSQL to fetch full product details:
   - Product name
   - Category arrays
   - Raw categories
3. Review the fetched products preview

### Step 5: Harmonize Categories

1. Click **"Harmonize Categories"**
2. The AI will process categories in batches
3. View confidence scores:
   - 🟢 **High**: Strong match
   - 🟡 **Medium**: Probable match
   - 🔴 **Low**: Uncertain match

### Step 6: Export Results

1. Click **"Export CSV"**
2. Choose save location
3. Get a CSV with all harmonized data:
   - Original product information
   - Mapped main/sub/sub-sub categories
   - Confidence scores

## 🗄️ Database Schema

Your PostgreSQL database should have a table with products containing:

```sql
CREATE TABLE products (
    id UUID PRIMARY KEY,
    product_id TEXT,
    supplier_id TEXT,
    product_name TEXT,
    category_array TEXT[] OR JSONB,
    -- other fields...
);
```

## 📁 Project Structure

```
healthmerch_category_harmonization/
├── src/                      # React frontend
│   ├── App.tsx              # Main application component
│   ├── App.css              # Styling
│   └── main.tsx             # Entry point
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── main.rs          # Tauri entry point
│   │   ├── lib.rs           # Main application logic
│   │   ├── config.rs        # Configuration management
│   │   ├── db.rs            # Database operations
│   │   └── openai.rs        # AI integration
│   ├── Cargo.toml           # Rust dependencies
│   └── tauri.conf.json      # Tauri configuration
├── config.json              # Application configuration
├── package.json             # Node dependencies
└── README.md                # This file
```

## 🛠️ Development

### Prerequisites for Development
- [VS Code](https://code.visualstudio.com/) (recommended)
- [Tauri VS Code Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### Development Commands

```bash
# Start development server
yarn tauri dev

# Build for production
yarn tauri build

# Run frontend only
yarn dev

# Lint code
yarn lint
```

## 🔧 Configuration Options

### AI Models Supported

**OpenAI**:
- `gpt-4`
- `gpt-4-turbo`
- `gpt-3.5-turbo`

**Local Models** (via LM Studio or similar):
- Any OpenAI-compatible endpoint
- Default endpoint: `http://localhost:1234/v1`

### Environment Variables

The application uses a temporary `.env` file during runtime:
```env
OPENAI_API_KEY=your-key-here
OPENAI_API_BASE=http://localhost:1234/v1  # Optional, for custom endpoints
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📝 License

This project is licensed under the MIT License.

## 🐛 Troubleshooting

### Common Issues

**1. "Failed to connect to AI endpoint"**
- Ensure LM Studio server is running if using local endpoint
- Check that the endpoint URL is correct (should end with `/v1`)
- Verify your firewall isn't blocking the connection

**2. "Database connection failed"**
- Verify `config.json` has correct PostgreSQL connection string
- Ensure database is running and accessible
- Check username/password credentials

**3. "OpenAI API error: 401"**
- API key is invalid or expired
- Check that API key starts with `sk-`
- Verify you have sufficient API credits

**4. Application won't start**
- Run `yarn install` to ensure all dependencies are installed
- Clear node_modules and reinstall: `rm -rf node_modules && yarn install`
- Rebuild Rust components: `cd src-tauri && cargo clean && cargo build`

## 📞 Support

For issues and questions, please open an issue on the [GitHub repository](https://github.com/rohangaikwadpw/healthmerch_category_harmonization/issues).

---

**Built with ❤️ using Tauri, React, and Rust**