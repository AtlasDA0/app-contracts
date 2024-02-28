all:
	make sg vanilla
	sha256sum artifacts/*

sg: 
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --no-default-features --features sg
	mkdir artifacts
	mv ./target/wasm32-unknown-unknown/release/raffles.wasm ./artifacts/sg_raffles.wasm 
	mv ./target/wasm32-unknown-unknown/release/nft_loans_nc.wasm ./artifacts/sg_nft_loans_nc.wasm

vanilla:
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --no-default-features --features vanilla
	mv ./target/wasm32-unknown-unknown/release/raffles.wasm ./artifacts/vanilla_raffles.wasm 
	mv ./target/wasm32-unknown-unknown/release/nft_loans_nc.wasm ./artifacts/vanilla_nft_loans_nc.wasm 


clean:
	cargo clean