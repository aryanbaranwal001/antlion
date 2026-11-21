// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One function per event shape: no fields, one data field, two data fields, an indexed
/// field with one data field, and two indexed fields with two data fields. `many` emits
/// three events in one call, and `emit_fail` emits one and then fails, so the event belongs
/// to a call that failed.
///
/// Function names match the sdk side so the builds pair by name.
contract Events {
    event Ping();
    event Value(uint32 v);
    event Keyed(uint32 indexed k, uint32 v);
    event Two(uint32 v, uint32 w);
    event Pair(address indexed from, uint32 indexed k, uint32 v, uint32 w);

    function ping() public returns (uint32) {
        emit Ping();
        return 1;
    }

    function value(uint32 v) public returns (uint32) {
        emit Value(v);
        return v;
    }

    function keyed(uint32 k, uint32 v) public returns (uint32) {
        emit Keyed(k, v);
        return v;
    }

    function two(uint32 v) public returns (uint32) {
        emit Two(v, v + 1);
        return v;
    }

    function pair(address from, uint32 k, uint32 v) public returns (uint32) {
        emit Pair(from, k, v, v + 1);
        return v;
    }

    function many(uint32 v) public returns (uint32) {
        emit Value(v);
        emit Value(v + 1);
        emit Value(v + 2);
        return v;
    }

    function emit_fail(uint32 v) public returns (uint32) {
        emit Value(v);
        require(v >= 10, "too small");
        return v;
    }
}
