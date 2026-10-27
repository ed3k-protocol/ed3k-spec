// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../../../LICENSE.
pragma solidity ^0.8.24;

import {Base} from "./Base.sol";
import {HoldPayableScenario} from "./Fuzz.t.sol";
import {CreditLedger, IOperators} from "ledger/CreditLedger.sol";

/// Regression tests of the 2026-10-18 audit (F1–F3) and of the follow-up (C5); each failed before the fix.
contract Regressions is HoldPayableScenario {
    // F1: a minter that errors (Subnet-EVM: caller not on the allow list) burns all forwarded gas,
    // so settle runs out of gas instead of "settling all the same" (comment at _mintCoin).
    function test_F1_minterErrorBurnsAllGas() public {
        vm.etch(MINTER_ADDR, hex"fe"); // INVALID: a non-revert error, like a precompile error
        uint32 d = today();
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](3);
        bytes[] memory ss = new bytes[](3);
        for (uint256 i; i < 3; ++i) {
            rs[i] = rc(root, address(uint160(0x5000 + i)), d, 4096, 0);
            ss[i] = sign(rootPk, rs[i]);
        }
        vm.prank(carol); // caller != holder: two mint calls per receipt
        (bool ok,) = address(L).call{gas: 3_000_000}(abi.encodeCall(L.settle, (rs, ss)));
        assertTrue(ok, "settle must not fail because the minter refuses");
    }

    // F2: a hold placed under the 90 % rule is short by 1 µ after one decay step (rounding),
    // and the payee's settlement then flags the payer overdrawn.
    function test_F2_holdShortAfterOneStep() public {
        // earned ≡ 1 (mod 10), received ≡ 1 (mod 10): both lose 0.9 µ to flooring
        uint64 k = 11 * 1024;
        while (value(k) % 10 != 1) ++k;
        fundKib(rootPk, alice, k);
        giveReceived(alice, 1_000_001);
        (uint256 e, uint256 r) = stored(alice);
        assertEq(e % 10, 1);
        assertEq(r % 10, 1);
        uint256 b = e + r;
        uint256 q = b * 90 / 100;
        vm.warp(graceEnd() + 1 days - 1 hours); // no step yet
        vm.prank(alice);
        L.hold(bob, q); // accepted: q fits 90 %
        vm.warp(block.timestamp + 2 hours); // one step, inside HOLD_TIME
        pay(alicePk, bob, uint64(q));
        assertFalse(L.isOverdrawn(alice), "payee's own hold must stay payable in full");
    }

    // F2b: a transfer may leave exactly _reserve(holds); one decay step later the hold is 1 µ short.
    function test_F2b_transferLeavesHoldShort() public {
        uint64 k = 11 * 1024;
        while (value(k) % 10 != 1) ++k;
        fundKib(rootPk, alice, k);
        giveReceived(alice, 5 * UNIT);
        (uint256 e,) = stored(alice);
        uint256 rKeep = 1_000_009; // ≡ 9 (mod 10)
        uint256 h = (e + rKeep) * 9 / 10; // e + rKeep ≡ 0 (mod 10): reserve(h) == e + rKeep
        vm.warp(graceEnd() + 1 days - 1 hours);
        vm.startPrank(alice);
        L.hold(bob, h);
        L.transfer(carol, 5 * UNIT - rKeep); // allowed: leaves exactly reserve(h)
        vm.stopPrank();
        vm.warp(block.timestamp + 2 hours);
        pay(alicePk, bob, uint64(h));
        assertFalse(L.isOverdrawn(alice), "hold must survive a transfer plus one step");
    }

    // F2 (fuzz, exact): the property the spec states.
    /// forge-config: default.fuzz.runs = 20000
    function testFuzz_F2_holdPayableExact(
        uint64 eKib,
        uint256 rMicro,
        uint256 dayOffset,
        uint256 tod,
        uint16 fracQ,
        uint16 fracR,
        uint256 otherPay,
        uint256 dt
    ) public {
        (uint256 q, uint256 taken) = _scenario(eKib, rMicro, dayOffset, tod, fracQ, fracR, otherPay, dt);
        assertEq(taken, q);
    }

    // F3: an operator contract that returns malformed data reverts the whole batch;
    // try/catch does not catch an ABI decoding failure of the return value.
    function test_F3_operatorsMalformedReturnRevertsBatch() public {
        MalformedOps bad = new MalformedOps();
        L = new CreditLedger(IOperators(address(bad)), REWARD, STAKE_RATE, STAKE_DELAY);
        CreditLedger.Receipt memory r = rc(alice, bob, today(), 4096, 0);
        bytes memory s = sign(alicePk, r);
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](1);
        rs[0] = r;
        ss[0] = s;
        (bool ok,) = address(L).call(abi.encodeCall(L.settle, (rs, ss)));
        assertTrue(ok, "a faulty operator contract must read as false, not revert the batch");
    }

    // C5: a caller that sends too little gas starves the capped minter call (EIP-150 gives it
    // 63/64 of the rest) and settle still succeeds: the holder's share is lost. Likewise the
    // isRoot call: a root's receipt then mints nothing. No gas limit may make settle succeed
    // without both mints and the mint of credits.
    function test_C5_noGasLimitStarvesCappedCalls() public {
        BurningMinter bm = new BurningMinter();
        vm.etch(MINTER_ADDR, address(bm).code);
        bm = BurningMinter(MINTER_ADDR);
        fund(alice, 1); // accounts and weeks known; one reward already paid
        uint256 n0 = bm.count();
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](1);
        rs[0] = rc(root, alice, today() - 1, 4096, 0); // a new pair and day: earns the reward
        ss[0] = sign(rootPk, rs[0]);
        bytes memory data = abi.encodeCall(L.settle, (rs, ss));
        uint256 okCount;
        for (uint256 g = 20_000; g <= 400_000; g += 250) {
            uint256 snap = vm.snapshotState();
            vm.prank(carol);
            (bool ok,) = address(L).call{gas: g}(data);
            if (ok) {
                ++okCount;
                assertEq(bm.count(), n0 + 2, "settle succeeded with a starved minter call");
                assertEq(L.balanceOf(alice), 5 * UNIT, "settle succeeded with a starved isRoot call");
            }
            vm.revertToState(snap);
        }
        assertGt(okCount, 0);
    }
}

/// Like the Subnet-EVM minter: costs 30,000 gas, and with less it fails and uses all it was given.
contract BurningMinter {
    uint256 public count;

    function mintNativeCoin(address, uint256) external {
        uint256 start = gasleft();
        while (start - gasleft() < 30_000) {}
        ++count;
    }
}

/// Returns nothing from isRoot (e.g. a proxy whose implementation is gone, or a non-conforming contract).
contract MalformedOps {
    fallback() external {}
}
