// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../../../LICENSE.
pragma solidity ^0.8.24;

import {Base, MockOperators} from "./Base.sol";
import {CreditLedger, IOperators} from "ledger/CreditLedger.sol";

/// Re-enters the ledger with stored calldata on its first call.
contract ReentrantMinter {
    address public target;
    bytes public data;
    bool public entered;
    bool public innerOk;

    function arm(address t, bytes calldata d) external {
        target = t;
        data = d;
    }

    function mintNativeCoin(address, uint256) external {
        if (entered || target == address(0)) return;
        entered = true;
        (innerOk,) = target.call(data);
    }
}

contract SettleTest is Base {
    event Registered(address indexed account);
    event Settle(address indexed issuer, address indexed holder, uint32 day, uint256 minted, uint256 taken);
    event Overdrawn(address indexed account, uint256 missing);

    function test_firstReceipt_mintsRegistersRewards() public {
        CreditLedger.Receipt memory r = rc(root, alice, today(), 2048, 0);
        vm.expectEmit(true, false, false, false);
        emit Registered(root);
        vm.expectEmit(true, false, false, false);
        emit Registered(alice);
        bytes memory sg = sign(rootPk, r);
        vm.prank(carol);
        settleRaw(r, sg);
        assertEq(L.balanceOf(alice), 2 * UNIT);
        assertEq(L.totalSupply(), 2 * UNIT);
        (uint64 k, uint64 sp) = L.settled(root, alice, today());
        assertEq(k, 2048);
        assertEq(sp, 0);
        assertEq(L.issuedKib(root, today() / 7), 2048);
        assertEq(L.pairKib(root, alice, today() / 7), 2048);
        CreditLedger.AccountView memory v = L.accountOf(alice);
        assertEq(v.lastActive, today());
        assertEq(v.inflow, 2048);
        assertEq(v.issuers, 1);
        assertEq(v.registeredAt, block.timestamp);
        assertEq(L.accountOf(root).registeredAt, block.timestamp);
        // reward: 60 % caller, 40 % holder
        assertEq(minter.count(), 2);
        assertEq(minter.tos(0), carol);
        assertEq(minter.amounts(0), REWARD * 60 / 100);
        assertEq(minter.tos(1), alice);
        assertEq(minter.amounts(1), REWARD - REWARD * 60 / 100);
    }

    function test_reward_allToHolderWhenCaller() public {
        CreditLedger.Receipt memory r = rc(root, alice, today(), 1024, 0);
        bytes memory s = sign(rootPk, r);
        vm.prank(alice);
        settleRaw(r, s);
        assertEq(minter.count(), 1);
        assertEq(minter.tos(0), alice);
        assertEq(minter.amounts(0), REWARD);
    }

    function test_reward_oncePerSettle() public {
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](3);
        bytes[] memory sg = new bytes[](3);
        rs[0] = rc(root, alice, today(), 512, 0);   // below 1 credit: earns nothing
        rs[1] = rc(root, bob, today(), 2048, 0);    // first that earns: bob gets the holder share
        rs[2] = rc(root, alice, today(), 4096, 0);
        for (uint256 i = 0; i < 3; ++i) sg[i] = sign(rootPk, rs[i]);
        vm.prank(carol);
        L.settle(rs, sg);
        assertEq(minter.count(), 2);
        assertEq(minter.tos(0), carol);
        assertEq(minter.amounts(0), REWARD * 60 / 100);
        assertEq(minter.tos(1), bob);
        assertEq(minter.amounts(1), REWARD - REWARD * 60 / 100);
    }

    function test_reward_noneBelowOneCredit() public {
        settle1(rootPk, rc(root, alice, today(), 1023, 0));
        assertEq(minter.count(), 0);
        assertEq(L.balanceOf(alice), value(1023));
    }

    function test_reward_zeroGasRewardNoCall() public {
        L = new CreditLedger(IOperators(address(ops)), 0);
        settle1(rootPk, rc(root, alice, today(), 4096, 0));
        assertEq(minter.count(), 0);
        assertEq(L.balanceOf(alice), 4 * UNIT);
    }

    function test_laterReceipt_mintsDeltaOnly() public {
        settle1(rootPk, rc(root, alice, today(), 1000, 0));
        settle1(rootPk, rc(root, alice, today(), 3000, 0));
        assertEq(L.balanceOf(alice), value(3000));
        settle1(rootPk, rc(root, alice, today(), 3001, 0));
        assertEq(L.balanceOf(alice), value(3001)); // cumulative rounding
    }

    function test_roundingDoesNotAddUp() public {
        for (uint64 k = 1; k <= 33; ++k) {
            settle1(rootPk, rc(root, alice, today(), k, 0));
        }
        assertEq(L.balanceOf(alice), value(33));
    }

    function test_newDay_separateCounters() public {
        settle1(rootPk, rc(root, alice, today(), 5000, 0));
        vm.warp(block.timestamp + 1 days);
        settle1(rootPk, rc(root, alice, today(), 1000, 0)); // lower than yesterday: own day
        assertEq(L.balanceOf(alice), value(5000) + value(1000));
        // yesterday can still grow
        settle1(rootPk, rc(root, alice, today() - 1, 6000, 0));
        assertEq(L.balanceOf(alice), value(6000) + value(1000));
    }

    function test_skipped_notReverted() public {
        uint32 d = today();
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](12);
        bytes[] memory ss = new bytes[](12);
        // 0: good
        rs[0] = rc(root, alice, d, 1024, 0);
        ss[0] = sign(rootPk, rs[0]);
        // 1: signed by someone else
        rs[1] = rc(root, bob, d, 1024, 0);
        ss[1] = sign(alicePk, rs[1]);
        // 2: issuer == holder
        rs[2] = rc(root, root, d, 1024, 0);
        ss[2] = sign(rootPk, rs[2]);
        // 3: zero holder
        rs[3] = rc(root, address(0), d, 1024, 0);
        ss[3] = sign(rootPk, rs[3]);
        // 4: future day
        rs[4] = rc(root, carol, d + 1, 1024, 0);
        ss[4] = sign(rootPk, rs[4]);
        // 5: 8 days back
        rs[5] = rc(root, carol, d - 8, 1024, 0);
        ss[5] = sign(rootPk, rs[5]);
        // 6: kib above max
        rs[6] = rc(root, carol, d, (50 << 20) + 1, 0);
        ss[6] = sign(rootPk, rs[6]);
        // 7: 64-byte sig
        rs[7] = rc(root, dave, d, 1024, 0);
        bytes memory s7 = sign(rootPk, rs[7]);
        bytes memory s64 = new bytes(64);
        for (uint256 j; j < 64; ++j) {
            s64[j] = s7[j];
        }
        ss[7] = s64;
        // 8: high-s twin of a valid signature
        rs[8] = rc(root, dave, d, 1024, 0);
        ss[8] = _malleate(sign(rootPk, rs[8]));
        // 9: v = 29
        rs[9] = rc(root, dave, d, 1024, 0);
        ss[9] = sign(rootPk, rs[9]);
        ss[9][64] = bytes1(uint8(29));
        // 10: zero issuer, garbage sig
        rs[10] = rc(address(0), dave, d, 1024, 0);
        ss[10] = new bytes(65);
        // 11: 7 days back is still fine
        rs[11] = rc(root, carol, d - 7, 2048, 0);
        ss[11] = sign(rootPk, rs[11]);
        L.settle(rs, ss);
        assertEq(L.balanceOf(alice), UNIT);
        assertEq(L.balanceOf(bob), 0);
        assertEq(L.balanceOf(carol), 2 * UNIT);
        assertEq(L.balanceOf(dave), 0);
        assertEq(L.accountOf(dave).registeredAt, 0);
        assertEq(L.totalSupply(), 3 * UNIT);
    }

    function test_skip_lowerCounters() public {
        settle1(rootPk, rc(root, alice, today(), 5000, 0));
        settle1(rootPk, rc(root, alice, today(), 4000, 0));
        assertEq(L.balanceOf(alice), value(5000));
        // equal receipt: no change, no revert, no reward
        uint256 n = minter.count();
        settle1(rootPk, rc(root, alice, today(), 5000, 0));
        assertEq(minter.count(), n);
    }

    function test_skip_lowerSpent() public {
        fund(alice, 100);
        pay(alicePk, bob, 5 * uint64(UNIT));
        uint256 b = L.balanceOf(alice);
        settle1(alicePk, rc(alice, bob, today(), 0, 4 * uint64(UNIT)));
        assertEq(L.balanceOf(alice), b);
    }

    function test_lengthMismatch() public {
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](0);
        vm.expectRevert(CreditLedger.LengthMismatch.selector);
        L.settle(rs, ss);
    }

    function test_registration_zeroReceipt() public {
        settle1(alicePk, rc(alice, bob, today(), 0, 0));
        assertEq(L.accountOf(alice).registeredAt, block.timestamp);
        assertEq(L.accountOf(bob).registeredAt, block.timestamp);
        assertEq(minter.count(), 0);
    }

    // ───────── maturity ─────────

    function _matureAlice() internal returns (uint256 t0) {
        t0 = block.timestamp;
        settle1(alicePk, rc(alice, bob, today(), 0, 0)); // registers alice
        fundKib(rootPk, alice, 400_000);
        fundKib(root2Pk, alice, 400_000);
        fundKib(root3Pk, alice, (1 << 20) - 800_000);
    }

    function test_maturity_allRules() public {
        uint256 t0 = _matureAlice();
        assertFalse(L.isMature(alice)); // age
        vm.warp(t0 + 30 days - 1);
        assertFalse(L.isMature(alice));
        vm.warp(t0 + 30 days);
        assertTrue(L.isMature(alice));
        assertTrue(L.isMature(root));
    }

    function test_maturity_needsThreeIssuers() public {
        uint256 t0 = block.timestamp;
        settle1(alicePk, rc(alice, bob, today(), 0, 0));
        fundKib(rootPk, alice, 600_000);
        fundKib(root2Pk, alice, 600_000);
        vm.warp(t0 + 31 days);
        assertFalse(L.isMature(alice));
        fundKib(root3Pk, alice, 1);
        assertTrue(L.isMature(alice));
    }

    function test_maturity_needsInflow() public {
        uint256 t0 = block.timestamp;
        settle1(alicePk, rc(alice, bob, today(), 0, 0));
        fundKib(rootPk, alice, 300_000);
        fundKib(root2Pk, alice, 300_000);
        fundKib(root3Pk, alice, 300_000);
        vm.warp(t0 + 31 days);
        assertFalse(L.isMature(alice));
    }

    function test_immatureIssuer_kibStays_thenMints() public {
        uint256 t0 = _matureAlice();
        vm.warp(t0 + 26 days);
        uint32 d = today();
        CreditLedger.Receipt memory r = rc(alice, carol, d, 4096, 0);
        bytes memory s = sign(alicePk, r);
        settleRaw(r, s);
        assertEq(L.balanceOf(carol), 0);
        (uint64 k,) = L.settled(alice, carol, d);
        assertEq(k, 0);
        assertEq(L.accountOf(carol).registeredAt, block.timestamp); // registered anyway
        vm.warp(t0 + 30 days);
        settleRaw(r, s);
        assertEq(L.balanceOf(carol), 4 * UNIT);
    }

    // ───────── limits ─────────

    function test_pairCap() public {
        uint32 d = today();
        settle1(rootPk, rc(root, alice, d, 10 << 20, 0));
        assertEq(L.balanceOf(alice), value(10 << 20));
        // another day of the same week: nothing more; kib still advances (lost)
        vm.warp(block.timestamp + 1 days);
        if (today() / 7 != d / 7) vm.warp(block.timestamp + 7 days); // keep the test meaningful
        uint32 d2 = today();
        uint256 before = L.balanceOf(alice);
        settle1(rootPk, rc(root, alice, d2, 1024, 0));
        if (d2 / 7 == d / 7) {
            assertEq(L.balanceOf(alice), before);
            (uint64 k,) = L.settled(root, alice, d2);
            assertEq(k, 1024);
        }
    }

    function test_pairCap_partialGrantThenNextWeek() public {
        // place at the first day of a week
        vm.warp((uint256(today()) / 7 * 7 + 7) * 1 days + 1 hours);
        uint32 d = today();
        settle1(rootPk, rc(root, alice, d, (10 << 20) - 512, 0));
        settle1(rootPk, rc(root, alice, d, (10 << 20) + 4096, 0)); // only 512 fit
        assertEq(L.pairKib(root, alice, d / 7), 10 << 20);
        assertEq(L.balanceOf(alice), value(10 << 20));
        vm.warp(block.timestamp + 7 days);
        uint256 b = L.balanceOf(alice); // decayed by now
        settle1(rootPk, rc(root, alice, today(), 2048, 0));
        assertEq(L.balanceOf(alice), b + 2 * UNIT);
    }

    function test_weekCap() public {
        vm.warp((uint256(today()) / 7 * 7 + 7) * 1 days + 1 hours);
        uint32 d = today();
        for (uint256 i = 0; i < 10; ++i) {
            address h = address(uint160(0x1000 + i));
            settle1(rootPk, rc(root, h, d, 10 << 20, 0));
            assertEq(L.balanceOf(h), value(10 << 20));
        }
        assertEq(L.issuedKib(root, d / 7), 100 << 20);
        settle1(rootPk, rc(root, alice, d, 1 << 20, 0));
        assertEq(L.balanceOf(alice), 0);
        (uint64 k,) = L.settled(root, alice, d);
        assertEq(k, 1 << 20); // lost
    }

    // ───────── payments ─────────

    function test_payment_fromReceivedFirst_shareAndBurn() public {
        fund(alice, 100);
        giveReceived(alice, 3 * UNIT);
        (uint256 e0, uint256 r0) = stored(alice);
        assertEq(e0, 100 * UNIT);
        assertEq(r0, 3 * UNIT);
        uint256 supply = L.totalSupply();
        vm.expectEmit(true, true, false, true);
        emit Settle(alice, bob, today(), 0, 10 * UNIT);
        pay(alicePk, bob, uint64(10 * UNIT));
        (uint256 e1, uint256 r1) = stored(alice);
        assertEq(r1, 0);
        assertEq(e1, 93 * UNIT);
        (, uint256 rb) = stored(bob);
        assertEq(rb, UNIT);
        assertEq(L.totalSupply(), supply - 9 * UNIT);
        assertFalse(L.isOverdrawn(alice));
        // took >= 1 credit: reward
    }

    function test_payment_short_flagsOverdrawn() public {
        fund(alice, 5);
        vm.expectEmit(true, false, false, true);
        emit Overdrawn(alice, 3 * UNIT);
        pay(alicePk, bob, uint64(8 * UNIT));
        assertTrue(L.isOverdrawn(alice));
        assertEq(L.balanceOf(alice), 0);
        (, uint64 sp) = L.settled(alice, bob, today());
        assertEq(sp, 8 * UNIT); // the rest is not owed any more
    }

    function test_payment_short_yesterday_flags() public {
        fund(alice, 5);
        vm.warp(block.timestamp + 1 days);
        payOn(alicePk, bob, today() - 1, uint64(8 * UNIT));
        assertTrue(L.isOverdrawn(alice));
    }

    function test_payment_short_twoDaysOld_noFlag() public {
        fund(alice, 5);
        vm.warp(block.timestamp + 2 days);
        payOn(alicePk, bob, today() - 2, uint64(8 * UNIT));
        assertFalse(L.isOverdrawn(alice));
        assertEq(L.balanceOf(alice), 0);
        assertEq(L.balanceOf(bob), UNIT / 2);
    }

    function test_overdrawn_blocksMintTransferHold() public {
        // make alice mature and overdrawn
        uint256 t0 = _matureAlice();
        vm.warp(t0 + 30 days);
        assertTrue(L.isMature(alice));
        uint256 bal = L.balanceOf(alice);
        // one receipt: spends more than it has and claims kib: flagged in step 1, no mint in step 2
        settle1(alicePk, rc(alice, carol, today(), 4096, uint64(bal + UNIT)));
        assertTrue(L.isOverdrawn(alice));
        assertEq(L.balanceOf(carol), bal / 10);
        (uint64 k,) = L.settled(alice, carol, today());
        assertEq(k, 0);
        settle1(alicePk, rc(alice, carol, today(), 8192, uint64(bal + UNIT)));
        assertEq(L.balanceOf(carol), bal / 10);
        fund(alice, 10);
        vm.prank(alice);
        vm.expectRevert(CreditLedger.IsOverdrawn.selector);
        L.transfer(bob, 1);
        vm.prank(alice);
        vm.expectRevert(CreditLedger.IsOverdrawn.selector);
        L.hold(bob, 1);
        // a release still works
        vm.prank(alice);
        L.hold(bob, 0);
    }

    function test_paymentAndMint_sameReceipt() public {
        fundKib(root2Pk, root, 100 * 1024);
        settle1(rootPk, rc(root, alice, today(), 2048, uint64(10 * UNIT)));
        assertEq(L.balanceOf(root), 90 * UNIT);
        assertEq(L.balanceOf(alice), 2 * UNIT + UNIT);
    }

    function test_payment_rewardOnlyWhenTakesOneCredit() public {
        fund(alice, 10);
        uint256 n = minter.count();
        pay(alicePk, bob, uint64(UNIT - 1));
        assertEq(minter.count(), n);
        pay(alicePk, bob, uint64(UNIT));
        assertEq(minter.count(), n + 2);
    }

    function test_reentrantMinter_noDoubleMint() public {
        ReentrantMinter rm = new ReentrantMinter();
        vm.etch(MINTER_ADDR, address(rm).code);
        rm = ReentrantMinter(MINTER_ADDR);
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](1);
        rs[0] = rc(root, alice, today(), 8192, 0);
        ss[0] = sign(rootPk, rs[0]);
        rm.arm(address(L), abi.encodeCall(L.settle, (rs, ss)));
        L.settle(rs, ss);
        assertTrue(rm.entered());
        assertTrue(rm.innerOk());
        assertEq(L.balanceOf(alice), 8 * UNIT);
        assertEq(L.totalSupply(), 8 * UNIT);
    }

    function test_reentrantMinter_payment() public {
        fund(alice, 100);
        ReentrantMinter rm = new ReentrantMinter();
        vm.etch(MINTER_ADDR, address(rm).code);
        rm = ReentrantMinter(MINTER_ADDR);
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](1);
        rs[0] = rc(alice, bob, today(), 0, uint64(10 * UNIT));
        ss[0] = sign(alicePk, rs[0]);
        rm.arm(address(L), abi.encodeCall(L.settle, (rs, ss)));
        L.settle(rs, ss);
        assertTrue(rm.entered());
        assertEq(L.balanceOf(alice), 90 * UNIT);
        assertEq(L.balanceOf(bob), UNIT);
    }
}
