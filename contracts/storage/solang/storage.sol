// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One `uint32` written and read back through each durability class. `scalar_store`
/// holds the type axis; this holds the durability axis, so both use a single `uint32`.
///
/// `put_idiomatic` is the one deliberately mismatched row: each side does what its own
/// community writes by default. Solidity omits the annotation and lands on `persistent`;
/// the sdk has no default, and every soroban example reaches for `instance`.
///
/// Functions are named the same on both sides so the builds pair by name.
contract Storage {
    uint32 instance inst_v;
    uint32 persistent pers_v;
    uint32 temporary temp_v;
    uint32 idio_v;

    function put_instance(uint32 a) public returns (uint32) {
        inst_v = a;
        return inst_v;
    }

    function put_persistent(uint32 a) public returns (uint32) {
        pers_v = a;
        return pers_v;
    }

    function put_temporary(uint32 a) public returns (uint32) {
        temp_v = a;
        return temp_v;
    }

    function put_idiomatic(uint32 a) public returns (uint32) {
        idio_v = a;
        return idio_v;
    }
}
