#![no_std]
use soroban_sdk::{Address, Env, contract, contractimpl, contracttype};

#[contracttype]
pub enum DataKey {
    Bal(Address),
}

#[contract]
pub struct Auth;

/// `require_auth` on an address argument. `no_auth` is the control and never asks;
/// `need_auth` asks and answers `1`; `auth_store` asks, then stores `a` under the address,
/// as a balance would be.
#[contractimpl]
impl Auth {
    pub fn no_auth(_user: Address) -> u32 {
        1
    }

    pub fn need_auth(user: Address) -> u32 {
        user.require_auth();
        1
    }

    pub fn auth_store(env: Env, user: Address, a: u32) -> u32 {
        user.require_auth();
        let key = DataKey::Bal(user);
        env.storage().instance().set(&key, &a);
        env.storage().instance().get(&key).unwrap()
    }
}
