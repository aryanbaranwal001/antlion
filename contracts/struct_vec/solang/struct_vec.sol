// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// A list of structs in instance storage. `push` appends one and counts the list,
/// `push_read` appends one and reads a field back, `write_field` appends one and then
/// changes one field of it in place.
///
/// Function names match the sdk side so the builds pair by name.
contract Struct_vec {
    struct Point {
        uint32 x;
        uint32 y;
    }

    Point[] instance ps;

    function push(uint32 a) public returns (uint32) {
        ps.push(Point(a, a));
        return uint32(ps.length);
    }

    function push_read(uint32 a) public returns (uint32) {
        ps.push(Point(a, a));
        return ps[0].x;
    }

    function write_field(uint32 a) public returns (uint32) {
        ps.push(Point(0, 0));
        ps[0].x = a;
        return ps[0].x;
    }
}
