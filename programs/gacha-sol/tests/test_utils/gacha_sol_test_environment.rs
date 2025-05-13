use std::sync::{Arc, Mutex};

use anyhow::Result;
use gacha_sol::{
    instruction,
    pda::{get_game_config_pubkey, get_pull_pubkey, get_reward_vault_pubkey},
    state::{GameConfig, Pull},
};
use solana_banks_interface::BanksTransactionResultWithSimulation;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};

use super::{
    program_test_fixtures::{setup_test_fixtures, ProgramTestFixtures},
    proof_account::{ProofAccount, SignerProofAccount},
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
                .create_confidential_transfer_mint(&reward_mint_proof_account, decimals)
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

    // pub async fn wait(&self, tx: Signature) -> Result<()> {
    //     let mut test_fixtures = self.test_fixtures.lock().unwrap();
    //     let mut sig: Option<solana_banks_interface::TransactionStatus> = None;

    //     while sig.is_none() {
    //         sig = test_fixtures
    //             .program_simulator
    //             .get_transaction_status(tx)
    //             .await?;

    //         if let Some(s) = &sig {
    //             if let Some(status) = &s.confirmation_status {
    //                 if status.eq(&TransactionConfirmationStatus::Confirmed) {
    //                     println!("xxx sig {:?}", status);
    //                     break;
    //                 }
    //             }
    //         }

    //         println!("xxx sig {:?}", sig);
    //     }

    //     Ok(())
    // }
}
