use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{Mint}
};
use crate::{
    constants::{NORMAL_REAL_QUOTE_THRESHOLD},
    error::MaxiFarmError,
    PoolState,
    CompleteEvent
};

// This function forcibly completes a bonding curve.
// Params
//   ctx - ForceComplete context
// Return
//   Ok on success, ErrorCode on Failure
//     CompleteEvent is emitted on success
pub fn force_complete(ctx: Context<AForceComplete>) -> Result<()> {
    let pool_state = &mut ctx.accounts.pool_state;

    require!(pool_state.complete.eq(&false), MaxiFarmError::BondingCurveComplete); // BondingCurve must not be complete
    // Check if bonding curve becomes complete
    if (pool_state.real_quote_threshold <= NORMAL_REAL_QUOTE_THRESHOLD) {
        require!(pool_state.real_quote_reserves >= pool_state.real_quote_threshold * 4 / 5, MaxiFarmError::InsufficientRealQuoteReserves);
    } else {
        require!(pool_state.real_quote_reserves >= NORMAL_REAL_QUOTE_THRESHOLD, MaxiFarmError::InsufficientRealQuoteReserves);
    }
    
    pool_state.complete = true;
    // Emit CompleteEvent
    emit!(CompleteEvent {
        base_mint: pool_state.base_mint, 
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

// CreatePool context
#[derive(Accounts)]
pub struct AForceComplete<'info> {
    #[account()]
    pub owner: Signer<'info>, // Pool owner

    #[account(
        mut,
        seeds = [
            PoolState::PREFIX_SEED,
            base_mint.key().as_ref(),
        ],
        bump,
        has_one = owner
    )]
    pub pool_state: Box<Account<'info, PoolState>>, // (New) PoolState account

    #[account(address = pool_state.base_mint)]
    pub base_mint: Box<InterfaceAccount<'info, Mint>> // Token mint address
}
