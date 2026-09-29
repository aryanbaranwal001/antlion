// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// A struct at the contract boundary. `get_x` only decodes one, `make` only encodes one,
/// `id` and `id_outer` do both, so each direction can be told apart.
///
/// Function names match the sdk side so the builds pair by name.
contract Struct_id {
    struct Point {
        uint32 x;
        uint32 y;
    }

    struct Outer {
        Point inner;
        uint32 tag;
    }

    function get_x(Point memory p) public pure returns (uint32) {
        return p.x;
    }

    function make(uint32 a) public pure returns (Point memory) {
        return Point(a, a);
    }

    function id(Point memory p) public pure returns (Point memory) {
        return p;
    }

    function id_outer(Outer memory o) public pure returns (Outer memory) {
        return o;
    }
}
