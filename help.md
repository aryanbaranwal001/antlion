antlion: a differential harness for Soroban contracts built two ways:
the Rust soroban-sdk, and Solidity through Solang.

USAGE

    antlion --build <contract>...
    antlion --compare <name>... --report <report>... [--json] [--bare] [--detail | --dump] [--jobs <n>]
    antlion --interactive
    antlion --help
    antlion --version

COMMANDS

    --build <contract>... compile each <contract>/{sdk,solang} into out/<name>/
    --compare <name>...   diff both builds of each contract, in the order given
    --interactive         open a terminal UI that suggests each word of a command as
                          you type it, runs it, and shows its output
    --help                print this help
    --version             print the version

    --build takes a path to each pair directory, relative to the working directory
    unless absolute. There is no fallback: a bare `flow` means ./flow. <name> is
    the last component of that path, and names the output directory.

OPTIONS

    --report <report>...  reports to run; required, pass `all` for every one
    --json                print one JSON document in place of the text reports
    --bare                print only the comparison: no notes, and no lines that
                          explain how a report ran
    --detail              opcodes: break each bucket into its opcodes; cost: add
                          what each cost type charged
    --dump                opcodes only: side-by-side disassembly
    --jobs <n>            threads for the reports that call contracts; default 4,
                          or fewer on a machine with fewer cores

REPORTS

    size        total wasm byte size
    sections    per-section byte breakdown
    imports     which host functions each side imports
    interface   every exported function's signature
    meta        the protocol each build needs, its metadata text, and every type,
                error and event its spec declares
    returns     what each function answers, side by side
    cost        metered cost of calling each function, module cached as on chain,
                split into instantiate and run, each call measured 3 times, and
                the cost to fail of every call that fails at any input
    scaling     how the cost of each function grows with its size, 1 to 64
    ledger      what each function leaves in storage: class, key and value
    access      which ledger entries each call reads and writes, their sizes, and
                every storage host call it makes, run as the network runs it
    trace       every host call each call makes, in order, with its arguments,
                counted and lined up side by side
    ttl         how long each entry lives, the rent of the deploy and of each
                call, and what each side does once its entries expire
    sequence    every function called in order on one deploy, twice over
    events      the events each function emits, and the diagnostics of a failure
    opcodes     opcode histogram, bucketed by kind
    functions   per-function size and complexity
    layout      declared memory, globals and tables
    all         every report above, in that order

INPUTS

    returns calls every function once per input row. cost, ledger, access,
    trace, ttl, sequence and events use `small`. scaling sets every integer argument to n and gives every
    list n elements, for n from 0 to 64.

    small       values that ride inline in the 56 bit Val payload
    large       values past the payload, passed as object handles
    bound       the largest inline value, and type extremes
    neg         negatives, zeros and empties

    Functions that take a struct also get, with every other argument at `small`:

    extra       the struct plus a field it does not declare
    short       the struct without its last field
    wrong       its first field as the wrong type
    asvec       the right values as a vec rather than a map

    Functions with any other argument also get:

    badarg      that argument as the wrong type: i32(-1), or u32::MAX for an i32

    Every row above runs with no authorization. Functions that take an address
    also get:

    authed      `small`, with every authorization the call asks for granted

    Structs are built the way soroban-sdk encodes them; a vec holds 3, 20, 1 or 0
    elements for small, large, bound and neg.

BUNDLED CONTRACTS

    under contracts/ in this repository

    baseline   scalar_id   scalar_math   scalar_store   storage   flow
    struct_store   struct_mem   struct_id   struct_class   struct_vec
    vec_mem   vec_id   mapping   location   slots   auth   errors   events
    numeric   bytes_id   ttl

EXAMPLES

    antlion --build contracts/scalar_id
    antlion --build ../elsewhere/pairs/mypair
    antlion --build contracts/scalar_id contracts/flow
    antlion --compare scalar_id --report all
    antlion --compare scalar_id scalar_math --report size returns
    antlion --compare flow --report opcodes --dump
    antlion --compare scalar_id --report all --json
    antlion --compare scalar_id --report all --bare

NOTES

    --compare reads out/<name>/{sdk,solang}.wasm, so run --build first.
    --detail applies to the opcodes and cost reports, --dump to opcodes only.
    --json, --bare, --detail, --dump and --jobs <n> come after the report names, in
    any order. The results are the same for every --jobs value.
    --bare changes only the text reports; the JSON is the same with or without it.
    With --json, --detail changes nothing: the opcodes JSON always lists every
    opcode. --dump has no JSON form, so opcodes is left out and named under
    `no_json`.
    returns prints both values in full when they differ, and both errors in full
    when both sides fail with different errors.
    Exit status: 0 on success, 1 when something failed, 2 for a wrong command.
    A difference between the two builds is a result, not a failure, and exits 0.
