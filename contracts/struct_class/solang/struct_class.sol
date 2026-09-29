// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One two field struct written whole and read back through each durability class.
/// `storage` does this for a `uint32`; this checks the storage class annotation holds for
/// a struct too.
///
/// Function names match the sdk side so the builds pair by name.
contract Struct_class {
    struct Point {
        uint32 x;
        uint32 y;
    }

    Point instance inst_p;
    Point persistent pers_p;
    Point temporary temp_p;

    function put_instance(uint32 a) public returns (uint32) {
        inst_p = Point(a, a);
        return inst_p.x;
    }

    function put_persistent(uint32 a) public returns (uint32) {
        pers_p = Point(a, a);
        return pers_p.x;
    }

    function put_temporary(uint32 a) public returns (uint32) {
        temp_p = Point(a, a);
        return temp_p.x;
    }
}
