#![allow(unused)]

use anchor_lang::prelude::*;

pub mod main_state;
pub mod pool;
pub mod referral;

pub mod constants;
pub mod error;
pub mod utils;

use main_state::*;
use pool::*;
use referral::*;

declare_id!("maxi7YSyG2Fpuh9hNzjojuV7woB9PTV4fRwZZ523ayu");

#[program]
pub mod maxi_farm {
    use super::*;

    pub fn init_main_state(ctx: Context<AInitMainState>, signer: Pubkey) -> Result<()> {
        main_state::init_main_state(ctx, signer)
    }

    pub fn transfer_ownership(ctx: Context<ATransferOwnership>, new_owner: Pubkey) -> Result<()> {
        main_state::transfer_ownership(ctx, new_owner)
    }
    
    pub fn update_main_state(ctx: Context<AUpdateMainState>, input: UpdateMainStateInput) -> Result<()> {
        main_state::update_main_state(ctx, input)
    }

    
    pub fn create_pool(ctx: Context<ACreatePool>, metadata_uri: String, tax_bps: u64, max_fee_tokens: u64, real_quote_threshold: u64, coin_type: u8, priv_sale_period: Option<u64>) -> Result<()> {
        pool::create_pool(ctx, metadata_uri, tax_bps, max_fee_tokens, real_quote_threshold, coin_type, priv_sale_period)
    }

    pub fn buy_tokens_from_exact_sol(ctx: Context<ABuy>, quote_amount: u64, min_base_amount: u64, tax_bps: u64, sig: Option<Vec<u8>>) -> Result<()> {
        pool::buy_tokens_from_exact_sol(ctx, quote_amount, min_base_amount, tax_bps, sig)
    }

    pub fn buy_exact_tokens_from_sol(ctx: Context<ABuy>, base_amount: u64, max_quote_amount: u64, tax_bps: u64, sig: Option<Vec<u8>>) -> Result<()> {
        pool::buy_exact_tokens_from_sol(ctx, base_amount, max_quote_amount, tax_bps, sig)
    }

    pub fn sell(ctx: Context<ASell>, amount: u64, min_sol_output: u64, tax_bps: u64, sig: Option<Vec<u8>>) -> Result<()> {
        pool::sell(ctx, amount, min_sol_output, tax_bps, sig)
    }
    
    pub fn update_tax(ctx: Context<AUpdateTax>, new_tax: u64) -> Result<()> {
        pool::update_tax(ctx, new_tax)
    }
    
    pub fn force_complete(ctx: Context<AForceComplete>) -> Result<()> {
        pool::force_complete(ctx)
    }


    pub fn register_user(ctx: Context<ARegisterUser>, referrer: Option<Pubkey>) -> Result<()> {
        referral::register_user(ctx, referrer)
    }

    pub fn claim_rewards(ctx: Context<AClaimRewards>) -> Result<()> {
        referral::claim_rewards(ctx)
    }

    
    pub fn withdraw(ctx: Context<AWithdraw>) -> Result<()> {
        pool::withdraw(ctx)
    }
}
