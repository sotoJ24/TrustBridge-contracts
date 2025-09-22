use std::fmt;

/// Asset type enumeration for supported assets
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetType {
    Native,      // XLM
    Credit(CreditAsset), // USDC, TrustBridge Token, etc.
}

/// Credit asset information with code and issuer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditAsset {
    pub code: String,
    pub issuer: String,
}

/// Standard asset information structure
#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub asset_type: AssetType,
    pub contract_address: String,
    pub balance: Option<i128>,
    pub metadata: AssetMetadata,
}

/// Asset metadata including display information
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    pub description: Option<String>,
    pub image: Option<String>,
}

/// Error types for asset fetching operations
#[derive(Debug)]
pub enum AssetFetchError {
    NetworkError(String),
    InvalidAddress(String),
    AssetNotFound(String),
    ParseError(String),
    InvalidAssetType(String),
    RateLimitExceeded,
    Unauthorized,
}

impl fmt::Display for AssetFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetFetchError::NetworkError(msg) => write!(f, "Network request failed: {}", msg),
            AssetFetchError::InvalidAddress(msg) => write!(f, "Invalid asset address: {}", msg),
            AssetFetchError::AssetNotFound(msg) => write!(f, "Asset not found: {}", msg),
            AssetFetchError::ParseError(msg) => write!(f, "Parsing error: {}", msg),
            AssetFetchError::InvalidAssetType(msg) => write!(f, "Invalid asset type: {}", msg),
            AssetFetchError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            AssetFetchError::Unauthorized => write!(f, "Unauthorized access"),
        }
    }
}

impl std::error::Error for AssetFetchError {}

/// Asset fetcher client for retrieving asset information
pub struct AssetFetcher {
    pub horizon_url: String,
    pub network_passphrase: String,
}

/// Predefined asset addresses for TrustBridge ecosystem
pub mod asset_addresses {
    pub const XLM_ADDRESS: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
    pub const USDC_ADDRESS: &str = "CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU";
    pub const TBRG_TOKEN_ADDRESS: &str = "CAAUAE53WKWR4X2BRCHXNUTDJGXTOBMHMK3KFTAPEUBA7MJEQBPWVWQU";
    pub const ORACLE_CONTRACT_ADDRESS: &str = "CBCIZHUC42CKOZHKKEYMSXVVY24ZK2EKEUU6NFGQS5YFG7GAMEU5L32M";
}

impl AssetFetcher {
    /// Create a new asset fetcher with default Stellar mainnet configuration
    pub fn new() -> Self {
        Self {
            horizon_url: "https://horizon.stellar.org".to_string(),
            network_passphrase: "Public Global Stellar Network ; September 2015".to_string(),
        }
    }
    
    /// Create a new asset fetcher with custom configuration
    pub fn with_config(horizon_url: String, network_passphrase: String) -> Self {
        Self {
            horizon_url,
            network_passphrase,
        }
    }
    
    /// Fetch asset information for XLM (native Stellar asset)
    pub fn fetch_xlm_info(&self, account_address: Option<&str>) -> Result<AssetInfo, AssetFetchError> {
        let asset_info = AssetInfo {
            asset_type: AssetType::Native,
            contract_address: asset_addresses::XLM_ADDRESS.to_string(),
            balance: if let Some(addr) = account_address {
                Some(self.fetch_account_balance(addr, &AssetType::Native)?)
            } else {
                None
            },
            metadata: AssetMetadata {
                symbol: "XLM".to_string(),
                name: "Stellar Lumens".to_string(),
                decimals: 7,
                description: Some("Native cryptocurrency of the Stellar network".to_string()),
                image: None,
            },
        };
        
        Ok(asset_info)
    }
    
    /// Fetch asset information for USDC on Stellar
    pub fn fetch_usdc_info(&self, account_address: Option<&str>) -> Result<AssetInfo, AssetFetchError> {
        let credit_asset = CreditAsset {
            code: "USDC".to_string(),
            issuer: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(), // Circle's USDC issuer
        };
        
        let asset_info = AssetInfo {
            asset_type: AssetType::Credit(credit_asset),
            contract_address: asset_addresses::USDC_ADDRESS.to_string(),
            balance: if let Some(addr) = account_address {
                Some(self.fetch_account_balance(addr, &AssetType::Credit(CreditAsset {
                    code: "USDC".to_string(),
                    issuer: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
                }))?)
            } else {
                None
            },
            metadata: AssetMetadata {
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                description: Some("USDC stablecoin on Stellar network".to_string()),
                image: None,
            },
        };
        
        Ok(asset_info)
    }
    
    /// Fetch asset information for TrustBridge Token
    pub fn fetch_tbrg_info(&self, account_address: Option<&str>) -> Result<AssetInfo, AssetFetchError> {
        let credit_asset = CreditAsset {
            code: "TBRG".to_string(),
            issuer: asset_addresses::TBRG_TOKEN_ADDRESS.to_string(), // Using the contract address as issuer for now
        };
        
        let asset_info = AssetInfo {
            asset_type: AssetType::Credit(credit_asset),
            contract_address: asset_addresses::TBRG_TOKEN_ADDRESS.to_string(),
            balance: if let Some(addr) = account_address {
                Some(self.fetch_account_balance(addr, &AssetType::Credit(CreditAsset {
                    code: "TBRG".to_string(),
                    issuer: asset_addresses::TBRG_TOKEN_ADDRESS.to_string(),
                }))?)
            } else {
                None
            },
            metadata: AssetMetadata {
                symbol: "TBRG".to_string(),
                name: "TrustBridge Token".to_string(),
                decimals: 7, // Standard Stellar decimal places
                description: Some("Native token of the TrustBridge ecosystem".to_string()),
                image: None,
            },
        };
        
        Ok(asset_info)
    }
    
    /// Fetch all supported assets information
    pub fn fetch_all_assets(&self, account_address: Option<&str>) -> Result<Vec<AssetInfo>, AssetFetchError> {
        let mut assets = Vec::new();
        
        // Fetch XLM info
        match self.fetch_xlm_info(account_address) {
            Ok(asset) => assets.push(asset),
            Err(e) => eprintln!("Failed to fetch XLM info: {}", e),
        }
        
        // Fetch USDC info  
        match self.fetch_usdc_info(account_address) {
            Ok(asset) => assets.push(asset),
            Err(e) => eprintln!("Failed to fetch USDC info: {}", e),
        }
        
        // Fetch TBRG info
        match self.fetch_tbrg_info(account_address) {
            Ok(asset) => assets.push(asset),
            Err(e) => eprintln!("Failed to fetch TBRG info: {}", e),
        }
        
        if assets.is_empty() {
            return Err(AssetFetchError::AssetNotFound("No assets could be fetched".to_string()));
        }
        
        Ok(assets)
    }
    
    /// Helper function to fetch account balance for a specific asset
    fn fetch_account_balance(&self, _account_address: &str, asset_type: &AssetType) -> Result<i128, AssetFetchError> {
        // In a real implementation, this would make HTTP requests to Horizon API
        // For now, returning mock data to satisfy the interface
        match asset_type {
            AssetType::Native => Ok(1000000000), // 100 XLM in stroops
            AssetType::Credit(_) => Ok(5000000), // 5 tokens 
        }
    }
    
    /// Validate asset address format
    pub fn validate_asset_address(address: &str) -> Result<(), AssetFetchError> {
        if address.len() != 56 {
            return Err(AssetFetchError::InvalidAddress(
                "Stellar address must be 56 characters long".to_string()
            ));
        }
        
        if !address.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            return Err(AssetFetchError::InvalidAddress(
                "Stellar address must contain only uppercase letters and digits".to_string()
            ));
        }
        
        Ok(())
    }
}

impl Default for AssetFetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for quick asset retrieval
pub mod quick_fetch {
    use super::*;
    
    /// Quick function to get XLM asset info
    pub fn xlm() -> Result<AssetInfo, AssetFetchError> {
        let fetcher = AssetFetcher::new();
        fetcher.fetch_xlm_info(None)
    }
    
    /// Quick function to get USDC asset info
    pub fn usdc() -> Result<AssetInfo, AssetFetchError> {
        let fetcher = AssetFetcher::new();
        fetcher.fetch_usdc_info(None)
    }
    
    /// Quick function to get TrustBridge token info
    pub fn tbrg() -> Result<AssetInfo, AssetFetchError> {
        let fetcher = AssetFetcher::new();
        fetcher.fetch_tbrg_info(None)
    }
    
    /// Quick function to get all supported assets
    pub fn all_assets() -> Result<Vec<AssetInfo>, AssetFetchError> {
        let fetcher = AssetFetcher::new();
        fetcher.fetch_all_assets(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_asset_address_validation() {
        // Valid address
        assert!(AssetFetcher::validate_asset_address("CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC").is_ok());
        
        // Invalid length
        assert!(AssetFetcher::validate_asset_address("TOOSHORT").is_err());
        
        // Invalid characters
        assert!(AssetFetcher::validate_asset_address("cdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc").is_err());
    }
    
    #[test]
    fn test_asset_info_creation() {
        let asset_info = AssetInfo {
            asset_type: AssetType::Native,
            contract_address: asset_addresses::XLM_ADDRESS.to_string(),
            balance: Some(1000000000),
            metadata: AssetMetadata {
                symbol: "XLM".to_string(),
                name: "Stellar Lumens".to_string(),
                decimals: 7,
                description: Some("Native cryptocurrency".to_string()),
                image: None,
            },
        };
        
        assert_eq!(asset_info.metadata.symbol, "XLM");
        assert_eq!(asset_info.metadata.decimals, 7);
        assert!(matches!(asset_info.asset_type, AssetType::Native));
    }
}