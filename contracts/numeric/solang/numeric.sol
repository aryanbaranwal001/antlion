// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One numeric edge case per function. Each takes one argument and keeps the other
/// operand fixed, so the `small`, `large`, `bound` and `neg` inputs reach the edge: zero,
/// the type's maximum or minimum, and a negative.
///
/// Arithmetic is checked, as in Solidity 0.8, except in `wrap`, which is `unchecked`.
///
/// Function names match the sdk side so the builds pair by name.
contract Numeric {
    /// Underflows at 0.
    function sub_one(uint32 a) public pure returns (uint32) {
        return a - 1;
    }

    /// Overflows past `type(uint32).max / 2`.
    function mul_two(uint32 a) public pure returns (uint32) {
        return a * 2;
    }

    /// Divides by zero at 0.
    function div_by(uint32 a) public pure returns (uint32) {
        return 100 / a;
    }

    /// Takes the remainder by zero at 0.
    function rem_by(uint32 a) public pure returns (uint32) {
        return 100 % a;
    }

    /// Overflows at `type(int32).min`, whose negation does not fit.
    function neg(int32 a) public pure returns (int32) {
        return -a;
    }

    /// Overflows at `type(int32).min`, for the same reason.
    function neg_div(int32 a) public pure returns (int32) {
        return a / -1;
    }

    /// Rounds toward zero: -1 / 2 is 0.
    function sdiv(int32 a) public pure returns (int32) {
        return a / 2;
    }

    /// Takes the sign of the dividend: -1 % 3 is -1.
    function smod(int32 a) public pure returns (int32) {
        return a % 3;
    }

    /// Wraps at `type(uint32).max` on purpose.
    function wrap(uint32 a) public pure returns (uint32) {
        unchecked {
            return a + 1;
        }
    }

    /// Overflows past 31.
    function pow2(uint32 a) public pure returns (uint32) {
        return uint32(2) ** a;
    }

    /// A shift by 32 or more moves every bit out: the result is 0.
    function shl(uint32 a) public pure returns (uint32) {
        return uint32(1) << a;
    }

    /// The same, to the right.
    function shr(uint32 a) public pure returns (uint32) {
        return type(uint32).max >> a;
    }

    /// Reads the same bits as signed: `type(uint32).max` is -1.
    function to_signed(uint32 a) public pure returns (int32) {
        return int32(a);
    }

    /// Keeps the low 32 bits.
    function narrow(uint64 a) public pure returns (uint32) {
        return uint32(a);
    }
}
