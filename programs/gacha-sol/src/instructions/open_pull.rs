use std::str::FromStr;

use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::{
    token::Token,
    token_2022::{self, TransferChecked},
    token_interface::{Mint, TokenAccount},
};
use spl_token_2022::{
    extension::confidential_transfer::instruction::withdraw,
    solana_zk_sdk::encryption::pod::auth_encryption::PodAeCiphertext,
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;

use crate::{
    error::GachaError,
    event::PullClaimed,
    state::{GameConfig, OpenPullParams, Pull, AE_CIPHERTEXT_MAX_BASE64_LEN},
    utils::{token_2022::Token2022, zk_elgamal_proof_program::ZkElgamalProof},
};

use super::OpenPullInstruction;

pub fn open_pull<'info>(
    ctx: Context<'_, '_, '_, 'info, OpenPull<'info>>,
    params: OpenPullParams,
) -> Result<()> {
    let pull_account = &mut ctx.accounts.pull;

    require!(
        pull_account.buyer == ctx.accounts.buyer.key(),
        GachaError::InvalidBuyer
    );
    require!(!pull_account.claimed, GachaError::PullAlreadyClaimed);

    pull_account.revealed_amount = params.amount;
    pull_account.claimed = true;

    // Withdraw confidential fund, prepare for transferring
    ctx.withdraw_reward(
        params.amount,
        params.decimals,
        params.new_decryptable_available_balance,
    )?;

    // Transfer reward to buyer
    ctx.transfer_reward(params.amount, params.decimals)?;

    // Emit an event
    emit!(PullClaimed {
        id: ctx.accounts.pull.id,
        pull: ctx.accounts.pull.key(),
        buyer: ctx.accounts.buyer.key()
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(params: OpenPullParams)]
pub struct OpenPull<'info> {
    #[account(mut,
        has_one=reward_vault,
        seeds = [b"pull", params.pull_id.to_le_bytes().as_ref()],
        bump
    )]
    pub pull: Account<'info, Pull>,
    #[account(
        has_one=reward_mint,
        has_one=authority,
    )]
    pub game_config: Account<'info, GameConfig>,
    /// CHECK buyer account
    #[account()]
    pub buyer: AccountInfo<'info>,
    /// CHECK reward vault
    #[account(mut,
        // token::token_program = token_2022_program
    )]
    pub reward_vault: AccountInfo<'info>,
    /// CHECK buyer reward token account
    #[account(mut,
        token::mint = reward_mint,
        token::authority = buyer,
        token::token_program = token_2022_program
    )]
    pub buyer_reward_account: InterfaceAccount<'info, TokenAccount>,
    pub reward_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Equality proof account
    #[account(
        owner = zk_elgamal_proof_program.key()
    )]
    pub equality_proof_account: AccountInfo<'info>,

    /// CHECK: Range proof account
    #[account(
        owner = zk_elgamal_proof_program.key()
    )]
    pub range_proof_account: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub zk_elgamal_proof_program: Program<'info, ZkElgamalProof>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> OpenPullInstruction for Context<'_, '_, '_, 'info, OpenPull<'info>> {
    fn transfer_reward(&self, amount: u64, decimals: u8) -> Result<()> {
        let signer_seeds = &self.accounts.pull.get_signer_seeds();
        let signer = &[&signer_seeds[..]];

        let cpi_accounts = TransferChecked {
            from: self.accounts.reward_vault.to_account_info(),
            to: self.accounts.buyer_reward_account.to_account_info(),
            authority: self.accounts.pull.to_account_info(),
            mint: self.accounts.reward_mint.to_account_info(),
        };
        let token_program = self.accounts.token_2022_program.to_account_info().clone();
        let cpi_context = CpiContext::new_with_signer(token_program, cpi_accounts, signer);

        token_2022::transfer_checked(cpi_context, amount, decimals)?;

        Ok(())
    }

    fn withdraw_reward(
        &self,
        amount: u64,
        decimals: u8,
        new_decryptable_available_balance: [u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Result<()> {
        let pull = &self.accounts.pull;

        let signer_seeds = &pull.get_signer_seeds();
        let signer = &[&signer_seeds[..]];

        let new_decryptable_available_balance = PodAeCiphertext::from_str(
            std::str::from_utf8(&new_decryptable_available_balance)
                .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?,
        )
        .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?;

        let equality_proof_data_location =
            ProofLocation::ContextStateAccount(self.accounts.equality_proof_account.key);

        let range_proof_data_location =
            ProofLocation::ContextStateAccount(self.accounts.range_proof_account.key);

        let withdraw_instructions = withdraw(
            self.accounts.token_2022_program.key,
            &self.accounts.reward_vault.key(),
            &self.accounts.reward_mint.key(),
            amount,
            decimals,
            &new_decryptable_available_balance,
            &self.accounts.pull.key(),
            &[],
            equality_proof_data_location,
            range_proof_data_location,
        )?;

        for ix in withdraw_instructions {
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
