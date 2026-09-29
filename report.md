# Solang Soroban Target

## Differential evaluation

Prepared as part of the LFDT mentorship on comparing Solang to the Stellar Rust SDK.

`antlion`, a differential harness that builds a contract written twice with the same
semantics, once from the Rust `soroban-sdk` and once from Solidity through Solang, and then
thoroughly compares the two WebAssembly modules.

---

## Contents

1. [Summary](#summary)
2. [Setup](#setup)
3. [Methodology](#methodology)
4. [How the runtime cost breaks down](#how-the-runtime-cost-breaks-down)
5. [What was tested](#what-was-tested)
6. [Limitations](#limitations)
7. [Findings](#findings)
8. [Recommendations](#recommendations)
9. [Appendix A: Contract Inventory](#appendix-a-contract-inventory)
10. [Appendix B: Harness Output](#appendix-b-harness-output)

---

## Summary

### Overview

Solang compiles Solidity for Soroban, where the main toolchain is the Rust `soroban-sdk`.
The two translate the same program differently: a different storage layout, different
encoding and decoding of values, and different optimisation. Writing each contract twice,
once in each language and with the same semantics, this evaluation compares:

- **Behaviour:** whether both builds return the same results for the same inputs.
- **Storage and encoding:** how each build lays out state and encodes values.
- **Runtime cost:** what each build costs to deploy and to run.
- **Cost breakdown:** where each cost difference comes from.

Seventeen contract pairs were built, each with identical exported function names on both
sides, and compared by `antlion` on ten reports: module size, section breakdown, host
function imports, declared interface, return values, metered runtime cost, storage after a
call, opcode histogram, per function complexity and declared memory layout.

### Results

Thirteen findings, numbered by importance (lower number = greater importance).
Each carries a confidence level: high where the harness measures the claim directly, medium
where the harness supports the claim but does not settle it, and further measurement could
confirm or change it.

- **F1: A wrongly typed argument is silently accepted.** Soroban leaves argument checking
  to the contract. The Rust build rejects any argument whose type does not match its
  declaration. The Solang build does not check `u32`, `i32`, `bool` or `address` arguments,
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
  The contract compiles and deploys without any warning, and the keys that do work sit
  under `Bool`, `Void` and `Error` values (Tables [6](#table-6-storage-the-storage-class-and-key-of-each-variable) and [7](#table-7-slots-the-storage-entries-after-each-function)).

- **F3: A `storage` reference local ignores the storage class.** A Solidity local that
  aliases a state variable, `T storage r = x`, always uses persistent storage, whatever the
  variable is declared as ([Table 11](#table-11-location-the-storage-class-passed-to-each-storage-call)). On an `instance` or `temporary` variable it fails on every call
  with `Error(Value, InvalidInput)`, for lists and structs, reads and writes alike
  ([Table 10](#table-10-location-each-alias-and-its-direct-twin)). Direct
  indexing works, and a `memory` copy works, including correctly discarding a write to the
  copy.

- **F4: Idiomatic code on each side writes to a different ledger namespace.** An
  unannotated Solidity state variable lands in persistent storage, while the habit in Rust
  is instance storage ([Table 6](#table-6-storage-the-storage-class-and-key-of-each-variable)). Both are documented behaviour, but porting a contract without
  noticing changes its storage lifetime and its rent.

- **F5: A struct is one host value in Rust and one per field in Solidity.** Stored, it is a
  `Map` in Rust and a `Vec` in Solidity under a different key ([Table 13](#table-13-struct_store-the-stored-struct)), so neither contract
  can read the other's struct, and Solang builds it one field at a time: 17 calls against 1
  to write a four field struct ([Table 15](#table-15-struct_store-per-function-metrics)). At the boundary both use the same `Map`, so a Rust client can call a
  Solidity contract with a struct, but Solang makes 9 host calls against 2 to pass one
  through (Tables [2](#table-2-struct_id-malformed-struct-arguments) and [16](#table-16-struct_id-calls-per-function-and-host-functions-imported)). Either way the Rust build is 9,000 to 45,000 CPU instructions a
  call cheaper ([Table 35](#table-35-runtime-cost-selected-functions-from-every-pair)).

- **F6: A mapping is one ledger entry in Solidity and one per key in Rust.** The whole
  mapping is a single value on the Solidity side ([Table 18](#table-18-mapping-the-stored-mapping)), so every access loads and stores all of
  it, the mapping is bounded by the ledger entry size limit, and neither contract can read
  the other's mapping. On a one key mapping the Rust build is 14,000 to 31,000 CPU
  instructions a call cheaper ([Table 19](#table-19-mapping-cost-per-call)).

- **F7: The error path is inlined at every trap site.** An error reporting sequence is
  pasted at every trap site rather than called as a shared handler, about 37% of the
  instructions in a small module ([Table 20](#table-20-flow-one-trap-site-on-the-solidity-side)). It costs size only, with no effect on
  behaviour.

- **F8: A list is held in linear memory rather than by the host.** Solang keeps a `memory`
  list in its own linear memory; the Rust build keeps it on the host. For a one element
  list built inside a call, the Solidity build measured about 12,000 CPU instructions
  cheaper and used about 65,000 more memory bytes ([Table 24](#table-24-vec_mem-cost-per-call)). At the contract boundary Solang
  copies every element in and out, 26,000 to 36,000 more CPU instructions a call
  ([Table 26](#table-26-vec_id-per-function-metrics-and-cost)). Both are
  measured at small sizes only.

- **F9: A local struct is materialised rather than eliminated.** 16,000 to 17,000 more CPU
  instructions a call for a struct the Rust build never creates ([Table 27](#table-27-struct_mem-per-function-metrics-memory-operations-and-cost)).

- **F10: 229 bytes of toolchain metadata ship in every build.** Build provenance, none of
  it read on chain. The Rust build carries 230 bytes of metadata of its own, which is why
  the size gap on an empty contract is only 36 bytes ([Table 29](#table-29-baseline-bytes-per-section-of-an-empty-contract)).

- **F11: Every aggregate state variable is written at deploy.** The Rust build writes
  nothing until a function does. Deployment pays one storage write per array, struct and
  mapping ([Table 30](#table-30-__constructor-on-the-solidity-side-per-pair)), and a `persistent` one pays rent from deploy rather than from first
  use.

- **F12: A nested loop is executed rather than reduced to a multiply.** Linear against
  quadratic for the same source. At runtime this shows only indirectly: at the largest
  input the Solidity build runs until its budget is exhausted while the Rust build traps at
  once ([Table 31](#table-31-flow-the-nested-loop)). The loop here is the simplest possible case.

- **F13: The decode of an unused argument is removed.** Only a decode that cannot trap is
  removable, so this is the static evidence that the four types in F1 are not validated
  ([Table 32](#table-32-scalar_id-bytes-per-function-with-one-and-two-arguments)).

### Across all pairs

**No well formed value is computed incorrectly.** 320 calls ran on both builds, and every
one returned the same value on both ([Table 34](#table-34-return-value-agreement-every-pair)). Arithmetic, control flow,
storage round trips, 64 bit and 128 bit values at the representation boundary, negatives,
zeros, empty sequences, structs passed in and returned, mappings and authorization all
produce identical results.

**On size**, the Rust build is smaller in all seventeen pairs, from 36 bytes on an empty
contract to 5,029 on the largest, with the ratio reaching 3.3x, mostly from F7 and F10
([Table 33](#table-33-module-size-every-pair)).

**On runtime cost**, the Rust build is cheaper in eleven of sixteen pairs, by 600 to 45,000
CPU instructions a call. The Solidity build is cheaper in three ([Table 35](#table-35-runtime-cost-selected-functions-from-every-pair)). See
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
| Harness | `antlion`, this repository |

### What is compared

The artifact each toolchain ships, the final WebAssembly modules. Both sides are built
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
skip link time optimisation, which a deployed contract does not. stellar-cli requires
`overflow-checks`.

The profile also makes Rust trap on overflow, as Solidity 0.8 always does, so overflow
behaves the same on both sides.

---

## Methodology

Each pair lives in `contracts/<name>/`: a Rust crate using `soroban-sdk` in `sdk/`, and a
Solidity file, `solang/<name>.sol`. Both export functions with the same names, so the
reports can match them.

Both are compiled to WebAssembly, into `out/<name>/sdk.wasm` and `out/<name>/solang.wasm`,
and ten reports compare them.

| Report | What it compares |
|---|---|
| `size` | total module bytes |
| `sections` | bytes per WebAssembly section, custom sections named |
| `imports` | the set of Soroban host functions each module imports |
| `interface` | every exported signature, decoded from `contractspecv0` |
| `returns` | the value each build answers, as an `ScVal` |
| `cost` | CPU instructions and memory bytes charged for one call, module cached |
| `ledger` | every storage entry after one call: class, key and value |
| `opcodes` | instruction histogram by kind, with optional disassembly |
| `functions` | per function bytes, instructions, locals, block depth, call count |
| `layout` | declared linear memory pages, tables and globals |

`returns`, `cost` and `ledger` run the contracts. Each call deploys the module into a fresh
`soroban-env-host`, so every call starts from empty storage. The other reports read the
module bytes.

`cost` puts the module in the host's module cache before the measured call. The network
keeps every live contract parsed (CAP-0065, protocol 23) and charges parsing at upload and
restore, not per call, so measuring without the cache would add a charge the network does
not make.

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

**Most of a call is fixed overhead.** The cheapest function measured, a single branch in
`flow`, costs about 207,000 CPU instructions on the Rust build and 210,000 on the Solidity
build. Every call pays at least that, so compare the difference between builds, not the
totals.

**Where the largest differences appear:**

| What differs | Which side pays | Evidence |
|---|---|---|
| Host calls per struct field | Solidity: 9 host calls against 2 for one struct round trip, 17 calls against 1 to write a four field struct | F5, F6 |
| Copying a list through linear memory | Solidity: `id` makes 9 calls against none in Rust, copying each element in and out | F8 |
| Guest memory | Solidity keeps memory aggregates in linear memory, about 65,000 to 68,000 more memory bytes a call; Rust keeps them as host objects | F8, F9 |

**By what a function does**, the Rust build is cheaper by:

| What the function does | Rust cheaper by |
|---|---|
| identity, arithmetic, control flow | 600 to 3,800 |
| one storage write and read | 700 to 5,600 |
| a struct or a map, stored or passed | 9,000 to 45,000 |
| a struct built and discarded | 16,000 to 17,000 |
| a list passed in and returned | 26,000 to 36,000 |

The Solidity build is cheaper on `vec_mem` by about 12,000, on `struct_vec` by 2,000 to
6,000, and on `slots` by 6,000 to 27,000. `scalar_store` and `location` are mixed. The
harness reports cost per call, not by category, so for these three it shows that the
Solidity build is cheaper but not which part of the call accounts for it. All of this is at
`n = 1` for list and loop functions; see [Limitations](#limitations).

---

## What was tested

Seventeen contract pairs, 103 exported function pairs, and 320 calls compared on both
builds.

**Scalar types at the contract boundary.** `bool`, `u32`, `i32`, `u64`, `i64`, `u128`,
`i128` and `address` passed in and returned unchanged, at one and two arguments, across
all four value classes. Covered by `scalar_id`.

**Arithmetic.** One addition per numeric type, with `&&` for `bool`, over the same type
set. Covered by `scalar_math`.

**Storage round trips.** Each scalar type written to instance storage and read back.
Covered by `scalar_store`.

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

These were not covered. These are gaps, which needs additional work.

- **Only four values per type.** Each function is called with four value classes. No range
  is swept and no input is randomised, so other values may behave differently.

- **Storage is compared after one call only.** The `ledger` report shows what a single
  call on a fresh deploy leaves behind. It does not follow state across a sequence of
  calls, and it uses `small` inputs only.

- **Failure modes are only partly compared.** Malformed structs and wrongly typed
  arguments are compared. No pair triggers `require`, `revert`, `assert` or a custom error
  on purpose, so whether contract logic fails the same way on both sides is untested.

- **Authorization is tested in recording mode only.** The host grants whatever is asked
  for. No signed authorization entries are built, nested and custom account authorization
  are not exercised, and `cost` runs without authorization, so the cost of a guarded call
  is not measured.

- **Time to live and rent are untested.** No pair extends a time to live. F4 shows state
  landing in a different storage class but does not measure the rent that follows.

- **Only the features in the pairs are tested.** Cross contract calls, events, modifiers,
  inheritance and every other Solidity feature not used by a pair are untested, and which
  features the Soroban target supports at all is not measured.

- **Cost scaling is not measured.** Every cost figure is a single call at one input, which
  is `n = 1` for list and loop functions. How cost grows with a mapping's size (F6)
  or a list's length (F8) is not measured, and neither is F12's quadratic
  loop, whose only runtime evidence is indirect: the budget running out at
  `n = u32::MAX`. A sweep mode that calls one function at several sizes would settle all
  three.

- **The first call after an upload is not measured.** A contract called in the ledger it
  was uploaded in pays for parsing on that call. `cost` measures the cached case that every
  later call pays.

- **The ledger entry size limit is not measured.** A Solidity mapping is one entry
  (F6), so it is bounded by the maximum size of one entry. No pair grows a mapping
  far enough to reach it.

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
| F8 | A list is held in linear memory rather than by the host | encoding, cost | medium, measured at `n = 1` only |
| F9 | A local struct is materialised rather than eliminated | cost | high |
| F10 | 229 bytes of toolchain metadata ship in every build | size | high |
| F11 | Every aggregate state variable is written at deploy | storage, cost | high |
| F12 | A nested loop is executed rather than reduced to a multiply | cost | medium, the curve is not measured |
| F13 | The decode of an unused argument is removed | evidence for F1 | high |

### F1: Any value is accepted for a `u32`, `i32`, `bool` or `address` argument

**Area:** behaviour. **Confidence:** high. **Contracts:** `contracts/scalar_id`, `contracts/struct_id`.

**What happens.** Soroban does not check call arguments against the contract
specification; the contract has to check what arrives. The Rust build checks the type tag
of every argument and traps on a mismatch. The Solang build checks `u64`, `i64`, `u128`
and `i128`, and does not check `u32`, `i32`, `bool` or `address` at all. A value of the
wrong type is read as the declared type, or passed through unchanged for `address`.

**Evidence.** The `badarg` row of `returns` sends each function's first non struct
argument as the wrong type: `i32(-1)`, or `u32::MAX` where an `i32` is declared.
`scalar_id` ([Table 1](#table-1-scalar_id-a-wrongly-typed-argument-to-every-function)):

```
function      input   Rust                           Solidity           verdict
u32_id        badarg  Error(WasmVm, InvalidAction)   U32(4294967295)    sdk failed
i32_id        badarg  Error(WasmVm, InvalidAction)   I32(-1)            sdk failed
bool_id       badarg  Error(WasmVm, InvalidAction)   Bool(true)         sdk failed
address_id    badarg  Error(WasmVm, InvalidAction)   I32(-1)            sdk failed
u64_id        badarg  Error(WasmVm, InvalidAction)   Error(Value, InvalidInput)   both failed
i64_id, u128_id, i128_id: the same as u64_id
```

A struct field behaves the same way under the `wrong` shape: `get_x` answers
`U32(4294967295)` on the Solidity build and fails on the Rust build ([Table 2](#table-2-struct_id-malformed-struct-arguments)). Across all
pairs, 48 `badarg` rows ([Table 3](#table-3-badarg-rows-the-solidity-build-accepts-counted-per-pair)) and 2 `wrong` rows ([Table 2](#table-2-struct_id-malformed-struct-arguments)) show the Solidity build
accepting an argument the Rust build rejected. In `flow`, `struct_mem`, `vec_mem` and
`scalar_math`, more rows fail on both sides, but the Solidity build fails later, for example
by running out of budget in a loop ([Table 4](#table-4-flow-and-vec_mem-calls-that-fail-on-both-sides-with-different-errors)).

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
`put_v09` are the same 160 bytes and differ only in the key constant, `14` against `15`
([Table 9](#table-9-slots-put_v08-against-put_v09)). All four value classes fail on `v09` to `v21`: 52 failed calls ([Table 8](#table-8-slots-results-either-side-of-the-limit)).

**Why it matters.** Fifteen slots is an ordinary contract size, and fewer variables fill it
when they are structs or fixed size arrays. The contract compiles and deploys with no
warning and fails only on the variables past the limit, so a test that uses only the first
few passes. The keys that do work are unusual too: the state sits under `Bool`, `Void` and
`Error` keys, which other tools will not look for.

The workaround is to keep the state within fifteen slots. Grouping scalars into a struct
does not help, since each field takes a slot.

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

In the disassembly, `direct` and `via_storage` differ in exactly three constants ([Table 11](#table-11-location-the-storage-class-passed-to-each-storage-call)): the
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
in three constants, `0` (`Temporary`) against `1`.

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
temporary, `1` persistent, `2` instance, as the last argument of each storage call:

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

The Rust build packs the whole struct in one host call. Solang builds or reads the `Vec`
one element at a time. The imports show the two approaches ([Table 14](#table-14-struct_store-host-functions-imported)):

```
                              sdk   solang
map_new_from_linear_memory    yes    no
vec_new, vec_put, vec_get      no   yes
```

**Evidence.** Host calls per function, for a four field struct ([Table 15](#table-15-struct_store-per-function-metrics)):

```
function      sdk   solang
write_one       2        5
write_all       1       17
read_one        1        4
read_all        1       16
```

Writing four fields costs the Rust build one host call and the Solang build seventeen,
nearly all of them `Vec` element operations. Reading four costs one against sixteen.

A struct inside a stored list is encoded the same way: a `Vec` of `Map`s in Rust, a `Vec`
of `Vec`s in Solidity (`struct_vec`). The storage class annotation works for a struct in
every class (`struct_class`).

At the contract boundary, both sides encode a struct as the same `Map` keyed by field name,
and all 16 `struct_id` calls agree, so a Rust client can call a Solidity contract with a
struct argument. Only the cost differs. Host calls when the call succeeds, for a two field
struct ([Table 16](#table-16-struct_id-calls-per-function-and-host-functions-imported)):

```
function   Rust   Solidity   what Solang does
get_x         1          4   symbol_new, map_get, per field
make          1          5   map_new, then symbol_new, map_put, per field
id            2          9   both of the above
```

Solang creates every key symbol at runtime, including one letter keys that fit a
`SymbolSmall`. `id` costs 31,165 more CPU instructions on the Solang build, and the nested
`id_outer` 45,414 more ([Table 17](#table-17-struct_id-cost-per-call)).

**Why it matters.** Cost, and compatibility of stored data. In storage the two builds use a
different key and a different encoding, so neither contract can read the other's storage.
At the boundary the encoding matches and only the cost differs.

**Recommendation.** None needed for correctness. Building a struct with
`map_new_from_linear_memory` and reading it with `map_unpack_to_linear_memory`, as the Rust
build does, would replace the per field calls with one.

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

**Why it matters.** The two layouts share no keys, so neither contract can read the other's
mapping. Since a Solidity mapping is a single value, every access loads and stores all of
it, and the whole mapping is bounded by the ledger entry size limit. The harness measures
one call on a fresh deploy, so how the cost of either layout grows with the number of keys
was not measured.

**Recommendation.** None needed for correctness. Writing one entry per key, keyed by the
slot and the mapping key, would keep the cost of an access independent of the mapping's
size and remove the entry size limit.

---

### F7: The error path is inlined at every trap site

**Area:** size. **Confidence:** high. **Contracts:** every Solang build.

**What happens.** The Rust build traps with a bare `unreachable` and imports nothing at
all. The Solang build calls the host to log a message at each failure site, then traps
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
total instructions in the module        216      481
```

Roughly 180 of the Solang build's 481 instructions, about 37% of the module, are copies of
that block. It also accounts for the data section: 272 bytes of message strings on the
Solang build of `scalar_id` against none on the Rust build ([Table 22](#table-22-scalar_id-bytes-per-section)).

**Why it matters.** Module size, and with it deployment and instantiation cost. It has no
effect on behaviour.

**Recommendation.** Emit one shared handler and call it. This is the larger of the two size
findings, and the change is local.

---

### F8: A list is held in linear memory rather than by the host

**Area:** encoding, cost. **Confidence:** medium: measured at `n = 1` only. **Contracts:** `contracts/vec_mem`, `contracts/vec_id`.

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

The Solang build imports no vector host functions. A `uint32[] memory` is allocated in the
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
and uses about 65,000 more memory bytes. The harness reports cost per call rather than by
category, so it does not say which part of the call accounts for the CPU difference. What
it does show is what differs: the Rust build imports four vector host functions and keeps
the list on the host, the Solidity build imports none and does the work in its own linear
memory. How either side scales with the length of the list was not measured.

The Solidity module is 3.3 times larger, the widest ratio of any pair, mostly in helper
functions: 57 bytes on the Rust build against 910 on the Solidity build ([Table 25](#table-25-vec_mem-size-and-per-function-metrics)).

A list at the contract boundary is copied the same way (`vec_id`). The Rust build returns
the handle it was given; the Solang build reads every element into linear memory and builds
a new list to return ([Table 26](#table-26-vec_id-per-function-metrics-and-cost)):

```
function   Rust calls   Solidity calls   Rust cpu   Solidity cpu        diff
id                  0                9    227,014        263,115    -36,101
len                 1                6    227,665        254,016    -26,351
sum                 2               11    230,579        256,268    -25,689
```

These are measured with the `small` input, a list of three elements.

**Why it matters.** Cost, for every array, string and bytes value a Solidity contract
handles.

**Recommendation.** None needed for correctness. Passing a list at the boundary through as a
host object, instead of copying it into linear memory and back, would remove the per
element copy. For a one element list built inside a call, linear memory measured cheaper.

---

### F9: A local struct is materialised rather than eliminated

**Area:** cost. **Confidence:** high. **Contracts:** `contracts/struct_mem`.

**What happens.** `build(a)` constructs `Point { x: a, y: a }` and returns `p.x + p.y`.

The Solang build allocates and stores it ([Table 28](#table-28-struct_mem-the-body-of-build)):

```
I32Const 8
Call 1              ; allocate 8 bytes
I32Store            ; write y
I32Store            ; write x
```

The Rust build has no struct. After the tag check the whole body is:

```
LocalGet 0
I64Const 1
I64Shl              ; a + a, as a shift
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

About 16,000 more CPU instructions and 68,000 more memory bytes per call.

**Why it matters.** Cost. A throwaway struct is free in Rust and not in Solidity.

**Recommendation.** None needed for correctness. It has the same cause as F8.

---

### F10: 229 bytes of toolchain metadata ship in every build

**Area:** size. **Confidence:** high. **Contracts:** every Solang build.

**What happens.** Section breakdown of an empty contract with no functions ([Table 29](#table-29-baseline-bytes-per-section-of-an-empty-contract)):

```
section                    sdk  solang
custom:producers             0     142
custom:target_features       0      44
custom:name                  0      43
custom:contractspecv0       15      55
custom:contractenvmetav0    30      30
custom:contractmetav0      230       0
export                      41      26
global                      25       9
everything else              3      29
                          ----    ----
total                      367     403
```

`producers`, `target_features` and `name` record how the module was built: which compiler,
which LLVM features, and internal symbol names. That is 229 bytes, none of it read on
chain.

The Rust build carries 230 bytes of its own, in two `contractmetav0` entries of 147 and 83
bytes. That is why the gap on an empty contract is only 36 bytes.

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
slots                          173      21   a mapping, an array, three structs and arrays
auth                            21       2   one mapping
```

**Why it matters.** Deployment pays one storage write per array, struct and mapping, and a
`persistent` one exists, and pays rent, from deploy instead of from first use. Behaviour is
the same: an unset value reads as zeros on both sides.

**Recommendation.** None needed. Writing these on first use, as Solang does for scalars,
would remove the writes at deploy.

---

### F12: A nested loop is executed rather than reduced to a multiply

**Area:** cost. **Confidence:** medium: the curve is not measured. **Contracts:** `contracts/flow`.

**What happens.** `nested(n)` is `for i in 0..n { for j in 0..n { t += 1 } }` on both sides.

The Rust build does not perform the accumulation. The returned value is computed directly
with one `I32Mul`, and what remains of the loop is an overflow guard that runs `n` times.
The Solang build keeps both loops and executes all n squared iterations.

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

That evidence is indirect. `cost` measures `nested` at `n = 1`, where both builds do one
unit of work, so the quadratic cost itself was not measured.

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

The Rust build pays every time. The Solang build pays for the 64 and 128 bit types and
nothing else.

**Why it matters.** Not directly. It shows where validation happens. Unused code can only
be removed when it has no side effects: a decode that can trap must stay, and a mask and
shift that cannot trap is removed.

`u32`, `i32`, `bool` and `address` cost nothing here, so those four are not validated. F1
shows the effect at runtime.

**Recommendation.** None. This finding supports F1.

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
   field at a time and creates every key symbol at runtime. This is the largest cost
   difference measured, 9,000 to 45,000 CPU instructions a call (F5). The Rust build uses
   `map_new_from_linear_memory` and `map_unpack_to_linear_memory` for the same work.

5. **Store a mapping as one entry per key.** A whole mapping in one ledger entry means
   every access loads and stores all of it, and bounds the mapping by the entry size limit
   (F6).

6. **Pass a list at the boundary through as a host object.** Copying it into linear memory
   and back costs 26,000 to 36,000 CPU instructions a call (F8).

7. **Emit one shared error handler and strip the build metadata sections.** F7 and F10
   account for most of the size gap, and neither affects behaviour.

8. **Consider warning on a state variable with no storage annotation.** F4 is documented
   behaviour, not a defect, but it is an easy mistake when moving between the two
   toolchains.

### For the Rust SDK

No finding calls for a change to the Rust SDK.

---

## Appendix A: Contract Inventory

| Pair | Exported functions | What it isolates |
|---|---|---|
| `baseline` | 0 | fixed cost of being a contract |
| `scalar_id` | 16 | value decode and encode at the boundary |
| `scalar_math` | 7 | one addition per numeric type |
| `scalar_store` | 8 | one storage round trip per type |
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

---

## Appendix B: Harness Output

Each table is antlion's own output, cut to the rows the report uses; `...` marks rows left
out. The first line of each is the command that produces it. Where the command has
`<pair>`, it is run once per pair and the table gathers one row from each. Run
`antlion --build contracts/<name>` for each pair first.

### Table 1: `scalar_id`, a wrongly typed argument to every function

```
$ antlion --compare scalar_id --report returns

function        input  sdk                      solang                   verdict
i32_id          badarg [err] HostError: Error…  I32(-1)                  sdk failed
i64_id          badarg [err] HostError: Error…  [err] HostError: Error…  both failed
u32_id          badarg [err] HostError: Error…  U32(4294967295)          sdk failed
u64_id          badarg [err] HostError: Error…  [err] HostError: Error…  both failed
bool_id         badarg [err] HostError: Error…  Bool(true)               sdk failed
i128_id         badarg [err] HostError: Error…  [err] HostError: Error…  both failed
i32_id2         badarg [err] HostError: Error…  I32(-1)                  sdk failed
i64_id2         badarg [err] HostError: Error…  [err] HostError: Error…  both failed
u128_id         badarg [err] HostError: Error…  [err] HostError: Error…  both failed
u32_id2         badarg [err] HostError: Error…  U32(4294967295)          sdk failed
u64_id2         badarg [err] HostError: Error…  [err] HostError: Error…  both failed
bool_id2        badarg [err] HostError: Error…  Bool(true)               sdk failed
i128_id2        badarg [err] HostError: Error…  [err] HostError: Error…  both failed
u128_id2        badarg [err] HostError: Error…  [err] HostError: Error…  both failed
address_id      badarg [err] HostError: Error…  I32(-1)                  sdk failed
address_id2     badarg [err] HostError: Error…  I32(-1)                  sdk failed
...
note: 66 call(s) ran on both sides, 0 disagreed
```

### Table 2: `struct_id`, malformed struct arguments

```
$ antlion --compare struct_id --report returns

function        input  sdk                      solang                   verdict
id              extra  Map(Some(ScMap(VecM([S…  Map(Some(ScMap(VecM([S…  same
id              short  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Object, MissingValue) DebugInfo not available
id              wrong  [err] HostError: Error…  Map(Some(ScMap(VecM([S…  sdk failed
id              asvec  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
make            badarg [err] HostError: Error…  Map(Some(ScMap(VecM([S…  sdk failed
get_x           extra  U32(1)                   U32(1)                   same
get_x           short  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Object, MissingValue) DebugInfo not available
get_x           wrong  [err] HostError: Error…  U32(4294967295)          sdk failed
get_x           asvec  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
id_outer        extra  Map(Some(ScMap(VecM([S…  Map(Some(ScMap(VecM([S…  same
id_outer        short  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Object, MissingValue) DebugInfo not available
id_outer        wrong  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
id_outer        asvec  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
note: 19 call(s) ran on both sides, 0 disagreed
```

### Table 3: `badarg` rows the Solidity build accepts, counted per pair

```
$ antlion --compare <pair> --report returns

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

```
$ antlion --compare flow vec_mem --report returns

function        input  sdk                      solang                   verdict
nested          large  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
nested          badarg [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
push            large  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, IndexBounds) DebugInfo not available
push            badarg [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(WasmVm, IndexBounds) DebugInfo not available
```

### Table 5: `auth`, with and without authorization

```
$ antlion --compare auth --report returns

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
need_auth       badarg [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
auth_store      small  [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      large  [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      bound  [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      neg    [err] HostError: Error…  [err] HostError: Error…  both failed
auth_store      authed U32(1)                   U32(1)                   same
auth_store      badarg [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Value, InvalidInput) DebugInfo not available
note: 7 call(s) ran on both sides, 0 disagreed
```

### Table 6: `storage`, the storage class and key of each variable

```
$ antlion --compare storage --report ledger

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

```
$ antlion --compare slots --report ledger

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

```
$ antlion --compare slots --report returns

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
put_v09         badarg [err] HostError: Error…  [err] HostError: Error…  both failed
...
note: 36 call(s) ran on both sides, 0 disagreed
```

### Table 9: `slots`, `put_v08` against `put_v09`

```
$ antlion --compare slots --report functions opcodes --dump

                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
put_v08           8   160  -152 │     4    66   -62 │    0    1   -1 │    0    3   -3 │    1    5   -4
put_v09           8   160  -152 │     4    66   -62 │    0    1   -1 │    0    3   -3 │    1    5   -4

Solidity side of --dump, put_v08:
I64Const { value: 14 }
Call { function_index: 3 }
I64Const { value: 14 }
Call { function_index: 0 }

Solidity side of --dump, put_v09:
I64Const { value: 15 }
Call { function_index: 3 }
I64Const { value: 15 }
Call { function_index: 0 }
```

### Table 10: `location`, each alias and its direct twin

```
$ antlion --compare location --report returns

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
```

### Table 11: `location`, the storage class passed to each storage call

```
$ antlion --compare location --report opcodes --dump

Solidity side, direct:
I64Const { value: 2 }
Call { function_index: 0 }
I64Const { value: 2 }
Call { function_index: 1 }
I64Const { value: 2 }
Call { function_index: 4 }

Solidity side, via_storage:
I64Const { value: 1 }
Call { function_index: 0 }
I64Const { value: 1 }
Call { function_index: 1 }
I64Const { value: 1 }
Call { function_index: 4 }
```

### Table 12: `storage`, cost per call

```
$ antlion --compare storage --report cost

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

```
$ antlion --compare struct_store --report ledger

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

```
$ antlion --compare struct_store --report imports

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

```
$ antlion --compare struct_store --report functions

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

```
$ antlion --compare struct_id --report functions imports

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

```
$ antlion --compare struct_id --report cost

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

```
$ antlion --compare mapping --report ledger

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

```
$ antlion --compare mapping --report cost

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

```
$ antlion --compare flow --report opcodes --dump

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
Call { function_index: 0 }
Drop
Unreachable
```

### Table 21: `flow`, instruction histogram

```
$ antlion --compare flow --report opcodes

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

```
$ antlion --compare scalar_id --report sections

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

```
$ antlion --compare vec_mem --report imports

import   function                                         sdk   solang
v.1      vec_get                                            ✓        —
v.3      vec_len                                            ✓        —
v.6      vec_push_back                                      ✓        —
v._      vec_new                                            ✓        —
x._      log_from_linear_memory                             —        ✓
total                                                       4        1
```

### Table 24: `vec_mem`, cost per call

```
$ antlion --compare vec_mem --report cost

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

```
$ antlion --compare vec_mem --report size functions

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

```
$ antlion --compare vec_id --report functions cost

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

```
$ antlion --compare struct_mem --report functions opcodes cost

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

```
$ antlion --compare struct_mem --report opcodes --dump

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
Call { function_index: 1 }
LocalTee { local_index: 1 }
I32Const { value: 4 }
I32Add
LocalGet { local_index: 0 }
I64Const { value: 32 }
I64ShrU
I32WrapI64
LocalTee { local_index: 2 }
I32Store { memarg: MemArg { align: 2, max_a…
LocalGet { local_index: 1 }
LocalGet { local_index: 2 }
I32Store { memarg: MemArg { align: 2, max_a…
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
Call { function_index: 0 }
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
Call { function_index: 0 }
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

```
$ antlion --compare baseline --report sections

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

```
$ antlion --compare <pair> --report functions

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

```
$ antlion --compare flow --report functions returns

                    bytes       │      instrs       │     locals     │     depth      │     calls
function        sdk   sol  diff │   sdk   sol  diff │  sdk  sol diff │  sdk  sol diff │  sdk  sol diff
nested           84   258  -174 │    50   123   -73 │    3    5   -2 │    3    7   -4 │    0    4   -4

function        input  sdk                      solang                   verdict
nested          small  U32(1)                   U32(1)                   same
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
nested          large  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
nested          bound  [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
nested          neg    U32(0)                   U32(0)                   same
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
nested          badarg [err] HostError: Error…  [err] HostError: Error…  both failed
                  sdk    [err] HostError: Error(WasmVm, InvalidAction) DebugInfo not available
                  solang [err] HostError: Error(Budget, ExceededLimit) DebugInfo not available
```

### Table 32: `scalar_id`, bytes per function, with one and two arguments

```
$ antlion --compare scalar_id --report functions

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

```
$ antlion --compare <pair> --report size

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

```
$ antlion --compare <pair> --report returns

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

```
$ antlion --compare <pair> --report cost

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
