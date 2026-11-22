#![no_std]
use soroban_sdk::{Env, Symbol, contract, contractimpl, symbol_short};

const P: Symbol = symbol_short!("P");
const T: Symbol = symbol_short!("T");
const I: Symbol = symbol_short!("I");

/// The TTL each class gets when extended, in ledgers. Each is also the threshold: a new entry
/// has less left than that, so a `bump_` call always extends. They differ, so the entries
/// expire one at a time.
const TEMPORARY_TO: u32 = 3000;
const PERSISTENT_TO: u32 = 6000;
const INSTANCE_TO: u32 = 9000;

#[contract]
pub struct Ttl;

/// One value in each storage class. `put_` writes it, `get_` reads it, giving 0 when it is
/// missing, and `bump_` writes it and extends its TTL. `bump_i` extends the instance, and with
/// it the contract code.
///
/// The values are `u64` to match the Solidity side, where `extendTtl` takes only a `uint64`
/// variable.
#[contractimpl]
impl Ttl {
    pub fn put_p(env: Env, v: u64) -> u64 {
        env.storage().persistent().set(&P, &v);
        v
    }

    pub fn put_t(env: Env, v: u64) -> u64 {
        env.storage().temporary().set(&T, &v);
        v
    }

    pub fn put_i(env: Env, v: u64) -> u64 {
        env.storage().instance().set(&I, &v);
        v
    }

    pub fn get_p(env: Env) -> u64 {
        env.storage().persistent().get(&P).unwrap_or(0)
    }

    pub fn get_t(env: Env) -> u64 {
        env.storage().temporary().get(&T).unwrap_or(0)
    }

    pub fn get_i(env: Env) -> u64 {
        env.storage().instance().get(&I).unwrap_or(0)
    }

    pub fn bump_p(env: Env, v: u64) -> u64 {
        env.storage().persistent().set(&P, &v);
        env.storage().persistent().extend_ttl(&P, PERSISTENT_TO, PERSISTENT_TO);
        v
    }

    pub fn bump_t(env: Env, v: u64) -> u64 {
        env.storage().temporary().set(&T, &v);
        env.storage().temporary().extend_ttl(&T, TEMPORARY_TO, TEMPORARY_TO);
        v
    }

    pub fn bump_i(env: Env) -> u64 {
        env.storage().instance().extend_ttl(INSTANCE_TO, INSTANCE_TO);
        1
    }
}
