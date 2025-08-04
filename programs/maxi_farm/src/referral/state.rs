use anchor_lang::prelude::*;

// Referral state
#[account]
pub struct ReferralState {
    pub user: Pubkey, // User's public key
    pub earned_rewards: u64, // Accumulated referral rewards
    pub referrer: Pubkey, // Direct referrer (Tier 1)
}

impl ReferralState {
    pub const MAX_SIZE: usize = std::mem::size_of::<Self>();    // Size of ReferralState
    pub const PREFIX_SEED: &'static [u8] = b"referral";             // Seed of ReferralState
}
