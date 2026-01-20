use anchor_lang::prelude::*;
use anchor_spl::token::{close_account, transfer_checked, CloseAccount, TransferChecked};
use anchor_spl::token_interface::Mint;
use anchor_spl::token_interface::TokenAccount;
use anchor_spl::{associated_token::AssociatedToken, token_interface::TokenInterface};

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
        has_one = maker @ BountyHunterErrors::InvalidBountyAuthority,      //alternativa constraint = bounty.maker == maker.key() @ BountyHunterErrors::InvalidBountyAuthority
        has_one = mint @ BountyHunterErrors::InvalidMint,
        constraint = bounty.accepted_submission == Pubkey::default() @ BountyHunterErrors::BountyClosed
    )]
    pub bounty: Account<'info, Bounty>,
    #[account(
        has_one = bounty @ BountyHunterErrors::BountyAndSubmissionMismatch, //only works for pubkeys
        //alternativa constraint = submission.bounty == bounty.key() @ BountyHunterErrors::BountyAndSubmissionMismatch
    )]
    pub submission: Account<'info, Submission>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bounty,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub hunter: SystemAccount<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = hunter,
        associated_token::token_program = token_program,
    )]
    pub hunter_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl AcceptSolution<'_> {
    pub fn handler(ctx: Context<AcceptSolution>) -> Result<()> {
        //alternativas 'as de cima: 
        //require!(ctx.accounts.bounty.maker == ctx.accounts.maker.key(), BountyHunterErrors::InvalidBountyAuthority);
        //require!(ctx.accounts.submission.bounty == ctx.accounts.bounty.key(), BountyHunterErrors::BountyAndSubmissionMismatch);

        ctx.accounts.bounty.accepted_submission = ctx.accounts.submission.key();
        ctx.accounts.transfer_reward()?;
        Ok(())
    }

    pub fn transfer_reward(&self) -> Result<()> {
        let bounty_seeds = [
            b"bounty",
            self.maker.key.as_ref(),
            &self.bounty.seed.to_le_bytes(),
            &[self.bounty.bump],
        ];
        let signer_seeds = [bounty_seeds.as_ref()];
        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.vault.to_account_info(),
                    mint: self.mint.to_account_info(),
                    to: self.hunter_token_account.to_account_info(),
                    authority: self.bounty.to_account_info(),
                },
                signer_seeds.as_ref(),
            ),
            self.vault.amount,
            self.mint.decimals,
        )?;

        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault.to_account_info(),
                destination: self.maker.to_account_info(),
                authority: self.bounty.to_account_info(),
            },
            signer_seeds.as_ref(),
        ))?;
        Ok(())
    }
}
