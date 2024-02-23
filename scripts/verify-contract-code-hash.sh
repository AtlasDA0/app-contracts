#!/bin/bash

# setup command-line arguments
if [ -n "$1" ]; then
    LOAN_CONTRACT="$1"
else
    LOAN_CONTRACT=
fi
if [ -n "$2" ]; then
    RAFFLE_CONTRACT="$2"
else
    RAFFLE_CONTRACT=
fi
    
# compute expected results
resl=$(st q wasm contract $LOAN_CONTRACT  --json)
resr=$(st q wasm contract $RAFFLE_CONTRACT  --json)

# get code id
code_idl=$(echo $resl | jq -r '.contract_info.code_id')
code_idr=$(echo $resr | jq -r '.contract_info.code_id')

# download binaries from network
st q wasm code $code_idl loan-code.wasm
st q wasm code $code_idr raffle-code.wasm

# verify codehash
sha256sum loan-code.wasm
sha256sum raffle-code.wasm
# 31fa695f6715cedcfd763d2ef4fc239fe2ab8ea20998069a5689b209741dd9bf  loan-code.wasm
# d7656019911745b97b54db86f6df2ef0a59ceaa12f70f358dd98fb8e29361720  raffle-code.wasm


