# Stake Deposit Interceptor CLI

## Stake

### Create Stake Account

```bash
solana-keygen grind --starts-with "a:1" --num-threads 64

solana create-stake-account aPrEkgZt19356GKqWEAwModRavVhVuyZ7WthbAs3tEu.json 1

solana delegate-stake aPrEkgZt19356GKqWEAwModRavVhVuyZ7WthbAs3tEu vgcDar2pryHvMgPkKaZfh8pQy4BJxv7SpwUG7zinWjG

solana stake-authorize aPrEkgZt19356GKqWEAwModRavVhVuyZ7WthbAs3tEu \
    --new-stake-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --stake-authority ~/.config/solana/id.json \
    --url https://api.devnet.solana.com

solana stake-authorize aPrEkgZt19356GKqWEAwModRavVhVuyZ7WthbAs3tEu \
    --new-withdraw-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --withdraw-authority ~/.config/solana/id.json \
    --url https://api.devnet.solana.com
```

## Stake Pool

### Deposit SOL

```bash
cargo r -p spl-stake-pool-cli -- deposit-sol JitoY5pcAxWX6iyP2QdFwTznGb8A99PRCUCVVxB46WZ 1 --program-id DPoo15wWDqpPJJtS2MUZ49aRxqz5ZaaJCJP4z8bLuib
```

## Stake Deposit Interceptor

### Interceptor

#### Create stake deposit authority

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    create-stake-deposit-authority \
    --pool  JitoY5pcAxWX6iyP2QdFwTznGb8A99PRCUCVVxB46WZ \
    --fee-wallet BBBATax9kikSHQp8UTcyQL3tfU3BmQD9yid5qhC7QEAA \
    --cool-down-seconds 100 \
    --initial-fee-bps 10 \
    --authority BBBATax9kikSHQp8UTcyQL3tfU3BmQD9yid5qhC7QEAA \
    --spl-stake-pool-program-id DPoo15wWDqpPJJtS2MUZ49aRxqz5ZaaJCJP4z8bLuib \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```

#### Update Stake Deposit Authority

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    update-stake-deposit-authority \
    --stake-deposit-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```

#### Get Stake Deposit Authority

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    get-stake-deposit-authority \
    --stake-deposit-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```

#### Deposit Stake

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    deposit-stake \
    --stake-deposit-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --stake-account aPrEkgZt19356GKqWEAwModRavVhVuyZ7WthbAs3tEu \
    --withdraw-authority BBBATax9kikSHQp8UTcyQL3tfU3BmQD9yid5qhC7QEAA \
    --spl-stake-pool-program-id DPoo15wWDqpPJJtS2MUZ49aRxqz5ZaaJCJP4z8bLuib \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```


#### List Receipts

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    list-receipts \
    --program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu \
    --stake-pool JitoY5pcAxWX6iyP2QdFwTznGb8A99PRCUCVVxB46WZ \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed
```

#### Claim Tokens

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    claim-tokens \
    --receipt-address 2BfodsQRsaQMT3rR7gyNCqge7FJEbiv5baQnXi59tLPp \
    --create-ata \
    --after-cooldown \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```


#### Deposit Stake Whitelisted

Deposits an active stake account into the stake pool via the interceptor program. 
The stake account must be delegated to a validator that is in the pool's validator list, and must be fully active (not activating). 

**Arguments:**

| Argument | Description |
|---|---|
| `--whitelist` | The whitelist account that authorizes this deposit |
| `--stake-deposit-authority` | The `StakePoolDepositStakeAuthority` PDA that the interceptor program uses to manage deposits |
| `--deposit-stake` | The stake account to deposit. Must be fully active and delegated to a validator in the pool |
| `--validator-stake` | The pool's validator stake account for the same validator your stake is delegated to (find this in the pool's validator list) |
| `--spl-stake-pool-program-id` | The program ID of the SPL stake pool program |
| `--rpc-url` | Solana RPC endpoint URL |
| `--signer` | Path to the keypair file used to sign the transaction |
| `--commitment` | Transaction commitment level (`processed`, `confirmed`, or `finalized`) |
| `--stake-deposit-interceptor-program-id` | The program ID of the stake deposit interceptor program |

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    deposit-stake-whitelisted \
    --whitelist 9KsQ7Wj99tz3ZayVXEEGSGCC2ZYb7CAyJn1GF94CyymS \
    --stake-deposit-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --deposit-stake 8JWkyisNoErjT28uYahdo9o1GfPxunMSW9Vyj99rtz7F \
    --validator-stake 6jDDM4Agc9sKq5JT1phcBSFCxCJPyjbusk6RmG5WEphT \
    --spl-stake-pool-program-id DPoo15wWDqpPJJtS2MUZ49aRxqz5ZaaJCJP4z8bLuib \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```

#### Withdraw Stake Whitelisted

Withdraws stake from the stake pool via the interceptor program. Splits stake from a validator's stake account in the pool and assigns it to a new stake account with the specified authority.

**Arguments:**

| Argument | Description |
|---|---|
| `--whitelist` | The whitelist account that authorizes this withdrawal |
| `--stake-deposit-authority` | The `StakePoolDepositStakeAuthority` PDA that the interceptor program uses to manage withdrawals |
| `--stake-split-from` | The pool's validator stake account to split stake from |
| `--stake-split-to` | Path to a keypair file for the new stake account that will receive the withdrawn stake |
| `--user-stake-authority` | The authority to set on the newly created stake account (both stake and withdraw authority) |
| `--fee-rebate-recipient` | The account that receives any fee rebate from the withdrawal |
| `--spl-stake-pool-program-id` | The program ID of the SPL stake pool program |
| `--amount` | Amount of pool tokens to withdraw (in SOL equivalent) |
| `--rpc-url` | Solana RPC endpoint URL |
| `--signer` | Path to the keypair file used to sign the transaction |
| `--commitment` | Transaction commitment level (`processed`, `confirmed`, or `finalized`) |
| `--stake-deposit-interceptor-program-id` | The program ID of the stake deposit interceptor program |

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    withdraw-stake-whitelisted \
    --whitelist 9KsQ7Wj99tz3ZayVXEEGSGCC2ZYb7CAyJn1GF94CyymS \
    --stake-deposit-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --stake-split-from 6jDDM4Agc9sKq5JT1phcBSFCxCJPyjbusk6RmG5WEphT \
    --stake-split-to ./target/deploy/stake_e.json \
    --user-stake-authority BBBATax9kikSHQp8UTcyQL3tfU3BmQD9yid5qhC7QEAA \
    --fee-rebate-recipient BBBATax9kikSHQp8UTcyQL3tfU3BmQD9yid5qhC7QEAA \
    --spl-stake-pool-program-id DPoo15wWDqpPJJtS2MUZ49aRxqz5ZaaJCJP4z8bLuib \
    --amount 1 \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```

#### Fund Hopper

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    fund-hopper \
    --whitelist 9KsQ7Wj99tz3ZayVXEEGSGCC2ZYb7CAyJn1GF94CyymS \
    --stake-deposit-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --lamports 1 \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```

#### Hopper Balance

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    hopper-balance \
    --whitelist 9KsQ7Wj99tz3ZayVXEEGSGCC2ZYb7CAyJn1GF94CyymS \
    --stake-deposit-authority Ne9DQUbAfSuFSfzwgGms3f1AZvGipcmnpj29YRqJqCN \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```
