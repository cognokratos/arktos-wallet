# Customization Guide: Arktos Wallet

Arktos Wallet is designed as a **customizable blueprint** for system owners and builders who need to adapt the architecture for specific needs. This guide provides patterns, examples, and best practices for extending and customizing Arktos.

## Overview

The Arktos architecture is modular and extensible. Key areas for customization include:

1. **Blockchain Support** - Add new blockchains (Solana, Polkadot, etc.)
2. **Authentication** - Integrate custom identity providers (OAuth2, JWT, mTLS)
3. **Storage Backend** - Replace SQLite with PostgreSQL, MongoDB, etc.
4. **API Design** - Adapt MCP tools or expose additional HTTP endpoints
5. **Audit & Observability** - Integrate with external logging and monitoring systems
6. **Performance Tuning** - Optimize for your specific deployment environment

## Architecture Foundation

Before customizing, understand the core architecture:

- **Monolithic Service**: Single Rust binary (Axum-based HTTP server)
- **Stateless Design**: No in-memory state between requests (facilitates horizontal scaling)
- **MCP-Centric API**: Core wallet logic exposed via Model Context Protocol (MCP) tools
- **Encrypted Persistence**: All sensitive data encrypted at rest using SQLCipher
- **Modular Code Organization**: Rust modules allow clean separation of concerns

See [Architecture Document](./architecture.md) for details.

---

## 1. Adding Blockchain Support

### Current Implementation

Arktos currently supports:
- **Bitcoin** via `rust-bitcoin` (BIP32/BIP39)
- **Ethereum** via `secp256k1` + `tiny-keccak` (HD wallet derivation)

Both use standard HD wallet derivation paths:
- Bitcoin: `m/44'/0'/0'/0/{account_index}`
- Ethereum: `m/44'/60'/0'/0/{account_index}`

### Pattern: Adding a New Blockchain

To add support for a new blockchain (e.g., Solana):

#### Step 1: Define Derivation Path

Choose a standard derivation path for your blockchain. Example for Solana:
```rust
// Path: m/44'/501'/0'/0/{account_index}
const SOLANA_DERIVATION_PATH: &str = "m/44'/501'/0'/0";
```

#### Step 2: Create Blockchain Module

Create a new module in `src/`:

```rust
// src/solana.rs
use bip32::{Seed, XPrivateKey};
use bip39::Mnemonic;

pub struct SolanaAccount {
    pub address: String,
    pub public_key: Vec<u8>,
}

pub fn derive_solana_address(
    mnemonic: &Mnemonic,
    account_index: u32,
) -> Result<SolanaAccount, Error> {
    // 1. Convert mnemonic to seed
    let seed = Seed::new(mnemonic, "");
    
    // 2. Derive private key using BIP32 path
    let derivation_path = format!("m/44'/501'/0'/0/{}", account_index);
    let child_key = XPrivateKey::new(seed).derive_private_key(&derivation_path)?;
    
    // 3. Convert to Solana keypair (uses Ed25519)
    // Use solana-sdk for conversion
    let solana_keypair = convert_to_solana_keypair(&child_key)?;
    
    // 4. Return address
    Ok(SolanaAccount {
        address: solana_keypair.pubkey().to_string(),
        public_key: solana_keypair.pubkey().as_ref().to_vec(),
    })
}
```

#### Step 3: Integrate into Wallet Service

Add the new blockchain to your wallet management module:

```rust
// In src/main.rs or src/wallet_manager.rs
pub async fn get_solana_address(
    wallet_id: String,
    account_index: u32,
) -> Result<GetAddressResponse, Error> {
    // 1. Retrieve encrypted mnemonic from database
    let wallet = db::get_wallet(&wallet_id)?;
    let mnemonic = wallet.decrypt_mnemonic()?;
    
    // 2. Derive address
    let solana_account = solana::derive_solana_address(&mnemonic, account_index)?;
    
    // 3. Return response
    Ok(GetAddressResponse {
        wallet_id,
        blockchain: "solana".to_string(),
        address: solana_account.address,
    })
}
```

#### Step 4: Expose as MCP Tool

Register the new function as an MCP tool in your server setup:

```rust
// In your MCP tools registration
tools.push(create_mcp_tool(
    "get_solana_address",
    "Retrieve a Solana address for a wallet",
    vec![
        MCP::Parameter {
            name: "wallet_id".to_string(),
            description: "The wallet ID".to_string(),
            type_: "string".to_string(),
        },
        MCP::Parameter {
            name: "account_index".to_string(),
            description: "The account index (0-based)".to_string(),
            type_: "number".to_string(),
        },
    ],
));
```

#### Step 5: Add Dependencies

Update `Cargo.toml`:

```toml
[dependencies]
solana-sdk = "1.x"      # For Solana keypair handling
ed25519-dalek = "1.x"   # For Ed25519 signature support
```

#### Step 6: Add Tests

Create unit tests for the new blockchain module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_derivation() {
        let mnemonic = Mnemonic::parse("test words...").unwrap();
        let account = derive_solana_address(&mnemonic, 0).unwrap();
        
        assert!(!account.address.is_empty());
        assert_eq!(account.public_key.len(), 32); // Ed25519 public key size
    }
}
```

### Reference: Bitcoin & Ethereum Implementation

See `src/main.rs` for current Bitcoin and Ethereum implementations as reference examples.

---

## 2. Custom Authentication

### Current Implementation

Arktos uses **API key-based authentication**:
- API keys stored in encrypted database
- Keys validated on every MCP request
- Stateless authentication (no sessions)

### Pattern: Integrate OAuth2

To replace API key auth with OAuth2 (e.g., for user-facing systems):

#### Step 1: Create Authentication Module

```rust
// src/auth/oauth2.rs
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct OAuthClaims {
    pub sub: String,      // Subject (user ID)
    pub aud: Vec<String>, // Audience
    pub exp: i64,         // Expiration
    pub iat: i64,         // Issued at
    pub scopes: Vec<String>,
}

pub struct OAuth2Provider {
    jwks_url: String,
    audience: String,
}

impl OAuth2Provider {
    pub async fn validate_token(&self, token: &str) -> Result<OAuthClaims, Error> {
        // Fetch JWKS from provider
        let jwks = fetch_jwks(&self.jwks_url).await?;
        
        // Decode and validate JWT
        let decoding_key = DecodingKey::from_jwks(&jwks)?;
        let token_data = decode::<OAuthClaims>(
            token,
            &decoding_key,
            &Validation::new(jsonwebtoken::Algorithm::RS256),
        )?;
        
        Ok(token_data.claims)
    }
}
```

#### Step 2: Create Middleware

```rust
// src/middleware/oauth2_middleware.rs
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

pub async fn oauth2_middleware(
    bearer_token: BearerToken,
    mut request: Request,
    next: Next,
) -> Result<Response, Error> {
    let claims = oauth2_provider.validate_token(&bearer_token.0).await?;
    
    // Store claims in request extensions
    request.extensions_mut().insert(claims);
    
    Ok(next.run(request).await)
}
```

#### Step 3: Update Route Handler

```rust
// In your MCP endpoint handler
pub async fn handle_mcp_request(
    AuthExtension(claims): AuthExtension<OAuthClaims>,
    payload: MCP Request,
) -> Result<MCP Response, Error> {
    // Use claims.sub as owner_id or organization_id
    let owner_id = claims.sub;
    
    // Process MCP request with owner context
    // ...
}
```

#### Step 4: Add Dependencies

```toml
[dependencies]
jsonwebtoken = "9.x"
reqwest = { version = "0.11", features = ["json"] }
```

### Other Authentication Patterns

- **mTLS**: Use certificate-based client authentication
  - Configure TLS in Axum with client certificate validation
  - Derive identity from certificate subject DN
  
- **JWT with RS256**: Similar to OAuth2 pattern above
  - Use `jsonwebtoken` crate for validation
  - Validate signature against public key

- **API Key + Database**: Current implementation
  - See API Key Authentication section in [Architecture](./architecture.md)

---

## 3. Storage Backend Customization

### Current Implementation

Arktos uses **SQLite + SQLCipher** for encrypted local persistence.

### Pattern: Migrate to PostgreSQL

To use PostgreSQL instead of SQLite:

#### Step 1: Choose a Database Library

```toml
[dependencies]
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }
tokio = { version = "1", features = ["full"] }
```

#### Step 2: Create Database Abstraction Layer

```rust
// src/db/mod.rs
use sqlx::PgPool;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(connection_string: &str) -> Result<Self, Error> {
        let pool = PgPool::connect(connection_string).await?;
        Ok(Database { pool })
    }
    
    pub async fn get_wallet(&self, wallet_id: &str) -> Result<Wallet, Error> {
        let wallet = sqlx::query_as::<_, Wallet>(
            "SELECT * FROM wallets WHERE id = $1"
        )
        .bind(wallet_id)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(wallet)
    }
}
```

#### Step 3: Define Migration Strategy

Use Flyway or `sqlx migrate` for schema management:

```bash
# Create migrations
sqlx migrate add -r create_wallets_table

# Run migrations
sqlx migrate run
```

#### Step 4: Update Application Startup

```rust
// In main()
let db = Database::new(&env::var("DATABASE_URL")?).await?;

// Or use connection pooling
let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&database_url)
    .await?;
```

### Benefits of PostgreSQL

- **Multi-instance**: Shared database for horizontal scaling
- **ACID Transactions**: Built-in transaction support
- **Advanced Features**: JSON columns, full-text search, spatial data
- **Production Ready**: Mature, battle-tested for production workloads

### Other Storage Options

- **MongoDB**: Document-oriented, flexible schema
  - Use `mongodb` crate
  - Trade-offs: Eventual consistency, larger disk footprint
  
- **DynamoDB**: AWS managed service
  - Use `rusoto_dynamodb` or `aws-sdk-dynamodb`
  - Trade-offs: Vendor lock-in, limited query flexibility

---

## 4. API Customization

### Extending MCP Tools

Add new MCP tools for custom wallet operations:

```rust
// Example: Custom fee estimation tool
pub async fn estimate_transaction_fee(
    blockchain: String,
    amount: u64,
) -> Result<EstimateFeeResponse, Error> {
    match blockchain.as_str() {
        "bitcoin" => {
            let fee = bitcoin::estimate_fee(amount).await?;
            Ok(EstimateFeeResponse { 
                blockchain, 
                estimated_fee: fee,
                unit: "satoshis".to_string(),
            })
        }
        "ethereum" => {
            let fee = ethereum::estimate_gas_price().await?;
            Ok(EstimateFeeResponse {
                blockchain,
                estimated_fee: fee,
                unit: "wei".to_string(),
            })
        }
        _ => Err(Error::UnsupportedBlockchain),
    }
}
```

### Adding HTTP Endpoints

Extend the HTTP API for non-MCP use cases:

```rust
// Example: Public price feed endpoint
pub async fn get_crypto_prices(
    Query(params): Query<PriceQuery>,
) -> Result<Json<PriceResponse>, Error> {
    let prices = external_api::fetch_prices(&params.symbols).await?;
    Ok(Json(PriceResponse { prices }))
}

// Register route
let app = Router::new()
    .route("/api/prices", get(get_crypto_prices))
    .route("/mcp", post(handle_mcp_request));
```

---

## 5. Observability & Monitoring

### Integrate External Logging

Replace or augment console logging with external services:

```rust
// src/observability/mod.rs
use tracing_subscriber::layer::SubscriberExt;
use tracing_stackdriver::layer as stackdriver_layer;

pub fn init_logging() {
    let stackdriver = stackdriver_layer();
    let subscriber = tracing_subscriber::registry()
        .with(stackdriver)
        .with(tracing_subscriber::fmt::layer());
    
    tracing::subscriber::set_default(subscriber);
}
```

### Add Metrics

Use Prometheus-compatible metrics:

```toml
[dependencies]
prometheus = "0.13"
```

```rust
// src/metrics/mod.rs
lazy_static::lazy_static! {
    pub static ref WALLET_CREATION_COUNT: Counter = 
        Counter::new("wallet_creation_total", "Total wallets created").unwrap();
    pub static ref ADDRESS_RETRIEVAL_LATENCY: Histogram =
        Histogram::new("address_retrieval_seconds", "Address retrieval latency").unwrap();
}
```

---

## 6. Testing Your Customizations

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_feature() {
        // Your test here
    }
}
```

### Integration Testing

```rust
// tests/custom_integration_test.rs
#[tokio::test]
async fn test_end_to_end() {
    let client = TestClient::new().await;
    
    // Create wallet
    let wallet = client.create_wallet().await.unwrap();
    
    // Test custom feature
    let result = client.custom_operation(&wallet.id).await.unwrap();
    assert!(result.is_valid());
}
```

### Regression Testing

Always run the full test suite after customization:

```bash
cargo test --all
```

---

## 7. Deployment Considerations

### Environment Variables

Extend the configuration for customizations:

```rust
pub struct Config {
    pub database_url: String,
    pub auth_provider: String,  // "api_key", "oauth2", "mtls"
    pub auth_config: String,     // Provider-specific config (JSON)
    pub log_backend: String,     // "stdout", "stackdriver", "elasticsearch"
}

impl Config {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")?,
            auth_provider: env::var("AUTH_PROVIDER").unwrap_or_else(|_| "api_key".to_string()),
            auth_config: env::var("AUTH_CONFIG")?,
            log_backend: env::var("LOG_BACKEND").unwrap_or_else(|_| "stdout".to_string()),
        })
    }
}
```

### Docker Customization

Update the `Dockerfile` for custom dependencies:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /workspace
COPY . .
RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/target/release/arktos-wallet /app/
ENTRYPOINT ["/app/arktos-wallet"]
```

---

## Best Practices for Customization

1. **Maintain API Compatibility**: Keep the MCP tool contracts stable for client compatibility
2. **Version Your Changes**: Use semantic versioning for custom extensions
3. **Document Customizations**: Add inline comments explaining custom logic
4. **Test Thoroughly**: Include tests for all customizations
5. **Performance Monitoring**: Measure impact of customizations on latency and throughput
6. **Security Review**: Ensure customizations don't introduce security vulnerabilities
7. **Backwards Compatibility**: Consider upgrade paths for existing deployments

---

## Common Pitfalls

| Pitfall | How to Avoid |
|---------|-------------|
| Breaking API contract | Use feature flags, version tools separately |
| Database schema drift | Use migration tools, version control schema changes |
| Performance degradation | Benchmark before and after customization |
| Security vulnerabilities | Follow OWASP guidelines, conduct security reviews |
| Difficult to maintain | Document decisions, keep code modular |

---

## Support & Examples

For more details on specific customizations:
- See [Architecture Document](./architecture.md) for design patterns
- Check source code in `src/` for reference implementations
- Review [Regional Compliance Guide](./regional-compliance.md) for compliance-specific customizations

For questions on customizations, refer to the project's issue tracker or documentation.
