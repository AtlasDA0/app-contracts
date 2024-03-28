#!/bin/bash

# setup command-line arguments
if [ -n "$1" ]; then
    LOAN_ID="$1"
else
    LOAN_ID=3649
fi
if [ -n "$2" ]; then
    RAFFLE_ID="$2"
else
    RAFFLE_ID=3650
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

# 31fa695f6715cedcfd763d2ef4fc239fe2ab8ea20998069a5689b209741dd9bf  loan-code.wasm
# d7656019911745b97b54db86f6df2ef0a59ceaa12f70f358dd98fb8e29361720  raffle-code.wasm