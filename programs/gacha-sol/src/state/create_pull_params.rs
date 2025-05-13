use anchor_lang::prelude::*;

use super::{AE_CIPHERTEXT_MAX_BASE64_LEN, ELGAMAL_CIPHERTEXT_LEN};

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct CreatePullParams {
    pub pull_id: u64,
    pub encrypted_amount: [u8; ELGAMAL_CIPHERTEXT_LEN],
    pub decryptable_zero_balance_base64: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
}
