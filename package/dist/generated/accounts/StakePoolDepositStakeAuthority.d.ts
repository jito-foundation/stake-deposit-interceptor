/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */
import * as web3 from '@solana/web3.js';
import * as beet from '@metaplex-foundation/beet';
import * as beetSolana from '@metaplex-foundation/beet-solana';
/**
 * Arguments used to create {@link StakePoolDepositStakeAuthority}
 * @category Accounts
 * @category generated
 */
export type StakePoolDepositStakeAuthorityArgs = {
    base: web3.PublicKey;
    stakePool: web3.PublicKey;
    poolMint: web3.PublicKey;
    authority: web3.PublicKey;
    vault: web3.PublicKey;
    stakePoolProgramId: web3.PublicKey;
    coolDownSeconds: beet.bignum;
    initalFeeBps: number;
    feeWallet: web3.PublicKey;
    bumpSeed: number;
    reserved: number[];
};
/**
 * Holds the data for the {@link StakePoolDepositStakeAuthority} Account and provides de/serialization
 * functionality for that data
 *
 * @category Accounts
 * @category generated
 */
export declare class StakePoolDepositStakeAuthority implements StakePoolDepositStakeAuthorityArgs {
    readonly base: web3.PublicKey;
    readonly stakePool: web3.PublicKey;
    readonly poolMint: web3.PublicKey;
    readonly authority: web3.PublicKey;
    readonly vault: web3.PublicKey;
    readonly stakePoolProgramId: web3.PublicKey;
    readonly coolDownSeconds: beet.bignum;
    readonly initalFeeBps: number;
    readonly feeWallet: web3.PublicKey;
    readonly bumpSeed: number;
    readonly reserved: number[];
    private constructor();
    /**
     * Creates a {@link StakePoolDepositStakeAuthority} instance from the provided args.
     */
    static fromArgs(args: StakePoolDepositStakeAuthorityArgs): StakePoolDepositStakeAuthority;
    /**
     * Deserializes the {@link StakePoolDepositStakeAuthority} from the data of the provided {@link web3.AccountInfo}.
     * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
     */
    static fromAccountInfo(accountInfo: web3.AccountInfo<Buffer>, offset?: number): [StakePoolDepositStakeAuthority, number];
    /**
     * Retrieves the account info from the provided address and deserializes
     * the {@link StakePoolDepositStakeAuthority} from its data.
     *
     * @throws Error if no account info is found at the address or if deserialization fails
     */
    static fromAccountAddress(connection: web3.Connection, address: web3.PublicKey, commitmentOrConfig?: web3.Commitment | web3.GetAccountInfoConfig): Promise<StakePoolDepositStakeAuthority>;
    /**
     * Provides a {@link web3.Connection.getProgramAccounts} config builder,
     * to fetch accounts matching filters that can be specified via that builder.
     *
     * @param programId - the program that owns the accounts we are filtering
     */
    static gpaBuilder(programId?: web3.PublicKey): beetSolana.GpaBuilder<{
        base: any;
        stakePool: any;
        coolDownSeconds: any;
        bumpSeed: any;
        reserved: any;
        poolMint: any;
        authority: any;
        vault: any;
        stakePoolProgramId: any;
        initalFeeBps: any;
        feeWallet: any;
    }>;
    /**
     * Deserializes the {@link StakePoolDepositStakeAuthority} from the provided data Buffer.
     * @returns a tuple of the account data and the offset up to which the buffer was read to obtain it.
     */
    static deserialize(buf: Buffer, offset?: number): [StakePoolDepositStakeAuthority, number];
    /**
     * Serializes the {@link StakePoolDepositStakeAuthority} into a Buffer.
     * @returns a tuple of the created Buffer and the offset up to which the buffer was written to store it.
     */
    serialize(): [Buffer, number];
    /**
     * Returns the byteSize of a {@link Buffer} holding the serialized data of
     * {@link StakePoolDepositStakeAuthority}
     */
    static get byteSize(): number;
    /**
     * Fetches the minimum balance needed to exempt an account holding
     * {@link StakePoolDepositStakeAuthority} data from rent
     *
     * @param connection used to retrieve the rent exemption information
     */
    static getMinimumBalanceForRentExemption(connection: web3.Connection, commitment?: web3.Commitment): Promise<number>;
    /**
     * Determines if the provided {@link Buffer} has the correct byte size to
     * hold {@link StakePoolDepositStakeAuthority} data.
     */
    static hasCorrectByteSize(buf: Buffer, offset?: number): boolean;
    /**
     * Returns a readable version of {@link StakePoolDepositStakeAuthority} properties
     * and can be used to convert to JSON and/or logging
     */
    pretty(): {
        base: string;
        stakePool: string;
        poolMint: string;
        authority: string;
        vault: string;
        stakePoolProgramId: string;
        coolDownSeconds: beet.bignum;
        initalFeeBps: number;
        feeWallet: string;
        bumpSeed: number;
        reserved: number[];
    };
}
/**
 * @category Accounts
 * @category generated
 */
export declare const stakePoolDepositStakeAuthorityBeet: beet.BeetStruct<StakePoolDepositStakeAuthority, StakePoolDepositStakeAuthorityArgs>;
