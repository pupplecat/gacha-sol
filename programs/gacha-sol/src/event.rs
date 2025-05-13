use anchor_lang::prelude::*;

use crate::state::ELGAMAL_CIPHERTEXT_LEN;

/// Event emitted when a game config is initialized
#[event]
pub struct GameConfigInitialized {
    pub game_config: Pubkey,
    pub authority: Pubkey,
    pub purchase_mint: Pubkey,
    pub reward_mint: Pubkey,
    pub game_vault: Pubkey,
    pub pull_price: u64,
}

/// Event emitted when a pull is created
#[event]
pub struct PullCreated {
    pub id: u64,
    pub pull: Pubkey,
    pub encrypted_amount: [u8; ELGAMAL_CIPHERTEXT_LEN],
}

/// Event emitted when a pull is created
#[event]
pub struct PullVerified {
    pub id: u64,
    pub pull: Pubkey,
}

/// Event emitted when a pull is created
#[event]
pub struct PullBought {
    pub id: u64,
    pub pull: Pubkey,
    pub buyer: Pubkey,
}

/// Event emitted when a pull is created
#[event]
pub struct PullClaimed {
    pub id: u64,
    pub pull: Pubkey,
    pub buyer: Pubkey,
}
