// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// A `u32` mapping and a struct mapping in instance storage. `put` and `put_p` write a key
/// and read it back; `get` and `get_px` read a key nothing wrote.
///
/// Function names match the sdk side so the builds pair by name.
contract Mapping {
    struct Point {
        uint32 x;
        uint32 y;
    }

    mapping(uint32 => uint32) instance m;
    mapping(uint32 => Point) instance mp;

    function put(uint32 k, uint32 v) public returns (uint32) {
        m[k] = v;
        return m[k];
    }

    function get(uint32 k) public view returns (uint32) {
        return m[k];
    }

    function put_p(uint32 k, uint32 a) public returns (uint32) {
        mp[k] = Point(a, a);
        return mp[k].x;
    }

    function get_px(uint32 k) public view returns (uint32) {
        return mp[k].x;
    }
}
