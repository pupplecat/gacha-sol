use anchor_lang::prelude::*;

use super::{Size, AE_CIPHERTEXT_MAX_BASE64_LEN, ELGAMAL_PUBKEY_MAX_BASE64_LEN};

#[account]
pub struct Pull {
    pub id: u64,
    pub reward_vault: Pubkey,
    pub encrypted_amount: [u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN],
    pub buyer: Pubkey,
    pub verified: bool,
    pub claimed: bool,
    pub revealed_amount: u64,
    pub transfer_amount_auditor_ciphertext_lo: [u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN],
    pub transfer_amount_auditor_ciphertext_hi: [u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN],
    pub final_decryptable_available_balance: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    pub equality_proof_account: Pubkey,
    pub ciphertext_validity_proof_account: Pubkey,
    pub range_proof_account: Pubkey,
    pub bump: u8,
}

impl Size for Pull {
    const SIZE: usize = 8       // discriminator
        + 8                    // id
        + 32                    // reward_vault
        + 64                    // encrypted_amount
        + 32                    // buyer
        + 1                    // verified
        + 1                    // claimed
        + 8                    // revealed_amount
        + 1                    // bump
        ;
}

impl Pull {
    pub fn get_signer_seeds<'a, 'b: 'a>(&'b self) -> [&'a [u8]; 3] {
        [
            b"pull",
            bytemuck::bytes_of(&self.id),
            std::slice::from_ref(&self.bump),
        ]
    }
}
