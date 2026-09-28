antlion — differential harness for Soroban contracts built two ways:
the Rust soroban-sdk, and Solidity through Solang.

USAGE

    cargo run -- --build <contract>
    cargo run -- --compare <name>... --report <report>... [--detail | --dump]
    cargo run -- --help

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
    cost        metered cost of calling each function, on a fresh host
    opcodes     opcode histogram, bucketed by kind
    functions   per-function size and complexity
    layout      declared memory, globals and tables
    all         every report above, in that order

BUNDLED CONTRACTS

    under contracts/ in this repository

    baseline   scalar_id   scalar_math   scalar_store   storage
    flow   struct_store   struct_mem   vec_mem   location

EXAMPLES

    cargo run -- --build contracts/scalar_id
    cargo run -- --build ../elsewhere/pairs/mypair
    cargo run -- --compare scalar_id --report all
    cargo run -- --compare scalar_id scalar_math --report size returns
    cargo run -- --compare flow --report opcodes --dump

NOTES

    --compare reads out/<name>/{sdk,solang}.wasm, so run --build first.
    --detail and --dump apply to the opcodes report only, and must come last.
