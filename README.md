# antlion

Differential testing tool for [Hyperledger Solang](https://github.com/hyperledger-solang/solang).

Build the same contract twice, once from the Rust `soroban-sdk` and once from Solidity
through Solang, then compare the two WebAssembly modules and the two running contracts.

Findings are in [findings.md](findings.md). A written assessment is in
[report.md](report.md).

## Prerequisites

- Rust with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- [`stellar-cli`](https://github.com/stellar/stellar-cli) v25.2.0 or newer. soroban-sdk 28
  refuses a plain `cargo build`, because the spec it emits carries markers the build system
  has to remove before `contractspecv0` is correct. The harness therefore builds the Rust
  side with `stellar contract build`.
- The Solang compiler on your `PATH`, named exactly `solang`. Either an official
  [release](https://github.com/hyperledger-solang/solang/releases) or built from
  [source](https://github.com/hyperledger-solang/solang). If the downloaded binary has a
  different name, such as `solang-linux-x86-64`, rename or symlink it to `solang`.

Versions this was last run against: solang v0.3.5, soroban-sdk 28.0.0, stellar-cli 26.0.0,
rustc 1.92.0.

## Layout

```
contracts/<name>/sdk/      rust soroban-sdk contract
contracts/<name>/solang/   solidity contract

out/<name>/{sdk,solang}.wasm   build output the harness compares
```

Both sides must export the same function names. That is what lets the reports pair rows.

## Run

```
cargo run -- --build <name>
cargo run -- --compare <name>... --report <report>... [--detail | --dump]
cargo run -- --help
```

Reports:

| Report | What it compares |
|---|---|
| `size` | total module bytes |
| `sections` | bytes per section, custom sections named |
| `imports` | which host functions each side imports |
| `interface` | every exported signature, from `contractspecv0` |
| `returns` | what each side answers, as an `ScVal` |
| `cost` | CPU instructions and memory bytes for one call |
| `opcodes` | instruction histogram, with `--detail` and `--dump` |
| `functions` | per function bytes, instructions, locals, depth, calls |
| `layout` | declared memory pages, tables and globals |
| `all` | every report above, in that order |

Contracts: `baseline`, `scalar_id`, `scalar_math`, `scalar_store`, `storage`, `flow`,
`struct_store`, `struct_mem`, `vec_mem`, `location`.

## Example

```
cargo run -- --build scalar_id
cargo run -- --compare scalar_id --report returns
```

```
function        input  sdk                      solang                   verdict
i64_id          small  I64(1)                   I64(1)                   same
i64_id          large  I64(115292150460684697…  I64(0)                   DIFFER
                  sdk    I64(1152921504606846976)
                  solang I64(0)
```

## Build profile

The root `Cargo.toml` carries the standard Soroban release profile: `opt-level = "z"`,
`lto`, `strip`, `overflow-checks`. Without it the Rust side ships debug symbols, no link
time optimisation and wrapping arithmetic, none of which is what a contract deploys.

It is not cosmetic. It moved `scalar_id` on the Rust side from 5,102 bytes to 1,299, and
inverted the size conclusion. Any measurement taken without it describes the build
configuration rather than the compilers.
