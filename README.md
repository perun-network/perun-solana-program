<h1 align="center"><br>
    <a href="https://perun.network/"><img src=".assets/go-perun.png" alt="Perun" width="196"></a>
<br></h1>


# perun-solana-program
Perun State Channels on Solana with **native EVM compatibility**, enabling secure and efficient cross-chain applications. 

## Project structure
```
.
├── scripts/         # Shell scripts for local deployment
├── src/             # Main Rust source code for the Solana program
│   ├── instructions/# Instruction definitions and handlers
│   ├── state/       # Data structures and serialization logic
│   ├── tests/       # Rust unit and integration tests
│   ├── entrypoint.rs# Solana program entrypoint
│   ├── error.rs     # Custom error types
│   ├── lib.rs       # Library root, exports modules
│   └── processor.rs # Core instruction processing logic
```
## Dependencies
Install the [Solana SDK](https://solana.com/docs/intro/installation)

## Build
```
cargo build-sbf
```

## Test
```
cargo test-sbf
```

## Deployment on local validators
Requirement: `make`, `tmux` and `tmuxp`

```bash
cd scripts

chmod +x *.sh

make dev
```


## Security Disclaimer

The authors take no responsibility for any loss of digital assets or other damage caused by the use of this software.

## Copyright

Copyright 2025 PolyCrypt GmbH.  
Use of the source code is governed by the Apache 2.0 license that can be found in the [LICENSE file](LICENSE).
