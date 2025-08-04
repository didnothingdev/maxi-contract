use anchor_lang::prelude::*;
use crate::{
    error::MaxiFarmError,
    ReferralState,
    UserRegisteredEvent,
};

pub fn register_user(
    ctx: Context<ARegisterUser>,
    referrer: Option<Pubkey> // Optional referrer
) -> Result<()> {
    let referral_account = &mut ctx.accounts.referral_account;

    referral_account.user = ctx.accounts.user.key();

    if let Some(referrer_key) = referrer {
        referral_account.referrer = referrer_key;

        if let Some(referrer_account) = &mut ctx.accounts.referrer_account {
            if referrer_account.user == Pubkey::default() {
                referrer_account.user = referrer_key;
            }
        }
    }

    emit!(UserRegisteredEvent {
        referree: referral_account.user.clone(),
        referrer: referrer.unwrap(),
        timestamp: Clock::get()?.unix_timestamp
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(referrer: Option<Pubkey>)]
pub struct ARegisterUser<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        init_if_needed,
        payer = user,
        seeds = [ReferralState::PREFIX_SEED, user.key().as_ref()],
        bump,
        space = 8 + ReferralState::MAX_SIZE
    )]
    pub referral_account: Account<'info, ReferralState>,

    #[account(
        init_if_needed,
        payer = user,
        seeds = [ReferralState::PREFIX_SEED, referrer.unwrap().key().as_ref()],
        bump,
        space = 8 + ReferralState::MAX_SIZE
    )]
    pub referrer_account: Option<Account<'info, ReferralState>>,
    
    pub system_program: Program<'info, System>
}
