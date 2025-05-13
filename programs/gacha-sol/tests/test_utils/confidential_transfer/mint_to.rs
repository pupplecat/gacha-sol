use anyhow::Result;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_2022::{
    extension::{
        confidential_mint_burn::instruction::confidential_mint_with_split_proofs,
        confidential_transfer::{
            instruction::{
                BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU128Data,
                CiphertextCommitmentEqualityProofData,
            },
            DecryptableBalance,
        },
    },
    instruction::mint_to,
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;

use crate::test_utils::pda::get_ata2022_pubkey;

use super::token_2022_program_id;

pub fn public_mint_to_ix(
    mint_authority_pubkey: &Pubkey,
    wallet_owner_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    mint_amount: u64,
) -> Result<Instruction> {
    let token_account_pubkey = get_ata2022_pubkey(wallet_owner_pubkey, mint_pubkey);

    let mint_to_instruction = mint_to(
        &spl_token_2022::id(),
        mint_pubkey,
        &token_account_pubkey,
        mint_authority_pubkey,
        &[mint_authority_pubkey],
        mint_amount,
    )?;

    Ok(mint_to_instruction)
}

/// Build the CPI(s) needed to mint `mint_amount` _confidentially_ into
/// the recipient’s Token-2022 ATA, all in one go.
///
/// # Parameters
///
/// - `token_program_id`: The SPL‑Token‑2022 program id (usually `spl_token_2022::id()`)
/// - `mint_pubkey`:      The Confidential‑Mint token account (PDA) you initialized
/// - `mint_authority`:   The key (or PDA) with minting rights for this mint
/// - `recipient_ata`:    The recipient’s Token-2022 associated token account
/// - `new_supply_bytes`: Ciphertext of the _new_ encrypted total supply (after this mint)
/// - `amount_lo_bytes`:  ElGamal ciphertext limb “low” for the mint amount
/// - `amount_hi_bytes`:  ElGamal ciphertext limb “high” for the mint amount
/// - `eq_proof_account`: Pre‑verified equality proof account (on‑chain)
/// - `val_proof_account`:Pre‑verified ciphertext‑validity proof account (on‑chain)
/// - `range_proof_account`: Pre‑verified range‑proof account (on‑chain)
///
/// # Returns
///
/// One or more `Instruction`s that you can pack into a `Transaction` and
/// sign with the `mint_authority`.
pub fn confidential_mint_to_ixs(
    mint_pubkey: &Pubkey,
    mint_authority: &Pubkey,
    recipient_token_account_pubkey: &Pubkey,
    new_decryptable_supply: &DecryptableBalance,
    amount_ciphertext_lo: &PodElGamalCiphertext,
    amount_ciphertext_hi: &PodElGamalCiphertext,
    equality_proof_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    ciphertext_validity_proof_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    range_proof_location: ProofLocation<BatchedRangeProofU128Data>,
) -> Result<Vec<Instruction>> {
    // Build the “confidential mint” CPI(s)
    let ixs = confidential_mint_with_split_proofs(
        &token_2022_program_id(),
        recipient_token_account_pubkey, // you’re minting _into_ this Token‑2022 account
        mint_pubkey,                    // the mint (with CT‑Mint‑Burn enabled)
        &amount_ciphertext_lo,          // auditor ciphertext low limb
        &amount_ciphertext_hi,          // auditor ciphertext high limb
        mint_authority,                 // must sign the transaction
        &[],                            // no multisig
        equality_proof_location,
        ciphertext_validity_proof_location,
        range_proof_location,
        &new_decryptable_supply,
    )?;

    Ok(ixs)
}
