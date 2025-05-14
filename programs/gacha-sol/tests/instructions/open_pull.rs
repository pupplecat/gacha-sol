use anyhow::Result;
use gacha_sol::{instruction, state::AE_CIPHERTEXT_MAX_BASE64_LEN};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_2022::{
    extension::confidential_transfer::account_info::WithdrawAccountInfo,
    solana_zk_sdk::encryption::pod::auth_encryption::PodAeCiphertext, ui_amount_to_amount,
};
use spl_token_confidential_transfer_proof_generation::withdraw::{
    withdraw_proof_data, WithdrawProofData,
};

use crate::test_utils::{
    confidential_transfer::get_zk_proof_context_state_account_creation_instructions,
    gacha_sol_test_environment::GachaSolTestEnvironment,
    proof_account::{ProofAccount, SignerProofAccount},
};

#[tokio::test]
async fn test_open_pull() -> Result<()> {
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

    env.create_ct_token_account(
        &reward_mint_pubkey,
        &env.authority,
        token_account_proof_account.clone(),
    )
    .await?;

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

    env.verify_pull(pull_id, pull_proof_account.clone()).await?;

    let pull = env.get_pull(pull_id).await?;

    assert!(pull.verified);

    // === start buy pull

    let purchase_mint_pubkey = env.purchase_mint_pubkey();
    let buyer = Keypair::new();
    let buyer_pubkey = buyer.pubkey();
    let buyer_purchase_token_account = env.create_ata(&purchase_mint_pubkey, &buyer_pubkey).await?;

    let mint_amount = 200_000_000_000;

    env.mint_purchase_token(&buyer_purchase_token_account, mint_amount)
        .await?;

    env.buy_pull(&buyer, &buyer_purchase_token_account, pull_id)
        .await?;

    let pull = env.get_pull(pull_id).await?;

    assert_eq!(pull.buyer, buyer_pubkey);

    // === open pull

    let reward_mint_pubkey = env.reward_mint_pubkey();
    let buyer_reward_token_account = env.create_ata(&reward_mint_pubkey, &buyer_pubkey).await?;

    let (encrypted_available_balance, new_decryptable_available_balance, current_balance) = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();

        let confidential_transfer_account = test_fixtures
            .get_token_account_credential_transfer_account(&reward_vault_pubkey)
            .await?;

        let available_balance = pull_proof_account
            .decrypt_supply(&confidential_transfer_account.decryptable_available_balance)?;

        let withdraw_account_info = WithdrawAccountInfo::new(&confidential_transfer_account);

        let new_decryptable_available_balance = withdraw_account_info
            .new_decryptable_available_balance(
                available_balance,
                &pull_proof_account.get_ae_key()?.try_into()?,
            )?;

        (
            confidential_transfer_account.available_balance,
            PodAeCiphertext::from(new_decryptable_available_balance),
            available_balance,
        )
    };

    let WithdrawProofData {
        equality_proof_data,
        range_proof_data,
    } = withdraw_proof_data(
        &encrypted_available_balance.try_into()?,
        current_balance,
        current_balance,
        &pull_proof_account.get_pod_elgamal_keypair()?,
    )?;

    // Create w proofs ------------------------------------------------------

    // Generate address for equality proof account
    let equality_proof_context_state_account = Keypair::new();
    let equality_proof_pubkey = equality_proof_context_state_account.pubkey();

    // Generate address for range proof account
    let range_proof_context_state_account = Keypair::new();
    let range_proof_pubkey = range_proof_context_state_account.pubkey();

    let payer_pubkey = env.payer.pubkey();
    let authority_pubkey = env.authority.pubkey();

    // Range Proof Instructions------------------------------------------------------------------------------
    let (range_create_ix, range_verify_ix) =
        get_zk_proof_context_state_account_creation_instructions(
            &payer_pubkey,
            &range_proof_context_state_account.pubkey(),
            &authority_pubkey,
            &range_proof_data,
        )?;

    // Equality Proof Instructions---------------------------------------------------------------------------
    let (equality_create_ix, equality_verify_ix) =
        get_zk_proof_context_state_account_creation_instructions(
            &payer_pubkey,
            &equality_proof_context_state_account.pubkey(),
            &authority_pubkey,
            &equality_proof_data,
        )?;

    let proof_accounts_ixs = [
        range_create_ix,
        equality_create_ix,
        range_verify_ix,
        equality_verify_ix,
    ];

    let proof_account_tx = env
        .process_instructions(
            &proof_accounts_ixs,
            &vec![
                &range_proof_context_state_account,
                &equality_proof_context_state_account,
            ],
            Some(&env.payer),
        )
        .await?;

    println!("proof accounts tx: {}", proof_account_tx);

    let decryptable_new_decryptable_available_balance_array = {
        let new_decryptable_available_balance_base64 =
            new_decryptable_available_balance.to_string();
        let new_decryptable_available_balance_bytes =
            new_decryptable_available_balance_base64.as_bytes();
        if new_decryptable_available_balance_bytes.len() != AE_CIPHERTEXT_MAX_BASE64_LEN {
            return Err(anyhow::anyhow!(
                "Invalid zero balance length: expected {}, got {}",
                AE_CIPHERTEXT_MAX_BASE64_LEN,
                new_decryptable_available_balance_bytes.len()
            ));
        }
        let mut decryptable_new_decryptable_available_balance_array =
            [0u8; AE_CIPHERTEXT_MAX_BASE64_LEN];
        decryptable_new_decryptable_available_balance_array
            .copy_from_slice(new_decryptable_available_balance_bytes);
        decryptable_new_decryptable_available_balance_array
    };

    let ix = instruction::OpenPull::populate(
        buyer_pubkey,
        buyer_reward_token_account,
        reward_mint_pubkey,
        equality_proof_pubkey,
        range_proof_pubkey,
        authority_pubkey,
        pull_id,
        current_balance,
        env.decimals,
        decryptable_new_decryptable_available_balance_array,
    );

    let tx = env
        .process_instruction(ix, &vec![&env.authority], Some(&env.payer))
        .await?;

    println!("open pull tx: {}", tx);

    let pull = env.get_pull(pull_id).await?;

    assert_eq!(pull.claimed, true);
    assert_eq!(pull.revealed_amount, current_balance);

    Ok(())
}
