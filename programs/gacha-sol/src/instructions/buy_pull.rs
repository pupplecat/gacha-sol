use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{
    error::GachaError,
    event::PullBought,
    state::{GameConfig, Pull},
};

use super::BuyPullInstruction;

pub fn buy_pull<'info>(ctx: Context<'_, '_, '_, 'info, BuyPull<'info>>) -> Result<()> {
    let pull = &mut ctx.accounts.pull;
    let game_config = &ctx.accounts.game_config;

    require!(
        pull.buyer == Pubkey::default(),
        GachaError::PullAlreadyPurchased
    );
    require!(!pull.verified, GachaError::PullNotVerified);
    require!(!pull.claimed, GachaError::PullAlreadyClaimed);

    pull.buyer = ctx.accounts.buyer.key();

    // Transfer
    ctx.transfer_purchase(game_config.pull_price)?;

    // Emit an event
    emit!(PullBought {
        id: ctx.accounts.pull.id,
        pull: ctx.accounts.pull.key(),
        buyer: ctx.accounts.buyer.key()
    });

    Ok(())
}

#[derive(Accounts)]
pub struct BuyPull<'info> {
    #[account(has_one=game_vault)]
    pub game_config: Account<'info, GameConfig>,
    #[account(mut)]
    pub pull: Account<'info, Pull>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut,
        token::mint = purchase_mint
    )]
    pub buyer_purchase_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub game_vault: Account<'info, TokenAccount>,
    pub purchase_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

impl<'info> BuyPullInstruction for Context<'_, '_, '_, 'info, BuyPull<'info>> {
    fn transfer_purchase(&self, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: self
                .accounts
                .buyer_purchase_account
                .to_account_info()
                .clone(),
            to: self.accounts.game_vault.to_account_info().clone(),
            authority: self.accounts.buyer.to_account_info().clone(),
        };
        let token_program = self.accounts.token_program.to_account_info().clone();
        let cpi_context = CpiContext::new(token_program, cpi_accounts);

        token::transfer(cpi_context, amount)?;
        Ok(())
    }
}
