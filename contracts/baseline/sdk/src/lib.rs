#![no_std]
use soroban_sdk::{contract, contractimpl};

/// Empty on purpose: its size is the fixed cost of being a contract, to subtract before
/// attributing bytes to a feature.
#[contract]
pub struct Baseline;

#[contractimpl]
impl Baseline {}
