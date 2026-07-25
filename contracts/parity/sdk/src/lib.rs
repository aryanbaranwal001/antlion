#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct ParityContract;

#[contractimpl]
impl ParityContract {
    pub fn sum(n: u32) -> u32 {
        let mut total: u32 = 0;
        let mut i: u32 = 1;
        while i <= n {
            if i % 2 == 0 {
                total += i;
            } else {
                total += 1;
            }
            i += 1;
        }
        total
    }
}
