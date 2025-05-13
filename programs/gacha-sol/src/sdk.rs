use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, system_program},
    InstructionData,
};
use anchor_spl::token_2022;

use crate::{
    accounts, instruction,
    pda::{get_game_config_pubkey, get_pull_pubkey, get_reward_vault_pubkey},
    state::{CreatePullParams, AE_CIPHERTEXT_MAX_BASE64_LEN, ELGAMAL_PUBKEY_MAX_BASE64_LEN},
    utils::rent::Rent,
    ID,
};

impl accounts::InitializeGameConfig {
    pub fn populate(
        authority: Pubkey,
        purchase_mint: Pubkey,
        reward_mint: Pubkey,
        game_vault: Pubkey,
        payer: Pubkey,
    ) -> Self {
        let game_config = get_game_config_pubkey();

        Self {
            game_config,
            authority,
            purchase_mint,
            reward_mint,
            game_vault,
            payer,
            system_program: system_program::ID,
        }
    }
}

impl accounts::CreatePull {
    pub fn populate(
        authority: Pubkey,
        reward_mint: Pubkey,
        payer: Pubkey,
        pubkey_validity_proof_data: Pubkey,
        pull_id: u64,
    ) -> Self {
        let game_config = get_game_config_pubkey();
        let pull = get_pull_pubkey(pull_id);
        let reward_vault = get_reward_vault_pubkey(pull);

        Self {
            pull,
            game_config,
            reward_vault,
            reward_mint,
            pubkey_validity_proof_data,
            authority,
            payer,
            system_program: system_program::ID,
            token_program: token_2022::ID,
            rent: Rent::id(),
        }
    }
}

impl instruction::InitializeGameConfig {
    pub fn populate(
        authority: Pubkey,
        purchase_mint: Pubkey,
        reward_mint: Pubkey,
        game_vault: Pubkey,
        payer: Pubkey,
        pull_price: u64,
    ) -> Instruction {
        let initialize_game_config_accounts = accounts::InitializeGameConfig::populate(
            authority,
            purchase_mint,
            reward_mint,
            game_vault,
            payer,
        )
        .to_account_metas(None);

        Instruction {
            program_id: ID,
            accounts: initialize_game_config_accounts,
            data: instruction::InitializeGameConfig {
                params: crate::InitializeGameConfigParams { pull_price },
            }
            .data(),
        }
    }
}

impl instruction::CreatePull {
    pub fn populate(
        authority: Pubkey,
        reward_mint: Pubkey,
        payer: Pubkey,
        pubkey_validity_proof_data: Pubkey,
        pull_id: u64,
        encrypted_amount: [u8; ELGAMAL_PUBKEY_MAX_BASE64_LEN],
        decryptable_zero_balance_base64: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Instruction {
        let create_pull_accounts = accounts::CreatePull::populate(
            authority,
            reward_mint,
            payer,
            pubkey_validity_proof_data,
            pull_id,
        )
        .to_account_metas(None);

        Instruction {
            program_id: ID,
            accounts: create_pull_accounts,
            data: instruction::CreatePull {
                params: CreatePullParams {
                    pull_id,
                    encrypted_amount,
                    decryptable_zero_balance_base64,
                },
            }
            .data(),
        }
    }
}
