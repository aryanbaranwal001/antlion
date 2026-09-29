# antlion

antlion is a differential testing tool for
[Hyperledger Solang](https://github.com/hyperledger-solang/solang) on Soroban.

You give it the same contract written twice, once in Rust with `soroban-sdk` and once in
Solidity. It builds both to Soroban WebAssembly and shows where the two differ: size,
return values, storage and runtime cost.

## Requirements

| Tool | Version tested |
|---|---|
| rustc | 1.92.0 |
| Rust target `wasm32v1-none` | |
| [stellar-cli](https://github.com/stellar/stellar-cli) | 28.1.0 |
| [solang](https://github.com/hyperledger-solang/solang) | `v0.3.5-88-ge6289eb7` |

Add the Rust target:

```
rustup target add wasm32v1-none
```

Note: `solang` must be on your `PATH` under that exact name. Release binaries download as
`solang-linux-x86-64` or similar, so rename or symlink it.

## Install

```
git clone https://github.com/aryanbaranwal001/antlion.git
cd antlion
cargo build --release
```

The binary is `target/release/antlion`. Copy it somewhere on your `PATH`, or run it from
the repository with `cargo run --`.

## Quick start

The repository comes with 17 contract pairs in `contracts/`. Build one:

```
antlion --build contracts/scalar_id
```

This writes both builds to `out/scalar_id/`. Then compare them:

```
antlion --compare scalar_id --report returns
```

`returns` calls every function on both builds and shows what each one answers. Use
`--report all` to run every report, or list the ones you want:

```
antlion --compare scalar_id --report size cost
```

To see every report and option:

```
antlion --help
```

Run `antlion --version` to check which version you have.

## Adding your own contract pair

A pair is a directory with this layout:

```
<pair>/
  sdk/
    Cargo.toml        cdylib crate depending on soroban-sdk
    src/lib.rs
  solang/
    <pair>.sol
```

- The two subdirectories must be named `sdk` and `solang`.
- The Solidity file must be named after the directory: `flow/solang/flow.sol`.
- Functions that do the same thing on both sides must have the same name, so antlion can
  match them up.
- The crate name and the Solidity contract name are up to you.

Build it by passing its path, and compare it by its name:

```
antlion --build path/to/<pair>
antlion --compare <pair> --report all
```

### Build profile

The Rust side must be built with this profile. Pairs inside this repository get it from the
workspace; add a new one's `sdk` path to `members` in the root `Cargo.toml`. A pair outside
the repository needs this in its own `Cargo.toml`:

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

## Learn more

- [User manual](user_manual.md): every report and option in detail
- `antlion --help`: a quick reference in the terminal
- [Report](report.md): what antlion found comparing Solang with the Rust SDK, with
  recommendations

## License

Apache License 2.0. See [LICENSE](LICENSE).
