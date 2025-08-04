use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, CloseAccount, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
    associated_token::AssociatedToken
};
use std::str::FromStr;
use crate::{
    constants::{NORMAL_REAL_QUOTE_THRESHOLD},
    error::MaxiFarmError,
    utils::{close_token_account, transfer_lamports},
    MainState, PoolState,
    WithdrawEvent
};

// This function is called by withdrawer to withdraw all remaining tokens and deposited SOL from the bonding curve
// Params
//   ctx - Withdraw context
// Return
//   Ok on success, ErrorCode on failure
pub fn withdraw(ctx: Context<AWithdraw>) -> Result<()> {
    let withdrawer = ctx.accounts.withdrawer.to_account_info();
    let main_state = &ctx.accounts.main_state;
    let pool_state = &mut ctx.accounts.pool_state;

    require!(pool_state.complete.eq(&true), MaxiFarmError::BondingCurveIncomplete); // BondingCurve must be complete
    require!(pool_state.real_base_reserves.gt(&0) && pool_state.real_quote_reserves.gt(&0), MaxiFarmError::BondingCurveAlreadyWithdrawn);

    let withdrawer_base_ata = ctx.accounts.withdrawer_base_ata.to_account_info();
    let token_program = ctx.accounts.token_program.to_account_info();

    // Transfer (meme) tokens from pool to withdrawer
    let pool_base_transfer_cpi_account = TransferChecked {
        from: ctx.accounts.reserver_base_ata.to_account_info(),
        mint: ctx.accounts.base_mint.to_account_info(),
        to: withdrawer_base_ata.clone(),
        authority: pool_state.to_account_info()
    };
    token_2022::transfer_checked(
        CpiContext::new_with_signer(
            token_program.clone(), 
            pool_base_transfer_cpi_account, 
            &[&[
                PoolState::PREFIX_SEED,
                pool_state.base_mint.as_ref(),
                &[ctx.bumps.pool_state]
            ]]
        ),
        pool_state.real_base_reserves /* ctx.accounts.reserver_base_ata.amount */,
        ctx.accounts.base_mint.decimals
    )?;

    // Transfer SOL from pool to withdrawer
    transfer_lamports(&pool_state.to_account_info(), &ctx.accounts.withdrawer, pool_state.real_quote_reserves)?;

    // Emit WithdrawEvent
    emit!(WithdrawEvent {
        withdrawer: withdrawer.key(),
        base_mint: pool_state.base_mint,
        base_amount: pool_state.real_base_reserves /* ctx.accounts.reserver_base_ata.amount */,
        quote_amount: pool_state.real_quote_reserves,
        timestamp: Clock::get()?.unix_timestamp
    });
    emit_cpi!(WithdrawEvent {
        withdrawer: withdrawer.key(),
        base_mint: pool_state.base_mint,
        base_amount: pool_state.real_base_reserves /* ctx.accounts.reserver_base_ata.amount */,
        quote_amount: pool_state.real_quote_reserves,
        timestamp: Clock::get()?.unix_timestamp
    });

    pool_state.real_base_reserves = 0;
    pool_state.real_quote_reserves = 0;
    
    Ok(())
}

// Withdraw context
#[event_cpi]
#[derive(Accounts)]
pub struct AWithdraw<'info> {
    #[account(mut)]
    pub withdrawer: Signer<'info>, // Current withdrawer

    #[account(
        seeds = [MainState::PREFIX_SEED],
        bump,
        has_one = withdrawer
    )]
    pub main_state: Box<Account<'info, MainState>>, // MainState account

    #[account(
        mut,
        seeds = [
            PoolState::PREFIX_SEED,
            base_mint.key().as_ref()
        ],
        bump
    )]
    pub pool_state: Box<Account<'info, PoolState>>, // PoolState account
    
    #[account(address = pool_state.base_mint)]
    pub base_mint: Box<InterfaceAccount<'info, Mint>>, // Token account

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = pool_state,
        associated_token::token_program = token_program
    )]
    pub reserver_base_ata: Box<InterfaceAccount<'info, TokenAccount>>, // PoolState's Token ATA

    #[account(
        init_if_needed,
        payer = withdrawer,
        associated_token::mint = base_mint,
        associated_token::authority = withdrawer,
        associated_token::token_program = token_program
    )]
    pub withdrawer_base_ata: Box<InterfaceAccount<'info, TokenAccount>>, // Admin's Token ATA

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>
}
