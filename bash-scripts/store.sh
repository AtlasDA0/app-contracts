#!/bin/bash
for d in ../artifacts/nft_loans_nc.wasm; do 
echo $d;
st tx wasm store artifacts/nft_loans_nc.wasm  --from test1 --gas auto --fees 10000000ustars --gas-adjustment 2 -y
done 
sleep 6

for e in ../artifacts/raffles.wasm; do 
echo $e;
st tx wasm store artifacts/raffles.wasm  --from test1 --gas auto --fees 10000000ustars --gas-adjustment 2 -y
done