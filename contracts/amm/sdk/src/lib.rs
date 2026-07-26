#![no_std]
use soroban_sdk::{Env, Symbol, contract, contractimpl, symbol_short};

const X: Symbol = symbol_short!("X");
const Y: Symbol = symbol_short!("Y");

#[contract]
pub struct AmmContract;

#[contractimpl]
impl AmmContract {
    /// Seed the pool with its starting reserves.
    pub fn init(env: Env, x0: u64, y0: u64) {
        env.storage().instance().set(&X, &x0);
        env.storage().instance().set(&Y, &y0);
    }

    /// Add liquidity to both sides.
    pub fn deposit(env: Env, dx: u64, dy: u64) {
        let x = load(&env, &X) + dx;
        let y = load(&env, &Y) + dy;
        env.storage().instance().set(&X, &x);
        env.storage().instance().set(&Y, &y);
    }

    /// Pay `dx` of X, receive Y. Returns the amount paid out.
    pub fn swap_x(env: Env, dx: u64) -> u64 {
        let x = load(&env, &X);
        let y = load(&env, &Y);

        let dy = quote(dx, x, y);
        env.storage().instance().set(&X, &(x + dx));
        env.storage().instance().set(&Y, &(y - dy));
        dy
    }

    /// Pay `dy` of Y, receive X. Returns the amount paid out.
    pub fn swap_y(env: Env, dy: u64) -> u64 {
        let x = load(&env, &X);
        let y = load(&env, &Y);

        let dx = quote(dy, y, x);
        env.storage().instance().set(&Y, &(y + dy));
        env.storage().instance().set(&X, &(x - dx));
        dx
    }

    /// Output for `dx` of the input side, without touching the pool.
    pub fn price(env: Env, dx: u64) -> u64 {
        quote(dx, load(&env, &X), load(&env, &Y))
    }

    pub fn reserve_x(env: Env) -> u64 {
        load(&env, &X)
    }

    pub fn reserve_y(env: Env) -> u64 {
        load(&env, &Y)
    }

    /// The constant product the pool is meant to hold.
    pub fn k(env: Env) -> u64 {
        load(&env, &X) * load(&env, &Y)
    }
}

fn load(env: &Env, key: &Symbol) -> u64 {
    env.storage().instance().get(key).unwrap_or(0)
}

/// Constant-product output: `dy = out * dx' / (inn + dx')`, where `dx'` is `dx` less
/// the 0.3% fee. Zero when the pool is empty.
fn quote(dx: u64, inn: u64, out: u64) -> u64 {
    let paid = dx * 997 / 1000;
    if inn + paid == 0 {
        return 0;
    }
    out * paid / (inn + paid)
}
