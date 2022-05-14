### RUST ALGOTRADER
A decently performant algorithmic trading system for crypto exchanges, built from the ground up as a personal project to help me and my friend explore the space. This project is a demonstration version of a private project and has had content redacted. It is incomplete, requiring additional API connections, stability fixes, account behaviors, documentation, and organization/scaffolding.
# Use at your own risk.
The implementation of insightful analysis and profitable strategies is left as an exercise to the reader.

## Quickstart (for Binance testnet)
1. Install Rust and Cargo
2. Make Binance testnet or live account
3. Make a real environment file from either shell/batch sample and exclude it in .gitignore
4. Populate env file with API keys
5. Run either `.env.bat` or `source .env.sh`
6. Run `cargo run --release`

## General Info
The trader maintains two primary threads: market modelling (src/signal_handler), and account modelling/strategy (src/strategy).
The market thread pipeline is to receive market updates from REST/websocket threads, update respective models, generate an analysis result, and push it to the strategy thread.
Order book and trade flow models are kept in the src/orderbook and src/tradeflow folders. Analysis can be found in src/analysis.
The strategy thread pipeline is to update account models with account and order updates, and execute orders with market model updates.
REST and websocket connectors are kept in src/backend.

This project makes use of the dec library
 https://docs.rs/dec/latest/dec/#
 It is provided in src/dec with modifications under its original license to solve a number of internal problems with using it as a crate.
