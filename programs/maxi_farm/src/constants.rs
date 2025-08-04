use std::str::FromStr;
use anchor_lang::solana_program::pubkey::Pubkey;

pub const NATIVE_MINT_2022_STR: &'static str = "9pan9bMn5HatX4EJdBwg9VgCa7Uz5HL8N1m5D3NdXejP"; // WSOL mint address

pub const FEE_PRE_DIV: u128 = 1000; // 1000 for 1%
pub const BPS: u64 = 100; // 100 for 1%

pub const MAX_TAX: u64 = 50 * FEE_PRE_DIV as u64; // 50%
pub const MAX_FEE_BPS: u64 = 100; // 1%
pub const MAX_PRIV_SALE_PERIOD: u64 = 24 * 60 * 60; // 1d
pub const MAX_TRADING_FEE: u64 = 5 * FEE_PRE_DIV as u64; // 5%

pub const DEF_PRIV_SALE_PERIOD: u64 = /* 30 */ 0 * 60; // /* 30 */ 0 min
pub const DEF_TIER1_REWARD: u64 = 25 * BPS; // 25%
pub const DEF_TIER2_REWARD: u64 =  7 * BPS / 2; // 3.5%
pub const DEF_TIER3_REWARD: u64 = 3 * BPS; // 3%

pub const NORMAL_REAL_QUOTE_THRESHOLD: u64 = 82_000_000_000; // 82 SOL

pub const PUBKEY_OFFSET: usize = 2 * 1 + 7 * 2;
pub const PUBKEY_LEN: usize = 32;
pub const SIG_LEN: usize = 64;
