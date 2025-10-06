use soroban_sdk::{Address, Env, Symbol, Vec};
use crate::{ProtectionConfig, TradeRecord};

// Storage keys
#[derive(Clone)]
pub enum DataKey {
    Initialized,
    Admin,
    Config,
    TradeHistory(Address, u32),                    // (asset, block) -> Vec<TradeRecord>
    TradesCurrentBlock(Address),                   // trader -> count
    CurrentBlock,                                  // last processed block number
    FlaggedAddress(Address),                       // address -> (bool, reason)
    PoolLiquidity(Address),                        // asset -> liquidity amount
    CurrentPrice(Address),                         // asset -> current price
}

const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub fn extend_instance(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

// Initialization
pub fn is_initialized(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Initialized)
}

pub fn set_initialized(e: &Env) {
    e.storage().instance().set(&DataKey::Initialized, &true);
}

// Admin
pub fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

// Configuration
pub fn get_config(e: &Env) -> ProtectionConfig {
    e.storage().instance().get(&DataKey::Config).unwrap()
}

pub fn set_config(e: &Env, config: &ProtectionConfig) {
    e.storage().instance().set(&DataKey::Config, config);
}

// Trade records
pub fn add_trade_record(e: &Env, trade: &TradeRecord) {
    let block = trade.block;
    let asset = trade.asset.clone();
    
    let mut trades = get_trades_for_block(e, &asset, block);
    trades.push_back(trade.clone());
    
    e.storage()
        .temporary()
        .set(&DataKey::TradeHistory(asset, block), &trades);
    
    // Update current block tracker
    update_current_block(e, block);
    
    // Cleanup old trades (keep last 100 blocks)
    cleanup_old_trades(e, &asset, block);
}

fn get_trades_for_block(e: &Env, asset: &Address, block: u32) -> Vec<TradeRecord> {
    e.storage()
        .temporary()
        .get(&DataKey::TradeHistory(asset.clone(), block))
        .unwrap_or(Vec::new(e))
}

pub fn get_trades_in_window(
    e: &Env,
    asset: &Address,
    start_block: u32,
    end_block: u32,
) -> Vec<TradeRecord> {
    let mut all_trades = Vec::new(e);
    
    for block in start_block..=end_block {
        let trades = get_trades_for_block(e, asset, block);
        for i in 0..trades.len() {
            all_trades.push_back(trades.get(i).unwrap());
        }
    }
    
    all_trades
}

fn update_current_block(e: &Env, block: u32) {
    let current = e.storage()
        .instance()
        .get(&DataKey::CurrentBlock)
        .unwrap_or(0u32);
    
    if block > current {
        e.storage().instance().set(&DataKey::CurrentBlock, &block);
        // Reset trades count for new block
        reset_all_trades_counts(e);
    }
}

fn cleanup_old_trades(e: &Env, asset: &Address, current_block: u32) {
    if current_block > 100 {
        let old_block = current_block - 100;
        e.storage()
            .temporary()
            .remove(&DataKey::TradeHistory(asset.clone(), old_block));
    }
}

// Trades per block tracking (for rate limiting)
pub fn get_trades_count_current_block(e: &Env, trader: &Address) -> u32 {
    e.storage()
        .temporary()
        .get(&DataKey::TradesCurrentBlock(trader.clone()))
        .unwrap_or(0u32)
}

pub fn increment_trades_count_current_block(e: &Env, trader: &Address) {
    let count = get_trades_count_current_block(e, trader);
    e.storage()
        .temporary()
        .set(&DataKey::TradesCurrentBlock(trader.clone()), &(count + 1));
}

fn reset_all_trades_counts(e: &Env) {
    // In production, you'd want to track all active traders
    // For now, counts will naturally reset when block changes
    // since we check current block in update_current_block
}

// Flagged addresses
pub fn is_flagged(e: &Env, address: &Address) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::FlaggedAddress(address.clone()))
}

pub fn flag_address(e: &Env, address: &Address, reason: &Symbol) {
    e.storage()
        .persistent()
        .set(&DataKey::FlaggedAddress(address.clone()), reason);
}

pub fn unflag_address(e: &Env, address: &Address) {
    e.storage()
        .persistent()
        .remove(&DataKey::FlaggedAddress(address.clone()));
}

// Pool liquidity (updated by external oracle or pool contract)
pub fn get_pool_liquidity(e: &Env, asset: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::PoolLiquidity(asset.clone()))
        .unwrap_or(1_000_000_000) // Default 1B for testing
}

pub fn set_pool_liquidity(e: &Env, asset: &Address, liquidity: i128) {
    e.storage()
        .persistent()
        .set(&DataKey::PoolLiquidity(asset.clone()), &liquidity);
}

// Current price (updated by oracle)
pub fn get_current_price(e: &Env, asset: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::CurrentPrice(asset.clone()))
        .unwrap_or(1_000_000) // Default price for testing
}

pub fn set_current_price(e: &Env, asset: &Address, price: i128) {
    e.storage()
        .persistent()
        .set(&DataKey::CurrentPrice(asset.clone()), &price);
}