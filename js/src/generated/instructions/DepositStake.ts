/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet'
import * as web3 from '@solana/web3.js'
import {
  DepositStakeArgs,
  depositStakeArgsBeet,
} from '../types/DepositStakeArgs'

/**
 * @category Instructions
 * @category DepositStake
 * @category generated
 */
export type DepositStakeInstructionArgs = {
  depositStakeArgs: DepositStakeArgs
}
/**
 * @category Instructions
 * @category DepositStake
 * @category generated
 */
export const DepositStakeStruct = new beet.BeetArgsStruct<
  DepositStakeInstructionArgs & {
    instructionDiscriminator: number
  }
>(
  [
    ['instructionDiscriminator', beet.u8],
    ['depositStakeArgs', depositStakeArgsBeet],
  ],
  'DepositStakeInstructionArgs'
)
/**
 * Accounts required by the _DepositStake_ instruction
 *
 * @property [_writable_, **signer**] payer
 * @property [] stakePoolProgram
 * @property [_writable_] depositReceipt
 * @category Instructions
 * @category DepositStake
 * @category generated
 */
export type DepositStakeInstructionAccounts = {
  payer: web3.PublicKey
  stakePoolProgram: web3.PublicKey
  depositReceipt: web3.PublicKey
}

export const depositStakeInstructionDiscriminator = 2

/**
 * Creates a _DepositStake_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category DepositStake
 * @category generated
 */
export function createDepositStakeInstruction(
  accounts: DepositStakeInstructionAccounts,
  args: DepositStakeInstructionArgs,
  programId = new web3.PublicKey('5TAiuAh3YGDbwjEruC1ZpXTJWdNDS7Ur7VeqNNiHMmGV')
) {
  const [data] = DepositStakeStruct.serialize({
    instructionDiscriminator: depositStakeInstructionDiscriminator,
    ...args,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.payer,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: accounts.stakePoolProgram,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: accounts.depositReceipt,
      isWritable: true,
      isSigner: false,
    },
  ]

  const ix = new web3.TransactionInstruction({
    programId,
    keys,
    data,
  })
  return ix
}