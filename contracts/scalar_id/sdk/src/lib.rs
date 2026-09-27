#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct ScalarId;

/// One identity function per scalar type. The body is a no-op on purpose: what is
/// measured here is the boundary — decoding the `i64` Val into a native value and
/// encoding it back — with no arithmetic or storage mixed in.
#[contractimpl]
impl ScalarId {
    pub fn bool_id(a: bool) -> bool {
        a
    }

    pub fn u32_id(a: u32) -> u32 {
        a
    }

    pub fn i32_id(a: i32) -> i32 {
        a
    }

    pub fn u64_id(a: u64) -> u64 {
        a
    }

    pub fn i64_id(a: i64) -> i64 {
        a
    }

    pub fn u128_id(a: u128) -> u128 {
        a
    }

    pub fn i128_id(a: i128) -> i128 {
        a
    }
}
