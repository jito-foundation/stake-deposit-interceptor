import {
  Connection,
  Keypair,
  PublicKey,
  Signer,
  StakeAuthorizationLayout,
  StakeProgram,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  TransactionInstruction,
  AccountMeta,
} from "@solana/web3.js";
import {
  createDepositStakeInstruction,
  DepositStakeInstructionAccounts,
  DepositStakeInstructionArgs,
  PROGRAM_ID,
  StakePoolDepositStakeAuthority,
} from "./generated";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import BN from "bn.js";
import { struct, u8, publicKey, u64 } from '@coral-xyz/borsh';

/**
 * Copied from @solana/spl-stake-pool for compatibility reasons.
 * Source: https://github.com/solana-labs/solana-program-library/blob/b7dd8fee/stake-pool/js/src/index.ts
 */
const STAKE_POOL_PROGRAM_ID = new PublicKey("SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy");

/**
 * Copied from @solana/spl-stake-pool for compatibility.
 * We only need the account data structure, not the full package.
 */
type StakePoolLayout = {
  accountType: number;
  manager: PublicKey;
  staker: PublicKey;
  stakeDepositAuthority: PublicKey;
  stakeWithdrawBumpSeed: number;
  validatorList: PublicKey;
  reserveStake: PublicKey;
  poolMint: PublicKey;
  managerFeeAccount: PublicKey;
  tokenProgramId: PublicKey;
  totalLamports: BN;
  poolTokenSupply: BN;
  lastUpdateEpoch: BN;
};

const StakePoolLayout = struct<StakePoolLayout>([
  u8('accountType'),
  publicKey('manager'),
  publicKey('staker'),
  publicKey('stakeDepositAuthority'),
  u8('stakeWithdrawBumpSeed'),
  publicKey('validatorList'),
  publicKey('reserveStake'),
  publicKey('poolMint'),
  publicKey('managerFeeAccount'),
  publicKey('tokenProgramId'),
  u64('totalLamports'),
  u64('poolTokenSupply'),
  u64('lastUpdateEpoch'),
]);

const getStakePoolAccount = async (
  connection: Connection,
  stakePoolAddress: PublicKey
) => {
  const account = await connection.getAccountInfo(stakePoolAddress);
  if (!account) throw new Error("Stake pool account not found");
  
  const data = StakePoolLayout.decode(account.data) as StakePoolLayout;
  
  return {
    pubkey: stakePoolAddress,
    account: {
      data
    }
  };
};

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
export const depositStake = async (
  connection: Connection,
  payer: PublicKey,
  stakePoolAddress: PublicKey,
  authorizedPubkey: PublicKey,
  validatorVote: PublicKey,
  depositStake: PublicKey,
  poolTokenReceiverAccount?: PublicKey,
  remainingAccounts?: AccountMeta[]
) => {
  const stakePool = await getStakePoolAccount(connection, stakePoolAddress);
  const stakePoolDepositAuthority =
    await StakePoolDepositStakeAuthority.fromAccountAddress(
      connection,
      stakePool.account.data.stakeDepositAuthority
    );
  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress
  );
  const validatorStake = await findStakeProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    validatorVote,
    stakePoolAddress
  );

  const instructions: TransactionInstruction[] = [];
  const signers: Signer[] = [];

  const base = Keypair.generate();
  const poolMint = stakePool.account.data.poolMint;

  signers.push(base);

  // Create token account if not specified
  if (!poolTokenReceiverAccount) {
    const associatedAddress = getAssociatedTokenAddressSync(
      poolMint,
      authorizedPubkey
    );
    instructions.push(
      createAssociatedTokenAccountIdempotentInstruction(
        authorizedPubkey,
        associatedAddress,
        authorizedPubkey,
        poolMint
      )
    );
    poolTokenReceiverAccount = associatedAddress;
  }

  instructions.push(
    ...StakeProgram.authorize({
      stakePubkey: depositStake,
      authorizedPubkey,
      newAuthorizedPubkey: stakePool.account.data.stakeDepositAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Staker,
    }).instructions
  );
  instructions.push(
    ...StakeProgram.authorize({
      stakePubkey: depositStake,
      authorizedPubkey,
      newAuthorizedPubkey: stakePool.account.data.stakeDepositAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Withdrawer,
    }).instructions
  );

  // Derive DepositReceipt Address
  const [depositReceiptAddress] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("deposit_receipt"),
      stakePoolAddress.toBuffer(),
      base.publicKey.toBuffer(),
    ],
    PROGRAM_ID
  );

  const depositStakeIxArgs: DepositStakeInstructionArgs = {
    depositStakeArgs: {
      owner: authorizedPubkey,
    },
  };
  const depositStakeIxAccounts: DepositStakeInstructionAccounts = {
    payer,
    stakePoolProgram: STAKE_POOL_PROGRAM_ID,
    depositReceipt: depositReceiptAddress,
    stakePool: stakePoolAddress,
    validatorStakeList: stakePool.account.data.validatorList,
    depositStakeAuthority: stakePool.account.data.stakeDepositAuthority,
    base: base.publicKey,
    stakePoolWithdrawAuthority: withdrawAuthority,
    stake: depositStake,
    validatorStakeAccount: validatorStake,
    reserveStakeAccount: stakePool.account.data.reserveStake,
    vault: stakePoolDepositAuthority.vault,
    managerFeeAccount: stakePool.account.data.managerFeeAccount,
    referrerPoolTokensAccount: poolTokenReceiverAccount,
    poolMint,
    clock: SYSVAR_CLOCK_PUBKEY,
    stakeHistory: SYSVAR_STAKE_HISTORY_PUBKEY,
    tokenProgram: TOKEN_PROGRAM_ID,
    stakeProgram: StakeProgram.programId,
    systemProgram: SystemProgram.programId,
  };
  const depositStakeIx = createDepositStakeInstruction(
    depositStakeIxAccounts,
    depositStakeIxArgs
  );

  // Add any remaining accounts to the instruction
  if (remainingAccounts?.length) {
    depositStakeIx.keys.push(...remainingAccounts);
  }

  instructions.push(depositStakeIx);

  return {
    instructions,
    signers,
  };
};

/**
 * Generates the withdraw authority program address for the stake pool
 */
const findWithdrawAuthorityProgramAddress = (
  programId: PublicKey,
  stakePoolAddress: PublicKey
) => {
  const [publicKey] = PublicKey.findProgramAddressSync(
    [stakePoolAddress.toBuffer(), Buffer.from("withdraw")],
    programId
  );
  return publicKey;
};

/**
 * Generates the stake program address for a validator's vote account
 */
const findStakeProgramAddress = (
  programId: PublicKey,
  voteAccountAddress: PublicKey,
  stakePoolAddress: PublicKey
) => {
  const [publicKey] = PublicKey.findProgramAddressSync(
    [
      voteAccountAddress.toBuffer(),
      stakePoolAddress.toBuffer(),
      Buffer.alloc(0),
    ],
    programId
  );
  return publicKey;
};
