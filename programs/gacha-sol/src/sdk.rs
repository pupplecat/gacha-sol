use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, system_program},
    InstructionData,
};
use anchor_spl::{token, token_2022};

use crate::{
    accounts, instruction,
    pda::{get_game_config_pubkey, get_pull_pubkey, get_reward_vault_pubkey},
    state::{
        ApplyPullPendingBalanceParams, BuyPullParams, CreatePullParams, OpenPullParams,
        AE_CIPHERTEXT_MAX_BASE64_LEN, ELGAMAL_PUBKEY_MAX_BASE64_LEN,
    },
    utils::{rent::Rent, zk_elgamal_proof_program::ZkElgamalProof},
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

impl accounts::ApplyPullPendingBalance {
    pub fn populate(authority: Pubkey, pull_id: u64) -> Self {
        let game_config = get_game_config_pubkey();
        let pull = get_pull_pubkey(pull_id);
        let reward_vault = get_reward_vault_pubkey(pull);

        Self {
            game_config,
            pull,
            reward_vault,
            authority,
            token_program: token_2022::ID,
        }
    }
}

impl accounts::VerifyPull {
    pub fn populate(
        authority: Pubkey,
        zero_ciphertext_proof_context: Pubkey,
        pull_id: u64,
    ) -> Self {
        let game_config = get_game_config_pubkey();
        let pull = get_pull_pubkey(pull_id);
        let reward_vault = get_reward_vault_pubkey(pull);

        Self {
            game_config,
            pull,
            reward_vault,
            authority,
            zero_ciphertext_proof_context,
            zk_elgamal_proof_program: ZkElgamalProof::id(),
            token_program: token_2022::ID,
        }
    }
}

impl accounts::BuyPull {
    pub fn populate(
        buyer: Pubkey,
        buyer_purchase_account: Pubkey,
        game_vault: Pubkey,
        purchase_mint: Pubkey,
        pull_id: u64,
    ) -> Self {
        let game_config = get_game_config_pubkey();
        let pull = get_pull_pubkey(pull_id);

        Self {
            game_config,
            pull,
            buyer,
            buyer_purchase_account,
            game_vault,
            purchase_mint,
            token_program: token::ID,
        }
    }
}

impl accounts::OpenPull {
    pub fn populate(
        buyer: Pubkey,
        buyer_reward_account: Pubkey,
        reward_mint: Pubkey,
        equality_proof_account: Pubkey,
        range_proof_account: Pubkey,
        authority: Pubkey,
        pull_id: u64,
    ) -> Self {
        let game_config = get_game_config_pubkey();
        let pull = get_pull_pubkey(pull_id);
        let reward_vault = get_reward_vault_pubkey(pull);

        Self {
            pull,
            game_config,
            buyer,
            reward_vault,
            buyer_reward_account,
            reward_mint,
            equality_proof_account,
            range_proof_account,
            authority,
            zk_elgamal_proof_program: ZkElgamalProof::id(),
            token_program: token::ID,
            token_2022_program: token_2022::ID,
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

impl instruction::ApplyPullPendingBalance {
    pub fn populate(
        authority: Pubkey,
        pull_id: u64,
        new_decryptable_available_balance: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Instruction {
        let apply_pull_pending_balance_accounts =
            accounts::ApplyPullPendingBalance::populate(authority, pull_id).to_account_metas(None);

        Instruction {
            program_id: ID,
            accounts: apply_pull_pending_balance_accounts,
            data: instruction::ApplyPullPendingBalance {
                params: ApplyPullPendingBalanceParams {
                    new_decryptable_available_balance,
                },
            }
            .data(),
        }
    }
}

impl instruction::VerifyPull {
    pub fn populate(
        authority: Pubkey,
        zero_ciphertext_proof_context: Pubkey,
        pull_id: u64,
    ) -> Instruction {
        let verify_pull_accounts =
            accounts::VerifyPull::populate(authority, zero_ciphertext_proof_context, pull_id)
                .to_account_metas(None);

        Instruction {
            program_id: ID,
            accounts: verify_pull_accounts,
            data: instruction::VerifyPull {}.data(),
        }
    }
}

impl instruction::BuyPull {
    pub fn populate(
        buyer: Pubkey,
        buyer_purchase_account: Pubkey,
        game_vault: Pubkey,
        purchase_mint: Pubkey,
        pull_id: u64,
    ) -> Instruction {
        let buy_pull_accounts = accounts::BuyPull::populate(
            buyer,
            buyer_purchase_account,
            game_vault,
            purchase_mint,
            pull_id,
        )
        .to_account_metas(None);

        Instruction {
            program_id: ID,
            accounts: buy_pull_accounts,
            data: instruction::BuyPull {
                params: BuyPullParams { pull_id },
            }
            .data(),
        }
    }
}

impl instruction::OpenPull {
    pub fn populate(
        buyer: Pubkey,
        buyer_reward_account: Pubkey,
        reward_mint: Pubkey,
        equality_proof_account: Pubkey,
        range_proof_account: Pubkey,
        authority: Pubkey,
        pull_id: u64,
        amount: u64,
        decimals: u8,
        new_decryptable_available_balance: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Instruction {
        let open_pull_accounts = accounts::OpenPull::populate(
            buyer,
            buyer_reward_account,
            reward_mint,
            equality_proof_account,
            range_proof_account,
            authority,
            pull_id,
        )
        .to_account_metas(None);

        Instruction {
            program_id: ID,
            accounts: open_pull_accounts,
            data: instruction::OpenPull {
                params: OpenPullParams {
                    pull_id,
                    amount,
                    decimals,
                    new_decryptable_available_balance,
                },
            }
            .data(),
        }
    }
}
