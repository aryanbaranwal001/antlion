#![no_std]
use soroban_sdk::{
    Address, Env, contract, contracterror, contractevent, contractimpl, panic_with_error,
};

#[contractevent]
pub struct Ping {}

#[contractevent]
pub struct Value {
    pub v: u32,
}

#[contractevent]
pub struct Keyed {
    #[topic]
    pub k: u32,
    pub v: u32,
}

#[contractevent]
pub struct Two {
    pub v: u32,
    pub w: u32,
}

#[contractevent]
pub struct Pair {
    #[topic]
    pub from: Address,
    #[topic]
    pub k: u32,
    pub v: u32,
    pub w: u32,
}

#[contracterror]
#[derive(Clone, Copy)]
pub enum Error {
    TooSmall = 1,
}

#[contract]
pub struct Events;

/// One function per event shape: no fields, one data field, two data fields, a `#[topic]`
/// field with one data field, and two `#[topic]` fields with two data fields. `many` emits
/// three events in one call, and `emit_fail` emits one and then fails, so the event belongs
/// to a call that failed. Each event is a `#[contractevent]` with its defaults: the name as
/// the first topic and the data fields as a map.

#[contractimpl]
impl Events {
    pub fn ping(env: Env) -> u32 {
        Ping {}.publish(&env);
        1
    }

    pub fn value(env: Env, v: u32) -> u32 {
        Value { v }.publish(&env);
        v
    }

    pub fn keyed(env: Env, k: u32, v: u32) -> u32 {
        Keyed { k, v }.publish(&env);
        v
    }

    pub fn two(env: Env, v: u32) -> u32 {
        Two { v, w: v + 1 }.publish(&env);
        v
    }

    pub fn pair(env: Env, from: Address, k: u32, v: u32) -> u32 {
        Pair {
            from,
            k,
            v,
            w: v + 1,
        }
        .publish(&env);
        v
    }

    pub fn many(env: Env, v: u32) -> u32 {
        Value { v }.publish(&env);
        Value { v: v + 1 }.publish(&env);
        Value { v: v + 2 }.publish(&env);
        v
    }

    pub fn emit_fail(env: Env, v: u32) -> u32 {
        Value { v }.publish(&env);
        if v < 10 {
            panic_with_error!(&env, Error::TooSmall);
        }
        v
    }
}
