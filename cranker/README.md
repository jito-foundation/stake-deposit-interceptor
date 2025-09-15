# Cranker Bot Quick-start

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

Then copy file contents below into the cranker sub-directory dotenv file at `./cranker/.env`. Everything should be set as-is, however you will need to include an `RPC_URL` and `WS_URL` that can handle getProgramAccounts calls.

```bash
# RPC URL for the cluster
RPC_URL="YOUR RPC URL"

# Websocket URL for the cluster
WS_URL="YOUR RPC WS URL"

# Cluster to specify (mainnet, testnet, devnet)
CLUSTER=mainnet

# Region to specify for metrics purposes (us-east, eu-west, local, etc.)
REGION=local

# Log levels
RUST_LOG="info,solana_gossip=error,solana_metrics=info"

# Keypair Configuration (choose ONE of the following methods):

# Method 1: Use keypair seed as JSON array (recommended for production)
KEYPAIR_SEED=[1,1,1,1,1,1,1,....]

# Method 2: Use keypair file path (if KEYPAIR_SEED is not provided)
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

#### Configuration Details:

**Keypair Configuration**:

**KEYPAIR_SEED**: Provide your keypair as a JSON array of 64 u8 values. This is the recommended method for production deployments as it avoids file system dependencies.

- To get the seed from an existing keypair file: cat ./credentials/keypair.json
- The bot will prioritize KEYPAIR_SEED if both are provided

**KEYPAIR_PATH**: Alternative method using a file path to your keypair JSON file

**Network Configuration**:

- RPC_URL: Must be a reliable RPC endpoint that supports getProgramAccounts calls
- WS_URL: WebSocket endpoint for real-time updates (should match your RPC provider)
- CLUSTER: Network identifier for logging and metrics
- REGION: Geographic region for monitoring purposes

**Operation Settings**:

- PROGRAM_ID: The Solana program this cranker monitors and interacts with
- INTERVAL_SECONDS: How frequently (in seconds) the bot checks for cranking opportunities

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

