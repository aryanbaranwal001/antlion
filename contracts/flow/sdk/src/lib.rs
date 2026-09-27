#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct Flow;

/// One control-flow construct per function, every trip count taken from the argument so
/// nothing folds to a constant. Subtract `scalar_id`'s `u32_id` to drop the boundary.
#[contractimpl]
impl Flow {
    pub fn branch(n: u32) -> u32 {
        if n % 2 == 0 { n / 2 } else { n + 1 }
    }

    pub fn loop_(n: u32) -> u32 {
        let mut t = 0;
        for i in 0..n {
            t += i;
        }
        t
    }

    pub fn while_(n: u32) -> u32 {
        let mut t = 0;
        let mut i = 0;
        while i < n {
            t += i;
            i += 1;
        }
        t
    }

    pub fn nested(n: u32) -> u32 {
        let mut t = 0;
        for _ in 0..n {
            for _ in 0..n {
                t += 1;
            }
        }
        t
    }

    pub fn early(n: u32) -> u32 {
        let mut t = 0;
        for i in 0..n {
            if i % 3 == 0 {
                continue;
            }
            if i == 9 {
                break;
            }
            t += i;
        }
        t
    }
}
