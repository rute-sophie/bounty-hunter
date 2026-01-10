use crate::state::{Bounty, Submission};
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

impl SubmitSolution<'_> {
    pub fn handler(ctx: Context<SubmitSolution>, link: String, notes: String) -> Result<()> {
        ctx.accounts.submission.set_inner(Submission {
            bounty: ctx.accounts.bounty.key(),
            link,
            hunter: ctx.accounts.hunter.key(),
            notes,
        });
        Ok(())
    }
}

//deser : 0101010 -> {a: 123, b:321}
//ser : {a: 123, b:321} -> 1010101
