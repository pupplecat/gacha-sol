use anyhow::Result;
use gacha_sol::{instruction, state::AE_CIPHERTEXT_MAX_BASE64_LEN};
use solana_sdk::signer::Signer as _;
use spl_token_2022::{
    extension::confidential_transfer::account_info::ApplyPendingBalanceAccountInfo,
    solana_zk_sdk::encryption::pod::auth_encryption::PodAeCiphertext, ui_amount_to_amount,
};

use crate::test_utils::{
    gacha_sol_test_environment::GachaSolTestEnvironment,
    proof_account::{ProofAccount, SignerProofAccount},
};

#[tokio::test]
async fn test_apply_pull_pending_balance() -> Result<()> {
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

    // ------- start verify pull ---------

    let authority_pubkey = env.authority.pubkey();

    let new_decryptable_available_balance = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();

        let confidential_transfer_account = test_fixtures
            .get_token_account_credential_transfer_account(&reward_vault_pubkey)
            .await?;

        let apply_pending_balance_account_info =
            ApplyPendingBalanceAccountInfo::new(&confidential_transfer_account);

        let new_decryptable_available_balance = apply_pending_balance_account_info
            .new_decryptable_available_balance(
                &pull_proof_account.get_pod_elgamal_keypair()?.secret(),
                &pull_proof_account.get_ae_key()?.try_into()?,
            )?;

        PodAeCiphertext::from(new_decryptable_available_balance)
    };

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

    let ix = instruction::ApplyPullPendingBalance::populate(
        authority_pubkey,
        pull_id,
        decryptable_new_decryptable_available_balance_array,
    );

    let tx = env
        .process_instruction(ix, &vec![&env.authority], Some(&env.payer))
        .await?;

    println!("apply pull pending balance tx: {}", tx);

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
    assert_eq!(pull_available_balance, expected_amount);

    Ok(())
}
