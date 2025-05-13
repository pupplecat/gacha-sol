use anchor_lang::prelude::Pubkey;
use spl_token_2022::ID;

#[derive(Clone)]
pub struct Token2022;

impl anchor_lang::Id for Token2022 {
    fn id() -> Pubkey {
        ID
    }
}
