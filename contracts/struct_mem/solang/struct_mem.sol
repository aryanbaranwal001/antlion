// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// A struct built in the function body and never stored. Values come from the argument,
/// but that does not keep the struct alive: rustc replaces it with its fields, so these
/// rows measure whether each compiler materialises a local aggregate at all.
///
/// Function names match the sdk side so the builds pair by name.
contract Struct_mem {
    struct Point {
        uint32 x;
        uint32 y;
    }

    struct Outer {
        Point inner;
        uint32 tag;
    }

    function build(uint32 a) public pure returns (uint32) {
        Point memory p = Point(a, a);
        return p.x + p.y;
    }

    function nested(uint32 a) public pure returns (uint32) {
        Outer memory o = Outer(Point(a, a), a);
        return o.inner.x + o.inner.y + o.tag;
    }
}
