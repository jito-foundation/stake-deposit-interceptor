# @jito-foundation/stake-deposit-interceptor-sdk

A TypeScript SDK for interacting with Jito's stake deposit interceptor program on Solana. This program acts as the `stake_deposit_authority` for SPL stake pools, implementing a time-decaying fee mechanism on LST (Liquid Staking Token) deposits.

More information available in the [Jito governance forum](https://forum.jito.network/t/jip-9-adopt-interceptor-liquidity-defense/444).

## Installation

```bash
npm install @jito-foundation/stake-deposit-interceptor-sdk
# or
yarn add @jito-foundation/stake-deposit-interceptor-sdk
```

## Key Features

- 🛡️ **Time-Decaying Fees**: Implements fees that linearly decay to zero over a configurable period
- 🔐 **Stake Pool Integration**: Acts as stake deposit authority for SPL stake pools
- 🎫 **Deposit Management**: Handles stake deposits with receipt tracking
- 🤖 **Automated Claims**: Permissionless cranking system for fee-free claims after cooldown
- 📝 **Type Safety**: Fully typed TypeScript SDK

## Important Concepts

### Deposit Flow
1. User deposits stake using `depositStake()` or `depositStakeWithSlippage()`
2. Stake begins earning rewards immediately
3. LST tokens are minted and held by the program
4. User can either:
   - Claim LST early with a fee
   - Wait for cooldown and claim fee-free (can be automated by cranker)

### Slippage Protection
`depositStakeWithSlippage()` allows setting a minimum LST output to protect against unfavorable rate changes during transaction processing.

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

## Instructions

### Stake Operations
- `depositStake()`: Deposit stake into the pool, creating a deposit receipt
- `depositStakeWithSlippage()`: Deposit stake with added slippage protection
- `claimPoolTokens()`: Claim LST tokens (with fees if before cooldown)

### Authority Management
- `initStakePoolDepositStakeAuthority()`: Initialize the deposit authority for a stake pool
- `updateStakePoolDepositStakeAuthority()`: Update authority parameters

### Receipt Management
- `updateOwner()`: Transfer deposit receipt ownership to a new address

## License

MIT

## Support

For more information and support:
- Governance Forum: [Jito Forum](https://forum.jito.network/t/jip-9-adopt-interceptor-liquidity-defense/444)
- Issues: [GitHub Repository Issues]