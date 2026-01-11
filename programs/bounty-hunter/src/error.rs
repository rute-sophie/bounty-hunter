use anchor_lang::prelude::*;

#[error_code]
pub enum BountyHunterErrors {
    #[msg("Invalid Bounty authority")]
    InvalidBountyAuthority,
    #[msg("Invalid Submission")]
    BountyAndSubmissionMismatch,
    #[msg("Bounty Already Closed")]
    BountyClosed,
}
