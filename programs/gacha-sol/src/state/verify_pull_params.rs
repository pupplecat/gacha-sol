use anchor_lang::prelude::*;

use super::{AE_CIPHERTEXT_MAX_BASE64_LEN, ELGAMAL_PUBKEY_MAX_BASE64_LEN};

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct VerifyPullParams {
    pub transfer_amount_auditor_ciphertext_lo: [u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN],
    pub transfer_amount_auditor_ciphertext_hi: [u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN],
    pub final_decryptable_available_balance: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
}
