#![no_std]
use soroban_sdk::{Env, Vec, contract, contractimpl, vec};

#[contract]
pub struct VecMem;

/// A list built inside the call and never stored. The two sides do not put it in the
/// same place: solidity allocates it in the wasm's own linear memory, while an sdk `Vec`
/// is a handle to an object that lives on the host.
///
/// Length comes from the argument so nothing folds to a constant.
#[contractimpl]
impl VecMem {
    pub fn push(env: Env, n: u32) -> u32 {
        build(&env, n).len()
    }

    pub fn sum(env: Env, n: u32) -> u32 {
        let v = build(&env, n);
        let mut t = 0;
        for x in v.iter() {
            t += x;
        }
        t
    }

    pub fn first(env: Env, n: u32) -> u32 {
        build(&env, n).get(0).unwrap_or(0)
    }
}

fn build(env: &Env, n: u32) -> Vec<u32> {
    let mut v = vec![env];
    for i in 0..n {
        v.push_back(i);
    }
    v
}
