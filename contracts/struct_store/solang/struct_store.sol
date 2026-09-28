// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// A four field struct in instance storage, one field and all four, written and read.
/// Storage forces both sides to materialise the struct, so nothing folds away.
///
/// Function names match the sdk side so the builds pair by name.
contract Struct_store {
    struct Point {
        uint32 x;
        uint32 y;
        uint32 z;
        uint32 w;
    }

    Point instance p;

    function write_one(uint32 a) public {
        p.x = a;
    }

    function write_all(uint32 a) public {
        p.x = a;
        p.y = a;
        p.z = a;
        p.w = a;
    }

    function read_one() public view returns (uint32) {
        return p.x;
    }

    function read_all() public view returns (uint32) {
        return p.x + p.y + p.z + p.w;
    }
}
