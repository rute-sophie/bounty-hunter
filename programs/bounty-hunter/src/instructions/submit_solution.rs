use crate::{Bounty, Submission};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SubmitSolution<'info> {
    #[account(mut)]
    pub hunter: Signer<'info>,
    #[account()]
    pub bounty: Account<'info, Bounty>,
    #[account(
        init_if_needed,
        payer = hunter,
        space = Submission::INIT_SPACE + Submission::DISCRIMINATOR.len(),
        seeds = [b"submission", hunter.key().as_ref(), bounty.key().as_ref()],
        bump,
    )]
    pub submission: Account<'info, Submission>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<SubmitSolution>) -> Result<()> {
    Ok(())
}
