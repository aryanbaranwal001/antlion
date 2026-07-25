// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

contract Parity {
    function sum(uint32 n) public pure returns (uint32) {
        uint32 total = 0;
        for (uint32 i = 1; i <= n; i++) {
            if (i % 2 == 0) {
                total += i;
            } else {
                total += 1;
            }
        }
        return total;
    }
}
