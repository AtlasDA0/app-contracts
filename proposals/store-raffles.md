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
e7b1f700d7bc23c9ad59871da006b95479b74f94313e8e161225d25670a6d6cc  target/wasm32-unknown-unknown/release/raffles.wasm
```
### Verify code 
```
starsd  q gov proposal $id --output json \
| jq -r '.content.wasm_byte_code' \
| base64 -d \
| gzip -dc \
| sha256sum
```