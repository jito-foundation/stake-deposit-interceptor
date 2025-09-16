# Stake Deposit Interceptor Cranker

Below are the steps to configuring and running the Stake Deposit Interceptor Bot. We recommend running it as a docker container.

## Setup

### Credentials

In the root directory create a new folder named `credentials` and then populate it with a keypair. This is keypair that signs and pays for all transactions.

```bash
mkdir credentials
solana-keygen new -o ./credentials/keypair.json
```

### ENV

In the cranker directory create `.env` file

```bash
cd cranker
touch .env
```

Then copy file contents below into the cranker sub-directory dotenv file at `./cranker/.env`. Everything should be set as-is, however you will need to include an `RPC_URL` that can handle getProgramAccounts calls.

```bash
# RPC URL for the cluster
RPC_URL="YOUR RPC URL"

# Cluster to specify (mainnet, testnet, devnet)
CLUSTER=mainnet

# Region to specify for metrics purposes (us-east, eu-west, local, etc.)
REGION=local

# Log levels
RUST_LOG="info,solana_gossip=error,solana_metrics=info"

# Path to keypair used to execute tranasactions
KEYPAIR_PATH=../credentials/keypair.json

# Program ID (Pubkey as base58 string)
PROGRAM_ID=5TAiuAh3YGDbwjEruC1ZpXTJWdNDS7Ur7VeqNNiHMmGV

# Interval in seconds to check for cranking conditions
INTERVAL_SECONDS=60

# Log levels
RUST_LOG="info,solana_gossip=error,solana_metrics=info"

# Metrics upload influx server (optional)
SOLANA_METRICS_CONFIG=""
```

## Running Docker image from source

Once the setup is complete use the following commands to run/manage the docker container:

> Note: We are running `Docker version 24.0.5, build ced0996`

### Start Docker

```bash
docker compose --env-file .env up -d --build  interceptor-cranker --remove-orphans
```

### View Logs

```bash
docker logs interceptor-cranker -f
```

### Stop Docker\*\*

```bash
docker stop interceptor-cranker; docker rm interceptor-cranker;
```

## Running as Binary

To run the keeper in terminal, build for release and run the program.

### Build for Release

```bash
cd cranker
cargo build --release
```

### Run Keeper

```bash
RUST_LOG=info ./target/release/stake-deposit-interceptor-cranker
```

To see all available parameters run:

```bash
RUST_LOG=info ./target/release/stake-deposit-interceptor-cranker -h
```

