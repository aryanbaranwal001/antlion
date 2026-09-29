#![no_std]
use soroban_sdk::{Env, Vec, contract, contractimpl, contracttype, vec};

/// One key per state variable. `V(n)` is the `uint32` the solidity side declares n-th,
/// counting from zero across every state variable.
#[contracttype]
pub enum DataKey {
    M(u32),
    Xs,
    P,
    O,
    Fx,
    V(u32),
}

#[contracttype]
#[derive(Clone)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[contracttype]
pub struct Outer {
    pub inner: Point,
    pub tag: u32,
}

#[contract]
pub struct Slots;

/// Twenty two state variables, each written and read back by its own function. The
/// solidity side declares a mapping, an array, a struct, a nested struct and a fixed size
/// array first, then seventeen `uint32`s, so this pair shows which key each kind of
/// variable is given and where the key sequence stops being usable.
///
/// Every key here is chosen by the author, so the Rust side has no limit on how many
/// there are. Rust has no fixed size array in storage, so `fx` is a `Vec` of four.
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

    pub fn put_o(env: Env, a: u32) -> u32 {
        let mut o = env.storage().instance().get(&DataKey::O).unwrap_or(Outer {
            inner: Point { x: 0, y: 0 },
            tag: 0,
        });
        o.inner.x = a;
        env.storage().instance().set(&DataKey::O, &o);
        let o: Outer = env.storage().instance().get(&DataKey::O).unwrap();
        o.inner.x
    }

    pub fn put_fx(env: Env, a: u32) -> u32 {
        let mut fx: Vec<u32> = env
            .storage()
            .instance()
            .get(&DataKey::Fx)
            .unwrap_or(vec![&env, 0, 0, 0, 0]);
        fx.set(0, a);
        env.storage().instance().set(&DataKey::Fx, &fx);
        let fx: Vec<u32> = env.storage().instance().get(&DataKey::Fx).unwrap();
        fx.get(0).unwrap()
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

    pub fn put_v20(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(20), a)
    }

    pub fn put_v21(env: Env, a: u32) -> u32 {
        put(&env, DataKey::V(21), a)
    }
}

fn put(env: &Env, key: DataKey, a: u32) -> u32 {
    env.storage().instance().set(&key, &a);
    env.storage().instance().get(&key).unwrap()
}
