#!/bin/bash
set -e
PROGRAM_PATH="../target/deploy/perun_solana_program.so"

echo "[deploy] Deploying program..."
solana program deploy "$PROGRAM_PATH"
