use anyhow::Result;
use gacha_sol::instruction;
use solana_sdk::signer::Signer;

use crate::test_utils::gacha_sol_test_environment::GachaSolTestEnvironment;

#[tokio::test]
async fn test_initialize_game_config() -> Result<()> {
    let env = GachaSolTestEnvironment::new().await?;

    let authority_pubkey = env.authority.pubkey();
    let purchase_mint_pubkey = env.purchase_mint_pubkey();
    let reward_mint_pubkey = env.reward_mint_pubkey();
    let game_vault_pubkey = env.game_vault_pubkey();

    let pull_price = 1001234;

    let ix = instruction::InitializeGameConfig::populate(
        authority_pubkey,
        purchase_mint_pubkey,
        reward_mint_pubkey,
        game_vault_pubkey,
        env.payer.pubkey(),
        pull_price,
    );

    let tx = env.process_instruction(ix, &vec![&env.payer], None).await?;

    println!("initialize game config tx: {}", tx);

    let game_config = env.get_game_config().await?;

    assert_eq!(game_config.authority, authority_pubkey);
    assert_eq!(game_config.purchase_mint, purchase_mint_pubkey);
    assert_eq!(game_config.reward_mint, reward_mint_pubkey);
    assert_eq!(game_config.game_vault, game_vault_pubkey);
    assert_eq!(game_config.pull_price, pull_price);
    assert_eq!(game_config.last_pull_id, 0);

    Ok(())
}
