# antlion user manual

This manual covers every command, option and report. For installing antlion and setting up
a contract pair, see the [README](README.md).

1. [Commands](#commands)
   - [Build](#build)
   - [Compare](#compare)
   - [Interactive](#interactive)
   - [Help and version](#help-and-version)
2. [Reading the output](#reading-the-output)
   - [JSON output](#json-output)
3. [Reports](#reports)
   - [size](#size)
   - [sections](#sections)
   - [imports](#imports)
   - [interface](#interface)
   - [meta](#meta)
   - [returns](#returns)
   - [cost](#cost)
   - [scaling](#scaling)
   - [ledger](#ledger)
   - [access](#access)
   - [trace](#trace)
   - [ttl](#ttl)
   - [sequence](#sequence)
   - [events](#events)
   - [opcodes](#opcodes)
   - [functions](#functions)
   - [layout](#layout)

## Commands

### Build

```
antlion --build <path to pair>...
```

Builds both sides of a pair: the Rust side with `stellar contract build` and the Solidity
side with `solang compile --target soroban`.

Give one or more pairs. Each pair is built on its own, and a pair is written to `out/`
only when both of its builds succeed. If one pair fails, the others still build, and
antlion exits with 1 at the end.

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
antlion --compare <name>... --report <report>... [--json] [--detail | --dump] [--jobs <n>]
```

Compares the two builds of a pair. It takes the pair's **name**, not its path, and reads
`out/<name>/`, so build the pair first.

`--report` is required. Give one or more report names, or `all` for every report. You can
also give several pairs at once. Pairs and reports run in the order you write them. If a
pair cannot be compared, for example because it has no build, the others still run, and
antlion exits with 1 at the end.

```
antlion --compare scalar_id --report all
antlion --compare scalar_id scalar_math --report size returns
antlion --compare flow --report opcodes --dump
antlion --compare scalar_id --report all --json
```

`--json`, `--detail`, `--dump` and `--jobs <n>` come after the report names, in any order.
`--json` prints the same results as JSON; see [JSON output](#json-output). `--detail`
affects the `opcodes` and `cost` reports, and `--dump` only `opcodes`; see
[opcodes](#opcodes) and [cost](#cost).

`--jobs <n>` sets how many threads the reports that call contracts use: `returns`, `cost`,
`scaling`, `ledger`, `access`, `trace`, `ttl`, `sequence` and `events`. The default is 4, or fewer on a machine with fewer
cores. More threads are faster and use more memory, and `--jobs 1` uses the least memory.
The results are the same for every value. If the system cannot start the threads, antlion
prints a warning and runs every call on one thread.

### Interactive

```
antlion --interactive
```

Opens a terminal UI where you type a command, see the words that can come next, run it,
and read and search its output. Start it from the project root, where `out/` and
`contracts/` are. It needs a terminal: in a pipe or a script it exits with 2, so use
`--build` and `--compare` there.

The UI runs antlion itself for each command, with the arguments you typed, so anything you
do in the UI you can also do in a shell or a script. `y` copies the command.

```
┌ Command ──────────────────────────────┐┌ Suggestions (4) ────────┐
│antlion --compare flow --report co     ││ cost                    │
└ unknown report `co` ... ──────────────┘│ opcodes                 │
┌ Output: antlion --compare flow ... ───┐│ sections                │
│ ...                                   │└─────────────────────────┘
│                                       │┌ report cost ────────────┐
│                                       ││metered cost of calling  │
│                                       ││each function, ...       │
└───────────────────────────────────────┘└─────────────────────────┘
 done  0.9 s  150 lines  tab 1 of 1
 Enter add  ^R run  Tab panel  ↑↓ move  ^F find  ^P palette  F1 help  ^C quit
```

**The command line.** It starts with `antlion`, which you cannot change, and you type the
rest. When the CLI would refuse what you typed, the lower border shows the reason in red,
and `Ctrl+R` does not run it.

**The suggestions.** The pane on the right lists every word that can come at the cursor,
in the order the CLI takes them: a command first, then the pairs after `--compare` (or the
pair folders after `--build`), then `--report`, then the reports, then `--json`, `--detail`,
`--dump` and `--jobs`. A word you already typed is not suggested again, and `--detail` and
`--dump` never go together. The word you are typing filters the list with fuzzy search:
the letters must come in order, with gaps allowed, so `stc` finds `struct_class`. The best
match comes first. `Enter` or `Tab` puts in the suggestion under the cursor and a space
after it, so the next words show at once.

**The details.** The pane under the suggestions shows the help for the suggestion under the
cursor: what a command or a flag does, what a report measures and which inputs it calls
each function with, or the build state of a pair and the signature of each function in
both builds. With no suggestion, it shows the help for the last flag you typed.

**The output.** Each run has its own tab.

**No modes.** The letters you type always go into the command line. `Tab` moves the focus
between the suggestions, the output and the details, and the moving keys act on the panel
that has the focus.

**The keys.** Press `F1` for every key. The most used:

| Key | Action |
|---|---|
| `Enter` | Put in the suggestion under the cursor |
| `Ctrl+R` | Run the command |
| `Tab`, `Shift+Tab` | Next panel, panel before |
| `Up`, `Down`, `PgUp`, `PgDn` | Move in the panel that has the focus |
| `Ctrl+Home`, `Ctrl+End` | Start, end of the panel; in the output, `Ctrl+End` follows new output |
| `Shift+Left`, `Shift+Right` | Scroll the output to the side |
| `Ctrl+W`, `Ctrl+U` | Delete the word before the cursor, or all text before it |
| `Ctrl+F` | Search the output |
| `Ctrl+N`, `Ctrl+B` | Next match, match before |
| `Ctrl+PgUp`, `Ctrl+PgDn` | Tab before, next tab |
| `Ctrl+S` | Save the output of the tab to a file |
| `Ctrl+Y` | Copy the command |
| `Ctrl+P` | Command palette |
| `F1` | Help |
| `Ctrl+C` | Cancel the run; with no run, quit |
| `Ctrl+Q` | Quit at once, and stop the run |
| `Esc` | Close a box or a prompt |

The output search ignores case unless you type a capital letter.

**The history.** The palette entry "history of runs" lists your last runs. Type part of one
and press `Enter`, and it goes into the command line, ready to change or to run.

**The palette.** `Ctrl+P` opens a box with every action. Type part of a name and press
`Enter`. Some actions are only in the palette:

- save the command as a preset, and load a preset into the command line;
- the doctor, which checks that `solang` and `stellar` are on `PATH`, that `out/` is there,
  and which pairs are stale or not built, with a fix for each problem;
- the `scripts/check.sh` tasks: `build`, `diff`, `save`, `wasm` and `time`. `save` asks
  first, because it writes over the golden output.

**Files.** The UI keeps two files:

| File | What it holds |
|---|---|
| `~/.local/state/antlion/history.jsonl` | The last 200 antlion runs: time, arguments, exit code and duration |
| `~/.config/antlion/config.toml` | Your presets |

`$XDG_STATE_HOME` and `$XDG_CONFIG_HOME` move them, and `ANTLION_CONFIG_DIR` puts both in
one folder. The config file looks like this:

```toml
[presets."cost of numeric pairs"]
args = ["--compare", "numeric", "--report", "cost"]
```

When the UI saves a preset, it keeps the rest of the file, comments too.

The UI is a cargo feature, `interactive`, on by default. `cargo build --no-default-features`
leaves it out, and then `--interactive` exits with 1.

### Help and version

```
antlion --help
antlion --version
```

`--help` prints a short reference of every command, option and report. `--version` prints
the version you have.

antlion exits with 0 on success, 1 when something failed, such as a missing build or a
failed compile, and 2 for a wrong command. An error is one line on stderr that starts
with `[err]`. A difference between the two builds is a result, not a failure, so it exits
with 0.

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

### JSON output

With `--json`, `--compare` prints one JSON document and nothing else, for scripts such as
CI. The numbers are the same as in the text reports.

```json
{
  "schema_version": 10,
  "antlion": "v0.1.1",
  "pairs": [
    {
      "name": "scalar_id",
      "size": { "sdk": 2845, "solang": 3734, "diff": -889 }
    }
  ],
  "no_json": [],
  "failed": []
}
```

- `schema_version` goes up when the shape of the document changes.
- `pairs` has one object per pair, in the order given. Each object has the pair's `name`
  and one key per report requested, in the order requested.
- Each value has an `sdk` and a `solang` side. A `diff` is given only where the text report
  shows one, and is `sdk` minus `solang`, as a plain number.
- A cell the text shows as `—` is `null`.
- Numbers are raw: memory pages are a page count, not `16 (1 MiB)`.
- Returned values and errors are strings, in full. The text cuts them to fit a column.
- `returns` and `sequence` count the calls that disagreed in `disagreed`, and the calls
  that failed on both sides with different errors in `errors_differ`.
- `returns`, `cost`, `scaling`, `ledger`, `access`, `trace`, `ttl`, `sequence` and `events` have `no_spec`, true when
  the contract has no spec and nothing was called, and `skipped`, the functions whose
  argument types antlion cannot build. `scaling` also has `unsized`, the functions with no
  argument that can carry a size.
- `cost` always has `types`, what each cost type charged, with or without `--detail`.
  Each side also has `failed`, the error code of a call that failed, and each function has
  `fails`, the calls past `small` that failed on one side or both.
- `no_json` names any report asked for that has no JSON form. Only `opcodes` with `--dump`
  has none; it is left out of every pair.
- `failed` lists each pair that could not be compared, with its `name` and the `error`.
  Such a pair is not in `pairs`, and the exit status is 1.

With `--json`, `--detail` changes nothing: the `opcodes` JSON always lists every opcode,
and the `cost` JSON always has every cost type.

## Reports

| Report | Answers |
|---|---|
| [`size`](#size) | How big is each module? |
| [`sections`](#sections) | Which part of the module holds the bytes? |
| [`imports`](#imports) | Which host functions does each side use? |
| [`interface`](#interface) | Do both sides declare the same functions? |
| [`meta`](#meta) | What protocol does each side need, and what types, errors and events does it declare? |
| [`returns`](#returns) | Do both sides give the same answers? |
| [`cost`](#cost) | What does one call cost to run, and how much of it is the function itself? |
| [`scaling`](#scaling) | How does the cost grow with the size of the input or the stored data? |
| [`ledger`](#ledger) | Do both sides store the same data under the same keys? |
| [`access`](#access) | Which ledger entries does each call touch, and how many storage calls does it make? |
| [`trace`](#trace) | Which host functions does each call use, in which order, and with which arguments? |
| [`ttl`](#ttl) | How long does each entry live, what rent does each side pay, and what happens when an entry expires? |
| [`sequence`](#sequence) | Do both sides agree when each call builds on the calls before it? |
| [`events`](#events) | Do both sides emit the same events, and what does a failure say? |
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

### meta

Decodes the three custom sections that describe a contract. If a section appears more than
once, its parts are read in order, as one.

- **`contractenvmetav0`**: the host protocol version the build was made for, and its pre
  release number. The network does not run a contract made for a newer protocol than its
  own.
- **`contractmetav0`**: key and value text, such as the compiler and SDK versions. A key
  that only one side has shows `—` on the other.
- **`contractspecv0`**: how many entries of each kind the spec declares: function, struct,
  union, enum, error enum and event. Every kind but functions is then listed by name, with
  its cases for a union, an enum or an error enum. Functions are compared by
  [interface](#interface).

The last line says whether both sides declare the same types, errors and events, in any
order.

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
| `both failed` | both builds failed, with the same error |
| `errors differ` | both builds failed, each with a different error; both are printed in full underneath |

For the malformed and `badarg` rows, a failure on both sides is the expected result, and
`sdk failed` means the Solidity build accepted what the Rust build rejected. The error is
compared by its code, such as `Error(Contract, #1)`, which is what a caller of the
contract sees. `errors differ` means a caller sees a different reason for the failure on
each side: one side may report its own contract error where the other only traps, or one
side may have accepted the input and failed later for another reason.

The last lines count the calls that ran on both sides, how many of them disagreed, and how
many failed on both sides with different errors.

### cost

The CPU instructions and memory bytes charged for one call of each function, on a fresh
host, with `small` arguments.

The module is put in the host's module cache before the call. The network keeps every live
contract parsed (CAP-0065, protocol 23), so a real call pays to set up the module but not
to parse it, and this report measures the same.

Two costs belong to the module, not to one function, and come first:

- **module parse** is what parsing the module costs. The network charges it once, at
  upload, so no call pays it.
- **module instantiate** is what every call pays before its function runs: making the
  cached module ready, with its linear memory. antlion measures it as a call to a function
  the module does not export. The host makes the module ready, then fails to find the
  function.

For each function, `cpu insns` is the whole call. `instantiate` is the module's instantiate
cost, and `run` is the rest: what the function itself used. Most of a small call is
instantiate, so `run` shows the difference that the code makes.

With `--detail`, each block also lists what each cost type (`ContractCostType`, the host's
names for what it charges) charged on each side, such as `WasmInsnExec` for the contract's
own instructions and `MemAlloc` for memory. A type's memory is what it charged. The total
memory is the peak, which is lower when memory was given back during the call.

A call that fails is measured too: the host charges for the work done up to the failure.
Its block starts with `[err] ... call failed`, its error code, and the table below is its
cost to fail. At the end, a **cost to fail** table lists every call that failed on one
side or both, at `small` and at `large`, `bound` and `neg`. For each it gives the `run` cost
of each side and whether that side failed (`failed`), answered (`ok`) or could not be
measured (`err`).

Every call runs 3 times, each time on a fresh host. The metering must give the same cost
every time, because every node on the network must agree on it. The last note says so, and
a `[warn]` line names any call whose cost changed.

### scaling

How the cost of each function grows with its size. Size `n` means that every integer
argument is `n` and every list has `n` elements. Every other argument is `small`.

Each function gets one deploy on each side, with every authorization granted. It is called
at every size from 0 to 64, in order, on that deploy. So each call also sees what the calls
before it wrote: at size `n`, a function that writes one key per call finds `n` keys
already there. This is how the report measures the cost of a growing mapping. Each call gets
a full budget, and its cost is the call alone.

The table shows sizes 1, 2, 4, 8, 16, 32 and 64. The last line gives the growth on each
side:

- `flat`: the CPU from size 4 to size 64 grows by less than 1%.
- `n^k`: the power of `n` that fits the growth from size 4 to 16 to 64. `n^1.0` is
  linear, and `n^2.0` is quadratic.
- `irregular`: the cost went down between two of those sizes.
- `n/a`: one of those calls failed.

A function with no integer or list argument is skipped.

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

### access

Calls every function once on a fresh deploy, with `small` arguments, the way the network
runs a transaction. First the call runs in recording mode, as a simulation does, which finds
its footprint (the ledger entries it may read and write) and its authorizations. Then it
runs again in enforcing mode with that footprint, and antlion records every storage host
call it makes. An address argument is the transaction's source account, so its
authorization needs no signature.

The first table has one row per side:

| Column | Means |
|---|---|
| `read` | ledger entries the call only reads, including the contract code and instance |
| `write` | ledger entries the call writes |
| `bytes` | the bytes it writes, which the network charges for |
| `result` | what the call returned, or its error |

Then, for each side:

- `wrote` names each entry written and its size before and after, in bytes. Instance
  storage is one entry, the contract instance, so a write to any instance key rewrites all
  of it.
- `called` counts each storage host call, such as `get_contract_data`, with its storage
  class: `instance`, `persistent` or `temporary`.

### trace

Every host call that each call makes, in order, with its arguments. The run is the one
`access` makes: once in recording mode, then again in enforcing mode with a trace of every
host call.

For each function there are three parts:

1. A table of each host function and how many times each side called it, with the total
   and what the call returned.
2. The two traces in order, lined up by host function name so that the longest run of
   names that both share stays together. Each line starts with a mark:

   | Mark | Means |
   |---|---|
   | `=` | the same host function with the same arguments on both sides |
   | `~` | the same host function with other arguments; both lines are shown |
   | `-` | a call only the sdk side makes |
   | `+` | a call only the solang side makes |

3. A last line, `trace: same`, or `trace: differs` with how many lines of each kind.

An argument is shown as the host renders it, cut to 40 characters; the JSON has it in
full. An object argument is a handle, such as `Vec(obj#3)`, so two equal objects can show
different numbers.

### ttl

How long each ledger entry lives, the rent each side pays, and what each side does once its
entries expire. The rent is given in ledger bytes, as the host meters it: one byte kept live
for one ledger. The rent fee is in proportion to it.

The report has three parts:

1. **Deploy.** The rent of uploading the code, creating the instance and running the
   constructor, and the part of it that the constructor's own writes add.
2. **Each call.** Every function runs once on a fresh deploy at ledger 0, with `small`
   arguments and every authorization granted. For each side, a table lists every entry the
   call created or whose lifetime it changed: `live until` is the last ledger it was live in
   before the call, or `new`, and `now` is the same after the call. At ledger 0 that is also
   the entry's TTL. A line per side then gives the rent the call pays and its answer.
3. **Lifetime.** Each side is deployed once, and every function is called in spec order.
   Then both ledgers move to one past the first entry, on either side, that expires, and every
   function is called again. This happens three times, and an entry that has expired once is
   not waited for again. Each round lists the ledger, the entries that have expired on each
   side, and each function's answer on both sides.

The host used here behaves as a simulation does since protocol 23: an expired temporary
entry is gone, and an expired persistent entry, or the instance and code, is restored when a
call touches it.

### sequence

Every other report that calls a contract starts each call on a fresh deploy, so no call
sees what an earlier call wrote. This one deploys each side once and calls every function
on that deploy, in the order of the spec, with `small` arguments and every authorization
granted. Then it does the full round a second time, so the second round reads what the
first one wrote. For example, `get(1)` answers `0` in the first round and `1` in the
second, after `put(1, 1)`.

Each call gets a full budget, as it would in its own transaction.

The two sides store their data in different layouts, which the `ledger` report shows, so
their storage is never equal and is not compared. At each step two things must agree: the
value returned, and whether the call changed the storage. The `changed` column says which
sides did: `both`, `none`, `sdk` or `solang`.

| Verdict | Means |
|---|---|
| `same` | both sides returned the same value, and both or neither changed the storage |
| `DIFFER` | both sides returned, with different values |
| `changes differ` | the same value, but only one side changed the storage |
| `sdk failed` | only the Rust build failed |
| `solang failed` | only the Solidity build failed |
| `both failed` | both builds failed, with the same error |
| `errors differ` | both builds failed, each with a different error; both are printed in full underneath |

On a step that is not `same`, both answers are printed in full, with the storage each side
holds after the step.

A failed call writes nothing. So after a step that failed on one side only, the two sides
hold different data, and later steps can disagree for that reason alone. When this
happens, the last lines say so.

In `--json`, each step has its `step` and `round` numbers, the `function`, the `verdict`,
and for each side the answer (`ok` or `err`), `changed`, and the `storage` after the step.
As in `returns`, `errors_differ` counts the steps that failed on both sides with different
errors.

### events

Calls every function once on a fresh deploy, with `small` arguments and every authorization
granted, as `ledger` does, and lists the events of the call on each side. The first row of
each side is the `result` of the call: the value, or the error code.

| Kind | Means |
|---|---|
| `contract` | an event the contract emitted, with its topics and its data |
| `log` | a diagnostic message, written by the contract or by the host |
| `error` | a diagnostic the host records for an error, with the error code as a topic |

Only `contract` events are compared. Each function ends with one of:

| Line | Means |
|---|---|
| `events: same` | both sides emitted the same events, with the same topics and data, in the same order |
| `events: differ` | the events differ in number, topics or data |
| `events: none emitted` | neither side emitted a `contract` event |
| `events: not compared, a deploy failed` | one side could not be deployed |

`log` and `error` come from the host's diagnostic mode, which is switched on after the
deploy, so only the call's own diagnostics are shown. They are where a failure says why it
failed. The host also records a `fn_call` and a `fn_return` for every call; they are the
same on both sides, so they are left out.

A `*` marks an event from a call that failed. The network keeps no `contract` event from a
failed call, only its diagnostics.

In `--json`, each function has `events`, the verdict, and for each side the `result` and
every event with its `kind`, `topics`, `data` and `failed_call`.

### opcodes

A count of the instructions in every function, grouped by kind. Imported host functions
have no code of their own, so they are not counted.

| Option | Shows |
|---|---|
| none | the count for each kind |
| `--detail` | each kind broken down into its instructions, plus any used by one side only |
| `--dump` | the full code of every function in two columns, instead of the counts |

Both options only work with `opcodes`. With `--json`, `--detail` changes nothing and `--dump`
leaves `opcodes` out; see [JSON output](#json-output).

In `--dump`, the two columns are independent: each side is listed top to bottom in its own
order, and the shorter one is padded. Nothing is lined up between them. Exported functions
come first, grouped by name, then internal functions. Instructions use their raw
WebAssembly names, such as `LocalGet` for `local.get`. A call is written `Call N`, and a
call to a host function also names it, for example `Call 3 ; put_contract_data`.

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
