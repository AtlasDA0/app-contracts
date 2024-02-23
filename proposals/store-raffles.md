# Upload Stargaze Raffles  v0.4.0 

This proposal is to store the Stargaze version of the on-chain raffle contract, with AtlasDAO core team granted instantiate permissions.

The source code is available at: https://github.com/AtlasDA0/app-contracts

Features of the raffle contract include:
- Create & configure raffles permissionlessly
- Purchase raffle tickets
- Verifiable randomness to determine winner, via Nois Network
- Admin controlled params to optimize configuration
- Governance override contract lock via SudoMsg
- Static fee to creating raffles
- Raffle ticket sales tax

### Compile Instructions
```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.15.0
```
determine the checksum
```sh
sha256sum artifacts/raffles.wasm
```
This results in the following SHA256 checksum: 
```
d7656019911745b97b54db86f6df2ef0a59ceaa12f70f358dd98fb8e29361720  raffles.wasm
```
### Verify code 
```
starsd  q gov proposal $id --output json \
| jq -r '.content.wasm_byte_code' \
| base64 -d \
| gzip -dc \
| sha256sum
```
