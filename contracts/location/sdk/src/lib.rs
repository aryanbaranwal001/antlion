#![no_std]
use soroban_sdk::{Env, Symbol, Vec, contract, contractimpl, contracttype, symbol_short, vec};

const XS: Symbol = symbol_short!("XS");
const PS: Symbol = symbol_short!("PS");
const TS: Symbol = symbol_short!("TS");
const PT: Symbol = symbol_short!("PT");
const SEED: u32 = 7;

#[contracttype]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[contract]
pub struct Location;

/// Solidity makes you say where a value lives. A `storage` local aliases the ledger, so
/// writing through it persists; a `memory` local is a copy, so writing through it does
/// not. Rust has no such keyword: you always get a copy, and the write persists only if
/// you call `set` again.
///
/// Each list function seeds its list, writes through one form or the other, then reads the
/// stored value back. The answer says which happened: `1` means the write landed, `7`
/// means it was discarded.
///
/// The `_pers` and `_temp` functions repeat `direct` and `via_storage` on a list held in
/// persistent and temporary storage, so the alias is tried in every storage class. The
/// `struct_` functions do the same for a struct field, with the alias on the write side
/// and on the read side separately.
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

    pub fn direct_pers(env: Env, v: u32) -> u32 {
        env.storage().persistent().set(&PS, &vec![&env, SEED]);
        let mut ps: Vec<u32> = env.storage().persistent().get(&PS).unwrap();
        ps.set(0, v);
        env.storage().persistent().set(&PS, &ps);
        let ps: Vec<u32> = env.storage().persistent().get(&PS).unwrap();
        ps.get(0).unwrap()
    }

    pub fn via_storage_pers(env: Env, v: u32) -> u32 {
        env.storage().persistent().set(&PS, &vec![&env, SEED]);
        let mut ps: Vec<u32> = env.storage().persistent().get(&PS).unwrap();
        ps.set(0, v);
        env.storage().persistent().set(&PS, &ps);
        let ps: Vec<u32> = env.storage().persistent().get(&PS).unwrap();
        ps.get(0).unwrap()
    }

    pub fn direct_temp(env: Env, v: u32) -> u32 {
        env.storage().temporary().set(&TS, &vec![&env, SEED]);
        let mut ts: Vec<u32> = env.storage().temporary().get(&TS).unwrap();
        ts.set(0, v);
        env.storage().temporary().set(&TS, &ts);
        let ts: Vec<u32> = env.storage().temporary().get(&TS).unwrap();
        ts.get(0).unwrap()
    }

    pub fn via_storage_temp(env: Env, v: u32) -> u32 {
        env.storage().temporary().set(&TS, &vec![&env, SEED]);
        let mut ts: Vec<u32> = env.storage().temporary().get(&TS).unwrap();
        ts.set(0, v);
        env.storage().temporary().set(&TS, &ts);
        let ts: Vec<u32> = env.storage().temporary().get(&TS).unwrap();
        ts.get(0).unwrap()
    }

    /// Control: set one field of the stored struct directly.
    pub fn struct_direct(env: Env, v: u32) -> u32 {
        set_x(&env, v);
        get_x(&env)
    }

    /// Solidity writes the field through an alias.
    pub fn struct_alias_write(env: Env, v: u32) -> u32 {
        set_x(&env, v);
        get_x(&env)
    }

    /// Solidity writes the field directly and reads it back through an alias.
    pub fn struct_alias_read(env: Env, v: u32) -> u32 {
        set_x(&env, v);
        get_x(&env)
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

fn set_x(env: &Env, v: u32) {
    let mut p = env
        .storage()
        .instance()
        .get(&PT)
        .unwrap_or(Point { x: 0, y: 0 });
    p.x = v;
    env.storage().instance().set(&PT, &p);
}

fn get_x(env: &Env) -> u32 {
    env.storage().instance().get::<_, Point>(&PT).unwrap().x
}
