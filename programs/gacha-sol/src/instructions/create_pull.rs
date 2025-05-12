use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, TokenAccount},
    token_2022::Token2022,
};

use crate::{
    error::GachaError,
    state::{CreatePullParams, GameConfig, Pull},
};

pub fn create_pull(ctx: Context<CreatePull>, params: CreatePullParams) -> Result<()> {
    let pull_account = &mut ctx.accounts.pull_account;
    pull_account.reward_token_account = ctx.accounts.pull_token_account.key();
    pull_account.buyer = Pubkey::default();
    pull_account.claimed = false;
    pull_account.revealed_amount = 0;

    require!(
        ctx.accounts.game_config.last_pull_id + 1 == params.pull_id,
        GachaError::InvalidPullId
    );

    // TODO: Implement confidential transfer of reward_amount to pull_token_account
    // Requires setting up confidential transfer proofs and invoking Token-2022

    Ok(())
}

#[derive(Accounts)]
#[instruction(params : CreatePullParams)]
pub struct CreatePull<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 1 + 8,
        seeds = [b"pull", game_config.key().as_ref(), &params.pull_id.to_le_bytes()],
        bump
    )]
    pub pull_account: Account<'info, Pull>,
    #[account(mut, has_one = authority)]
    pub game_config: Account<'info, GameConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub pull_token_account: Account<'info, TokenAccount>,
    pub reward_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
    pub rent: Sysvar<'info, Rent>,
}
