#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct ScalarMath;

/// One operation per scalar type, the same operation on every numeric type, so the rows
/// compare to each other. `scalar_id` measures the boundary; the difference between the
/// two contracts is what the arithmetic itself costs.
///
/// `bool` has no addition, so it gets the nearest thing — it is its own baseline, not a
/// row to line up against the numerics.
///
/// Both sides trap on overflow: solidity 0.8 by default, rust because the workspace
/// release profile sets `overflow-checks = true`.
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
