use anchor_spl::token::{spl_token::state::Account, TokenAccount};
use anyhow::Result;
use gacha_sol::instruction;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_2022::ui_amount_to_amount;

use crate::test_utils::{
    gacha_sol_test_environment::GachaSolTestEnvironment,
    proof_account::{ProofAccount, SignerProofAccount},
};

#[tokio::test]
async fn test_buy_pull() -> Result<()> {
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
    let game_vault_pubkey = env.game_vault_pubkey();
    let buyer = Keypair::new();
    let buyer_pubkey = buyer.pubkey();
    let buyer_purchase_token_account = env.create_ata(&purchase_mint_pubkey, &buyer_pubkey).await?;

    let mint_amount = 200_000_000_000;

    env.mint_purchase_token(&buyer_purchase_token_account, mint_amount)
        .await?;

    let ix: solana_sdk::instruction::Instruction = instruction::BuyPull::populate(
        buyer_pubkey,
        buyer_purchase_token_account,
        env.game_vault_pubkey(),
        purchase_mint_pubkey,
        pull_id,
    );

    let tx = env
        .process_instruction(ix, &vec![&buyer], Some(&env.payer))
        .await?;

    println!("buy pull tx: {}", tx);

    let pull = env.get_pull(pull_id).await?;

    assert_eq!(pull.buyer, buyer_pubkey);

    let buyer_balance = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();
        let ta: Account = test_fixtures
            .program_simulator
            .get_packed_account_data(buyer_purchase_token_account)
            .await?;

        ta.amount
    };

    println!("buyer remaining amount: {}", buyer_balance); // 199__900_000_000

    let game_vault_balance = {
        let mut test_fixtures = env.test_fixtures.lock().unwrap();
        let ta: Account = test_fixtures
            .program_simulator
            .get_packed_account_data(game_vault_pubkey)
            .await?;

        ta.amount
    };

    println!("game_vault_balance: {}", game_vault_balance); // 100_000_000

    Ok(())
}
