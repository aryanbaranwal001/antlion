// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// Strings and bytes at the contract boundary. `_id` decodes one and encodes it back,
/// `_len` only decodes one, `bytes_make` only encodes one, and `bytes_first` reads one byte,
/// which fails on an empty value.
///
/// Function names match the sdk side so the builds pair by name.
contract Bytes_id {
    function string_id(string memory a) public pure returns (string memory) {
        return a;
    }

    function string_len(string memory a) public pure returns (uint32) {
        return uint32(bytes(a).length);
    }

    function bytes_id(bytes memory a) public pure returns (bytes memory) {
        return a;
    }

    function bytes_len(bytes memory a) public pure returns (uint32) {
        return uint32(a.length);
    }

    function bytes_first(bytes memory a) public pure returns (uint32) {
        return uint32(uint8(a[0]));
    }

    /// Solang has no `uint8` on Soroban and rejects `bytes1(uint8(a))`, so the low byte is
    /// taken from `bytes4(a)`.
    function bytes_make(uint32 a) public pure returns (bytes memory) {
        bytes4 x = bytes4(a);
        bytes memory b = new bytes(3);
        b[0] = x[3];
        b[1] = x[3];
        b[2] = x[3];
        return b;
    }

    function bytes32_id(bytes32 a) public pure returns (bytes32) {
        return a;
    }
}
