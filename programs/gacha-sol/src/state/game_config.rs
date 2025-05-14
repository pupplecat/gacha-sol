use anchor_lang::prelude::*;

use super::Size;

#[account]
pub struct GameConfig {
    pub authority: Pubkey,
    pub purchase_mint: Pubkey,
    pub reward_mint: Pubkey,
    pub game_vault: Pubkey,
    pub pull_price: u64,
    pub last_pull_id: u64,
}

impl Size for GameConfig {
    const SIZE: usize = 8       // discriminator
        + 32                    // authority
        + 32                    // purchase_mint
        + 32                    // reward_mint
        + 32                    // game_vault
        + 8                    // pull_price
        + 8                    // last_pull_id
        ;
}
