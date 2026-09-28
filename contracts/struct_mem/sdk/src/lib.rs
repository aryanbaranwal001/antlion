#![no_std]
use soroban_sdk::{contract, contractimpl};

/// Plain rust structs, no `#[contracttype]`: these never reach storage or the contract
/// boundary, so they need no Val form.
pub struct Point {
    pub x: u32,
    pub y: u32,
}

pub struct Outer {
    pub inner: Point,
    pub tag: u32,
}

#[contract]
pub struct StructMem;

/// A struct built in the function body and never stored. Values come from the argument,
/// but that does not keep the struct alive: rustc replaces it with its fields, so these
/// rows measure whether each compiler materialises a local aggregate at all.
#[contractimpl]
impl StructMem {
    pub fn build(a: u32) -> u32 {
        let p = Point { x: a, y: a };
        p.x + p.y
    }

    pub fn nested(a: u32) -> u32 {
        let o = Outer { inner: Point { x: a, y: a }, tag: a };
        o.inner.x + o.inner.y + o.tag
    }
}
