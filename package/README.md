# @jito-foundation/stake-deposit-interceptor-sdk

A TypeScript SDK for interacting with Jito's stake deposit interceptor program on Solana. This program acts as the `stake_deposit_authority` for SPL stake pools, implementing a time-decaying fee mechanism on LST (Liquid Staking Token) deposits.

More information available in the [Jito governance forum](https://forum.jito.network/t/jip-9-adopt-interceptor-liquidity-defense/444).

## Installation

```bash
npm install @jito-foundation/stake-deposit-interceptor-sdk
```

## Core Functionality

The SDK provides functionality to interact with the stake deposit interceptor program:

### Deposit Stake
```typescript
import { depositStake } from '@jito-foundation/stake-deposit-interceptor-sdk';

const { instructions, signers } = await depositStake(
  connection,
  payer,
  stakePoolAddress,
  authorizedPubkey,
  validatorVote,
  depositStake,
  poolTokenReceiverAccount // optional
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