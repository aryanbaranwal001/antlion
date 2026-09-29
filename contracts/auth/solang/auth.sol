// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// `requireAuth` on an address argument. `no_auth` is the control and never asks;
/// `need_auth` asks and answers `1`; `auth_store` asks, then stores `a` under the address,
/// as a balance would be.
///
/// Function names match the sdk side so the builds pair by name.
contract Auth {
    mapping(address => uint32) instance bal;

    function no_auth(address user) public pure returns (uint32) {
        user;
        return 1;
    }

    function need_auth(address user) public returns (uint32) {
        user.requireAuth();
        return 1;
    }

    function auth_store(address user, uint32 a) public returns (uint32) {
        user.requireAuth();
        bal[user] = a;
        return bal[user];
    }
}
