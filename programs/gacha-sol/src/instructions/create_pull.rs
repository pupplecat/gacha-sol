use std::str::FromStr as _;

use anchor_lang::{
    prelude::*,
    solana_program::{
        self,
        program::{invoke, invoke_signed},
    },
};

use crate::{
    error::GachaError,
    event::PullCreated,
    state::{CreatePullParams, GameConfig, Pull, Size, AE_CIPHERTEXT_MAX_BASE64_LEN},
    utils::token_2022::Token2022,
};
use spl_token_2022::{
    extension::{confidential_transfer::instruction::configure_account, ExtensionType},
    instruction::initialize_account3,
    solana_zk_sdk::{
        encryption::pod::auth_encryption::PodAeCiphertext,
        zk_elgamal_proof_program::instruction::{close_context_state, ContextStateInfo},
    },
    state::Account as TokenAccount,
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;

use super::CreatePullInstruction;

pub fn create_pull<'info>(
    ctx: Context<'_, '_, '_, 'info, CreatePull<'info>>,
    params: CreatePullParams,
) -> Result<()> {
    require!(
        ctx.accounts.game_config.last_pull_id + 1 == params.pull_id,
        GachaError::InvalidPullId
    );

    {
        let pull = &mut ctx.accounts.pull;
        pull.id = params.pull_id;
        pull.reward_vault = ctx.accounts.reward_vault.key();
        pull.encrypted_amount = params.encrypted_amount;
        pull.buyer = Pubkey::default();
        pull.verified = false;
        pull.claimed = false;
        pull.revealed_amount = 0;
        pull.bump = ctx.bumps.pull;
    }

    {
        let game_config = &mut ctx.accounts.game_config;
        game_config.last_pull_id = params.pull_id;
    }

    ctx.create_and_configure_reward_vault(&params.decryptable_zero_balance_base64)?;

    // Emit an event
    {
        let pull = &ctx.accounts.pull;
        emit!(PullCreated {
            id: pull.id,
            pull: ctx.accounts.pull.key(),
            encrypted_amount: pull.encrypted_amount
        });
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(params : CreatePullParams)]
pub struct CreatePull<'info> {
    #[account(
        init,
        payer = payer,
        space = Pull::SIZE,
        seeds = [b"pull", params.pull_id.to_le_bytes().as_ref()],
        bump
    )]
    pub pull: Account<'info, Pull>,

    #[account(mut, has_one = authority, has_one=reward_mint)]
    pub game_config: Box<Account<'info, GameConfig>>,

    /// CHECK: Token account to be internally created and initialized
    #[account(
        mut,
        seeds = [b"reward_vault", pull.key().as_ref()],
        bump,
    )]
    pub reward_vault: AccountInfo<'info>,

    /// CHECK: Token-2022 mint with Confidential Transfer extension
    pub reward_mint: AccountInfo<'info>,

    /// CHECK: A PubkeyValidityProofData account
    pub pubkey_validity_proof_data: AccountInfo<'info>,

    pub authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> CreatePullInstruction for Context<'_, '_, '_, 'info, CreatePull<'info>> {
    fn initialize_token_account_with_extension(&self) -> Result<()> {
        let required_space = spl_token_2022::extension::ExtensionType::try_calculate_account_len::<
            TokenAccount,
        >(&[ExtensionType::ConfidentialTransferAccount])?;

        let lamports = self.accounts.rent.minimum_balance(required_space);
        let pull_pubkey = self.accounts.pull.key();
        let reward_vault_pubkey = self.accounts.reward_vault.key();
        let mint_pubkey = self.accounts.reward_mint.key();
        let payer_pubkey = self.accounts.payer.key();
        let token_program_id = self.accounts.token_program.key();

        let create_account_ix = solana_program::system_instruction::create_account(
            &payer_pubkey,
            &reward_vault_pubkey,
            lamports,
            required_space as u64,
            &token_program_id,
        );

        let init_account_ix = initialize_account3(
            &token_program_id,
            &reward_vault_pubkey,
            &mint_pubkey,
            &pull_pubkey,
        )?;

        let reward_vault_seeds = &[
            b"reward_vault",
            pull_pubkey.as_ref(),
            &[self.bumps.reward_vault],
        ];

        invoke_signed(
            &create_account_ix,
            &[
                self.accounts.payer.to_account_info(),
                self.accounts.reward_vault.to_account_info(),
                self.accounts.system_program.to_account_info(),
            ],
            &[&reward_vault_seeds[..]],
        )?;

        invoke(
            &init_account_ix,
            &[
                self.accounts.reward_vault.to_account_info(),
                self.accounts.reward_mint.to_account_info(),
                self.accounts.pull.to_account_info(),
                self.accounts.rent.to_account_info(),
            ],
        )?;

        Ok(())
    }

    fn configure_token_account(
        &self,
        decryptable_zero_balance_base64: &[u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Result<()> {
        let signer_seeds = &self.accounts.pull.get_signer_seeds();
        let signer = &[&signer_seeds[..]];

        let decryptable_zero_balance = PodAeCiphertext::from_str(
            std::str::from_utf8(decryptable_zero_balance_base64)
                .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?,
        )
        .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?;

        let proof_data_location =
            ProofLocation::ContextStateAccount(self.accounts.pubkey_validity_proof_data.key);

        let configure_account_ixs = configure_account(
            &self.accounts.token_program.key,
            &self.accounts.reward_vault.key(),
            &self.accounts.reward_mint.key(),
            &decryptable_zero_balance,
            65536, // maximum_pending_balance_credit_counter
            &self.accounts.pull.key(),
            &[],
            proof_data_location,
        )?;

        for ix in configure_account_ixs {
            let accounts = ix
                .accounts
                .iter()
                .map(|acc| {
                    Ok(match acc.pubkey {
                        k if k == self.accounts.reward_vault.key() => {
                            self.accounts.reward_vault.to_account_info().clone()
                        }
                        k if k == self.accounts.reward_mint.key() => {
                            self.accounts.reward_mint.to_account_info().clone()
                        }
                        k if k == self.accounts.pubkey_validity_proof_data.key() => self
                            .accounts
                            .pubkey_validity_proof_data
                            .to_account_info()
                            .clone(),
                        k if k == self.accounts.pull.key() => {
                            self.accounts.pull.to_account_info().clone()
                        }
                        _ => return Err(GachaError::InvalidAccount.into()),
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            invoke_signed(&ix, &accounts, signer)?;
        }

        Ok(())
    }

    fn close_context_states(&self) -> Result<()> {
        let signer_seeds = &self.accounts.pull.get_signer_seeds();
        let signer = &[&signer_seeds[..]];

        let instruction = close_context_state(
            ContextStateInfo {
                context_state_account: self.accounts.pubkey_validity_proof_data.key,
                context_state_authority: &self.accounts.pull.key(),
            },
            self.accounts.payer.key,
        );

        let accounts = vec![
            self.accounts.pubkey_validity_proof_data.to_account_info(),
            self.accounts.payer.to_account_info(),
            self.accounts.pull.to_account_info(),
        ];

        invoke_signed(&instruction, &accounts, signer)?;

        Ok(())
    }

    fn get_reward_vault_pubkey(&self) -> Pubkey {
        self.accounts.reward_vault.key()
    }
}
