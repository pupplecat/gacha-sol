use anchor_lang::prelude::*;

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct InitializeGameConfigParams {
    pub pull_price: u64,
}
