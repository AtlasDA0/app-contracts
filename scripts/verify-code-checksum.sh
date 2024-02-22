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
sha256sum loan-code.wasm
sha256sum raffle-code.wasm 

# 8ff7d3f96fdad07e4157dfe067700ad3f06be712d9ceead374b92c87c3288856  loan-code.wasm
# 1ced0dd38d7a2588c37c5e9d2c723a15b98502eef04c5c1c82dd154ebd9bb02f  raffle-code.wasm