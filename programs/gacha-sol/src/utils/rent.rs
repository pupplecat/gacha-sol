use anchor_lang::prelude::Pubkey;

#[derive(Clone)]
pub struct Rent;

impl anchor_lang::Id for Rent {
    fn id() -> Pubkey {
        Pubkey::from_str_const("SysvarRent111111111111111111111111111111111")
    }
}
