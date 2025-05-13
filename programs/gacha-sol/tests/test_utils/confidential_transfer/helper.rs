use anyhow::Result;
use bytemuck::try_from_bytes;
use solana_sdk::pubkey::Pubkey;
use spl_token_2022::{
    extension::confidential_transfer::DecryptableBalance,
    solana_zk_sdk::encryption::AE_CIPHERTEXT_LEN, ID,
};

pub fn token_2022_program_id() -> Pubkey {
    ID
}

pub fn zk_elgamal_proof_program_id() -> Pubkey {
    spl_token_2022::solana_zk_sdk::zk_elgamal_proof_program::ID
}

/// Casts the raw byte array into DecryptableBalance
pub fn unpack_balance(bytes: &[u8; AE_CIPHERTEXT_LEN]) -> Result<DecryptableBalance> {
    let decryptable_balance = try_from_bytes::<DecryptableBalance>(bytes)
        .map_err(|_| anyhow::anyhow!("unpack balance error"))?;

    Ok(*decryptable_balance)
}
