#![no_std]
use soroban_sdk::{Env, contract, contractimpl, contracttype};

/// One ledger entry per key, keyed by an enum variant carrying the mapping key. This is
/// how soroban examples write a solidity `mapping`.
#[contracttype]
pub enum DataKey {
    M(u32),
    Mp(u32),
}

#[contracttype]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[contract]
pub struct Mapping;

/// A `u32` mapping and a struct mapping in instance storage. `put` and `put_p` write a key
/// and read it back; `get` and `get_px` read a key nothing wrote.
///
/// Reads use `unwrap_or` so a missing key gives zero rather than a panic, matching what
/// solidity does on an unset key.
#[contractimpl]
impl Mapping {
    pub fn put(env: Env, k: u32, v: u32) -> u32 {
        env.storage().instance().set(&DataKey::M(k), &v);
        env.storage().instance().get(&DataKey::M(k)).unwrap()
    }

    pub fn get(env: Env, k: u32) -> u32 {
        env.storage().instance().get(&DataKey::M(k)).unwrap_or(0)
    }

    pub fn put_p(env: Env, k: u32, a: u32) -> u32 {
        env.storage().instance().set(&DataKey::Mp(k), &Point { x: a, y: a });
        let p: Point = env.storage().instance().get(&DataKey::Mp(k)).unwrap();
        p.x
    }

    pub fn get_px(env: Env, k: u32) -> u32 {
        env.storage()
            .instance()
            .get::<_, Point>(&DataKey::Mp(k))
            .map(|p| p.x)
            .unwrap_or(0)
    }
}
