use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
    associated_token::AssociatedToken
};
use solana_program::{
    system_instruction,
    instruction::Instruction,
    ed25519_program::ID as ED25519_ID,
    program::invoke,
    sysvar::instructions::{load_instruction_at_checked, ID as IX_ID}
};
use crate::{
    constants::{FEE_PRE_DIV, BPS},
    error::MaxiFarmError,
    utils::{calculate_fee, calculate_total_amount, close_token_account, sync_native_amount},
    ed25519::{verify_ed25519_ix, merge_values},
    MainState, PoolState, ReferralState,
    TradeEvent, CompleteEvent
};

// Internal buy function
// Params
//   ctx - Buy context
//   base_amount - Amount of tokens to buy
//   fee - Trading fee
//   input_quote_amount - Amount of SOL to buy with (fee excluded)
// Return
//   Ok on success
//     If successful, emits (Buy) TradeEvent
//       And if reaches complete marketCap, emits CompleteEvent as well
fn buy_finalize(ctx: Context<ABuy>, base_amount: u64, tax: u64, fee: u64, input_quote_amount: u64) -> Result<()> {
    let main_state = &mut ctx.accounts.main_state;
    let pool_state = &mut ctx.accounts.pool_state;
    let buyer = ctx.accounts.buyer.to_account_info();
    let buyer_base_ata = &ctx.accounts.buyer_base_ata;
    let token_program = ctx.accounts.token_program.to_account_info();

    if ctx.accounts.tier1_referral.is_some() { // If referral is valid, divide the fee
        // Transfer fee (SOL) from buyer to feeRecpient
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &main_state.fee_recipient.key(),
                fee * (100 * BPS - main_state.tier1_reward) / (100 * BPS)
            ),
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.fee_recipient.to_account_info(),
                ctx.accounts.system_program.to_account_info()
            ]
        )?;

        // Transfer reward fee (SOL) from buyer to main_state
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &main_state.key(),
                fee * main_state.tier1_reward / (100 * BPS)
            ),
            &[
                ctx.accounts.buyer.to_account_info(),
                main_state.to_account_info(),
                ctx.accounts.system_program.to_account_info()
            ]
        )?;
    } else {
        // Transfer fee (SOL) from buyer to feeRecpient
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &main_state.fee_recipient.key(),
                fee
            ),
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.fee_recipient.to_account_info(),
                ctx.accounts.system_program.to_account_info()
            ]
        )?;
    }
    
    // Transfer input_quote_amount (SOL) from buyer to pool
    invoke(
        &system_instruction::transfer(
            &ctx.accounts.buyer.key(),
            &pool_state.key(),
            input_quote_amount
        ),
        &[
            ctx.accounts.buyer.to_account_info(),
            pool_state.to_account_info(),
            ctx.accounts.system_program.to_account_info()
        ]
    )?;

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
    
    // Transfer (meme) tokens from pool to buyer
    let original_amount = ctx.accounts.buyer_base_ata.amount;
    let output_amount_transfer_cpi_account = TransferChecked {
        from: ctx.accounts.reserver_base_ata.to_account_info(),
        mint: ctx.accounts.base_mint.to_account_info(),
        to: buyer_base_ata.to_account_info(),
        authority: pool_state.to_account_info()
    };
    token_2022::transfer_checked(
        CpiContext::new_with_signer(
            token_program.clone(), 
            output_amount_transfer_cpi_account, 
            &[&[
                PoolState::PREFIX_SEED,
                pool_state.base_mint.as_ref(),
                &[ctx.bumps.pool_state]
            ]]
        ),
        base_amount,
        ctx.accounts.base_mint.decimals
    )?;
    ctx.accounts.buyer_base_ata.reload()?;
    let after_amount = ctx.accounts.buyer_base_ata.amount;
    require!(after_amount - original_amount == base_amount - tax, MaxiFarmError::InvalidTax);

    // Emit (Buy) TradeEvent
    emit!(TradeEvent {
        user: buyer.key(), 
        base_mint: pool_state.base_mint, 
        token_amount: base_amount, 
        sol_amount: fee + input_quote_amount, 
        base_reserves: pool_state.real_base_reserves + pool_state.virt_base_reserves, 
        quote_reserves: pool_state.virt_quote_reserves + pool_state.real_quote_reserves, 
        is_buy: true, 
        timestamp: Clock::get()?.unix_timestamp,
        tier1_referrer: tier1_referrer,
        tier1_reward: tier1_reward,
        tier2_referrer: tier2_referrer,
        tier2_reward: tier2_reward,
        tier3_referrer: tier3_referrer,
        tier3_reward: tier3_reward
    });
    emit_cpi!(TradeEvent {
        user: buyer.key(), 
        base_mint: pool_state.base_mint, 
        token_amount: base_amount, 
        sol_amount: fee + input_quote_amount, 
        base_reserves: pool_state.real_base_reserves + pool_state.virt_base_reserves, 
        quote_reserves: pool_state.virt_quote_reserves + pool_state.real_quote_reserves, 
        is_buy: true, 
        timestamp: Clock::get()?.unix_timestamp,
        tier1_referrer: tier1_referrer,
        tier1_reward: tier1_reward,
        tier2_referrer: tier2_referrer,
        tier2_reward: tier2_reward,
        tier3_referrer: tier3_referrer,
        tier3_reward: tier3_reward
    });

    // Check if bonding curve becomes complete
    if (pool_state.real_quote_reserves >= pool_state.real_quote_threshold) {
        pool_state.complete = true;
        
        // Emit CompleteEvent
        emit!(CompleteEvent {
            base_mint: pool_state.base_mint, 
            timestamp: Clock::get()?.unix_timestamp,
        });
    }

    Ok(())
}

// This function buys tokens on the bonding curve, with specified amount of SOL
// Params
//   ctx - Buy context
//   quote_amount - Amount of SOL to buy tokens with
//   min_base_amount - Minimum amount of tokens to receive
// Return
//   Ok on success, ErrorCode on failure
pub fn buy_tokens_from_exact_sol(ctx:Context<ABuy>, quote_amount: u64, min_base_amount: u64, tax_bps: u64, sig: Option<Vec<u8>>) -> Result<()> {
    require!(quote_amount.gt(&0), MaxiFarmError::WrongQuoteAmount); // quote_amount must be greater than 0

    let main_state = &ctx.accounts.main_state;
    
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

    let buyer = ctx.accounts.buyer.to_account_info();
    let buyer_base_ata = &ctx.accounts.buyer_base_ata;
    let token_program = ctx.accounts.token_program.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();

    // if new real_quote_reserves exceeds threshold, restrict quote_amount
    let mut _quote_amount = quote_amount;
    let mut fee = calculate_fee(main_state.trading_fee, _quote_amount);
    if (pool_state.real_quote_reserves + (_quote_amount - fee) > pool_state.real_quote_threshold) {
        _quote_amount = calculate_total_amount(main_state.trading_fee, pool_state.real_quote_threshold - pool_state.real_quote_reserves);
        fee = calculate_fee(main_state.trading_fee, _quote_amount);
    }
    
    let input_quote_amount = _quote_amount - fee;
    let output_base_amount = pool_state.compute_receivable_amount_on_buy(input_quote_amount);
    let tax_fee = calculate_fee(tax_bps, output_base_amount);
    require!(output_base_amount.checked_sub(tax_fee).unwrap() >= min_base_amount, MaxiFarmError::TooFewOutputTokens); // Check minimum amount

    pool_state.real_quote_reserves += input_quote_amount; // Increase Real SOL
    pool_state.real_base_reserves -= output_base_amount; // Decrease Real tokens

    buy_finalize(ctx, output_base_amount, tax_fee, fee, input_quote_amount)
}

// This function buys specified amount tokens on the bonding curve (required SOL amount is calculated internally)
// Params
//   ctx - Buy context
//   base_amount - Amount of tokens to buy
//   max_quote_amount - Maximum amount of SOL allowed to spend
// Return
//   Ok on success, ErrorCode on failure
pub fn buy_exact_tokens_from_sol(ctx:Context<ABuy>, base_amount: u64, max_quote_amount: u64, tax_bps: u64, sig: Option<Vec<u8>>) -> Result<()> {
    let main_state = &ctx.accounts.main_state;
    
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

    let total_base_amount = calculate_total_amount(tax_bps, base_amount);

    // base_amount must be greater than 0 and less than real_base_reserves
    require!(total_base_amount.gt(&0) && total_base_amount.le(&pool_state.real_base_reserves), MaxiFarmError::WrongBaseAmount);

    let buyer = ctx.accounts.buyer.to_account_info();
    let buyer_base_ata = &ctx.accounts.buyer_base_ata;
    let token_program = ctx.accounts.token_program.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();

    let mut input_base_amount = total_base_amount;
    let mut input_quote_amount = pool_state.compute_required_amount_on_buy(input_base_amount);
    // if new real_quote_reserves exceeds threshold, restrict quote_amount
    if (pool_state.real_quote_reserves + input_quote_amount > pool_state.real_quote_threshold) {
        input_quote_amount = pool_state.real_quote_threshold - pool_state.real_quote_reserves;
        input_base_amount = pool_state.compute_receivable_amount_on_buy(input_quote_amount);
    }
    
    let total_quote_amount = calculate_total_amount(main_state.trading_fee, input_quote_amount);
    let fee = calculate_fee(main_state.trading_fee, total_quote_amount);
    require!(total_quote_amount <= max_quote_amount, MaxiFarmError::TooMuchInputSol);

    pool_state.real_base_reserves -= input_base_amount; // Decrease Real Tokens
    pool_state.real_quote_reserves += input_quote_amount; // Increase Real SOL
    
    let tax_fee = calculate_fee(tax_bps, input_base_amount);
    buy_finalize(ctx, input_base_amount, tax_fee, fee, input_quote_amount)
}


// Buy context
#[event_cpi]
#[derive(Accounts)]
pub struct ABuy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>, // Buyer
    
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
        init_if_needed,
        payer = buyer,
        associated_token::mint = base_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program
    )]
    pub buyer_base_ata: Box<InterfaceAccount<'info, TokenAccount>>, // Buyer's Token ATA

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
