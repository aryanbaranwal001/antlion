// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// A list at the contract boundary. `len` and `first` only decode one, `make` only encodes
/// one, `id` does both, and `sum` walks every element.
///
/// Function names match the sdk side so the builds pair by name.
contract Vec_id {
    function len(uint32[] memory v) public pure returns (uint32) {
        return uint32(v.length);
    }

    function first(uint32[] memory v) public pure returns (uint32) {
        return v.length == 0 ? 0 : v[0];
    }

    function make(uint32 a) public pure returns (uint32[] memory) {
        uint32[] memory v = new uint32[](3);
        v[0] = a;
        v[1] = a;
        v[2] = a;
        return v;
    }

    function id(uint32[] memory v) public pure returns (uint32[] memory) {
        return v;
    }

    function sum(uint32[] memory v) public pure returns (uint32) {
        uint32 t = 0;
        for (uint32 i = 0; i < v.length; i++) {
            t += v[i];
        }
        return t;
    }
}
