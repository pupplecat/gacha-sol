use anchor_lang::prelude::*;

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct CreatePullParams {
    pub pull_id: u64,
    pub reward_amount: u64, // TODO: Should be confidential
}
