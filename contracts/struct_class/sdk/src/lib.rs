#![no_std]
use soroban_sdk::{Env, Symbol, contract, contractimpl, contracttype, symbol_short};

const INST: Symbol = symbol_short!("INST");
const PERS: Symbol = symbol_short!("PERS");
const TEMP: Symbol = symbol_short!("TEMP");

#[contracttype]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[contract]
pub struct StructClass;

/// One two field struct written whole and read back through each durability class.
/// `storage` does this for a `u32`; this checks the storage class annotation holds for a
/// struct too.
#[contractimpl]
impl StructClass {
    pub fn put_instance(env: Env, a: u32) -> u32 {
        env.storage().instance().set(&INST, &Point { x: a, y: a });
        env.storage().instance().get::<_, Point>(&INST).unwrap().x
    }

    pub fn put_persistent(env: Env, a: u32) -> u32 {
        env.storage().persistent().set(&PERS, &Point { x: a, y: a });
        env.storage().persistent().get::<_, Point>(&PERS).unwrap().x
    }

    pub fn put_temporary(env: Env, a: u32) -> u32 {
        env.storage().temporary().set(&TEMP, &Point { x: a, y: a });
        env.storage().temporary().get::<_, Point>(&TEMP).unwrap().x
    }
}
