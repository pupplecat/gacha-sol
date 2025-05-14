use anyhow::Result;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_2022::{
    extension::confidential_transfer::{
        self,
        instruction::{
            BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU128Data,
            CiphertextCommitmentEqualityProofData,
        },
        DecryptableBalance,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;

use super::token_2022_program_id;

pub fn confidential_transfer_ixs(
    owner_pubkey: &Pubkey,
    source_token_account_pubkey: &Pubkey,
    destination_token_account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    new_source_decryptable_available_balance: &DecryptableBalance,
    transfer_amount_auditor_ciphertext_lo: &PodElGamalCiphertext,
    transfer_amount_auditor_ciphertext_hi: &PodElGamalCiphertext,
    equality_proof_data_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    ciphertext_validity_proof_data_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    range_proof_data_location: ProofLocation<BatchedRangeProofU128Data>,
) -> Result<Vec<Instruction>> {
    let ixs = confidential_transfer::instruction::transfer(
        &token_2022_program_id(),
        source_token_account_pubkey,
        mint_pubkey,
        destination_token_account_pubkey,
        new_source_decryptable_available_balance,
        transfer_amount_auditor_ciphertext_lo,
        transfer_amount_auditor_ciphertext_hi,
        owner_pubkey,
        &[],
        equality_proof_data_location,
        ciphertext_validity_proof_data_location,
        range_proof_data_location,
    )?;

    Ok(ixs)
}
