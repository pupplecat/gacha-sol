use anchor_lang::prelude::Pubkey;
use spl_token_2022::solana_zk_sdk::zk_elgamal_proof_program::ID;

#[derive(Clone, PartialEq, Eq)]
pub struct ZkElgamalProof;

impl anchor_lang::Id for ZkElgamalProof {
    fn id() -> Pubkey {
        ID
    }
}
