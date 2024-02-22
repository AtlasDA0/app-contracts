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
res=$(st q wasm contract $LOAN_CONTRACT  --json)
res=$(st q wasm contract $RAFFLE_CONTRACT  --json)

# get code id
code_id=$(echo $res | jq -r '.contract_info.code_id')

# download binaries from network
st q wasm code $code_id loan-code.wasm
st q wasm code $code_id raffle-code.wasm

# verify codehash
sha256sum loan-code.wasm
# 8ff7d3f96fdad07e4157dfe067700ad3f06be712d9ceead374b92c87c3288856  loan-code.wasm
# 1ced0dd38d7a2588c37c5e9d2c723a15b98502eef04c5c1c82dd154ebd9bb02f  raffle-code.wasm


