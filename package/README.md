# @jito-foundation/stake-deposit-interceptor-sdk

A TypeScript SDK for interacting with Jito's stake deposit interceptor program on Solana. This program acts as the `stake_deposit_authority` for SPL stake pools, implementing a time-decaying fee mechanism on LST (Liquid Staking Token) deposits.

More information available in the [Jito governance forum](https://forum.jito.network/t/jip-9-adopt-interceptor-liquidity-defense/444).

## Installation

```bash
npm install @jito-foundation/stake-deposit-interceptor-sdk
```

## Migration from @solana/spl-stake-pool

This SDK extends the standard SPL stake pool functionality by adding a cool-down period and fee mechanism for stake deposits.

### Depositing Stake
When depositing stake, instead of receiving LST tokens immediately, they are held by the interceptor program until claimed. All you need to do is swap out the spl stake pool program depositStake method for the jito interceptor version:

```typescript
import { depositStake } from '@jito-foundation/stake-deposit-interceptor-sdk';

// Returns instructions to deposit stake and create a deposit receipt
const { instructions, signers } = await depositStake(
  connection,
  wallet.publicKey,  // payer for deposit receipt rent
  stakePoolAddress,  // stake pool to deposit into
  wallet.publicKey,  // authorized withdrawer/staker
  validatorVote,     // validator vote account
  stakeAccount,      // stake account to deposit
  poolTokenReceiverAccount // optional, will create ATA if not provided
);
```

Note: When depositing stake, the LST tokens are not immediately sent to the token account. Instead:
1. The tokens are minted and held by the program in its vault
2. A deposit receipt is created with `authorizedPubkey` as the owner
3. The owner can later claim the LST tokens:
   - During cooldown period: Pay a time-decaying fee
   - After cooldown period: Claim without fees (can be done by anyone)

### Generated Instructions
The SDK also includes generated instruction builders for:
- `createDepositStakeWithSlippageInstruction`: Deposit with minimum LST output protection
- `createClaimPoolTokensInstruction`: Claim LST tokens (with fees during cooldown)

### Claiming Deposited Stake
After the deposit, you can claim the LST tokens. A fee may apply if claiming before the cool-down period ends:

```typescript
import { 
  createClaimPoolTokensInstruction,
  StakePoolDepositStakeAuthority,
  PROGRAM_ID
} from '@jito-foundation/stake-deposit-interceptor-sdk';

// Get the deposit authority account which contains fee/vault info
const depositAuthority = await StakePoolDepositStakeAuthority.fromAccountAddress(
  connection,
  receipt.stakePoolDepositStakeAuthority
);

// Create claim instruction
const claimIx = createClaimPoolTokensInstruction({
  depositReceipt: receiptAddress,    // address of the deposit receipt
  owner: wallet.publicKey,           // receipt owner
  vault: depositAuthority.vault,     // vault holding LST tokens
  destination: destinationTokenAccount, // where to send claimed tokens
  feeWallet: feeWalletAta,          // where fees are sent if applicable
  depositAuthority: receipt.stakePoolDepositStakeAuthority,
  poolMint,                         // LST mint
  tokenProgram: TOKEN_PROGRAM_ID,
  systemProgram: SystemProgram.programId,
});
```

### Finding Deposit Receipts
To fetch all unclaimed deposits for a wallet:

```typescript
import { DepositReceipt, PROGRAM_ID } from '@jito-foundation/stake-deposit-interceptor-sdk';

// Fetch all receipts for a wallet
const accounts = await connection.getProgramAccounts(PROGRAM_ID, {
  filters: [
    // Filter by owner (40 bytes offset: 8 bytes discriminator + 32 bytes base)
    { memcmp: { offset: 40, bytes: walletPublicKey.toBase58() } },
  ],
});

// Parse the receipt accounts
const receipts = accounts.map(({ pubkey, account }) => {
  const [receipt] = DepositReceipt.fromAccountInfo(account);
  return {
    address: pubkey,    // deposit receipt address
    receipt,           // parsed receipt data including:
                      // - lstAmount: amount of LST to claim
                      // - depositTime: when stake was deposited
                      // - coolDownSeconds: required wait for fee-free claim
                      // - initialFeeBps: starting fee rate
  };
});
```

## Program Accounts

### StakePoolDepositStakeAuthority
The main control account for the stake pool's deposit authority:
```typescript
interface StakePoolDepositStakeAuthority {
    base: PublicKey;
    stakePool: PublicKey;
    poolMint: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
    stakePoolProgramId: PublicKey;
    coolDownSeconds: BN;
    initialFeeBps: number;
    feeWallet: PublicKey;
    bumpSeed: number;
    reserved: number[]; // 256 bytes of reserved space
}
```

### DepositReceipt
Tracks individual stake deposits and their associated parameters:
```typescript
interface DepositReceipt {
    base: PublicKey;
    owner: PublicKey;
    stakePool: PublicKey;
    stakePoolDepositStakeAuthority: PublicKey;
    depositTime: BN;
    lstAmount: BN;
    coolDownSeconds: BN;
    initialFeeBps: number;
    bumpSeed: number;
    reserved: number[]; // 256 bytes of reserved space
}
```

## How It Works

1. When a user deposits stake, the program acts as the stake pool's deposit authority
2. LST tokens are minted but held by the program in its vault
3. Users can:
   - Claim LST early by paying a time-decaying fee
   - Wait for the cooldown period to claim without fees
4. After cooldown, claims can be processed by anyone (permissionless cranking)

## License

MIT

## More Information

For more details about the program and its mechanics, see the [Jito governance forum](https://forum.jito.network/t/jip-9-adopt-interceptor-liquidity-defense/444).