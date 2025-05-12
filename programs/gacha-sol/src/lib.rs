use anchor_lang::prelude::*;

declare_id!("B71jh4j5NX3cXyKJ92YjpNApiHk93x2UKXPSqicY5jz1");

#[program]
pub mod gacha_sol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
