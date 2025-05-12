use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, TokenAccount},
    token_2022::Token2022,
};

use crate::state::{GameConfig, InitializeGameConfigParams, Size};

pub fn initialize_game_config<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeGameConfig<'info>>,
    params: InitializeGameConfigParams,
) -> Result<()> {
    let game_config = &mut ctx.accounts.game_config;
    game_config.authority = ctx.accounts.authority.key();
    game_config.purchase_mint = ctx.accounts.purchase_mint.key();
    game_config.reward_mint = ctx.accounts.reward_mint.key();
    game_config.game_vault = ctx.accounts.game_vault.key();
    game_config.pull_price = params.pull_price;
    game_config.last_pull_id = 0;
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeGameConfig<'info> {
    #[account(
        init,
        payer = payer,
        space = GameConfig::SIZE,
        seeds = [b"game_config"],
        bump
    )]
    pub game_config: Account<'info, GameConfig>,
    /// CHECK: Authority account.
    pub authority: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub purchase_mint: Account<'info, Mint>,
    pub reward_mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"game_vault"],
        bump,
        payer = payer,
        token::authority = authority,
        token::mint = purchase_mint
    )]
    pub game_vault: Box<Account<'info, TokenAccount>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
}
