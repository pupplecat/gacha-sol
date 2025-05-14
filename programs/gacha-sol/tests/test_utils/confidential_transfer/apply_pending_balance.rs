use anyhow::{Ok, Result};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_2022::extension::confidential_transfer::{instruction, DecryptableBalance};

use super::token_2022_program_id;

pub fn apply_pending_balance_ixs(
    owner_pubkey: &Pubkey,
    token_account_pubkey: &Pubkey,
    new_decryptable_available_balance: &DecryptableBalance,
    expected_pending_balance_credit_counter: u64,
) -> Result<Vec<Instruction>> {
    let apply_pending_balance_instruction = instruction::apply_pending_balance(
        &token_2022_program_id(),
        &token_account_pubkey,
        expected_pending_balance_credit_counter, // Expected number of times the pending balance has been credited
        &new_decryptable_available_balance, // Cipher text of the new decryptable available balance
        owner_pubkey,
        &[],
    )?;

    Ok(vec![apply_pending_balance_instruction])
}
