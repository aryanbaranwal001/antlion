#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct ScalarMath;

/// One add per numeric type, `&&` for bool; both sides trap on overflow. Subtract
/// `scalar_id`'s `_id2` rows to isolate the arithmetic from the boundary.
///
/// No `address` row: solang has no operation on one — `==` and `!=` both crash it.
#[contractimpl]
impl ScalarMath {
    pub fn bool_op(a: bool, b: bool) -> bool {
        a && b
    }

    pub fn u32_op(a: u32, b: u32) -> u32 {
        a + b
    }

    pub fn i32_op(a: i32, b: i32) -> i32 {
        a + b
    }

    pub fn u64_op(a: u64, b: u64) -> u64 {
        a + b
    }

    pub fn i64_op(a: i64, b: i64) -> i64 {
        a + b
    }

    pub fn u128_op(a: u128, b: u128) -> u128 {
        a + b
    }

    pub fn i128_op(a: i128, b: i128) -> i128 {
        a + b
    }
}
