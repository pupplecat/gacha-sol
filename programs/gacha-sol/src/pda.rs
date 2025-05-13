use anchor_lang::prelude::*;

use crate::ID;

fn find_program_address(seed: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seed, &ID).0
}

pub fn get_game_config_seed<'a>() -> [&'a [u8]; 1] {
    [b"game_config"]
}

pub fn get_game_config_pubkey() -> Pubkey {
    find_program_address(&get_game_config_seed())
}

pub fn get_pull_pubkey(pull_id: u64) -> Pubkey {
    let pull_id_seed = [pull_id];
    let pull_id_bytes = bytemuck::bytes_of(&pull_id_seed);

    find_program_address(&[b"pull", pull_id_bytes])
}

pub fn get_reward_vault_pubkey(pull: Pubkey) -> Pubkey {
    find_program_address(&[b"reward_vault", pull.as_ref()])
}
