use anchor_lang::prelude::*;

use crate::{error::BountyHunterErrors, state::Bounty};

#[derive(Accounts)]
pub struct CancelBounty<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        close = maker,
        has_one = maker @ BountyHunterErrors::InvalidBountyAuthority,
    )]
    pub bounty: Account<'info, Bounty>,

    pub system_program: Program<'info, System>,
}

impl CancelBounty<'_> {
    pub fn handler(_ctx: Context<CancelBounty>) -> Result<()> {
        Ok(())
    }
}
