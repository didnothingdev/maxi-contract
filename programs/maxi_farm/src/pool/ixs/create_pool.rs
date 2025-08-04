use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, SyncNative, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
    associated_token::AssociatedToken
};
use std::str::FromStr;
use crate::{
    constants::{NATIVE_MINT_2022_STR, FEE_PRE_DIV, BPS, MAX_TAX, MAX_FEE_BPS, MAX_PRIV_SALE_PERIOD, DEF_PRIV_SALE_PERIOD},
    error::MaxiFarmError,
    utils::{check_balance, sync_native_amount, calculate_fee},
    MainState, PoolState,
    CreateEvent
};

/*** Note: Here, 'pool' means 'bonding curve' - they've got the same meaning ***/

// This function creates a new pool
// Params
//   ctx - CreatePool context
//   base_amount - Token amount to put in the bonding curve
// Return
//   Ok on success, ErrorCode on Failure
//     CreateEvent is emitted on success
pub fn create_pool(ctx: Context<ACreatePool>, metadata_uri:String, tax_bps: u64, max_fee_tokens: u64, real_quote_threshold: u64, coin_type: u8, opt_priv_sale_period: Option<u64>) -> Result<()> {
    // input parameters check
    require!(
        tax_bps.le(&MAX_TAX),
        MaxiFarmError::InvalidTax
    );
    require!(real_quote_threshold > 0, MaxiFarmError::InvalidRealQuoteThreshold);

    let base_amount: u64 = ctx.accounts.reserver_base_ata.amount;
    require!(base_amount.eq(&ctx.accounts.base_mint.supply), MaxiFarmError::WrongBaseAmountOnCreation);
    require!(
        max_fee_tokens.le(&(base_amount * MAX_FEE_BPS / (100 * BPS))),
        MaxiFarmError::InvalidMaxFeeTokens
    );
    require!(&ctx.accounts.base_mint.mint_authority.is_some().eq(&false), MaxiFarmError::BaseTokenMustNotBeMintable);
    require!(&ctx.accounts.base_mint.freeze_authority.is_some().eq(&false), MaxiFarmError::BaseTokenMustNotBeFreezable);

    if let Some(priv_sale_period) = opt_priv_sale_period {
        require!(
            priv_sale_period.gt(&(0 as u64)) && priv_sale_period.le(&MAX_PRIV_SALE_PERIOD),
            MaxiFarmError::InvalidPrivSalePeriod
        );
    }

    let main_state = &mut ctx.accounts.main_state;
    let pool_state = &mut ctx.accounts.pool_state;
    let creator = ctx.accounts.creator.to_account_info();
    let token_program = ctx.accounts.token_program.to_account_info();
    let cur_timestamp = Clock::get()?.unix_timestamp as u64;
    
    // Initialize all members of pool_state
    pool_state.owner = creator.key(); // Creator's address
    pool_state.tax = tax_bps; // Transfer tax
    pool_state.max_fee_tokens = max_fee_tokens;
    pool_state.base_mint = ctx.accounts.base_mint.key(); // Token mint address
    pool_state.real_base_reserves = base_amount; // Total supply of tokens is all put into the pool (except tax)
    pool_state.virt_base_reserves = base_amount.checked_div(15).unwrap(); // Initial virtual token reserves
    pool_state.real_quote_reserves = 0; // 0 SOL
    pool_state.virt_quote_reserves = real_quote_threshold.checked_div(3).unwrap(); // Initial virtual SOL reserves
    pool_state.real_quote_threshold = real_quote_threshold; // Real SOL threshold
    pool_state.created_time = cur_timestamp;
    if let Some(priv_sale_period) = opt_priv_sale_period {
        pool_state.priv_sale_period = priv_sale_period;
    } else {
        pool_state.priv_sale_period = DEF_PRIV_SALE_PERIOD;
    }
    pool_state.complete = false;

    // Emit createPool event
    emit!(CreateEvent {
        creator: pool_state.owner, 
        tax: pool_state.tax, 
        max_fee_tokens: pool_state.max_fee_tokens, 
        base_mint: pool_state.base_mint, 
        metadata_uri: metadata_uri.clone(), 
        total_supply: base_amount, 
        real_quote_threshold: real_quote_threshold, 
        base_reserves: pool_state.real_base_reserves + pool_state.virt_base_reserves, // Token reserves is the sum of real token reserves and virtual token reserves
        quote_reserves: pool_state.virt_quote_reserves, // SOL reserves is equal to virtual SOL reserves
        priv_sale_period: pool_state.priv_sale_period,
        timestamp: cur_timestamp as i64,
        coin_type: coin_type
    });
    emit_cpi!(CreateEvent {
        creator: pool_state.owner, 
        tax: pool_state.tax, 
        max_fee_tokens: pool_state.max_fee_tokens, 
        base_mint: pool_state.base_mint, 
        metadata_uri: metadata_uri, 
        total_supply: base_amount, 
        real_quote_threshold: real_quote_threshold, 
        base_reserves: pool_state.real_base_reserves + pool_state.virt_base_reserves, // Token reserves is the sum of real token reserves and virtual token reserves
        quote_reserves: pool_state.virt_quote_reserves, // SOL reserves is equal to virtual SOL reserves
        priv_sale_period: pool_state.priv_sale_period,
        timestamp: cur_timestamp as i64,
        coin_type: coin_type
    });

    Ok(())
}

// CreatePool context
#[event_cpi]
#[derive(Accounts)]
pub struct ACreatePool<'info> {
    #[account(mut)]
    pub creator: Signer<'info>, // Pool creator

    #[account(
        seeds = [MainState::PREFIX_SEED],
        bump
    )]
    pub main_state: Box<Account<'info, MainState>>, // MainState account
    
    #[account(
        init,
        payer = creator,
        seeds = [
            PoolState::PREFIX_SEED,
            base_mint.key().as_ref(),
        ],
        bump,
        space = 8 + PoolState::MAX_SIZE
    )]
    pub pool_state: Box<Account<'info, PoolState>>, // (New) PoolState account

    #[account(
        constraint = base_mint.key().to_string() != NATIVE_MINT_2022_STR @ MaxiFarmError::InvalidTokenPair
    )]
    pub base_mint: Box<InterfaceAccount<'info, Mint>>, // Token mint address

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = pool_state,
        associated_token::token_program = token_program
    )]
    pub reserver_base_ata: Box<InterfaceAccount<'info, TokenAccount>>, // PoolState's Token ATA

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>
}
