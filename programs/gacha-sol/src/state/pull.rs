use anchor_lang::prelude::*;

use super::Size;

#[account]
pub struct Pull {
    pub reward_token_account: Pubkey,
    pub buyer: Pubkey,
    pub claimed: bool,
    pub revealed_amount: u64,
}

impl Size for Pull {
    const SIZE: usize = 8       // discriminator
        + 32                    // reward_token_account
        + 32                    // buyer
        + 1                    // claimed
        + 8                    // revealed_amount
        ;
}
