use anchor_lang::prelude::*;

use crate::state::Bounty;

//use anchor_spl::token::{Token};
use anchor_spl::{associated_token::AssociatedToken, token_interface::TokenInterface};
use anchor_spl::token_interface::Mint;
use anchor_spl::token_interface::TokenAccount;

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

    pub mint: InterfaceAccount<'info, Mint>,

    // the token account associated with the maker and mint used to deposit tokens in the vault
    #[account(
            mut,
            associated_token::mint = mint,
            associated_token::authority = bounty,
            associated_token::token_program = token_program,
        )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    // the token account associated with the escrow and mint where deposit tokens are parked
    #[account(
            init,
            payer = maker,
            associated_token::mint = mint,
            associated_token::authority = bounty,
            associated_token::token_program = token_program
        )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>
}

impl CreateBounty<'_> {
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
}
