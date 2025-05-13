use anyhow::Result;
use gacha_sol::{
    instruction,
    state::{AE_CIPHERTEXT_MAX_BASE64_LEN, ELGAMAL_PUBKEY_MAX_BASE64_LEN},
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token_2022::{
    extension::confidential_transfer::instruction::PubkeyValidityProofData,
    solana_zk_sdk::encryption::pod::{
        auth_encryption::PodAeCiphertext, elgamal::PodElGamalCiphertext,
    },
};

use crate::test_utils::{
    confidential_transfer::get_zk_proof_context_state_account_creation_instructions,
    gacha_sol_test_environment::GachaSolTestEnvironment,
    proof_account::{ProofAccount, SignerProofAccount},
};

#[tokio::test]
async fn test_create_pull() -> Result<()> {
    let env = GachaSolTestEnvironment::new().await?;

    let pull_price = 100_000_000;
    env.initialize_game_config(pull_price).await?;
    let pull_id = env.get_game_config().await?.last_pull_id + 1;

    let payer_pubkey = env.payer.pubkey();
    let authority_pubkey = env.authority.pubkey();
    let reward_mint_pubkey = env.reward_mint_pubkey();
    let pull_pubkey = env.pull_pubkey(pull_id);
    let reward_vault_pubkey = env.reward_vault_pubkey(pull_pubkey);
    let expected_amount = 200_000_000;

    let pull_proof_account = SignerProofAccount::new();

    let decryptable_zero_balance: PodAeCiphertext = pull_proof_account.encrypt_supply(0)?;
    let encrypted_amount: PodElGamalCiphertext =
        pull_proof_account.encrypt_amount_ciphertext(expected_amount)?;

    // Convert to base64 strings
    let decryptable_zero_balance_base64 = decryptable_zero_balance.to_string();
    let encrypted_amount_base64 = encrypted_amount.to_string();

    // Get base64 bytes
    let zero_balance_bytes = decryptable_zero_balance_base64.as_bytes();
    let amount_bytes = encrypted_amount_base64.as_bytes();

    // Verify lengths match exactly
    if zero_balance_bytes.len() != AE_CIPHERTEXT_MAX_BASE64_LEN {
        return Err(anyhow::anyhow!(
            "Invalid zero balance length: expected {}, got {}",
            AE_CIPHERTEXT_MAX_BASE64_LEN,
            zero_balance_bytes.len()
        ));
    }
    if amount_bytes.len() != ELGAMAL_PUBKEY_MAX_BASE64_LEN {
        return Err(anyhow::anyhow!(
            "Invalid encrypted amount length: expected {}, got {}",
            ELGAMAL_PUBKEY_MAX_BASE64_LEN,
            amount_bytes.len()
        ));
    }

    // Initialize fixed-size arrays and copy bytes
    let mut decryptable_zero_balance_array = [0u8; AE_CIPHERTEXT_MAX_BASE64_LEN];
    let mut encrypted_amount_array = [0u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN];
    decryptable_zero_balance_array.copy_from_slice(zero_balance_bytes);
    encrypted_amount_array.copy_from_slice(amount_bytes);

    let pubkey_validity_proof_data_account = Keypair::new();
    let pubkey_validity_proof_data_pubkey = pubkey_validity_proof_data_account.pubkey();

    let pubkey_validity_proof_data =
        PubkeyValidityProofData::new(&pull_proof_account.get_pod_elgamal_keypair()?)
            .map_err(|_| anyhow::anyhow!("proof generation failed"))?;
    let (pubkey_proof_create_ix, pubkey_proof_verify_ix) =
        get_zk_proof_context_state_account_creation_instructions(
            &payer_pubkey,
            &pubkey_validity_proof_data_pubkey,
            &pull_pubkey,
            &pubkey_validity_proof_data,
        )?;

    let ix = instruction::CreatePull::populate(
        authority_pubkey,
        reward_mint_pubkey,
        payer_pubkey,
        pubkey_validity_proof_data_pubkey,
        pull_id,
        encrypted_amount_array,
        decryptable_zero_balance_array,
    );

    let tx = env
        .process_instructions(
            &[pubkey_proof_create_ix, pubkey_proof_verify_ix, ix],
            &vec![
                &env.payer,
                &env.authority,
                &pubkey_validity_proof_data_account,
            ],
            None,
        )
        .await?;

    println!("create pull tx: {}", tx);

    let pull = env.get_pull(pull_id).await?;
    assert_eq!(pull.id, pull_id);
    assert_eq!(pull.reward_vault, reward_vault_pubkey);
    assert_eq!(pull.encrypted_amount, encrypted_amount_array);
    assert_eq!(pull.buyer, Pubkey::default());
    assert_eq!(pull.verified, false);
    assert_eq!(pull.claimed, false);
    assert_eq!(pull.revealed_amount, 0);
    assert_eq!(
        pull.transfer_amount_auditor_ciphertext_lo,
        [0u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN]
    );
    assert_eq!(
        pull.transfer_amount_auditor_ciphertext_hi,
        [0u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN]
    );
    assert_eq!(
        pull.final_decryptable_available_balance,
        [0u8; AE_CIPHERTEXT_MAX_BASE64_LEN]
    );
    assert_eq!(pull.equality_proof_account, Pubkey::default());
    assert_eq!(pull.ciphertext_validity_proof_account, Pubkey::default());
    assert_eq!(pull.range_proof_account, Pubkey::default());
    assert!(pull.bump > 0);

    Ok(())
}
