use anyhow::Result;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, rent::Rent, system_instruction::create_account,
};
use spl_token_2022::{
    extension::{confidential_mint_burn, ExtensionType},
    instruction::initialize_mint2,
    solana_zk_sdk::encryption::{auth_encryption::AeKey, pod::elgamal::PodElGamalPubkey},
    state::Mint,
};
use spl_token_client::token::ExtensionInitializationParams;

use super::token_2022_program_id;

pub fn create_mint_with_confidential_extensions_ixs(
    payer: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    mint_ae_key: &AeKey,
    supply_elgamal_pubkey: &PodElGamalPubkey,
    auditor_elgamal_pubkey: Option<PodElGamalPubkey>,
    decimals: u8,
) -> Result<Vec<Instruction>> {
    // 1) compute space & rent
    let space = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::ConfidentialTransferMint,
        ExtensionType::ConfidentialMintBurn,
    ])?;

    let rent_lamports = Rent::default().minimum_balance(space);

    println!("xxx mint space {}, lamports {}", space, rent_lamports);

    // 2) allocate & fund the mint account (must be signed by the mint keypair)
    let ix_create_account = create_account(
        payer,
        mint,
        // Rent::default().minimum_balance(Mint::LEN),
        // Mint::LEN as u64,
        rent_lamports,
        space as u64,
        &token_2022_program_id(),
    );

    // 3) initialize the Token‑2022 mint (no freeze authority)
    let ix_init_mint = initialize_mint2(
        &token_2022_program_id(),
        mint,
        mint_authority,
        freeze_authority,
        decimals,
    )?;

    // 4) enable ConfidentialTransferMint extension
    let ix_ct_mint = ExtensionInitializationParams::ConfidentialTransferMint {
        authority: Some(*mint_authority),
        auto_approve_new_accounts: true,
        auditor_elgamal_pubkey,
    }
    .instruction(&token_2022_program_id(), mint)?;

    // 5) enable ConfidentialMintBurn extension (persist this AeKey off‑chain!)
    let decryptable_supply = mint_ae_key.encrypt(0);
    let ix_mb = confidential_mint_burn::instruction::initialize_mint(
        &token_2022_program_id(),
        mint,
        supply_elgamal_pubkey,
        &decryptable_supply.into(),
    )?;

    Ok(vec![ix_create_account, ix_ct_mint, ix_mb, ix_init_mint])
}

pub fn create_confidential_transfer_mint_ixs(
    payer: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    auditor_elgamal_pubkey: Option<PodElGamalPubkey>,
    decimals: u8,
) -> Result<Vec<Instruction>, anyhow::Error> {
    println!("xxx create_confidential_transfer_mint_ixs mint {}", mint);
    // 1) compute needed space & rent
    let space = ExtensionType::try_calculate_account_len::<Mint>(&[
        ExtensionType::ConfidentialTransferMint,
    ])?;
    let rent_lamports = Rent::default().minimum_balance(space);

    // 2) allocate & fund the mint account
    let ix_create_account = create_account(
        payer,
        mint,
        rent_lamports,
        space as u64,
        &token_2022_program_id(),
    );

    // 3) initialize the 2022‐style mint
    let ix_init_mint = initialize_mint2(
        &token_2022_program_id(),
        mint,
        mint_authority,
        freeze_authority,
        decimals,
    )?;

    // 4) enable the ConfidentialTransferMint extension
    let ix_ct = ExtensionInitializationParams::ConfidentialTransferMint {
        authority: Some(*mint_authority),
        auto_approve_new_accounts: true,
        auditor_elgamal_pubkey,
    }
    .instruction(&token_2022_program_id(), mint)?;

    Ok(vec![ix_create_account, ix_ct, ix_init_mint])
}
