use anchor_lang::prelude::*;

pub mod error;
pub mod event;
pub mod pda;
pub mod sdk;

mod instructions;
mod utils;

pub mod state;

use instructions::*;
use state::*;

declare_id!("GaChaWso1g5y2Rby6pwLSRxezyoxtbaMDyb5sqwYvNGq");

#[program]
pub mod gacha_sol {

    use super::*;

    pub fn initialize_game_config<'info>(
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

    pub fn apply_pull_pending_balance<'info>(
        ctx: Context<'_, '_, '_, 'info, ApplyPullPendingBalance<'info>>,
        params: ApplyPullPendingBalanceParams,
    ) -> Result<()> {
        instructions::apply_pull_pending_balance(ctx, params)
    }

    pub fn verify_pull<'info>(ctx: Context<'_, '_, '_, 'info, VerifyPull<'info>>) -> Result<()> {
        instructions::verify_pull(ctx)
    }

    pub fn buy_pull<'info>(
        ctx: Context<'_, '_, '_, 'info, BuyPull<'info>>,
        params: BuyPullParams,
    ) -> Result<()> {
        instructions::buy_pull(ctx, params)
    }

    pub fn open_pull<'info>(
        ctx: Context<'_, '_, '_, 'info, OpenPull<'info>>,
        params: OpenPullParams,
    ) -> Result<()> {
        instructions::open_pull(ctx, params)
    }
}
