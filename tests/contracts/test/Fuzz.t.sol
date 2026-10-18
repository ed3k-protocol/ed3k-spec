// SPDX-License-Identifier: LicenseRef-ed3k-public-domain
// Placed in the public domain, see ../../../LICENSE.
pragma solidity ^0.8.24;

import {Base, MockMinter} from "./Base.sol";
import {CreditLedger, IOperators} from "ledger/CreditLedger.sol";
import {Test} from "forge-std/Test.sol";

/// Shared scenario: a hold placed under the 90 % rule, one possible decay step,
/// another payee's payment, then the payee takes its hold.
abstract contract HoldPayableScenario is Base {
    /// Returns (hold, taken).
    function _scenario(
        uint64 eKib,
        uint256 rMicro,
        uint256 dayOffset,
        uint256 tod,
        uint16 fracQ,
        uint16 fracR,
        uint256 otherPay,
        uint256 dt
    ) internal returns (uint256 q, uint256 taken) {
        eKib = uint64(bound(eKib, 1024, 10 << 20));
        rMicro = bound(rMicro, 0, 1000 * UNIT);
        fundKib(rootPk, alice, eKib);
        if (rMicro != 0) giveReceived(alice, rMicro);
        // place the holds some time later, maybe after decay steps
        dayOffset = bound(dayOffset, 0, 20);
        tod = bound(tod, 0, 1 days - 1);
        vm.warp(uint256(today()) * 1 days + dayOffset * 1 days + tod);
        uint256 b = L.balanceOf(alice);
        uint256 cap = b * 90 / 100;
        q = cap * bound(fracQ, 1, 100) / 100;
        uint256 r = (cap - q) * bound(fracR, 0, 100) / 100;
        if (q == 0) return (0, 0);
        vm.prank(alice);
        L.hold(bob, q);
        if (r != 0) {
            vm.prank(alice);
            L.hold(carol, r);
        }
        vm.warp(block.timestamp + bound(dt, 0, 6 hours - 1));
        otherPay = bound(otherPay, 0, 2 * r + 1);
        if (otherPay != 0) pay(alicePk, carol, uint64(otherPay));
        uint256 before = L.balanceOf(alice);
        pay(alicePk, bob, uint64(q));
        taken = before - L.balanceOf(alice);
    }
}

contract HoldPayableFuzz is HoldPayableScenario {
    /// Holds stay payable, within 2 µ of rounding (see FINDINGS F2 for the exact property).
    /// forge-config: default.fuzz.runs = 20000
    function testFuzz_holdPayable_within2micro(
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
        assertLe(taken, q);
        assertGe(taken + 2, q);
    }

    function testFuzz_releaseNeverReverts(uint256 amt, uint256 dt, bool asPayer) public {
        fund(alice, 100);
        amt = bound(amt, 0, 90 * UNIT);
        vm.prank(alice);
        L.hold(bob, amt);
        vm.warp(block.timestamp + bound(dt, 0, 30 days));
        if (asPayer && block.timestamp >= T0 + 6 hours) {
            vm.prank(alice);
            L.hold(bob, 0);
        } else {
            vm.prank(bob);
            L.release(alice);
        }
        assertEq(holdAmt(alice, bob), 0);
        assertEq(heldTotal(alice), 0);
    }

    function testFuzz_valueCumulative(uint64 a, uint64 b) public {
        a = uint64(bound(a, 0, 5 << 20));
        b = uint64(bound(b, a, 10 << 20));
        settle1(rootPk, rc(root, alice, today(), a, 0));
        settle1(rootPk, rc(root, alice, today(), b, 0));
        assertEq(L.balanceOf(alice), value(b));
    }

    function testFuzz_transferNeverDipsIntoReserve(uint256 h, uint256 v) public {
        fund(alice, 100);
        h = bound(h, 0, 90 * UNIT);
        vm.prank(alice);
        L.hold(bob, h);
        v = bound(v, 0, 100 * UNIT);
        uint256 reserve = (h * 100 + 89) / 90;
        vm.prank(alice);
        if (v + reserve > 100 * UNIT) {
            vm.expectRevert(CreditLedger.InsufficientBalance.selector);
            L.transfer(carol, v);
        } else {
            L.transfer(carol, v);
            assertGe(L.balanceOf(alice), reserve);
        }
    }
}

/// Random actions over a few actors.
contract Handler is Test {
    CreditLedger public L;
    uint256[] public pks;
    address[] public actors;
    uint256[3] rootPks;
    bool public releaseFailed;
    uint256 public calls;
    uint256 public holdsOk;
    uint256 public transfersOk;

    constructor(CreditLedger l, uint256[] memory pks_, uint256[3] memory roots_) {
        L = l;
        pks = pks_;
        for (uint256 i; i < pks_.length; ++i) {
            actors.push(vm.addr(pks_[i]));
        }
        rootPks = roots_;
    }

    function actorCount() external view returns (uint256) {
        return actors.length;
    }

    function _today() internal view returns (uint32) {
        return uint32(block.timestamp / 1 days);
    }

    function _settle(uint256 pk, CreditLedger.Receipt memory r) internal {
        (uint8 v, bytes32 rr, bytes32 s) = vm.sign(pk, L.receiptDigest(r));
        CreditLedger.Receipt[] memory rs = new CreditLedger.Receipt[](1);
        bytes[] memory ss = new bytes[](1);
        rs[0] = r;
        ss[0] = abi.encodePacked(rr, s, v);
        L.settle(rs, ss);
    }

    function fund(uint256 ri, uint256 ai, uint256 kib) external {
        ++calls;
        uint256 pk = rootPks[ri % 3];
        address i = vm.addr(pk);
        address h = actors[ai % actors.length];
        (uint64 k, uint64 sp) = L.settled(i, h, _today());
        _settle(pk, CreditLedger.Receipt(i, h, _today(), k + uint64(bound(kib, 0, 300_000)), sp));
    }

    function pay(uint256 pi, uint256 qi, uint256 amount, bool yesterday) external {
        ++calls;
        uint256 pk = pks[pi % pks.length];
        address p = actors[pi % actors.length];
        address q = actors[qi % actors.length];
        if (p == q) return;
        uint32 d = yesterday ? _today() - 1 : _today();
        (uint64 k, uint64 sp) = L.settled(p, q, d);
        _settle(pk, CreditLedger.Receipt(p, q, d, k, sp + uint64(bound(amount, 0, 80e6))));
    }

    function hold(uint256 pi, uint256 qi, uint256 amount) external {
        ++calls;
        vm.prank(actors[pi % actors.length]);
        try L.hold(actors[qi % actors.length], bound(amount, 0, 200e6)) {
            ++holdsOk;
        } catch {}
    }

    function holdFor(uint256 pi, uint256 qi, uint256 amount) external {
        ++calls;
        address p = actors[pi % actors.length];
        address q = actors[qi % actors.length];
        amount = bound(amount, 0, 200e6);
        uint256 nonce = L.holdNonces(p, q) + 1;
        uint256 dl = block.timestamp + 600;
        bytes32 sh = keccak256(abi.encode(L.HOLD_TYPEHASH(), p, q, amount, nonce, dl));
        (uint8 v, bytes32 r, bytes32 s) =
            vm.sign(pks[pi % pks.length], keccak256(abi.encodePacked("\x19\x01", L.DOMAIN_SEPARATOR(), sh)));
        try L.holdFor(p, q, amount, nonce, dl, abi.encodePacked(r, s, v)) {
            ++holdsOk;
        } catch {}
    }

    function release(uint256 qi, uint256 pi) external {
        ++calls;
        vm.prank(actors[qi % actors.length]);
        try L.release(actors[pi % actors.length]) {}
        catch {
            releaseFailed = true;
        }
    }

    function sweep(uint256 pi, uint256 qi) external {
        ++calls;
        try L.sweep(actors[pi % actors.length], actors[qi % actors.length]) {} catch {}
    }

    function transfer(uint256 pi, uint256 qi, uint256 v) external {
        ++calls;
        vm.prank(actors[pi % actors.length]);
        try L.transfer(actors[qi % actors.length], bound(v, 0, 100e6)) {
            ++transfersOk;
        } catch {}
    }

    function warp(uint256 dt) external {
        ++calls;
        vm.warp(block.timestamp + bound(dt, 0, 9 hours));
    }

    function poke(uint256 ai) external {
        ++calls;
        L.poke(actors[ai % actors.length]);
    }
}

contract LedgerInvariants is Base {
    Handler h;

    function setUp() public override {
        super.setUp();
        uint256[] memory pks = new uint256[](4);
        pks[0] = alicePk;
        pks[1] = bobPk;
        pks[2] = carolPk;
        pks[3] = davePk;
        h = new Handler(L, pks, [rootPk, root2Pk, root3Pk]);
        targetContract(address(h));
    }

    function invariant_storedSumEqualsSupply() public view {
        uint256 sum;
        uint256 dec;
        for (uint256 i; i < h.actorCount(); ++i) {
            (uint256 e, uint256 r) = stored(h.actors(i));
            sum += e + r;
            dec += L.balanceOf(h.actors(i));
        }
        assertEq(sum, L.totalSupply());
        assertLe(dec, L.totalSupply());
    }

    function invariant_heldTotalIsSumOfHolds() public view {
        for (uint256 i; i < h.actorCount(); ++i) {
            address p = h.actors(i);
            uint256 s;
            for (uint256 j; j < h.actorCount(); ++j) {
                s += holdAmt(p, h.actors(j));
            }
            assertEq(heldTotal(p), s);
        }
    }

    function afterInvariant() public {
        emit log_named_uint("holds ok", h.holdsOk());
        emit log_named_uint("transfers ok", h.transfersOk());
        emit log_named_uint("supply", L.totalSupply());
    }

    function invariant_releaseNeverFailed() public view {
        assertFalse(h.releaseFailed());
    }
}
