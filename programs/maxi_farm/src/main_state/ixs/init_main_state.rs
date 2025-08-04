use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{Token2022},
    token_interface::{Mint, TokenAccount},
    associated_token::AssociatedToken
};
use std::str::FromStr;
use crate::{
    constants::{FEE_PRE_DIV, DEF_TIER1_REWARD, DEF_TIER2_REWARD, DEF_TIER3_REWARD},
    error::MaxiFarmError,
    MainState,
    MainStateInitialized
};

// This function initializes main state
// Params
//   ctx - MainState initialization context
// Return
//   Ok on success, ErrorCode on failure
pub fn init_main_state(ctx: Context<AInitMainState>, signer: Pubkey) -> Result<()> {
    let state = &mut ctx.accounts.main_state;

    // Initialize all members
    state.owner = ctx.accounts.owner.key();
    state.signer = signer;
    state.withdrawer = ctx.accounts.owner.key();
    
    state.trading_fee = (FEE_PRE_DIV / 2) as u64; // 0.5%
    state.fee_recipient = ctx.accounts.owner.key();

    state.tier1_reward = DEF_TIER1_REWARD;
    state.tier2_reward = DEF_TIER2_REWARD;
    state.tier3_reward = DEF_TIER3_REWARD;
    
    emit!(MainStateInitialized {
        owner: state.owner,
        signer: state.signer,
        withdrawer: state.withdrawer,
        
        trading_fee: state.trading_fee,
        fee_recipient: state.fee_recipient
    });

    Ok(())
}

// MainState initialization struct - passed with accounts
#[derive(Accounts)]
pub struct AInitMainState<'info> {
    #[account(mut)]
    pub owner: Signer<'info>, // Program owner
    
    #[account(
        init,
        payer = owner,
        seeds = [MainState::PREFIX_SEED],
        bump,
        space = 8 + MainState::MAX_SIZE
    )]
    pub main_state: Box<Account<'info, MainState>>, // MainState account

    pub system_program: Program<'info, System>
}
