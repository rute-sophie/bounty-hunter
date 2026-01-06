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

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}
