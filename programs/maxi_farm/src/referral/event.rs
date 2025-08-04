use anchor_lang::prelude::*;

// User registered event
#[event]
pub struct UserRegisteredEvent {
    pub referree: Pubkey, // Referree wallet address
    pub referrer: Pubkey, // Referrer wallet address
    pub timestamp: i64 // Registered time
}

// Rewards claimed event
#[event]
pub struct RewardsClaimEvent {
    pub user: Pubkey, // User wallet address
    pub rewards: u64, // Rewards amount
    pub timestamp: i64 // Registered time
}