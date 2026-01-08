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

    pub fn create_bounty(ctx: Context<CreateBounty>, seed: u64, description: String, link: String, reward: u64) -> Result<()> {
        create_bounty::handler(ctx, seed, description, link, reward)
    }
    pub fn cancel_bounty(ctx: Context<CancelBounty>) -> Result<()> {
        cancel_bounty::handler(ctx)
    }
    pub fn submit_solution(ctx: Context<SubmitSolution>, link: String, notes: String) -> Result<()> {
        submit_solution::handler(ctx, link, notes)
    }
    pub fn accept_solution(ctx: Context<AcceptSolution>) -> Result<()> {
        accept_solution::handler(ctx)
    }

}