// contracts/oracle-aggregator/src/contract.rs
use soroban_sdk::{contract, contractimpl, Address, Env, Vec, Map};

#[contract]
pub struct OracleAggregator;

#[contractimpl]
impl OracleAggregator {
    /// Initialize oracle aggregator
    pub fn initialize(
        env: Env,
        admin: Address,
        price_deviation_threshold: u32, // Basis points (500 = 5%)
        heartbeat_timeout: u64, // Seconds
        min_oracles_required: u32
    ) -> Result<(), OracleError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(OracleError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::PriceDeviationThreshold, &price_deviation_threshold);
        env.storage().instance().set(&DataKey::HeartbeatTimeout, &heartbeat_timeout);
        env.storage().instance().set(&DataKey::MinOraclesRequired, &min_oracles_required);
        env.storage().instance().set(&DataKey::Initialized, &true);

        Ok(())
    }

    /// Add oracle source
    pub fn add_oracle_source(
        env: Env,
        admin: Address,
        asset: Address,
        oracle: Address,
        weight: u32 // Weight for weighted average (e.g., 100 = 100%)
    ) -> Result<(), OracleError> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        let mut sources = Self::get_oracle_sources(&env, &asset);

        // Check if oracle already exists
        if sources.iter().any(|s| s.oracle == oracle) {
            return Err(OracleError::OracleAlreadyExists);
        }

        let source = OracleSource {
            oracle,
            weight,
            last_update: 0,
            is_active: true,
        };

        sources.push_back(source);
        env.storage().persistent().set(&DataKey::OracleSources(asset.clone()), &sources);

        emit_oracle_source_added(&env, asset, oracle, weight);
        Ok(())
    }

    /// Get aggregated price with validation
    pub fn get_price(env: Env, asset: Address) -> Result<(i128, u64), OracleError> {
        let sources = Self::get_oracle_sources(&env, &asset);
        let min_required = Self::get_min_oracles_required(&env);

        if sources.len() < min_required {
            return Err(OracleError::InsufficientOracleSources);
        }

        let current_time = env.ledger().timestamp();
        let heartbeat_timeout = Self::get_heartbeat_timeout(&env);

        let mut valid_prices: Vec<(i128, u32, u64)> = Vec::new(&env); // (price, weight, timestamp)
        let mut total_weight = 0u32;

        // Collect valid prices from all sources
        for source in sources {
            if !source.is_active {
                continue;
            }

            // Get price from oracle
            match Self::get_oracle_price(&env, &source.oracle, &asset) {
                Ok((price, timestamp)) => {
                    // Check heartbeat
                    if current_time - timestamp <= heartbeat_timeout {
                        valid_prices.push_back((price, source.weight, timestamp));
                        total_weight += source.weight;
                    }
                },
                Err(_) => continue, // Skip failing oracles
            }
        }

        if valid_prices.len() < min_required {
            return Err(OracleError::InsufficientValidPrices);
        }

        // Calculate weighted average
        let mut weighted_sum = 0i128;
        let mut latest_timestamp = 0u64;

        for (price, weight, timestamp) in valid_prices.iter() {
            weighted_sum += price * (*weight as i128);
            latest_timestamp = latest_timestamp.max(*timestamp);
        }

        let aggregated_price = weighted_sum / (total_weight as i128);

        // Validate price deviation
        Self::validate_price_deviation(&env, &asset, aggregated_price, &valid_prices)?;

        // Store aggregated price
        env.storage().persistent().set(
            &DataKey::AggregatedPrice(asset.clone()),
            &(aggregated_price, latest_timestamp)
        );

        emit_price_aggregated(&env, asset, aggregated_price, valid_prices.len(), total_weight);
        Ok((aggregated_price, latest_timestamp))
    }

    /// Emergency price override (guardian only)
    pub fn emergency_set_price(
        env: Env,
        guardian: Address,
        asset: Address,
        price: i128,
        duration: u64 // Emergency price validity duration
    ) -> Result<(), OracleError> {
        guardian.require_auth();
        Self::require_emergency_guardian(&env, &guardian)?;

        let emergency_price = EmergencyPrice {
            price,
            set_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + duration,
            set_by: guardian.clone(),
        };

        env.storage().persistent().set(&DataKey::EmergencyPrice(asset.clone()), &emergency_price);

        emit_emergency_price_set(&env, asset, price, duration, guardian);
        Ok(())
    }

    /// Validate price deviation among sources
    fn validate_price_deviation(
        env: &Env,
        asset: &Address,
        aggregated_price: i128,
        prices: &Vec<(i128, u32, u64)>
    ) -> Result<(), OracleError> {
        let deviation_threshold = Self::get_price_deviation_threshold(env);

        for (price, _, _) in prices.iter() {
            let deviation = if *price > aggregated_price {
                ((*price - aggregated_price) * 10000) / aggregated_price
            } else {
                ((aggregated_price - *price) * 10000) / aggregated_price
            };

            if deviation > deviation_threshold as i128 {
                emit_price_deviation_alert(env, asset.clone(), *price, aggregated_price, deviation);
                // Don't fail, but log for monitoring
            }
        }

        Ok(())
    }

    /// Circuit breaker for extreme price movements
    pub fn check_circuit_breaker(
        env: Env,
        asset: Address,
        new_price: i128
    ) -> Result<bool, OracleError> {
        if let Ok((last_price, last_timestamp)) = env.storage().persistent()
            .get::<DataKey, (i128, u64)>(&DataKey::AggregatedPrice(asset.clone())) {

            let time_diff = env.ledger().timestamp() - last_timestamp;

            // Check for dramatic price changes in short time
            if time_diff < 300 { // 5 minutes
                let price_change = if new_price > last_price {
                    ((new_price - last_price) * 10000) / last_price
                } else {
                    ((last_price - new_price) * 10000) / last_price
                };

                // Trigger circuit breaker for >50% price change in 5 minutes
                if price_change > 5000 {
                    emit_circuit_breaker_triggered(&env, asset, last_price, new_price, time_diff);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[derive(Clone, Debug)]
pub struct OracleSource {
    pub oracle: Address,
    pub weight: u32,
    pub last_update: u64,
    pub is_active: bool,
}

#[derive(Clone, Debug)]
pub struct EmergencyPrice {
    pub price: i128,
    pub set_at: u64,
    pub expires_at: u64,
    pub set_by: Address,
}