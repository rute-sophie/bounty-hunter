pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("ELt3SqpiHUsHJ5fxZpH1ksug6nWjAvYBxxKqK5PHfkBa");

#[program]
pub mod bounty_hunter {
    use super::*;

    pub fn create_bounty(ctx: Context<CreateBounty>) -> Result<()> {
        create_bounty::handler(ctx)
    }
    pub fn cancel_bounty(ctx: Context<CancelBounty>) -> Result<()> {
        cancel_bounty::handler(ctx)
    }
    pub fn submit_solution(ctx: Context<SubmitSolution>) -> Result<()> {
        submit_solution::handler(ctx)
    }
    pub fn accept_solution(ctx: Context<AcceptSolution>) -> Result<()> {
        accept_solution::handler(ctx)
    }

}
