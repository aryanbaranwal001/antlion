// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// Solidity makes you say where a value lives. A `storage` local aliases the ledger, so
/// writing through it persists; a `memory` local is a copy, so writing through it does
/// not. Rust has no such keyword: you always get a copy, and the write persists only if
/// you call `set` again.
///
/// Each function seeds the list, writes through one form or the other, then reads the
/// stored value back. The answer says which happened: `1` means the write landed, `7`
/// means it was discarded.
///
/// A list rather than a struct, because a struct in storage does not run on solang.
///
/// Function names match the sdk side so the builds pair by name.
contract Location {
    uint32[] instance xs;

    /// Control: write straight to storage, no local in between.
    function direct(uint32 v) public returns (uint32) {
        seed();
        xs[0] = v;
        return xs[0];
    }

    /// Writes through an alias, so the ledger changes.
    function via_storage(uint32 v) public returns (uint32) {
        seed();
        uint32[] storage r = xs;
        r[0] = v;
        return xs[0];
    }

    /// Mutates a copy and drops it, so the ledger does not change.
    function via_memory(uint32 v) public returns (uint32) {
        seed();
        uint32[] memory c = xs;
        c[0] = v;
        return xs[0];
    }

    function seed() private {
        while (xs.length > 0) {
            xs.pop();
        }
        xs.push(7);
    }
}
