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

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    deposit-stake-whitelisted \
    --whitelist 7Qh3p1FpSAAeZer9kLXMyT8dS5PZatLgF9dxBd1BEoKV \
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

```bash
cargo r -p stake-deposit-interceptor-cli -- \
    stake-deposit-interceptor \
    interceptor \
    withdraw-stake-whitelisted \
    --whitelist 7Qh3p1FpSAAeZer9kLXMyT8dS5PZatLgF9dxBd1BEoKV \
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
    --whitelist 7Qh3p1FpSAAeZer9kLXMyT8dS5PZatLgF9dxBd1BEoKV \
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
    --whitelist 7Qh3p1FpSAAeZer9kLXMyT8dS5PZatLgF9dxBd1BEoKV \
    --rpc-url https://api.devnet.solana.com \
    --signer ~/.config/solana/id.json \
    --commitment confirmed \
    --stake-deposit-interceptor-program-id 2KVTQfCi5YfmgmTKyHTZVz8s1G3YHAxuhpW1J65sdwwu
```
