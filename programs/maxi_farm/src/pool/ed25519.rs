use anchor_lang::prelude::*;
use solana_program::{
    instruction::Instruction,
    ed25519_program::ID as ED25519_ID
};
use std::convert::TryInto;
use crate::{
    constants::*,
    error::*
};


pub fn merge_values(array1: &[u8], array2: &[u8]/* , array3: &[u8], array4: &[u8], array5: &[u8], array6: &[u8] */) -> Vec<u8> {
    // Create a new Vec<u8> with enough capacity
    let mut merged = Vec::with_capacity(array1.len() + array2.len()/*  + array3.len() + array4.len() + array5.len() + array6.len() */);
    
    // Extend the vector with both arrays
    merged.extend_from_slice(array1);
    merged.extend_from_slice(array2);
    // merged.extend_from_slice(array3);
    // merged.extend_from_slice(array4);
    // merged.extend_from_slice(array5);
    // merged.extend_from_slice(array6);
    
    merged
}

/// Verify Ed25519Program instruction fields
pub fn verify_ed25519_ix(ix: &Instruction, pubkey: &[u8], msg: &[u8], sig: &[u8]) -> Result<()> {
    msg!("ix.program_id: {}", ix.program_id);
    msg!("ix.accounts.len(): {}", ix.accounts.len());
    msg!("ix.data.len(): {}", ix.data.len());

    require!(
        ix.program_id == ED25519_ID
            && ix.accounts.len() == 0
            && ix.data.len() == 16 + 32 + 64 + msg.len(),
        MaxiFarmError::InvalidMessageFormat
    );

    // If that's not the case, check data
    check_ed25519_data(&ix.data, pubkey, msg, sig)
}

/// Verify serialized Ed25519Program instruction data
fn check_ed25519_data(data: &[u8], pubkey: &[u8], msg: &[u8], sig: &[u8]) -> Result<()> {
    // According to this layout used by the Ed25519Program
    // https://github.com/solana-labs/solana-web3.js/blob/master/src/ed25519-program.ts#L33

    require!(data.len() > PUBKEY_OFFSET + PUBKEY_LEN + SIG_LEN, MaxiFarmError::TooShortDataLen);
    require!(pubkey.len() == PUBKEY_LEN, MaxiFarmError::InvalidPubkeyLen);
    require!(sig.len() == SIG_LEN, MaxiFarmError::InvalidSigLen);

    let data_pubkey = &data[PUBKEY_OFFSET..PUBKEY_OFFSET + PUBKEY_LEN]; // Bytes 16..16+32
    let data_sig = &data[PUBKEY_OFFSET + PUBKEY_LEN..PUBKEY_OFFSET + PUBKEY_LEN + SIG_LEN]; // Bytes 48..48+64
    let data_msg = &data[PUBKEY_OFFSET + PUBKEY_LEN + SIG_LEN..]; // Bytes 112..end

    // Arguments
    require!(
        data_pubkey == pubkey && data_msg == msg && data_sig == sig,
        MaxiFarmError::WrongSignatureParams
    );

    // "Deserializing" byte slices
    let num_signatures = &[data[0]]; // Byte  0
    let padding = &[data[1]]; // Byte  1
    let signature_offset = &data[2..=3]; // Bytes 2,3
    let signature_instruction_index = &data[4..=5]; // Bytes 4,5
    let public_key_offset = &data[6..=7]; // Bytes 6,7
    let public_key_instruction_index = &data[8..=9]; // Bytes 8,9
    let message_data_offset = &data[10..=11]; // Bytes 10,11
    let message_data_size = &data[12..=13]; // Bytes 12,13
    let message_instruction_index = &data[14..=15]; // Bytes 14,15

    // Expected values
    let exp_public_key_offset: u16 = PUBKEY_OFFSET as u16; // 2*u8 + 7*u16
    let exp_signature_offset: u16 = exp_public_key_offset + pubkey.len() as u16;
    let exp_message_data_offset: u16 = exp_signature_offset + sig.len() as u16;
    let exp_num_signatures: u8 = 1;
    let exp_message_data_size: u16 = msg.len().try_into().unwrap();

    // Header and Arg Checks
    msg!("Signature offset: {:?}", signature_offset);
    msg!("Expected signature offset: {:?}", exp_signature_offset);
    msg!("Public key offset: {:?}", public_key_offset);
    msg!("Expected public key offset: {:?}", exp_public_key_offset);
    msg!("Message data offset: {:?}", message_data_offset);
    msg!("Expected message data offset: {:?}", exp_message_data_offset);

    // Header
    require!(
        num_signatures == &exp_num_signatures.to_le_bytes()
            && padding == &[0]
            && signature_offset == &exp_signature_offset.to_le_bytes()
            && signature_instruction_index == &u16::MAX.to_le_bytes()
            && public_key_offset == &exp_public_key_offset.to_le_bytes()
            && public_key_instruction_index == &u16::MAX.to_le_bytes()
            && message_data_offset == &exp_message_data_offset.to_le_bytes()
            && message_data_size == &exp_message_data_size.to_le_bytes()
            && message_instruction_index == &u16::MAX.to_le_bytes(),
        MaxiFarmError::SigVerificationFailed
    );

    Ok(())
}
