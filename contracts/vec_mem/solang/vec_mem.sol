// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// A list built inside the call and never stored. The two sides do not put it in the
/// same place: solidity allocates it in the wasm's own linear memory, while an sdk `Vec`
/// is a handle to an object that lives on the host.
///
/// Length comes from the argument so nothing folds to a constant.
///
/// Function names match the sdk side so the builds pair by name.
contract Vec_mem {
    function push(uint32 n) public pure returns (uint32) {
        return uint32(build(n).length);
    }

    function sum(uint32 n) public pure returns (uint32) {
        uint32[] memory v = build(n);
        uint32 t = 0;
        for (uint32 i = 0; i < v.length; i++) {
            t += v[i];
        }
        return t;
    }

    function first(uint32 n) public pure returns (uint32) {
        uint32[] memory v = build(n);
        return v.length == 0 ? 0 : v[0];
    }

    function build(uint32 n) private pure returns (uint32[] memory) {
        uint32[] memory v = new uint32[](n);
        for (uint32 i = 0; i < n; i++) {
            v[i] = i;
        }
        return v;
    }
}
