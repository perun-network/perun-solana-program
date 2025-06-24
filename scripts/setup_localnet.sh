#!/bin/bash
set -e

SCRIPTS_DIR="scripts"
ACCOUNTS_DIR="accounts"

if [ -d $ACCOUNTS_DIR ]; then
  rm -rf $ACCOUNTS_DIR/*
fi
mkdir -p $ACCOUNTS_DIR

# Build the program
echo "[build] Building the program..."
cd ..
cargo build-sbf 
cd $SCRIPTS_DIR

# Ensure CLI is localnet
solana config set -ul

echo "[keygen] Generating fee payer..."
solana-keygen new --no-bip39-passphrase --force --outfile $ACCOUNTS_DIR/fee_payer.json

echo "[setup] Generating keypairs"
solana-keygen new --no-bip39-passphrase --force --outfile $ACCOUNTS_DIR/alice.json
solana-keygen new --no-bip39-passphrase --force --outfile $ACCOUNTS_DIR/bob.json
