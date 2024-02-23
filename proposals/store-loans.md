# Upload Stargaze NFT Loans NC  v0.4.0 

This proposal is to store the Stargaze version of the on-chain loans contract, with AtlasDAO core team granted instantiate permissions.

The source code is available at: https://github.com/AtlasDA0/app-contracts

Features of the loans contract include:
- Create listings of nfts as collateral, permissionlessly.
- Create loan term offers on collateral listings.
- Accept or reject loan term offers.
- Hold collateral in escrow until repayment of fees.
- Upon loan repayment, collateral returned to owner
- Default function upon failure to repay loan within agreed upon timeframe.
- Admin controlled contract config
- Governance override contract lock via SudoMsg
- Static fee to create a loan listing
- Repaid interest tax % enforced

### Compile Instructions
```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.15.0
```
determine the checksum
```sh
sha256sum artifacts/nft_loans_nc.wasm
```
This results in the following SHA256 checksum: 
```
31fa695f6715cedcfd763d2ef4fc239fe2ab8ea20998069a5689b209741dd9bf  nft_loans_nc.wasm
```
### Verify code 
```
starsd  q gov proposal $id --output json \
| jq -r '.content.wasm_byte_code' \
| base64 -d \
| gzip -dc \
| sha256sum
```