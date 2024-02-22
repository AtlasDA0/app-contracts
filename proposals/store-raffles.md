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
rustup target add wasm32-unknown-unknown
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --no-default-features --features sg
```
determine the checksum
```sh
sha256sum target/wasm32-unknown-unknown/release/raffles.wasm
```
This results in the following SHA256 checksum: 
```
1ced0dd38d7a2588c37c5e9d2c723a15b98502eef04c5c1c82dd154ebd9bb02f  target/wasm32-unknown-unknown/release/raffles.wasm
```
### Verify code 
```
starsd  q gov proposal $id --output json \
| jq -r '.content.wasm_byte_code' \
| base64 -d \
| gzip -dc \
| sha256sum
```