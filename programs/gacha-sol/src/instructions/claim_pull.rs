use std::str::FromStr;

use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::token::{Mint, TokenAccount};
use spl_token_2022::{
    extension::confidential_transfer::instruction::transfer,
    solana_zk_sdk::encryption::pod::{
        auth_encryption::PodAeCiphertext, elgamal::PodElGamalCiphertext,
    },
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;

use crate::{
    error::GachaError,
    event::PullClaimed,
    state::{GameConfig, Pull},
    utils::{token_2022::Token2022, zk_elgamal_proof_program::ZkElgamalProof},
};

use super::ClaimPullInstruction;

pub fn claim_pull<'info>(ctx: Context<'_, '_, '_, 'info, ClaimPull<'info>>) -> Result<()> {
    let pull_account = &mut ctx.accounts.pull;

    require!(
        pull_account.buyer == ctx.accounts.buyer.key(),
        GachaError::InvalidBuyer
    );
    require!(!pull_account.claimed, GachaError::PullAlreadyClaimed);

    // TODO: Transfer reward from pull_token_account to buyer_reward_account
    // Decrypt and reveal reward amount, update revealed_amount
    pull_account.revealed_amount = 0; // Placeholder
    pull_account.claimed = true;

    ctx.transfer_reward()?;

    // Emit an event
    emit!(PullClaimed {
        id: ctx.accounts.pull.id,
        pull: ctx.accounts.pull.key(),
        buyer: ctx.accounts.buyer.key()
    });

    Ok(())
}

#[derive(Accounts)]
pub struct ClaimPull<'info> {
    #[account(mut,
        has_one=reward_vault,
        has_one=equality_proof_account,
        has_one=ciphertext_validity_proof_account,
        has_one=range_proof_account,
    )]
    pub pull: Account<'info, Pull>,
    #[account(
        has_one=reward_mint,
    )]
    pub game_config: Account<'info, GameConfig>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub reward_vault: Account<'info, TokenAccount>,
    #[account(mut,
        token::mint = reward_mint,
        token::authority = buyer,
    )]
    pub buyer_reward_account: Account<'info, TokenAccount>,
    pub reward_mint: Account<'info, Mint>,

    /// CHECK: Equality proof account
    #[account(
        owner = zk_elgamal_proof_program.key()
    )]
    pub equality_proof_account: AccountInfo<'info>,

    /// CHECK: Ciphertext validity proof account
    #[account(
        owner = zk_elgamal_proof_program.key()
    )]
    pub ciphertext_validity_proof_account: AccountInfo<'info>,

    /// CHECK: Range proof account
    #[account(
        owner = zk_elgamal_proof_program.key()
    )]
    pub range_proof_account: AccountInfo<'info>,
    pub zk_elgamal_proof_program: Program<'info, ZkElgamalProof>,
    pub token_program: Program<'info, Token2022>,
}

impl<'info> ClaimPullInstruction for Context<'_, '_, '_, 'info, ClaimPull<'info>> {
    fn transfer_reward(&self) -> Result<()> {
        let pull = &self.accounts.pull;

        let signer_seeds = &pull.get_signer_seeds();
        let signer = &[&signer_seeds[..]];

        let new_decryptable_available_balance = PodAeCiphertext::from_str(
            std::str::from_utf8(&pull.final_decryptable_available_balance)
                .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?,
        )
        .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?;

        let transfer_amount_auditor_ciphertext_lo = PodElGamalCiphertext::from_str(
            std::str::from_utf8(&pull.transfer_amount_auditor_ciphertext_lo)
                .map_err(|_| GachaError::CipherTextBalanceConversionFailed)?,
        )
        .map_err(|_| GachaError::CipherTextBalanceConversionFailed)?;

        let transfer_amount_auditor_ciphertext_hi = PodElGamalCiphertext::from_str(
            std::str::from_utf8(&pull.transfer_amount_auditor_ciphertext_hi)
                .map_err(|_| GachaError::CipherTextBalanceConversionFailed)?,
        )
        .map_err(|_| GachaError::CipherTextBalanceConversionFailed)?;

        let equality_proof_data_location =
            ProofLocation::ContextStateAccount(&pull.equality_proof_account);
        let ciphertext_validity_proof_data_location =
            ProofLocation::ContextStateAccount(&pull.ciphertext_validity_proof_account);
        let range_proof_data_location =
            ProofLocation::ContextStateAccount(&pull.range_proof_account);

        let ixs = transfer(
            self.accounts.token_program.key,
            &self.accounts.reward_vault.key(),
            &self.accounts.reward_mint.key(),
            &self.accounts.buyer_reward_account.key(),
            &new_decryptable_available_balance,
            &transfer_amount_auditor_ciphertext_lo,
            &transfer_amount_auditor_ciphertext_hi,
            &pull.key(),
            &[],
            equality_proof_data_location,
            ciphertext_validity_proof_data_location,
            range_proof_data_location,
        )?;

        for ix in ixs {
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
                        k if k == self.accounts.buyer_reward_account.key() => {
                            self.accounts.buyer_reward_account.to_account_info().clone()
                        }
                        k if k == self.accounts.equality_proof_account.key() => self
                            .accounts
                            .equality_proof_account
                            .to_account_info()
                            .clone(),
                        k if k == self.accounts.ciphertext_validity_proof_account.key() => self
                            .accounts
                            .ciphertext_validity_proof_account
                            .to_account_info()
                            .clone(),
                        k if k == self.accounts.range_proof_account.key() => {
                            self.accounts.range_proof_account.to_account_info().clone()
                        }
                        k if k == self.accounts.zk_elgamal_proof_program.key() => self
                            .accounts
                            .zk_elgamal_proof_program
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
}
