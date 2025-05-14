use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;

pub fn get_ata2022_pubkey(owner_pubkey: &Pubkey, mint_pubkey: &Pubkey) -> Pubkey {
    get_ata_pubkey(owner_pubkey, mint_pubkey, &spl_token_2022::id())
}

pub fn get_ata_pubkey(
    owner_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    token_program_id: &Pubkey,
) -> Pubkey {
    get_associated_token_address_with_program_id(owner_pubkey, mint_pubkey, token_program_id)
}

// pub fn derive_proof_pda(
//     mint: &Pubkey,
//     kind: &str, // e.g. "eq", "validity", "range"
// ) -> (Pubkey, u8) {
//     Pubkey::find_program_address(
//         &[b"ct-proof", mint.as_ref(), kind.as_bytes()],
//         &TOKEN_2022_PROOF_EXTRACTION_PROGRAM_ID,
//     )
// }
