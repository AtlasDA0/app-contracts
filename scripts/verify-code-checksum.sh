#!/bin/bash

# setup command-line arguments
if [ -n "$1" ]; then
    LOAN_ID="$1"
else
    LOAN_ID=3640
fi
if [ -n "$2" ]; then
    RAFFLE_ID="$2"
else
    RAFFLE_ID=3641
fi
    
# compute expected results
sha256sum target/wasm32-unknown-unknown/release/raffles.wasm
sha256sum target/wasm32-unknown-unknown/release/nft_loans_nc.wasm

# download binaries from network
st q wasm code $RAFFLE_ID raffle-code.wasm
st q wasm code $LOAN_ID loan-code.wasm

# compute download binary checksums 
sha256sum raffle-code.wasm 
sha256sum loan-code.wasm

