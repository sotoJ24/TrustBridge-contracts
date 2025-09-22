use test_suites::asset_fetcher::{
    AssetFetcher, AssetInfo, AssetType, CreditAsset, AssetFetchError, 
    asset_addresses, quick_fetch
};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_asset_fetcher_creation() {
        let fetcher = AssetFetcher::new();
        assert_eq!(fetcher.horizon_url, "https://horizon.stellar.org");
        assert_eq!(fetcher.network_passphrase, "Public Global Stellar Network ; September 2015");
    }
    
    #[test]
    fn test_asset_fetcher_with_custom_config() {
        let custom_url = "https://horizon-testnet.stellar.org".to_string();
        let custom_passphrase = "Test SDF Network ; September 2015".to_string();
        
        let fetcher = AssetFetcher::with_config(
            custom_url.clone(), 
            custom_passphrase.clone()
        );
        
        assert_eq!(fetcher.horizon_url, custom_url);
        assert_eq!(fetcher.network_passphrase, custom_passphrase);
    }
    
    #[test]
    fn test_asset_address_constants() {
        assert_eq!(asset_addresses::XLM_ADDRESS, "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC");
        assert_eq!(asset_addresses::USDC_ADDRESS, "CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU");
        assert_eq!(asset_addresses::TBRG_TOKEN_ADDRESS, "CAAUAE53WKWR4X2BRCHXNUTDJGXTOBMHMK3KFTAPEUBA7MJEQBPWVWQU");
        assert_eq!(asset_addresses::ORACLE_CONTRACT_ADDRESS, "CBCIZHUC42CKOZHKKEYMSXVVY24ZK2EKEUU6NFGQS5YFG7GAMEU5L32M");
    }
    
    #[test]
    fn test_asset_address_validation_valid() {
        let valid_addresses = vec![
            asset_addresses::XLM_ADDRESS,
            asset_addresses::USDC_ADDRESS,
            asset_addresses::TBRG_TOKEN_ADDRESS,
            asset_addresses::ORACLE_CONTRACT_ADDRESS,
        ];
        
        for addr in valid_addresses {
            assert!(
                AssetFetcher::validate_asset_address(addr).is_ok(),
                "Address {} should be valid", addr
            );
        }
    }
    
    #[test]
    fn test_asset_address_validation_invalid_length() {
        let short_address = "TOOSHORT";
        let long_address = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSCTOOLONG";
        
        assert!(AssetFetcher::validate_asset_address(short_address).is_err());
        assert!(AssetFetcher::validate_asset_address(long_address).is_err());
    }
    
    #[test]
    fn test_asset_address_validation_invalid_characters() {
        let lowercase_address = "cdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc";
        let special_chars_address = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCY$C";
        
        assert!(AssetFetcher::validate_asset_address(lowercase_address).is_err());
        assert!(AssetFetcher::validate_asset_address(special_chars_address).is_err());
    }
    
    #[test]
    fn test_fetch_xlm_info_without_account() {
        let fetcher = AssetFetcher::new();
        let result = fetcher.fetch_xlm_info(None);
        
        assert!(result.is_ok());
        let asset_info = result.unwrap();
        
        assert!(matches!(asset_info.asset_type, AssetType::Native));
        assert_eq!(asset_info.contract_address, asset_addresses::XLM_ADDRESS);
        assert_eq!(asset_info.metadata.symbol, "XLM");
        assert_eq!(asset_info.metadata.name, "Stellar Lumens");
        assert_eq!(asset_info.metadata.decimals, 7);
        assert!(asset_info.balance.is_none());
    }
    
    #[test]
    fn test_fetch_xlm_info_with_account() {
        let fetcher = AssetFetcher::new();
        let test_account = "GCKFBEIYTKP5RHALFTTZDRJ75ML6KQFMDBXQDVZPSYQ7EOCMUWDLRPNW";
        let result = fetcher.fetch_xlm_info(Some(test_account));
        
        assert!(result.is_ok());
        let asset_info = result.unwrap();
        
        assert!(matches!(asset_info.asset_type, AssetType::Native));
        assert!(asset_info.balance.is_some());
        assert_eq!(asset_info.balance.unwrap(), 1000000000); // Mock balance
    }
    
    #[test]
    fn test_fetch_usdc_info_without_account() {
        let fetcher = AssetFetcher::new();
        let result = fetcher.fetch_usdc_info(None);
        
        assert!(result.is_ok());
        let asset_info = result.unwrap();
        
        match asset_info.asset_type {
            AssetType::Credit(credit_asset) => {
                assert_eq!(credit_asset.code, "USDC");
                assert_eq!(credit_asset.issuer, "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5");
            },
            _ => panic!("Expected Credit asset type for USDC"),
        }
        
        assert_eq!(asset_info.contract_address, asset_addresses::USDC_ADDRESS);
        assert_eq!(asset_info.metadata.symbol, "USDC");
        assert_eq!(asset_info.metadata.name, "USD Coin");
        assert_eq!(asset_info.metadata.decimals, 6);
        assert!(asset_info.balance.is_none());
    }
    
    #[test]
    fn test_fetch_usdc_info_with_account() {
        let fetcher = AssetFetcher::new();
        let test_account = "GCKFBEIYTKP5RHALFTTZDRJ75ML6KQFMDBXQDVZPSYQ7EOCMUWDLRPNW";
        let result = fetcher.fetch_usdc_info(Some(test_account));
        
        assert!(result.is_ok());
        let asset_info = result.unwrap();
        
        assert!(asset_info.balance.is_some());
        assert_eq!(asset_info.balance.unwrap(), 5000000); // Mock balance
    }
    
    #[test]
    fn test_fetch_tbrg_info_without_account() {
        let fetcher = AssetFetcher::new();
        let result = fetcher.fetch_tbrg_info(None);
        
        assert!(result.is_ok());
        let asset_info = result.unwrap();
        
        match asset_info.asset_type {
            AssetType::Credit(credit_asset) => {
                assert_eq!(credit_asset.code, "TBRG");
                assert_eq!(credit_asset.issuer, asset_addresses::TBRG_TOKEN_ADDRESS);
            },
            _ => panic!("Expected Credit asset type for TBRG"),
        }
        
        assert_eq!(asset_info.contract_address, asset_addresses::TBRG_TOKEN_ADDRESS);
        assert_eq!(asset_info.metadata.symbol, "TBRG");
        assert_eq!(asset_info.metadata.name, "TrustBridge Token");
        assert_eq!(asset_info.metadata.decimals, 7);
        assert!(asset_info.balance.is_none());
    }
    
    #[test]
    fn test_fetch_tbrg_info_with_account() {
        let fetcher = AssetFetcher::new();
        let test_account = "GCKFBEIYTKP5RHALFTTZDRJ75ML6KQFMDBXQDVZPSYQ7EOCMUWDLRPNW";
        let result = fetcher.fetch_tbrg_info(Some(test_account));
        
        assert!(result.is_ok());
        let asset_info = result.unwrap();
        
        assert!(asset_info.balance.is_some());
        assert_eq!(asset_info.balance.unwrap(), 5000000); // Mock balance
    }
    
    #[test]
    fn test_fetch_all_assets_without_account() {
        let fetcher = AssetFetcher::new();
        let result = fetcher.fetch_all_assets(None);
        
        assert!(result.is_ok());
        let assets = result.unwrap();
        
        assert_eq!(assets.len(), 3);
        
        // Check that we have all expected assets
        let symbols: Vec<String> = assets.iter().map(|a| a.metadata.symbol.clone()).collect();
        assert!(symbols.contains(&"XLM".to_string()));
        assert!(symbols.contains(&"USDC".to_string()));
        assert!(symbols.contains(&"TBRG".to_string()));
    }
    
    #[test]
    fn test_fetch_all_assets_with_account() {
        let fetcher = AssetFetcher::new();
        let test_account = "GCKFBEIYTKP5RHALFTTZDRJ75ML6KQFMDBXQDVZPSYQ7EOCMUWDLRPNW";
        let result = fetcher.fetch_all_assets(Some(test_account));
        
        assert!(result.is_ok());
        let assets = result.unwrap();
        
        assert_eq!(assets.len(), 3);
        
        // Check that all assets have balances
        for asset in assets {
            assert!(asset.balance.is_some());
        }
    }
    
    #[test]
    fn test_quick_fetch_xlm() {
        let result = quick_fetch::xlm();
        assert!(result.is_ok());
        
        let asset_info = result.unwrap();
        assert!(matches!(asset_info.asset_type, AssetType::Native));
        assert_eq!(asset_info.metadata.symbol, "XLM");
    }
    
    #[test]
    fn test_quick_fetch_usdc() {
        let result = quick_fetch::usdc();
        assert!(result.is_ok());
        
        let asset_info = result.unwrap();
        assert_eq!(asset_info.metadata.symbol, "USDC");
    }
    
    #[test]
    fn test_quick_fetch_tbrg() {
        let result = quick_fetch::tbrg();
        assert!(result.is_ok());
        
        let asset_info = result.unwrap();
        assert_eq!(asset_info.metadata.symbol, "TBRG");
    }
    
    #[test]
    fn test_quick_fetch_all_assets() {
        let result = quick_fetch::all_assets();
        assert!(result.is_ok());
        
        let assets = result.unwrap();
        assert_eq!(assets.len(), 3);
    }
    
    #[test]
    fn test_asset_type_equality() {
        let native1 = AssetType::Native;
        let native2 = AssetType::Native;
        assert_eq!(native1, native2);
        
        let credit1 = AssetType::Credit(CreditAsset {
            code: "USDC".to_string(),
            issuer: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
        });
        let credit2 = AssetType::Credit(CreditAsset {
            code: "USDC".to_string(),
            issuer: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
        });
        assert_eq!(credit1, credit2);
        
        assert_ne!(native1, credit1);
    }
    
    #[test]
    fn test_credit_asset_equality() {
        let credit1 = CreditAsset {
            code: "USDC".to_string(),
            issuer: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
        };
        let credit2 = CreditAsset {
            code: "USDC".to_string(),
            issuer: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5".to_string(),
        };
        let credit3 = CreditAsset {
            code: "TBRG".to_string(),
            issuer: asset_addresses::TBRG_TOKEN_ADDRESS.to_string(),
        };
        
        assert_eq!(credit1, credit2);
        assert_ne!(credit1, credit3);
    }
    
    #[test]
    fn test_error_display() {
        let errors = vec![
            AssetFetchError::NetworkError("Connection failed".to_string()),
            AssetFetchError::InvalidAddress("Bad address".to_string()),
            AssetFetchError::AssetNotFound("Asset missing".to_string()),
            AssetFetchError::ParseError("JSON parse error".to_string()),
            AssetFetchError::InvalidAssetType("Unknown type".to_string()),
            AssetFetchError::RateLimitExceeded,
            AssetFetchError::Unauthorized,
        ];
        
        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
        }
    }
    
    #[test]
    fn test_default_asset_fetcher() {
        let fetcher1 = AssetFetcher::default();
        let fetcher2 = AssetFetcher::new();
        
        assert_eq!(fetcher1.horizon_url, fetcher2.horizon_url);
        assert_eq!(fetcher1.network_passphrase, fetcher2.network_passphrase);
    }
}