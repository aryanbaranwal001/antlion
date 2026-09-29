// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// Solidity makes you say where a value lives. A `storage` local aliases the ledger, so
/// writing through it persists; a `memory` local is a copy, so writing through it does
/// not. Rust has no such keyword: you always get a copy, and the write persists only if
/// you call `set` again.
///
/// Each list function seeds its list, writes through one form or the other, then reads the
/// stored value back. The answer says which happened: `1` means the write landed, `7`
/// means it was discarded.
///
/// The `_pers` and `_temp` functions repeat `direct` and `via_storage` on a list held in
/// persistent and temporary storage, so the alias is tried in every storage class. The
/// `struct_` functions do the same for a struct field, with the alias on the write side
/// and on the read side separately.
///
/// Function names match the sdk side so the builds pair by name.
contract Location {
    struct Point {
        uint32 x;
        uint32 y;
    }

    uint32[] instance xs;
    uint32[] persistent ps;
    uint32[] temporary ts;
    Point instance pt;

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

    function direct_pers(uint32 v) public returns (uint32) {
        seed_pers();
        ps[0] = v;
        return ps[0];
    }

    function via_storage_pers(uint32 v) public returns (uint32) {
        seed_pers();
        uint32[] storage r = ps;
        r[0] = v;
        return ps[0];
    }

    function direct_temp(uint32 v) public returns (uint32) {
        seed_temp();
        ts[0] = v;
        return ts[0];
    }

    function via_storage_temp(uint32 v) public returns (uint32) {
        seed_temp();
        uint32[] storage r = ts;
        r[0] = v;
        return ts[0];
    }

    /// Control: set one field of the stored struct directly.
    function struct_direct(uint32 v) public returns (uint32) {
        pt.x = v;
        return pt.x;
    }

    /// Writes the field through an alias.
    function struct_alias_write(uint32 v) public returns (uint32) {
        Point storage r = pt;
        r.x = v;
        return pt.x;
    }

    /// Writes the field directly and reads it back through an alias.
    function struct_alias_read(uint32 v) public returns (uint32) {
        pt.x = v;
        Point storage r = pt;
        return r.x;
    }

    function seed() private {
        while (xs.length > 0) {
            xs.pop();
        }
        xs.push(7);
    }

    function seed_pers() private {
        while (ps.length > 0) {
            ps.pop();
        }
        ps.push(7);
    }

    function seed_temp() private {
        while (ts.length > 0) {
            ts.pop();
        }
        ts.push(7);
    }
}
