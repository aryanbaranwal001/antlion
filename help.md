antlion: a differential harness for Soroban contracts built two ways:
the Rust soroban-sdk, and Solidity through Solang.

USAGE

    antlion --build <contract>
    antlion --compare <name>... --report <report>... [--detail | --dump]
    antlion --help

COMMANDS

    --build <contract>    compile <contract>/{sdk,solang} into out/<name>/
    --compare <name>...   diff both builds of each contract, in the order given
    --help                print this help

    --build takes a path to the pair directory, relative to the working directory
    unless absolute. There is no fallback: a bare `flow` means ./flow. <name> is
    the last component of that path, and names the output directory.

OPTIONS

    --report <report>...  reports to run; required, pass `all` for every one
    --detail              opcodes only: break each bucket into its opcodes
    --dump                opcodes only: side-by-side disassembly

REPORTS

    size        total wasm byte size
    sections    per-section byte breakdown
    imports     which host functions each side imports
    interface   every exported function's signature
    returns     what each function answers, side by side
    cost        metered cost of calling each function, module cached as on chain
    ledger      what each function leaves in storage: class, key and value
    opcodes     opcode histogram, bucketed by kind
    functions   per-function size and complexity
    layout      declared memory, globals and tables
    all         every report above, in that order

INPUTS

    returns calls every function once per input row. cost and ledger use `small`.

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
    vec_mem   vec_id   mapping   location   slots   auth

EXAMPLES

    antlion --build contracts/scalar_id
    antlion --build ../elsewhere/pairs/mypair
    antlion --compare scalar_id --report all
    antlion --compare scalar_id scalar_math --report size returns
    antlion --compare flow --report opcodes --dump

NOTES

    --compare reads out/<name>/{sdk,solang}.wasm, so run --build first.
    --detail and --dump apply to the opcodes report only, and must come last.
    returns prints both values in full when they differ, and both errors in full
    when both sides fail with different errors.
