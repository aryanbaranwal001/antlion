#![no_std]
use soroban_sdk::{Bytes, BytesN, Env, String, contract, contractimpl};

#[contract]
pub struct BytesId;

/// Strings and bytes at the contract boundary. `_id` decodes one and encodes it back,
/// `_len` only decodes one, `bytes_make` only encodes one, and `bytes_first` reads one byte,
/// which fails on an empty value.
#[contractimpl]
impl BytesId {
    pub fn string_id(a: String) -> String {
        a
    }

    pub fn string_len(a: String) -> u32 {
        a.len()
    }

    pub fn bytes_id(a: Bytes) -> Bytes {
        a
    }

    pub fn bytes_len(a: Bytes) -> u32 {
        a.len()
    }

    pub fn bytes_first(a: Bytes) -> u32 {
        a.get(0).unwrap() as u32
    }

    pub fn bytes_make(env: Env, a: u32) -> Bytes {
        Bytes::from_array(&env, &[a as u8; 3])
    }

    pub fn bytes32_id(a: BytesN<32>) -> BytesN<32> {
        a
    }
}
