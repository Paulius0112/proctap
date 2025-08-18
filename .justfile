
check:
    cargo check --workspace
    cargo fmt --all -- --check
    cargo clippy --all-targets

fix:
    cargo fmt --all
    cargo clippy --allow-dirty --allow-staged --fix --all-targets --all-features -- -W unused_imports -W clippy::all
    cargo clippy --allow-dirty --allow-staged --fix -- -W unused_imports -W clippy::all

build:
    cargo build --all-targets

run:
    RUST_LOG=info cargo run