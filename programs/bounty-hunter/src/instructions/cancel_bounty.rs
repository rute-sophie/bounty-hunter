use anchor_lang::prelude::*;

use crate::state::Bounty;

#[derive(Accounts)]
pub struct CancelBounty<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        close = maker,
    )]
    pub bounty: Account<'info, Bounty>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CancelBounty>) -> Result<()> {
    
    Ok(())
}
