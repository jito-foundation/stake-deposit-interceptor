use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// Initialize arguments for FeeParameters
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct InitFeeParametersArgs {
    pub cool_down_period: i64,
    pub initial_fee_rate: u32,
    pub bump_seed: u8,
}

/// Instructions supported by the StakeDepositInterceptor program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum StakeDepositInterceptorInstruction {
    ///   Initializes the FeeParameters for the program.
    ///
    ///   0. `[w]` New FeeParameters to create.
    ///   1. `[s]` Authority
    InitFeeParameters(InitFeeParametersArgs),
    DepositStake,
}

pub fn derive_fee_parameters(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"fee_parameters"], program_id)
}

/// Creates instruction to set up the FeeParameter to be used in the
pub fn create_init_fee_parameters_instruction(
    program_id: &Pubkey,
    cool_down_period: i64,
    initial_fee_rate: u32,
    authority: &Pubkey,
) -> Instruction {
    let (fee_parameter_pubkey, bump_seed) = derive_fee_parameters(program_id);
    let init_fee_parameters_args = InitFeeParametersArgs {
        initial_fee_rate,
        cool_down_period,
        bump_seed,
    };
    let accounts = vec![
        AccountMeta::new(fee_parameter_pubkey, false),
        AccountMeta::new_readonly(*authority, true),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakeDepositInterceptorInstruction::InitFeeParameters(
            init_fee_parameters_args,
        ))
        .unwrap(),
    }
}
