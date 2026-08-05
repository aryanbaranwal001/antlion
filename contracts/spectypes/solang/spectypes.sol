// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

/// The Solidity half of the spec-type probe. Mirrors the sdk contract's function
/// names, minus everything solang's soroban target can't express.
contract Spectypes {
    /// No uint256/int256 — solang's soroban encoder rejects both.
    function ints(uint32, int32, uint64, int64, uint128, int128) public pure returns (bool) {
        return true;
    }

    /// No bytes, bytes32, symbol or muxed address either.
    function words(string memory a, address) public pure returns (string memory) {
        return a;
    }

    /// Solidity has no timepoint or duration, so these are plain uint64.
    function times(uint64 a, uint64) public pure returns (uint64) {
        return a;
    }
}
