use anchor_lang::prelude::*;
use solana_program::{
    system_instruction,
    program::invoke,
};
use crate::{
    error::MaxiFarmError,
    utils::transfer_lamports,
    MainState, ReferralState,
    RewardsClaimEvent,
};

pub fn claim_rewards(ctx: Context<AClaimRewards>) -> Result<()> {
    let user = ctx.accounts.user.to_account_info();
    let main_state = ctx.accounts.main_state.to_account_info();
    let referral_account = &mut ctx.accounts.referral_account;
    let reward_amount = referral_account.earned_rewards;

    require!(reward_amount > 0, MaxiFarmError::NoRewardsAvailable);

    // Transfer earned_rewards (SOL) from main_state to user
    transfer_lamports(&main_state, &user, reward_amount)?;

    // Reset earned rewards
    referral_account.earned_rewards = 0;

    emit!(RewardsClaimEvent {
        user: user.key(),
        rewards: reward_amount,
        timestamp: Clock::get()?.unix_timestamp
    });

    Ok(())
}

#[derive(Accounts)]
pub struct AClaimRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [MainState::PREFIX_SEED],
        bump
    )]
    pub main_state: Box<Account<'info, MainState>>, // MainState account
    
    #[account(mut)]
    pub referral_account: Account<'info, ReferralState>,
    
    pub system_program: Program<'info, System>
}
