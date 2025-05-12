use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, TokenAccount},
    token_2022::Token2022,
};

use crate::{error::GachaError, state::Pull};

pub fn claim_pull(ctx: Context<ClaimPull>) -> Result<()> {
    let pull_account = &mut ctx.accounts.pull_account;

    require!(
        pull_account.buyer == ctx.accounts.buyer.key(),
        GachaError::InvalidBuyer
    );
    require!(!pull_account.claimed, GachaError::PullAlreadyClaimed);

    // TODO: Transfer reward from pull_token_account to buyer_reward_account
    // Decrypt and reveal reward amount, update revealed_amount
    pull_account.revealed_amount = 0; // Placeholder
    pull_account.claimed = true;

    Ok(())
}

#[derive(Accounts)]
pub struct ClaimPull<'info> {
    #[account(mut)]
    pub pull_account: Account<'info, Pull>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub pull_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_reward_account: Account<'info, TokenAccount>,
    pub reward_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
}
