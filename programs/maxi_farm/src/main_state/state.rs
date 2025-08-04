use anchor_lang::prelude::*;

// Main state of Program
#[account]
pub struct MainState {
    pub owner: Pubkey,                  // Address of the Program owner (The initializer becomes the initial program owner)
    pub signer: Pubkey,                 // Address of signer
    pub withdrawer: Pubkey,             // Address of withdrawer
    
    pub trading_fee: u64,               // Trading fee applied on buying/selling tokens (default: 1%)
    pub fee_recipient: Pubkey,          // Address of the fee recipient (Owner becomes the initial fee recipient)

    pub tier1_reward: u64,              // Tier1 reward percent (25%)
    pub tier2_reward: u64,              // Tier2 reward percent (3.5%)
    pub tier3_reward: u64,              // Tier3 reward percent (3%)
}

impl MainState {
    pub const MAX_SIZE: usize = std::mem::size_of::<Self>();    // Size of MainState
    pub const PREFIX_SEED: &'static [u8] = b"main";             // Seed of MainState
}
