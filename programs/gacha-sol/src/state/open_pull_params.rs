use anchor_lang::prelude::*;

use super::AE_CIPHERTEXT_MAX_BASE64_LEN;

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct OpenPullParams {
    pub pull_id: u64,
    pub amount: u64,
    pub decimals: u8,
    pub new_decryptable_available_balance: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
}
