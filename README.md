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

## How a contract pair must be arranged

A contract pair is a directory holding both implementations:

```
<pair>/
  sdk/
    Cargo.toml        a cdylib crate depending on soroban-sdk
    src/lib.rs
  solang/
    <pair>.sol        named after the directory
```

Three rules, and only three.

1. **The two subdirectories are named `sdk` and `solang`.**
2. **The solidity file is named after its directory.** `contracts/flow` holds
   `contracts/flow/solang/flow.sol`. The crate name in `sdk/Cargo.toml` and the contract
   name inside the `.sol` are both free, only the filename is fixed.
3. **Both sides export the same function names.** This is the one that matters. The reports
   pair rows by name, so a function present on one side only is reported as unpaired rather
   than compared.

Build output goes to `out/<pair>/sdk.wasm` and `out/<pair>/solang.wasm`, where `<pair>` is
the last component of the directory you passed.

A crate outside this repository's workspace needs its own `[profile.release]`, otherwise
it is built with rust defaults and the size comparison measures the build configuration
instead of the compilers. See [Build profile](#build-profile) below.

## Run

```
cargo run -- --build <contract>
cargo run -- --compare <contract>... --report <report>... [--detail | --dump]
cargo run -- --help
```

`--build` takes a path to the pair directory, relative to the working directory unless
absolute. There is no fallback: a bare `flow` means `./flow`, not `contracts/flow`.

| Argument | Builds from | Into |
|---|---|---|
| `contracts/flow` | `./contracts/flow` | `out/flow/` |
| `flow` | `./flow` | `out/flow/` |
| `../elsewhere/pairs/flow` | that path | `out/flow/` |
| `/abs/path/to/flow` | that path | `out/flow/` |

The bundled pairs under `contracts/` are reached exactly like any other tree, so the tool
is not tied to this repository.

`--compare` takes the name, and reads `out/<name>/`:

```
cargo run -- --build contracts/scalar_id
cargo run -- --compare scalar_id --report all
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
cargo run -- --build contracts/scalar_id
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
