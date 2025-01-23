# Stake Depsit Interceptor CLI

A lot of the CLI code is copy/pasta'd from Solana Program Library at commit `d85ea9ff0573cd82c9245965a55f379b4dc263bc` to keep in line with solana-program versions being used by other deps.

See https://spl.solana.com/stake-pool for the base functionality.

## New Commands

### set-funding-authority-ix-serialized
A command that prints a serialized version of SetFundingAuthority instruction intended to be used within SPL Governance.

### interceptor create-stake-deposit-authority
Creates the StakePoolStakeDepositAuthority on the interceptor program. This account would become the `stake_deposit_authority` on any StakePool that should be proxied through the Interceptor program.

### interceptor deposit-stake
Deposit a Stake account to the specified StakePool through the Interceptor. This should only be used when the StakePool has a StakePoolStakeDepositAuthority as the `stake_deposit_authority`.
