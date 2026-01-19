use anchor_lang::prelude::*;

use crate::{error::BountyHunterErrors, state::Bounty};
use anchor_spl::token::{close_account, transfer_checked, CloseAccount, TransferChecked};
use anchor_spl::token_interface::Mint;
use anchor_spl::token_interface::TokenAccount;
use anchor_spl::{associated_token::AssociatedToken, token_interface::TokenInterface};

#[derive(Accounts)]
pub struct CancelBounty<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        close = maker,
        has_one = maker @ BountyHunterErrors::InvalidBountyAuthority,
        has_one = mint @ BountyHunterErrors::InvalidMint,
    )]
    pub bounty: Account<'info, Bounty>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bounty,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl CancelBounty<'_> {
    pub fn handler(ctx: Context<CancelBounty>) -> Result<()> {
        ctx.accounts.refund_tokens()?;
        Ok(())
    }

    fn refund_tokens(&self) -> Result<()> {
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
                    to: self.maker_token_account.to_account_info(),
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
