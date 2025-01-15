import { Connection, PublicKey, Signer, TransactionInstruction, AccountMeta } from "@solana/web3.js";
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
 * @param remainingAccounts - optional additional accounts to append to the instruction
 */
export declare const depositStake: (connection: Connection, payer: PublicKey, stakePoolAddress: PublicKey, authorizedPubkey: PublicKey, validatorVote: PublicKey, depositStake: PublicKey, poolTokenReceiverAccount?: PublicKey, remainingAccounts?: AccountMeta[]) => Promise<{
    instructions: TransactionInstruction[];
    signers: Signer[];
}>;
