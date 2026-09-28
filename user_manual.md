# antlion user manual

1. [Contract pair](#contract-pair)
2. [Build](#build)
3. [Compare](#compare)
4. [Reports](#reports)
   - [size](#size)
   - [sections](#sections)
   - [imports](#imports)
   - [interface](#interface)
   - [returns](#returns)
   - [cost](#cost)
   - [opcodes](#opcodes)
   - [functions](#functions)
   - [layout](#layout)

## Contract pair

A pair is one directory holding the same contract written twice.

```
<pair>/
  sdk/
    Cargo.toml        cdylib crate depending on soroban-sdk
    src/lib.rs
  solang/
    <pair>.sol
```

| | Rule |
|---|---|
| subdirectories | named `sdk` and `solang` |
| `.sol` filename | matches the directory exactly, `flow/` holds `flow.sol` |
| crate name, contract name | free |
| exported functions | identical on both sides |

The last one is what makes the comparison work. Reports pair rows by name, so a function
on one side only is reported as unpaired rather than compared.

The Rust crate needs the release profile at whichever `Cargo.toml` cargo treats as the
root. See README.

## Build

```
antlion --build <path to pair>
```

Takes a path, relative to the working directory unless absolute. A bare `flow` means
`./flow`.

```
antlion --build contracts/scalar_id      ->  out/scalar_id/
antlion --build ../elsewhere/mypair      ->  out/mypair/
antlion --build /abs/path/to/mypair      ->  out/mypair/
```

Output is `out/<last component>/sdk.wasm` and `out/<last component>/solang.wasm`,
relative to where you ran the command.

## Compare

```
antlion --compare <name>... --report <report>... [--detail | --dump]
```

Takes the **name**, not the path, and reads `out/<name>/`. Build first.

```
antlion --compare scalar_id --report all
antlion --compare scalar_id scalar_math --report size returns
```

`--report` is required. Pass `all` for every report. Several names and several reports may
be given at once, and each runs in the order written.

`diff` is always sdk minus solang. Positive means the Rust build has more.

## Reports

| | Answers |
|---|---|
| [`size`](#size) | how big is each module |
| [`sections`](#sections) | which part of the module holds the bytes |
| [`imports`](#imports) | which host functions each side needs |
| [`interface`](#interface) | do both sides declare the same signatures |
| [`returns`](#returns) | do both sides answer the same thing |
| [`cost`](#cost) | what does one call charge |
| [`opcodes`](#opcodes) | what kind of code does each side emit |
| [`functions`](#functions) | which function carries the size or complexity |
| [`layout`](#layout) | what memory, tables and globals are declared |

### size

Total module bytes, one line.

### sections

Bytes per WebAssembly section, with custom sections named individually. Use it when `size`
shows a gap and you want to know whether it is code, data or metadata.

### imports

The host functions each side pulls in, as a sorted union with a marker per side.

A function present on one side only is the strongest single signal in the tool. It means
one build cannot do something the other can, before any code is read.

### interface

Every exported signature, decoded from `contractspecv0`.

These are Soroban types. In the wasm itself every parameter and return is an `i64`, so a
mismatch here is a mismatch in what each side declares to callers, not in the wasm.

### returns

Invokes every function on both builds and compares the answers as `ScVal`s. Each module is
deployed into a fresh host, so every call starts from empty storage.

Each function runs four times:

| Class | Passes |
|---|---|
| `small` | values that fit inside the 56 bit payload and travel inline |
| `large` | values past the payload, which arrive as object handles |
| `bound` | the largest value that still fits inline, and type extremes |
| `neg` | negatives, zeros and empty sequences |

A row reading `DIFFER` prints both values in full underneath. A call that fails prints its
`HostError` in place of the value.

### cost

CPU instructions and memory bytes charged for one call, on a fresh host.

Absolute figures are dominated by virtual machine instantiation, so read the difference,
not the total.

### opcodes

Instruction histogram, bucketed by kind, over every function body in the module. Imported
host functions have no body and are never counted here.

| Flag | Shows |
|---|---|
| none | per bucket counts |
| `--detail` | each bucket broken into its opcodes, plus opcodes exclusive to one side |
| `--dump` | per function disassembly in two columns, replaces the histogram |

Both flags apply to `opcodes` alone and must come last.

In `--dump` the two columns are independent. Nothing is diffed or aligned; each side reads
top to bottom in program order, the shorter one padded. Exported functions come first,
grouped by name, then internal functions. Opcode names are the raw wasm variant names,
`LocalGet` rather than `local.get`.

### functions

Per function bytes, instructions, locals, control flow depth and call count.

Exported functions pair by name. Internal functions are anonymous and index N on one side
has nothing to do with index N on the other, so they collapse into one summed row, with
depth kept as the maximum.

Byte counts are only comparable within one contract. Adding a function changes the
existing rows, because shared paths get outlined and identical bodies merged.

### layout

Declared linear memory pages, tables and globals.
