use std::str::FromStr;

use anyhow::Result;
use gacha_sol::instruction;
use solana_sdk::{signature::Keypair, signer::Signer as _};
use spl_token_2022::{
    extension::{
        confidential_transfer::{
            instruction::{ZeroCiphertextProofData, ZkProofData},
            ConfidentialTransferAccount,
        },
        BaseStateWithExtensions, StateWithExtensions,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
    state::Account as Token2022Account,
    ui_amount_to_amount,
};
use spl_token_confidential_transfer_ciphertext_arithmetic::subtract;

use crate::test_utils::{
    confidential_transfer::{
        create_close_context_state_ixs, get_zk_proof_context_state_account_creation_instructions,
    },
    gacha_sol_test_environment::GachaSolTestEnvironment,
    proof_account::{ProofAccount, SignerProofAccount},
};

#[tokio::test]
async fn test_verify_pull() -> Result<()> {
    let env = GachaSolTestEnvironment::new().await?;

    let pull_price = 100_000_000;
    env.initialize_game_config(pull_price).await?;
    let pull_id = env.get_game_config().await?.last_pull_id + 1;

    let pull_proof_account = SignerProofAccount::new();

    let expected_amount = ui_amount_to_amount(100.0, env.decimals);
    env.create_pull(pull_id, pull_proof_account.clone(), expected_amount)
        .await?;

    let reward_mint_pubkey = env.reward_mint_pubkey();
    let token_account_proof_account = SignerProofAccount::new();
    let token_account_pubkey = token_account_proof_account.pubkey();
    let pull_pubkey = env.pull_pubkey(pull_id);
    let reward_vault_pubkey = env.reward_vault_pubkey(pull_pubkey);

    print!("xxx s-create_ct_token_account");
    env.create_ct_token_account(
        &reward_mint_pubkey,
        &env.authority,
        token_account_proof_account.clone(),
    )
    .await?;

    // {
    //     let mut test_fixtures = env.test_fixtures.lock().unwrap();
    //     let account = test_fixtures
    //         .program_simulator
    //         .get_account(token_account_pubkey)
    //         .await?;

    //     let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
    //         .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

    //     // 3) grab the ConfidentialTransferAccount extension
    //     let ext = state
    //         .get_extension::<ConfidentialTransferAccount>()
    //         .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

    //     println!("xxx ext.{}", ext.available_balance)
    // }
    println!("xxx e-create_ct_token_account: {}", token_account_pubkey);

    let mint_amount = ui_amount_to_amount(100_000.0, env.decimals);
    env.mint_reward_token(&token_account_pubkey, mint_amount)
        .await?;

    env.deposit_reward(&token_account_pubkey, &env.authority, mint_amount)
        .await?;

    env.apply_pending_balance(token_account_proof_account.clone(), &env.authority)
        .await?;

    let available_balance = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();
        test_fixtures
            .get_token_account_decrypted_decryptable_available_balance(&token_account_proof_account)
            .await?
    };

    println!("xxx available_balance: {}", available_balance); // 100_000__000_000_000

    env.ct_transfer_reward_token(
        token_account_proof_account.clone(),
        &env.authority,
        &reward_vault_pubkey,
        expected_amount,
    )
    .await?;

    let available_balance = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();
        test_fixtures
            .get_token_account_decrypted_decryptable_available_balance(&token_account_proof_account)
            .await?
    };

    println!("xxx available_balance: {}", available_balance); // 99_900__000_000_000

    env.apply_pull_pending_balance(pull_id, pull_proof_account.clone())
        .await?;

    let pull_available_balance = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();
        test_fixtures
            .get_token_account_decrypted_decryptable_available_balance_with_pubkey(
                &reward_vault_pubkey,
                &pull_proof_account.clone(),
            )
            .await?
    };

    println!("xxx pull_available_balance: {}", pull_available_balance); // 100__000_000_000

    // ------- start verify pull ---------

    let authority_pubkey = env.authority.pubkey();
    let pull = env.get_pull(pull_id).await?;
    let payer_pubkey = env.payer.pubkey();

    let ciphertext_available_balance = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();

        let confidential_transfer_account = test_fixtures
            .get_token_account_credential_transfer_account(&reward_vault_pubkey)
            .await?;

        confidential_transfer_account.available_balance
    };

    let ciphertext_expected_amount = {
        let expected_amount = PodElGamalCiphertext::from_str(
            std::str::from_utf8(&pull.encrypted_amount)
                .map_err(|_| anyhow::anyhow!("ciphertext expected amount conversion failed"))?,
        )
        .map_err(|_| anyhow::anyhow!("ciphertext expected amount conversion failed"))?;

        expected_amount
    };

    let zero_ciphertext =
        subtract(&ciphertext_available_balance, &ciphertext_expected_amount).unwrap();

    let zero_proof_data = ZeroCiphertextProofData::new(
        &pull_proof_account.get_pod_elgamal_keypair()?,
        &zero_ciphertext.try_into()?,
    )?;

    zero_proof_data.verify_proof()?;

    let zero_ciphertext_proof_context_state_account = Keypair::new();
    let zero_ciphertext_proof_pubkey = zero_ciphertext_proof_context_state_account.pubkey();

    // Zero Ciphertext Proof Instructions------------------------------------------------------------------------------
    let (zero_proof_create_ix, zero_proof_verify_ix) =
        get_zk_proof_context_state_account_creation_instructions(
            &payer_pubkey,
            &zero_ciphertext_proof_pubkey,
            &authority_pubkey,
            &zero_proof_data,
        )?;

    let proof_accounts_ixs = [zero_proof_create_ix, zero_proof_verify_ix];

    let proof_account_tx = env
        .process_instructions(
            &proof_accounts_ixs,
            &vec![
                // &env.authority,
                &zero_ciphertext_proof_context_state_account,
            ],
            Some(&env.payer),
        )
        .await?;

    println!("proof accounts tx: {}", proof_account_tx);

    let ix =
        instruction::VerifyPull::populate(authority_pubkey, zero_ciphertext_proof_pubkey, pull_id);

    let tx = env
        .process_instructions(&[ix], &vec![&env.authority], None)
        .await?;

    println!("verify pull tx: {}", tx);

    let pull = env.get_pull(pull_id).await?;

    assert_eq!(pull.verified, true);

    // Close context states Instructions ---------------------------------------------------------------

    let close_context_state_ixs = create_close_context_state_ixs(
        &[zero_ciphertext_proof_pubkey],
        &authority_pubkey,
        &payer_pubkey,
    );

    let close_context_state_tx = env
        .process_instructions(&close_context_state_ixs, &vec![&env.authority], None)
        .await?;

    println!("close context accounts tx: {}", close_context_state_tx);

    Ok(())
}
