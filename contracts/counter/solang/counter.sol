// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

contract Counter {
    uint32 private count;

    function increment() public returns (uint32) {
        count += 1;
        return count;
    }

    function get() public view returns (uint32) {
        return count;
    }
}
