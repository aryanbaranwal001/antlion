// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// Write one value to instance storage and read it straight back. Subtract
/// `scalar_id`'s `_id` rows to isolate the storage round-trip from the boundary.
///
/// `instance` is stated on both sides — solidity would otherwise default to
/// `persistent`, which is not what the sdk idiom uses.
///
/// Functions are named after the rust type so both builds pair by name.
contract Scalar_store {
    bool instance bool_v;
    uint32 instance u32_v;
    int32 instance i32_v;
    uint64 instance u64_v;
    int64 instance i64_v;
    uint128 instance u128_v;
    int128 instance i128_v;
    address instance addr_v;

    function bool_store(bool a) public returns (bool) {
        bool_v = a;
        return bool_v;
    }

    function u32_store(uint32 a) public returns (uint32) {
        u32_v = a;
        return u32_v;
    }

    function i32_store(int32 a) public returns (int32) {
        i32_v = a;
        return i32_v;
    }

    function u64_store(uint64 a) public returns (uint64) {
        u64_v = a;
        return u64_v;
    }

    function i64_store(int64 a) public returns (int64) {
        i64_v = a;
        return i64_v;
    }

    function u128_store(uint128 a) public returns (uint128) {
        u128_v = a;
        return u128_v;
    }

    function i128_store(int128 a) public returns (int128) {
        i128_v = a;
        return i128_v;
    }

    function address_store(address a) public returns (address) {
        addr_v = a;
        return addr_v;
    }
}
