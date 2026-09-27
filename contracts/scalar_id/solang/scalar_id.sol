// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One identity function per scalar type. The body is a no-op on purpose: what is
/// measured here is the boundary — decoding the `i64` Val into a native value and
/// encoding it back — with no arithmetic or storage mixed in.
///
/// Functions are named after the rust type, not the solidity one, so both builds
/// pair by name in the reports.
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
}
