#![no_std]
use soroban_sdk::{Env, Vec, contract, contractimpl, contracttype};

/// One key per state variable. `V(n)` is the `uint32` the solidity side declares n-th.
#[contracttype]
pub enum DataKey {
    M(u32),
    Xs,
    P,
    V(u32),
}

#[contracttype]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[contract]
pub struct Slots;

/// Twenty state variables, each written and read back by its own function. The solidity
/// side declares a mapping, an array and a struct first, then seventeen `uint32`s, so this
/// pair shows which key each kind of variable is given and where the key sequence stops
/// being usable.
///
/// Every key here is chosen by the author, so the Rust side has no limit on how many
/// there are.
#[contractimpl]
impl Slots {
    pub fn put_m(env: Env, a: u32) -> u32 {
        put(&env, DataKey::M(a), a)
    }

    pub fn put_xs(env: Env, a: u32) -> u32 {
        let mut xs: Vec<u32> = env
            .storage()
            .instance()
            .get(&DataKey::Xs)
            .unwrap_or(Vec::new(&env));
        xs.push_back(a);
        env.storage().instance().set(&DataKey::Xs, &xs);
        let xs: Vec<u32> = env.storage().instance().get(&DataKey::Xs).unwrap();
        xs.last().unwrap()
    }

    pub fn put_p(env: Env, a: u32) -> u32 {
        let mut p = env
            .storage()
            .instance()
            .get(&DataKey::P)
            .unwrap_or(Point { x: 0, y: 0 });
        p.x = a;
        env.storage().instance().set(&DataKey::P, &p);
        let p: Point = env.storage().instance().get(&DataKey::P).unwrap();
        p.x
    }

    pub fn put_v03(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(3), a)
    }

    pub fn put_v04(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(4), a)
    }

    pub fn put_v05(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(5), a)
    }

    pub fn put_v06(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(6), a)
    }

    pub fn put_v07(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(7), a)
    }

    pub fn put_v08(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(8), a)
    }

    pub fn put_v09(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(9), a)
    }

    pub fn put_v10(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(10), a)
    }

    pub fn put_v11(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(11), a)
    }

    pub fn put_v12(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(12), a)
    }

    pub fn put_v13(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(13), a)
    }

    pub fn put_v14(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(14), a)
    }

    pub fn put_v15(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(15), a)
    }

    pub fn put_v16(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(16), a)
    }

    pub fn put_v17(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(17), a)
    }

    pub fn put_v18(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(18), a)
    }

    pub fn put_v19(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(19), a)
    }
}

fn put(env: &Env, key: DataKey, a: u32) -> u32 {
    env.storage().instance().set(&key, &a);
    env.storage().instance().get(&key).unwrap()
}
