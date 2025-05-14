use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    vec,
};

use anchor_spl::token::spl_token::instruction::mint_to;
use anyhow::Result;
use gacha_sol::{
    instruction,
    pda::{get_game_config_pubkey, get_pull_pubkey, get_reward_vault_pubkey},
    state::{GameConfig, Pull, AE_CIPHERTEXT_MAX_BASE64_LEN, ELGAMAL_PUBKEY_MAX_BASE64_LEN},
};
use solana_banks_interface::BanksTransactionResultWithSimulation;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_token_2022::{
    extension::confidential_transfer::{
        account_info::{ApplyPendingBalanceAccountInfo, TransferAccountInfo},
        instruction::{PubkeyValidityProofData, ZeroCiphertextProofData, ZkProofData},
    },
    solana_zk_sdk::encryption::pod::{
        auth_encryption::PodAeCiphertext, elgamal::PodElGamalCiphertext,
    },
};
use spl_token_confidential_transfer_ciphertext_arithmetic::subtract;
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;
use spl_token_confidential_transfer_proof_generation::{
    mint::{mint_split_proof_data, MintProofData},
    transfer::TransferProofData,
};

use crate::test_utils::confidential_transfer::{
    confidential_mint_to_ixs, confidential_transfer_ixs, create_close_context_state_ixs,
    get_zk_proof_context_state_account_creation_instructions,
};

use super::{
    confidential_transfer::{apply_pending_balance_ixs, create_confidential_token_account_ixs},
    program_test_fixtures::{setup_test_fixtures, ProgramTestFixtures},
    proof_account::{self, ProofAccount, SignerProofAccount},
};

pub struct GachaSolTestEnvironment {
    pub test_fixtures: Arc<Mutex<ProgramTestFixtures>>,
    pub payer: Keypair,
    pub authority: Keypair,
    pub purchase_mint: Pubkey,
    pub purchase_mint_authority: Keypair,
    pub reward_mint_authority: Keypair,
    pub reward_mint_proof_account: SignerProofAccount,
    pub game_vault: Pubkey,
    pub decimals: u8,
}

impl GachaSolTestEnvironment {
    pub async fn process_instructions(
        &self,
        instructions: &[Instruction],
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<Signature> {
        let mut test_fixtures = self.test_fixtures.lock().unwrap();
        let signature = test_fixtures
            .program_simulator
            .process_ixs_with_default_compute_limit(instructions, signers, payer)
            .await?;

        Ok(signature)
    }

    pub async fn process_instruction(
        &self,
        instruction: Instruction,
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<Signature> {
        self.process_instructions(&[instruction], signers, payer)
            .await
    }

    pub async fn simulate_instructions(
        &self,
        instructions: &[Instruction],
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<BanksTransactionResultWithSimulation> {
        let mut test_fixtures = self.test_fixtures.lock().unwrap();
        let result = test_fixtures
            .program_simulator
            .simulate_ixs_with_default_compute_limit(instructions, signers, payer)
            .await?;

        Ok(result)
    }

    pub async fn simulate_instruction(
        &self,
        instruction: Instruction,
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<BanksTransactionResultWithSimulation> {
        self.simulate_instructions(&[instruction], signers, payer)
            .await
    }
}

impl GachaSolTestEnvironment {
    pub async fn new() -> Result<Self> {
        let test_fixtures = setup_test_fixtures().await;
        let payer = test_fixtures.payer.clone();
        let authority = Keypair::new();
        let purchase_mint_authority = Keypair::new();
        let reward_mint_authority = Keypair::new();
        let reward_mint_proof_account = SignerProofAccount::new();
        let decimals = 9;

        let test_fixtures = Arc::new(Mutex::new(test_fixtures));

        let purchase_mint = {
            let mut test_fixtures = test_fixtures.lock().unwrap();
            test_fixtures
                .create_mint(&purchase_mint_authority.pubkey(), decimals)
                .await?
        };

        {
            let mut test_fixtures = test_fixtures.lock().unwrap();

            test_fixtures
                .create_confidential_transfer_mint(
                    &reward_mint_proof_account,
                    &reward_mint_authority,
                    decimals,
                )
                .await?;
        };

        let game_vault = {
            let mut test_fixtures = test_fixtures.lock().unwrap();

            test_fixtures
                .create_ata(&purchase_mint, &authority.pubkey())
                .await?
        };

        Ok(Self {
            test_fixtures,
            payer: payer.insecure_clone(),
            authority,
            purchase_mint,
            purchase_mint_authority,
            reward_mint_authority,
            reward_mint_proof_account,
            game_vault,
            decimals: 9,
        })
    }

    pub fn reward_mint_pubkey(&self) -> Pubkey {
        self.reward_mint_proof_account.pubkey()
    }

    pub fn purchase_mint_pubkey(&self) -> Pubkey {
        self.purchase_mint
    }

    pub fn game_vault_pubkey(&self) -> Pubkey {
        self.game_vault
    }

    pub fn game_config_pubkey(&self) -> Pubkey {
        get_game_config_pubkey()
    }

    pub fn pull_pubkey(&self, pull_id: u64) -> Pubkey {
        get_pull_pubkey(pull_id)
    }

    pub fn reward_vault_pubkey(&self, pull: Pubkey) -> Pubkey {
        get_reward_vault_pubkey(pull)
    }

    pub async fn initialize_game_config(&self, pull_price: u64) -> Result<Signature> {
        let authority_pubkey = self.authority.pubkey();
        let purchase_mint_pubkey = self.purchase_mint_pubkey();
        let reward_mint_pubkey = self.reward_mint_pubkey();
        let game_vault_pubkey = self.game_vault_pubkey();

        let ix = instruction::InitializeGameConfig::populate(
            authority_pubkey,
            purchase_mint_pubkey,
            reward_mint_pubkey,
            game_vault_pubkey,
            self.payer.pubkey(),
            pull_price,
        );

        let tx = self
            .process_instruction(ix, &vec![&self.payer], None)
            .await?;

        println!("initialize game config tx: {}", tx);

        Ok(tx)
    }

    pub async fn create_pull(
        &self,
        pull_id: u64,
        pull_proof_account: impl ProofAccount,
        expected_amount: u64,
    ) -> Result<Signature> {
        let payer_pubkey = self.payer.pubkey();
        let authority_pubkey = self.authority.pubkey();
        let reward_mint_pubkey = self.reward_mint_pubkey();
        let pull_pubkey = self.pull_pubkey(pull_id);

        let decryptable_zero_balance = pull_proof_account.encrypt_supply(0)?;
        let encrypted_amount = pull_proof_account.encrypt_amount_ciphertext(expected_amount)?;

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

        let tx = self
            .process_instructions(
                &[pubkey_proof_create_ix, pubkey_proof_verify_ix, ix],
                &vec![
                    &self.payer,
                    &self.authority,
                    &pubkey_validity_proof_data_account,
                ],
                None,
            )
            .await?;

        println!("create pull tx: {}", tx);

        Ok(tx)
    }

    pub async fn apply_pull_pending_balance(
        &self,
        pull_id: u64,
        pull_proof_account: SignerProofAccount,
    ) -> Result<()> {
        let authority_pubkey = self.authority.pubkey();
        let pull_pubkey = self.pull_pubkey(pull_id);
        let reward_vault_pubkey = self.reward_vault_pubkey(pull_pubkey);

        let new_decryptable_available_balance = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();

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

        let tx = self
            .process_instruction(ix, &vec![&self.authority], Some(&self.payer))
            .await?;

        println!("apply pull pending balance tx: {}", tx);
        Ok(())
    }

    pub async fn get_game_config(&self) -> Result<GameConfig> {
        let game_config_pubkey = self.game_config_pubkey();
        let mut test_fixtures = self.test_fixtures.lock().unwrap();
        let game_config = test_fixtures
            .program_simulator
            .get_anchor_account_data(game_config_pubkey)
            .await?;

        Ok(game_config)
    }

    pub async fn get_pull(&self, pull_id: u64) -> Result<Pull> {
        let pull_pubkey = self.pull_pubkey(pull_id);
        let mut test_fixtures = self.test_fixtures.lock().unwrap();
        let game_config = test_fixtures
            .program_simulator
            .get_anchor_account_data(pull_pubkey)
            .await?;

        Ok(game_config)
    }

    pub async fn create_ct_token_account(
        &self,
        mint_pubkey: &Pubkey,
        owner_keypair: &Keypair,
        token_account_proof_account: SignerProofAccount,
    ) -> Result<Pubkey> {
        let payer_pubkey = self.payer.pubkey();
        let owner_pubkey = owner_keypair.pubkey();
        let token_account_pubkey = token_account_proof_account.pubkey();
        let token_account_ae_key = token_account_proof_account.get_ae_key()?;
        let token_account_elgamal_keypair =
            token_account_proof_account.get_pod_elgamal_keypair()?;

        let token_account_ixs = create_confidential_token_account_ixs(
            &payer_pubkey,
            &owner_pubkey,
            mint_pubkey,
            &token_account_pubkey,
            &token_account_ae_key,
            &token_account_elgamal_keypair,
        )?;

        let tx = self
            .process_instructions(
                &token_account_ixs,
                &vec![
                    &self.payer,
                    owner_keypair,
                    &token_account_proof_account.keypair,
                ],
                None,
            )
            .await?;

        println!("token account tx: {}", tx);

        Ok(Pubkey::default())
    }

    pub async fn mint_reward_token(
        &self,
        token_account_pubkey: &Pubkey,
        mint_amount: u64,
    ) -> Result<()> {
        let mint_pubkey = self.reward_mint_pubkey();
        let mint_proof_account = &self.reward_mint_proof_account;

        let destination_elgamal_pubkey = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();
            test_fixtures
                .get_token_account_elgamal_pubkey(token_account_pubkey)
                .await?
        };

        let (current_supply_ciphertext, current_supply, new_decryptable_supply) = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();
            let current_mint_confidentail_supply = test_fixtures
                .get_mint_confidential_supply(&mint_pubkey)
                .await?;

            let current_supply = test_fixtures
                .get_mint_decrypted_decryptable_supply(mint_proof_account)
                .await?;

            let new_supply = current_supply + mint_amount;
            let new_decryptable_supply = mint_proof_account.encrypt_supply(new_supply)?;

            (
                current_mint_confidentail_supply,
                current_supply,
                new_decryptable_supply,
            )
        };

        let supply_elgamal_keypair = mint_proof_account.get_pod_elgamal_keypair()?;

        let MintProofData {
            equality_proof_data,
            ciphertext_validity_proof_data_with_ciphertext,
            range_proof_data,
        } = mint_split_proof_data(
            &current_supply_ciphertext.try_into()?,
            mint_amount,
            current_supply,
            &supply_elgamal_keypair,
            &destination_elgamal_pubkey.try_into()?,
            None, // no auditor
        )?;

        // Create 3 proofs ------------------------------------------------------

        // Generate address for equality proof account
        let equality_proof_context_state_account = Keypair::new();
        let equality_proof_pubkey = equality_proof_context_state_account.pubkey();

        // Generate address for ciphertext validity proof account
        let ciphertext_validity_proof_context_state_account = Keypair::new();
        let ciphertext_validity_proof_pubkey =
            ciphertext_validity_proof_context_state_account.pubkey();

        // Generate address for range proof account
        let range_proof_context_state_account = Keypair::new();
        let range_proof_pubkey = range_proof_context_state_account.pubkey();

        let payer_pubkey = self.payer.pubkey();
        let authority_pubkey = self.reward_mint_authority.pubkey();

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

        // Ciphertext Validity Proof Instructions ----------------------------------------------------------------
        let (cv_create_ix, cv_verify_ix) =
            get_zk_proof_context_state_account_creation_instructions(
                &payer_pubkey,
                &ciphertext_validity_proof_context_state_account.pubkey(),
                &authority_pubkey,
                &ciphertext_validity_proof_data_with_ciphertext.proof_data,
            )?;

        let proof_accounts_ixs = [
            range_create_ix,
            equality_create_ix,
            cv_create_ix,
            range_verify_ix,
            equality_verify_ix,
            cv_verify_ix,
        ];

        let proof_account_tx = self
            .process_instructions(
                &proof_accounts_ixs,
                &vec![
                    &range_proof_context_state_account,
                    &equality_proof_context_state_account,
                    &ciphertext_validity_proof_context_state_account,
                ],
                Some(&self.payer),
            )
            .await?;

        println!("proof accounts tx: {}", proof_account_tx);

        // Confidential Mint To Instructions ---------------------------------------------------------------

        let equality_proof_location = ProofLocation::ContextStateAccount(&equality_proof_pubkey);
        let ciphertext_validity_proof_location =
            ProofLocation::ContextStateAccount(&ciphertext_validity_proof_pubkey);
        let range_proof_location = ProofLocation::ContextStateAccount(&range_proof_pubkey);

        let mint_to_ixs = confidential_mint_to_ixs(
            &mint_pubkey,
            &authority_pubkey,
            &token_account_pubkey,
            &new_decryptable_supply.try_into()?,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
            equality_proof_location,
            ciphertext_validity_proof_location,
            range_proof_location,
        )?;

        let mint_to_tx = self
            .process_instructions(
                &mint_to_ixs,
                &vec![&self.reward_mint_authority],
                Some(&self.payer),
            )
            .await?;

        println!("mint to tx: {}", mint_to_tx);

        // Close context states Instructions ---------------------------------------------------------------

        let close_context_state_ixs = create_close_context_state_ixs(
            &[
                equality_proof_pubkey,
                ciphertext_validity_proof_pubkey,
                range_proof_pubkey,
            ],
            &authority_pubkey,
            &payer_pubkey,
        );

        let close_context_state_tx = self
            .process_instructions(
                &close_context_state_ixs,
                &vec![&self.reward_mint_authority],
                Some(&self.payer),
            )
            .await?;

        println!("close context accounts tx: {}", close_context_state_tx);

        Ok(())
    }

    pub async fn apply_pending_balance(
        &self,
        token_account_proof_account: SignerProofAccount,
        owner: &Keypair,
    ) -> Result<()> {
        let token_account_pubkey = token_account_proof_account.pubkey();
        let owner_pubkey = owner.pubkey();

        let apply_pending_balance_account_info = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();
            let confidential_transfer_account = test_fixtures
                .get_token_account_credential_transfer_account(&token_account_pubkey)
                .await?;

            ApplyPendingBalanceAccountInfo::new(&confidential_transfer_account)
        };

        let new_decryptable_available_balance = apply_pending_balance_account_info
            .new_decryptable_available_balance(
                &token_account_proof_account
                    .get_pod_elgamal_keypair()?
                    .secret(),
                &token_account_proof_account.get_ae_key()?.try_into()?,
            )?;

        let expected_pending_balance_credit_counter: u64 =
            apply_pending_balance_account_info.pending_balance_credit_counter();

        let apply_pending_balance_instructions = apply_pending_balance_ixs(
            &owner_pubkey,
            &token_account_pubkey,
            &new_decryptable_available_balance.into(),
            expected_pending_balance_credit_counter,
        )?;

        let apply_pending_balance_tx = self
            .process_instructions(
                &apply_pending_balance_instructions,
                &vec![owner],
                Some(&self.payer),
            )
            .await?;

        println!("apply pending balance tx: {}", apply_pending_balance_tx);

        Ok(())
    }

    pub async fn verify_pull(
        &self,
        pull_id: u64,
        pull_proof_account: SignerProofAccount,
    ) -> Result<()> {
        let payer_pubkey = self.payer.pubkey();
        let authority_pubkey = self.authority.pubkey();
        let pull_pubkey = self.pull_pubkey(pull_id);
        let reward_vault_pubkey = self.reward_vault_pubkey(pull_pubkey);

        let pull = self.get_pull(pull_id).await?;

        let ciphertext_available_balance = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();

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

        let proof_account_tx = self
            .process_instructions(
                &proof_accounts_ixs,
                &vec![
                    // &self.authority,
                    &zero_ciphertext_proof_context_state_account,
                ],
                Some(&self.payer),
            )
            .await?;

        println!("proof accounts tx: {}", proof_account_tx);

        let ix = instruction::VerifyPull::populate(
            authority_pubkey,
            zero_ciphertext_proof_pubkey,
            pull_id,
        );

        let tx = self
            .process_instructions(&[ix], &vec![&self.authority], None)
            .await?;

        println!("verify pull tx: {}", tx);

        let pull = self.get_pull(pull_id).await?;

        assert_eq!(pull.verified, true);

        // Close context states Instructions ---------------------------------------------------------------

        let close_context_state_ixs = create_close_context_state_ixs(
            &[zero_ciphertext_proof_pubkey],
            &authority_pubkey,
            &payer_pubkey,
        );

        let close_context_state_tx = self
            .process_instructions(&close_context_state_ixs, &vec![&self.authority], None)
            .await?;

        println!("close context accounts tx: {}", close_context_state_tx);

        Ok(())
    }

    pub async fn buy_pull(
        &self,
        buyer: &Keypair,
        buyer_purchase_token_account: &Pubkey,
        pull_id: u64,
    ) -> Result<()> {
        let buyer_pubkey = buyer.pubkey();
        let purchase_mint_pubkey = self.purchase_mint_pubkey();

        let ix: solana_sdk::instruction::Instruction = instruction::BuyPull::populate(
            buyer_pubkey,
            *buyer_purchase_token_account,
            self.game_vault_pubkey(),
            purchase_mint_pubkey,
            pull_id,
        );

        let tx = self
            .process_instruction(ix, &vec![&buyer], Some(&self.payer))
            .await?;

        println!("buy pull tx: {}", tx);
        Ok(())
    }

    pub async fn ct_transfer_reward_token(
        &self,
        token_account_proof_account: SignerProofAccount,
        owner: &Keypair,
        target_token_account_pubkey: &Pubkey,
        transfer_amount: u64,
    ) -> Result<()> {
        let mint_pubkey = self.reward_mint_pubkey();

        let transfer_account_info = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();
            let confidential_transfer_account = test_fixtures
                .get_token_account_credential_transfer_account(
                    &token_account_proof_account.pubkey(),
                )
                .await?;

            TransferAccountInfo::new(&confidential_transfer_account)
        };

        let new_decryptable_available_balance = transfer_account_info
            .new_decryptable_available_balance(
                transfer_amount,
                &token_account_proof_account.get_ae_key()?,
            )
            .map_err(|e| anyhow::anyhow!("decrypt decryptable_available_balance failed: {}", e))?;

        let source_elgamal_keypair = token_account_proof_account.get_pod_elgamal_keypair()?;
        let source_aes_key = token_account_proof_account.get_ae_key()?;
        let destination_elgamal_pubkey = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();
            test_fixtures
                .get_token_account_elgamal_pubkey(target_token_account_pubkey)
                .await?
        };

        let TransferProofData {
            equality_proof_data,
            ciphertext_validity_proof_data_with_ciphertext,
            range_proof_data,
        } = transfer_account_info
            .generate_split_transfer_proof_data(
                transfer_amount,
                &source_elgamal_keypair,
                &source_aes_key,
                &destination_elgamal_pubkey.try_into()?,
                None,
            )
            .map_err(|_| anyhow::anyhow!("proof generation failed"))?;

        // Create 3 proofs ------------------------------------------------------

        // Generate address for equality proof account
        let equality_proof_context_state_account = Keypair::new();
        let equality_proof_pubkey = equality_proof_context_state_account.pubkey();

        // Generate address for ciphertext validity proof account
        let ciphertext_validity_proof_context_state_account = Keypair::new();
        let ciphertext_validity_proof_pubkey =
            ciphertext_validity_proof_context_state_account.pubkey();

        // Generate address for range proof account
        let range_proof_context_state_account = Keypair::new();
        let range_proof_pubkey = range_proof_context_state_account.pubkey();

        let payer_pubkey = self.payer.pubkey();
        let authority_pubkey = owner.pubkey();

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

        // Ciphertext Validity Proof Instructions ----------------------------------------------------------------
        let (cv_create_ix, cv_verify_ix) =
            get_zk_proof_context_state_account_creation_instructions(
                &payer_pubkey,
                &ciphertext_validity_proof_context_state_account.pubkey(),
                &authority_pubkey,
                &ciphertext_validity_proof_data_with_ciphertext.proof_data,
            )?;

        let proof_accounts_ixs = [
            range_create_ix,
            equality_create_ix,
            cv_create_ix,
            range_verify_ix,
            equality_verify_ix,
            cv_verify_ix,
        ];

        let proof_account_tx = self
            .process_instructions(
                &proof_accounts_ixs,
                &vec![
                    // &self.authority,
                    &range_proof_context_state_account,
                    &equality_proof_context_state_account,
                    &ciphertext_validity_proof_context_state_account,
                ],
                Some(&self.payer),
            )
            .await?;

        println!("proof accounts tx: {}", proof_account_tx);

        let equality_proof_data_location =
            ProofLocation::ContextStateAccount(&equality_proof_pubkey);
        let ciphertext_validity_proof_data_location =
            ProofLocation::ContextStateAccount(&ciphertext_validity_proof_pubkey);
        let range_proof_data_location = ProofLocation::ContextStateAccount(&range_proof_pubkey);

        let ixs = confidential_transfer_ixs(
            &owner.pubkey(),
            &token_account_proof_account.pubkey(),
            &target_token_account_pubkey,
            &mint_pubkey,
            &new_decryptable_available_balance.try_into()?,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
            equality_proof_data_location,
            ciphertext_validity_proof_data_location,
            range_proof_data_location,
        )?;

        let tx = self
            .process_instructions(&ixs, &vec![owner], Some(&self.payer))
            .await?;

        println!("transfer tx: {}", tx);

        // Close context states Instructions ---------------------------------------------------------------

        let close_context_state_ixs = create_close_context_state_ixs(
            &[
                equality_proof_pubkey,
                ciphertext_validity_proof_pubkey,
                range_proof_pubkey,
            ],
            &owner.pubkey(),
            &payer_pubkey,
        );

        let close_context_state_tx = self
            .process_instructions(&close_context_state_ixs, &vec![&owner], Some(&self.payer))
            .await?;

        println!("close context accounts tx: {}", close_context_state_tx);

        Ok(())
    }

    pub async fn create_ata(&self, mint: &Pubkey, owner: &Pubkey) -> Result<Pubkey> {
        let mut test_fixtures = self.test_fixtures.lock().unwrap();
        let ata = test_fixtures.create_ata(mint, owner).await?;

        Ok(ata)
    }

    pub async fn mint_purchase_token(&self, token_account: &Pubkey, amount: u64) -> Result<()> {
        let mint_pubkey = self.purchase_mint_pubkey();
        let token_program_id = {
            let mut test_fixtures = self.test_fixtures.lock().unwrap();
            test_fixtures
                .program_simulator
                .get_account(mint_pubkey)
                .await?
                .owner
        };

        let ix = mint_to(
            &token_program_id,
            &mint_pubkey,
            token_account,
            &self.purchase_mint_authority.pubkey(),
            &[],
            amount,
        )?;

        let tx = self
            .process_instruction(ix, &vec![&self.purchase_mint_authority], None)
            .await?;

        println!("mint to tx: {}", tx);
        Ok(())
    }
}
