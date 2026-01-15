use anchor_lang::prelude::*;

use crate::{
    error::BountyHunterErrors,
    state::{Bounty, Submission},
};

#[derive(Accounts)]
pub struct AcceptSolution<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(
        mut,
        has_one = maker @ BountyHunterErrors::InvalidBountyAuthority,
        constraint = bounty.accepted_submission == Pubkey::default() @ BountyHunterErrors::BountyClosed
        //constraint = bounty.maker == maker.key() @ BountyHunterErrors::InvalidBountyAuthority
    )]
    pub bounty: Account<'info, Bounty>,
    #[account(
        has_one = bounty @ BountyHunterErrors::BountyAndSubmissionMismatch, //only works for pubkeys
        //constraint = submission.bounty == bounty.key() @ BountyHunterErrors::BountyAndSubmissionMismatch
    )]
    pub submission: Account<'info, Submission>,
}

impl AcceptSolution<'_> {
    pub fn handler(ctx: Context<AcceptSolution>) -> Result<()> {
        //require!(ctx.accounts.bounty.maker == ctx.accounts.maker.key(), BountyHunterErrors::InvalidBountyAuthority);
        //require!(ctx.accounts.submission.bounty == ctx.accounts.bounty.key(), BountyHunterErrors::BountyAndSubmissionMismatch);

        ctx.accounts.bounty.accepted_submission = ctx.accounts.submission.key();
        Ok(())
    }
}
