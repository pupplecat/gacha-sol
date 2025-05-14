use anyhow::Result;
use solana_sdk::{signature::Keypair, signer::Signer as _};
use spl_token_2022::{
    extension::{
        confidential_transfer::{instruction::deposit, ConfidentialTransferAccount},
        BaseStateWithExtensions, StateWithExtensions,
    },
    instruction::mint_to,
    state::Account,
};

use crate::test_utils::{
    confidential_transfer::{
        create_confidential_token_account_ixs, create_confidential_transfer_mint_ixs,
        token_2022_program_id,
    },
    gacha_sol_test_environment::GachaSolTestEnvironment,
    proof_account::{ProofAccount, SignerProofAccount},
};

#[tokio::test]
async fn test_basic_flow() -> Result<()> {
    let env = GachaSolTestEnvironment::new().await?;

    let mint = Keypair::new();
    let mint_authority = Keypair::new();
    let user = Keypair::new();
    let token_account_proof_account = SignerProofAccount::new();

    let payer_pubkey = env.payer.pubkey();
    let mint_pubkey = mint.pubkey();
    let mint_authority_pubkey = mint_authority.pubkey();
    let user_pubkey = user.pubkey();
    let token_account_pubkey = token_account_proof_account.pubkey();

    let ixs = create_confidential_transfer_mint_ixs(
        &payer_pubkey,
        &mint_pubkey,
        &mint_authority_pubkey,
        None,
        None,
        9,
    )?;

    let tx = env
        .process_instructions(&ixs, &vec![&mint, &env.payer], None)
        .await?;

    println!("xxx create mint tx: {}", tx);

    let ixs = create_confidential_token_account_ixs(
        &payer_pubkey,
        &user_pubkey,
        &mint_pubkey,
        &token_account_pubkey,
        &token_account_proof_account.get_ae_key()?,
        &token_account_proof_account.get_pod_elgamal_keypair()?,
    )?;

    let tx = env
        .process_instructions(
            &ixs,
            &vec![&token_account_proof_account.keypair, &env.payer, &user],
            None,
        )
        .await?;

    println!("xxx create ata tx: {}", tx);

    let amount = 100_000_000_000;

    let ix = mint_to(
        &token_2022_program_id(),
        &mint_pubkey,
        &token_account_pubkey,
        &mint_authority_pubkey,
        &[],
        amount,
    )?;

    let tx = env
        .process_instruction(ix, &vec![&mint_authority], Some(&env.payer))
        .await?;

    println!("xxx mint_to tx: {}", tx);

    let (balance, pending_lo, pending_hi) = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();

        let account = test_fixtures
            .program_simulator
            .get_account(token_account_pubkey)
            .await?;

        let state = StateWithExtensions::<Account>::unpack(&account.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        (
            state.base.amount,
            ext.pending_balance_lo,
            ext.pending_balance_hi,
        )
    };

    println!(
        "xxx balance {}, pending_lo: {}, pending_hi: {}",
        balance, pending_lo, pending_hi
    );

    let ix = deposit(
        &token_2022_program_id(),
        &token_account_pubkey,
        &mint_pubkey,
        amount,
        9,
        &user_pubkey,
        &[],
    )?;

    let tx = env
        .process_instruction(ix, &vec![&user], Some(&env.payer))
        .await?;

    println!("xxx deposit tx: {}", tx);

    let (balance, pending_lo, pending_hi) = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();

        let account = test_fixtures
            .program_simulator
            .get_account(token_account_pubkey)
            .await?;

        let state = StateWithExtensions::<Account>::unpack(&account.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        (
            state.base.amount,
            ext.pending_balance_lo,
            ext.pending_balance_hi,
        )
    };

    println!(
        "xxx balance {}, pending_lo: {}, pending_hi: {}",
        balance, pending_lo, pending_hi
    );

    Ok(())
}
