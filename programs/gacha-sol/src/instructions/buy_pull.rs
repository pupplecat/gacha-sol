use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, TokenAccount},
    token_2022::Token2022,
};

use crate::{
    error::GachaError,
    state::{GameConfig, Pull},
};

pub fn buy_pull(ctx: Context<BuyPull>) -> Result<()> {
    let pull_account = &mut ctx.accounts.pull_account;
    let game_config = &ctx.accounts.game_config;

    require!(
        pull_account.buyer == Pubkey::default(),
        GachaError::PullAlreadyPurchased
    );
    require!(!pull_account.claimed, GachaError::PullAlreadyClaimed);

    pull_account.buyer = ctx.accounts.buyer.key();

    // TODO: Transfer game_config.pull_price USDC from buyer to vault using confidential transfer

    Ok(())
}

#[derive(Accounts)]
pub struct BuyPull<'info> {
    #[account(mut)]
    pub pull_account: Account<'info, Pull>,
    pub game_config: Account<'info, GameConfig>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub buyer_purchase_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_purchase_account: Account<'info, TokenAccount>,
    pub purchase_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
}
