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
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --no-default-features --features sg
```
determine the checksum
```sh
sha256sum target/wasm32-unknown-unknown/release/nft_loans_nc.wasm
```
This results in the following SHA256 checksum: 
```
8ff7d3f96fdad07e4157dfe067700ad3f06be712d9ceead374b92c87c3288856  target/wasm32-unknown-unknown/release/nft_loans_nc.wasm
```
### Verify code 
```
starsd  q gov proposal $id --output json \
| jq -r '.content.wasm_byte_code' \
| base64 -d \
| gzip -dc \
| sha256sum
```