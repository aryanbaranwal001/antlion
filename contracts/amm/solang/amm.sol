// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

contract Amm {
    uint64 private x;
    uint64 private y;

    /// Seed the pool with its starting reserves.
    function init(uint64 x0, uint64 y0) public {
        x = x0;
        y = y0;
    }

    /// Add liquidity to both sides.
    function deposit(uint64 dx, uint64 dy) public {
        x += dx;
        y += dy;
    }

    /// Pay `dx` of X, receive Y. Returns the amount paid out.
    function swap_x(uint64 dx) public returns (uint64) {
        uint64 dy = quote(dx, x, y);
        x += dx;
        y -= dy;
        return dy;
    }

    /// Pay `dy` of Y, receive X. Returns the amount paid out.
    function swap_y(uint64 dy) public returns (uint64) {
        uint64 dx = quote(dy, y, x);
        y += dy;
        x -= dx;
        return dx;
    }

    /// Output for `dx` of the input side, without touching the pool.
    function price(uint64 dx) public view returns (uint64) {
        return quote(dx, x, y);
    }

    function reserve_x() public view returns (uint64) {
        return x;
    }

    function reserve_y() public view returns (uint64) {
        return y;
    }

    /// The constant product the pool is meant to hold.
    function k() public view returns (uint64) {
        return x * y;
    }

    /// Constant-product output: `dy = out * dx' / (inn + dx')`, where `dx'` is `dx`
    /// less the 0.3% fee. Zero when the pool is empty.
    function quote(uint64 dx, uint64 inn, uint64 out) private pure returns (uint64) {
        uint64 paid = (dx * 997) / 1000;
        if (inn + paid == 0) {
            return 0;
        }
        return (out * paid) / (inn + paid);
    }
}
