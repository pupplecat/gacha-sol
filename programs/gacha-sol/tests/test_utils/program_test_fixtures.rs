use std::sync::Arc;

use anchor_spl::token;
use anyhow::Result;
use cargo_metadata::MetadataCommand;
use solana_banks_interface::BanksTransactionResultWithSimulation;
use solana_program_simulator::program_simulator::ProgramSimulator;
use solana_program_test::ProgramTest;
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction::create_account,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::{
    extension::{
        confidential_mint_burn::ConfidentialMintBurn,
        confidential_transfer::{
            account_info::{combine_balances, ApplyPendingBalanceAccountInfo, TransferAccountInfo},
            ConfidentialTransferAccount,
        },
        BaseStateWithExtensions as _, StateWithExtensions,
    },
    instruction::initialize_mint2,
    solana_zk_sdk::encryption::{
        elgamal::ElGamalCiphertext,
        pod::{
            auth_encryption::PodAeCiphertext,
            elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        },
    },
    state::{Account as Token2022Account, Mint},
    ui_amount_to_amount,
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;
use spl_token_confidential_transfer_proof_generation::{
    mint::{mint_split_proof_data, MintProofData},
    transfer::TransferProofData,
};

use crate::{
    anchor_processor,
    test_utils::{
        confidential_transfer::{
            apply_pending_balance_ixs, confidential_mint_to_ixs, confidential_transfer_ixs,
            create_close_context_state_ixs, create_confidential_token_account_ixs,
            create_mint_with_confidential_extensions_ixs,
            get_zk_proof_context_state_account_creation_instructions, token_2022_program_id,
        },
        proof_account::{ProofAccount, SignerProofAccount},
    },
};

use super::confidential_transfer::create_confidential_transfer_mint_ixs;

pub struct ProgramTestFixtures {
    pub program_simulator: ProgramSimulator,
    pub payer: Arc<Keypair>,
    pub authority: Keypair,
    pub gacha_sol_program_id: Pubkey,
}

fn set_sbf_out_dir() {
    // Use cargo_metadata to fetch workspace metadata
    let metadata = MetadataCommand::new()
        .exec()
        .expect("Failed to fetch workspace metadata");

    let workspace_root = metadata.workspace_root;

    // Construct and set the SBF output directory
    let sbf_out_dir = workspace_root.join("target/deploy");

    std::env::set_var("SBF_OUT_DIR", sbf_out_dir);
}

pub async fn setup_test_fixtures() -> ProgramTestFixtures {
    set_sbf_out_dir();

    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program("gacha_sol", gacha_sol::ID, anchor_processor!(gacha_sol));

    program_test.prefer_bpf(true);

    let mut program_simulator = ProgramSimulator::start_from_program_test(program_test).await;

    let authority = Keypair::new();
    let payer = program_simulator.get_funded_keypair().await.unwrap();

    ProgramTestFixtures {
        program_simulator,
        payer: Arc::new(payer),
        authority,
        gacha_sol_program_id: gacha_sol::ID,
    }
}

impl ProgramTestFixtures {
    pub async fn process_instructions(
        &mut self,
        instructions: &[Instruction],
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<Signature> {
        let signature = self
            .program_simulator
            .process_ixs_with_default_compute_limit(instructions, signers, payer)
            .await?;

        Ok(signature)
    }

    pub async fn process_instruction(
        &mut self,
        instruction: Instruction,
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<Signature> {
        self.process_instructions(&[instruction], signers, payer)
            .await
    }

    pub async fn simulate_instructions(
        &mut self,
        instructions: &[Instruction],
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<BanksTransactionResultWithSimulation> {
        let result = self
            .program_simulator
            .simulate_ixs_with_default_compute_limit(instructions, signers, payer)
            .await?;

        Ok(result)
    }

    pub async fn simulate_instruction(
        &mut self,
        instruction: Instruction,
        signers: &Vec<&Keypair>,
        payer: Option<&Keypair>,
    ) -> Result<BanksTransactionResultWithSimulation> {
        self.simulate_instructions(&[instruction], signers, payer)
            .await
    }

    pub async fn create_mint(
        &mut self,
        mint_authority_pubkey: &Pubkey,
        decimals: u8,
    ) -> Result<Pubkey> {
        let mint_keypair = Keypair::new();

        let create_account_instruction = create_account(
            &self.payer.pubkey(),
            &mint_keypair.pubkey(),
            Rent::default().minimum_balance(Mint::LEN),
            Mint::LEN as u64,
            &token::ID,
        );

        let initialize_mint_instruction = initialize_mint2(
            &token::ID,
            &mint_keypair.pubkey(),
            mint_authority_pubkey,
            None,
            decimals,
        )
        .unwrap();

        self.program_simulator
            .process_ixs_with_default_compute_limit(
                &[create_account_instruction, initialize_mint_instruction],
                &vec![&mint_keypair],
                Some(&self.payer),
            )
            .await?;

        Ok(mint_keypair.pubkey())
    }

    pub async fn create_ata(&mut self, mint: &Pubkey, wallet_address: &Pubkey) -> Result<Pubkey> {
        let account = self.program_simulator.get_account(*mint).await?;
        // Derive the associated token account address
        let ata_pubkey =
            get_associated_token_address_with_program_id(wallet_address, mint, &account.owner);

        // Create an instruction to create the ATA
        let create_ata_instruction = create_associated_token_account(
            &self.payer.pubkey(),
            wallet_address, // Owner of the ATA
            mint,           // Mint of the token
            &account.owner,
        );

        // Process the instruction
        self.program_simulator
            .process_ixs_with_default_compute_limit(
                &[create_ata_instruction], // Instruction to execute
                &vec![],                   // Signers
                Some(&self.payer),         // Fee payer
            )
            .await?;

        Ok(ata_pubkey) // Return the derived ATA address
    }

    /// Fetches the ***encrypted*** total supply ciphertext from a CT mint
    pub async fn get_mint_decryptable_supply(
        &mut self,
        mint_pubkey: &Pubkey,
    ) -> Result<PodAeCiphertext> {
        // 1) pull down the raw account data
        let acct = self.program_simulator.get_account(*mint_pubkey).await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Mint>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Mint+extensions"))?;

        // 3) grab the ConfidentialMintBurn extension
        let ext = state
            .get_extension::<ConfidentialMintBurn>()
            .map_err(|_| anyhow::anyhow!("Mint is missing ConfidentialMintBurn"))?;

        // 4) return the encrypted supply blob
        Ok(ext.decryptable_supply)
    }

    pub async fn get_mint_confidential_supply(
        &mut self,
        mint_pubkey: &Pubkey,
    ) -> Result<PodElGamalCiphertext> {
        // 1) pull down the raw account data
        let acct = self.program_simulator.get_account(*mint_pubkey).await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Mint>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Mint+extensions"))?;

        // 3) grab the ConfidentialMintBurn extension
        let ext = state
            .get_extension::<ConfidentialMintBurn>()
            .map_err(|_| anyhow::anyhow!("Mint is missing ConfidentialMintBurn"))?;

        // 4) return the encrypted supply blob
        Ok(ext.confidential_supply)
    }

    pub async fn get_mint_decrypted_decryptable_supply(
        &mut self,
        proof_account: &impl ProofAccount,
    ) -> Result<u64> {
        let encrypt_supply = self
            .get_mint_decryptable_supply(&proof_account.pubkey())
            .await?;
        Ok(proof_account.decrypt_supply(&encrypt_supply)?)
    }

    pub async fn get_mint_decrypted_confidential_supply(
        &mut self,
        proof_account: &impl ProofAccount,
    ) -> Result<u64> {
        let encrypt_supply = self
            .get_mint_confidential_supply(&proof_account.pubkey())
            .await?;
        Ok(proof_account.decrypt_amount_ciphertext(&encrypt_supply)?)
    }

    pub async fn get_token_account_elgamal_pubkey(
        &mut self,
        token_account_pubkey: &Pubkey,
    ) -> Result<PodElGamalPubkey> {
        // 1) pull down the raw account data
        let acct = self
            .program_simulator
            .get_account(*token_account_pubkey)
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        Ok(ext.elgamal_pubkey)
    }

    pub async fn get_token_account_decryptable_available_balance(
        &mut self,
        token_account_pubkey: &Pubkey,
    ) -> Result<PodAeCiphertext> {
        // 1) pull down the raw account data
        let acct = self
            .program_simulator
            .get_account(*token_account_pubkey)
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        // 4) return the encrypted balance blob
        Ok(ext.decryptable_available_balance)
    }

    pub async fn get_token_account_decrypted_decryptable_available_balance(
        &mut self,
        proof_account: &impl ProofAccount,
    ) -> Result<u64> {
        let encrypt_balance = self
            .get_token_account_decryptable_available_balance(&proof_account.pubkey())
            .await?;
        Ok(proof_account.decrypt_supply(&encrypt_balance)?)
    }

    pub async fn get_token_account_decrypted_decryptable_available_balance_with_pubkey(
        &mut self,
        token_account: &Pubkey,
        proof_account: &impl ProofAccount,
    ) -> Result<u64> {
        let encrypt_balance = self
            .get_token_account_decryptable_available_balance(token_account)
            .await?;
        Ok(proof_account.decrypt_supply(&encrypt_balance)?)
    }

    pub async fn get_token_account_available_balance(
        &mut self,
        token_account_pubkey: &Pubkey,
    ) -> Result<PodElGamalCiphertext> {
        // 1) pull down the raw account data
        let acct = self
            .program_simulator
            .get_account(*token_account_pubkey)
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        // 4) return the encrypted balance blob
        Ok(ext.available_balance)
    }

    pub async fn get_token_account_pending_balance_lo(
        &mut self,
        token_account_pubkey: &Pubkey,
    ) -> Result<PodElGamalCiphertext> {
        // 1) pull down the raw account data
        let acct = self
            .program_simulator
            .get_account(*token_account_pubkey)
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        // 4) return the encrypted balance blob
        Ok(ext.pending_balance_lo)
    }

    pub async fn get_token_account_pending_balance_hi(
        &mut self,
        token_account_pubkey: &Pubkey,
    ) -> Result<PodElGamalCiphertext> {
        // 1) pull down the raw account data
        let acct = self
            .program_simulator
            .get_account(*token_account_pubkey)
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        // 4) return the encrypted balance blob
        Ok(ext.pending_balance_hi)
    }

    pub async fn get_token_account_credential_transfer_account<'a>(
        &mut self,
        token_account_pubkey: &Pubkey,
    ) -> Result<ConfidentialTransferAccount> {
        // 1) pull down the raw account data
        let acct = self
            .program_simulator
            .get_account(*token_account_pubkey)
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        Ok(ext.clone())
    }

    pub async fn get_token_account_pending_balance(
        &mut self,
        token_account_proof_account: &SignerProofAccount,
    ) -> Result<u64> {
        // 1) pull down the raw account data
        let acct = self
            .program_simulator
            .get_account(token_account_proof_account.pubkey())
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Token2022Account>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Account+extensions"))?;

        // 3) grab the ConfidentialTransferAccount extension
        let ext = state
            .get_extension::<ConfidentialTransferAccount>()
            .map_err(|_| anyhow::anyhow!("Account is missing ConfidentialTransferAccount"))?;

        let balance_lo =
            token_account_proof_account.decrypt_amount_ciphertext(&ext.pending_balance_lo)?;
        let balance_hi =
            token_account_proof_account.decrypt_amount_ciphertext(&ext.pending_balance_hi)?;

        combine_balances(balance_lo, balance_hi).ok_or(anyhow::anyhow!("combine_balances failed"))
    }

    pub async fn get_token_account_decrypted_available_balance(
        &mut self,
        proof_account: &impl ProofAccount,
    ) -> Result<u64> {
        let encrypt_balance = self
            .get_token_account_available_balance(&proof_account.pubkey())
            .await?;
        Ok(proof_account.decrypt_amount_ciphertext(&encrypt_balance)?)
    }

    pub async fn get_token_account_decrypted_pending_balance_lo(
        &mut self,
        proof_account: &impl ProofAccount,
    ) -> Result<u64> {
        let encrypt_balance = self
            .get_token_account_pending_balance_lo(&proof_account.pubkey())
            .await?;
        Ok(proof_account.decrypt_amount_ciphertext(&encrypt_balance)?)
    }

    pub async fn get_token_account_decrypted_pending_balance_hi(
        &mut self,
        proof_account: &impl ProofAccount,
    ) -> Result<u64> {
        let encrypt_balance = self
            .get_token_account_pending_balance_hi(&proof_account.pubkey())
            .await?;
        Ok(proof_account.decrypt_amount_ciphertext(&encrypt_balance)?)
    }

    /// Computes the current supply from the decryptable supply and the
    /// difference between the decryptable supply and the ElGamal encrypted
    /// supply ciphertext
    pub async fn calculate_mint_current_supply(
        &mut self,
        mint_proof_account: &SignerProofAccount,
    ) -> Result<u64> {
        let elgamal_keypair = mint_proof_account.get_pod_elgamal_keypair()?;
        let acct = self
            .program_simulator
            .get_account(mint_proof_account.pubkey())
            .await?;

        // 2) unpack base+extensions
        let state = StateWithExtensions::<Mint>::unpack(&acct.data)
            .map_err(|_| anyhow::anyhow!("Failed to unpack Mint+extensions"))?;

        // 3) grab the ConfidentialMintBurn extension
        let ext = state
            .get_extension::<ConfidentialMintBurn>()
            .map_err(|_| anyhow::anyhow!("Mint is missing ConfidentialMintBurn"))?;

        // decrypt the decryptable supply
        let current_decyptable_supply =
            mint_proof_account.decrypt_supply(&ext.decryptable_supply)?;
        let current_supply = ext.confidential_supply;

        // get the difference between the supply ciphertext and the decryptable supply
        // explanation see https://github.com/solana-labs/solana-program-library/pull/6881#issuecomment-2385579058
        let decryptable_supply_ciphertext =
            elgamal_keypair.pubkey().encrypt(current_decyptable_supply);

        let supply_delta_ciphertext =
            decryptable_supply_ciphertext - ElGamalCiphertext::try_from(current_supply)?;

        let decryptable_to_current_diff = elgamal_keypair
            .secret()
            .decrypt_u32(&supply_delta_ciphertext)
            .ok_or(anyhow::anyhow!("decrypt error"))?;

        // compute the current supply
        current_decyptable_supply
            .checked_sub(decryptable_to_current_diff)
            .ok_or(anyhow::anyhow!("over flow"))
    }

    pub async fn create_confidential_transfer_mint(
        &mut self,
        mint_proof_account: &SignerProofAccount,
        mint_authority: &Keypair,
        decimals: u8,
    ) -> Result<Signature> {
        let payer_pubkey = self.payer.pubkey();
        let authority_pubkey = mint_authority.pubkey();

        let mint_pubkey = mint_proof_account.pubkey();

        let mint_ixs = create_confidential_transfer_mint_ixs(
            &payer_pubkey,
            &mint_pubkey,
            &authority_pubkey,
            None,
            None,
            decimals,
        )?;

        let tx = self
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &mint_ixs,
                &[&self.payer, &mint_proof_account.keypair],
                None,
            )
            .await?;

        println!("create confidential transfer mint tx: {}", tx);

        Ok(tx)
    }

    pub async fn create_confidential_transfer_token_account(
        &mut self,
        owner_keypair: &Keypair,
        token_account_proof_account: &SignerProofAccount,
        mint_pubkey: &Pubkey,
    ) -> Result<Signature> {
        let payer_pubkey = self.payer.pubkey();

        let owner_pubkey = owner_keypair.pubkey();
        let token_account_pubkey = token_account_proof_account.pubkey();
        let token_account_ae_key = token_account_proof_account.get_ae_key()?;
        let token_account_elgamal_keypair =
            token_account_proof_account.get_pod_elgamal_keypair()?;

        let token_account_ixs = create_confidential_token_account_ixs(
            &payer_pubkey,
            &owner_pubkey,
            &mint_pubkey,
            &token_account_pubkey,
            &token_account_ae_key,
            &token_account_elgamal_keypair,
        )?;

        let tx = self
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &token_account_ixs,
                &[
                    &self.payer,
                    &owner_keypair,
                    &token_account_proof_account.keypair,
                ],
                None,
            )
            .await?;

        Ok(tx)
    }

    pub async fn confidential_mint_to(
        &mut self,
        mint_proof_account: &SignerProofAccount,
        token_account_pubkey: &Pubkey,
        amount: f64,
        decimals: u8,
    ) -> Result<Signature> {
        let payer_pubkey = self.payer.pubkey();
        let authority_pubkey = self.authority.pubkey();

        let mint_pubkey = mint_proof_account.pubkey();

        let current_supply_ciphertext = self.get_mint_confidential_supply(&mint_pubkey).await?;
        let mint_amount = ui_amount_to_amount(amount, decimals);
        let current_supply = self
            .get_mint_decrypted_decryptable_supply(mint_proof_account)
            .await?;
        let new_supply = current_supply + mint_amount;
        let new_decryptable_supply = mint_proof_account.encrypt_supply(new_supply)?;

        let supply_elgamal_keypair = mint_proof_account.get_pod_elgamal_keypair()?;
        let destination_elgamal_pubkey = self
            .get_token_account_elgamal_pubkey(&token_account_pubkey)
            .await?
            .try_into()?;

        let MintProofData {
            equality_proof_data,
            ciphertext_validity_proof_data_with_ciphertext,
            range_proof_data,
        } = mint_split_proof_data(
            &current_supply_ciphertext.try_into()?,
            mint_amount,
            current_supply,
            &supply_elgamal_keypair,
            &destination_elgamal_pubkey,
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

        let _ = self
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &proof_accounts_ixs,
                &[
                    &range_proof_context_state_account,
                    &equality_proof_context_state_account,
                    &ciphertext_validity_proof_context_state_account,
                ],
                Some(&self.payer),
            )
            .await?;

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
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &mint_to_ixs,
                &[&self.authority],
                Some(&self.payer),
            )
            .await?;

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

        let _: Signature = self
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &close_context_state_ixs,
                &[&self.authority],
                Some(&self.payer),
            )
            .await?;

        Ok(mint_to_tx)
    }

    pub async fn apply_pending_balance(
        &mut self,
        owner_keypair: &Keypair,
        token_account_proof_account: &SignerProofAccount,
    ) -> Result<Signature> {
        let owner_pubkey = owner_keypair.pubkey();
        let token_account_pubkey = token_account_proof_account.pubkey();

        let confidential_transfer_account = self
            .get_token_account_credential_transfer_account(&token_account_pubkey)
            .await?;

        let apply_pending_balance_account_info =
            ApplyPendingBalanceAccountInfo::new(&confidential_transfer_account);

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
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &apply_pending_balance_instructions,
                &[owner_keypair],
                Some(&self.payer),
            )
            .await?;

        Ok(apply_pending_balance_tx)
    }

    pub async fn confidential_transfer(
        &mut self,
        owner_keypair: &Keypair,
        source_token_account_proof_account: &SignerProofAccount,
        destination_token_account_pubkey: &Pubkey,
        mint_pubkey: &Pubkey,
        amount: f64,
        decimals: u8,
    ) -> Result<Signature> {
        let owner_pubkey = owner_keypair.pubkey();
        let payer_pubkey = self.payer.pubkey();
        let source_token_account_pubkey = source_token_account_proof_account.pubkey();

        let transfer_amount = ui_amount_to_amount(amount, decimals);

        let confidential_transfer_account = self
            .get_token_account_credential_transfer_account(&source_token_account_pubkey)
            .await?;
        let transfer_account_info = TransferAccountInfo::new(&confidential_transfer_account);

        let new_decryptable_available_balance = transfer_account_info
            .new_decryptable_available_balance(
                transfer_amount,
                &source_token_account_proof_account.get_ae_key()?,
            )
            .map_err(|_| anyhow::anyhow!("decrypt decryptable_available_balance failed"))?;

        let source_elgamal_keypair =
            source_token_account_proof_account.get_pod_elgamal_keypair()?;
        let source_aes_key = source_token_account_proof_account.get_ae_key()?;
        let destination_elgamal_pubkey = self
            .get_token_account_elgamal_pubkey(destination_token_account_pubkey)
            .await?;

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

        // Range Proof Instructions------------------------------------------------------------------------------
        let (range_create_ix, range_verify_ix) =
            get_zk_proof_context_state_account_creation_instructions(
                &payer_pubkey,
                &range_proof_context_state_account.pubkey(),
                &owner_pubkey,
                &range_proof_data,
            )?;

        // Equality Proof Instructions---------------------------------------------------------------------------
        let (equality_create_ix, equality_verify_ix) =
            get_zk_proof_context_state_account_creation_instructions(
                &payer_pubkey,
                &equality_proof_context_state_account.pubkey(),
                &owner_pubkey,
                &equality_proof_data,
            )?;

        // Ciphertext Validity Proof Instructions ----------------------------------------------------------------
        let (cv_create_ix, cv_verify_ix) =
            get_zk_proof_context_state_account_creation_instructions(
                &payer_pubkey,
                &ciphertext_validity_proof_context_state_account.pubkey(),
                &owner_pubkey,
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

        let _ = self
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &proof_accounts_ixs,
                &[
                    // &env.authority,
                    &range_proof_context_state_account,
                    &equality_proof_context_state_account,
                    &ciphertext_validity_proof_context_state_account,
                ],
                Some(&self.payer),
            )
            .await?;

        let equality_proof_data_location =
            ProofLocation::ContextStateAccount(&equality_proof_pubkey);
        let ciphertext_validity_proof_data_location =
            ProofLocation::ContextStateAccount(&ciphertext_validity_proof_pubkey);
        let range_proof_data_location = ProofLocation::ContextStateAccount(&range_proof_pubkey);

        let ixs = confidential_transfer_ixs(
            &owner_pubkey,
            &source_token_account_pubkey,
            &destination_token_account_pubkey,
            &mint_pubkey,
            &new_decryptable_available_balance.try_into()?,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
            equality_proof_data_location,
            ciphertext_validity_proof_data_location,
            range_proof_data_location,
        )?;

        let tx = self
            .program_simulator
            .process_ixs_with_default_compute_limit(&ixs, &[&owner_keypair], Some(&self.payer))
            .await?;

        // Close context states Instructions ---------------------------------------------------------------

        let close_context_state_ixs = create_close_context_state_ixs(
            &[
                equality_proof_pubkey,
                ciphertext_validity_proof_pubkey,
                range_proof_pubkey,
            ],
            &owner_pubkey,
            &payer_pubkey,
        );

        let _ = self
            .program_simulator
            .process_ixs_with_default_compute_limit(
                &close_context_state_ixs,
                &[&owner_keypair],
                Some(&self.payer),
            )
            .await?;

        Ok(tx)
    }
}
