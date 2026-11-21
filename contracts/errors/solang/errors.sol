// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// The four ways Solidity fails a call, each against its nearest Rust form. Every one fails
/// when `v` is below 10 and answers `v` otherwise, so `small` and `neg` fail and `large`
/// and `bound` pass.
///
/// `req`, `rev`, `asrt` and `custom` are `require`, `revert`, `assert` and a custom error.
/// `div` divides by zero for an even `v`, a failure neither side writes by hand. `set`
/// stores `v`; `set_fail` stores `v + 100` and then fails, and `get` reads the stored
/// value, so a later `get` shows whether the failed write was undone.
///
/// Function names match the sdk side so the builds pair by name.
contract Errors {
    error TooSmall(uint32 v);

    uint32 instance stored;

    function req(uint32 v) public pure returns (uint32) {
        require(v >= 10, "too small");
        return v;
    }

    function rev(uint32 v) public pure returns (uint32) {
        if (v < 10) {
            revert("too small");
        }
        return v;
    }

    function asrt(uint32 v) public pure returns (uint32) {
        assert(v >= 10);
        return v;
    }

    function custom(uint32 v) public pure returns (uint32) {
        if (v < 10) {
            revert TooSmall(v);
        }
        return v;
    }

    function div(uint32 v) public pure returns (uint32) {
        return 1000 / (v % 2);
    }

    function set(uint32 v) public returns (uint32) {
        stored = v;
        return stored;
    }

    function set_fail(uint32 v) public returns (uint32) {
        stored = v + 100;
        require(v >= 10, "too small");
        return stored;
    }

    function get() public view returns (uint32) {
        return stored;
    }
}
