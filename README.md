# Gacha-Sol: Solana Gacha Game Smart Contract

Welcome to **Gacha-Sol**, a decentralized gacha game built on the Solana blockchain using Anchor. This smart contract implements a Gashapon-inspired game where users can buy pulls, and an authority handles reward distribution with confidential transfer (CT) mechanics for privacy. The project showcases Solana's capabilities, including token management and zero-ciphertext proof verification.

## Overview

- **Purpose**: A gacha game on Solana where users purchase pulls using a public token payment. The game authority verifies the reward vault balance using zero-ciphertext proofs to ensure it matches the expected amount for each pull. Rewards are then confidentially withdrawn from the vault and transferred to the user via a standard transfer after authority verification.
- **Key Features**:
  - Uses Solana's SPL Token 2022 with confidential transfer extensions.
  - Implements zero-ciphertext proofs for reward verification.
  - Authority-driven flow for pull opening and reward distribution.
  - Tested with Rust-based unit tests.

## Prerequisites

- **Rust**: Install via [rustup](https://rustup.rs/).
- **Solana CLI**: Install via [Solana Docs](https://docs.solana.com/cli/install-solana-cli-tools).
- **Anchor**: Install version 0.31.1 with `cargo install anchor-cli --version 0.31.1`.
- **Node.js**: Required for frontend integration (optional, for UI testing).
- **Git**: For cloning the repository.

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/pupplecat/gacha-sol.git
   cd gacha-sol
   ```

2. Install dependencies:

   ```bash
   cargo build-bpf
   anchor build
   ```

3. Configure Solana CLI for devnet:

   ```bash
   solana config set --url https://api.devnet.solana.com
   ```

## Usage

### Build the Contract

Compile the smart contract:

```bash
anchor build
```

### Deploy to Devnet

Deploy the contract to Solana devnet:

```bash
anchor deploy --provider.cluster devnet
```

Note the program ID from the output for use in the frontend or tests.

### Interact with the Contract

- Use the provided frontend (if integrated) or Anchor client to call instructions like `buy_pull`, `create_pull`, or `open_pull`.
- Refer to the [Solana Docs](https://docs.solana.com/) for client-side interaction examples.

## Testing

Run the Rust-based unit tests:

```bash
cargo test
```

- Tests are written using `solana_program_test` to simulate the Solana runtime.
- Coverage includes key instructions (e.g., `buy_pull`, `apply_pull_pending_balance`).

To test on devnet (skipping local validator):

```bash
anchor test --skip-local-validator
```

## Project Structure

- `Cargo.toml`, `Xargo.toml`: Configuration files for Rust and cross-compilation.
- `src/`: Core smart contract code.
  - `lib.rs`: Entry point with Anchor program setup.
  - `instructions/`: Instruction handlers, for example `buy_pull.rs`, `verify_pull.rs`
  - `state/`: Account structs and parameters. `pull.rs`, `game_config.rs`
- `tests/`: Test suite.
  - `instructions/`: Test cases for each instruction.
  - `test_utils/`: Testing utilities.

## Contributing

Contributions are welcome! Please fork the repository and submit pull requests. Ensure tests pass and add new tests for new features.

## License

[MIT License](LICENSE)

## Acknowledgments

- Built with [Anchor](https://www.anchor-lang.com/) v0.31.1.
- Leverages [Solana](https://solana.com/) and SPL Token 2022.
- Inspired by Gashapon mechanics and confidential transfer technology.
