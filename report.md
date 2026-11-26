# Solang Soroban Target

## Differential evaluation

Prepared as part of the LFDT mentorship on comparing Solang to the Stellar Rust SDK.

The results come from `antlion`, a differential harness. It builds a contract written twice
with the same semantics, once with the Rust `soroban-sdk` and once in Solidity through
Solang, and compares the two WebAssembly modules.

---

## Contents

1. [Summary](#summary)
2. [Setup](#setup)
3. [Methodology](#methodology)
4. [How the runtime cost breaks down](#how-the-runtime-cost-breaks-down)
5. [What was tested](#what-was-tested)
6. [Limitations](#limitations)
7. [Findings](#findings)
8. [State across a sequence of calls](#state-across-a-sequence-of-calls)
9. [What a failure says](#what-a-failure-says)
10. [Recommendations](#recommendations)
11. [Appendix A: Contract Inventory](#appendix-a-contract-inventory)
12. [Appendix B: Harness Output](#appendix-b-harness-output)

---

## Summary

### Overview

Solang compiles Solidity for Soroban, where the main toolchain is the Rust `soroban-sdk`.
The two translate the same program differently: a different storage layout, different
encoding and decoding of values, and different optimisation. Each contract here is written
twice, once in each language with the same semantics, and the evaluation compares:

- **Behaviour:** whether both builds return the same results for the same inputs.
- **Storage and encoding:** how each build lays out state and encodes values.
- **Cost:** how big each build is and what it costs to run.
- **Cost breakdown:** where each cost difference comes from.

Twenty two contract pairs were built, each with identical exported function names on both
sides, and compared by `antlion` on seventeen reports: module size, section breakdown, host
function imports, declared interface, the metadata sections, return values, metered runtime
cost split into instantiate and run, how cost grows with size, storage after a call, the
ledger entries and storage calls of a call, its trace of host calls, rent and time to live,
state across a sequence of calls, events, opcode histogram, per function complexity and
declared memory layout.

### Results

Twenty four findings. F1 to F13 are numbered from most to least important. F14 to F24 come
from the seven later reports and the five later pairs, and are numbered in the order they
were found.
Each carries a confidence level: high where the harness measures the claim directly, medium
where the harness supports the claim but does not settle it, and further measurement could
confirm or change it.

- **F1: A wrongly typed argument is silently accepted.** Soroban leaves argument checking
  to the contract. The Rust build rejects any argument whose type does not match its
  declaration. The Solidity build does not check `u32`, `i32`, `bool` or `address` arguments,
  including struct fields, and reads whatever arrives as the declared type: an `i32` of `-1`
  sent for a `u32` is used as `4294967295`, and an `address` parameter accepts an `i32`
  ([Table 1](#table-1-scalar_id-a-wrongly-typed-argument-to-every-function)). No
  error is raised, so a contract relying on its declared types for a bound or a check does
  not get it. Authorization is not bypassed this way: a non address that reaches
  `requireAuth` is rejected by the host ([Table 5](#table-5-auth-with-and-without-authorization)).

- **F2: State past the fifteenth storage slot fails.** Solang uses a variable's slot number
  as its storage key, reinterpreted as a Soroban `Val`, whose low bits are a type tag. Slots
  0 to 14 land on valid tags and work. From slot 15 the tag is invalid and every access
  fails with `Error(Value, InvalidInput)` (Tables [7](#table-7-slots-the-storage-entries-after-each-function) and [8](#table-8-slots-results-either-side-of-the-limit)). A struct takes one slot per leaf
  field and a fixed size array one per element, so fewer than fifteen variables can reach
  the limit ([Table 7](#table-7-slots-the-storage-entries-after-each-function)).
  The contract compiles and deploys without any warning, and the keys that do work
  include `Bool`, `Void` and `Error` values (Tables [6](#table-6-storage-the-storage-class-and-key-of-each-variable) and [7](#table-7-slots-the-storage-entries-after-each-function)).

- **F3: A `storage` reference local ignores the storage class.** A Solidity local that
  aliases a state variable, `T storage r = x`, always uses persistent storage, whatever the
  variable is declared as ([Table 11](#table-11-location-the-storage-class-passed-to-each-storage-call)). On an `instance` or `temporary` variable it fails on every call
  with `Error(Value, InvalidInput)`, for lists and structs, reads and writes alike
  ([Table 10](#table-10-location-each-alias-and-its-direct-twin)). Direct
  indexing works, and a `memory` copy works, including correctly discarding a write to the
  copy.

- **F4: Idiomatic code on each side writes to a different ledger namespace.** An
  unannotated Solidity state variable lands in persistent storage, while Rust written like
  the Soroban examples uses instance storage ([Table 6](#table-6-storage-the-storage-class-and-key-of-each-variable)). Both are documented behaviour, but porting a contract without
  noticing changes its storage lifetime and its rent.

- **F5: A struct is one host value in Rust and one per field in Solidity.** Stored, it is a
  `Map` in Rust and a `Vec` in Solidity under a different key ([Table 13](#table-13-struct_store-the-stored-struct)), so neither contract
  can read the other's struct. Solang also writes it one field at a time, reading and
  rewriting the stored struct for each field: 16 host calls against 2 to write a four field
  struct ([Table 36](#table-36-struct_store-every-call-each-function-makes)). At the boundary both use the same `Map`, so a Rust client can call a
  Solidity contract with a struct, but Solang makes 9 host calls against 2 to pass one
  through (Tables [2](#table-2-struct_id-malformed-struct-arguments) and [37](#table-37-struct_id-every-call-each-function-makes)). Either way the Rust build is 9,000 to 45,000 CPU instructions a
  call cheaper ([Table 35](#table-35-runtime-cost-selected-functions-from-every-pair)).

- **F6: A mapping is one ledger entry in Solidity and one per key in Rust.** The whole
  mapping is a single value on the Solidity side ([Table 18](#table-18-mapping-the-stored-mapping)), so every access loads and stores all of
  it, the mapping is bounded by the ledger entry size limit, and neither contract can read
  the other's mapping. On a one key mapping the Rust build is 14,000 to 31,000 CPU
  instructions a call cheaper ([Table 19](#table-19-mapping-cost-per-call)). The Rust keys
  here are in instance storage, so they are inside one ledger entry too, and at 64 keys the
  Rust build costs more per write (F17).

- **F7: The error path is inlined at every trap site.** An error reporting sequence is
  pasted at every trap site rather than called as a shared handler, about 38% of the
  instructions in a small module ([Table 20](#table-20-flow-one-trap-site-on-the-solidity-side)). It costs size only, with no effect on
  behaviour.

- **F8: A list is held in linear memory rather than by the host.** Solang keeps a `memory`
  list in its own linear memory; the Rust build keeps it on the host. For a one element
  list built inside a call, the Solidity build measured about 12,000 CPU instructions
  cheaper and used about 65,000 more memory bytes ([Table 24](#table-24-vec_mem-cost-per-call)). At the contract boundary Solang
  copies every element in and out, 26,000 to 36,000 more CPU instructions a call
  ([Table 26](#table-26-vec_id-per-function-metrics-and-cost)). Both are
  measured at small sizes only. At larger sizes each side costs more per element on the list
  it does not own (F17).

- **F9: A local struct is materialised rather than eliminated.** The Solidity build costs
  16,000 to 17,000 more CPU instructions a call for a struct the Rust build never creates ([Table 27](#table-27-struct_mem-per-function-metrics-memory-operations-and-cost)).
  Most of it is one page of linear memory that the Solidity build grows during the call (F16).

- **F10: 229 bytes of toolchain metadata ship in every build.** They record how the module
  was built, and none of it is read on chain. The Rust build carries 230 bytes of metadata of its own, which is why
  the size gap on an empty contract is only 36 bytes ([Table 29](#table-29-baseline-bytes-per-section-of-an-empty-contract)).

- **F11: Every aggregate state variable is written at deploy.** The Rust build writes
  nothing until a function does. Deployment pays one storage write per array, struct and
  mapping ([Table 30](#table-30-__constructor-on-the-solidity-side-per-pair)), and a `persistent` one pays rent from deploy rather than from first
  use.

- **F12: A nested loop is executed rather than reduced to a multiply.** The same source
  runs in linear time on the Rust build and quadratic time on the Solidity build. At runtime this shows only indirectly: at the largest
  input the Solidity build runs until its budget is exhausted while the Rust build traps at
  once ([Table 31](#table-31-flow-the-nested-loop)). `scaling` measures the curve: `n^1.9` on
  the Solidity side and `n` on the Rust side (F17). The loop here is the simplest possible case.

- **F13: The decode of an unused argument is removed.** Only a decode that cannot trap is
  removable, so this is the static evidence that the four types in F1 are not validated
  ([Table 32](#table-32-scalar_id-bytes-per-function-with-one-and-two-arguments)).

- **F14: Every Solidity failure reaches the caller as the same bare trap.** `require`,
  `revert`, `assert` and a custom error all fail with `Error(WasmVm, InvalidAction)`, with no
  code of the contract's own, and the spec declares no errors. The Rust build can fail with
  the code the contract declared, `Error(Contract, #1)`
  ([Table 38](#table-38-errors-each-failure-form)).

- **F15: An event loses its name and all but its first data field.** The Solidity build
  emits only the indexed fields as topics, with no name, and only the first data field as the
  data, and declares no events in its spec ([Table 43](#table-43-events-each-event-shape)).

- **F16: Most of a call's cost is making the module ready.** Instantiation is 81.5% to 100% of
  a call's CPU. The two modules' instantiate costs differ by -14,182 to +23,821 CPU
  instructions, so the sign of a close total can come from the module alone
  ([Table 47](#table-47-flow-cost-split-and-cost-types)).

- **F17: How cost grows with size.** Measured at sizes 0 to 64. A list costs per element on
  the side that does not own it, a Rust mapping in instance storage grows faster per key than
  the Solidity one, and F12's loop is quadratic on the Solidity side
  ([Table 49](#table-49-flow-cost-scaling)).

- **F18: The Solidity build makes more storage calls for the same entries.** The footprint
  is the same in 94 of 95 functions, but the Solidity build makes 204 storage calls against
  139, because it reads a stored mapping or struct before each write
  ([Table 52](#table-52-access-a-mapping-write-and-a-struct-write)).

- **F19: The two builds declare different protocols and different specs.** Protocol 28 on
  the Rust side and 23 on the Solidity side, no `contractmetav0` on the Solidity side, and
  no error enums or events in its spec ([Table 54](#table-54-meta-the-events-pair)).

- **F20: Two checked operations are not checked in the Solidity build.** A shift by 32 or
  more, and `type(int32).min / -1`, return a value where Solidity 0.8 defines another result
  or a revert ([Table 55](#table-55-numeric-the-edge-cases-that-differ)).

- **F21: 256 bit values, strings and bytes cross the boundary intact.** Every value comes
  back the same on both sides. `bytes32` is declared as `bytes` in the Solidity spec, and 256
  bit arithmetic is done in the contract rather than on the host
  ([Table 57](#table-57-scalar_math-256-bit-addition)).

- **F22: The two builds use the host in different ways for the same work.** The trace is the
  same in 52 of 124 functions. The Solidity build builds values one element at a time, reads
  before it writes, and uses raw slot words that the host reads as 14 different types as its
  keys ([Table 58](#table-58-trace-a-mapping-write)).

- **F23: The Solidity build costs more to fail.** In 55 of 65 calls that fail on both sides,
  with a median `run` cost of 4,654 CPU instructions against 954, and up to about 17,500 more
  for an `assert` ([Table 59](#table-59-cost-to-fail-errors-and-flow)).

- **F24: The Solidity build pays more rent at deploy and less per call.** Its deploy rent is
  1.01 to 3.16 times the Rust build's, and the Rust build pays more in 38 of 48 calls that pay
  rent ([Table 60](#table-60-ttl-the-ttl-pair)).

### Across all pairs

**No well formed value is computed incorrectly.** 496 calls ran on both builds, and every
one returned the same value on both ([Table 34](#table-34-return-value-agreement-every-pair) for the first seventeen pairs). Arithmetic, control flow,
storage round trips, 64, 128 and 256 bit values at the representation boundary, strings and
bytes, negatives, zeros, empty sequences, structs passed in and returned, mappings and
authorization all produce identical results. Calls where one side fails and the other
answers are not counted as disagreeing; F20 lists the three where the answer is wrong. The
same holds across a sequence of calls on one deploy: 266 steps, every value the same
([State across a sequence of calls](#state-across-a-sequence-of-calls)).

**On size**, the Rust build is smaller in all twenty two pairs. Across the first seventeen,
the gap is from 36 bytes on an empty contract to 5,029 on the largest, with the ratio
reaching 3.3x; F7 and F10 are two of the causes ([Table 33](#table-33-module-size-every-pair)).

**On runtime cost**, the Rust build is cheaper in eleven of the sixteen pairs of
[Table 35](#table-35-runtime-cost-selected-functions-from-every-pair), by 650 to 45,000 CPU
instructions a call. The Solidity build is cheaper in three, and two are mixed. Of the later
pairs, the Solidity build is also cheaper on `errors`, by about 11,000 to 13,000 CPU
instructions a call with about 65,000 more memory bytes on the Rust side
([Table 42](#table-42-errors-size-and-cost)), and on `events` (F15). In most of these the
difference comes from the module rather than the code (F16). See
[How the runtime cost breaks down](#how-the-runtime-cost-breaks-down) for why.

---

## Setup

| | |
|---|---|
| Solang | `v0.3.5-88-ge6289eb7`, main, 88 commits past the v0.3.5 release |
| soroban-sdk | 28.0.0 |
| soroban-env-host | 28.0.2 |
| stellar-cli | 28.1.0 |
| rustc | 1.92.0 |
| Rust target | `wasm32v1-none` |
| antlion | `v0.1.0-3-g6c03470` for F1 to F13 and Tables 1 to 37; `v0.1.1-66-g0ed5c67` for F14 to F24 and Tables 38 to 60 |

### What is compared

What is compared is what each toolchain ships: the final WebAssembly module. Both sides are built
the way they would be deployed, so everything each toolchain does counts.

For the Rust side that means `stellar contract build`, which runs wasm-opt and trims
`contractspecv0` to the entries the contract references. For the Solidity side it means
`solang compile --target soroban`, with no further flags.

The Rust side is built with the release profile a deployed Soroban contract uses:

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

Every size figure depends on it. Without it the Rust build would carry debug symbols and
skip link time optimisation, neither of which a deployed contract does. stellar-cli requires
`overflow-checks`.

The profile also makes Rust trap on overflow, as Solidity 0.8 always does, so overflow
behaves the same on both sides.

---

## Methodology

Each pair lives in `contracts/<name>/`: a Rust crate using `soroban-sdk` in `sdk/`, and a
Solidity file, `solang/<name>.sol`. Both export functions with the same names, so the
reports can match them.

Both are compiled to WebAssembly, into `out/<name>/sdk.wasm` and `out/<name>/solang.wasm`,
and seventeen reports compare them.

| Report | What it compares |
|---|---|
| `size` | total module bytes |
| `sections` | bytes per WebAssembly section, custom sections named |
| `imports` | the set of Soroban host functions each module imports |
| `interface` | every exported signature, decoded from `contractspecv0` |
| `meta` | the three custom sections that describe a contract, decoded |
| `returns` | the value each build answers, as an `ScVal` |
| `cost` | CPU instructions and memory bytes charged for one call, module cached, split into instantiate and run, and the cost to fail |
| `scaling` | how the cost of each function grows with its size, 1 to 64 |
| `ledger` | every storage entry after one call: class, key and value |
| `access` | the ledger entries each call reads and writes, their sizes, and every storage host call, run as the network runs it |
| `trace` | every host call each call makes, in order, with its arguments, lined up side by side |
| `ttl` | how long each entry lives, the rent of the deploy and of each call, and what each side does once its entries expire |
| `sequence` | every function called in order on one deploy, twice over |
| `events` | the events each function emits, and the diagnostics of a failure |
| `opcodes` | instruction histogram by kind, with optional disassembly that names the host function each call goes to |
| `functions` | per function bytes, instructions, locals, block depth, call count |
| `layout` | declared linear memory pages, tables and globals |

`returns`, `cost`, `scaling`, `ledger`, `access`, `trace`, `ttl`, `sequence` and `events`
run the contracts. Each call deploys the module into a fresh `soroban-env-host`, so no call
sees state left by another, except in `scaling` and `sequence`, which call one deploy in
turn. The other reports read the module bytes.

`cost` puts the module in the host's module cache before the measured call. The network
keeps every live contract parsed (CAP-0065, protocol 23) and charges parsing at upload and
restore, not per call, so measuring without the cache would add a charge the network does
not make.

### The reports that came later

F14 to F24 come from seven reports added after F1 to F13, and from changes to two of the
first ten.

**The `errors differ` verdict.** `returns` and `sequence` compare the error code each side
returns when a call fails on both sides, such as `Error(Contract, #1)`, which is what a
caller sees. `both failed` means the same code on both sides, and `errors differ` a
different code on each.

**The `cost` report, split.** Two costs belong to a module rather than to one function, and
are measured on their own. **Parse** is what parsing the module costs; the network charges
it once, at upload. **Instantiate** is what every call pays before its function runs:
making the cached module ready, with its linear memory. It is measured as a call to a
function the module does not export, which the host makes ready and then fails to find. For
each function, `run` is the call's CPU less the instantiate cost: what the function itself
used. With `--detail`, every block also lists what each `ContractCostType` charged. Every
call is measured three times, each on a fresh host, and the report says whether the cost
was the same each time.

**The cost to fail.** `cost` also measures a call that fails, since the host charges for the
work done up to the failure. A failed call at `small` shows its cost in its own block, and a
table at the end lists every call that fails on one side or both, at `small`, `large`,
`bound` and `neg`, with the `run` cost of each side.

**`scaling`.** Calls each function at every size from 0 to 64, in order, on one deploy per
side. Size `n` means every integer argument is `n` and every list has `n` elements. Since
the calls share a deploy, a function that writes one key per call finds `n` keys already
stored at size `n`. The report shows the cost at sizes 1 to 64 and the power of `n` that
fits the growth from 4 to 16 to 64.

**`access`.** Runs each function once on a fresh deploy the way the network runs a
transaction: first in recording mode, as a simulation does, which finds the call's
footprint (the ledger entries it may read and write) and its authorizations, then in
enforcing mode with that footprint. During the second run it counts every storage host
call, such as `get_contract_data`, by storage class. It shows how many entries the call
reads and writes, the bytes it writes, and the size of each entry written, before and after.
An address argument is the transaction's source account, so its authorization needs no
signature. For all 236 calls the result agrees with the `small` (or `authed`) row of
`returns`, value or error code.

**`meta`.** Decodes the three custom sections that describe a contract:
`contractenvmetav0`, the host protocol the build was made for; `contractmetav0`, its key and
value text; and `contractspecv0`, every entry by kind, where `interface` reads only the
functions and structs.

**`trace`.** Records every host call each call makes, in order, with its arguments, on the
same run as `access`. It counts the calls per host function, lines the two traces up by name
with a longest common subsequence, and marks each line as the same on both sides, the same
host function with other arguments, or one side only.

**`ttl`.** How long each ledger entry lives and the rent each side pays, in ledger bytes as
the host meters them: one byte kept live for one ledger, to which the rent fee is in
proportion. It measures the rent of the deploy, the rent of each call on a fresh deploy, and
the lifetime of every entry the call creates or extends. A lifetime run then moves the
ledger past each expiry in turn and calls every function again.

**`sequence`.** Deploys each side once, calls every function on that one deploy in spec
order with `small` arguments and every authorization granted, and then does the full round
again. At each step it compares the value returned and whether the call changed the
storage. The storage itself is not compared, because the two sides lay it out differently
by design (F5, F6).

**`events`.** Calls every function once on a fresh deploy, as `ledger` does, and lists the
events of the call on each side. Events the contract emits are compared by their topics and
data. The host's `log` and `error` diagnostics are shown, not compared: they are where a
failure says why it failed.

### Argument synthesis

Arguments are generated from each function's declared types, so a new contract needs no
harness change. Each function is called once with each of four value classes:

| Class | Values |
|---|---|
| `small` | small enough to fit inside the 56 bit `Val` payload |
| `large` | too big for the payload, so the host passes them as object handles |
| `bound` | the largest value that fits in the payload, and each type's limits |
| `neg` | negatives, zeros and empty values |

Object values are created in the host that runs the call, since a handle only means
something in the host that made it.

A struct argument is built from the Rust build's struct definition, field by field, and
encoded the way soroban-sdk encodes it: a `Map` from field name to value, with keys sorted.
Both builds get the same value, so the row shows whether Solang accepts what a Rust client
sends.

A function that takes a struct is also called with four malformed versions of it, every
other argument at `small`:

| Shape | What is sent |
|---|---|
| `extra` | a field the struct does not declare, added |
| `short` | the last declared field, left out |
| `wrong` | the first declared field, as `i32(-1)` (or `u32::MAX` where an `i32` is declared) |
| `asvec` | the right values in declaration order, as a `Vec` rather than a `Map` |

A function with any other argument also gets a `badarg` row: its first such argument is
sent as the wrong type, by the same rule as `wrong`.

For these rows, both builds rejecting the call is the expected result, and one accepting it
is a difference. When both fail with different errors, the report prints both, because one
side may have accepted the input and failed later for another reason.

All of these rows run without authorization, so any `require_auth` call fails. A function
that takes an address also gets an `authed` row: `small` arguments, with the host granting
every authorization the call asks for.

### Ledger inspection

The `ledger` report calls each function once on a fresh deploy, with every authorization
granted, then lists every stored entry on each side: storage class, key and value. Instance
storage is split into one row per key, and entries written at deploy are included. F2's
slot keys, F5, F6 and F11 come from it.

### Guarding against the optimiser

A measurement only counts if the code being measured survives compilation. Two rules keep
it from being optimised away.

First, every loop bound and field value comes from a function argument. A hardcoded value
would be folded into a constant, leaving nothing to measure.

Second, before a number is recorded, the reports are checked for signs that code was
removed: `locals: 0` where the source has locals, a block depth of 1 where it has a loop,
or a size close to an empty function. This caught two cases. In `struct_mem` the Rust build
replaces the struct with its fields (F9). In `flow` it replaces the nested loop with a
multiply (F12).

---

## How the runtime cost breaks down

This section explains where the cost differences come from. Figures are CPU instructions
for one call with the module cached, from the `cost` report, checked against the
`imports`, `functions` and `opcodes` reports.

**Most of a call is fixed overhead.** A single branch in `flow` costs about 207,000 CPU
instructions on the Rust build and 210,000 on the Solidity build, and the cheapest function
measured, `struct_mem`'s `build`, still costs about 200,000. Every call pays at least that,
so compare the difference between builds, not the totals. F16 measures this overhead on its
own: instantiating the module is 81.5% to 100% of a call's CPU, and the two modules'
instantiate costs differ by -14,182 to +23,821 CPU instructions.

**Where the largest differences appear:**

| What differs | Which side pays | Evidence |
|---|---|---|
| Host calls per struct field | Solidity: 9 host calls against 2 to pass one struct through; 16 against 2 to write a four field struct, reading and rewriting it once per field | F5, F6 |
| Copying a list through linear memory | Solidity: `vec_id`'s `id` makes 9 calls against none in Rust, copying each element in and out | F8 |
| Guest memory | Solidity keeps memory aggregates in linear memory, about 65,000 to 68,000 more memory bytes a call; Rust keeps them as host objects | F8, F9 |

**By what a function does**, the Rust build is cheaper by:

| What the function does | Rust cheaper by |
|---|---|
| identity, arithmetic, control flow | 650 to 3,800 |
| one storage write and read | 700 to 5,600 |
| a struct or a map, stored or passed | 9,000 to 45,000 |
| a struct built and discarded | 16,000 to 17,000 |
| a list passed in and returned | 26,000 to 36,000 |

The Solidity build is cheaper on `vec_mem` by about 12,000, on `struct_vec` by 2,000 to
6,000, and on `slots` by 6,000 to 27,000. `scalar_store` and `location` are mixed. The split
`cost` report shows which part of the call accounts for it. In `vec_mem` the Rust module
costs 23,821 more to instantiate, while the Solidity run costs 11,176 to 11,854 more, so the
lower Solidity total comes from its module (F16). All of this is at `n = 1` for list and
loop functions; F17 shows how each grows up to `n = 64`.

---

## What was tested

Twenty two contract pairs, 156 exported function pairs, and 496 calls compared on both
builds.

**Scalar types at the contract boundary.** `bool`, `u32`, `i32`, `u64`, `i64`, `u128`,
`i128`, `u256`, `i256` and `address` passed in and returned unchanged, at one and two
arguments, across all four value classes. Covered by `scalar_id`.

**Arithmetic.** One addition per numeric type, with `&&` for `bool`, over the same type
set. Covered by `scalar_math`.

**Storage round trips.** Each scalar type written to instance storage and read back.
Covered by `scalar_store`.

**Numeric edges.** One edge case per function, each with one argument and the other operand
fixed, so the four input classes reach the edge. Covered by `numeric`:

| Function | Body | Edge |
|---|---|---|
| `sub_one`, `mul_two` | `a - 1`, `a * 2` | underflow at 0, overflow past half the maximum |
| `div_by`, `rem_by` | `100 / a`, `100 % a` | division and remainder by zero |
| `neg`, `neg_div` | `-a`, `a / -1` | overflow at `i32::MIN` |
| `sdiv`, `smod` | `a / 2`, `a % 3` | rounding and sign of a negative |
| `wrap` | `a + 1` in `unchecked`, `wrapping_add` in Rust | wraparound at the maximum |
| `pow2` | `2 ** a` | overflow past 31 |
| `shl`, `shr` | `1 << a`, `max >> a` | a shift by 32 or more |
| `to_signed`, `narrow` | `int32(a)`, `uint32(a)` | the same bits read as signed, a narrowing |

**Strings and bytes.** `string`, `bytes` and `bytes32` passed in and returned, their
lengths, a byte read (`bytes_first`, which fails on an empty value) and a `bytes` built in
the call. Covered by `bytes_id`.

**Failures.** The four ways Solidity fails a call, each next to its nearest Rust form, all
failing when the argument is below 10. Covered by `errors`:

| Function | Solidity | Rust |
|---|---|---|
| `req` | `require(v >= 10, "too small")` | `panic_with_error!(&env, Error::TooSmall)` |
| `rev` | `revert("too small")` | `panic!("too small")` |
| `asrt` | `assert(v >= 10)` | `assert!(v >= 10)` |
| `custom` | `revert TooSmall(v)`, a custom error | returns `Err(Error::TooSmall)` |
| `div` | `1000 / (v % 2)` | `1000 / (v % 2)` |
| `set` | stores `v` | stores `v` |
| `set_fail` | stores `v + 100`, then `require` fails | stores `v + 100`, then `panic_with_error!` |
| `get` | reads the stored value | reads the stored value |

`set`, `set_fail` and `get` together test rollback: after a failed call, the next read
shows whether its write was undone.

**Events.** One function for each event shape, the same event declared on both sides. The
Rust side uses `#[contractevent]` with its defaults. Covered by `events`:

| Function | Event | Fields |
|---|---|---|
| `ping` | `Ping` | none |
| `value` | `Value` | `v` |
| `two` | `Two` | `v`, `w` |
| `keyed` | `Keyed` | `k` indexed (a `#[topic]` in Rust), `v` |
| `pair` | `Pair` | `from` and `k` indexed, `v`, `w` |
| `many` | `Value` three times | `v` |
| `emit_fail` | `Value`, then the call fails | `v` |

**Time to live.** A `put`, a `get` and a `bump` for each storage class. Each `bump` extends
its entry to its own TTL, 3000 ledgers for temporary, 6000 for persistent and 9000 for the
instance, so the entries expire one at a time. The values are `u64`, because Solang has
`extendTtl` only for a `uint64` variable: its source marks the other types as not done yet.
Covered by `ttl`.

**Storage durability classes.** The same `u32` written through `instance`, `persistent`
and `temporary`, plus a fourth function where each side uses its usual default. Covered by
`storage`.

**Control flow.** A branch, a `for` loop, the same loop written with `while`, a nested
loop, and a loop with `break` and `continue`, all with trip counts taken from the
argument. Covered by `flow`.

**Structs.** A struct built in the function body and discarded, covered by `struct_mem`.
A struct held in instance storage and accessed by one field and by all four, covered by
`struct_store`. A struct passed in, returned, and both, flat and nested, covered by
`struct_id`. The same struct written in each storage class, covered by `struct_class`. A
list of structs in storage, appended to and written one field at a time, covered by
`struct_vec`.

**Mappings.** A `u32` mapping and a struct mapping in instance storage, each written and
read back, and each read at a key nothing wrote. Covered by `mapping`.

**Lists.** A list built in the call, filled, summed and indexed, with length taken from
the argument, covered by `vec_mem`. A list passed in, returned, measured and summed,
covered by `vec_id`.

**Solidity data locations.** A `storage` local that aliases the ledger against a `memory`
local that copies out of it, with the stored value read back afterwards to show which
write survived. The alias is repeated on a list in each storage class, and on a struct
field for reading and for writing. Covered by `location`.

**Storage slot allocation.** A mapping, an array, a two field struct, a nested struct, a
fixed size array and seventeen `uint32`s, each written and read back by its own function,
to show which key each variable gets and where the keys stop working. Covered by `slots`.

**Authorization.** `require_auth` on an address argument, called without authorization and
with every authorization granted, plus a guarded write keyed by the address. Covered by
`auth`.

**Fixed module overhead.** An empty contract with no functions, to separate the cost of
being a contract from the cost of any feature. Covered by `baseline`.

---

## Limitations

These were not covered. They are gaps that need more work.

- **Only four values per type.** Each function is called with four value classes. No range
  is swept and no input is randomised, so other values may behave differently.

- **State is followed across one order of calls only.** The `ledger` report shows what a
  single call on a fresh deploy leaves behind. `sequence` follows state across one fixed
  order of calls with `small` inputs. Other orders and inputs are untested.

- **A failure inside a call to another contract is untested.** The four Solidity failure
  forms, rollback, the diagnostics a failure writes and the cost of a failure (F23) are
  compared, as are malformed structs and wrongly typed arguments.

- **Authorization is tested in recording mode only.** The host grants whatever is asked
  for. No signed authorization entries are built, nested and custom account authorization
  are not exercised, and `cost` runs without authorization, so the cost of a guarded call
  is not measured.

- **Time to live is tested for `u64` values only.** `ttl` measures the rent that follows a
  storage class and the lifetime of every entry (F24), but Solang has `extendTtl` only for
  a `uint64` variable, so only the `ttl` pair extends an entry.

- **Only the features in the pairs are tested.** Cross contract calls, modifiers,
  inheritance and every other Solidity feature not used by a pair are untested, and which
  features the Soroban target supports at all is not measured. `Symbol` and `Map`
  arguments are not tested, because Solidity on Soroban has neither type for a parameter.

- **Cost scaling is measured up to `n = 64` only.** `scaling` calls each function at every
  size from 0 to 64 with `small` elements and values (F17). Larger sizes, and a mapping in
  persistent storage, where each key would be its own ledger entry, are untested.

- **The first call after an upload is partly measured.** A contract called in the ledger it
  was uploaded in pays for parsing on that call. `cost` measures that parse on its own, per
  module (F16), and the cached case that every later call pays, but not the two together in
  one call.

- **The ledger entry size limit is not measured.** A Solidity mapping is one entry, and so
  is a Rust mapping in instance storage (F6, F17), so both are bounded by the maximum size
  of one entry. No pair grows a mapping far enough to reach it.

- **F2's slot counts cover only the variables in `slots`.** Mappings of structs, arrays of
  structs and structs holding arrays were not tested.

- **Some F3 functions have no runtime numbers.** Four of `location`'s alias functions fail
  on the Solidity side, so only their static numbers exist.

---

## Findings

Each finding states what happens, the evidence, why it matters and the recommendation.
[Appendix B](#appendix-b-harness-output) lists the command that reproduces each
one.

| # | Finding | Area | Confidence |
|---|---|---|---|
| F1 | Any value is accepted for a `u32`, `i32`, `bool` or `address` argument | behaviour | high |
| F2 | State past the fifteenth storage slot fails at runtime | storage | high |
| F3 | A `storage` reference local ignores the storage class | storage | high |
| F4 | Idiomatic code on each side writes to a different ledger namespace | storage | high |
| F5 | A struct is one host value in Rust and one per field in Solidity | encoding, cost | high |
| F6 | A mapping is one ledger entry in Solidity and one per key in Rust | storage, cost | high |
| F7 | The error path is inlined at every trap site | size | high |
| F8 | A list is held in linear memory rather than by the host | encoding, cost | high |
| F9 | A local struct is materialised rather than eliminated | cost | high |
| F10 | 229 bytes of toolchain metadata ship in every build | size | high |
| F11 | Every aggregate state variable is written at deploy | storage, cost | high |
| F12 | A nested loop is executed rather than reduced to a multiply | cost | high |
| F13 | The decode of an unused argument is removed | evidence for F1 | high |
| F14 | Every Solidity failure reaches the caller as the same bare trap | behaviour | high |
| F15 | An event loses its name and all but its first data field | behaviour, encoding | high |
| F16 | Most of a call's cost is making the module ready | cost | high |
| F17 | How cost grows with size | cost, storage | high for sizes up to 64 |
| F18 | The Solidity build makes more storage calls for the same entries | storage, cost | high |
| F19 | The two builds declare different protocols and different specs | interop | high |
| F20 | Two checked operations are not checked in the Solidity build | behaviour | high |
| F21 | 256 bit values, strings and bytes cross the boundary intact | encoding, cost | high |
| F22 | The two builds use the host in different ways for the same work | behaviour, cost | high |
| F23 | The Solidity build costs more to fail | cost, errors | high |
| F24 | The Solidity build pays more rent at deploy and less per call | cost, storage | high |

### F1: Any value is accepted for a `u32`, `i32`, `bool` or `address` argument

**Area:** behaviour. **Confidence:** high. **Contracts:** `contracts/scalar_id`, `contracts/struct_id`.

**What happens.** Soroban does not check call arguments against the contract
specification; the contract has to check what arrives. The Rust build checks the type tag
of every argument and traps on a mismatch. The Solidity build checks `u64`, `i64`, `u128`
and `i128`, and does not check `u32`, `i32`, `bool` or `address` at all. A value of the
wrong type is read as the declared type, or passed through unchanged for `address`.

**Evidence.** The `badarg` row of `returns` sends each function's first non struct
argument as the wrong type: `i32(-1)`, or `u32::MAX` where an `i32` is declared.
`scalar_id` ([Table 1](#table-1-scalar_id-a-wrongly-typed-argument-to-every-function)):

```
function      input   Rust    Solidity                     verdict
u32_id        badarg  [err]   U32(4294967295)              sdk failed
i32_id        badarg  [err]   I32(-1)                      sdk failed
bool_id       badarg  [err]   Bool(true)                   sdk failed
address_id    badarg  [err]   I32(-1)                      sdk failed
u64_id        badarg  [err]   [err]                        errors differ
                sdk     Error(WasmVm, InvalidAction)
                solang  Error(Value, InvalidInput)
i64_id, u128_id, i128_id: the same as u64_id
```

`returns` prints a failing call's error in full only where both sides fail with different
errors, the `errors differ` verdict; there the Rust build's error is
`Error(WasmVm, InvalidAction)`, a trap, and the Solidity build's is
`Error(Value, InvalidInput)`, raised by the host.

A struct field behaves the same way under the `wrong` shape: `get_x` answers
`U32(4294967295)` on the Solidity build and fails on the Rust build ([Table 2](#table-2-struct_id-malformed-struct-arguments)). Across all
pairs, 48 `badarg` rows ([Table 3](#table-3-badarg-rows-the-solidity-build-accepts-counted-per-pair)) and 2 `wrong` rows ([Table 2](#table-2-struct_id-malformed-struct-arguments)) show the Solidity build
accepting an argument the Rust build rejected. In `flow` and `vec_mem`, more rows fail on
both sides with different errors: the Solidity build accepts the value and fails later, by
running out of budget or linear memory ([Table 4](#table-4-flow-and-vec_mem-calls-that-fail-on-both-sides-with-different-errors)).

F13 shows the same from the disassembly: the decode of an unused argument of these four
types is removed, because it is a plain mask and shift with nothing that can trap
([Table 32](#table-32-scalar_id-bytes-per-function-with-one-and-two-arguments)).

The other malformed struct shapes behave the same on both builds ([Table 2](#table-2-struct_id-malformed-struct-arguments)). A struct with an extra
field is accepted and the field ignored; a struct missing a field, or sent as a `Vec`
rather than a `Map`, is rejected.

**Why it matters.** A caller picks the value, and the Solidity contract uses it as the
declared type with no error. A negative `i32` becomes a `u32` near four billion. An
`address` parameter accepts a value that is not an address, though the host rejects one
that reaches `requireAuth`, as `auth`'s `badarg` rows show ([Table 5](#table-5-auth-with-and-without-authorization)). A contract that relies on its
declared types for a bound or a check does not get it, and the call's result gives no sign
of the problem.

**Recommendation.** Check the type tag of every argument and struct field on entry, as is
done for `u64` and wider, and trap on a mismatch.

---

### F2: State past the fifteenth storage slot fails at runtime

**Area:** storage. **Confidence:** high. **Contracts:** `contracts/slots`.

**What happens.** Solang uses a state variable's slot number directly as its storage key,
read as a `Val`. The low 8 bits of a `Val` are its type tag, so slot `n` becomes a value
of whatever type tag `n` names. Tag 15 is not a valid type, so slot 15 cannot be a key,
and neither could any slot tested above it, up to 27.

**Evidence.** `slots` declares a mapping, an array, a two field struct, a nested struct, a
fixed size array, then seventeen `uint32`s, each with a function that writes it and reads it
back. Keys from the `ledger` report ([Table 7](#table-7-slots-the-storage-entries-after-each-function)):

```
variable              slots   key (slot)            result
m (mapping)               1   false (0)             works
xs (array)                1   true (1)              works
p (struct, 2 fields)      2   void (2)              works
o (Outer, Point + tag)    3   0u32 (4)              works
fx (uint32[4])            4   0i64 (7)              works
v05                       1   0i128 (11)            works
...
v08                       1   <empty symbol> (14)   works
v09                       1   slot 15               Error(Value, InvalidInput)
v10 to v21                1   slots 16 to 27        Error(Value, InvalidInput)
```

A struct takes one slot per leaf field, nested structs included, and a fixed size array
one per element, but each is stored whole under its first slot. `v09` is only the tenth
variable declared, and it fails. In the disassembly, the working `put_v08` and the failing
`put_v09` are the same 160 bytes and differ only in the key constant, `14` against `15`,
and two offsets into the error message data
([Table 9](#table-9-slots-put_v08-against-put_v09)). All four value classes fail on `v09` to `v21`: 52 failed calls ([Table 8](#table-8-slots-results-either-side-of-the-limit)).

**Why it matters.** Fifteen slots is an ordinary contract size, and fewer variables fill it
when they are structs or fixed size arrays. The contract compiles and deploys with no
warning and fails only on the variables past the limit, so a test that uses only the first
few passes. The keys that do work are unusual too: the state sits under `Bool`, `Void` and
`Error` values rather than numbers or names.

The workaround is to keep the state within fifteen slots. Grouping scalars into a struct
does not help, since each field takes a slot.

The `trace` report shows the slot keys at run time, in every storage call: the host reads
them as 14 different types (F22).

**Recommendation.** Encode the slot as a valid value, such as a `U32` or a `Symbol`, rather
than as a raw `Val` word. Until then, reject contracts that need more than fifteen slots
at compile time.

---

### F3: A `storage` reference local ignores the storage class

**Area:** storage. **Confidence:** high. **Contracts:** `contracts/location`.

**What happens.** Solidity lets a local alias a state variable rather than copy it. Writing
through the alias persists; writing through a `memory` copy does not.

```solidity
uint32[] instance xs;

function direct(uint32 v)      public returns (uint32) { xs[0] = v; return xs[0]; }
function via_storage(uint32 v) public returns (uint32) { uint32[] storage r = xs; r[0] = v; return xs[0]; }
function via_memory(uint32 v)  public returns (uint32) { uint32[] memory  c = xs; c[0] = v; return xs[0]; }
```

Each function seeds the list with `7` first, so the returned value says which write landed.

**Evidence.** From [Table 10](#table-10-location-each-alias-and-its-direct-twin):

```
function      input  sdk              solang                             verdict
direct        small  U32(1)           U32(1)                             same
via_memory    small  U32(7)           U32(7)                             same
via_storage   small  U32(1)           [err] Error(Value, InvalidInput)   solang failed
```

`direct` works. `via_memory` is correct: it returns the seeded `7`, because the write went
to a copy that was discarded. Only the alias fails, on all four value classes. The
semantics Solang implements are right; the alias path is what breaks.

In the disassembly, `direct` and `via_storage` differ in five constants: two offsets into
the error message data, and three storage class constants ([Table 11](#table-11-location-the-storage-class-passed-to-each-storage-call)): the
`StorageType` passed to `has_contract_data`, `get_contract_data` and `put_contract_data`.
It is `2` (`Instance`) in `direct` and `1` (`Persistent`) in `via_storage`. The alias drops
the variable's annotation and uses the default class. `location` repeats the test on a
list in each class ([Table 10](#table-10-location-each-alias-and-its-direct-twin)):

```
list held in   direct function   alias function      alias result
instance       direct            via_storage         Error(Value, InvalidInput)
persistent     direct_pers       via_storage_pers    works
temporary      direct_temp       via_storage_temp    Error(Value, InvalidInput)
```

Every `direct` function works. `via_storage_temp` differs from `direct_temp` the same way,
with `0` (`Temporary`) against `1` in the three storage class constants.

A struct alias fails the same way ([Table 10](#table-10-location-each-alias-and-its-direct-twin)). `struct_alias_write` writes a field through
`Point storage r` and `struct_alias_read` reads one through it; both fail on an `instance`
struct, while `struct_direct` works. The alias cannot write to the wrong variable by
mistake, because Solang numbers slots across all classes together (F2), so the persistent
key an alias falls back to never belongs to another variable.

**Why it matters.** `T storage r = x` is ordinary Solidity, used to avoid repeating a long
storage expression. On any `instance` or `temporary` variable it compiles and then fails
on every call. The failure is loud, so nothing wrong is stored. The workarounds are to
declare the variable `persistent` or not use the alias.

**Recommendation.** Pass the variable's storage class through the alias, as direct access
does.

---

### F4: Idiomatic code on each side writes to a different ledger namespace

**Area:** storage. **Confidence:** high. **Contracts:** `contracts/storage`.

**What happens.** Soroban has three storage classes. The Solidity annotation is optional,
and the Rust SDK has no default, so the usual code on each side ends up in different
classes.

| | Written as | Lands on |
|---|---|---|
| Solidity, no annotation | `uint32 v;` | persistent |
| soroban-sdk, as in its examples | `env.storage().instance()` | instance |

**Evidence.** From the compiled code. Solang passes `StorageType` as a plain number, `0`
temporary, `1` persistent, `2` instance, as the last argument of each storage call. The
class each function passes to `put_contract_data`, `has_contract_data` and
`get_contract_data`:

```
put_instance      Instance    Instance    Instance
put_persistent    Persistent  Persistent  Persistent
put_temporary     Temporary   Temporary   Temporary
put_idiomatic     Persistent  Persistent  Persistent
```

All three keywords work. `put_idiomatic` has no annotation and lands on persistent, while
the Rust side of the same function uses instance. The `ledger` report shows the same
classes directly ([Table 6](#table-6-storage-the-storage-class-and-key-of-each-variable)).

The classes also differ in cost ([Table 12](#table-12-storage-cost-per-call)):

```
             sdk cpu   solang cpu
instance      233345       234575
persistent    237550       238964
```

**Why it matters.** Porting a contract between the toolchains without noticing changes
both how its state is stored and what it pays in rent. Instance data lives inside the
contract instance entry and shares its lifetime; persistent data is a separate entry with
its own rent and eviction.

This is documented behaviour on both sides, not a defect, but it is an easy mistake when
moving between them.

**Recommendation.** Consider warning when a state variable has no storage annotation.

---

### F5: A struct is one host value in Rust and one per field in Solidity

**Area:** encoding, cost. **Confidence:** high. **Contracts:** `contracts/struct_store`, `contracts/struct_id`.

**What happens.** Both builds store a struct as one ledger entry, but not as the same value.
Read with the `ledger` report after `write_all(1)` ([Table 13](#table-13-struct_store-the-stored-struct)):

```
Rust       P       =>  {w: 1u32, x: 1u32, y: 1u32, z: 1u32}
Solidity   false   =>  [1u32, 1u32, 1u32, 1u32]
```

The Rust build packs the whole struct in one host call and stores it once. Solang reads and
rewrites the stored `Vec` once for every field it touches. The imports show the two
approaches ([Table 14](#table-14-struct_store-host-functions-imported)):

```
                              sdk   solang
map_new_from_linear_memory    yes    no
vec_new, vec_put, vec_get      no   yes
```

**Evidence.** Host calls when the call succeeds, for a four field struct. `--dump` names the
host function each call goes to ([Table 36](#table-36-struct_store-every-call-each-function-makes)):

```
function      Rust   Solidity   what Solang does
write_one        5          4   one storage read, one vec_put, one storage write
write_all        2         16   the same, once per field
read_one         3          3   one storage read, one vec_get
read_all         3         12   the same, once per field
```

A storage read is `has_contract_data` and `get_contract_data`; a write is
`put_contract_data`. The Rust build reads with those two plus
`sparse_map_unpack_to_linear_memory`, and writes with `map_new_from_linear_memory` plus
`put_contract_data`. Writing four fields costs the Rust build one storage write and the
Solidity build four reads and four writes of the same entry. For a single field Solang makes
one host call fewer, and still costs 9,357 more CPU instructions
([Table 35](#table-35-runtime-cost-selected-functions-from-every-pair)).

The `access` report counts these extra storage calls at run time: 12 against one `put` for
`write_all` (F18).

The `functions` report's `calls` column ([Table 15](#table-15-struct_store-per-function-metrics))
counts every call instruction instead: on the Rust side the calls into its helpers, on the
Solidity side the host calls above plus the calls that log an error.

A struct inside a stored list is encoded the same way: a `Vec` of `Map`s in Rust, a `Vec`
of `Vec`s in Solidity (`struct_vec`). The storage class annotation works for a struct in
every class (`struct_class`).

At the contract boundary, both sides encode a struct as the same `Map` keyed by field name,
and all 16 calls to `struct_id` with the four value classes agree, so a Rust client can
call a Solidity contract with a struct argument. Only the cost differs. Host calls when the call succeeds, for a two field
struct ([Table 37](#table-37-struct_id-every-call-each-function-makes)):

```
function   Rust   Solidity   what Solang does
get_x         1          4   symbol_new, map_get, per field
make          1          5   map_new, then symbol_new, map_put, per field
id            2          9   both of the above
```

Solang creates every key symbol at runtime, including one letter keys that fit a
`SymbolSmall`: it imports `symbol_new_from_linear_memory` ([Table 16](#table-16-struct_id-calls-per-function-and-host-functions-imported)). `id` costs 31,165 more CPU instructions on the Solidity build, and the nested
`id_outer` 45,414 more ([Table 17](#table-17-struct_id-cost-per-call)).

**Why it matters.** Cost, and compatibility of stored data. In storage the two builds use a
different key and a different encoding, so neither contract can read the other's storage.
At the boundary the encoding matches and only the cost differs.

**Recommendation.** None needed for correctness. Building a struct with
`map_new_from_linear_memory` and reading it with `sparse_map_unpack_to_linear_memory`, as the
Rust build does, would replace the per field calls with one, and reading and writing a
stored struct once per call, not once per field, would remove the repeated storage
round trips.

---

### F6: A mapping is one ledger entry in Solidity and one per key in Rust

**Area:** storage, cost. **Confidence:** high. **Contracts:** `contracts/mapping`.

**What happens.** The Rust build writes one entry per key, keyed by a `#[contracttype]` enum
variant, as the Soroban examples do. Solang writes the whole mapping as one `Map` under its
slot key (F2), with a struct value inside it as a `Vec` (F5). All 16
`mapping` calls agree.

**Evidence.** From the `ledger` report, after one `put(1, 1)` and after one `put_p(1, 1)`
([Table 18](#table-18-mapping-the-stored-mapping)):

```
function   side       key            value
put        Rust       [M, 1u32]      1u32
put        Solidity   false          {1u32: 1u32}
put        Solidity   true           {}
put_p      Rust       [Mp, 1u32]     {x: 1u32, y: 1u32}
put_p      Solidity   false          {}
put_p      Solidity   true           {1u32: [1u32, 1u32]}
```

The two empty maps are written by Solang's constructor at deploy (F11). On a one key
mapping the Rust build is cheaper per call, by 14,179 to 30,535 CPU instructions
([Table 19](#table-19-mapping-cost-per-call)).

**One per key holds for the keys, not for the ledger.** The Rust `mapping` stores each key
with `env.storage().instance()`, and instance storage is one map inside the contract
instance, a single ledger entry. So both builds keep this mapping in one ledger entry, and
both are bounded by its size limit. The Rust layout costs more per stored key: 4,504 CPU
instructions per key against 166 (F17).

**Why it matters.** The two layouts share no keys, so neither contract can read the other's
mapping. Since a Solidity mapping is a single value, every access loads and stores all of
it, and the whole mapping is bounded by the ledger entry size limit. At small sizes the
Rust build is cheaper on a mapping, and at 64 keys it is 262,901 dearer per `put` (F17).

**Recommendation.** None needed for correctness. Writing one entry per key, keyed by the
slot and the mapping key, would keep the cost of an access independent of the mapping's
size and remove the entry size limit.

---

### F7: The error path is inlined at every trap site

**Area:** size. **Confidence:** high. **Contracts:** every Solidity build.

**What happens.** In `flow`, the Rust build traps with a bare `unreachable` and imports
nothing at all. The Solidity build calls the host to log a message at each failure site, then traps
([Table 20](#table-20-flow-one-trap-site-on-the-solidity-side)).

```
I32Const 1408          ; message offset
I64ExtendI32U
I64Const 32
I64Shl
I64Const 4
I64Or
LocalTee 0
I64Const 219043332100  ; the same error constant, every time
LocalGet 0
I64Const 4
Call 0                 ; log_from_linear_memory
Drop
Unreachable
```

**Evidence.** Counted across the `flow` pair's `--dump` output, which is pure `u32`
arithmetic and touches nothing (Tables [20](#table-20-flow-one-trap-site-on-the-solidity-side) and [21](#table-21-flow-instruction-histogram)):

```
                                        sdk   solang
imports                                   0        1
trap sites                                8       15
host calls before a trap                  0       15
total instructions in the module        211      474
```

180 of the Solidity build's 474 instructions, about 38% of the module, are copies of that
block: 12 instructions at each of the 15 trap sites. It also accounts for the data section: 272 bytes of message strings on the
Solidity build of `scalar_id` against none on the Rust build ([Table 22](#table-22-scalar_id-bytes-per-section)).

**Why it matters.** Module size, and with it the cost of deploying the contract. It has no
effect on behaviour.

**Recommendation.** Emit one shared handler and call it. The change is local.

---

### F8: A list is held in linear memory rather than by the host

**Area:** encoding, cost. **Confidence:** high. **Contracts:** `contracts/vec_mem`, `contracts/vec_id`.

**What happens.** The two builds keep a list built inside a call in different places. Their
imports do not overlap ([Table 23](#table-23-vec_mem-host-functions-imported)):

```
import            sdk   solang
vec_new           yes    no
vec_push_back     yes    no
vec_get           yes    no
vec_len           yes    no
log                no   yes
```

The Solidity build imports no vector host functions. A `uint32[] memory` is allocated in the
module's own linear memory, and each element is a store. The Rust `Vec<u32>` is a handle to
an object held by the host, so the data never enters the module.

**Evidence.** Cost per call ([Table 24](#table-24-vec_mem-cost-per-call)):

```
         sdk cpu   solang cpu     sdk mem   solang mem
push      232895       220250     1125166     1189794
sum       234121       222154     1125166     1189794
first     233685       221242     1125166     1189794
```

At `n = 1`, the `small` input, the Solidity build is about 12,000 CPU instructions cheaper
and uses about 65,000 more memory bytes. The Rust build imports four vector host functions
and keeps the list on the host, the Solidity build imports none and does the work in its
own linear memory. The lower Solidity total comes from its cheaper module: the Rust module
costs 23,821 more to instantiate, and the Solidity run costs 11,176 to 11,854 more (F16).
The 65,000 memory bytes are one page of linear memory that the Solidity build grows during
the call (F16). At larger sizes the Rust build costs more per element, 2,279 to 3,125 CPU
instructions against 123 to 295, so at 64 elements it costs 148,260 to 190,246 more (F17).

The Solidity module is 3.3 times larger, the widest ratio of any pair. Most of the code
difference is in helper functions: 57 bytes on the Rust build against 910 on the Solidity
build. Most of the rest is the 622 byte data section, which holds the error messages of F7 ([Table 25](#table-25-vec_mem-size-and-per-function-metrics)).

A list at the contract boundary is copied the same way (`vec_id`). The Rust build returns
the handle it was given; the Solidity build reads every element into linear memory and builds
a new list to return ([Table 26](#table-26-vec_id-per-function-metrics-and-cost)):

```
function   Rust calls   Solidity calls   Rust cpu   Solidity cpu        diff
id                  0                9    227,014        263,115    -36,101
len                 1                6    227,665        254,016    -26,351
sum                 2               11    230,579        256,268    -25,689
```

These are measured with the `small` input, a list of three elements.

At the boundary it is the reverse: the Solidity build pays 893 CPU instructions per element
passed in, and 3,236 per element passed in and returned, while the Rust build pays the same
at every length (F17).

**Why it matters.** Cost, for every `memory` array a Solidity contract handles. Strings
and bytes are copied into linear memory the same way (F22).

**Recommendation.** None needed for correctness. Passing a list at the boundary through as a
host object, instead of copying it into linear memory and back, would remove the per
element copy. For a one element list built inside a call, linear memory measured cheaper.

---

### F9: A local struct is materialised rather than eliminated

**Area:** cost. **Confidence:** high. **Contracts:** `contracts/struct_mem`.

**What happens.** `build(a)` constructs `Point { x: a, y: a }` and returns `p.x + p.y`.

The Solidity build allocates and stores it ([Table 28](#table-28-struct_mem-the-body-of-build)):

```
I32Const 8
Call 1              ; an internal helper that allocates 8 bytes
I32Store            ; write y
I32Store            ; write x
```

The Rust build has no struct. After the tag check the whole body is:

```
LocalGet 0
I64Const 0
I64LtS
BrIf 1              ; trap if a + a would overflow u32
LocalGet 0
I64Const 1
I64Shl              ; a + a, as a shift
I64Const -8589934592
I64And
I64Const 4
I64Or               ; tag the result as a u32
```

rustc replaces the struct with its fields and then folds the addition. Across the module,
the Rust build has 0 linear memory operations and the Solidity build 17 ([Table 27](#table-27-struct_mem-per-function-metrics-memory-operations-and-cost)).

Taking the values from the argument does not stop this. The optimisation depends only on
whether the struct leaves the function, and a local struct read into a number never does.

**Evidence.** From [Table 27](#table-27-struct_mem-per-function-metrics-memory-operations-and-cost):

```
          sdk cpu   solang cpu      sdk mem   solang mem
build      199813       215835      1121424      1189083
nested     199861       216839      1121424      1189083
```

16,000 to 17,000 more CPU instructions and about 68,000 more memory bytes a call. The memory
bytes are one page of linear memory: the Solidity build declares 16 pages and grows its
memory by one page with `memory.grow` when it first allocates. That page costs 8,630 CPU
instructions of the call's `MemAlloc`, so `build` runs in 94 on the Rust side and 14,084 on
the Solidity side (F16).

**Why it matters.** Cost. A throwaway struct is free in Rust and not in Solidity.

**Recommendation.** None needed for correctness. Replacing a local struct that never
leaves the function with its fields, as rustc does, would remove the allocation.

---

### F10: 229 bytes of toolchain metadata ship in every build

**Area:** size. **Confidence:** high. **Contracts:** every Solidity build.

**What happens.** Section breakdown of an empty contract with no functions ([Table 29](#table-29-baseline-bytes-per-section-of-an-empty-contract)):

```
section                       sdk  solang
custom:producers                0     142
custom:target_features          0      44
custom:name                     0      43
custom:contractspecv0          15      55
custom:contractenvmetav0       30      30
custom:contractmetav0         230       0
export                         41      26
global                         25       9
everything else                 3      21
module and section headers     23      33
                             ----    ----
total                         367     403
```

`producers`, `target_features` and `name` record how the module was built: which compiler,
which LLVM features, and internal symbol names. That is 229 bytes, none of it read on
chain.

The Rust build carries 230 bytes of its own, in four `contractmetav0` entries: the Rust
compiler version, the SDK version and commit, an SDK spec setting and the CLI version and
commit (F19). That is why the gap on an empty contract is only 36 bytes.

**Why it matters.** 229 extra bytes in every Solidity contract, none of them used on chain,
and deployment cost grows with module size.

**Recommendation.** Strip `producers`, `target_features` and `name` in release builds, or
provide a flag that does.

---

### F11: Every aggregate state variable is written at deploy

**Area:** storage, cost. **Confidence:** high. **Contracts:** `contracts/struct_store`, `contracts/mapping`, `contracts/location`.

**What happens.** Solang's constructor writes an initial value for every array, struct and
mapping state variable. The Rust build writes nothing until a function does. Read with the
`ledger` report after a function that only reads (Tables [13](#table-13-struct_store-the-stored-struct) and [18](#table-18-mapping-the-stored-mapping)):

```
function               Rust                Solidity
struct_store read_one  (nothing stored)    false => [0u32, 0u32, 0u32, 0u32]
mapping get            (nothing stored)    false => {}
                                           true  => {}
```

Scalar state is not initialised ([Table 30](#table-30-__constructor-on-the-solidity-side-per-pair)):

```
pair           __constructor bytes   calls   initialises
scalar_store                     4       0   nothing
storage                          4       0   nothing
location                        88      10   three arrays, one struct
mapping                         38       4   two mappings
struct_store                    53       6   one four field struct
struct_vec                      21       2   one list of structs
struct_class                   103      12   one struct in each storage class
slots                          173      21   a mapping, an array, two structs, a fixed size array
auth                            21       2   one mapping
```

**Why it matters.** Deployment pays one storage write per array, struct and mapping, and a
`persistent` one exists, and pays rent, from deploy instead of from first use. Behaviour is
the same: an unset value reads as zeros on both sides.

**Recommendation.** None needed. Writing these on first use, as Solang does for scalars,
would remove the writes at deploy.

---

### F12: A nested loop is executed rather than reduced to a multiply

**Area:** cost. **Confidence:** high. **Contracts:** `contracts/flow`.

**What happens.** `nested(n)` is `for i in 0..n { for j in 0..n { t += 1 } }` on both sides.

The Rust build does not perform the accumulation. The returned value is computed directly
with one `I32Mul`, and what remains of the loop is an overflow guard that runs `n` times.
The Solidity build keeps both loops and executes all n squared iterations.

```
                   sdk   solang
loops in nested      1        2
I32Mul present     yes       no
```

**Evidence.** From [Table 31](#table-31-flow-the-nested-loop):

```
function     sdk   sol  diff │  locals  │ depth
nested        84   258  -174 │  3    5  │  3   7
```

`loop_` and `while_` compile to the same code within each build, so choosing `for` or
`while` makes no difference on either side.

**Why it matters.** The same source costs linear time on one side and quadratic on the
other. At `n = 1000` that is a thousand iterations against a million.

This should not be read too widely. The loop body is a constant increment, the simplest
case for this optimisation. It shows that rustc makes the reduction and Solang does not.

At runtime it shows at `n = u32::MAX` ([Table 31](#table-31-flow-the-nested-loop)). The Rust build overflows computing
`n * n` and traps at once. The Solidity build runs the loop until it fails with
`Error(Budget, ExceededLimit)`, so a call that fails on both sides uses the whole budget on
the Solidity side.

`scaling` measures the curve (F17): `nested` grows as `n^1.9` on the Solidity side and as `n`
on the Rust side, 6,188 against 72 CPU instructions for each step of `n`. At `n = 64` the
Solidity call costs 600,194 against 211,037.

**Recommendation.** Low priority. It is recorded because how cost grows matters to users,
and this is the only such difference seen.

---

### F13: The decode of an unused argument is removed

**Area:** evidence for F1. **Confidence:** high. **Contracts:** `contracts/scalar_id`.

**What happens.** The `_id2` functions take two arguments and return the first. The second
is decoded and thrown away. Bytes spent on that second argument, from [Table 32](#table-32-scalar_id-bytes-per-function-with-one-and-two-arguments):

| | sdk | solang |
|---|---|---|
| address | +13 | +0 |
| bool | +11 | +0 |
| u32, i32 | +12 | +0 |
| i64 | +19 | +21 |
| u64 | +19 | +32 |
| u128, i128 | +25 | +73 |

The Rust build pays every time. The Solidity build pays for the 64 and 128 bit types and
nothing else.

**Why it matters.** Not directly. It shows where validation happens. Unused code can only
be removed when it has no side effects: a decode that can trap must stay, and a mask and
shift that cannot trap is removed.

`u32`, `i32`, `bool` and `address` cost nothing here, so those four are not validated. F1
shows the effect at runtime.

**Recommendation.** None. This finding supports F1.

---

### F14: Every Solidity failure reaches the caller as the same bare trap

**Area:** behaviour. **Confidence:** high. **Contracts:** `errors`.

**What happens.** On the Solidity build, `require`, `revert`, `assert` and a custom error
all fail the call with `Error(WasmVm, InvalidAction)`: a WebAssembly trap, with no code of
the contract's own. Before the trap, each failure passes a fixed text to
`log_from_linear_memory` (F7), which only writes a diagnostic event and is not part of the
result. The text names the failure and its place in the source, not the message the code
gave. From `strings out/errors/solang.wasm`:

```
runtime_error:  require condition failed in errors.sol:20:26-37
runtime_error:  revert encountered in errors.sol:26:13-32
runtime_error: assert failure in errors.sol:32:16-23
runtime_error: TooSmall revert encountered in errors.sol:38:13-31
```

The string `"too small"` given to `require` and `revert` is not in the module at all. The
custom error survives only as the name `TooSmall` in that text, and its argument `v` is
lost.

The Rust build can fail either way. `panic!` and `assert!` trap with
`Error(WasmVm, InvalidAction)`, the same as Solidity. `panic_with_error!` and a returned
`Err` fail with `Error(Contract, #1)`, the code the contract declared.

**Evidence.** [Table 38](#table-38-errors-each-failure-form):

| Function | Rust | Solidity | Verdict |
|---|---|---|---|
| `req` | `Error(Contract, #1)` | `Error(WasmVm, InvalidAction)` | `errors differ` |
| `custom` | `Error(Contract, #1)` | `Error(WasmVm, InvalidAction)` | `errors differ` |
| `set_fail` | `Error(Contract, #1)` | `Error(WasmVm, InvalidAction)` | `errors differ` |
| `rev` | `Error(WasmVm, InvalidAction)` | `Error(WasmVm, InvalidAction)` | `both failed` |
| `asrt` | `Error(WasmVm, InvalidAction)` | `Error(WasmVm, InvalidAction)` | `both failed` |

The declared interface shows the same gap ([Table 39](#table-39-errors-the-declared-interface)).
The Rust spec declares `custom(u32) -> result<u32, error>` and lists the error type; the
Solidity spec declares `custom(u32) -> u32`, so a client built from the spec does not know
the function can fail with `TooSmall`.

Division by zero also fails differently: the Rust build checks the divisor and panics,
`Error(WasmVm, InvalidAction)`, while the Solidity build leaves it to the WebAssembly
division, which traps with `Error(WasmVm, ArithDomain)`.

**Why it matters.** A caller that handles failures, such as a client that retries on one
error and not another, or a contract that calls another with `try_call`, gets the same
trap for every reason on the Solidity side. A Solidity contract cannot report an error
that a caller can act on, and its spec does not say which errors exist.

**Recommendation.** Lower `revert` with a custom error, and `require`, to
`fail_with_error` with a contract error code, and put the custom errors in
`contractspecv0` as an error enum, as the Rust SDK does for `#[contracterror]`.

---

### F15: An event loses its name and all but its first data field

**Area:** behaviour, encoding. **Confidence:** high. **Contracts:** `events`.

**What happens.** The Rust build emits each event with its name as the first topic, then
its indexed fields, and all its data fields as one map. The Solidity build emits only the
indexed fields as topics, with no name, and only the first data field as the data. Every
data field after the first is not sent at all.

**Evidence.** [Table 43](#table-43-events-each-event-shape):

| Event | Rust topics | Rust data | Solidity topics | Solidity data |
|---|---|---|---|---|
| `Ping()` | `[ping]` | `{}` | `[]` | `false` |
| `Value(v)` | `[value]` | `{v: 1}` | `[]` | `1` |
| `Two(v, w)` | `[two]` | `{v: 1, w: 2}` | `[]` | `1` |
| `Keyed(indexed k, v)` | `[keyed, 1]` | `{v: 1}` | `[1]` | `1` |
| `Pair(indexed from, indexed k, v, w)` | `[pair, from, 1]` | `{v: 1, w: 2}` | `[from, 1]` | `1` |

The disassembly of `two` on the Solidity side shows the cause
([Table 44](#table-44-events-the-solidity-build-of-two)): `v + 1` is computed for its
overflow check, then dropped. `contract_event` is called with an empty topic list and `v`
alone. An event with no data field sends the raw word `0`, which the host reads as `false`.

The Rust spec declares each event, its topics and its fields; the Solidity spec declares
none, so a client built from the spec does not know the contract emits events.

**Why it matters.** Wallets, indexers and other off chain clients find events by their
topics. With no name in the topics, two Solidity events with the same indexed fields
cannot be told apart, and an event with no indexed field has no topics at all. The lost
data fields cannot be recovered by any client. A Solidity contract that emits an event the
way a Solidity programmer expects, such as a transfer with an amount and a memo, publishes
only part of it.

The Solidity build is 12,000 to 28,000 CPU instructions a call cheaper on these functions
([Table 46](#table-46-events-size-and-cost)). Of that, 13,114 is the module: the Rust module
costs more to instantiate (F16). What is left is the emit itself, 2,048 to 14,602 cheaper on
the Solidity side because it sends less: no name symbol and no data map. In `pair`, which
has two indexed fields, the Solidity run costs 503 more.

**Recommendation.** Put the event name in the first topic, send every data field, as a
`Vec` or as a `Map` by field name, and declare each event in `contractspecv0`, as the Rust
SDK does for `#[contractevent]`.

---

### F16: Most of a call's cost is making the module ready

**Area:** cost. **Confidence:** high. **Contracts:** every pair.

**What happens.** Before its function runs, every call pays to instantiate the cached
module: the host builds the instance and allocates its linear memory. Over the 203 calls
that `cost` measures, counting each build on its own, instantiate is 81.5% to 100% of the
CPU. The function itself uses 30 to 53,130 CPU instructions, against 199,719 to 341,165 for
the instantiation ([Table 47](#table-47-flow-cost-split-and-cost-types)).

```
                       sdk     solang
flow  instantiate   206800     208901
flow  early run        354       1330
```

**Why it matters.** The totals of a whole call, as in F4 to F9 and Table 35, compare modules
more than code. The two sides'
instantiate costs differ by -14,182 (`mapping`) to +23,821 (`vec_mem`) CPU instructions, and
that difference is in every call of the pair. Where a total is close, the sign of the
difference can come from the module alone:

- In `vec_mem`, the Solidity total is about 12,000 cheaper (F8), but its run costs 11,176 to
  11,854 more. The Rust module costs 23,821 more to instantiate
  ([Table 48](#table-48-vec_mem-and-struct_mem-cost-split)).
- In `errors`, the Solidity total is 11,000 to 13,000 cheaper. The Rust module costs 12,887
  more to instantiate, so the functions themselves cost about the same: `div` runs in 74
  against 398 ([Table 42](#table-42-errors-size-and-cost)).
- In `events`, 13,114 of each difference is instantiation (F15).

**The 65,000 memory bytes are one page of linear memory.** A page is 65,536 bytes. Eleven
Rust builds declare 17 pages against Solang's 16, so the Rust side pays for one more page at
every instantiation: 62,146 to 66,508 more memory bytes on every call of those pairs. In
`vec_mem` and `struct_mem` the Solidity build declares 16 pages and then grows its memory by
one page during the call, with `memory.grow`, when it first allocates. That page costs
8,630 CPU instructions of the call's `MemAlloc`. So in `struct_mem`, `build` runs in 94 on
the Rust side and 14,084 on the Solidity side (F9), and the memory differences in F8 and F9
are this page.

**Parse.** The Solidity module costs more to parse in 14 of 18 pairs, by up to 206,649 CPU
instructions (`vec_id`). The Rust module costs more in `scalar_store`, `mapping`, `events`
and `slots`, by up to 56,697. The network charges this once, at upload.

**Metering is stable.** Each of those 203 calls ran three times on a fresh host and cost the
same each time, by every cost type, on both builds.

**Recommendation.** Compare the `run` row when the question is the code, and the
instantiate cost when the question is the module. A smaller module is cheaper on every call,
not only at upload.

---

### F17: How cost grows with size

**Area:** cost, storage. **Confidence:** high for sizes up to 64. **Contracts:** `flow`,
`vec_id`, `vec_mem`, `mapping`, `slots`, `struct_vec`.

**What happens.** `scaling` called 94 functions at sizes 0 to 64. For 51 of them the cost
is flat on both sides. The rest, as the CPU added for each step of `n` from 1 to 64
([Tables 49 to 51](#table-49-flow-cost-scaling)):

| Pair | Function | What grows | Rust | Solidity |
|---|---|---|---|---|
| `flow` | `nested` | loop count | 72, `n^1.0` | 6,188, `n^1.9` |
| `flow` | `loop_`, `while_` | loop count | 64 | 92 |
| `vec_id` | `len`, `first` | list passed in | 0, flat | 893 |
| `vec_id` | `id` | list passed in and returned | 0, flat | 3,236 |
| `vec_id` | `sum` | list passed in and walked | 846 | 1,065 |
| `vec_mem` | `push`, `first` | list built in the call | 2,279 | 123 |
| `vec_mem` | `sum` | list built and walked | 3,125 | 295 |
| `mapping` | `put` | keys already stored | 4,504 | 166 |
| `mapping` | `put_p` | keys already stored | 9,300 | 2,810 |
| `slots` | `put_m` | keys already stored | 4,504 | 166 |
| `struct_vec` | `push` | list already stored | 4,872 | 2,720 |

Three results follow.

**F12 is measured.** `nested` grows as `n^1.9` on the Solidity side and as `n` on the Rust
side. At `n = 64` the Solidity call costs 600,194 against 211,037.

**A list costs per element on the side that does not own it.** A list passed in is a
handle on the Rust side, so `len`, `first` and `id` cost the same at every length. Solang
copies every element into its linear memory and back out, 893 CPU instructions per element
in and 3,236 in and out (F8). A list built inside the call is the reverse. The Rust build
makes a host call for each element, 2,279 to 3,125 per element, and Solang writes to its
own memory, 123 to 295. At 64 elements, `vec_mem` costs 148,260 to 190,246 more on the Rust
side.

**A Rust mapping in instance storage grows faster than the Solidity one.** The Rust
`mapping` stores each key with `env.storage().instance()`. Instance storage is one map
inside the contract instance, which is a single ledger entry. So both builds keep the
whole mapping in one ledger entry, and both are bounded by its size limit. Each call loads
that map, and the cost grows with the number of keys on both sides: 4,504 per key in Rust,
166 in Solidity. A one off count of the cost types of `put` at `n = 1` and `n = 64` puts the
Rust growth in `MemAlloc` (+201,600), `MemCpy` (+38,067), `VisitObject` (+37,620) and
`MemCmp` (+6,486): each stored key is a `[M, k]` vector, an object the host builds again
on every call. The Solidity map holds plain `u32` keys and values, which ride inline.

**Why it matters.** At small sizes the Rust build is cheaper on a mapping, at 64 keys it is
262,901 dearer per `put`. The Rust layout that F6 describes as one entry per key
is, in instance storage, one key per mapping key inside one entry.

This should not be read too widely. The sizes stop at 64, every element and value is
`small`, and persistent storage, where each key would be its own ledger entry, is not
tested here.

**Recommendation.** For the Rust side, which is the reference here: a mapping that grows
belongs in persistent storage, one entry per key, as the Soroban documentation advises for
unbounded data. For Solang: none from these numbers. Its mapping is cheaper per key than the
Rust instance layout up to the sizes measured.

---

### F18: The Solidity build makes more storage calls for the same entries

**Area:** storage, cost. **Confidence:** high. **Contracts:** `mapping`, `struct_store`,
`struct_vec`, `slots`, `location`, `auth`, `errors`.

**What happens.** For the 95 functions that succeed on both sides, the footprint is the
same in 94: both builds read and write the same number of ledger entries. The exception is
`storage`'s `put_idiomatic`, where the Rust build uses instance storage and the Solidity
build persistent storage, as F4 records. But of the 44 functions that touch
storage, the Solidity build makes more storage host calls in 24 and fewer in none: 204 calls
against 139 ([Table 52](#table-52-access-a-mapping-write-and-a-struct-write)).

```
                         Rust              Solidity
mapping put       get 1 has 1 put 1   get 2 has 2 put 1
struct_store      put 1               get 4 has 4 put 4
  write_all
location direct   get 2 has 2 put 2   get 4 has 4 put 2
```

Both builds read a value with a `has` and then a `get`. The extra calls on the Solidity
side come from two places:

- **A write reads first.** A mapping, or a struct, is one stored value on the Solidity side
  (F5, F6). To change one key or field, the build reads the whole value, changes it, and
  writes it back. `put` then reads it again to return `m[k]`, so it reads twice and writes
  once.
- **A struct is written one field at a time.** `write_all` sets four fields, and each one
  is a read and a write of the whole struct: 12 storage calls against one `put` on the Rust
  side. This is F5's static count, now seen at run time.

The bytes written differ by -52 to +188 per call: the Solidity build writes more in 12 of
the 39 functions that write and less in 24. In `slots` it writes 292 to 336 bytes where the
Rust build writes 148 to 232, because its instance entry holds every slot (F2). Each write
to instance storage rewrites the whole instance entry, on both sides, so the bytes written
are the full size of that entry.

No contract on either side deletes an entry or extends a TTL in any pair.

**Why it matters.** The fees that depend on the footprint (entries read and written) are
the same on both sides. The extra storage calls cost CPU instructions, which are part of the
`run` row of `cost` (F16), and the bytes written are charged per byte.

**Recommendation.** Read a stored mapping or struct once per call, change it in memory, and
write it once. Write all the fields of a struct with one write.

---

### F19: The two builds declare different protocols and different specs

**Area:** interop. **Confidence:** high. **Contracts:** every pair.

**What happens.** In all nineteen pairs, the Rust build declares protocol 28 in
`contractenvmetav0` and the Solidity build declares protocol 23, both with pre release 0
([Table 54](#table-54-meta-the-events-pair)). The host refuses a module whose protocol is
newer than the ledger's. So the Rust builds here run only on a network at protocol 28 or
later, and the Solidity builds on one at protocol 23 or later.

The Rust number comes from the SDK version used for this report, `soroban-sdk` 28, not from
the language: the SDK writes the protocol it was built for. Solang writes 23. Both run on
the protocol 28 host that antlion uses, so the comparisons in both reports are not
affected.

`contractmetav0` is the Rust build's alone. It holds four entries: the Rust compiler
version (`rsver`), the SDK version and commit (`rssdkver`), an SDK spec setting
(`rssdk_spec_shaking`) and the CLI version and commit (`cliver`). The Solidity build has no
such section, so nothing on chain says which Solang version built it.

In `contractspecv0`, the two builds declare the same structs in every pair, in a different
order in `struct_id`. They differ in two ways:

- **Error enums and events.** The Rust build declares the `Error` enum in `errors` and
  `events`, and the five events in `events`. The Solidity build declares neither (F14, F15).
- **The constructor.** The Solidity build declares `__constructor` in every pair, as the
  `interface` report shows. The Rust builds have no constructor.

**Why it matters.** A tool that reads the spec, such as a client generator, finds no error
codes and no events for a Solidity contract. A tool that reads `contractmetav0` to find out
how a contract was built finds nothing for a Solidity contract.

**Recommendation.** Write the Solang version into `contractmetav0`, as the Rust SDK writes
its own. Declare error enums and events in `contractspecv0` (F14, F15).

---

### F20: Two checked operations are not checked in the Solidity build

**Area:** behaviour. **Confidence:** high. **Contracts:** `contracts/numeric`.

**What happens.** Solidity 0.8 checks arithmetic: an operation whose result does not fit
reverts. In two cases the Solidity build returns a wrong value instead
([Table 55](#table-55-numeric-the-edge-cases-that-differ)):

```
function   input  Rust    Solidity           Solidity semantics
shl        large  [err]   U32(2147483648)    0
shr        large  [err]   U32(1)             0
neg_div    bound  [err]   I32(-2147483648)   revert
```

- **A shift by 32 or more.** In Solidity, `x << y` is `x * 2**y` cut to the type and
  `x >> y` is `x / 2**y`, so a shift by the width or more gives 0. The Solidity build uses
  the wasm `i32.shl`, which takes the shift amount modulo 32, with no check: `1 << u32::MAX`
  is `1 << 31`. The Rust build panics, because the release profile checks overflow.
- **`type(int32).min / -1`.** This is the one division that overflows, and Solidity 0.8
  reverts on it. Solang compiles `a / -1` to `0 - a` with a plain `i32.sub`, so the check
  is lost and `MIN` comes back. `neg`, which is written `-a`, keeps its check and fails on
  both sides.

Every other edge agrees: underflow, overflow in `mul` and `pow`, wraparound in `unchecked`,
rounding toward zero, the sign of a remainder, and both conversions. Division and remainder
by zero fail on both sides with different errors: `Error(WasmVm, InvalidAction)` in Rust,
which panics first, and `Error(WasmVm, ArithDomain)` in Solidity, the wasm trap of the
division itself (F14).

**Why it matters.** A contract that shifts by an amount the caller chooses, or divides by
a value that can be -1, gets a value that Solidity says it cannot get, and no error.

**Recommendation.** Give a shift by the width or more the result 0, as Solidity defines.
Keep the overflow check when `a / -1` is turned into a negation.

---

### F21: 256 bit values, strings and bytes cross the boundary intact

**Area:** encoding, cost. **Confidence:** high. **Contracts:** `scalar_id`, `scalar_math`,
`scalar_store`, `bytes_id`.

**What happens.** Every `u256`, `i256`, `string`, `bytes` and `bytes32` value comes back the
same on both sides, at every input class, and a stored `u256` or `i256` is encoded the same
way in both ledgers. Two things differ.

- **`bytes32` is declared as `bytes`.** The Rust spec declares `bytes32_id(bytes32) ->
  bytes32`, and the Solidity spec `bytes32_id(bytes) -> bytes`
  ([Table 56](#table-56-bytes_id-the-declared-interface)). The value on the wire is the
  same, and the Solidity build still rejects a `bytes` of another length. But a client
  that reads the spec sees a `bytes` of any length.
- **256 bit arithmetic is done in the contract.** The Rust build keeps a `U256` as a host
  object and calls the host's `u256_add` and `i256_add`. The Solidity build imports neither:
  it takes each value apart with four host calls, adds in wasm, and builds the result with
  one more ([Table 57](#table-57-scalar_math-256-bit-addition)). An add costs about the
  same on both sides, 6,819 against 7,816 CPU instructions for the function itself. An
  identity costs 1,350 against 7,202, because the Rust build passes the handle through and
  the Solidity build takes the value apart and builds it again.

An overflow fails on both sides, with `Error(Object, ArithDomain)` from the host in Rust and
`Error(WasmVm, InvalidAction)` from the contract's own check in Solidity.

**The new functions make the old ones dearer.** Adding the 256 bit rows grew the Solidity
`scalar_id` module by 1,260 bytes and its instantiate cost by 74,385 CPU instructions, so
every call of the old functions costs that much more. The Rust module grew by 421 bytes
and 9,580. The `run` row of every old function is exactly the same as before (F16).

**Recommendation.** Declare `bytesN` as `bytesN` in the spec.

---

### F22: The two builds use the host in different ways for the same work

**Area:** behaviour, cost. **Confidence:** high. **Contracts:** every pair.

**What happens.** For the 124 functions that succeed on both sides, the trace is the same
in 52: the same host calls, with the same arguments, in the same order. In the rest, the
Solidity build makes more host calls in 36 functions and fewer in 17: 570 calls against
482 in total.

**Which host functions.** The two builds build values in different ways:

| Host function | Rust | Solidity |
|---|---|---|
| `vec_new_from_linear_memory` | 45 | 0 |
| `map_new_from_linear_memory` | 18 | 0 |
| `sparse_map_unpack_to_linear_memory` | 15 | 0 |
| `vec_new` and `vec_push_back` | 21 | 49 |
| `map_get`, `map_put` and `map_has` | 0 | 30 |
| `get_contract_data` and `has_contract_data` | 100 | 161 |

The Rust build makes a whole vector or map from linear memory in one call. The Solidity
build makes one element at a time, and changes its stored mappings key by key with
`map_get`, `map_put` and `map_has` (F6, F18). Strings and bytes are copied into the
Solidity build's linear memory (`string_copy_to_linear_memory`,
`bytes_copy_to_linear_memory`); the Rust build reads them where they are, on the host.

**In which order.** Of the 41 functions that write storage on both sides, the Solidity
build reads first and then writes in 10 where the Rust build writes first. The reverse
never happens. In `mapping`'s `put`, for example, the Solidity build reads the whole map,
changes one key and writes it back, then reads it again for the return value
([Table 58](#table-58-trace-a-mapping-write)).

**With which arguments.** The storage class is the same in every storage call except
`storage`'s `put_idiomatic`, as already known. `require_auth` gets the same address on both
sides. The keys differ. The Rust build's are `Symbol` (103 calls) or `Vec` (42). The
Solidity build's are its raw slot words, which the host reads as 14 different types:
`False`, `True`, `Void`, `Error`, `U32`, `I32`, `U64`, `I64`, `I128`, `U256`, `I256`,
`Timepoint`, `Duration` and `Symbol`. This is F2 seen at run time: a slot number is not a
key of one type, but whatever value its bits happen to spell.

**Why it matters.** Each host call costs CPU instructions, so the extra calls are part of
the `run` row of `cost` (F16). A tool that reads a Solidity contract's storage must know
the slot layout, because the keys say nothing about what they hold.

**Recommendation.** As for F2 and F18: give each slot a key of one type, and read and write
a stored mapping once per call.

---

### F23: The Solidity build costs more to fail

**Area:** cost, errors. **Confidence:** high. **Contracts:** every pair with a failing call.

**What happens.** 65 calls fail on both sides, across all pairs and inputs. The Solidity
build costs more to fail in 55 of them and less in 10. The median `run` cost of a failure
is 954 CPU instructions on the Rust side and 4,654 on the Solidity side
([Table 59](#table-59-cost-to-fail-errors-and-flow)).

**A plain failure.** The Rust build panics and traps at once. The Solidity build first
writes a message and logs it (F7, F14), so it pays for that before the trap:

```
          Rust   Solidity
req         941      1,009
rev          78      1,009
asrt         78     17,559
custom       86     17,679
div          74        398
```

`assert` and a custom error cost the most. The Solidity build writes a 54 byte message in
its own memory (8,632 CPU of wasm instructions), grows its memory by one page for it
(about 8,600 CPU of `MemAlloc`, F16), and logs it with one host call. On the Rust side the
same failures cost 78 to 86.

**A failure that runs to the limit.** A few failures happen only when a resource runs out,
and there the two builds can be far apart:

```
                        Rust                       Solidity
flow nested   large     222       overflow trap    95,456,070   budget used up
vec_mem push  large     15,854,377  budget used up   507,028    out of linear memory
```

`nested` with `u32::MAX` traps on the Rust side as soon as `n * n` overflows, and runs the
loop on the Solidity side until the CPU budget is gone (F12). `vec_mem` is the reverse: the
Rust build grows a host vector until the budget is gone, and the Solidity build runs out of
linear memory first.

**A failure on one side only.** In `numeric`, the shifts and `neg_div` fail on the Rust
side in 86 to 110 CPU instructions and return a value on the Solidity side (F20).

**Why it matters.** The caller pays for a failed call. A Solidity contract fails at a
higher cost in most cases, by up to about 17,500 CPU instructions for an `assert`, and a
loop that the Rust build stops at once can run to the full budget.

**Recommendation.** Keep the message of a failure short, or behind a debug flag, as the
Rust release build does. Check for overflow before a loop where the bound is known, as the
Rust build does for `nested`.

---

### F24: The Solidity build pays more rent at deploy and less per call

**Area:** cost, storage. **Confidence:** high. **Contracts:** every pair, `contracts/ttl`.

**What happens.** Rent is charged when an entry is created, grows or is extended, in
proportion to its size and the ledgers it stays live. The two builds pay it at different
times.

- **At deploy the Solidity build pays more, in every pair.** Its rent is 1.01 to 3.16 times
  the Rust build's ([Table 60](#table-60-ttl-the-ttl-pair)). Most of it is the code entry,
  whose size for rent grows with the module, so the larger Solidity modules cost more to
  keep live (F16 for the same cause in CPU). In seven pairs the Solidity constructor also
  writes storage, which adds 98,280 to 769,860 ledger bytes.
- **Per call the Rust build pays more, in 38 of 48 calls that pay rent.** The Solidity
  constructor has already written those entries at deploy (F11), so the call does not grow
  them. The Rust build creates or grows the entry in the call itself. Eight calls pay the
  same. The Solidity build pays more in two: `storage`'s `put_idiomatic`, where it uses
  persistent storage and the Rust build instance storage, and `ttl`'s `bump_i`.
- **Extending the instance extends the code.** `bump_i` costs 262,976,670 ledger bytes on the
  Rust side and 382,158,360 on the Solidity side, because the code entry is extended with
  the instance and the Solidity code is larger.

**Lifetime.** Every new entry gets the same TTL on both sides: 15 ledgers for temporary and
4,095 for persistent, the network minimum. Both builds extend an entry only when the code
asks, with the same host calls and values (F22), and no pair other than `ttl` asks. When an
entry expires, the two builds behave the same: a temporary value is gone and reads give 0,
and a persistent value, or the instance and code, is restored when a call touches it, as a
transaction does since protocol 23. Across the lifetime runs of all twenty two pairs, every
answer agrees, except the calls that already fail on one side (F2, F3).

**Why it matters.** Over a contract's life the deploy rent, and every extension of the
instance, is the larger part: a call pays tens or hundreds of thousands of ledger bytes,
while a deploy or an instance extension pays hundreds of millions. A larger module costs
more each time it is kept live, not only at upload.

**Recommendation.** Support `extendTtl` for every storage type, not only `uint64`. A smaller
module also lowers the rent of the code entry (F10, F16).

---

## State across a sequence of calls

Over all twenty two pairs, 266 steps ran on both sides, and every value returned agreed
([Table 40](#table-40-sequence-every-pair)). So the result in the summary, that no well formed
value is computed incorrectly, still holds when calls build on earlier calls: values
written by one call are read back correctly by the next on both sides.

**A failed call is undone on both sides.** In `errors`, `set(1)` stores `1`, then
`set_fail(1)` stores `101` and fails. The next `get()` answers `1` on both builds, and
neither storage changed in the failed step ([Table 41](#table-41-errors-rollback-across-a-sequence)).

**One step disagrees, as a consequence of F3.** In `location`, step 11 `direct` returns the
same value on both sides, but only the Solidity storage changes. At step 5, `via_storage`
failed on the Solidity side only (F3), so its write was undone there and the two sides
held different lists from that point. `direct` then writes the same value the Rust side
already held. This is not a new defect; the report notes it whenever one side failed an
earlier step.

## What a failure says

The `events` report shows the host's diagnostics for each call. A diagnostic is recorded
only when diagnostics are switched on, as in a simulation or a test, and is not part of
the result a caller gets; on the network it is only for debugging.

Here the Solidity build tells more than the Rust build. In `emit_fail`, it logs
`runtime_error:  require condition failed in events.sol:51:26-37`, the failure and its place
in the source ([Table 45](#table-45-events-the-diagnostics-of-a-failed-call)). The Rust
build, built in release mode, keeps no message of its own; the host reports
`failing with contract error` and the code, `Error(Contract(1))`. For the failures of F2
and F3, neither side says more than the host: `Error(Value, InvalidInput)`.

An event emitted by a call that then fails is recorded on both sides with the call marked
as failed, so the network keeps neither.

---

## Recommendations

Each is backed by the finding it cites. [Appendix B](#appendix-b-harness-output)
gives the command that reproduces it.

### For Solang

1. **Check the type of every argument.** Under F1 a caller can pass a value of the wrong
   type into a `u32`, `i32`, `bool` or `address` parameter or struct field, and it is used
   without error. The wider integer types are checked.

2. **Use storage keys that are valid values.** F2 breaks every contract past fifteen
   storage slots. Encoding the slot as a `U32` or a `Symbol` instead of a raw `Val` word
   would remove the limit and make the keys readable.

3. **Keep the storage class in a `storage` reference local.** `T storage r = x` is
   ordinary Solidity and fails on every call unless the variable is `persistent` (F3).
   The alias passes `Persistent` instead of the variable's own class.

4. **Build and read a struct in one host call.** Solang reads and writes a struct one
   field at a time, creates every key symbol at runtime, and reads and rewrites a stored
   struct once per field. This is the largest cost difference measured, 9,000 to 45,000
   CPU instructions a call (F5). The Rust build uses `map_new_from_linear_memory` and
   `sparse_map_unpack_to_linear_memory` for the same work.

5. **Store a mapping as one entry per key.** A whole mapping in one ledger entry means
   every access loads and stores all of it, and bounds the mapping by the entry size limit
   (F6).

6. **Pass a list at the boundary through as a host object.** Copying it into linear memory
   and back costs 26,000 to 36,000 CPU instructions a call (F8).

7. **Emit one shared error handler and strip the build metadata sections.** F7 and F10
   both add size to every Solidity contract, and neither affects behaviour.

8. **Consider warning on a state variable with no storage annotation.** F4 is documented
   behaviour, not a defect, but it is an easy mistake when moving between the two
   toolchains.

9. **Fail with a contract error code.** Every Solidity failure reaches the caller as the
   same trap (F14). Lowering `require` and a `revert` with a custom error to
   `fail_with_error`, and declaring the custom errors in `contractspecv0` as an error enum,
   would let a caller act on the reason.

10. **Emit the event name and every data field, and declare events in the spec.** A
    Solidity event loses its name and all but its first data field, and no client can
    recover them (F15).

11. **Read and write a stored mapping or struct once per call.** Reading the whole value
    before each write, and writing a struct one field at a time, makes 204 storage calls
    against 139 for the same entries (F18).

12. **Write the Solang version into `contractmetav0`.** Nothing on chain says which Solang
    version built a contract, while the Rust SDK writes its own (F19).

13. **Give a shift by the width or more the result 0, and keep the overflow check of
    `a / -1`.** Both return a value that Solidity 0.8 says they cannot (F20).

14. **Declare `bytesN` as `bytesN` in the spec.** `bytes32` is declared as `bytes`, so a
    client sees a `bytes` of any length (F21).

15. **Support `extendTtl` for every storage type.** Only a `uint64` variable can be
    extended now (F24). A smaller module also lowers the rent of the code entry (F10, F16).

### For the Rust SDK

No finding calls for a change to the Rust SDK itself. F17 has one for Rust contracts: a
mapping that grows belongs in persistent storage, one entry per key, as the Soroban
documentation advises for unbounded data. In instance storage the whole mapping is one
ledger entry, and each write costs more as it grows.

---

## Appendix A: Contract Inventory

| Pair | Exported functions | What it isolates |
|---|---|---|
| `baseline` | 0 | fixed cost of being a contract |
| `scalar_id` | 20 | value decode and encode at the boundary |
| `scalar_math` | 9 | one addition per numeric type |
| `scalar_store` | 10 | one storage round trip per type |
| `storage` | 4 | the three durability classes, and the default |
| `flow` | 5 | branch, loop, while, nested loop, early exit |
| `struct_store` | 4 | a struct held in storage |
| `struct_mem` | 2 | a struct built in the call and discarded |
| `struct_id` | 4 | a struct passed in and returned |
| `mapping` | 4 | a `u32` mapping and a struct mapping |
| `vec_mem` | 3 | a list built in the call |
| `location` | 10 | Solidity `storage` and `memory` locals, in each storage class |
| `slots` | 22 | which storage key each state variable gets |
| `struct_class` | 3 | a struct in each storage class |
| `struct_vec` | 3 | a list of structs in storage |
| `vec_id` | 5 | a list passed in and returned |
| `auth` | 3 | `require_auth`, with and without authorization |
| `errors` | 8 | the four Solidity failure forms, division by zero, and rollback |
| `events` | 7 | one function for each event shape |
| `numeric` | 14 | one numeric edge case per function |
| `bytes_id` | 7 | `string`, `bytes` and `bytes32` at the boundary |
| `ttl` | 9 | a `put`, a `get` and a `bump` for each storage class |

---

## Appendix B: Harness Output

Each table is antlion's own output, cut to the rows the report uses; `...` marks rows left
out. Above each is the command that produces it. Where the command has
`<pair>`, it is run once per pair and the table gathers one row from each. Run
`antlion --build contracts/<name>` for each pair first.

Tables 1 to 37 come from antlion `v0.1.0-3-g6c03470`, and Tables 38 to 60 from
`v0.1.1-66-g0ed5c67`. The values and errors in Tables 1 to 37 are the same in the newer
version, with three differences:

- **Verdicts.** A call that fails on both sides with different errors is now marked
  `errors differ`, and only then are both errors printed. Tables 1 to 37 show the new
  verdict.
- **The `cost` text.** It has new rows, `instantiate` and `run`, and two module blocks at
  the top, so the cost tables here no longer match the output line for line.
- **The `scalar_*` cost tables.** With the 256 bit rows, every call in `scalar_id`,
  `scalar_math` and `scalar_store` costs more than these tables show, by the growth of the
  module's instantiate cost (F21). The `run` cost of each function is unchanged.

### Table 1: `scalar_id`, a wrongly typed argument to every function

```sh
antlion --compare scalar_id --report returns
```

```
function        input  sdk                      solang                   verdict
i32_id          badarg [err] HostError: Error…  I32(-1)                  sdk failed
i64_id          badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
u32_id          badarg [err] HostError: Error…  U32(4294967295)          sdk failed
u64_id          badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
bool_id         badarg [err] HostError: Error…  Bool(true)               sdk failed
i128_id         badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
i32_id2         badarg [err] HostError: Error…  I32(-1)                  sdk failed
i64_id2         badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
u128_id         badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
u32_id2         badarg [err] HostError: Error…  U32(4294967295)          sdk failed
u64_id2         badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
bool_id2        badarg [err] HostError: Error…  Bool(true)               sdk failed
i128_id2        badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
u128_id2        badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
address_id      badarg [err] HostError: Error…  I32(-1)                  sdk failed
address_id2     badarg [err] HostError: Error…  I32(-1)                  sdk failed
...
note: 66 call(s) ran on both sides, 0 disagreed
```

### Table 2: `struct_id`, malformed struct arguments

```sh
antlion --compare struct_id --report returns
```

```
function        input  sdk                      solang                   verdict
id              extra  Map(Some(ScMap(VecM([S…  Map(Some(ScMap(VecM([S…  same
id              short  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Object, MissingValue) DebugInfo not available
id              wrong  [err] HostError: Error…  Map(Some(ScMap(VecM([S…  sdk failed
id              asvec  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
make            badarg [err] HostError: Error…  Map(Some(ScMap(VecM([S…  sdk failed
get_x           extra  U32(1)                   U32(1)                   same
get_x           short  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Object, MissingValue) DebugInfo not available
get_x           wrong  [err] HostError: Error…  U32(4294967295)          sdk failed
get_x           asvec  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
id_outer        extra  Map(Some(ScMap(VecM([S…  Map(Some(ScMap(VecM([S…  same
id_outer        short  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Object, MissingValue) DebugInfo not available
id_outer        wrong  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
id_outer        asvec  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
note: 19 call(s) ran on both sides, 0 disagreed
```

### Table 3: `badarg` rows the Solidity build accepts, counted per pair

```sh
antlion --compare <pair> --report returns
```

```
pair            badarg rows ending in sdk failed
scalar_id         8
scalar_math       2
scalar_store      4
storage           4
struct_store      2
location          6
struct_id         1
mapping           4
slots             9
struct_class      3
struct_vec        3
vec_id            1
auth              1
total            48
```

### Table 4: `flow` and `vec_mem`, calls that fail on both sides with different errors

```sh
antlion --compare flow vec_mem --report returns
```

```
function        input  sdk                      solang                   verdict
nested          large  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
nested          badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
push            large  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, IndexBounds) DebugInfo not available
push            badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, IndexBounds) DebugInfo not available
```

### Table 5: `auth`, with and without authorization

```sh
antlion --compare auth --report returns
```

```
function        input  sdk                      solang                   verdict
no_auth         small  U32(1)                   U32(1)                   same
no_auth         large  U32(1)                   U32(1)                   same
no_auth         bound  U32(1)                   U32(1)                   same
no_auth         neg    U32(1)                   U32(1)                   same
no_auth         authed U32(1)                   U32(1)                   same
no_auth         badarg [err] HostError: Error…  U32(1)                   sdk failed
need_auth       small  [err] HostError: Error…  [err] HostError: Error…  both failed
need_auth       large  [err] HostError: Error…  [err] HostError: Error…  both failed
need_auth       bound  [err] HostError: Error…  [err] HostError: Error…  both failed
need_auth       neg    [err] HostError: Error…  [err] HostError: Error…  both failed
need_auth       authed U32(1)                   U32(1)                   same
need_auth       badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
auth_store      small  [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      large  [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      bound  [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      neg    [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      authed U32(1)                   U32(1)                   same
auth_store      badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
note: 7 call(s) ran on both sides, 0 disagreed
```

### Table 6: `storage`, the storage class and key of each variable

```sh
antlion --compare storage --report ledger
```

```
put_instance(u32) -> u32
side    class       key                    value
sdk     instance    INST                   1u32
solang  instance    false                  1u32
layout: differs

put_persistent(u32) -> u32
side    class       key                    value
sdk     persistent  PERS                   1u32
solang  persistent  true                   1u32
layout: differs

put_temporary(u32) -> u32
side    class       key                    value
sdk     temporary   TEMP                   1u32
solang  temporary   void                   1u32
layout: differs

put_idiomatic(u32) -> u32
side    class       key                    value
sdk     instance    IDIO                   1u32
solang  persistent  Error(Contract(0))     1u32
layout: differs
```

### Table 7: `slots`, the storage entries after each function

```sh
antlion --compare slots --report ledger
```

```
put_m(u32) -> u32
sdk     instance    [M, 1u32]              1u32
solang  instance    false                  {1u32: 1u32}
solang  instance    true                   []
solang  instance    void                   [0u32, 0u32]
solang  instance    0u32                   [[0u32, 0u32], 0u32]
solang  instance    0i64                   [0u32, 0u32, 0u32, 0u32]

put_xs(u32) -> u32
sdk     instance    [Xs]                   [1u32]
solang  instance    false                  {}
solang  instance    true                   [1u32]
solang  instance    void                   [0u32, 0u32]
solang  instance    0u32                   [[0u32, 0u32], 0u32]
solang  instance    0i64                   [0u32, 0u32, 0u32, 0u32]

put_p(u32) -> u32
sdk     instance    [P]                    {x: 1u32, y: 0u32}
solang  instance    false                  {}
solang  instance    true                   []
solang  instance    void                   [1u32, 0u32]
solang  instance    0u32                   [[0u32, 0u32], 0u32]
solang  instance    0i64                   [0u32, 0u32, 0u32, 0u32]

put_o(u32) -> u32
sdk     instance    [O]                    {inner: {x: 1u32, y: 0u32}, tag: 0u32}
solang  instance    false                  {}
solang  instance    true                   []
solang  instance    void                   [0u32, 0u32]
solang  instance    0u32                   [[1u32, 0u32], 0u32]
solang  instance    0i64                   [0u32, 0u32, 0u32, 0u32]

put_fx(u32) -> u32
sdk     instance    [Fx]                   [1u32, 0u32, 0u32, 0u32]
solang  instance    false                  {}
solang  instance    true                   []
solang  instance    void                   [0u32, 0u32]
solang  instance    0u32                   [[0u32, 0u32], 0u32]
solang  instance    0i64                   [1u32, 0u32, 0u32, 0u32]

put_v05(u32) -> u32
sdk     instance    [V, 5u32]              1u32
solang  instance    false                  {}
solang  instance    true                   []
solang  instance    void                   [0u32, 0u32]
solang  instance    0u32                   [[0u32, 0u32], 0u32]
solang  instance    0i64                   [0u32, 0u32, 0u32, 0u32]
solang  instance    0i128                  1u32

put_v08(u32) -> u32
sdk     instance    [V, 8u32]              1u32
solang  instance    false                  {}
solang  instance    true                   []
solang  instance    void                   [0u32, 0u32]
solang  instance    0u32                   [[0u32, 0u32], 0u32]
solang  instance    0i64                   [0u32, 0u32, 0u32, 0u32]
solang  instance    <empty symbol>         1u32

put_v09(u32) -> u32
sdk     instance    [V, 9u32]              1u32
solang  [err] HostError: Error(Value, InvalidInput) DebugInfo not available
```

### Table 8: `slots`, results either side of the limit

```sh
antlion --compare slots --report returns
```

```
function        input  sdk                      solang                   verdict
put_v08         small  U32(1)                   U32(1)                   same
put_v08         large  U32(4294967295)          U32(4294967295)          same
put_v08         bound  U32(4294967295)          U32(4294967295)          same
put_v08         neg    U32(0)                   U32(0)                   same
put_v08         badarg [err] HostError: Error…  U32(4294967295)          sdk failed
put_v09         small  U32(1)                   [err] HostError: Error…  solang failed
put_v09         large  U32(4294967295)          [err] HostError: Error…  solang failed
put_v09         bound  U32(4294967295)          [err] HostError: Error…  solang failed
put_v09         neg    U32(0)                   [err] HostError: Error…  solang failed
put_v09         badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
...
note: 36 call(s) ran on both sides, 0 disagreed
```

### Table 9: `slots`, `put_v08` against `put_v09`

```sh
antlion --compare slots --report functions opcodes --dump
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
put_v08           8   160  -152 │     4    66   -62 │    0    1   -1 │    0    3   -3 │    1    5   -4
put_v09           8   160  -152 │     4    66   -62 │    0    1   -1 │    0    3   -3 │    1    5   -4

Solidity side of --dump, put_v08:
I64Const { value: 14 }
Call 3 ; put_contract_data
I64Const { value: 14 }
Call 0 ; has_contract_data

Solidity side of --dump, put_v09:
I64Const { value: 15 }
Call 3 ; put_contract_data
I64Const { value: 15 }
Call 0 ; has_contract_data
```

### Table 10: `location`, each alias and its direct twin

```sh
antlion --compare location --report returns ledger
```

```
function        input  sdk                      solang                   verdict
direct          small  U32(1)                   U32(1)                   same
via_memory      small  U32(7)                   U32(7)                   same
direct_pers     small  U32(1)                   U32(1)                   same
direct_temp     small  U32(1)                   U32(1)                   same
via_storage     small  U32(1)                   [err] HostError: Error…  solang failed
struct_direct   small  U32(1)                   U32(1)                   same
via_storage_pers small  U32(1)                   U32(1)                   same
via_storage_temp small  U32(1)                   [err] HostError: Error…  solang failed
struct_alias_read small  U32(1)                   [err] HostError: Error…  solang failed
struct_alias_write small  U32(1)                   [err] HostError: Error…  solang failed
...
note: 24 call(s) ran on both sides, 0 disagreed

From --report ledger, the functions that fail:
via_storage(u32) -> u32
solang  [err] HostError: Error(Value, InvalidInput) DebugInfo not available
via_storage_temp(u32) -> u32
solang  [err] HostError: Error(Value, InvalidInput) DebugInfo not available
struct_alias_write(u32) -> u32
solang  [err] HostError: Error(Value, InvalidInput) DebugInfo not available
struct_alias_read(u32) -> u32
solang  [err] HostError: Error(Value, InvalidInput) DebugInfo not available
```

### Table 11: `location`, the storage class passed to each storage call

```sh
antlion --compare location --report opcodes --dump
```

```
Solidity side, direct:
I64Const { value: 2 }
Call 0 ; has_contract_data
I64Const { value: 2 }
Call 1 ; get_contract_data
I64Const { value: 2 }
Call 4 ; put_contract_data

Solidity side, via_storage:
I64Const { value: 1 }
Call 0 ; has_contract_data
I64Const { value: 1 }
Call 1 ; get_contract_data
I64Const { value: 1 }
Call 4 ; put_contract_data
```

### Table 12: `storage`, cost per call

```sh
antlion --compare storage --report cost
```

```
put_instance(u32) -> u32
metric              sdk       solang         diff
cpu insns        233345       234575        -1230
mem bytes       1125633      1127304        -1671
put_idiomatic(u32) -> u32
metric              sdk       solang         diff
cpu insns        233345       238981        -5636
mem bytes       1125633      1127862        -2229
put_temporary(u32) -> u32
metric              sdk       solang         diff
cpu insns        237550       238216         -666
mem bytes       1126191      1127862        -1671
put_persistent(u32) -> u32
metric              sdk       solang         diff
cpu insns        237550       238964        -1414
mem bytes       1126191      1127862        -1671
```

### Table 13: `struct_store`, the stored struct

```sh
antlion --compare struct_store --report ledger
```

```
write_all(u32)
side    class       key                    value
sdk     instance    P                      {w: 1u32, x: 1u32, y: 1u32, z: 1u32}
solang  instance    false                  [1u32, 1u32, 1u32, 1u32]
layout: differs

read_one() -> u32
side    class       key                    value
sdk     —           (nothing stored)
solang  instance    false                  [0u32, 0u32, 0u32, 0u32]
layout: differs
```

### Table 14: `struct_store`, host functions imported

```sh
antlion --compare struct_store --report imports
```

```
import   function                                         sdk   solang
l.0      has_contract_data                                  ✓        ✓
l.1      get_contract_data                                  ✓        ✓
l._      put_contract_data                                  ✓        ✓
m.9      map_new_from_linear_memory                         ✓        —
m.c      sparse_map_unpack_to_linear_memory                 ✓        —
v.0      vec_put                                            —        ✓
v.1      vec_get                                            —        ✓
v.6      vec_push_back                                      —        ✓
v._      vec_new                                            —        ✓
x._      log_from_linear_memory                             —        ✓
total                                                       5        8
```

### Table 15: `struct_store`, per function metrics

```sh
antlion --compare struct_store --report functions
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
read_all         94   434  -340 │    51   184  -133 │    3    6   -3 │    2    4   -2 │    1   16  -15
read_one         37   108   -71 │    18    44   -26 │    2    1   +1 │    0    1   -1 │    1    4   -3
write_all        73   318  -245 │    37   122   -85 │    2    2   +0 │    1    2   -1 │    1   17  -16
write_one        53   123   -70 │    28    50   -22 │    1    1   +0 │    1    2   -1 │    2    5   -3
__constructor     —    53     — │     —    15     — │    —    0    — │    —    0    — │    —    6    —
2│0 internal    394     0  +394 │   191     0  +191 │    8    0   +8 │    4    0   +4 │    5    0   +5
```

### Table 16: `struct_id`, calls per function and host functions imported

```sh
antlion --compare struct_id --report functions imports
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
get_x            62   243  -181 │    33    94   -61 │    1    2   -1 │    1    3   -2 │    1    8   -7
id               63   460  -397 │    32   179  -147 │    1    2   -1 │    1    6   -5 │    2   16  -14
id_outer        214   867  -653 │    98   333  -235 │    2    5   -3 │    2   11   -9 │    4   31  -27
make             29   308  -279 │    16   123  -107 │    1    3   -2 │    1    4   -3 │    1   10   -9
__constructor     —     4     — │     —     2     — │    —    0    — │    —    0    — │    —    0    —
6│1 internal    355   191  +164 │   174    82   +92 │    7    3   +4 │    2    2   +0 │    6    0   +6

import   function                                         sdk   solang
b.j      symbol_new_from_linear_memory                      —        ✓
m.0      map_put                                            —        ✓
m.1      map_get                                            —        ✓
m.9      map_new_from_linear_memory                         ✓        —
m._      map_new                                            —        ✓
m.c      sparse_map_unpack_to_linear_memory                 ✓        —
x._      log_from_linear_memory                             —        ✓
total                                                       2        5
```

### Table 17: `struct_id`, cost per call

```sh
antlion --compare struct_id --report cost
```

```
id(Point) -> Point
metric              sdk       solang         diff
cpu insns        234172       265337       -31165
mem bytes       1191041      1193557        -2516
make(u32) -> Point
metric              sdk       solang         diff
cpu insns        230817       258060       -27243
mem bytes       1190985      1193291        -2306
get_x(Point) -> u32
metric              sdk       solang         diff
cpu insns        229687       253465       -23778
mem bytes       1190809      1192899        -2090
id_outer(Outer) -> Outer
metric              sdk       solang         diff
cpu insns        239782       285196       -45414
mem bytes       1191329      1194493        -3164
```

### Table 18: `mapping`, the stored mapping

```sh
antlion --compare mapping --report ledger
```

```
put(u32, u32) -> u32
side    class       key                    value
sdk     instance    [M, 1u32]              1u32
solang  instance    false                  {1u32: 1u32}
solang  instance    true                   {}
layout: differs

put_p(u32, u32) -> u32
side    class       key                    value
sdk     instance    [Mp, 1u32]             {x: 1u32, y: 1u32}
solang  instance    false                  {}
solang  instance    true                   {1u32: [1u32, 1u32]}
layout: differs

get(u32) -> u32
side    class       key                    value
sdk     —           (nothing stored)
solang  instance    false                  {}
solang  instance    true                   {}
layout: differs
```

### Table 19: `mapping`, cost per call

```sh
antlion --compare mapping --report cost
```

```
get(u32) -> u32
metric              sdk       solang         diff
cpu insns        264219       279154       -14935
mem bytes       1194972      1133002       +61970
put(u32, u32) -> u32
metric              sdk       solang         diff
cpu insns        277493       293657       -16164
mem bytes       1196268      1134994       +61274
put_p(u32, u32) -> u32
metric              sdk       solang         diff
cpu insns        285902       316437       -30535
mem bytes       1196956      1201146        -4190
get_px(u32) -> u32
metric              sdk       solang         diff
cpu insns        265247       279426       -14179
mem bytes       1194972      1133002       +61970
```

### Table 20: `flow`, one trap site on the Solidity side

```sh
antlion --compare flow --report opcodes --dump
```

```
Solidity side, nested:
I32Const { value: 1408 }
I64ExtendI32U
I64Const { value: 32 }
I64Shl
I64Const { value: 4 }
I64Or
LocalTee { local_index: 0 }
I64Const { value: 219043332100 }
LocalGet { local_index: 0 }
I64Const { value: 4 }
Call 0 ; log_from_linear_memory
Drop
Unreachable
```

### Table 21: `flow`, instruction histogram

```sh
antlion --compare flow --report opcodes
```

```
kind                     sdk  solang    diff
control flow              58     106     -48
calls                      0      15     -15
local/global              60     111     -51
const                     36     121     -85
numeric                   56     106     -50
memory/other               1      15     -14
total                    211     474    -263
```

### Table 22: `scalar_id`, bytes per section

```sh
antlion --compare scalar_id --report sections
```

```
section                      sdk  solang    diff
type                          17      24      -7
import                        61      67      -6
function                      25      18      +7
table                          0       5      -5
memory                         3       3      +0
global                        33       9     +24
export                       207     192     +15
code                        1286    1666    -380
data                           0     272    -272
custom:contractenvmetav0      30      30      +0
custom:contractspecv0        919     959     -40
custom:name                    0     261    -261
custom:producers               0     142    -142
custom:target_features         0      44     -44
custom:contractmetav0        230       0    +230
  ├ (1)                      147       —       —
  └ (2)                       83       —       —
```

### Table 23: `vec_mem`, host functions imported

```sh
antlion --compare vec_mem --report imports
```

```
import   function                                         sdk   solang
v.1      vec_get                                            ✓        —
v.3      vec_len                                            ✓        —
v.6      vec_push_back                                      ✓        —
v._      vec_new                                            ✓        —
x._      log_from_linear_memory                             —        ✓
total                                                       4        1
```

### Table 24: `vec_mem`, cost per call

```sh
antlion --compare vec_mem --report cost
```

```
sum(u32) -> u32
metric              sdk       solang         diff
cpu insns        234121       222154       +11967
mem bytes       1125166      1189794       -64628
push(u32) -> u32
metric              sdk       solang         diff
cpu insns        232895       220250       +12645
mem bytes       1125166      1189794       -64628
first(u32) -> u32
metric              sdk       solang         diff
cpu insns        233685       221242       +12443
mem bytes       1125166      1189794       -64628
```

### Table 25: `vec_mem`, size and per function metrics

```sh
antlion --compare vec_mem --report size functions
```

```
sdk     solang  diff
869     2842    -1973

                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
first            72   192  -120 │    38    86   -48 │    0    2   -2 │    2    3   -1 │    3    5   -2
push             35    77   -42 │    19    35   -16 │    0    1   -1 │    1    1   +0 │    2    2   +0
sum             115   316  -201 │    65   144   -79 │    4    5   -1 │    4    6   -2 │    3    7   -4
__constructor     —     4     — │     —     2     — │    —    0    — │    —    0    — │    —    0    —
1│3 internal     57   910  -853 │    29   443  -414 │    3   12   -9 │    2    5   -3 │    2    5   -3
```

### Table 26: `vec_id`, per function metrics and cost

```sh
antlion --compare vec_id --report functions cost
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
first            99   372  -273 │    48   167  -119 │    2    6   -4 │    2    6   -4 │    2    9   -7
id               51   361  -310 │    25   163  -138 │    1    6   -5 │    1    6   -5 │    0    9   -9
len              61   255  -194 │    30   115   -85 │    1    6   -5 │    1    5   -4 │    1    6   -5
make            194   212   -18 │   106    97    +9 │    3    3   +0 │    5    4   +1 │    1    5   -4
sum             137   498  -361 │    75   225  -150 │    5    7   -2 │    4    8   -4 │    2   11   -9
__constructor     —     4     — │     —     2     — │    —    0    — │    —    0    — │    —    0    —
0│2 internal      0   710  -710 │     0   355  -355 │    0    7   -7 │    0    5   -5 │    0    0   +0

id(vec<u32>) -> vec<u32>
metric              sdk       solang         diff
cpu insns        227014       263115       -36101
mem bytes       1124466      1193797       -69331
len(vec<u32>) -> u32
metric              sdk       solang         diff
cpu insns        227665       254016       -26351
mem bytes       1124466      1193285       -68819
sum(vec<u32>) -> u32
metric              sdk       solang         diff
cpu insns        230579       256268       -25689
mem bytes       1124466      1193285       -68819
make(u32) -> vec<u32>
metric              sdk       solang         diff
cpu insns        229451       258375       -28924
mem bytes       1124586      1193773       -69187
first(vec<u32>) -> u32
metric              sdk       solang         diff
cpu insns        228455       255004       -26549
mem bytes       1124466      1193285       -68819
```

### Table 27: `struct_mem`, per function metrics, memory operations and cost

```sh
antlion --compare struct_mem --report functions opcodes cost
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
build            42   135   -93 │    24    61   -37 │    0    2   -2 │    2    2   +0 │    0    3   -3
nested           59   221  -162 │    36    98   -62 │    2    3   -1 │    2    3   -1 │    0    5   -5
__constructor     —     4     — │     —     2     — │    —    0    — │    —    0    — │    —    0    —
0│1 internal      0   191  -191 │     0    82   -82 │    0    3   -3 │    0    2   -2 │    0    0   +0

kind                     sdk  solang    diff
control flow              19      36     -17
calls                      0       8      -8
loads                      0       5      -5
stores                     0      12     -12
local/global              10      64     -54
const                     14      59     -45
numeric                   17      52     -35
memory/other               0       7      -7
total                     60     243    -183

build(u32) -> u32
metric              sdk       solang         diff
cpu insns        199813       215835       -16022
mem bytes       1121424      1189083       -67659
nested(u32) -> u32
metric              sdk       solang         diff
cpu insns        199861       216839       -16978
mem bytes       1121424      1189083       -67659
```

### Table 28: `struct_mem`, the body of `build`

```sh
antlion --compare struct_mem --report opcodes --dump
```

```
Rust side, build:
locals: none
Block { blockty: Empty }
LocalGet { local_index: 0 }
I64Const { value: 255 }
I64And
I64Const { value: 4 }
I64Eq
If { blockty: Empty }
LocalGet { local_index: 0 }
I64Const { value: 0 }
I64LtS
BrIf { relative_depth: 1 }
LocalGet { local_index: 0 }
I64Const { value: 1 }
I64Shl
I64Const { value: -8589934592 }
I64And
I64Const { value: 4 }
I64Or
Return
End
Unreachable
End
Unreachable
End

Solidity side, build:
locals: 2× i32
I32Const { value: 8 }
Call 1
LocalTee { local_index: 1 }
I32Const { value: 4 }
I32Add
LocalGet { local_index: 0 }
I64Const { value: 32 }
I64ShrU
I32WrapI64
LocalTee { local_index: 2 }
I32Store { memarg: MemArg { align: 2, max_align: 2, offset:…
LocalGet { local_index: 1 }
LocalGet { local_index: 2 }
I32Store { memarg: MemArg { align: 2, max_align: 2, offset:…
Block { blockty: Empty }
Block { blockty: Empty }
LocalGet { local_index: 2 }
LocalGet { local_index: 2 }
I32Add
LocalTee { local_index: 1 }
LocalGet { local_index: 2 }
I32LtU
BrIf { relative_depth: 0 }
I32Const { value: 0 }
I32Eqz
BrIf { relative_depth: 1 }
I32Const { value: 1216 }
I64ExtendI32U
I64Const { value: 32 }
I64Shl
I64Const { value: 4 }
I64Or
LocalTee { local_index: 0 }
I64Const { value: 244813135876 }
LocalGet { local_index: 0 }
I64Const { value: 4 }
Call 0 ; log_from_linear_memory
Drop
Unreachable
End
I32Const { value: 1024 }
I64ExtendI32U
I64Const { value: 32 }
I64Shl
I64Const { value: 4 }
I64Or
LocalTee { local_index: 0 }
I64Const { value: 244813135876 }
LocalGet { local_index: 0 }
I64Const { value: 4 }
Call 0 ; log_from_linear_memory
Drop
Unreachable
End
LocalGet { local_index: 1 }
I64ExtendI32U
I64Const { value: 32 }
I64Shl
I64Const { value: 4 }
I64Or
End
```

### Table 29: `baseline`, bytes per section of an empty contract

```sh
antlion --compare baseline --report sections
```

```
section                      sdk  solang    diff
type                           0       5      -5
function                       0       2      -2
table                          0       5      -5
memory                         3       3      +0
global                        25       9     +16
export                        41      26     +15
code                           0       6      -6
custom:contractenvmetav0      30      30      +0
custom:contractspecv0         15      55     -40
custom:name                    0      43     -43
custom:producers               0     142    -142
custom:target_features         0      44     -44
custom:contractmetav0        230       0    +230
  ├ (1)                      147       —       —
  └ (2)                       83       —       —
```

### Table 30: `__constructor` on the Solidity side, per pair

```sh
antlion --compare <pair> --report functions
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
scalar_store __constructor     —     4     — │     —     2     — │    —    0    — │    —    0    — │    —    0    —
storage __constructor     —     4     — │     —     2     — │    —    0    — │    —    0    — │    —    0    —
location __constructor     —    88     — │     —    26     — │    —    0    — │    —    0    — │    —   10    —
mapping __constructor     —    38     — │     —    12     — │    —    0    — │    —    0    — │    —    4    —
struct_store __constructor     —    53     — │     —    15     — │    —    0    — │    —    0    — │    —    6    —
struct_vec __constructor     —    21     — │     —     7     — │    —    0    — │    —    0    — │    —    2    —
struct_class __constructor     —   103     — │     —    29     — │    —    0    — │    —    0    — │    —   12    —
slots __constructor     —   173     — │     —    47     — │    —    0    — │    —    0    — │    —   21    —
auth __constructor     —    21     — │     —     7     — │    —    0    — │    —    0    — │    —    2    —
```

### Table 31: `flow`, the nested loop

```sh
antlion --compare flow --report functions returns
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
nested           84   258  -174 │    50   123   -73 │    3    5   -2 │    3    7   -4 │    0    4   -4

function        input  sdk                      solang                   verdict
nested          small  U32(1)                   U32(1)                   same
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
nested          large  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
nested          bound  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
nested          neg    U32(0)                   U32(0)                   same
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
nested          badarg [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
```

### Table 32: `scalar_id`, bytes per function, with one and two arguments

```sh
antlion --compare scalar_id --report functions
```

```
                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
address_id       18     4   +14 │    10     2    +8 │    0    0   +0 │    1    0   +1 │    0    0   +0
address_id2      31     4   +27 │    18     2   +16 │    0    0   +0 │    1    0   +1 │    0    0   +0
bool_id          41     8   +33 │    24     5   +19 │    1    0   +1 │    1    0   +1 │    0    0   +0
bool_id2         52     8   +44 │    32     5   +27 │    1    0   +1 │    1    0   +1 │    0    0   +0
i128_id          50   177  -127 │    25    82   -57 │    1    2   -1 │    1    2   -1 │    2    5   -3
i128_id2         75   250  -175 │    37   113   -76 │    2    2   +0 │    1    3   -2 │    3    8   -5
i32_id           24    56   -32 │    12    24   -12 │    0    0   +0 │    1    1   +0 │    0    1   -1
i32_id2          36    56   -20 │    20    24    -4 │    0    0   +0 │    1    1   +0 │    0    1   -1
i64_id           45   113   -68 │    23    53   -30 │    1    1   +0 │    1    2   -1 │    2    3   -1
i64_id2          64   134   -70 │    33    64   -31 │    1    0   +1 │    1    2   -1 │    3    4   -1
u128_id          50   174  -124 │    25    80   -55 │    1    1   +0 │    1    2   -1 │    2    5   -3
u128_id2         75   247  -172 │    37   111   -74 │    2    1   +1 │    1    3   -2 │    3    8   -5
u32_id           24    56   -32 │    12    24   -12 │    0    0   +0 │    1    1   +0 │    0    1   -1
u32_id2          36    56   -20 │    20    24    -4 │    0    0   +0 │    1    1   +0 │    0    1   -1
u64_id           45   131   -86 │    23    64   -41 │    1    1   +0 │    1    2   -1 │    2    3   -1
u64_id2          64   163   -99 │    33    80   -47 │    1    1   +0 │    1    2   -1 │    3    4   -1
__constructor     —     4     — │     —     2     — │    —    0    — │    —    0    — │    —    0    —
8│0 internal    531     0  +531 │   245     0  +245 │    8    0   +8 │    4    0   +4 │   10    0  +10
```

### Table 33: Module size, every pair

```sh
antlion --compare <pair> --report size
```

```
pair          sdk     solang  diff
baseline      367     403     -36
auth          1054    1423    -369
struct_mem    599     1473    -874
scalar_id     2845    3734    -889
scalar_math   2153    3066    -913
struct_store  1370    2287    -917
storage       1032    2000    -968
scalar_store  2304    3379    -1075
struct_class  1062    2158    -1096
struct_vec    1346    2479    -1133
mapping       1837    3026    -1189
struct_id     1784    3347    -1563
flow          1036    2723    -1687
vec_mem       869     2842    -1973
vec_id        1252    3892    -2640
location      2710    6794    -4084
slots         4207    9236    -5029
```

One row per pair. The Rust build is smaller in all seventeen pairs.

### Table 34: Return value agreement, every pair

```sh
antlion --compare <pair> --report returns
```

```
scalar_id     note: 66 call(s) ran on both sides, 0 disagreed
scalar_store  note: 33 call(s) ran on both sides, 0 disagreed
scalar_math   note: 24 call(s) ran on both sides, 0 disagreed
storage       note: 16 call(s) ran on both sides, 0 disagreed
struct_store  note: 16 call(s) ran on both sides, 0 disagreed
struct_id     note: 19 call(s) ran on both sides, 0 disagreed
mapping       note: 16 call(s) ran on both sides, 0 disagreed
flow          note: 10 call(s) ran on both sides, 0 disagreed
location      note: 24 call(s) ran on both sides, 0 disagreed
vec_mem       note: 6 call(s) ran on both sides, 0 disagreed
struct_mem    note: 4 call(s) ran on both sides, 0 disagreed
slots         note: 36 call(s) ran on both sides, 0 disagreed
struct_class  note: 12 call(s) ran on both sides, 0 disagreed
struct_vec    note: 12 call(s) ran on both sides, 0 disagreed
vec_id        note: 19 call(s) ran on both sides, 0 disagreed
auth          note: 7 call(s) ran on both sides, 0 disagreed
```

One line per pair, 320 calls in total, none disagreeing. `baseline` has no functions. Not
counted: four of `location`'s alias functions, which fail on the Solidity side (F3);
`slots`' `v09` to `v21`, which fail on the Solidity side (F2); and the `large` and `bound`
calls in `flow` and `vec_mem`, which fail on both sides. Most of those overflow on both
sides with the same error. Two do not: `nested` traps on overflow at once in Rust and runs
until the budget is used up in Solidity (F12), and `vec_mem` runs out of budget in Rust and
out of linear memory in Solidity. Malformed and wrongly typed arguments are covered under
F1.

### Table 35: Runtime cost, selected functions from every pair

```sh
antlion --compare <pair> --report cost
```

```
pair          function           sdk cpu  solang cpu     diff
scalar_id     i32_id           298508    300228     -1720
scalar_id     u128_id          301393    302741     -1348
scalar_id     i64_id2          300508    301160      -652
scalar_math   i32_op           276988    279243     -2255
scalar_store  i32_store        308269    308934      -665
scalar_store  i64_store        310537    310190      +347
storage       put_instance     233345    234575     -1230
mapping       put              277493    293657    -16164
mapping       put_p            285902    316437    -30535
slots         put_v05          367058    354194    +12864
slots         put_fx           386164    359502    +26662
struct_class  put_instance     258053    277081    -19028
struct_vec    push             297014    294872     +2142
struct_vec    write_field      311276    305560     +5716
vec_id        id               227014    263115    -36101
auth          no_auth          250007    257732     -7725
struct_store  write_all        256931    283389    -26458
struct_store  write_one        257955    267312     -9357
flow          branch           206938    209659     -2721
struct_mem    build            199813    215835    -16022
struct_id     id               234172    265337    -31165
struct_id     id_outer         239782    285196    -45414
vec_mem       push             232895    220250    +12645
location      via_memory       315812    323709     -7897
location      struct_direct    309185    300758     +8427
```

The `cpu insns` row for each function, measured with the module cached. Every call has a
large fixed overhead, so read the difference column. The Rust build is cheaper on eleven
of the sixteen pairs that have functions to call. It is more expensive on three: `vec_mem`
by about 12,000 (F8), `struct_vec` by 2,000 to 6,000, and `slots` by 6,000 to 27,000.
`scalar_store` is within 700 either way and `location` within 8,500 either way.

### Table 36: `struct_store`, every call each function makes

```sh
antlion --compare struct_store --report opcodes --dump
```

```
write_all, Rust side:
Call 5
write_all, Solidity side:
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 2 ; vec_put
Call 3 ; put_contract_data
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 2 ; vec_put
Call 3 ; put_contract_data
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 2 ; vec_put
Call 3 ; put_contract_data
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 2 ; vec_put
Call 3 ; put_contract_data
Call 4 ; log_from_linear_memory

read_all, Rust side:
Call 6
read_all, Solidity side:
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 5 ; vec_get
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 5 ; vec_get
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 5 ; vec_get
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 5 ; vec_get
Call 4 ; log_from_linear_memory
Call 4 ; log_from_linear_memory
Call 4 ; log_from_linear_memory
Call 4 ; log_from_linear_memory

write_one, Rust side:
Call 6
Call 5
write_one, Solidity side:
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 2 ; vec_put
Call 3 ; put_contract_data
Call 4 ; log_from_linear_memory

read_one, Rust side:
Call 6
read_one, Solidity side:
Call 0 ; has_contract_data
Call 1 ; get_contract_data
Call 5 ; vec_get
Call 4 ; log_from_linear_memory

Rust side, internal functions:
function #5
Call 0 ; map_new_from_linear_memory
Call 1 ; put_contract_data
function #6
Call 2 ; has_contract_data
Call 3 ; get_contract_data
Call 4 ; sparse_map_unpack_to_linear_memory
```

Only the `Call` lines of each function are shown. A call to an imported host function is
named; a call without a name goes to one of the contract's own internal functions, listed
at the end.

### Table 37: `struct_id`, every call each function makes

```sh
antlion --compare struct_id --report opcodes --dump
```

```
get_x, Rust side:
Call 2
get_x, Solidity side:
Call 0 ; symbol_new_from_linear_memory
Call 1 ; map_get
Call 0 ; symbol_new_from_linear_memory
Call 1 ; map_get
Call 5
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory

make, Rust side:
Call 5
make, Solidity side:
Call 5
Call 3 ; map_new
Call 0 ; symbol_new_from_linear_memory
Call 4 ; map_put
Call 0 ; symbol_new_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 4 ; map_put

id, Rust side:
Call 2
Call 5
id, Solidity side:
Call 0 ; symbol_new_from_linear_memory
Call 1 ; map_get
Call 0 ; symbol_new_from_linear_memory
Call 1 ; map_get
Call 5
Call 3 ; map_new
Call 0 ; symbol_new_from_linear_memory
Call 4 ; map_put
Call 0 ; symbol_new_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 2 ; log_from_linear_memory
Call 4 ; map_put

Rust side, internal functions:
function #2
Call 3
function #3
Call 4
function #4
Call 1 ; sparse_map_unpack_to_linear_memory
function #5
Call 6
function #6
Call 7
function #7
Call 0 ; map_new_from_linear_memory
```

As in Table 36, only the `Call` lines are shown. Each `log_from_linear_memory` call is on
an error path, before a trap.

### Table 38: `errors`, each failure form

```sh
antlion --compare errors --report returns
```

```
function        input  sdk                      solang                   verdict
div             small  U32(1000)                U32(1000)                same
div             neg    [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, ArithDomain) DebugInfo not available
get             small  U32(0)                   U32(0)                   same
req             small  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(Contract, #1) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
rev             small  [err] HostError: Error…  [err] HostError: Error…  both failed
set             small  U32(1)                   U32(1)                   same
asrt            small  [err] HostError: Error…  [err] HostError: Error…  both failed
custom          small  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(Contract, #1) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
set_fail        small  [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(Contract, #1) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
...
note: 19 call(s) ran on both sides, 0 disagreed
note: 7 call(s) failed on both sides with different errors
```

The `small` rows, and `neg` for `div`, the only function that fails at `neg` and not at
`small`. `rev` and `asrt` fail with `Error(WasmVm, InvalidAction)` on both sides, so their
errors are not printed.

### Table 39: `errors`, the declared interface

```sh
antlion --compare errors --report interface
```

```
sdk                                solang

div(u32) -> u32                    div(u32) -> u32
get() -> u32                       get() -> u32
req(u32) -> u32                    req(u32) -> u32
rev(u32) -> u32                    rev(u32) -> u32
set(u32) -> u32                    set(u32) -> u32
asrt(u32) -> u32                   asrt(u32) -> u32
custom(u32) -> result<u32, error>  custom(u32) -> u32
set_fail(u32) -> u32               set_fail(u32) -> u32
—                                  __constructor() -> void
```

### Table 40: `sequence`, every pair

```sh
antlion --compare <pair> --report sequence
```

```
scalar_id     note: 32 step(s) ran on both sides, 0 disagreed
scalar_store  note: 16 step(s) ran on both sides, 0 disagreed
scalar_math   note: 14 step(s) ran on both sides, 0 disagreed
storage       note: 8 step(s) ran on both sides, 0 disagreed
struct_store  note: 8 step(s) ran on both sides, 0 disagreed
struct_id     note: 8 step(s) ran on both sides, 0 disagreed
mapping       note: 8 step(s) ran on both sides, 0 disagreed
flow          note: 10 step(s) ran on both sides, 0 disagreed
location      note: 12 step(s) ran on both sides, 1 disagreed
vec_mem       note: 6 step(s) ran on both sides, 0 disagreed
struct_mem    note: 4 step(s) ran on both sides, 0 disagreed
slots         note: 18 step(s) ran on both sides, 0 disagreed
struct_class  note: 6 step(s) ran on both sides, 0 disagreed
struct_vec    note: 6 step(s) ran on both sides, 0 disagreed
vec_id        note: 10 step(s) ran on both sides, 0 disagreed
auth          note: 6 step(s) ran on both sides, 0 disagreed
errors        note: 6 step(s) ran on both sides, 0 disagreed
events        note: 12 step(s) ran on both sides, 0 disagreed
```

One line per pair, 190 steps in total. The one `location` step is a consequence of F3, as
described above. `baseline` has no functions.

### Table 41: `errors`, rollback across a sequence

```sh
antlion --compare errors --report sequence
```

```
step  function        sdk                      solang                   changed verdict
───────────────────────────────────────────────────────────────────────────────────────
1     div             U32(1000)                U32(1000)                none    same
2     get             U32(0)                   U32(0)                   none    same
...
5     set             U32(1)                   U32(1)                   both    same
...
8     set_fail        [err] HostError: Error…  [err] HostError: Error…  none    errors differ
       sdk    [err] HostError: Error(Contract, #1) DebugInfo not available
       solang [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available

sdk     instance    STORED                 1u32
solang  instance    false                  1u32

      round 2
9     div             U32(1000)                U32(1000)                none    same
10    get             U32(1)                   U32(1)                   none    same
```

`set_fail` stored `101` before it failed. Neither storage changed in step 8, both still
hold `1` after it, and `get` answers `1` on both sides at step 10.

### Table 42: `errors`, size and cost

```sh
antlion --compare errors --report size cost
```

```
sdk     solang  diff
1453    3876    -2423

module instantiate, paid by every call before its function runs
metric                 sdk       solang         diff
cpu insns           248518       235631       +12887
mem bytes          1192560      1127873       +64687

div(u32) -> u32
metric                 sdk       solang         diff
cpu insns           248592       236029       +12563
  instantiate       248518       235631       +12887
  run                   74          398         -324
mem bytes          1192592      1127905       +64687

get() -> u32
metric                 sdk       solang         diff
cpu insns           249650       236947       +12703
  instantiate       248518       235631       +12887
  run                 1132         1316         -184
mem bytes          1192560      1127873       +64687

set(u32) -> u32
metric                 sdk       solang         diff
cpu insns           255460       244273       +11187
  instantiate       248518       235631       +12887
  run                 6942         8642        -1700
mem bytes          1193512      1128825       +64687
```

The functions that do not fail at `small`. The others are not measured, because `cost`
measures a call only when it succeeds.

### Table 43: `events`, each event shape

```sh
antlion --compare events --report events
```

```
two(u32) -> u32

side    kind      topics                         data
sdk     result    1u32
sdk     contract  [two]                          {v: 1u32, w: 2u32}
solang  result    1u32
solang  contract  []                             1u32

events: differ

pair(address, u32, u32) -> u32

side    kind      topics                         data
sdk     result    1u32
sdk     contract  [pair, contract(ba863dea..), 1u32] {v: 1u32, w: 2u32}
solang  result    1u32
solang  contract  [contract(ba863dea..), 1u32]   1u32

events: differ

ping() -> u32

side    kind      topics                         data
sdk     result    1u32
sdk     contract  [ping]                         {}
solang  result    1u32
solang  contract  []                             false

events: differ

value(u32) -> u32

side    kind      topics                         data
sdk     result    1u32
sdk     contract  [value]                        {v: 1u32}
solang  result    1u32
solang  contract  []                             1u32

events: differ
```

`keyed`, `many` and `emit_fail` are left out; they follow the same pattern.

### Table 44: `events`, the Solidity build of `two`

```sh
antlion --compare events --report opcodes --dump
```

```
Call 0 ; vec_new
LocalGet { local_index: 0 }
I64Const { value: -4294967296 }
I64And
I64Const { value: 4 }
I64Or
Call 1 ; contract_event
```

The Solidity column only, cut to the lines before `contract_event`. `vec_new` makes the
empty topic list; the data is `v`, made into a `u32` value. Before these lines, `v + 1` is
computed and checked for overflow, and its result is not used.

### Table 45: `events`, the diagnostics of a failed call

```sh
antlion --compare events --report events
```

```
emit_fail(u32) -> u32

side    kind      topics                         data
sdk     result    [err] Error(Contract, #1)
sdk     contract  [value]                        {v: 1u32} *
sdk     error     [error, Error(Contract(1))]    ["failing with contract error", 1u32] *
sdk     error     [error, Error(Contract(1))]    "escalating error to VM trap from failed host function call: fail_with_error" *
sdk     log       [log]                          ["VM call trapped with HostError", emit_fail, Error(Contract(1))] *
sdk     error     [error, Error(Contract(1))]    ["contract call failed", emit_fail, [1u32]]
solang  result    [err] Error(WasmVm, InvalidAction)
solang  contract  []                             1u32 *
solang  log       [log]                          "runtime_error:  require condition failed in events.sol:51:26-37,\n" *
solang  error     [error, Error(WasmVm(InvalidAction))] ["VM call trapped: UnreachableCodeReached", emit_fail] *
solang  error     [error, Error(WasmVm(InvalidAction))] ["contract call failed", emit_fail, [1u32]]
```

### Table 46: `events`, size and cost

```sh
antlion --compare events --report size cost
```

```
sdk     solang  diff
2638    3646    -1008

module instantiate, paid by every call before its function runs
metric                 sdk       solang         diff
cpu insns           246271       233157       +13114

two(u32) -> u32
cpu insns           255241       238220       +17021
  instantiate       246271       233157       +13114
  run                 8970         5063        +3907
many(u32) -> u32
cpu insns           274010       246294       +27716
  instantiate       246271       233157       +13114
  run                27739        13137       +14602
pair(address, u32, u32) -> u32
cpu insns           255982       243371       +12611
  instantiate       246271       233157       +13114
  run                 9711        10214         -503
ping() -> u32
cpu insns           255111       237172       +17939
  instantiate       246271       233157       +13114
  run                 8840         4015        +4825
keyed(u32, u32) -> u32
cpu insns           255217       240055       +15162
  instantiate       246271       233157       +13114
  run                 8946         6898        +2048
value(u32) -> u32
cpu insns           255572       237560       +18012
  instantiate       246271       233157       +13114
  run                 9301         4403        +4898
```

The `cpu insns`, `instantiate` and `run` rows only. `emit_fail` is not measured, because it
fails at `small`.

### Table 47: `flow`, cost split and cost types

```sh
antlion --compare flow --report cost --detail
```

```
module parse, paid once at upload
metric                 sdk       solang         diff
cpu insns           174628       250185       -75557
mem bytes            33067        50291       -17224

module instantiate, paid by every call before its function runs
metric                 sdk       solang         diff
cpu insns           206800       208901        -2101
mem bytes          1121835      1124566        -2731
cost type                             sdk cpu   solang cpu      sdk mem   solang mem
MemAlloc                               136848       136853      1049856      1049896
MemCpy                                   2231         2236            0            0
MemCmp                                    710          710            0            0
VisitObject                               660          660            0            0
InvokeVmFunction                         1945         1945           14           14
InstantiateWasmInstructions             43030        43030        70704        70704
InstantiateWasmExports                  20889        16247          645          501

early(u32) -> u32
metric                 sdk       solang         diff
cpu insns           207154       210231        -3077
  instantiate       206800       208901        -2101
  run                  354         1330         -976
mem bytes          1121867      1124598        -2731
cost type                             sdk cpu   solang cpu      sdk mem   solang mem
WasmInsnExec                              348         1324            0            0
MemAlloc                               136852       136857      1049888      1049928
```

The module blocks and `early`, with the cost types cut to the largest rows.

### Table 48: `vec_mem` and `struct_mem`, cost split

```sh
antlion --compare vec_mem struct_mem --report cost
```

```
sum(u32) -> u32
metric                 sdk       solang         diff
cpu insns           234121       222154       +11967
  instantiate       228103       204282       +23821
  run                 6018        17872       -11854
mem bytes          1125166      1189794       -64628

push(u32) -> u32
metric                 sdk       solang         diff
cpu insns           232895       220250       +12645
  instantiate       228103       204282       +23821
  run                 4792        15968       -11176
mem bytes          1125166      1189794       -64628

build(u32) -> u32
metric                 sdk       solang         diff
cpu insns           199813       215835       -16022
  instantiate       199719       201751        -2032
  run                   94        14084       -13990
mem bytes          1121424      1189083       -67659
```

`sum` and `push` from `vec_mem`, `build` from `struct_mem`.

### Table 49: `flow`, cost scaling

```sh
antlion --compare flow --report scaling
```

```
nested(u32) -> u32
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           206501       210350        -3849      1121851      1124582
2           206573       211082        -4509      1121851      1124582
4           206717       213074        -6357      1121851      1124582
8           207005       219170       -12165      1121851      1124582
16          207581       239810       -32229      1121851      1124582
32          208733       314882      -106149      1121851      1124582
64          211037       600194      -389157      1121851      1124582
growth: sdk n^1.0, solang n^1.9

loop_(u32) -> u32
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           206533       209598        -3065      1121851      1124582
2           206597       209690        -3093      1121851      1124582
4           206725       209874        -3149      1121851      1124582
8           206981       210242        -3261      1121851      1124582
16          207493       210978        -3485      1121851      1124582
32          208517       212450        -3933      1121851      1124582
64          210565       215394        -4829      1121851      1124582
growth: sdk n^1.0, solang n^1.0
```

### Table 50: `vec_id` and `vec_mem`, cost scaling

```sh
antlion --compare vec_id vec_mem --report scaling
```

```
id(vec<u32>) -> vec<u32>
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           226493       256238       -29745      1124450      1193501
2           226493       259237       -32744      1124450      1193637
4           226493       265597       -39104      1124450      1193933
8           226493       278341       -51848      1124450      1194621
16          226493       303925       -77432      1124450      1196381
32          226493       355477      -128984      1124450      1201437
64          226493       460117      -233624      1124450      1217693
growth: sdk flat, solang n^1.0

len(vec<u32>) -> u32
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           227144       251703       -24559      1124450      1193269
2           227144       252421       -25277      1124450      1193269
4           227144       254213       -27069      1124450      1193269
8           227144       257797       -30653      1124450      1193269
16          227144       264965       -37821      1124450      1193269
32          227144       279301       -52157      1124450      1193269
64          227144       307973       -80829      1124450      1193269
growth: sdk flat, solang n^1.0

push(u32) -> u32
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           232374       219729       +12645      1125150      1189778
2           234591       219677       +14914      1125286      1189778
4           239031       219929       +19102      1125582      1189778
8           247935       220433       +27502      1126270      1189778
16          265839       221441       +44398      1128030      1189778
32          302031       223457       +78574      1133086      1189778
64          375951       227489      +148462      1149342      1189778
growth: sdk n^1.0, solang n^1.0
```

`id` and `len` from `vec_id`, `push` from `vec_mem`.

### Table 51: `mapping` and `struct_vec`, cost scaling

```sh
antlion --compare mapping struct_vec --report scaling
```

```
put(u32, u32) -> u32
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           282881       293310       -10429      1197710      1135602
2           287969       293478        -5509      1199120      1136226
4           297158       293817        +3341      1201940      1137474
8           315857       294486       +21371      1207580      1139970
16          352256       295815       +56441      1218860      1144962
32          424055       298464      +125591      1241420      1154946
64          566654       303753      +262901      1286540      1174914
growth: sdk n^1.0, solang n^1.0

put_p(u32, u32) -> u32
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           296086       318734       -22648      1199812      1202490
2           305970       321546       -15576      1202636      1203850
4           324751       327173        -2422      1208284      1206570
8           362634       338418       +24216      1219580      1212010
16          437401       360899       +76502      1242172      1222890
32          585936       405852      +180084      1287356      1244650
64          882007       495749      +386258      1377724      1288170
growth: sdk n^1.0, solang n^1.0

push(u32) -> u32
n          sdk cpu   solang cpu         diff      sdk mem   solang mem
1           306750       297071        +9679      1201208      1199052
2           311622       299791       +11831      1202924      1200092
4           321366       305231       +16135      1206356      1202172
8           340854       316111       +24743      1213220      1206332
16          379830       337871       +41959      1226948      1214652
32          457782       381391       +76391      1254404      1231292
64          613686       468431      +145255      1309316      1264572
growth: sdk n^1.0, solang n^1.0
```

`put` and `put_p` from `mapping`, `push` from `struct_vec`.

### Table 52: `access`, a mapping write and a struct write

```sh
antlion --compare mapping struct_store --report access
```

```
put(u32, u32) -> u32

side      read  write  bytes  result
sdk          1      1    148  1u32
solang       1      1    164  1u32

sdk     wrote  instance entry: 104 -> 148 bytes
sdk     called get_contract_data instance x1
sdk     called has_contract_data instance x1
sdk     called put_contract_data instance x1
solang  wrote  instance entry: 148 -> 164 bytes
solang  called get_contract_data instance x2
solang  called has_contract_data instance x2
solang  called put_contract_data instance x1

write_all(u32)

side      read  write  bytes  result
sdk          1      1    212  void
solang       1      1    160  void

sdk     wrote  instance entry: 104 -> 212 bytes
sdk     called put_contract_data instance x1
solang  wrote  instance entry: 160 -> 160 bytes
solang  called get_contract_data instance x4
solang  called has_contract_data instance x4
solang  called put_contract_data instance x4
```

`put` from `mapping`, `write_all` from `struct_store`.

### Table 53: `access`, a storage class and the slots

```sh
antlion --compare storage slots --report access
```

```
put_idiomatic(u32) -> u32

side      read  write  bytes  result
sdk          1      1    128  1u32
solang       2      1     76  1u32

sdk     wrote  instance entry: 104 -> 128 bytes
sdk     called get_contract_data instance x1
sdk     called has_contract_data instance x1
sdk     called put_contract_data instance x1
solang  wrote  persistent Error(Contract(0)): 0 -> 76 bytes
solang  called get_contract_data persistent x1
solang  called has_contract_data persistent x1
solang  called put_contract_data persistent x1

put_m(u32) -> u32

side      read  write  bytes  result
sdk          1      1    148  1u32
solang       1      1    308  1u32

sdk     wrote  instance entry: 104 -> 148 bytes
sdk     called get_contract_data instance x1
sdk     called has_contract_data instance x1
sdk     called put_contract_data instance x1
solang  wrote  instance entry: 292 -> 308 bytes
solang  called get_contract_data instance x2
solang  called has_contract_data instance x2
solang  called put_contract_data instance x1
```

`put_idiomatic` from `storage`, `put_m` from `slots`.

### Table 54: `meta`, the `events` pair

```sh
antlion --compare events --report meta
```

```
contractenvmetav0: the host protocol each build was made for

item               sdk                                             solang
protocol           28                                              23
pre_release        0                                               0

contractmetav0: key and value text each build carries

key                sdk                                             solang
rsver              1.92.0                                          —
rssdkver           28.0.0#48d506712f964094d14176e2f0b02afcd1054567 —
rssdk_spec_shaking 2                                               —
cliver             28.1.0#c0f4d0da891bbf214c08b8c5035ae6db80e9a3bd —

contractspecv0: what each build declares, by kind

kind              sdk solang
function            7      8
struct              0      0
union               0      0
enum                0      0
error enum          1      0
event               5      0

sdk     error enum  Error{TooSmall = 1}
sdk     event       Two
sdk     event       Pair
sdk     event       Ping
sdk     event       Keyed
sdk     event       Value

note: the builds declare different types, errors or events; functions are compared by `interface`
```

### Table 55: `numeric`, the edge cases that differ

```sh
antlion --compare numeric --report returns
```

```
function        input  sdk                      solang                   verdict
───────────────────────────────────────────────────────────────────────────────
neg             small  I32(-1)                  I32(-1)                  same
neg             large  I32(-2147483647)         I32(-2147483647)         same
neg             bound  [err] HostError: Error…  [err] HostError: Error…  both failed
neg             neg    I32(1)                   I32(1)                   same
shl             small  U32(2)                   U32(2)                   same
shl             large  [err] HostError: Error…  U32(2147483648)          sdk failed
shl             bound  [err] HostError: Error…  U32(2147483648)          sdk failed
shl             neg    U32(1)                   U32(1)                   same
shr             small  U32(2147483647)          U32(2147483647)          same
shr             large  [err] HostError: Error…  U32(1)                   sdk failed
shr             bound  [err] HostError: Error…  U32(1)                   sdk failed
shr             neg    U32(4294967295)          U32(4294967295)          same
div_by          small  U32(100)                 U32(100)                 same
div_by          large  U32(0)                   U32(0)                   same
div_by          bound  U32(0)                   U32(0)                   same
div_by          neg    [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, ArithDomain) DebugInfo not available
rem_by          small  U32(0)                   U32(0)                   same
rem_by          large  U32(100)                 U32(100)                 same
rem_by          bound  U32(100)                 U32(100)                 same
rem_by          neg    [err] HostError: Error…  [err] HostError: Error…  errors differ
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, ArithDomain) DebugInfo not available
neg_div         small  I32(-1)                  I32(-1)                  same
neg_div         large  I32(-2147483647)         I32(-2147483647)         same
neg_div         bound  [err] HostError: Error…  I32(-2147483648)         sdk failed
neg_div         neg    I32(1)                   I32(1)                   same
```

The rows of `shl`, `shr`, `neg`, `neg_div`, `div_by` and `rem_by`, without `badarg`.

### Table 56: `bytes_id`, the declared interface

```sh
antlion --compare bytes_id --report interface
```

```
sdk                             solang
bytes_id(bytes) -> bytes        bytes_id(bytes) -> bytes
bytes_len(bytes) -> u32         bytes_len(bytes) -> u32
string_id(string) -> string     string_id(string) -> string
bytes32_id(bytes32) -> bytes32  bytes32_id(bytes) -> bytes
bytes_make(u32) -> bytes        bytes_make(u32) -> bytes
string_len(string) -> u32       string_len(string) -> u32
bytes_first(bytes) -> u32       bytes_first(bytes) -> u32
—                               __constructor() -> void
```

### Table 57: `scalar_math`, 256 bit addition

```sh
antlion --compare scalar_math --report imports
antlion --compare scalar_math scalar_id --report cost
```

```
import   function                                         sdk   solang
i.9      obj_from_u256_pieces                               —        ✓
i.c      obj_to_u256_hi_hi                                  —        ✓
i.n      u256_add                                           ✓        —
i.v      i256_add                                           ✓        —
total                                                      12       21

u128_op(u128, u128) -> u128
metric                 sdk       solang         diff
cpu insns           299259       353600       -54341
  instantiate       294535       348116       -53581
  run                 4724         5484         -760
mem bytes          1132947      1141184        -8237

u256_op(u256, u256) -> u256
metric                 sdk       solang         diff
cpu insns           301354       355932       -54578
  instantiate       294535       348116       -53581
  run                 6819         7816         -997
mem bytes          1133046      1141184        -8138

u128_id2(u128, u128) -> u128
metric                 sdk       solang         diff
cpu insns           313208       379301       -66093
  instantiate       308038       374255       -66217
  run                 5170         5046         +124
mem bytes          1133956      1142850        -8894

u256_id2(u256, u256) -> u256
metric                 sdk       solang         diff
cpu insns           309388       381457       -72069
  instantiate       308038       374255       -66217
  run                 1350         7202        -5852
mem bytes          1133956      1142850        -8894
```

One import of each kind and the two adds; the Solidity build imports all ten 256 bit piece
functions. Then `u128` next to `u256`.

### Table 58: `trace`, a mapping write

```sh
antlion --compare mapping --report trace
```

```
put(u32, u32) -> u32

host function                               sdk solang
get_contract_data                             1      2
has_contract_data                             1      2
map_get                                       0      1
map_has                                       0      1
map_put                                       0      1
obj_cmp                                       2      0
put_contract_data                             1      1
vec_new_from_linear_memory                    2      0
total                                         7      8
result                                   1u32   1u32

  -  sdk     vec_new_from_linear_memory(U32(1048528), U32(2))
  +  solang  has_contract_data(False, Instance)
  +  solang  get_contract_data(False, Instance)
  +  solang  map_put(Map(obj#3), U32(1), U32(1))
  ~  sdk     put_contract_data(Vec(obj#3), U32(1), Instance)
     solang  put_contract_data(False, Map(obj#7), Instance)
  -  sdk     vec_new_from_linear_memory(U32(1048528), U32(2))
  ~  sdk     has_contract_data(Vec(obj#5), Instance)
     solang  has_contract_data(False, Instance)
  -  sdk     obj_cmp(Vec(obj#3), Vec(obj#5))
  ~  sdk     get_contract_data(Vec(obj#5), Instance)
     solang  get_contract_data(False, Instance)
  -  sdk     obj_cmp(Vec(obj#3), Vec(obj#5))
  +  solang  map_has(Map(obj#7), U32(1))
  +  solang  map_get(Map(obj#7), U32(1))

trace: differs: 0 same, 3 with other arguments, 4 sdk only, 5 solang only
```

`put` only.


### Table 59: cost to fail, `errors` and `flow`

```sh
antlion --compare errors flow --report cost
```

```
cost to fail, for every call that failed on one side or both

function           input       sdk run   solang run  sdk      solang
div                neg              74          398  failed   failed
req                small           941         1009  failed   failed
req                neg             941         1009  failed   failed
rev                small            78         1009  failed   failed
rev                neg              78         1009  failed   failed
asrt               small            78        17559  failed   failed
asrt               neg              78        17559  failed   failed
custom             small            86        17679  failed   failed
custom             neg              86        17679  failed   failed
set_fail           small          2646         3562  failed   failed
set_fail           large           756         2563  failed   failed
set_fail           bound           756         2563  failed   failed
set_fail           neg            2646         3562  failed   failed

cost to fail, for every call that failed on one side or both

function           input       sdk run   solang run  sdk      solang
early              large      16497286     16801253  failed   failed
early              bound      16497286     16801253  failed   failed
loop_              large       5931838      8528165  failed   failed
loop_              bound       5931838      8528165  failed   failed
branch             large           138         1053  failed   failed
branch             bound           138         1053  failed   failed
nested             large           222     95456070  failed   failed
nested             bound           222     95456070  failed   failed
while_             large       5931838      8528165  failed   failed
while_             bound       5931838      8528165  failed   failed
```

The cost to fail tables only.

### Table 60: `ttl`, the `ttl` pair

```sh
antlion --compare ttl --report ttl
```

```
deploy: uploading the code, creating the instance and running the constructor

sdk     rent: persistent 219549330 ledger bytes, of which the constructor 0
solang  rent: persistent 319049640 ledger bytes, of which the constructor 0

bump_i() -> u64

side    entry                          live until        now
sdk     instance entry                       4095       9000
sdk     contract code                        4095       9000
solang  instance entry                       4095       9000
solang  contract code                        4095       9000

sdk     rent: persistent 262976670 ledger bytes over 2 entries, temporary 0 over 0; answer 1u64
solang  rent: persistent 382158360 ledger bytes over 2 entries, temporary 0 over 0; answer 1u64

lifetime: one deploy per side, every function in order, then the ledger moved past the next expiry

ledger 0
  get_i        0u64                     0u64                     same
  get_p        0u64                     0u64                     same
  get_t        0u64                     0u64                     same
  put_i        1u64                     1u64                     same
  put_p        1u64                     1u64                     same
  put_t        1u64                     1u64                     same
  bump_i       1u64                     1u64                     same
  bump_p       1u64                     1u64                     same
  bump_t       1u64                     1u64                     same

ledger 3001
  expired on sdk: temporary T
  expired on solang: temporary true
  get_i        1u64                     1u64                     same
  get_p        1u64                     1u64                     same
  get_t        0u64                     0u64                     same
  put_i        1u64                     1u64                     same
  put_p        1u64                     1u64                     same
  put_t        1u64                     1u64                     same
  bump_i       1u64                     1u64                     same
  bump_p       1u64                     1u64                     same
  bump_t       1u64                     1u64                     same

ledger 9002
  expired on sdk: persistent P, temporary T
  expired on solang: persistent false, temporary true
  get_i        1u64                     1u64                     same
  get_p        1u64                     1u64                     same
  get_t        0u64                     0u64                     same
  put_i        1u64                     1u64                     same
  put_p        1u64                     1u64                     same
  put_t        1u64                     1u64                     same
  bump_i       1u64                     1u64                     same
  bump_p       1u64                     1u64                     same
  bump_t       1u64                     1u64                     same

ledger 18003
  expired on sdk: persistent P, temporary T, instance entry, contract code
  expired on solang: persistent false, temporary true, instance entry, contract code
  get_i        1u64                     1u64                     same
  get_p        1u64                     1u64                     same
  get_t        0u64                     0u64                     same
  put_i        1u64                     1u64                     same
  put_p        1u64                     1u64                     same
  put_t        1u64                     1u64                     same
  bump_i       1u64                     1u64                     same
  bump_p       1u64                     1u64                     same
  bump_t       1u64                     1u64                     same
```

The deploy, `bump_i`, and the lifetime run.
