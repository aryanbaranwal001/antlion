// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One add per numeric type, `&&` for bool; both sides trap on overflow. Subtract
/// `scalar_id`'s `_id2` rows to isolate the arithmetic from the boundary.
///
/// No `address` row: solang has no operation on one — `==` and `!=` both crash it.
///
/// Functions are named after the rust type so both builds pair by name.
contract Scalar_math {
    function bool_op(bool a, bool b) public pure returns (bool) {
        return a && b;
    }

    function u32_op(uint32 a, uint32 b) public pure returns (uint32) {
        return a + b;
    }

    function i32_op(int32 a, int32 b) public pure returns (int32) {
        return a + b;
    }

    function u64_op(uint64 a, uint64 b) public pure returns (uint64) {
        return a + b;
    }

    function i64_op(int64 a, int64 b) public pure returns (int64) {
        return a + b;
    }

    function u128_op(uint128 a, uint128 b) public pure returns (uint128) {
        return a + b;
    }

    function i128_op(int128 a, int128 b) public pure returns (int128) {
        return a + b;
    }
}
