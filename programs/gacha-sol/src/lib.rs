use anchor_lang::prelude::*;

pub mod error;
pub mod event;
pub mod sdk;

mod instructions;
mod pda;

pub mod state;

use instructions::*;
use state::*;

declare_id!("B71jh4j5NX3cXyKJ92YjpNApiHk93x2UKXPSqicY5jz1");

#[program]
pub mod gacha_sol {

    use super::*;

    pub fn initialize_payment_vault<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeGameConfig<'info>>,
        params: InitializeGameConfigParams,
    ) -> Result<()> {
        instructions::initialize_game_config(ctx, params)
    }

    pub fn create_pull<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePull<'info>>,
        params: CreatePullParams,
    ) -> Result<()> {
        instructions::create_pull(ctx, params)
    }

    pub fn buy_pull<'info>(ctx: Context<'_, '_, '_, 'info, BuyPull<'info>>) -> Result<()> {
        instructions::buy_pull(ctx)
    }

    pub fn claim_pull<'info>(ctx: Context<'_, '_, '_, 'info, ClaimPull<'info>>) -> Result<()> {
        instructions::claim_pull(ctx)
    }
}
