/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as web3 from '@solana/web3.js'
import * as beet from '@metaplex-foundation/beet'
import * as beetSolana from '@metaplex-foundation/beet-solana'
import * as customSerializer from '../../custom/stake-pool-deposit-stake-authority-serializer'

/**
 * Arguments used to create {@link StakePoolDepositStakeAuthority}
 * @category Accounts
 * @category generated
 */
export type StakePoolDepositStakeAuthorityArgs = {
  base: web3.PublicKey
  stakePool: web3.PublicKey
  poolMint: web3.PublicKey
  authority: web3.PublicKey
  vault: web3.PublicKey
  stakePoolProgramId: web3.PublicKey
  coolDownSeconds: beet.bignum
  initalFeeBps: number
  feeWallet: web3.PublicKey
  bumpSeed: number
  reserved: number[] /* size: 256 */
}
/**
 * Holds the data for the {@link StakePoolDepositStakeAuthority} Account and provides de/serialization
 * functionality for that data
 *
 * @category Accounts
 * @category generated
 */
export class StakePoolDepositStakeAuthority
  implements StakePoolDepositStakeAuthorityArgs
{
  private constructor(
    readonly base: web3.PublicKey,
    readonly stakePool: web3.PublicKey,
    readonly poolMint: web3.PublicKey,
    readonly authority: web3.PublicKey,
    readonly vault: web3.PublicKey,
    readonly stakePoolProgramId: web3.PublicKey,
    readonly coolDownSeconds: beet.bignum,
    readonly initalFeeBps: number,
    readonly feeWallet: web3.PublicKey,
    readonly bumpSeed: number,
    readonly reserved: number[] /* size: 256 */
  ) {}

  /**
   * Creates a {@link StakePoolDepositStakeAuthority} instance from the provided args.
   */
  static fromArgs(args: StakePoolDepositStakeAuthorityArgs) {
    return new StakePoolDepositStakeAuthority(
      args.base,
      args.stakePool,
      args.poolMint,
      args.authority,
      args.vault,
      args.stakePoolProgramId,
      args.coolDownSeconds,
      args.initalFeeBps,
      args.feeWallet,
      args.bumpSeed,
      args.reserved
    )
  }

  /**
   * Deserializes the {@link StakePoolDepositStakeAuthority} from the data of the provided {@link web3.AccountInfo}.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static fromAccountInfo(
    accountInfo: web3.AccountInfo<Buffer>,
    offset = 0
  ): [StakePoolDepositStakeAuthority, number] {
    return StakePoolDepositStakeAuthority.deserialize(accountInfo.data, offset)
  }

  /**
   * Retrieves the account info from the provided address and deserializes
   * the {@link StakePoolDepositStakeAuthority} from its data.
   *
   * @throws Error if no account info is found at the address or if deserialization fails
   */
  static async fromAccountAddress(
    connection: web3.Connection,
    address: web3.PublicKey,
    commitmentOrConfig?: web3.Commitment | web3.GetAccountInfoConfig
  ): Promise<StakePoolDepositStakeAuthority> {
    const accountInfo = await connection.getAccountInfo(
      address,
      commitmentOrConfig
    )
    if (accountInfo == null) {
      throw new Error(
        `Unable to find StakePoolDepositStakeAuthority account at ${address}`
      )
    }
    return StakePoolDepositStakeAuthority.fromAccountInfo(accountInfo, 0)[0]
  }

  /**
   * Provides a {@link web3.Connection.getProgramAccounts} config builder,
   * to fetch accounts matching filters that can be specified via that builder.
   *
   * @param programId - the program that owns the accounts we are filtering
   */
  static gpaBuilder(
    programId: web3.PublicKey = new web3.PublicKey(
      '5TAiuAh3YGDbwjEruC1ZpXTJWdNDS7Ur7VeqNNiHMmGV'
    )
  ) {
    return beetSolana.GpaBuilder.fromStruct(
      programId,
      stakePoolDepositStakeAuthorityBeet
    )
  }

  /**
   * Deserializes the {@link StakePoolDepositStakeAuthority} from the provided data Buffer.
   * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
   */
  static deserialize(
    buf: Buffer,
    offset = 0
  ): [StakePoolDepositStakeAuthority, number] {
    return resolvedDeserialize(buf, offset)
  }

  /**
   * Serializes the {@link StakePoolDepositStakeAuthority} into a Buffer.
   * @returns a tuple of the created Buffer and the offset up to which the buffer was written to store it.
   */
  serialize(): [Buffer, number] {
    return resolvedSerialize(this)
  }

  /**
   * Returns the byteSize of a {@link Buffer} holding the serialized data of
   * {@link StakePoolDepositStakeAuthority}
   */
  static get byteSize() {
    return stakePoolDepositStakeAuthorityBeet.byteSize
  }

  /**
   * Fetches the minimum balance needed to exempt an account holding
   * {@link StakePoolDepositStakeAuthority} data from rent
   *
   * @param connection used to retrieve the rent exemption information
   */
  static async getMinimumBalanceForRentExemption(
    connection: web3.Connection,
    commitment?: web3.Commitment
  ): Promise<number> {
    return connection.getMinimumBalanceForRentExemption(
      StakePoolDepositStakeAuthority.byteSize,
      commitment
    )
  }

  /**
   * Determines if the provided {@link Buffer} has the correct byte size to
   * hold {@link StakePoolDepositStakeAuthority} data.
   */
  static hasCorrectByteSize(buf: Buffer, offset = 0) {
    return buf.byteLength - offset === StakePoolDepositStakeAuthority.byteSize
  }

  /**
   * Returns a readable version of {@link StakePoolDepositStakeAuthority} properties
   * and can be used to convert to JSON and/or logging
   */
  pretty() {
    return {
      base: this.base.toBase58(),
      stakePool: this.stakePool.toBase58(),
      poolMint: this.poolMint.toBase58(),
      authority: this.authority.toBase58(),
      vault: this.vault.toBase58(),
      stakePoolProgramId: this.stakePoolProgramId.toBase58(),
      coolDownSeconds: this.coolDownSeconds,
      initalFeeBps: this.initalFeeBps,
      feeWallet: this.feeWallet.toBase58(),
      bumpSeed: this.bumpSeed,
      reserved: this.reserved,
    }
  }
}

/**
 * @category Accounts
 * @category generated
 */
export const stakePoolDepositStakeAuthorityBeet = new beet.BeetStruct<
  StakePoolDepositStakeAuthority,
  StakePoolDepositStakeAuthorityArgs
>(
  [
    ['base', beetSolana.publicKey],
    ['stakePool', beetSolana.publicKey],
    ['poolMint', beetSolana.publicKey],
    ['authority', beetSolana.publicKey],
    ['vault', beetSolana.publicKey],
    ['stakePoolProgramId', beetSolana.publicKey],
    ['coolDownSeconds', beet.u64],
    ['initalFeeBps', beet.u32],
    ['feeWallet', beetSolana.publicKey],
    ['bumpSeed', beet.u8],
    ['reserved', beet.uniformFixedSizeArray(beet.u8, 256)],
  ],
  StakePoolDepositStakeAuthority.fromArgs,
  'StakePoolDepositStakeAuthority'
)

const serializer = customSerializer as unknown as {
  serialize: typeof stakePoolDepositStakeAuthorityBeet.serialize
  deserialize: typeof stakePoolDepositStakeAuthorityBeet.deserialize
}

const resolvedSerialize =
  typeof serializer.serialize === 'function'
    ? serializer.serialize.bind(serializer)
    : stakePoolDepositStakeAuthorityBeet.serialize.bind(
        stakePoolDepositStakeAuthorityBeet
      )
const resolvedDeserialize =
  typeof serializer.deserialize === 'function'
    ? serializer.deserialize.bind(serializer)
    : stakePoolDepositStakeAuthorityBeet.deserialize.bind(
        stakePoolDepositStakeAuthorityBeet
      )
