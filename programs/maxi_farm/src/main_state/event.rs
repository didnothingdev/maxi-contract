use anchor_lang::prelude::*;

// MainState initialization event
#[event]
pub struct MainStateInitialized {
    pub owner: Pubkey,
    pub signer: Pubkey,
    pub withdrawer: Pubkey,
    
    pub trading_fee: u64,
    pub fee_recipient: Pubkey
}

// Transfer ownership event
#[event]
pub struct OwnershipTransferred {
    pub previous_owner: Pubkey,
    pub new_owner: Pubkey
}

// MainState updated event
#[event]
pub struct MainStateUpdated {
    pub signer: Pubkey,
    pub withdrawer: Pubkey,
    
    pub trading_fee: u64,
    pub fee_recipient: Pubkey
}
