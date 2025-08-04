use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
    associated_token::AssociatedToken
};
use solana_program::{
    instruction::Instruction,
    ed25519_program::ID as ED25519_ID,
    sysvar::instructions::{load_instruction_at_checked, ID as IX_ID}
};
use crate::{
    constants::BPS,
    error::MaxiFarmError,
    utils::{calculate_fee, close_token_account, sync_native_amount, check_balance, transfer_lamports},
    ed25519::{verify_ed25519_ix, merge_values},
    MainState, PoolState, ReferralState,
    TradeEvent
};

// This function sells tokens on the bonding curve
// Params
//   ctx - Sell context
//   base_amount - Amount of tokens to sell
//   min_quote_amount - Minimum amount of SOL to receive
// Return
//   Ok on success, ErrorCode on failure
pub fn sell(ctx:Context<ASell>, base_amount: u64, min_quote_amount: u64, tax_bps: u64, sig: Option<Vec<u8>>) -> Result<()> {
    // base_amount must be greater than 0 and less than real_base_reserves
    require!(base_amount.gt(&0), MaxiFarmError::WrongBaseAmount);

    let main_state = &mut ctx.accounts.main_state;

    let pool_state = &mut ctx.accounts.pool_state;
    require!(pool_state.complete.eq(&false), MaxiFarmError::BondingCurveComplete); // BondingCurve must not be complete

    let cur_timestamp = Clock::get()?.unix_timestamp as u64;
    if cur_timestamp < pool_state.created_time + pool_state.priv_sale_period {  // If not elapsed priv. sale period
        if let Some(sig) = sig.clone() {
            let mut ix: Instruction = load_instruction_at_checked(0, &ctx.accounts.ix_sysvar.to_account_info())?;
            // if registerUser instruction was added at first, load next
            if ix.program_id != ED25519_ID {
                ix = load_instruction_at_checked(1, &ctx.accounts.ix_sysvar.to_account_info())?;
            }
            let signer = main_state.signer;
            let msg = pool_state.base_mint.key();
    
            // Check that ix is what we expect to have sent
            verify_ed25519_ix(&ix, signer.as_ref(), &msg.as_ref(), &sig)?;
    
            msg!("Signature is valid!");
        } else {
            require!(false, MaxiFarmError::MissingSignature);
        }
    }

    let seller = ctx.accounts.seller.to_account_info();
    let seller_base_ata = &ctx.accounts.seller_base_ata;
    let token_program = ctx.accounts.token_program.to_account_info();
    
    let tax_fee = calculate_fee(tax_bps, base_amount);
    let input_base_amount = base_amount.checked_sub(tax_fee).unwrap();
    let mut _output_amount = pool_state.compute_receivable_amount_on_sell(input_base_amount);
    
    let mut fee = calculate_fee(main_state.trading_fee, _output_amount);
    let mut output_amount = _output_amount - fee;

    require!(output_amount >= min_quote_amount, MaxiFarmError::TooLowOuputSol);

    pool_state.real_base_reserves += input_base_amount; // Increase Real Tokens
    if _output_amount > pool_state.real_quote_reserves {
        _output_amount = pool_state.real_quote_reserves;
        fee = calculate_fee(main_state.trading_fee, _output_amount);
        output_amount = _output_amount - fee;
    }
    pool_state.real_quote_reserves -= _output_amount; // Decrease Real SOL

    // Transfer (meme) tokens from seller to pool
    let orginal_amount = ctx.accounts.reserver_base_ata.amount;
    let input_amount_transfer_cpi_account = TransferChecked {
        from: seller_base_ata.to_account_info(),
        mint: ctx.accounts.base_mint.to_account_info(),
        to: ctx.accounts.reserver_base_ata.to_account_info(),
        authority: seller.clone()
    };
    token_2022::transfer_checked(
        CpiContext::new(token_program.clone(), input_amount_transfer_cpi_account), 
        base_amount,
        ctx.accounts.base_mint.decimals
    )?;
    ctx.accounts.reserver_base_ata.reload()?;
    let after_amount = ctx.accounts.reserver_base_ata.amount;
    require!(after_amount - orginal_amount == input_base_amount, MaxiFarmError::InvalidTax);
    
    if ctx.accounts.tier1_referral.is_some() {
        // Transfer fee (SOL) from pool to feeRecipient
        transfer_lamports(&pool_state.to_account_info(), &ctx.accounts.fee_recipient, fee * (100 * BPS - main_state.tier1_reward) / (100 * BPS))?;
        // Transfer reward fee (SOL) from pool to main_state
        transfer_lamports(&pool_state.to_account_info(), &main_state.to_account_info(), fee * main_state.tier1_reward / (100 * BPS))?;
    } else {
        // Transfer fee (SOL) from pool to feeRecipient
        transfer_lamports(&pool_state.to_account_info(), &ctx.accounts.fee_recipient, fee)?;
    }
    // Transfer output_amount (SOL) from pool to seller
    transfer_lamports(&pool_state.to_account_info(), &seller, output_amount)?;

    let mut tier1_referrer = Pubkey::default();
    let mut tier1_reward = 0;
    let mut tier2_referrer = Pubkey::default();
    let mut tier2_reward = 0;
    let mut tier3_referrer = Pubkey::default();
    let mut tier3_reward = 0;
    
    if let Some(tier3_referral) = &mut ctx.accounts.tier3_referral {
        tier3_referrer = tier3_referral.user.clone();
        tier3_reward = fee * main_state.tier3_reward / (100 * BPS);
        tier3_referral.earned_rewards += tier3_reward;

        if let Some(tier2_referral) = &mut ctx.accounts.tier2_referral { // If tier2 referrer exists, divide rewards
            tier2_referrer = tier2_referral.user.clone();
            tier2_reward = fee * main_state.tier2_reward / (100 * BPS);
            tier2_referral.earned_rewards += tier2_reward;
        }
        if let Some(tier1_referral) = &mut ctx.accounts.tier1_referral {
            tier1_referrer = tier1_referral.user.clone();
            tier1_reward = fee * (main_state.tier1_reward - main_state.tier2_reward - main_state.tier3_reward) / (100 * BPS);
            tier1_referral.earned_rewards += tier1_reward;
        }
    } else if let Some(tier2_referral) = &mut ctx.accounts.tier2_referral { // If tier2 referrer exists, divide rewards
        tier2_referrer = tier2_referral.user.clone();
        tier2_reward = fee * main_state.tier2_reward / (100 * BPS);
        tier2_referral.earned_rewards += tier2_reward;

        if let Some(tier1_referral) = &mut ctx.accounts.tier1_referral {
            tier1_referrer = tier1_referral.user.clone();
            tier1_reward = fee * (main_state.tier1_reward - main_state.tier2_reward) / (100 * BPS);
            tier1_referral.earned_rewards += tier1_reward;
        }
    } else {
        if let Some(tier1_referral) = &mut ctx.accounts.tier1_referral {
            tier1_referrer = tier1_referral.user.clone();
            tier1_reward = fee * main_state.tier1_reward / (100 * BPS);
            tier1_referral.earned_rewards += tier1_reward;
        }
    }

    // Emit (Sell) TradeEvent
    emit!(TradeEvent {
        user: seller.key(), 
        base_mint: pool_state.base_mint, 
        token_amount: input_base_amount, 
        sol_amount: output_amount, 
        base_reserves: pool_state.real_base_reserves + pool_state.virt_base_reserves, 
        quote_reserves: pool_state.virt_quote_reserves + pool_state.real_quote_reserves, 
        is_buy: false, 
        timestamp: Clock::get()?.unix_timestamp,
        tier1_referrer: tier1_referrer,
        tier1_reward: tier1_reward,
        tier2_referrer: tier2_referrer,
        tier2_reward: tier2_reward,
        tier3_referrer: tier3_referrer,
        tier3_reward: tier3_reward
    });
    emit_cpi!(TradeEvent {
        user: seller.key(), 
        base_mint: pool_state.base_mint, 
        token_amount: input_base_amount, 
        sol_amount: output_amount, 
        base_reserves: pool_state.real_base_reserves + pool_state.virt_base_reserves, 
        quote_reserves: pool_state.virt_quote_reserves + pool_state.real_quote_reserves, 
        is_buy: false, 
        timestamp: Clock::get()?.unix_timestamp,
        tier1_referrer: tier1_referrer,
        tier1_reward: tier1_reward,
        tier2_referrer: tier2_referrer,
        tier2_reward: tier2_reward,
        tier3_referrer: tier3_referrer,
        tier3_reward: tier3_reward
    });

    Ok(())
}

// Sell context
#[event_cpi]
#[derive(Accounts)]
#[instruction(base_amount: u64)]
pub struct ASell<'info> {
    #[account(mut)]
    pub seller: Signer<'info>, // Seller
    
    #[account(
        mut,
        seeds = [MainState::PREFIX_SEED],
        bump
    )]
    pub main_state: Box<Account<'info, MainState>>, // MainState account

    #[account(
        mut,
        address = main_state.fee_recipient
    )]
    /// CHECK: this should be set by owner
    pub fee_recipient: AccountInfo<'info>, // FeeRecipient

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
        associated_token::authority = seller,
        associated_token::token_program = token_program,
        constraint = check_balance(seller_base_ata.as_ref(), base_amount) @ MaxiFarmError::InsufficientFund
    )]
    pub seller_base_ata: Box<InterfaceAccount<'info, TokenAccount>>, // Seller's Token ATA

    #[account(
        mut,
        associated_token::mint = base_mint,
        associated_token::authority = pool_state,
        associated_token::token_program = token_program
    )]
    pub reserver_base_ata: Box<InterfaceAccount<'info, TokenAccount>>, // PoolState's Token ATA

    #[account(mut)]
    pub tier1_referral: Option<Box<Account<'info, ReferralState>>>,
    #[account(mut)]
    pub tier2_referral: Option<Box<Account<'info, ReferralState>>>,
    #[account(mut)]
    pub tier3_referral: Option<Box<Account<'info, ReferralState>>>,

    /// CHECK: this should be checked by owner
    #[account(address = IX_ID)]
    pub ix_sysvar: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>
}
