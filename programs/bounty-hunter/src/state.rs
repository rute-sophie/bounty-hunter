use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account(discriminator = 1)]
pub struct Bounty {
    pub seed: u64,
    #[max_len(1024)]
    pub description: String,
    #[max_len(100)]
    pub link: String,
    pub reward: u64,
    pub bump: u8,
    pub maker: Pubkey,
    pub accepted_submission: Pubkey,
}

#[derive(InitSpace)]
#[account(discriminator = 2)]
pub struct Submission {
    pub bounty: Pubkey,
    #[max_len(100)]
    pub link: String,
    pub hunter: Pubkey,
    #[max_len(1024)]
    pub notes: String,
}
