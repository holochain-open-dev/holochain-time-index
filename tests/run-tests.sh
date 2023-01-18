CARGO_TARGET_DIR=target cargo build --release --target wasm32-unknown-unknown
hc app pack workdir
cd tests && npm run test
