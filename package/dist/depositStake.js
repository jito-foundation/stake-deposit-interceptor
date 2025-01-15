"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.depositStake = void 0;
const web3_js_1 = require("@solana/web3.js");
const generated_1 = require("./generated");
const spl_token_1 = require("@solana/spl-token");
const borsh_1 = require("@coral-xyz/borsh");
/**
 * Copied from @solana/spl-stake-pool for compatibility reasons.
 * Source: https://github.com/solana-labs/solana-program-library/blob/b7dd8fee/stake-pool/js/src/index.ts
 */
const STAKE_POOL_PROGRAM_ID = new web3_js_1.PublicKey("SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy");
const StakePoolLayout = (0, borsh_1.struct)([
    (0, borsh_1.u8)('accountType'),
    (0, borsh_1.publicKey)('manager'),
    (0, borsh_1.publicKey)('staker'),
    (0, borsh_1.publicKey)('stakeDepositAuthority'),
    (0, borsh_1.u8)('stakeWithdrawBumpSeed'),
    (0, borsh_1.publicKey)('validatorList'),
    (0, borsh_1.publicKey)('reserveStake'),
    (0, borsh_1.publicKey)('poolMint'),
    (0, borsh_1.publicKey)('managerFeeAccount'),
    (0, borsh_1.publicKey)('tokenProgramId'),
    (0, borsh_1.u64)('totalLamports'),
    (0, borsh_1.u64)('poolTokenSupply'),
    (0, borsh_1.u64)('lastUpdateEpoch'),
]);
const getStakePoolAccount = async (connection, stakePoolAddress) => {
    const account = await connection.getAccountInfo(stakePoolAddress);
    if (!account)
        throw new Error("Stake pool account not found");
    const data = StakePoolLayout.decode(account.data);
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
const depositStake = async (connection, payer, stakePoolAddress, authorizedPubkey, validatorVote, depositStake, poolTokenReceiverAccount, remainingAccounts) => {
    const stakePool = await getStakePoolAccount(connection, stakePoolAddress);
    const stakePoolDepositAuthority = await generated_1.StakePoolDepositStakeAuthority.fromAccountAddress(connection, stakePool.account.data.stakeDepositAuthority);
    const withdrawAuthority = await findWithdrawAuthorityProgramAddress(STAKE_POOL_PROGRAM_ID, stakePoolAddress);
    const validatorStake = await findStakeProgramAddress(STAKE_POOL_PROGRAM_ID, validatorVote, stakePoolAddress);
    const instructions = [];
    const signers = [];
    const base = web3_js_1.Keypair.generate();
    const poolMint = stakePool.account.data.poolMint;
    signers.push(base);
    // Create token account if not specified
    if (!poolTokenReceiverAccount) {
        const associatedAddress = (0, spl_token_1.getAssociatedTokenAddressSync)(poolMint, authorizedPubkey);
        instructions.push((0, spl_token_1.createAssociatedTokenAccountIdempotentInstruction)(authorizedPubkey, associatedAddress, authorizedPubkey, poolMint));
        poolTokenReceiverAccount = associatedAddress;
    }
    instructions.push(...web3_js_1.StakeProgram.authorize({
        stakePubkey: depositStake,
        authorizedPubkey,
        newAuthorizedPubkey: stakePool.account.data.stakeDepositAuthority,
        stakeAuthorizationType: web3_js_1.StakeAuthorizationLayout.Staker,
    }).instructions);
    instructions.push(...web3_js_1.StakeProgram.authorize({
        stakePubkey: depositStake,
        authorizedPubkey,
        newAuthorizedPubkey: stakePool.account.data.stakeDepositAuthority,
        stakeAuthorizationType: web3_js_1.StakeAuthorizationLayout.Withdrawer,
    }).instructions);
    // Derive DepositReceipt Address
    const [depositReceiptAddress] = web3_js_1.PublicKey.findProgramAddressSync([
        Buffer.from("deposit_receipt"),
        stakePoolAddress.toBuffer(),
        base.publicKey.toBuffer(),
    ], generated_1.PROGRAM_ID);
    const depositStakeIxArgs = {
        depositStakeArgs: {
            owner: authorizedPubkey,
        },
    };
    const depositStakeIxAccounts = {
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
        clock: web3_js_1.SYSVAR_CLOCK_PUBKEY,
        stakeHistory: web3_js_1.SYSVAR_STAKE_HISTORY_PUBKEY,
        tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
        stakeProgram: web3_js_1.StakeProgram.programId,
        systemProgram: web3_js_1.SystemProgram.programId,
    };
    const depositStakeIx = (0, generated_1.createDepositStakeInstruction)(depositStakeIxAccounts, depositStakeIxArgs);
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
exports.depositStake = depositStake;
/**
 * Generates the withdraw authority program address for the stake pool
 */
const findWithdrawAuthorityProgramAddress = (programId, stakePoolAddress) => {
    const [publicKey] = web3_js_1.PublicKey.findProgramAddressSync([stakePoolAddress.toBuffer(), Buffer.from("withdraw")], programId);
    return publicKey;
};
/**
 * Generates the stake program address for a validator's vote account
 */
const findStakeProgramAddress = (programId, voteAccountAddress, stakePoolAddress) => {
    const [publicKey] = web3_js_1.PublicKey.findProgramAddressSync([
        voteAccountAddress.toBuffer(),
        stakePoolAddress.toBuffer(),
        Buffer.alloc(0),
    ], programId);
    return publicKey;
};
