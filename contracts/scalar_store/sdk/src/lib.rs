#![no_std]
use soroban_sdk::{Address, Env, Symbol, contract, contractimpl, symbol_short};

const BOOL: Symbol = symbol_short!("BOOL");
const U32: Symbol = symbol_short!("U32");
const I32: Symbol = symbol_short!("I32");
const U64: Symbol = symbol_short!("U64");
const I64: Symbol = symbol_short!("I64");
const U128: Symbol = symbol_short!("U128");
const I128: Symbol = symbol_short!("I128");
const ADDR: Symbol = symbol_short!("ADDR");

#[contract]
pub struct ScalarStore;

/// Write one value to instance storage and read it straight back. Subtract
/// `scalar_id`'s `_id` rows to isolate the storage round-trip from the boundary.
///
/// `instance` is stated on both sides — solidity would otherwise default to
/// `persistent`, which is not what the sdk idiom uses.
#[contractimpl]
impl ScalarStore {
    pub fn bool_store(env: Env, a: bool) -> bool {
        env.storage().instance().set(&BOOL, &a);
        env.storage().instance().get(&BOOL).unwrap()
    }

    pub fn u32_store(env: Env, a: u32) -> u32 {
        env.storage().instance().set(&U32, &a);
        env.storage().instance().get(&U32).unwrap()
    }

    pub fn i32_store(env: Env, a: i32) -> i32 {
        env.storage().instance().set(&I32, &a);
        env.storage().instance().get(&I32).unwrap()
    }

    pub fn u64_store(env: Env, a: u64) -> u64 {
        env.storage().instance().set(&U64, &a);
        env.storage().instance().get(&U64).unwrap()
    }

    pub fn i64_store(env: Env, a: i64) -> i64 {
        env.storage().instance().set(&I64, &a);
        env.storage().instance().get(&I64).unwrap()
    }

    pub fn u128_store(env: Env, a: u128) -> u128 {
        env.storage().instance().set(&U128, &a);
        env.storage().instance().get(&U128).unwrap()
    }

    pub fn i128_store(env: Env, a: i128) -> i128 {
        env.storage().instance().set(&I128, &a);
        env.storage().instance().get(&I128).unwrap()
    }

    pub fn address_store(env: Env, a: Address) -> Address {
        env.storage().instance().set(&ADDR, &a);
        env.storage().instance().get(&ADDR).unwrap()
    }
}
