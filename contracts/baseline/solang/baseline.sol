// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// The empty contract — no functions at all. Whatever this weighs is the fixed cost of
/// being a contract: spec section, env metadata, allocator, host imports.
///
/// It has to be empty. A baseline with even one function is a baseline plus that
/// function's export entry, spec entry and body, so subtracting it from another contract
/// would under-count by one function every time.
contract Baseline { }
