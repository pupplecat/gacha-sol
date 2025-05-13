use anchor_lang::prelude::*;

use crate::{error::GachaError, state::AE_CIPHERTEXT_MAX_BASE64_LEN};

pub trait InitializeGameConfigInstruction {
    fn verify_reward_mint(&self) -> Result<()>;
}

pub trait CreatePullInstruction {
    fn create_and_configure_reward_vault(
        &self,
        decryptable_zero_balance_base64: &[u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Result<()> {
        self.initialize_token_account_with_extension()?;
        self.configure_token_account(decryptable_zero_balance_base64)
            .map_err(|_| GachaError::ConfigureTokenAccountFailed)?;
        self.close_context_states()
            .map_err(|_| GachaError::CloseContextStateFailed)?;

        msg!(
            "Configured confidential transfer for token account: {}",
            self.get_reward_vault_pubkey()
        );

        Ok(())
    }

    fn initialize_token_account_with_extension(&self) -> Result<()>;

    fn configure_token_account(
        &self,
        decryptable_zero_balance_base64: &[u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Result<()>;

    fn close_context_states(&self) -> Result<()>;

    fn get_reward_vault_pubkey(&self) -> Pubkey;
}

pub trait VerifyPullInstruction {
    fn apply_pending_balance(
        &self,
        new_decryptable_available_balance: &[u8; AE_CIPHERTEXT_MAX_BASE64_LEN],
    ) -> Result<()>;

    fn verify_reward_mint(&self) -> Result<()>;
}

pub trait BuyPullInstruction {
    fn transfer_purchase(&self, amount: u64) -> Result<()>;
}

pub trait ClaimPullInstruction {
    fn transfer_reward(&self) -> Result<()>;
}
