use std::str::FromStr;

use anchor_lang::prelude::*;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::{
    extension::{
        confidential_transfer::{
            instruction::{
                pod::PodProofType, ProofContextState, ProofType, ZeroCiphertextProofContext,
            },
            ConfidentialTransferAccount,
        },
        BaseStateWithExtensions, StateWithExtensions,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
    state::Account as Token2022Account,
};
use spl_token_confidential_transfer_ciphertext_arithmetic::subtract;

use crate::{
    error::GachaError,
    event::PullVerified,
    state::{GameConfig, Pull},
    utils::{token_2022::Token2022, zk_elgamal_proof_program::ZkElgamalProof},
};

use super::VerifyPullInstruction;

pub fn verify_pull<'info>(ctx: Context<'_, '_, '_, 'info, VerifyPull<'info>>) -> Result<()> {
    // verify the current reward balance
    ctx.verify_reward_balance()?;

    // Set verification flag
    let pull = &mut ctx.accounts.pull;
    pull.verified = true;

    emit!(PullVerified {
        id: pull.id,
        pull: pull.key()
    });

    Ok(())
}

#[derive(Accounts)]
pub struct VerifyPull<'info> {
    #[account( has_one=authority)]
    pub game_config: Box<Account<'info, GameConfig>>,
    #[account(mut, has_one=reward_vault)]
    pub pull: Box<Account<'info, Pull>>,
    /// CHECK: Token account 2022
    pub reward_vault: AccountInfo<'info>,
    pub authority: Signer<'info>,
    /// CHECK: Zero proof account
    #[account(
        owner = zk_elgamal_proof_program.key()
    )]
    pub zero_ciphertext_proof_context: AccountInfo<'info>,
    pub zk_elgamal_proof_program: Program<'info, ZkElgamalProof>,
    pub token_program: Program<'info, Token2022>,
}

impl<'info> VerifyPullInstruction for Context<'_, '_, '_, 'info, VerifyPull<'info>> {
    fn verify_reward_balance(&self) -> Result<()> {
        let reward_vault = &self.accounts.reward_vault;

        let data = reward_vault.try_borrow_data()?;
        let state = StateWithExtensions::<Token2022Account>::unpack(&data)?;
        let confidential_transfer_account = state.get_extension::<ConfidentialTransferAccount>()?;

        let available_balance: PodElGamalCiphertext =
            confidential_transfer_account.available_balance;

        let expected_amount = PodElGamalCiphertext::from_str(
            std::str::from_utf8(&self.accounts.pull.encrypted_amount)
                .map_err(|_| GachaError::CipherTextBalanceConversionFailed)?,
        )
        .map_err(|_| GachaError::CipherTextBalanceConversionFailed)?;

        // use check verified account method
        let context_state_account_data = self.accounts.zero_ciphertext_proof_context.data.borrow();
        let context_state = pod_from_bytes::<ProofContextState<ZeroCiphertextProofContext>>(
            &context_state_account_data,
        )?;

        require!(
            context_state.proof_type == PodProofType::from(ProofType::ZeroCiphertext),
            GachaError::InvalidProofType
        );
        require!(
            context_state.proof_context.pubkey == confidential_transfer_account.elgamal_pubkey,
            GachaError::InvalidElgamalPubkey
        );
        require!(
            context_state.context_state_authority == self.accounts.authority.key(),
            GachaError::InvalidContextAuthority
        );

        let remaining_balance = subtract(&available_balance, &expected_amount)
            .ok_or(GachaError::CiphertextArithmeticFailed)?;

        require!(
            remaining_balance == context_state.proof_context.ciphertext,
            GachaError::CiphertextZeroBalanceMismatch
        );

        Ok(())
    }
}
