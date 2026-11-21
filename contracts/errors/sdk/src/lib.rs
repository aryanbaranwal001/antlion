#![no_std]
use soroban_sdk::{Env, contract, contracterror, contractimpl, panic_with_error, symbol_short};

#[contracterror]
#[derive(Clone, Copy)]
pub enum Error {
    TooSmall = 1,
}

const STORED: soroban_sdk::Symbol = symbol_short!("STORED");

#[contract]
pub struct Errors;

/// The four ways Solidity fails a call, each against its nearest Rust form. Every one fails
/// when `v` is below 10 and answers `v` otherwise, so `small` and `neg` fail and `large`
/// and `bound` pass.
///
/// `require` and a custom error map to a contract error: `req` panics with one and
/// `custom` returns one. `revert` with a message maps to `panic!` in `rev`, and `assert`
/// to `assert!` in `asrt`. `div` divides by zero for an even `v`. `set` stores `v`;
/// `set_fail` stores `v + 100` and then fails, and `get` reads the stored value, so a later
/// `get` shows whether the failed write was undone.
#[contractimpl]
impl Errors {
    pub fn req(env: Env, v: u32) -> u32 {
        if v < 10 {
            panic_with_error!(&env, Error::TooSmall);
        }
        v
    }

    pub fn rev(v: u32) -> u32 {
        if v < 10 {
            panic!("too small");
        }
        v
    }

    pub fn asrt(v: u32) -> u32 {
        assert!(v >= 10);
        v
    }

    pub fn custom(v: u32) -> Result<u32, Error> {
        if v < 10 {
            return Err(Error::TooSmall);
        }
        Ok(v)
    }

    pub fn div(v: u32) -> u32 {
        1000 / (v % 2)
    }

    pub fn set(env: Env, v: u32) -> u32 {
        env.storage().instance().set(&STORED, &v);
        v
    }

    pub fn set_fail(env: Env, v: u32) -> u32 {
        env.storage().instance().set(&STORED, &(v + 100));
        if v < 10 {
            panic_with_error!(&env, Error::TooSmall);
        }
        env.storage().instance().get(&STORED).unwrap()
    }

    pub fn get(env: Env) -> u32 {
        env.storage().instance().get(&STORED).unwrap_or(0)
    }
}
