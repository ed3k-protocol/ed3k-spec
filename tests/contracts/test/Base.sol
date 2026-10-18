// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../../../LICENSE.
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {CreditLedger, IOperators} from "ledger/CreditLedger.sol";

/// Records mintNativeCoin calls.
contract MockMinter {
    uint256 public count;
    mapping(uint256 => uint256) private _c; // to | amount << 160

    function mintNativeCoin(address to, uint256 amount) external {
        _c[count++] = uint160(to) | (amount << 160);
    }

    function tos(uint256 i) external view returns (address) {
        return address(uint160(_c[i]));
    }

    function amounts(uint256 i) external view returns (uint256) {
        return _c[i] >> 160;
    }
}

contract MockOperators is IOperators {
    mapping(address => bool) public roots;

    function set(address a, bool b) external {
        roots[a] = b;
    }

    function isRoot(address a) external view returns (bool) {
        return roots[a];
    }
}

abstract contract Base is Test {
    address constant MINTER_ADDR = 0x0200000000000000000000000000000000000001;
    uint256 constant UNIT = 1e6;
    uint256 constant REWARD = 1e15;
    uint256 constant T0 = 1_759_968_000 + 12 hours; // midday of day 20370 (a literal: constant division would be exact)

    CreditLedger L;
    MockOperators ops;
    MockMinter minter;

    uint256 rootPk = 0xA11CE01;
    uint256 root2Pk = 0xA11CE02;
    uint256 root3Pk = 0xA11CE03;
    uint256 alicePk = 0xB0B01;
    uint256 bobPk = 0xB0B02;
    uint256 carolPk = 0xB0B03;
    uint256 davePk = 0xB0B04;
    address root;
    address root2;
    address root3;
    address alice;
    address bob;
    address carol;
    address dave;

    function setUp() public virtual {
        vm.warp(T0);
        ops = new MockOperators();
        MockMinter m = new MockMinter();
        vm.etch(MINTER_ADDR, address(m).code);
        minter = MockMinter(MINTER_ADDR);
        L = new CreditLedger(IOperators(address(ops)), REWARD);
        root = vm.addr(rootPk);
        root2 = vm.addr(root2Pk);
        root3 = vm.addr(root3Pk);
        alice = vm.addr(alicePk);
        bob = vm.addr(bobPk);
        carol = vm.addr(carolPk);
        dave = vm.addr(davePk);
        ops.set(root, true);
        ops.set(root2, true);
        ops.set(root3, true);
    }

    // ───────── helpers ─────────

    function today() internal view returns (uint32) {
        return uint32(block.timestamp / 1 days);
    }

    function rc(address i, address h, uint32 day, uint64 kib, uint64 spent)
        internal
        pure
        returns (CreditLedger.Receipt memory r)
    {
        r = CreditLedger.Receipt(i, h, day, kib, spent);
    }

    function sign(uint256 pk, CreditLedger.Receipt memory r) internal view returns (bytes memory) {
        (uint8 v, bytes32 rr, bytes32 s) = vm.sign(pk, L.receiptDigest(r));
        return abi.encodePacked(rr, s, v);
    }

    function settle1(uint256 pk, CreditLedger.Receipt memory r) internal {
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](1);
        rs[0] = r;
        ss[0] = sign(pk, r);
        L.settle(rs, ss);
    }

    function settleRaw(CreditLedger.Receipt memory r, bytes memory s) internal {
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](1);
        rs[0] = r;
        ss[0] = s;
        L.settle(rs, ss);
    }

    /// Mints `kib` more to `h` from root issuer `pk`, today.
    function fundKib(uint256 pk, address h, uint64 kib) internal {
        address i = vm.addr(pk);
        (uint64 k, uint64 sp) = L.settled(i, h, today());
        settle1(pk, rc(i, h, today(), k + kib, sp));
    }

    /// Mints `credits` to `h` (1024 KiB each) from root.
    function fund(address h, uint256 credits) internal {
        fundKib(rootPk, h, uint64(credits * 1024));
    }

    /// Issuer `pk` pays `amount` µ more to `h` today (kib unchanged).
    function pay(uint256 pk, address h, uint64 amount) internal {
        payOn(pk, h, today(), amount);
    }

    function payOn(uint256 pk, address h, uint32 day, uint64 amount) internal {
        address i = vm.addr(pk);
        (uint64 k, uint64 sp) = L.settled(i, h, day);
        settle1(pk, rc(i, h, day, k, sp + amount));
    }

    /// Gives `who` exactly `r` µ of received: a funded helper pays 10·r to it.
    function giveReceived(address who, uint256 r) internal {
        uint256 helperPk = uint256(keccak256(abi.encode("helper", who, r, block.timestamp)));
        address helper = vm.addr(helperPk);
        fund(helper, (r * 10) / UNIT + 2);
        pay(helperPk, who, uint64(r * 10));
    }

    function stored(address a) internal view returns (uint256 earned, uint256 received) {
        bytes32 slot = keccak256(abi.encode(a, uint256(0)));
        uint256 v = uint256(vm.load(address(L), slot));
        earned = uint128(v);
        received = v >> 128;
    }

    function heldTotal(address a) internal view returns (uint256) {
        return uint256(vm.load(address(L), keccak256(abi.encode(a, uint256(9)))));
    }

    function holdAmt(address payer, address payee) internal view returns (uint256 a) {
        (a,) = L.holdOf(payer, payee);
    }

    function signHold(uint256 pk, address payer, address payee, uint256 amount, uint256 nonce, uint256 deadline)
        internal
        view
        returns (bytes memory)
    {
        bytes32 sh = keccak256(abi.encode(L.HOLD_TYPEHASH(), payer, payee, amount, nonce, deadline));
        bytes32 d = keccak256(abi.encodePacked("\x19\x01", L.DOMAIN_SEPARATOR(), sh));
        (uint8 v, bytes32 rr, bytes32 s) = vm.sign(pk, d);
        return abi.encodePacked(rr, s, v);
    }

    function value(uint256 kib) internal pure returns (uint256) {
        return kib * 15625 / 16;
    }

    /// Start of the first decay step boundary after the grace of an account active today.
    function graceEnd() internal view returns (uint256) {
        return uint256(today()) * 1 days + 3 days;
    }

    uint256 constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;

    /// The high-s twin of a valid 65-byte signature (same signer).
    function _malleate(bytes memory sig) internal pure returns (bytes memory) {
        bytes32 r;
        bytes32 s;
        uint8 v = uint8(sig[64]);
        assembly {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
        }
        bytes32 s2 = bytes32(SECP_N - uint256(s));
        uint8 v2 = v == 27 ? 28 : 27;
        return abi.encodePacked(r, s2, v2);
    }
}
