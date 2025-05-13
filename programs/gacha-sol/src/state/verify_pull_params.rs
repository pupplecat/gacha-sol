use anchor_lang::prelude::*;

use super::AE_CIPHERTEXT_MAX_BASE64_LEN;

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct VerifyPullParams {
    pub new_decryptable_available_balance: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
}
