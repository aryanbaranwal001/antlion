# antlion user manual

This manual covers every command, option and report. For installing antlion and setting up
a contract pair, see the [README](README.md).

1. [Commands](#commands)
   - [Build](#build)
   - [Compare](#compare)
   - [Help and version](#help-and-version)
2. [Reading the output](#reading-the-output)
3. [Reports](#reports)
   - [size](#size)
   - [sections](#sections)
   - [imports](#imports)
   - [interface](#interface)
   - [returns](#returns)
   - [cost](#cost)
   - [ledger](#ledger)
   - [opcodes](#opcodes)
   - [functions](#functions)
   - [layout](#layout)

## Commands

### Build

```
antlion --build <path to pair>
```

Builds both sides of a pair: the Rust side with `stellar contract build` and the Solidity
side with `solang compile --target soroban`.

The path is relative to the directory you run the command from, unless it is absolute. A
bare `flow` means `./flow`; antlion does not look anywhere else for it.

The output goes to `out/<name>/sdk.wasm` and `out/<name>/solang.wasm`, where `<name>` is
the last part of the path. `out/` is created in the directory you run the command from.

```
antlion --build contracts/scalar_id      ->  out/scalar_id/
antlion --build ../elsewhere/mypair      ->  out/mypair/
antlion --build /abs/path/to/mypair      ->  out/mypair/
```

### Compare

```
antlion --compare <name>... --report <report>... [--detail | --dump]
```

Compares the two builds of a pair. It takes the pair's **name**, not its path, and reads
`out/<name>/`, so build the pair first.

`--report` is required. Give one or more report names, or `all` for every report. You can
also give several pairs at once. Pairs and reports run in the order you write them.

```
antlion --compare scalar_id --report all
antlion --compare scalar_id scalar_math --report size returns
antlion --compare flow --report opcodes --dump
```

`--detail` and `--dump` only affect the `opcodes` report and must come last. See
[opcodes](#opcodes).

### Help and version

```
antlion --help
antlion --version
```

`--help` prints a short reference of every command, option and report. `--version` prints
the version you have.

## Reading the output

Every report shows the Rust build as `sdk` and the Solidity build as `solang`, side by
side. Where there is a `diff` column, it is always `sdk` minus `solang`: a positive number
means the Rust build has more, a negative number means the Solidity build has more.

```
sdk     solang  diff
2845    3734    -889
```

Here the Solidity build is 889 bytes larger.

A dash, `—`, in a column means that side has nothing to show there, for example a function
that exists on one side only.

## Reports

| Report | Answers |
|---|---|
| [`size`](#size) | How big is each module? |
| [`sections`](#sections) | Which part of the module holds the bytes? |
| [`imports`](#imports) | Which host functions does each side use? |
| [`interface`](#interface) | Do both sides declare the same functions? |
| [`returns`](#returns) | Do both sides give the same answers? |
| [`cost`](#cost) | What does one call cost to run? |
| [`ledger`](#ledger) | Do both sides store the same data under the same keys? |
| [`opcodes`](#opcodes) | What kind of code does each side produce? |
| [`functions`](#functions) | Which functions account for the size and complexity? |
| [`layout`](#layout) | What memory, tables and globals does each side declare? |

### size

The total size of each module in bytes.

### sections

Bytes per WebAssembly section, with each custom section named. Use it when `size` shows a
gap and you want to know whether it is code, data or metadata.

### imports

The Soroban host functions each side imports, as one sorted list with a mark for each side.

A function imported by one side only is a quick sign that the two builds do something
differently, before you read any code.

### interface

Every exported function with its argument and return types, read from the module's
`contractspecv0` section.

These are the Soroban types callers see. Inside the WebAssembly every argument and return
value is an `i64`, so a difference here is in what each side tells callers, not in the
code.

### returns

Calls every function on both builds and compares the answers. Each call deploys the module
into a fresh host, so every call starts with empty storage.

Arguments are generated from each function's declared types. Every function is called once
with each of four value classes:

| Class | Values |
|---|---|
| `small` | small enough to fit inside the 56 bit `Val` payload |
| `large` | too big for the payload, so the host passes them as object handles |
| `bound` | the largest value that fits in the payload, and each type's limits |
| `neg` | negatives, zeros and empty values |

A function that takes a struct is also called with four malformed versions of it, every
other argument at `small`:

| Shape | Sends |
|---|---|
| `extra` | the struct plus a field it does not declare |
| `short` | the struct without its last field |
| `wrong` | the first field as the wrong type |
| `asvec` | the right values as a vec instead of a map |

A function with any other argument also gets a `badarg` row: its first such argument sent
as the wrong type, `i32(-1)`, or `u32::MAX` where an `i32` is declared.

All of these rows run without authorization, so a function that calls `require_auth`
fails on them. A function that takes an address also gets an `authed` row: `small`
arguments, with every authorization the call asks for granted.

Each row ends with a verdict:

| Verdict | Means |
|---|---|
| `same` | both sides returned the same value |
| `DIFFER` | both sides returned, with different values; both are printed in full underneath |
| `sdk failed` | only the Rust build failed |
| `solang failed` | only the Solidity build failed |
| `both failed` | both builds failed |

For the malformed and `badarg` rows, `both failed` is the expected result, and `sdk failed`
means the Solidity build accepted what the Rust build rejected. When both fail with
different errors, both errors are printed in full, because one side may have accepted the
input and failed later for another reason.

The last line counts the calls that ran on both sides and how many of them disagreed.

### cost

The CPU instructions and memory bytes charged for one call of each function, on a fresh
host, with `small` arguments.

The module is put in the host's module cache before the call. The network keeps every live
contract parsed (CAP-0065, protocol 23), so a real call pays to set up the module but not
to parse it, and this report measures the same.

Every call has a large fixed cost, so compare the `diff` column, not the totals.

### ledger

Calls every function once on a fresh deploy, with `small` arguments and every authorization
granted, then lists everything each side has stored: storage class, key and value.
Instance storage is shown as one row per key.

Entries written when the contract was deployed, before the call, are included. So a side
that sets up storage in its constructor shows entries even for a function that only reads.

Each function ends with one of:

| Line | Means |
|---|---|
| `layout: same` | both sides stored exactly the same entries |
| `layout: differs` | the entries differ in class, key or value |
| `layout: not compared, a call failed` | one side's call failed, so there is nothing to compare |

Numbers carry their type, such as `1u32` or `0i64`, so values of different types never look
the same.

### opcodes

A count of the instructions in every function, grouped by kind. Imported host functions
have no code of their own, so they are not counted.

| Option | Shows |
|---|---|
| none | the count for each kind |
| `--detail` | each kind broken down into its instructions, plus any used by one side only |
| `--dump` | the full code of every function in two columns, instead of the counts |

Both options only work with `opcodes` and must come last.

In `--dump`, the two columns are independent: each side is listed top to bottom in its own
order, and the shorter one is padded. Nothing is lined up between them. Exported functions
come first, grouped by name, then internal functions. Instructions use their raw
WebAssembly names, such as `LocalGet` for `local.get`.

### functions

For each function: bytes, instructions, locals, how deeply its control flow nests, and how
many calls it makes.

Exported functions are matched by name. Internal functions have no names, and number N on
one side has nothing to do with number N on the other, so they are added up into one row,
with nesting depth kept as the largest.

Byte counts are only comparable within one contract. Adding a function to a contract can
change the existing rows, because the compiler moves shared code into helpers and merges
identical ones.

### layout

The linear memory pages, tables and globals each module declares.
