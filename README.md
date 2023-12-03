# SolPie Solana Program

## Project Description

This Solana program is designed for a decentralized pass marketplace on the Solana blockchain. It facilitates the buying and selling of passes, which could be interpreted as NFTs or access tokens. The program includes mechanisms for price determination, fee setting, and initialization of marketplace parameters.

## Scripts and Their Purpose

- `init.rs`: Initializes the marketplace with necessary parameters and configurations.
- `set_fee_pct.rs`: Sets the percentage fee for transactions within the marketplace.
- `set_protocol_fee_dst.rs`: Defines the destination address for protocol fees.
- `buy_passes.rs`: Allows users to purchase passes.
- `sell_passes.rs`: Enables users to sell their passes.
- `get_price.rs`: Retrieves the current price of passes.
- `buy_passes_sol.rs`: Specialized script for purchasing passes using Solana (SOL) cryptocurrency.
