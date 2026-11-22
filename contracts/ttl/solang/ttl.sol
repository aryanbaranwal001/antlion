// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One value in each storage class. `put_` writes it, `get_` reads it, giving 0 when it is
/// missing, and `bump_` writes it and extends its TTL. `bump_i` extends the instance, and with
/// it the contract code.
///
/// Every `bump_` extends a temporary value to 3000 ledgers, a persistent one to 6000 and the
/// instance to 9000, so the entries expire one at a time. Each target is also the threshold:
/// a new entry has less left than that, so a `bump_` call always extends.
///
/// The values are `uint64` because Solang has `extendTtl` for a `uint64` variable only.
///
/// Function names match the sdk side so the builds pair by name.
contract Ttl {
    uint64 persistent p;
    uint64 temporary t;
    uint64 instance i;

    function put_p(uint64 v) public returns (uint64) {
        p = v;
        return v;
    }

    function put_t(uint64 v) public returns (uint64) {
        t = v;
        return v;
    }

    function put_i(uint64 v) public returns (uint64) {
        i = v;
        return v;
    }

    function get_p() public view returns (uint64) {
        return p;
    }

    function get_t() public view returns (uint64) {
        return t;
    }

    function get_i() public view returns (uint64) {
        return i;
    }

    function bump_p(uint64 v) public returns (uint64) {
        p = v;
        p.extendTtl(6000, 6000);
        return v;
    }

    function bump_t(uint64 v) public returns (uint64) {
        t = v;
        t.extendTtl(3000, 3000);
        return v;
    }

    function bump_i() public returns (uint64) {
        extendInstanceTtl(9000, 9000);
        return 1;
    }
}
