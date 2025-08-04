use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{Token2022},
    token_interface::{Mint, TokenAccount},
    associated_token::AssociatedToken
};
use std::str::FromStr;
use crate::{
    constants::{MAX_TRADING_FEE},
    error::MaxiFarmError,
    MainState,
    MainStateUpdated
};

// MainState update parameters
#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone, Copy)]
pub struct UpdateMainStateInput {
    signer: Pubkey,         // New signer
    withdrawer: Pubkey,     // New withdrawer
    trading_fee: u64,       // New trading fee
    fee_recipient: Pubkey   // New fee recipient
}

// This function updates main state
// Params
//   ctx - MainStatate update context
//   input - MainState update parameters
// Return
//   Ok on success, ErrorCode on failure
pub fn update_main_state(
    ctx: Context<AUpdateMainState>,
    input: UpdateMainStateInput
) -> Result<()> {
    require!(
        input.trading_fee.le(&MAX_TRADING_FEE),
        MaxiFarmError::InvalidTradingFee
    );
    
    let main_state = &mut ctx.accounts.main_state;

    // Update new members
    main_state.signer = input.signer;
    main_state.withdrawer = input.withdrawer;
    
    main_state.trading_fee = input.trading_fee;
    main_state.fee_recipient = input.fee_recipient;
    
    emit!(MainStateUpdated {
        signer: input.signer,
        withdrawer: input.withdrawer,
        
        trading_fee: input.trading_fee,
        fee_recipient: input.fee_recipient
    });
    
    Ok(())
}

// MainState update context - passed with accounts
#[derive(Accounts)]
#[instruction(input: UpdateMainStateInput)]
pub struct AUpdateMainState<'info> {
    #[account(mut)]
    pub owner: Signer<'info>, // Current owner
    
    #[account(
        mut,
        seeds = [MainState::PREFIX_SEED],
        bump,
        has_one = owner
    )]
    pub main_state: Box<Account<'info, MainState>> // MainState account with new values
}
