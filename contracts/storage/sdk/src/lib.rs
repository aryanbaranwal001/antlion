#![no_std]
use soroban_sdk::{Env, Symbol, contract, contractimpl, symbol_short};

const INST: Symbol = symbol_short!("INST");
const PERS: Symbol = symbol_short!("PERS");
const TEMP: Symbol = symbol_short!("TEMP");
const IDIO: Symbol = symbol_short!("IDIO");

#[contract]
pub struct Storage;

/// One `u32` written and read back through each durability class. `scalar_store` holds
/// the type axis; this holds the durability axis, so both use a single `u32`.
///
/// `put_idiomatic` is the one deliberately mismatched row: each side does what its own
/// community writes by default. Solidity omits the annotation and lands on `persistent`;
/// the sdk has no default, and every soroban example reaches for `instance`.
#[contractimpl]
impl Storage {
    pub fn put_instance(env: Env, a: u32) -> u32 {
        env.storage().instance().set(&INST, &a);
        env.storage().instance().get(&INST).unwrap()
    }

    pub fn put_persistent(env: Env, a: u32) -> u32 {
        env.storage().persistent().set(&PERS, &a);
        env.storage().persistent().get(&PERS).unwrap()
    }

    pub fn put_temporary(env: Env, a: u32) -> u32 {
        env.storage().temporary().set(&TEMP, &a);
        env.storage().temporary().get(&TEMP).unwrap()
    }

    pub fn put_idiomatic(env: Env, a: u32) -> u32 {
        env.storage().instance().set(&IDIO, &a);
        env.storage().instance().get(&IDIO).unwrap()
    }
}
