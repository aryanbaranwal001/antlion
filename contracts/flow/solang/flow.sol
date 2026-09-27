// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// One control-flow construct per function, every trip count taken from the argument so
/// nothing folds to a constant. Subtract `scalar_id`'s `u32_id` to drop the boundary.
///
/// Function names match the sdk side so the builds pair by name.
contract Flow {
    function branch(uint32 n) public pure returns (uint32) {
        if (n % 2 == 0) {
            return n / 2;
        } else {
            return n + 1;
        }
    }

    function loop_(uint32 n) public pure returns (uint32) {
        uint32 t = 0;
        for (uint32 i = 0; i < n; i++) {
            t += i;
        }
        return t;
    }

    function while_(uint32 n) public pure returns (uint32) {
        uint32 t = 0;
        uint32 i = 0;
        while (i < n) {
            t += i;
            i++;
        }
        return t;
    }

    function nested(uint32 n) public pure returns (uint32) {
        uint32 t = 0;
        for (uint32 i = 0; i < n; i++) {
            for (uint32 j = 0; j < n; j++) {
                t += 1;
            }
        }
        return t;
    }

    function early(uint32 n) public pure returns (uint32) {
        uint32 t = 0;
        for (uint32 i = 0; i < n; i++) {
            if (i % 3 == 0) {
                continue;
            }
            if (i == 9) {
                break;
            }
            t += i;
        }
        return t;
    }
}
