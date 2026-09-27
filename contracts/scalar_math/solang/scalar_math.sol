// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One operation per scalar type, the same operation on every numeric type, so the rows
/// compare to each other. `scalar_id` measures the boundary; the difference between the
/// two contracts is what the arithmetic itself costs.
///
/// `bool` has no addition, so it gets the nearest thing — it is its own baseline, not a
/// row to line up against the numerics.
///
/// Both sides are written the idiomatic way, which is not the same semantics: solidity
/// 0.8 traps on overflow, rust wraps unless `overflow-checks` is on, and this workspace
/// does not set it.
///
/// Functions are named after the rust type so both builds pair by name in the reports.
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
