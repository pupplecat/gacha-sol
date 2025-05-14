use anchor_lang::prelude::*;

#[error_code]
#[derive(PartialEq, Eq)]
pub enum GachaError {
    #[msg("Invalid reward mint")]
    InvalidRewardMint,

    #[msg("Pull price is zero")]
    InvalidZeroPullPrice,

    #[msg("Invalid pull id")]
    InvalidPullId,

    #[msg("Failed to configure confidential transfer account")]
    ConfigureTokenAccountFailed,

    #[msg("Failed to close context state account")]
    CloseContextStateFailed,

    #[msg("ProofDataConversionError")]
    ProofDataConversionError,

    #[msg("Invalid account provided")]
    InvalidAccount,

    #[msg("Pull not verified")]
    PullNotVerified,

    #[msg("Pull already purchased")]
    PullAlreadyPurchased,

    #[msg("Pull already claimed")]
    PullAlreadyClaimed,

    #[msg("Invalid buyer")]
    InvalidBuyer,

    #[msg("Invalid proof type")]
    InvalidProofType,

    #[msg("Invalid elgamal pubkey")]
    InvalidElgamalPubkey,

    #[msg("Invalid context authority")]
    InvalidContextAuthority,

    #[msg("Ciphertext arithmetic failed")]
    CiphertextArithmeticFailed,

    #[msg("Ciphertext zero balance mismatch")]
    CiphertextZeroBalanceMismatch,

    #[msg("Decryptable balance conversion failed")]
    DecryptableBalanceConversionFailed,

    #[msg("Ciphertext balance conversion failed")]
    CipherTextBalanceConversionFailed,
}
