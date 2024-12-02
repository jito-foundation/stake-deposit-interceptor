/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet'
import * as web3 from '@solana/web3.js'
import {
  UpdateStakePoolDepositStakeAuthorityArgs,
  updateStakePoolDepositStakeAuthorityArgsBeet,
} from '../types/UpdateStakePoolDepositStakeAuthorityArgs'

/**
 * @category Instructions
 * @category UpdateStakePoolDepositStakeAuthority
 * @category generated
 */
export type UpdateStakePoolDepositStakeAuthorityInstructionArgs = {
  updateStakePoolDepositStakeAuthorityArgs: UpdateStakePoolDepositStakeAuthorityArgs
}
/**
 * @category Instructions
 * @category UpdateStakePoolDepositStakeAuthority
 * @category generated
 */
export const UpdateStakePoolDepositStakeAuthorityStruct =
  new beet.FixableBeetArgsStruct<
    UpdateStakePoolDepositStakeAuthorityInstructionArgs & {
      instructionDiscriminator: number
    }
  >(
    [
      ['instructionDiscriminator', beet.u8],
      [
        'updateStakePoolDepositStakeAuthorityArgs',
        updateStakePoolDepositStakeAuthorityArgsBeet,
      ],
    ],
    'UpdateStakePoolDepositStakeAuthorityInstructionArgs'
  )
/**
 * Accounts required by the _UpdateStakePoolDepositStakeAuthority_ instruction
 *
 * @property [_writable_] depositStakeAuthority
 * @property [**signer**] authority
 * @property [**signer**] newAuthority (optional)
 * @category Instructions
 * @category UpdateStakePoolDepositStakeAuthority
 * @category generated
 */
export type UpdateStakePoolDepositStakeAuthorityInstructionAccounts = {
  depositStakeAuthority: web3.PublicKey
  authority: web3.PublicKey
  newAuthority?: web3.PublicKey
}

export const updateStakePoolDepositStakeAuthorityInstructionDiscriminator = 1

/**
 * Creates a _UpdateStakePoolDepositStakeAuthority_ instruction.
 *
 * Optional accounts that are not provided default to the program ID since
 * this was indicated in the IDL from which this instruction was generated.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category UpdateStakePoolDepositStakeAuthority
 * @category generated
 */
export function createUpdateStakePoolDepositStakeAuthorityInstruction(
  accounts: UpdateStakePoolDepositStakeAuthorityInstructionAccounts,
  args: UpdateStakePoolDepositStakeAuthorityInstructionArgs,
  programId = new web3.PublicKey('5TAiuAh3YGDbwjEruC1ZpXTJWdNDS7Ur7VeqNNiHMmGV')
) {
  const [data] = UpdateStakePoolDepositStakeAuthorityStruct.serialize({
    instructionDiscriminator:
      updateStakePoolDepositStakeAuthorityInstructionDiscriminator,
    ...args,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.depositStakeAuthority,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: accounts.authority,
      isWritable: false,
      isSigner: true,
    },
    {
      pubkey: accounts.newAuthority ?? programId,
      isWritable: false,
      isSigner: accounts.newAuthority != null,
    },
  ]

  const ix = new web3.TransactionInstruction({
    programId,
    keys,
    data,
  })
  return ix
}
