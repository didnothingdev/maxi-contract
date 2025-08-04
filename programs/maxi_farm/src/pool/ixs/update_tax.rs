use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{Mint}
};
use crate::{
    constants::{MAX_TAX},
    error::MaxiFarmError,
    PoolState,
    TaxUpdatedEvent
};

// This function updates tax of BondingCurve token.
// Params
//   ctx - UpdateTax context
// Return
//   Ok on success, ErrorCode on Failure
//     UpdateTax event is emitted on success
pub fn update_tax(ctx: Context<AUpdateTax>, new_tax: u64) -> Result<()> {
    // input parameters check
    require!(
        new_tax.le(&MAX_TAX),
        MaxiFarmError::InvalidTax
    );

    let owner = ctx.accounts.owner.to_account_info();
    let pool_state = &mut ctx.accounts.pool_state;
    pool_state.tax = new_tax;
        
    // Emit CompleteEvent
    emit!(TaxUpdatedEvent {
        owner: owner.key(),
        tax: new_tax
    });

    Ok(())
}

// CreatePool context
#[derive(Accounts)]
pub struct AUpdateTax<'info> {
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
