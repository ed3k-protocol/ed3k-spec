// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../../../LICENSE.
pragma solidity ^0.8.24;

import {Base} from "./Base.sol";
import {CreditLedger} from "ledger/CreditLedger.sol";

contract DecayTest is Base {
    event Transfer(address indexed from, address indexed to, uint256 value);

    function test_grace_thenStep() public {
        fund(alice, 100);
        uint256 g = graceEnd();
        vm.warp(g + 1 days - 1);
        assertEq(L.balanceOf(alice), 100 * UNIT);
        vm.warp(g + 1 days);
        assertEq(L.balanceOf(alice), 90 * UNIT);
        vm.warp(g + 2 days - 1);
        assertEq(L.balanceOf(alice), 90 * UNIT);
        L.poke(alice); // partial day kept
        vm.warp(g + 2 days);
        assertEq(L.balanceOf(alice), 81 * UNIT);
    }

    function test_31pctAfterTwoWeeks() public {
        fund(alice, 100);
        vm.warp(uint256(today()) * 1 days + 14 days);
        uint256 e = 100 * UNIT;
        for (uint256 i; i < 11; ++i) {
            e = e * 9 / 10;
        }
        assertApproxEqAbs(L.balanceOf(alice), e, 11); // fixed: decay rounds up
        assertApproxEqRel(e, 31.38e6 * 100 / 100, 0.001e18);
    }

    function test_earnedFloor() public {
        fund(alice, 11);
        vm.warp(graceEnd() + 1 days);
        assertEq(L.balanceOf(alice), 9.9e6);
        vm.warp(graceEnd() + 30 days);
        assertEq(L.balanceOf(alice), 9.9e6);
        L.poke(alice);
        assertEq(L.balanceOf(alice), 9.9e6);
    }

    function test_earnedBelowFloor_neverDecays() public {
        fund(alice, 9);
        vm.warp(graceEnd() + 100 days);
        assertEq(L.balanceOf(alice), 9 * UNIT);
    }

    function test_received_cutBelowOneCredit() public {
        giveReceived(alice, 1.05e6); // alice never active: no grace
        assertEq(L.balanceOf(alice), 1.05e6);
        vm.warp(block.timestamp + 1 days);
        assertEq(L.balanceOf(alice), 0);
    }

    function test_received_keptWhileHolding() public {
        giveReceived(alice, 1.05e6);
        vm.prank(alice);
        L.hold(bob, 0.5e6);
        vm.warp(block.timestamp + 1 days);
        assertEq(L.balanceOf(alice), 945_000);
        vm.warp(block.timestamp + 1 days);
        assertEq(L.balanceOf(alice), 850_500);
        // bob takes his hold in full
        pay(alicePk, bob, 0.5e6);
        assertFalse(L.isOverdrawn(alice));
    }

    function test_activity_resetsGrace() public {
        fund(alice, 100);
        vm.warp(block.timestamp + 2 days);
        fund(alice, 1); // >= 1 credit: active today
        vm.warp(graceEnd() + 1 days - 1);
        assertEq(L.balanceOf(alice), 101 * UNIT);
    }

    function test_smallMint_notActivity() public {
        fund(alice, 100);
        vm.warp(block.timestamp + 2 days);
        fundKib(rootPk, alice, 1023); // < 1 credit
        assertEq(L.accountOf(alice).lastActive, today() - 2);
    }

    function test_poke_writesBackAndBurns() public {
        fund(alice, 100);
        vm.warp(graceEnd() + 1 days);
        uint256 supply = L.totalSupply();
        vm.expectEmit(true, true, false, true);
        emit Transfer(alice, address(0), 10 * UNIT);
        L.poke(alice);
        assertEq(L.totalSupply(), supply - 10 * UNIT);
        (uint256 e,) = stored(alice);
        assertEq(e, 90 * UNIT);
    }

    function test_longInactive_loopBounded() public {
        fundKib(rootPk, alice, 10 << 20);
        giveReceived(alice, 1000 * UNIT);
        vm.prank(alice);
        L.hold(bob, 1); // keep received from being cut: loop runs to 0
        vm.warp(block.timestamp + 3000 days);
        uint256 g = gasleft();
        L.poke(alice);
        g -= gasleft();
        emit log_named_uint("poke gas after 3000 idle days with a hold", g);
        assertLt(g, 300_000);
    }
}

contract HoldTest is Base {
    event Held(address indexed payer, address indexed payee, uint256 amount, uint64 until);

    function setUp() public override {
        super.setUp();
        fund(alice, 100);
    }

    function test_hold_basic() public {
        vm.expectEmit(true, true, false, true);
        emit Held(alice, bob, 50 * UNIT, uint64(block.timestamp + 6 hours));
        vm.prank(alice);
        L.hold(bob, 50 * UNIT);
        (uint256 a, uint64 u) = L.holdOf(alice, bob);
        assertEq(a, 50 * UNIT);
        assertEq(u, block.timestamp + 6 hours);
        assertEq(heldTotal(alice), 50 * UNIT);
    }

    function test_hold_liveOnlyRaiseOrExtend() public {
        vm.startPrank(alice);
        L.hold(bob, 50 * UNIT);
        vm.expectRevert(CreditLedger.HoldLocked.selector);
        L.hold(bob, 50 * UNIT - 1);
        vm.expectRevert(CreditLedger.HoldLocked.selector);
        L.hold(bob, 0);
        vm.warp(block.timestamp + 1 hours);
        L.hold(bob, 50 * UNIT); // renew
        (, uint64 u) = L.holdOf(alice, bob);
        assertEq(u, block.timestamp + 6 hours);
        L.hold(bob, 60 * UNIT);
        assertEq(heldTotal(alice), 60 * UNIT);
        vm.stopPrank();
    }

    function test_hold_afterUntil_lowerAndZero() public {
        vm.startPrank(alice);
        L.hold(bob, 50 * UNIT);
        vm.warp(block.timestamp + 6 hours);
        L.hold(bob, 10 * UNIT);
        assertEq(heldTotal(alice), 10 * UNIT);
        vm.warp(block.timestamp + 6 hours);
        L.hold(bob, 0);
        (uint256 a, uint64 u) = L.holdOf(alice, bob);
        assertEq(a, 0);
        assertEq(u, 0);
        assertEq(heldTotal(alice), 0);
        vm.stopPrank();
    }

    function test_hold_90pct() public {
        vm.startPrank(alice);
        L.hold(bob, 45 * UNIT);
        L.hold(carol, 45 * UNIT);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.hold(dave, 1);
        vm.stopPrank();
    }

    function test_hold_renewalChecked_afterDecay() public {
        vm.prank(alice);
        L.hold(bob, 90 * UNIT);
        vm.warp(block.timestamp + 5 hours);
        vm.prank(alice);
        L.hold(bob, 90 * UNIT); // no decay: fine
        vm.warp(graceEnd() + 1 days); // one step: balance 90
        vm.prank(alice);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.hold(bob, 90 * UNIT);
        vm.prank(alice);
        L.hold(bob, 81 * UNIT);
    }

    function test_hold_guards() public {
        vm.startPrank(alice);
        vm.expectRevert(CreditLedger.ZeroAddress.selector);
        L.hold(address(0), 1);
        vm.expectRevert(CreditLedger.AmountTooLarge.selector);
        L.hold(bob, uint256(type(uint128).max) + 1);
        vm.stopPrank();
    }

    function test_release_neverFails() public {
        vm.prank(alice);
        L.hold(bob, 50 * UNIT);
        vm.prank(bob);
        L.release(alice);
        assertEq(holdAmt(alice, bob), 0);
        assertEq(heldTotal(alice), 0);
        vm.prank(bob);
        L.release(alice); // again
        vm.prank(carol);
        L.release(address(0)); // nothing there
        // payer overdrawn: still releasable
        vm.prank(alice);
        L.hold(carol, 50 * UNIT);
        pay(alicePk, dave, uint64(200 * UNIT));
        assertTrue(L.isOverdrawn(alice));
        vm.prank(carol);
        L.release(alice);
        assertEq(heldTotal(alice), 0);
    }

    function test_sweep() public {
        vm.prank(alice);
        L.hold(bob, 50 * UNIT);
        vm.expectRevert(CreditLedger.HoldLocked.selector);
        L.sweep(alice, bob);
        vm.warp(block.timestamp + 6 hours);
        L.sweep(alice, bob);
        assertEq(heldTotal(alice), 0);
        L.sweep(alice, bob); // nothing left: no revert
    }

    function test_settlementUsesHold() public {
        vm.prank(alice);
        L.hold(bob, 50 * UNIT);
        pay(alicePk, bob, uint64(20 * UNIT));
        assertEq(holdAmt(alice, bob), 30 * UNIT);
        assertEq(heldTotal(alice), 30 * UNIT);
        pay(alicePk, bob, uint64(40 * UNIT)); // beyond the hold: hold to 0
        assertEq(holdAmt(alice, bob), 0);
        assertEq(heldTotal(alice), 0);
        assertFalse(L.isOverdrawn(alice));
    }

    // ───────── holdFor ─────────

    function test_holdFor() public {
        uint256 dl = block.timestamp + 10 minutes;
        bytes memory s = signHold(alicePk, alice, bob, 30 * UNIT, 1000, dl);
        vm.prank(carol);
        L.holdFor(alice, bob, 30 * UNIT, 1000, dl, s);
        assertEq(holdAmt(alice, bob), 30 * UNIT);
        assertEq(L.holdNonces(alice, bob), 1000);
        // replay
        vm.expectRevert(CreditLedger.BadNonce.selector);
        L.holdFor(alice, bob, 30 * UNIT, 1000, dl, s);
        // lower nonce
        bytes memory s2 = signHold(alicePk, alice, bob, 30 * UNIT, 999, dl);
        vm.expectRevert(CreditLedger.BadNonce.selector);
        L.holdFor(alice, bob, 30 * UNIT, 999, dl, s2);
        // other payee, lower nonce: own counter
        bytes memory s3 = signHold(alicePk, alice, carol, 10 * UNIT, 5, dl);
        L.holdFor(alice, carol, 10 * UNIT, 5, dl, s3);
        assertEq(heldTotal(alice), 40 * UNIT);
        // gaps allowed
        bytes memory s4 = signHold(alicePk, alice, bob, 31 * UNIT, 1_700_000_000_000, dl);
        L.holdFor(alice, bob, 31 * UNIT, 1_700_000_000_000, dl, s4);
        assertEq(holdAmt(alice, bob), 31 * UNIT);
    }

    function test_holdFor_rejects() public {
        uint256 dl = block.timestamp + 10 minutes;
        bytes memory s = signHold(alicePk, alice, bob, 30 * UNIT, 1, dl);
        // other amount than signed
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.holdFor(alice, bob, 31 * UNIT, 1, dl, s);
        // wrong signer
        bytes memory sb = signHold(bobPk, alice, bob, 30 * UNIT, 1, dl);
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.holdFor(alice, bob, 30 * UNIT, 1, dl, sb);
        // high s
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.holdFor(alice, bob, 30 * UNIT, 1, dl, _malleate(s));
        // zero payer with garbage
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.holdFor(address(0), bob, 30 * UNIT, 1, dl, new bytes(65));
        // deadline
        vm.warp(dl + 1);
        vm.expectRevert(CreditLedger.BadDeadline.selector);
        L.holdFor(alice, bob, 30 * UNIT, 1, dl, s);
    }

    function test_holdFor_otherChain() public {
        uint256 dl = block.timestamp + 10 minutes;
        bytes memory s = signHold(alicePk, alice, bob, 30 * UNIT, 1, dl);
        vm.chainId(block.chainid + 1);
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.holdFor(alice, bob, 30 * UNIT, 1, dl, s);
    }

    function test_receipt_otherChain_skipped() public {
        CreditLedger.Receipt memory r = rc(root, bob, today(), 4096, 0);
        bytes memory s = sign(rootPk, r);
        vm.chainId(block.chainid + 1);
        settleRaw(r, s);
        assertEq(L.balanceOf(bob), 0);
    }

    // ───────── _freeFor ─────────

    function test_freeFor_ownHoldAndBeyond() public {
        vm.startPrank(alice);
        L.hold(bob, 40 * UNIT);
        L.hold(carol, 40 * UNIT);
        vm.stopPrank();
        // bob may take 40 + (100 - 40 - ceil(40/0.9)) = 55.555555
        pay(alicePk, bob, uint64(50 * UNIT));
        assertFalse(L.isOverdrawn(alice));
        assertEq(holdAmt(alice, bob), 0);
        // carol's hold stays whole
        assertEq(L.balanceOf(alice), 50 * UNIT);
        // bob beyond: only 50 - 44.444445 = 5.555555
        pay(alicePk, bob, uint64(10 * UNIT));
        assertTrue(L.isOverdrawn(alice));
        assertEq(L.balanceOf(alice), 44_444_445);
        // carol still gets hers in full
        pay(alicePk, carol, uint64(40 * UNIT));
        assertEq(holdAmt(alice, carol), 0);
        assertEq(L.balanceOf(alice), 4_444_445);
    }

    function test_freeFor_ownHoldBeyondBalanceLessOthers() public {
        vm.startPrank(alice);
        L.hold(bob, 45 * UNIT);
        L.hold(carol, 45 * UNIT);
        vm.stopPrank();
        vm.warp(graceEnd() + 1 days); // balance 90 = sum of holds
        pay(alicePk, bob, uint64(46 * UNIT));
        assertTrue(L.isOverdrawn(alice));
        assertEq(L.balanceOf(alice), 45 * UNIT); // took 45: its hold, as far as 90-45 covers it
    }

    function test_freeFor_noHold() public {
        vm.prank(alice);
        L.hold(carol, 45 * UNIT);
        // dave, no hold: 100 - 50 = 50
        pay(alicePk, dave, uint64(50 * UNIT));
        assertFalse(L.isOverdrawn(alice));
        pay(alicePk, dave, 1);
        assertTrue(L.isOverdrawn(alice));
    }
}

contract TransferTest is Base {
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    function setUp() public override {
        super.setUp();
        fund(alice, 100);
    }

    function test_transfer_burnFromReceivedFirst() public {
        giveReceived(alice, 3 * UNIT);
        uint256 supply = L.totalSupply();
        vm.expectEmit(true, true, false, true);
        emit Transfer(alice, bob, 9 * UNIT);
        vm.expectEmit(true, true, false, true);
        emit Transfer(alice, address(0), UNIT);
        vm.prank(alice);
        assertTrue(L.transfer(bob, 10 * UNIT));
        (uint256 e, uint256 r) = stored(alice);
        assertEq(r, 0);
        assertEq(e, 93 * UNIT);
        (uint256 eb, uint256 rb) = stored(bob);
        assertEq(eb, 0);
        assertEq(rb, 9 * UNIT);
        assertEq(L.totalSupply(), supply - UNIT);
        CreditLedger.AccountView memory v = L.accountOf(bob);
        assertEq(v.lastActive, 0); // not activity
        assertEq(v.inflow, 0); // not maturity
        assertEq(v.registeredAt, 0);
    }

    function test_transfer_self() public {
        vm.prank(alice);
        L.transfer(alice, 10 * UNIT);
        assertEq(L.balanceOf(alice), 99 * UNIT);
    }

    function test_transfer_respectsHoldsWithMargin() public {
        vm.prank(alice);
        L.hold(bob, 45 * UNIT); // reserve 50
        vm.prank(alice);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.transfer(carol, 50 * UNIT + 1);
        vm.prank(alice);
        L.transfer(carol, 50 * UNIT);
    }

    function test_transfer_expiredHoldCountsUntilSwept() public {
        vm.prank(alice);
        L.hold(bob, 45 * UNIT);
        vm.warp(block.timestamp + 7 hours);
        vm.prank(alice);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.transfer(carol, 60 * UNIT);
        L.sweep(alice, bob);
        vm.prank(alice);
        L.transfer(carol, 60 * UNIT);
    }

    function test_transfer_guards() public {
        vm.prank(alice);
        vm.expectRevert(CreditLedger.ZeroAddress.selector);
        L.transfer(address(0), 1);
        vm.prank(alice);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.transfer(bob, 100 * UNIT + 1);
        pay(alicePk, dave, uint64(200 * UNIT));
        vm.prank(alice);
        vm.expectRevert(CreditLedger.IsOverdrawn.selector);
        L.transfer(bob, 0);
    }

    function test_transferFrom_finite() public {
        vm.prank(alice);
        L.approve(carol, 30 * UNIT);
        vm.prank(carol);
        L.transferFrom(alice, bob, 20 * UNIT);
        assertEq(L.allowance(alice, carol), 10 * UNIT);
        assertEq(L.balanceOf(bob), 18 * UNIT);
        vm.prank(carol);
        vm.expectRevert(CreditLedger.InsufficientAllowance.selector);
        L.transferFrom(alice, bob, 10 * UNIT + 1);
    }

    function test_transferFrom_unlimited() public {
        vm.prank(alice);
        L.approve(carol, type(uint256).max);
        vm.prank(carol);
        L.transferFrom(alice, bob, 20 * UNIT);
        assertEq(L.allowance(alice, carol), type(uint256).max);
    }

    function _permitSig(uint256 pk, address owner, address spender, uint256 v, uint256 nonce, uint256 dl)
        internal
        view
        returns (uint8, bytes32, bytes32)
    {
        bytes32 sh = keccak256(abi.encode(L.PERMIT_TYPEHASH(), owner, spender, v, nonce, dl));
        return vm.sign(pk, keccak256(abi.encodePacked("\x19\x01", L.DOMAIN_SEPARATOR(), sh)));
    }

    function test_permit() public {
        uint256 dl = block.timestamp + 1 hours;
        (uint8 v, bytes32 r, bytes32 s) = _permitSig(alicePk, alice, carol, 7 * UNIT, 0, dl);
        vm.expectEmit(true, true, false, true);
        emit Approval(alice, carol, 7 * UNIT);
        L.permit(alice, carol, 7 * UNIT, dl, v, r, s);
        assertEq(L.allowance(alice, carol), 7 * UNIT);
        assertEq(L.nonces(alice), 1);
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.permit(alice, carol, 7 * UNIT, dl, v, r, s); // replay
        (v, r, s) = _permitSig(bobPk, alice, carol, 7 * UNIT, 1, dl);
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.permit(alice, carol, 7 * UNIT, dl, v, r, s);
        (v, r, s) = _permitSig(alicePk, alice, carol, 7 * UNIT, 1, dl);
        bytes32 sHigh = bytes32(SECP_N - uint256(s));
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.permit(alice, carol, 7 * UNIT, dl, v == 27 ? 28 : 27, r, sHigh);
        vm.warp(dl + 1);
        vm.expectRevert(CreditLedger.BadDeadline.selector);
        L.permit(alice, carol, 7 * UNIT, dl, v, r, s);
        vm.expectRevert(CreditLedger.BadSignature.selector);
        L.permit(address(0), carol, 1, type(uint256).max, 27, bytes32(0), bytes32(0));
    }

    function test_metadata() public view {
        assertEq(L.decimals(), 6);
        assertEq(L.symbol(), "E3C");
        assertEq(L.name(), "ed3k credit");
        bytes32 ds = keccak256(
            abi.encode(
                keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                keccak256("ed3k credit"),
                keccak256("1"),
                block.chainid,
                address(L)
            )
        );
        assertEq(L.DOMAIN_SEPARATOR(), ds);
    }
}
