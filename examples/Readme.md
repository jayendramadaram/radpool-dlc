# Radpool-DLC

A Rust-based implementation of the Discreet Log Contract (DLC) protocol, built on top of the `dlc-manager` crate.

## Overview

This example demonstrates a complete DLC execution flow between two parties, Alice and Bob. It utilizes the `dlc-manager` crate from `rust-dlc` to handle the DLC lifecycle, including offer creation, acceptance, signing, and settlement.

## Prerequisites

1. Docker and Docker Compose
2. Rust

## Getting Started

1. Start the Oracle server, Oracle DB, and Bitcoin node in regtest mode, along with Electrs:

```bash
docker-compose --profile oracle up -d
```

2. Create wallets for Alice and Bob in the Bitcoin node:

```bash
docker compose exec bitcoind /scripts/create_wallets.sh
```

3. Run the example execution flow:
```bash
cargo run --example main    
```

This will execute the DLC flow between Alice and Bob. During the execution, the program will print a log asking you to mine 10 Bitcoin blocks.

4. Mine the 10 blocks:
```bash
docker compose exec bitcoind /scripts/generate_blocks.sh
```

This will mine the 10 blocks, allowing the DLC funding transaction to confirm and the parties to proceed with the contract maturity and settlement.

DLC Execution Flow
The main.rs example demonstrates the following DLC execution flow:

- Setup the parties (Alice and Bob) with their respective DlcManager and KeysManager.
- Read the contract input from a JSON file.
- Create an offer for the contract and send it to Alice.
- Process the offer on Alice's side and accept the contract.
- Handle the response from Bob and complete the contract signing.
- Monitor the contract maturity and process the settlement.

The example logs the progress of the DLC execution, including the contract state transitions and outcome calculations.
