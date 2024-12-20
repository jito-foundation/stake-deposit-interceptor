import { Connection, PublicKey, Signer, TransactionInstruction } from "@solana/web3.js";
/**
 * Creates instructions required to deposit stake to stake pool via
 * Stake Deposit Interceptor.
 *
 * @param connection
 * @param payer - [NEW] pays rent for DepositReceipt
 * @param stakePoolAddress
 * @param authorizedPubkey
 * @param validatorVote
 * @param depositStake
 * @param poolTokenReceiverAccount
 */
export declare const depositStake: (connection: Connection, payer: PublicKey, stakePoolAddress: PublicKey, authorizedPubkey: PublicKey, validatorVote: PublicKey, depositStake: PublicKey, poolTokenReceiverAccount?: PublicKey) => Promise<{
    instructions: TransactionInstruction[];
    signers: Signer[];
}>;
