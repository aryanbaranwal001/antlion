#![no_std]
use soroban_sdk::{Env, Symbol, contract, contractimpl, contracttype, symbol_short};

const P: Symbol = symbol_short!("P");

/// `#[contracttype]` is what gives the struct a Val form, so the whole thing is one
/// ledger entry. Solang keys each field separately instead.
#[contracttype]
#[derive(Clone)]
pub struct Point {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
}

const ZERO: Point = Point { x: 0, y: 0, z: 0, w: 0 };

#[contract]
pub struct StructStore;

/// A four field struct in instance storage, one field and all four, written and read.
/// Storage forces both sides to materialise the struct, so nothing folds away.
///
/// Reads use `unwrap_or` so empty storage gives zeros rather than a panic, matching what
/// solidity does on an unset slot.
#[contractimpl]
impl StructStore {
    pub fn write_one(env: Env, a: u32) {
        let mut p = load(&env);
        p.x = a;
        env.storage().instance().set(&P, &p);
    }

    pub fn write_all(env: Env, a: u32) {
        let p = Point { x: a, y: a, z: a, w: a };
        env.storage().instance().set(&P, &p);
    }

    pub fn read_one(env: Env) -> u32 {
        load(&env).x
    }

    pub fn read_all(env: Env) -> u32 {
        let p = load(&env);
        p.x + p.y + p.z + p.w
    }
}

fn load(env: &Env) -> Point {
    env.storage().instance().get(&P).unwrap_or(ZERO)
}
