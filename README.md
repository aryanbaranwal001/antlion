# antlion

Differential testing tool for [Hyperledger Solang](https://github.com/hyperledger-solang/solang):
build a contract two ways — from the Rust `soroban-sdk` and from Solidity via Solang,
then diff the resulting Soroban `.wasm` on various parameters - size, section bytes breakdown, interface, and
runtime cost.

## Prerequisites

- Rust with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- The Solang compiler on your `PATH`, named exactly `solang` (the harness invokes it
  as `solang`) — either an official
  [release](https://github.com/hyperledger-solang/solang/releases) or built from
  [source](https://github.com/hyperledger-solang/solang); if the binary has a
  different name (example like `solang-linux-x86-64`), rename or symlink
  it to `solang`.

## Layout

```
contracts/<name>/sdk/      Rust soroban-sdk contract
contracts/<name>/solang/   Solidity contract

out/<name>/{sdk,solang}.wasm   build output the harness compares
```

Bundled contracts: `adder`, `counter`.

## Run

Build both wasm variants for a contract:

```
cargo run -- --build <name>
```

Compare them:

```
cargo run -- --compare <name>
```

Run only some sections (default is all):

```
cargo run -- --compare <name> --sections size interface cost
```

Sections: `size`, `bytes`, `interface`, `cost` (or `all`).

### Example

```
cargo run -- --build adder
cargo run -- --compare adder
```
