use anyhow::Result;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, rent::Rent, system_instruction};
use spl_token_2022::solana_zk_sdk::zk_elgamal_proof_program::{
    self,
    instruction::{close_context_state, ContextStateInfo},
};

pub fn get_zk_proof_context_state_account_creation_instructions<
    ZK: bytemuck::Pod + zk_elgamal_proof_program::proof_data::ZkProofData<U>,
    U: bytemuck::Pod,
>(
    fee_payer_pubkey: &Pubkey,
    context_state_account_pubkey: &Pubkey,
    context_state_authority_pubkey: &Pubkey,
    proof_data: &ZK,
) -> Result<(
    solana_sdk::instruction::Instruction,
    solana_sdk::instruction::Instruction,
)> {
    use spl_token_confidential_transfer_proof_extraction::instruction::zk_proof_type_to_instruction;
    use std::mem::size_of;

    let space = size_of::<zk_elgamal_proof_program::state::ProofContextState<U>>();
    let rent = Rent::default().minimum_balance(space);

    let context_state_info = ContextStateInfo {
        context_state_account: context_state_account_pubkey,
        context_state_authority: context_state_authority_pubkey,
    };

    let instruction_type = zk_proof_type_to_instruction(ZK::PROOF_TYPE)?;

    let create_account_ix = system_instruction::create_account(
        fee_payer_pubkey,
        context_state_account_pubkey,
        rent,
        space as u64,
        &zk_elgamal_proof_program::id(),
    );

    let verify_proof_ix =
        instruction_type.encode_verify_proof(Some(context_state_info), proof_data);

    // Return a tuple containing the create account instruction and verify proof instruction.
    Ok((create_account_ix, verify_proof_ix))
}

pub fn create_close_context_state_ixs(
    proof_account_pubkeys: &[Pubkey],
    authority: &Pubkey,
    destination_account: &Pubkey,
) -> Vec<Instruction> {
    proof_account_pubkeys
        .iter()
        .map(|k| {
            let context_state_info = ContextStateInfo {
                context_state_account: k,
                context_state_authority: authority,
            };
            close_context_state(context_state_info, destination_account)
        })
        .collect()
}
