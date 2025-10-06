#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, Address, Env, Symbol, Vec,
};

mod storage;
mod error;
mod events;

pub use error::OracleError;
pub use events::OracleEvents;

// SEP-40 PriceData structure with enhanced metadata
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    pub price: i128,           // Price with decimals precision
    pub timestamp: u64,        // Unix timestamp
    pub source_count: u32,     // Number of sources used for this price
    pub confidence: u32,       // Confidence score (0-100)
}

// Price source for multi-oracle aggregation
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceSource {
    pub source_id: Symbol,     // Source identifier
    pub price: i128,           // Price from this source
    pub timestamp: u64,        // When this source was updated
    pub weight: u32,           // Weight in aggregation (0-100)
}

// Circuit breaker state
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreaker {
    pub is_paused: bool,
    pub pause_timestamp: u64,
    pub reason: Symbol,
}

// Oracle configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub max_price_deviation_bps: u32,  // Max deviation in basis points (e.g., 1000 = 10%)
    pub max_staleness_seconds: u64,     // Max time before price is stale
    pub min_sources_required: u32,      // Minimum sources needed for valid price
    pub heartbeat_interval: u64,        // Required update frequency
}

// Asset representation for SEP-40 compatibility
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Stellar(Address),     // Stellar asset contract address
    Other(Symbol),        // Other asset identifier
}

/// Secure TrustBridge Oracle Contract
/// 
/// Enhanced implementation with:
/// - Multi-source price aggregation
/// - Price deviation protection
/// - Staleness checks
/// - Circuit breakers
/// - Multi-sig admin capabilities
#[contract]
pub struct TrustBridgeOracle;

pub trait OracleTrait {
    /// Initialize the oracle with admins and configuration
    fn init(
        e: Env,
        admins: Vec<Address>,
        min_signatures: u32,
        config: OracleConfig,
    );

    /// Submit price from a trusted source (multi-sig required)
    fn submit_price(
        e: Env,
        asset: Asset,
        price: i128,
        source_id: Symbol,
    );

    /// Get the aggregated price for an asset (with staleness check)
    fn lastprice(e: Env, asset: Asset) -> Option<PriceData>;

    /// Get decimals
    fn decimals(e: Env) -> u32;

    /// Emergency pause (multi-sig required)
    fn pause(e: Env, reason: Symbol);

    /// Resume operations (multi-sig required)
    fn resume(e: Env);

    /// Update oracle configuration (multi-sig required)
    fn update_config(e: Env, config: OracleConfig);

    /// Add trusted price source (multi-sig required)
    fn add_source(e: Env, source_id: Symbol, weight: u32);

    /// Remove trusted price source (multi-sig required)
    fn remove_source(e: Env, source_id: Symbol);

    /// Get circuit breaker status
    fn get_circuit_breaker(e: Env) -> CircuitBreaker;

    /// Get oracle configuration
    fn get_config(e: Env) -> OracleConfig;

    /// Add admin (multi-sig required)
    fn add_admin(e: Env, new_admin: Address);

    /// Remove admin (multi-sig required)
    fn remove_admin(e: Env, admin: Address);

    /// Get all admins
    fn get_admins(e: Env) -> Vec<Address>;

    /// Get price with all source data (for transparency)
    fn get_price_sources(e: Env, asset: Asset) -> Vec<PriceSource>;
}

#[contractimpl]
impl OracleTrait for TrustBridgeOracle {
    fn init(
        e: Env,
        admins: Vec<Address>,
        min_signatures: u32,
        config: OracleConfig,
    ) {
        if storage::has_admins(&e) {
            panic_with_error!(&e, OracleError::AlreadyInitialized);
        }

        if admins.is_empty() || min_signatures == 0 || min_signatures > admins.len() {
            panic_with_error!(&e, OracleError::InvalidInput);
        }

        // Validate config
        if config.max_price_deviation_bps > 10000 {  // Max 100%
            panic_with_error!(&e, OracleError::InvalidInput);
        }

        storage::set_admins(&e, &admins);
        storage::set_min_signatures(&e, min_signatures);
        storage::set_config(&e, &config);
        
        // Initialize circuit breaker as active
        let cb = CircuitBreaker {
            is_paused: false,
            pause_timestamp: 0,
            reason: Symbol::new(&e, ""),
        };
        storage::set_circuit_breaker(&e, &cb);

        OracleEvents::initialized(&e, admins.get(0).unwrap());
    }

    fn submit_price(
        e: Env,
        asset: Asset,
        price: i128,
        source_id: Symbol,
    ) {
        // Check circuit breaker
        let cb = storage::get_circuit_breaker(&e);
        if cb.is_paused {
            panic_with_error!(&e, OracleError::CircuitBreakerActive);
        }

        // Verify caller is authorized source
        storage::require_authorized_source(&e, &source_id);

        if price <= 0 {
            panic_with_error!(&e, OracleError::InvalidPrice);
        }

        let config = storage::get_config(&e);
        let timestamp = e.ledger().timestamp();

        // Check if we have a previous aggregated price for deviation check
        if let Some(prev_price_data) = storage::get_aggregated_price(&e, &asset) {
            // Check price deviation
            let deviation_bps = calculate_deviation_bps(price, prev_price_data.price);
            if deviation_bps > config.max_price_deviation_bps {
                // Price change too large - auto-pause
                Self::auto_pause(&e, Symbol::new(&e, "deviation"));
                panic_with_error!(&e, OracleError::PriceDeviationExceeded);
            }

            // Check heartbeat
            let time_since_update = timestamp - prev_price_data.timestamp;
            if time_since_update > config.heartbeat_interval * 2 {
                // Missed heartbeat - flag warning
                OracleEvents::heartbeat_missed(&e, asset.clone(), time_since_update);
            }
        }

        // Store price from this source
        let source_weight = storage::get_source_weight(&e, &source_id);
        let price_source = PriceSource {
            source_id: source_id.clone(),
            price,
            timestamp,
            weight: source_weight,
        };

        storage::set_price_source(&e, &asset, &source_id, &price_source);

        // Aggregate prices from all sources
        Self::aggregate_prices(&e, &asset, &config);

        OracleEvents::price_submitted(&e, asset, source_id, price, timestamp);
    }

    fn lastprice(e: Env, asset: Asset) -> Option<PriceData> {
        let config = storage::get_config(&e);
        let price_data = storage::get_aggregated_price(&e, &asset)?;
        
        // Check staleness
        let current_time = e.ledger().timestamp();
        let age = current_time - price_data.timestamp;
        
        if age > config.max_staleness_seconds {
            OracleEvents::stale_price_detected(&e, asset, age);
            return None;  // Price too old
        }

        // Check minimum sources
        if price_data.source_count < config.min_sources_required {
            return None;  // Not enough sources
        }

        Some(price_data)
    }

    fn decimals(_e: Env) -> u32 {
        7  // TrustBridge Oracle uses 7 decimals
    }

    fn pause(e: Env, reason: Symbol) {
        storage::require_multi_sig(&e);

        let cb = CircuitBreaker {
            is_paused: true,
            pause_timestamp: e.ledger().timestamp(),
            reason: reason.clone(),
        };

        storage::set_circuit_breaker(&e, &cb);
        OracleEvents::circuit_breaker_triggered(&e, reason);
    }

    fn resume(e: Env) {
        storage::require_multi_sig(&e);

        let cb = CircuitBreaker {
            is_paused: false,
            pause_timestamp: 0,
            reason: Symbol::new(&e, ""),
        };

        storage::set_circuit_breaker(&e, &cb);
        OracleEvents::circuit_breaker_reset(&e);
    }

    fn update_config(e: Env, config: OracleConfig) {
        storage::require_multi_sig(&e);

        if config.max_price_deviation_bps > 10000 {
            panic_with_error!(&e, OracleError::InvalidInput);
        }

        storage::set_config(&e, &config);
        OracleEvents::config_updated(&e);
    }

    fn add_source(e: Env, source_id: Symbol, weight: u32) {
        storage::require_multi_sig(&e);

        if weight > 100 {
            panic_with_error!(&e, OracleError::InvalidInput);
        }

        storage::add_trusted_source(&e, &source_id, weight);
        OracleEvents::source_added(&e, source_id, weight);
    }

    fn remove_source(e: Env, source_id: Symbol) {
        storage::require_multi_sig(&e);

        storage::remove_trusted_source(&e, &source_id);
        OracleEvents::source_removed(&e, source_id);
    }

    fn get_circuit_breaker(e: Env) -> CircuitBreaker {
        storage::get_circuit_breaker(&e)
    }

    fn get_config(e: Env) -> OracleConfig {
        storage::get_config(&e)
    }

    fn add_admin(e: Env, new_admin: Address) {
        storage::require_multi_sig(&e);

        storage::add_admin(&e, &new_admin);
        OracleEvents::admin_added(&e, new_admin);
    }

    fn remove_admin(e: Env, admin: Address) {
        storage::require_multi_sig(&e);

        let admins = storage::get_admins(&e);
        let min_sigs = storage::get_min_signatures(&e);

        if admins.len() <= min_sigs {
            panic_with_error!(&e, OracleError::InsufficientAdmins);
        }

        storage::remove_admin(&e, &admin);
        OracleEvents::admin_removed(&e, admin);
    }

    fn get_admins(e: Env) -> Vec<Address> {
        storage::get_admins(&e)
    }

    fn get_price_sources(e: Env, asset: Asset) -> Vec<PriceSource> {
        storage::get_all_price_sources(&e, &asset)
    }
}

// Internal helper functions
impl TrustBridgeOracle {
    fn aggregate_prices(e: &Env, asset: &Asset, config: &OracleConfig) {
        let sources = storage::get_all_price_sources(e, asset);
        
        if sources.is_empty() {
            return;
        }

        let current_time = e.ledger().timestamp();
        let mut total_weighted_price: i128 = 0;
        let mut total_weight: u32 = 0;
        let mut valid_sources: u32 = 0;

        // Calculate weighted average
        for i in 0..sources.len() {
            let source = sources.get(i).unwrap();
            
            // Skip stale sources
            if current_time - source.timestamp > config.max_staleness_seconds {
                continue;
            }

            total_weighted_price += source.price * (source.weight as i128);
            total_weight += source.weight;
            valid_sources += 1;
        }

        if valid_sources == 0 || total_weight == 0 {
            return;
        }

        let aggregated_price = total_weighted_price / (total_weight as i128);

        // Calculate confidence based on source count and weight distribution
        let confidence = calculate_confidence(valid_sources, sources.len());

        let price_data = PriceData {
            price: aggregated_price,
            timestamp: current_time,
            source_count: valid_sources,
            confidence,
        };

        storage::set_aggregated_price(e, asset, &price_data);
    }

    fn auto_pause(e: &Env, reason: Symbol) {
        let cb = CircuitBreaker {
            is_paused: true,
            pause_timestamp: e.ledger().timestamp(),
            reason: reason.clone(),
        };

        storage::set_circuit_breaker(e, &cb);
        OracleEvents::circuit_breaker_triggered(e, reason);
    }
}

// Helper functions
fn calculate_deviation_bps(new_price: i128, old_price: i128) -> u32 {
    if old_price == 0 {
        return 10000;  // 100% deviation
    }

    let diff = if new_price > old_price {
        new_price - old_price
    } else {
        old_price - new_price
    };

    ((diff * 10000) / old_price) as u32
}

fn calculate_confidence(valid_sources: u32, total_sources: u32) -> u32 {
    if total_sources == 0 {
        return 0;
    }

    // Confidence based on percentage of sources reporting
    let base_confidence = (valid_sources * 100) / total_sources;

    // Bonus for having multiple sources
    let source_bonus = if valid_sources >= 3 { 10 } else { 0 };

    u32::min(100, base_confidence + source_bonus)
}

#[cfg(test)]
mod test;