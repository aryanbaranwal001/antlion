#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct Numeric;

/// One numeric edge case per function. Each takes one argument and keeps the other
/// operand fixed, so the `small`, `large`, `bound` and `neg` inputs reach the edge: zero,
/// the type's maximum or minimum, and a negative.
///
/// Each body is the plain Rust form of the Solidity one. The release profile keeps
/// overflow checks on, so an overflow panics, as checked Solidity arithmetic reverts.
#[contractimpl]
impl Numeric {
    /// Underflows at 0.
    pub fn sub_one(a: u32) -> u32 {
        a - 1
    }

    /// Overflows past `u32::MAX / 2`.
    pub fn mul_two(a: u32) -> u32 {
        a * 2
    }

    /// Divides by zero at 0.
    pub fn div_by(a: u32) -> u32 {
        100 / a
    }

    /// Takes the remainder by zero at 0.
    pub fn rem_by(a: u32) -> u32 {
        100 % a
    }

    /// Overflows at `i32::MIN`, whose negation does not fit.
    pub fn neg(a: i32) -> i32 {
        -a
    }

    /// Overflows at `i32::MIN`, for the same reason.
    pub fn neg_div(a: i32) -> i32 {
        a / -1
    }

    /// Rounds toward zero: -1 / 2 is 0.
    pub fn sdiv(a: i32) -> i32 {
        a / 2
    }

    /// Takes the sign of the dividend: -1 % 3 is -1.
    pub fn smod(a: i32) -> i32 {
        a % 3
    }

    /// Wraps at `u32::MAX` on purpose, as an `unchecked` block does.
    pub fn wrap(a: u32) -> u32 {
        a.wrapping_add(1)
    }

    /// Overflows past 31.
    pub fn pow2(a: u32) -> u32 {
        2u32.pow(a)
    }

    /// A shift by 32 or more does not fit in a `u32`.
    pub fn shl(a: u32) -> u32 {
        1 << a
    }

    /// The same, to the right.
    pub fn shr(a: u32) -> u32 {
        u32::MAX >> a
    }

    /// Reads the same bits as signed: `u32::MAX` is -1.
    pub fn to_signed(a: u32) -> i32 {
        a as i32
    }

    /// Keeps the low 32 bits.
    pub fn narrow(a: u64) -> u32 {
        a as u32
    }
}
