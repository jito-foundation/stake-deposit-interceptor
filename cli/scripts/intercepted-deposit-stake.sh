#!/bin/bash
set -e

# Parameters:
# $1 - Path to staker keypair (withdraw authority)
# $2 - Stake deposit authority
# $3 - Amount to deposit

staker_keypair=$1
stake_deposit_authority=$2
amount=$3
stake_pool="HYV3n5Qj3ycbZxJdDPgRp8ok8peVAychHw6jGqHRpqrF"
stake_account_keypair="./keys/stake-account.json"

# First update the pool
echo "Updating stake pool..."
spl-stake-pool update $stake_pool --no-merge
sleep 2
spl-stake-pool update $stake_pool

# Request airdrop
echo "Requesting airdrop of $amount SOL"
solana airdrop $amount

# Create stake account
echo "Generating a new keypair"
solana-keygen new --no-bip39-passphrase -o $stake_account_keypair --force

stake_account=$(solana-keygen pubkey $stake_account_keypair)
echo "Creating stake account"
solana create-stake-account $stake_account_keypair $amount

echo "Delegating stake"
vote_account=$(solana-keygen pubkey ./keys/vote_1.json)
echo "Using vote account: $vote_account"
solana delegate-stake --force $stake_account_keypair $vote_account

# Update pool again after delegation
echo "Updating stake pool after delegation..."
spl-stake-pool update $stake_pool --no-merge
sleep 2
spl-stake-pool update $stake_pool

echo "Depositing stake via Interceptor"
../target/debug/spl-stake-pool-interceptor interceptor deposit-stake \
    $stake_deposit_authority \
    $stake_account \
    --staker $staker_keypair