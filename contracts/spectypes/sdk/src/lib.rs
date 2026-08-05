#![no_std]
#![allow(unused_variables)]
use soroban_sdk::{
    Address, Bytes, BytesN, Duration, I256, Map, MuxedAddress, String, Symbol, Timepoint, U256,
    Vec, contract, contracterror, contractimpl, contracttype,
};

#[contracttype]
pub struct Point {
    pub x: u32,
    pub y: i128,
}

#[contracttype]
pub enum Shape {
    Dot,
    Line(Point),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Oops {
    Bad = 1,
}

#[contract]
pub struct SpecTypes;

#[contractimpl]
impl SpecTypes {
    pub fn ints(a: u32, b: i32, c: u64, d: i64, e: u128, f: i128, g: U256, h: I256) -> bool {
        true
    }

    pub fn words(a: Bytes, b: BytesN<32>, c: String, d: Symbol, e: Address, f: MuxedAddress) -> Symbol {
        d
    }

    pub fn times(a: Timepoint, b: Duration) -> Timepoint {
        a
    }

    pub fn nested(a: Option<u32>, b: Vec<i128>, c: Map<Symbol, Address>, d: (u32, bool)) -> Option<Vec<u32>> {
        None
    }

    pub fn udt(p: Point, s: Shape) -> Result<Point, Oops> {
        Ok(p)
    }
}
