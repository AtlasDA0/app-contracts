all: 
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown
	cp ./target/wasm32-unknown-unknown/release/crosschain_contract.wasm ./crosschain_contract.wasm

sg: 
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --no-default-features --features sg
	cp ./target/wasm32-unknown-unknown/release/crosschain_contract.wasm ./crosschain_contract.wasm

vanilla:
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --no-default-features --features vanilla
	cp ./target/wasm32-unknown-unknown/release/crosschain_contract.wasm ./crosschain_contract.wasm


clean:
	cargo clean
	-rm -f ./v1_contract.wasm