#![no_std]
#![allow(unused_variables)]
use soroban_sdk::{Address, contract, contractimpl};

#[contract]
pub struct ScalarId;

/// Identity only — measures decoding the `i64` Val and encoding it back, nothing else.
/// `_id` takes one argument, `_id2` takes two and returns the first; `_id2` is what
/// `scalar_math` subtracts to isolate the arithmetic.
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

    pub fn address_id(a: Address) -> Address {
        a
    }

    pub fn bool_id2(a: bool, b: bool) -> bool {
        a
    }

    pub fn u32_id2(a: u32, b: u32) -> u32 {
        a
    }

    pub fn i32_id2(a: i32, b: i32) -> i32 {
        a
    }

    pub fn u64_id2(a: u64, b: u64) -> u64 {
        a
    }

    pub fn i64_id2(a: i64, b: i64) -> i64 {
        a
    }

    pub fn u128_id2(a: u128, b: u128) -> u128 {
        a
    }

    pub fn i128_id2(a: i128, b: i128) -> i128 {
        a
    }

    pub fn address_id2(a: Address, b: Address) -> Address {
        a
    }
}
