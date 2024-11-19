STAKE_POOL_KEYPAIR_PATH="./keys/stake-pool.json"
FEE_WALLET_KEYPAIR="./keys/identity_1.json"

# TODO update authority keypair to be that of the cli config manager?
AUTHORITY_KEYPAIR=$(solana config get | grep "Keypair Path" | awk '{print $3}')

if [ ! -f "$STAKE_POOL_KEYPAIR_PATH" ]; then
  echo "Error: Keypair file not found at $STAKE_POOL_KEYPAIR_PATH"
  exit 1
fi

if [ ! -f "$FEE_WALLET_KEYPAIR" ]; then
  echo "Error: Keypair file not found at $FEE_WALLET_KEYPAIR"
  exit 1
fi
if [ ! -f "$AUTHORITY_KEYPAIR" ]; then
  echo "Error: Keypair file not found at $AUTHORITY_KEYPAIR"
  exit 1
fi

STAKE_POOL=$(solana-keygen pubkey "$STAKE_POOL_KEYPAIR_PATH")
FEE_WALLET=$(solana-keygen pubkey "$FEE_WALLET_KEYPAIR")
AUTHORITY=$(solana-keygen pubkey "$AUTHORITY_KEYPAIR")

# create stake_deposit_authority and set pubkey to var
STAKE_DEPOSIT_AUTHORITY=$(../target/debug/spl-stake-pool-interceptor interceptor create-stake-deposit-authority --pool $STAKE_POOL --fee-wallet $FEE_WALLET --authority $AUTHORITY_KEYPAIR --cool-down-seconds 200 --initial-fee-bps 100 | tail -1)

# TODO update stake pool stake_deposit_authority to $STAKE_DEPOSIT_AUTHORITY