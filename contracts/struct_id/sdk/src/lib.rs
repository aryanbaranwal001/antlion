#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype};

/// `#[contracttype]` gives these a Val form, so they can cross the contract boundary.
#[contracttype]
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
pub struct StructId;

/// A struct at the contract boundary. `get_x` only decodes one, `make` only encodes one,
/// `id` and `id_outer` do both, so each direction can be told apart.
#[contractimpl]
impl StructId {
    pub fn get_x(p: Point) -> u32 {
        p.x
    }

    pub fn make(a: u32) -> Point {
        Point { x: a, y: a }
    }

    pub fn id(p: Point) -> Point {
        p
    }

    pub fn id_outer(o: Outer) -> Outer {
        o
    }
}
