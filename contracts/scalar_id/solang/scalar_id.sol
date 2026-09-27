// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// Identity only — measures decoding the `i64` Val and encoding it back, nothing else.
/// `_id` takes one argument, `_id2` takes two and returns the first; `_id2` is what
/// `scalar_math` subtracts to isolate the arithmetic.
///
/// Functions are named after the rust type so both builds pair by name.
contract Scalar_id {
    function bool_id(bool a) public pure returns (bool) {
        return a;
    }

    function u32_id(uint32 a) public pure returns (uint32) {
        return a;
    }

    function i32_id(int32 a) public pure returns (int32) {
        return a;
    }

    function u64_id(uint64 a) public pure returns (uint64) {
        return a;
    }

    function i64_id(int64 a) public pure returns (int64) {
        return a;
    }

    function u128_id(uint128 a) public pure returns (uint128) {
        return a;
    }

    function i128_id(int128 a) public pure returns (int128) {
        return a;
    }

    function address_id(address a) public pure returns (address) {
        return a;
    }

    function bool_id2(bool a, bool b) public pure returns (bool) {
        return a;
    }

    function u32_id2(uint32 a, uint32 b) public pure returns (uint32) {
        return a;
    }

    function i32_id2(int32 a, int32 b) public pure returns (int32) {
        return a;
    }

    function u64_id2(uint64 a, uint64 b) public pure returns (uint64) {
        return a;
    }

    function i64_id2(int64 a, int64 b) public pure returns (int64) {
        return a;
    }

    function u128_id2(uint128 a, uint128 b) public pure returns (uint128) {
        return a;
    }

    function i128_id2(int128 a, int128 b) public pure returns (int128) {
        return a;
    }

    function address_id2(address a, address b) public pure returns (address) {
        return a;
    }
}
