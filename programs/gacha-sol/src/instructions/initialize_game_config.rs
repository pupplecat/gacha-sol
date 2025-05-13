use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use spl_token_2022::{
    extension::{
        confidential_transfer::ConfidentialTransferMint, BaseStateWithExtensions,
        StateWithExtensions,
    },
    state::Mint as Mint2022,
};

use crate::{
    error::GachaError,
    event::GameConfigInitialized,
    state::{GameConfig, InitializeGameConfigParams, Size},
};

use super::InitializeGameConfigInstruction;

pub fn initialize_game_config<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeGameConfig<'info>>,
    params: InitializeGameConfigParams,
) -> Result<()> {
    require!(params.pull_price > 0, GachaError::InvalidZeroPullPrice);

    // Setup game config
    {
        let game_config = &mut ctx.accounts.game_config;
        game_config.authority = ctx.accounts.authority.key();
        game_config.purchase_mint = ctx.accounts.purchase_mint.key();
        game_config.reward_mint = ctx.accounts.reward_mint.key();
        game_config.game_vault = ctx.accounts.game_vault.key();
        game_config.pull_price = params.pull_price;
        game_config.last_pull_id = 0;
    }

    // Verify reward mint
    ctx.verify_reward_mint()?;

    // Emit event
    {
        let game_config = &ctx.accounts.game_config;
        emit!(GameConfigInitialized {
            game_config: ctx.accounts.game_config.key(),
            authority: game_config.authority,
            purchase_mint: game_config.purchase_mint,
            reward_mint: game_config.reward_mint,
            game_vault: game_config.game_vault,
            pull_price: game_config.pull_price
        });
    }

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
    pub game_config: Box<Account<'info, GameConfig>>,
    /// CHECK: Authority account.
    pub authority: AccountInfo<'info>,
    pub purchase_mint: Box<Account<'info, Mint>>,
    /// CHECK: Token-2022 mint with Confidential Transfer extension
    pub reward_mint: AccountInfo<'info>,
    #[account(
        token::mint = purchase_mint,
        token::authority = authority,
    )]
    pub game_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeGameConfigInstruction
    for Context<'_, '_, '_, 'info, InitializeGameConfig<'info>>
{
    fn verify_reward_mint(&self) -> Result<()> {
        let mint_data = self.accounts.reward_mint.data.borrow();
        let mint_state = StateWithExtensions::<Mint2022>::unpack(&mint_data)?;

        require!(
            mint_state
                .get_extension::<ConfidentialTransferMint>()
                .is_ok(),
            GachaError::InvalidRewardMint
        );

        Ok(())
    }
}
