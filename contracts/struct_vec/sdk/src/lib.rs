#![no_std]
use soroban_sdk::{Env, Symbol, Vec, contract, contractimpl, contracttype, symbol_short};

const PS: Symbol = symbol_short!("PS");

#[contracttype]
#[derive(Clone)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[contract]
pub struct StructVec;

/// A list of structs in instance storage. `push` appends one and counts the list,
/// `push_read` appends one and reads a field back, `write_field` appends one and then
/// changes one field of it in place.
#[contractimpl]
impl StructVec {
    pub fn push(env: Env, a: u32) -> u32 {
        let mut ps = load(&env);
        ps.push_back(Point { x: a, y: a });
        save(&env, &ps);
        load(&env).len()
    }

    pub fn push_read(env: Env, a: u32) -> u32 {
        let mut ps = load(&env);
        ps.push_back(Point { x: a, y: a });
        save(&env, &ps);
        load(&env).get(0).unwrap().x
    }

    pub fn write_field(env: Env, a: u32) -> u32 {
        let mut ps = load(&env);
        ps.push_back(Point { x: 0, y: 0 });
        let mut p = ps.get(0).unwrap();
        p.x = a;
        ps.set(0, p);
        save(&env, &ps);
        load(&env).get(0).unwrap().x
    }
}

fn load(env: &Env) -> Vec<Point> {
    env.storage()
        .instance()
        .get(&PS)
        .unwrap_or(Vec::new(env))
}

fn save(env: &Env, ps: &Vec<Point>) {
    env.storage().instance().set(&PS, ps);
}
