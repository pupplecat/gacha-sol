use anchor_lang::prelude::*;

#[error_code]
#[derive(PartialEq, Eq)]
pub enum GachaError {
    #[msg("Invalid pull id")]
    InvalidPullId,

    #[msg("Pull already purchased")]
    PullAlreadyPurchased,

    #[msg("Pull already claimed")]
    PullAlreadyClaimed,

    #[msg("Invalid Buyer")]
    InvalidBuyer,
}
