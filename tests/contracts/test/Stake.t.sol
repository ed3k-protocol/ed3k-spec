// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../../../LICENSE.
pragma solidity ^0.8.24;

import {Base} from "./Base.sol";
import {CreditLedger} from "ledger/CreditLedger.sol";
import {StakeToken} from "ledger/StakeToken.sol";

/// Burning credits for the stake token (credit.md §6.7).
contract StakeTest is Base {
    StakeToken T;

    function setUp() public override {
        super.setUp();
        T = L.stakeToken();
    }

    function _claim(uint256 id) internal view returns (address to, uint64 readyAt, uint256 amount) {
        (to, readyAt, amount) = L.stakeClaims(id);
    }

    // ───────── deployment ─────────

    function test_deployment() public view {
        assertEq(T.minter(), address(L));
        assertEq(L.stakeRate(), STAKE_RATE);
        assertEq(L.stakeDelay(), STAKE_DELAY);
        assertEq(L.nextClaimId(), 1);
        assertEq(T.decimals(), 18);
        assertEq(T.totalSupply(), 0);
    }

    function test_constructorRefusesBadParams() public {
        vm.expectRevert(CreditLedger.BadParam.selector);
        new CreditLedger(ops, REWARD, STAKE_RATE, 365 days + 1);
        vm.expectRevert(CreditLedger.BadParam.selector);
        new CreditLedger(ops, REWARD, uint256(type(uint128).max) + 1, STAKE_DELAY);
    }

    // ───────── burn ─────────

    function test_burnTakesCreditsAndRecordsClaim() public {
        fund(alice, 100);
        uint256 supply = L.totalSupply();
        vm.prank(alice);
        uint256 id = L.burnForStake(40 * UNIT, bob);
        assertEq(id, 1);
        assertEq(L.nextClaimId(), 2);
        assertEq(L.balanceOf(alice), 60 * UNIT);
        assertEq(L.totalSupply(), supply - 40 * UNIT);
        (address to, uint64 readyAt, uint256 amount) = _claim(id);
        assertEq(to, bob);
        assertEq(readyAt, block.timestamp + STAKE_DELAY);
        assertEq(amount, 40e18);
        assertEq(T.totalSupply(), 0); // nothing before the claim
    }

    function test_burnTakesReceivedFirst() public {
        fund(alice, 100);
        giveReceived(alice, 5 * UNIT);
        vm.prank(alice);
        L.burnForStake(7 * UNIT, alice);
        (uint256 e, uint256 r) = stored(alice);
        assertEq(r, 0);
        assertEq(e, 98 * UNIT);
    }

    function test_burnAppliesDecayFirst() public {
        fund(alice, 100);
        vm.warp(graceEnd() + 1 days); // one step: 90 left
        vm.prank(alice);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.burnForStake(91 * UNIT, alice);
        vm.prank(alice);
        L.burnForStake(90 * UNIT, alice);
        assertEq(L.balanceOf(alice), 0);
    }

    function test_burnIsNotActivity() public {
        fund(alice, 100);
        CreditLedger.AccountView memory before = L.accountOf(alice);
        vm.warp(block.timestamp + 2 days);
        vm.prank(alice);
        L.burnForStake(10 * UNIT, alice);
        CreditLedger.AccountView memory v = L.accountOf(alice);
        assertEq(v.lastActive, before.lastActive);
        assertEq(v.inflow, before.inflow);
        assertEq(v.issuers, before.issuers);
    }

    function test_burnNeverDipsIntoHolds() public {
        fund(alice, 100);
        vm.prank(alice);
        L.hold(bob, 45 * UNIT); // reserve 50
        vm.prank(alice);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.burnForStake(50 * UNIT + 1, alice);
        vm.prank(alice);
        L.burnForStake(50 * UNIT, alice);
        assertEq(L.balanceOf(alice), 50 * UNIT);
    }

    function test_burnRefusedWhenOverdrawn() public {
        fund(alice, 10);
        pay(alicePk, bob, uint64(20 * UNIT));
        assertTrue(L.isOverdrawn(alice));
        vm.prank(alice);
        vm.expectRevert(CreditLedger.IsOverdrawn.selector);
        L.burnForStake(1, alice);
    }

    function test_burnRefusals() public {
        fund(alice, 10);
        vm.startPrank(alice);
        vm.expectRevert(CreditLedger.ZeroAddress.selector);
        L.burnForStake(UNIT, address(0));
        vm.expectRevert(CreditLedger.ZeroAmount.selector);
        L.burnForStake(0, alice);
        vm.expectRevert(CreditLedger.InsufficientBalance.selector);
        L.burnForStake(11 * UNIT, alice);
        vm.stopPrank();

        ops.setStake(L, 0, STAKE_DELAY);
        vm.prank(alice);
        vm.expectRevert(CreditLedger.StakeOff.selector);
        L.burnForStake(UNIT, alice);
    }

    function test_burnRoundsDownToZeroIsRefused() public {
        fund(alice, 10);
        ops.setStake(L, 1, STAKE_DELAY); // 1 base unit per credit
        vm.prank(alice);
        vm.expectRevert(CreditLedger.ZeroAmount.selector);
        L.burnForStake(UNIT - 1, alice);
        vm.prank(alice);
        uint256 id = L.burnForStake(UNIT, alice);
        (,, uint256 amount) = _claim(id);
        assertEq(amount, 1);
    }

    // ───────── parameters ─────────

    function test_paramsFixedAtBurn() public {
        fund(alice, 100);
        vm.prank(alice);
        uint256 id = L.burnForStake(10 * UNIT, alice);
        uint256 t = block.timestamp;
        ops.setStake(L, 5e17, 1 days);
        vm.prank(alice);
        uint256 id2 = L.burnForStake(10 * UNIT, alice);

        (, uint64 r1, uint256 a1) = _claim(id);
        (, uint64 r2, uint256 a2) = _claim(id2);
        assertEq(a1, 10e18);
        assertEq(r1, t + 30 days);
        assertEq(a2, 5e18);
        assertEq(r2, t + 1 days);
    }

    function test_onlyOperatorsSetParams() public {
        vm.expectRevert(CreditLedger.NotOperators.selector);
        L.setStakeParams(1, 1);
        vm.prank(root);
        vm.expectRevert(CreditLedger.NotOperators.selector);
        L.setStakeParams(1, 1);

        vm.expectRevert(CreditLedger.BadParam.selector);
        ops.setStake(L, 1, 365 days + 1);
        vm.expectRevert(CreditLedger.BadParam.selector);
        ops.setStake(L, uint256(type(uint128).max) + 1, 1);

        ops.setStake(L, 2e18, 0);
        assertEq(L.stakeRate(), 2e18);
        assertEq(L.stakeDelay(), 0);
    }

    function test_zeroDelayClaimsAtOnce() public {
        fund(alice, 10);
        ops.setStake(L, STAKE_RATE, 0);
        vm.prank(alice);
        uint256 id = L.burnForStake(UNIT, alice);
        L.claimStake(id);
        assertEq(T.balanceOf(alice), 1e18);
    }

    // ───────── claim ─────────

    function test_claimAfterDelay() public {
        fund(alice, 100);
        vm.prank(alice);
        uint256 id = L.burnForStake(25 * UNIT, bob);

        vm.warp(block.timestamp + STAKE_DELAY - 1);
        vm.expectRevert(CreditLedger.NotReady.selector);
        L.claimStake(id);

        vm.warp(block.timestamp + 1);
        vm.prank(carol); // anyone may claim; tokens go to `to`
        L.claimStake(id);
        assertEq(T.balanceOf(bob), 25e18);
        assertEq(T.balanceOf(carol), 0);
        assertEq(T.totalSupply(), 25e18);

        vm.expectRevert(CreditLedger.NoClaim.selector);
        L.claimStake(id);
        (address to,,) = _claim(id);
        assertEq(to, address(0));
    }

    function test_claimUnknownId() public {
        vm.expectRevert(CreditLedger.NoClaim.selector);
        L.claimStake(0);
        vm.expectRevert(CreditLedger.NoClaim.selector);
        L.claimStake(7);
    }

    // ───────── token ─────────

    function test_tokenOnlyLedgerMints() public {
        vm.expectRevert(StakeToken.NotMinter.selector);
        T.mint(alice, 1);
        vm.prank(address(ops));
        vm.expectRevert(StakeToken.NotMinter.selector);
        T.mint(alice, 1);
    }

    function _minted(address to, uint256 credits) internal {
        fund(to, credits);
        vm.prank(to);
        uint256 id = L.burnForStake(credits * UNIT, to);
        vm.warp(block.timestamp + STAKE_DELAY);
        L.claimStake(id);
    }

    function test_tokenTransferNoDecayNoBurn() public {
        _minted(alice, 10);
        vm.prank(alice);
        T.transfer(bob, 4e18);
        assertEq(T.balanceOf(alice), 6e18);
        assertEq(T.balanceOf(bob), 4e18);
        vm.warp(block.timestamp + 400 days);
        assertEq(T.balanceOf(alice), 6e18);
        assertEq(T.totalSupply(), 10e18);

        vm.prank(alice);
        vm.expectRevert(StakeToken.InsufficientBalance.selector);
        T.transfer(bob, 6e18 + 1);
        vm.prank(alice);
        vm.expectRevert(StakeToken.ZeroAddress.selector);
        T.transfer(address(0), 1);
    }

    function test_tokenApproveTransferFrom() public {
        _minted(alice, 10);
        vm.prank(alice);
        T.approve(carol, 3e18);
        vm.prank(carol);
        T.transferFrom(alice, dave, 2e18);
        assertEq(T.allowance(alice, carol), 1e18);
        vm.prank(carol);
        vm.expectRevert(StakeToken.InsufficientAllowance.selector);
        T.transferFrom(alice, dave, 2e18);

        vm.prank(alice);
        T.approve(carol, type(uint256).max);
        vm.prank(carol);
        T.transferFrom(alice, dave, 1e18);
        assertEq(T.allowance(alice, carol), type(uint256).max);
        assertEq(T.balanceOf(dave), 3e18);
    }

    function test_tokenPermit() public {
        uint256 dl = block.timestamp + 1 hours;
        bytes32 sh = keccak256(abi.encode(T.PERMIT_TYPEHASH(), alice, carol, 5e18, 0, dl));
        (uint8 v, bytes32 r, bytes32 s) =
            vm.sign(alicePk, keccak256(abi.encodePacked("\x19\x01", T.DOMAIN_SEPARATOR(), sh)));
        T.permit(alice, carol, 5e18, dl, v, r, s);
        assertEq(T.allowance(alice, carol), 5e18);
        assertEq(T.nonces(alice), 1);
        vm.expectRevert(StakeToken.BadSignature.selector); // nonce used
        T.permit(alice, carol, 5e18, dl, v, r, s);

        vm.warp(dl + 1);
        vm.expectRevert(StakeToken.BadDeadline.selector);
        T.permit(alice, carol, 5e18, dl, v, r, s);
    }

    function test_tokenDomainDiffersFromLedger() public view {
        assertTrue(T.DOMAIN_SEPARATOR() != L.DOMAIN_SEPARATOR());
    }

    // ───────── fuzz ─────────

    function testFuzz_burnAmount(uint256 credits, uint256 rate, uint256 value) public {
        credits = bound(credits, 1, 1000);
        rate = bound(rate, 1, type(uint128).max);
        fund(alice, credits);
        ops.setStake(L, rate, STAKE_DELAY);
        value = bound(value, 1, credits * UNIT);
        uint256 amount = value * rate / UNIT;
        vm.prank(alice);
        if (amount == 0) {
            vm.expectRevert(CreditLedger.ZeroAmount.selector);
            L.burnForStake(value, alice);
            return;
        }
        uint256 id = L.burnForStake(value, alice);
        (,, uint256 a) = _claim(id);
        assertEq(a, amount);
        assertEq(L.balanceOf(alice), credits * UNIT - value);
    }
}
