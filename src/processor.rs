use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction::StakeDepositInterceptorInstruction;

pub struct Processor;

impl Processor {
    pub fn process(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = StakeDepositInterceptorInstruction::try_from_slice(input)?;
        Ok(())
    }
}
