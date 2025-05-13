use std::str::FromStr;

use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use spl_token_2022::{
    extension::{
        confidential_transfer::{instruction::apply_pending_balance, ConfidentialTransferAccount},
        BaseStateWithExtensions, StateWithExtensions,
    },
    solana_zk_sdk::encryption::pod::auth_encryption::PodAeCiphertext,
    state::Account as Token2022Account,
};

use crate::{
    error::GachaError,
    event::PendingBalanceApplied,
    state::{ApplyPullPendingBalanceParams, GameConfig, Pull, AE_CIPHERTEXT_MAX_BASE64_LEN},
    utils::token_2022::Token2022,
};

use super::ApplyPullPendingBalanceInstruction;

pub fn apply_pull_pending_balance<'info>(
    ctx: Context<'_, '_, '_, 'info, ApplyPullPendingBalance<'info>>,
    params: ApplyPullPendingBalanceParams,
) -> Result<()> {
    // Apply pending balance
    let pull_id = ctx.accounts.pull.id;
    let pull_pubkey = ctx.accounts.pull.key();

    ctx.apply_pending_balance(&params.new_decryptable_available_balance)?;

    emit!(PendingBalanceApplied {
        id: pull_id,
        pull: pull_pubkey
    });

    Ok(())
}

#[derive(Accounts)]
pub struct ApplyPullPendingBalance<'info> {
    #[account(has_one=authority)]
    pub game_config: Box<Account<'info, GameConfig>>,
    #[account(has_one=reward_vault)]
    pub pull: Box<Account<'info, Pull>>,
    /// CHECK: Token account 2022
    #[account(mut)]
    pub reward_vault: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

impl<'info> ApplyPullPendingBalanceInstruction
    for Context<'_, '_, '_, 'info, ApplyPullPendingBalance<'info>>
{
    fn apply_pending_balance(
        &self,
        new_decryptable_available_balance: &[u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Result<()> {
        let expected_pending_credit_counter = {
            let reward_vault = &self.accounts.reward_vault;

            let data = reward_vault.try_borrow_data()?;
            let state = StateWithExtensions::<Token2022Account>::unpack(&data)?;
            let confidential_transfer_account =
                state.get_extension::<ConfidentialTransferAccount>()?;

            let expected_pending_credit_counter = confidential_transfer_account
                .pending_balance_credit_counter
                .into();
            expected_pending_credit_counter
        };

        let new_decryptable_available_balance = PodAeCiphertext::from_str(
            std::str::from_utf8(new_decryptable_available_balance)
                .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?,
        )
        .map_err(|_| GachaError::DecryptableBalanceConversionFailed)?;

        let apply_pending_balance_instructions = apply_pending_balance(
            self.accounts.token_program.key,
            self.accounts.reward_vault.key,
            expected_pending_credit_counter,
            &new_decryptable_available_balance,
            &self.accounts.pull.key(),
            &[],
        )?;

        let accounts = vec![
            self.accounts.reward_vault.to_account_info(),
            self.accounts.pull.to_account_info(),
        ];

        let signer_seeds = &self.accounts.pull.get_signer_seeds();
        let signer = &[&signer_seeds[..]];

        invoke_signed(&apply_pending_balance_instructions, &accounts, signer)?;

        Ok(())
    }
}
