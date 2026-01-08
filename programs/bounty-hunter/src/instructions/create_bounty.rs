use anchor_lang::prelude::*;

use crate::state::Bounty;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct CreateBounty<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        init,
        payer = maker,
        space = Bounty::INIT_SPACE + Bounty::DISCRIMINATOR.len(),
        seeds = [b"bounty", maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump,
    )]
    pub bounty: Account<'info, Bounty>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateBounty>,
    seed: u64,
    description: String,
    link: String,
    reward: u64,
) -> Result<()> {
    ctx.accounts.bounty.set_inner(Bounty {
        seed,
        description,
        link,
        reward,
        bump: ctx.bumps.bounty,
        maker: *ctx.accounts.maker.key,
        accepted_submission: Pubkey::default(),
    });
    Ok(())
}
