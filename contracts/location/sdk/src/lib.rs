#![no_std]
use soroban_sdk::{Env, Symbol, Vec, contract, contractimpl, symbol_short, vec};

const XS: Symbol = symbol_short!("XS");
const SEED: u32 = 7;

#[contract]
pub struct Location;

/// Solidity makes you say where a value lives. A `storage` local aliases the ledger, so
/// writing through it persists; a `memory` local is a copy, so writing through it does
/// not. Rust has no such keyword: you always get a copy, and the write persists only if
/// you call `set` again.
///
/// Each function seeds the list, writes through one form or the other, then reads the
/// stored value back. The answer says which happened: `1` means the write landed, `7`
/// means it was discarded.
///
/// A list rather than a struct, because a struct in storage does not run on solang.
#[contractimpl]
impl Location {
    /// Control: write straight to storage, no local in between.
    pub fn direct(env: Env, v: u32) -> u32 {
        seed(&env);
        let mut xs = load(&env);
        xs.set(0, v);
        save(&env, &xs);
        first(&env)
    }

    /// Solidity writes through an alias. Rust has to store the copy back.
    pub fn via_storage(env: Env, v: u32) -> u32 {
        seed(&env);
        let mut xs = load(&env);
        xs.set(0, v);
        save(&env, &xs);
        first(&env)
    }

    /// Solidity mutates a copy and drops it. Rust does the same by not storing it.
    pub fn via_memory(env: Env, v: u32) -> u32 {
        seed(&env);
        let mut xs = load(&env);
        xs.set(0, v);
        first(&env)
    }
}

fn seed(env: &Env) {
    save(env, &vec![env, SEED]);
}

fn load(env: &Env) -> Vec<u32> {
    env.storage().instance().get(&XS).unwrap_or(vec![env])
}

fn save(env: &Env, xs: &Vec<u32>) {
    env.storage().instance().set(&XS, xs);
}

fn first(env: &Env) -> u32 {
    load(env).get(0).unwrap_or(0)
}
