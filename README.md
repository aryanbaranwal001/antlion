# antlion

Differential testing tool for [Hyperledger Solang](https://github.com/hyperledger-solang/solang).

Takes a contract written twice, once in Rust against `soroban-sdk` and once in Solidity,
compiles both to Soroban WebAssembly, and reports where the two artifacts differ in size,
structure, cost and behaviour.

See [user_manual](user_manual.md) for more information.

## Prerequisites

| Requirement | Minimum version |
|---|---|
| rustc | 1.92.0 |
| Rust target `wasm32v1-none` | |
| [stellar-cli](https://github.com/stellar/stellar-cli) | 28.1.0 |
| [solang](https://github.com/hyperledger-solang/solang) | 0.3.5 |

```
rustup target add wasm32v1-none
```

Note: Solang must be on `PATH` under that exact name. Release binaries download as `solang-linux-x86-64` or similar, so rename and/or symlink.

## Contract pair layout

```
<pair>/
  sdk/
    Cargo.toml        cdylib crate depending on soroban-sdk
    src/lib.rs
  solang/
    <pair>.sol
```

- New contracts must follow the above given layout.
- Subdirectories  named `sdk` and `solang`.
- The `.sol` filename must match the directory name. Only crate name is free.
- Both sides export functions must have similar names for similar semantics.

Note: A pair outside this workspace needs its own `[profile.release]`, see
[Build profile](#build-profile).

## Run

```
cargo run -- --build <contract>
cargo run -- --compare <name>... --report <report>... [--detail | --dump]
cargo run -- --help
cargo run -- --version
```

Or, if you are using cli

```
antlion --build <contract>
antlion --compare <name>... --report <report>... [--detail | --dump]
antlion --help
antlion --version
```

## Example

```
cargo run -- --build contracts/scalar_id
cargo run -- --compare scalar_id --report returns
```

Or

```
antlion --build contracts/scalar_id
antlion --compare scalar_id --report returns
```

## Build profile

`[profile.release]` must contain:

```toml
[profile.release]
opt-level = "z"
overflow-checks = true
debug = 0
strip = "symbols"
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = true
```
