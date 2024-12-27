# CLMM AI Dex

AI Dex is an open-source concentrated liquidity AMM contract on the Solana blockchain, developed by Solanex Team.

## Requirements

- Anchor 0.30.1
- Solana 1.18.26
- Rust 1.75.0+

## Setup

Install Anchor using instructions found [here](https://book.anchor-lang.com/getting_started/installation.html#anchor).

## Build

run `anchor build` in the project's root to generate target folder

After that, you'll get `target/deploy/ai_dex-keypair.json` keypair. Please check if the pubkey of that keypair is the same as in the `lib.rs` declare_id and also in the `Anchor.toml`. If no, please change it

## Deploy

run `anchor deploy` 

## Verify

run `anchor idl init -f target/idl/ai_dex.json <YOUR_PUBKEY_OF_AI_DEX_KEYPAIR>`

## Test

To run internal rust test cases, please run `cargo test`

# License

[Apache 2.0](https://choosealicense.com/licenses/apache-2.0/)
