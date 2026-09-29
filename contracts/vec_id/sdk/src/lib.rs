#![no_std]
use soroban_sdk::{Env, Vec, contract, contractimpl, vec};

#[contract]
pub struct VecId;

/// A list at the contract boundary. `len` and `first` only decode one, `make` only encodes
/// one, `id` does both, and `sum` walks every element.
#[contractimpl]
impl VecId {
    pub fn len(v: Vec<u32>) -> u32 {
        v.len()
    }

    pub fn first(v: Vec<u32>) -> u32 {
        v.get(0).unwrap_or(0)
    }

    pub fn make(env: Env, a: u32) -> Vec<u32> {
        vec![&env, a, a, a]
    }

    pub fn id(v: Vec<u32>) -> Vec<u32> {
        v
    }

    pub fn sum(v: Vec<u32>) -> u32 {
        let mut t: u32 = 0;
        for x in v.iter() {
            t += x;
        }
        t
    }
}
