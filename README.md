# Topdown Racer

## Prerequisites

- Rust (stable toolchain)
- Bevy prerequisites for your platform (if you run into runtime issues)

## Quick setup

From repository root:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

Because this repository includes `rust-toolchain.toml`, `cargo` will prefer the pinned
`stable` toolchain automatically once installed.

## Sanity check commands

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo run --package topdown-racer
```
