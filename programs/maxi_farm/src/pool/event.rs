use anchor_lang::prelude::*;

// BondingCurve create event
#[event]
pub struct CreateEvent {
    pub creator: Pubkey,        // Creator wallet address
    pub base_mint: Pubkey,      // Creating token mint address
    pub metadata_uri: String,   // Metadata URI
    pub tax: u64,               // Tax
    pub max_fee_tokens: u64,    // Max. fee tokens
    pub total_supply: u64,      // Total supply
    pub real_quote_threshold: u64,  // Real quote threshold
    pub base_reserves: u64,     // Number of total token reserves
    pub quote_reserves: u64,    // Number of total SOL reserves
    pub priv_sale_period: u64,  // Private sale period
    pub timestamp: i64,         // Creation time
    pub coin_type: u8           // Coin type
}

// BondingCurve trade event
#[event]
pub struct TradeEvent {
    pub user: Pubkey,           // Trader wallet address
    pub base_mint: Pubkey,      // Trading token mint address
    pub sol_amount: u64,        // Traded amount of SOL
    pub token_amount: u64,      // Traded amount of tokens
    pub base_reserves: u64,     // Updated token reserves
    pub quote_reserves: u64,    // Updated SOL reserves
    pub is_buy: bool,           // Flag indicating whether the user bought or sold
    pub timestamp: i64,         // Traded time,
    pub tier1_referrer: Pubkey, // Tier1 referrer
    pub tier1_reward: u64,      // Tier1 reward
    pub tier2_referrer: Pubkey, // Tier2 referrer
    pub tier2_reward: u64,      // Tier2 reward
    pub tier3_referrer: Pubkey, // Tier3 referrer
    pub tier3_reward: u64,      // Tier3 reward
}

// Tax updated event
#[event]
pub struct TaxUpdatedEvent {
    pub owner: Pubkey,          // Owner
    pub tax: u64                // New tax
}

// BondingCurve complete event
#[event]
pub struct CompleteEvent {
    pub base_mint: Pubkey,      // Completed token mint address
    pub timestamp: i64          // Completed time
}

// BondingCurve withdraw event
#[event]
pub struct WithdrawEvent {
    pub withdrawer: Pubkey,
    pub base_mint: Pubkey,      // Completed token mint address
    pub base_amount: u64,       // Withdrawn token amount
    pub quote_amount: u64,      // Withdrawn SOL amount
    pub timestamp: i64          // Completed time
}
