// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// Twenty state variables, each written and read back by its own function. A mapping, an
/// array and a struct come first, then seventeen `uint32`s, so this pair shows which key
/// each kind of variable is given and where the key sequence stops being usable.
///
/// The aggregates are declared first on purpose: solang writes them at deploy, so one
/// placed past the limit would make every call fail and hide the rest of the pair.
///
/// Function names match the sdk side so the builds pair by name.
contract Slots {
    struct Point {
        uint32 x;
        uint32 y;
    }

    mapping(uint32 => uint32) instance m;
    uint32[] instance xs;
    Point instance p;
    uint32 instance v03;
    uint32 instance v04;
    uint32 instance v05;
    uint32 instance v06;
    uint32 instance v07;
    uint32 instance v08;
    uint32 instance v09;
    uint32 instance v10;
    uint32 instance v11;
    uint32 instance v12;
    uint32 instance v13;
    uint32 instance v14;
    uint32 instance v15;
    uint32 instance v16;
    uint32 instance v17;
    uint32 instance v18;
    uint32 instance v19;

    function put_m(uint32 a) public returns (uint32) {
        m[a] = a;
        return m[a];
    }

    function put_xs(uint32 a) public returns (uint32) {
        xs.push(a);
        return xs[xs.length - 1];
    }

    function put_p(uint32 a) public returns (uint32) {
        p.x = a;
        return p.x;
    }

    function put_v03(uint32 a) public returns (uint32) {
        v03 = a;
        return v03;
    }

    function put_v04(uint32 a) public returns (uint32) {
        v04 = a;
        return v04;
    }

    function put_v05(uint32 a) public returns (uint32) {
        v05 = a;
        return v05;
    }

    function put_v06(uint32 a) public returns (uint32) {
        v06 = a;
        return v06;
    }

    function put_v07(uint32 a) public returns (uint32) {
        v07 = a;
        return v07;
    }

    function put_v08(uint32 a) public returns (uint32) {
        v08 = a;
        return v08;
    }

    function put_v09(uint32 a) public returns (uint32) {
        v09 = a;
        return v09;
    }

    function put_v10(uint32 a) public returns (uint32) {
        v10 = a;
        return v10;
    }

    function put_v11(uint32 a) public returns (uint32) {
        v11 = a;
        return v11;
    }

    function put_v12(uint32 a) public returns (uint32) {
        v12 = a;
        return v12;
    }

    function put_v13(uint32 a) public returns (uint32) {
        v13 = a;
        return v13;
    }

    function put_v14(uint32 a) public returns (uint32) {
        v14 = a;
        return v14;
    }

    function put_v15(uint32 a) public returns (uint32) {
        v15 = a;
        return v15;
    }

    function put_v16(uint32 a) public returns (uint32) {
        v16 = a;
        return v16;
    }

    function put_v17(uint32 a) public returns (uint32) {
        v17 = a;
        return v17;
    }

    function put_v18(uint32 a) public returns (uint32) {
        v18 = a;
        return v18;
    }

    function put_v19(uint32 a) public returns (uint32) {
        v19 = a;
        return v19;
    }
}
