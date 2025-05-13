use anyhow::Result;
use solana_sdk::{
    instruction::Instruction, program_pack::Pack as _, pubkey::Pubkey, rent::Rent,
    system_instruction::create_account,
};
use spl_token_2022::{
    error::TokenError,
    extension::{
        confidential_transfer::instruction::{
            configure_account, PubkeyValidityProofData, ZeroCiphertextProofData,
        },
        ExtensionType,
    },
    instruction::{initialize_account3, reallocate},
    solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
    state::Account,
};
use spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation};

use super::token_2022_program_id;

pub fn create_confidential_token_account_ixs(
    payer_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    token_account_pubkey: &Pubkey,
    token_account_authority_ae_key: &AeKey,
    token_account_authority_elgamal_keypair: &ElGamalKeypair,
) -> Result<Vec<Instruction>> {
    // Instruction to create associated token account
    // 2) allocate & fund the mint account (must be signed by the mint keypair)
    let ix_create_account = create_account(
        payer_pubkey,
        token_account_pubkey,
        Rent::default().minimum_balance(Account::LEN),
        Account::LEN as u64,
        &token_2022_program_id(),
    );

    // 3) initialize the Token‑2022 mint (no freeze authority)
    let ix_init_account = initialize_account3(
        &token_2022_program_id(),
        token_account_pubkey,
        mint_pubkey,
        owner_pubkey,
    )?;

    // Instruction to reallocate the token account to include the `ConfidentialTransferAccount` extension
    let reallocate_instruction = reallocate(
        &spl_token_2022::id(),
        &token_account_pubkey,
        payer_pubkey,
        &owner_pubkey,
        &[],
        &[ExtensionType::ConfidentialTransferAccount],
    )?;

    // The maximum number of `Deposit` and `Transfer` instructions that can
    // credit `pending_balance` before the `ApplyPendingBalance` instruction is executed
    let maximum_pending_balance_credit_counter = 65536;

    // Initial token balance is 0
    let decryptable_balance = token_account_authority_ae_key.encrypt(0);

    // The instruction data that is needed for the `ProofInstruction::VerifyPubkeyValidity` instruction.
    // It includes the cryptographic proof as well as the context data information needed to verify the proof.
    // Generating the proof data client-side (instead of using a separate proof account)
    let proof_data = PubkeyValidityProofData::new(&token_account_authority_elgamal_keypair)
        .map_err(|_| TokenError::ProofGeneration)?;

    // `InstructionOffset` indicates that proof is included in the same transaction
    // This means that the proof instruction offset must be always be 1.
    let proof_location = ProofLocation::InstructionOffset(
        1.try_into().unwrap(),
        ProofData::InstructionData(&proof_data),
    );

    // Instructions to configure the token account, including the proof instruction
    // Appends the `VerifyPubkeyValidityProof` instruction right after the `ConfigureAccount` instruction.
    let configure_account_instructions = configure_account(
        &spl_token_2022::id(),                  // Program ID
        &token_account_pubkey,                  // Token account
        mint_pubkey,                            // Mint
        &decryptable_balance.into(),            // Initial balance
        maximum_pending_balance_credit_counter, // Maximum pending balance credit counter
        &owner_pubkey,
        &[],            // Additional signers
        proof_location, // Proof location
    )
    .unwrap();

    // Instructions to configure account must come after `initialize_account` instruction
    let mut instructions = vec![ix_create_account, ix_init_account, reallocate_instruction];
    instructions.extend(configure_account_instructions);

    Ok(instructions)
}
