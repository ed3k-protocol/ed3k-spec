#!/usr/bin/env python3
"""Gas of CreditLedger calls, one transaction each, on anvil (hardfork cancun).

Run from the forge project root:  python3 -I gas/measure.py
Starts anvil itself on port 8547 (ANVIL_PORT), deploys MockOperators + CreditLedger (gasReward 1e15),
puts a no-op (STOP) at the native minter address (its gas cap is still checked), and prints receipt gasUsed (incl. 21,000
and calldata, after refunds).
"""
import json, os, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("FOUNDRY_BIN", "")  # directory of forge/cast/anvil; empty = from PATH
CAST, FORGE, ANVIL = (os.path.join(BIN, x) if BIN else x for x in ("cast", "forge", "anvil"))
PORT = os.environ.get("ANVIL_PORT", "8547")
RPC = f"http://127.0.0.1:{PORT}"
MN = "test test test test test test test test test test test junk"
MINTER = "0x0200000000000000000000000000000000000001"
RECEIPT_T = "(address,address,uint32,uint64,uint64)"
DAY = 86400


def sh(*args, check=True):
    r = subprocess.run(args, capture_output=True, text=True, cwd=ROOT)
    if check and r.returncode != 0:
        raise RuntimeError(f"{args}\n{r.stdout}\n{r.stderr}")
    return r.stdout.strip()


def cast(*a):
    return sh(CAST, *a)


def pk(i):
    return cast("wallet", "private-key", "--mnemonic", MN, "--mnemonic-index", str(i))


def addr(k):
    return cast("wallet", "address", "--private-key", k)


def rpc(method, *params):
    return cast("rpc", "--rpc-url", RPC, method, *[json.dumps(p) if not isinstance(p, str) else p for p in params])


T = [0]


def at(t):
    """Next block at timestamp t."""
    assert t >= T[0]
    T[0] = t
    rpc("evm_setNextBlockTimestamp", str(t))


def send(k, to, sig, *args):
    out = cast("send", "--rpc-url", RPC, "--private-key", k, "--json", to, sig, *args)
    j = json.loads(out)
    assert j["status"] == "0x1", out
    T[0] += 1  # anvil advances by 1 s per block when not set
    return int(j["gasUsed"], 16)


def call(to, sig, *args):
    return cast("call", "--rpc-url", RPC, to, sig, *args)


results = []


def rec(name, g, note=""):
    results.append((name, g, note))
    print(f"{name:70s} {g:>9,} {note}", flush=True)


def main():
    an = subprocess.Popen([ANVIL, "--port", PORT, "--hardfork", "cancun", "--accounts", "20",
                           "--gas-limit", "30000000", "--timestamp", "1700000000", "--silent"])
    try:
        for _ in range(50):
            try:
                cast("block-number", "--rpc-url", RPC)
                break
            except Exception:
                time.sleep(0.2)
        run()
    finally:
        an.terminate()
        an.wait()
    with open(os.path.join(ROOT, "gas", "results.json"), "w") as f:
        json.dump(results, f, indent=1)


def run():
    K = [pk(i) for i in range(20)]
    A = [addr(k) for k in K]
    dep = K[0]
    # start of a week (day ≡ 0 mod 7) + 1 h
    t0 = (1700000000 // DAY // 7 + 1) * 7 * DAY + 3600
    D = t0 // DAY
    assert D % 7 == 0
    at(t0 - 600)
    ops = json.loads(sh(FORGE, "create", "--rpc-url", RPC, "--private-key", dep, "--broadcast", "--json",
                        "test/Base.sol:MockOperators"))["deployedTo"]
    L = json.loads(sh(FORGE, "create", "--rpc-url", RPC, "--private-key", dep, "--broadcast", "--json",
                      "../../contracts/CreditLedger.sol:CreditLedger", "--constructor-args", ops, str(10**15)))["deployedTo"]
    rpc("anvil_setCode", MINTER, "0x00")  # no-op minter: measures the call overhead only
    ROOTK = K[19]
    ROOTA = A[19]
    send(dep, ops, "set(address,bool)", ROOTA, "true")
    DS = call(L, "DOMAIN_SEPARATOR()(bytes32)")
    HOLD_TH = call(L, "HOLD_TYPEHASH()(bytes32)")
    nonlocal_L = [L]
    caller = K[0]  # third party: reward goes 60/40 → two minter calls

    settled = {}

    def receipt(i, h, d, kib, spent):
        return f"({i},{h},{d},{kib},{spent})"

    def sig_receipt(k, r):
        dg = call(nonlocal_L[0], f"receiptDigest({RECEIPT_T})(bytes32)", r)
        return cast("wallet", "sign", "--no-hash", "--private-key", k, dg)

    def settle(items, sender=caller):
        """items: list of (issuerKey, holderAddr, day, dkib, dspent) — deltas on the stored counters."""
        rs, ss = [], []
        for (ik, h, d, dk, dsp) in items:
            i = addr(ik)
            k0, s0 = settled.get((i, h, d), (0, 0))
            k1, s1 = k0 + dk, s0 + dsp
            settled[(i, h, d)] = (k1, s1)
            r = receipt(i, h, d, k1, s1)
            rs.append(r)
            ss.append(sig_receipt(ik, r))
        return send(sender, nonlocal_L[0], f"settle({RECEIPT_T}[],bytes[])", "[" + ",".join(rs) + "]", "[" + ",".join(ss) + "]")

    def hold_sig(k, payer, payee, amount, nonce, dl):
        enc = cast("abi-encode", "f(bytes32,address,address,uint256,uint256,uint256)", HOLD_TH, payer, payee,
                   str(amount), str(nonce), str(dl))
        shh = cast("keccak", enc)
        dg = cast("keccak", "0x1901" + DS[2:] + shh[2:])
        return cast("wallet", "sign", "--no-hash", "--private-key", k, dg)

    U = 10**6
    CR = 1024  # KiB per credit
    t = t0
    # ───────── settle ─────────
    H1, H2, H3, H4, H5, H6 = A[1], A[2], A[3], A[4], A[5], A[6]
    at(t)
    rec("settle: first receipt of pair+day, 2 new accounts (also first pair+week, issuer+week)",
        settle([(ROOTK, H1, D, 4 * CR, 0)]))
    rec("settle: later receipt, same pair+day (mints >=1 credit; pair+day already rewarded)", settle([(ROOTK, H1, D, 4 * CR, 0)]))
    rec("settle: later receipt, same pair+day (again)", settle([(ROOTK, H1, D, 4 * CR, 0)]))
    g1 = settle([(ROOTK, H1, D, 4 * CR, 0)])
    g2 = settle([(ROOTK, H1, D, 4 * CR, 0)] * 2)
    g5 = settle([(ROOTK, H1, D, 4 * CR, 0)] * 5)
    rec("settle: batch of 1 later receipt", g1)
    rec("settle: batch of 2 later receipts (same pair+day)", g2)
    rec("settle: batch of 5 later receipts (same pair+day)", g5)
    rec("settle: per extra receipt in a batch, same pair+day ((g5-g1)/4)", (g5 - g1) // 4)
    # extra receipts of different pairs (pair-days known): H2, H3 first
    settle([(ROOTK, H2, D, 4 * CR, 0)])
    settle([(ROOTK, H3, D, 4 * CR, 0)])
    gd = settle([(ROOTK, H1, D, 4 * CR, 0), (ROOTK, H2, D, 4 * CR, 0), (ROOTK, H3, D, 4 * CR, 0)])
    rec("settle: batch of 3 later receipts, 3 different holders", gd)
    rec("settle: per extra receipt in a batch, other holder ((g3d-g1)/2)", (gd - g1) // 2)
    # first receipt of a pair+day, both accounts known, issuer week known: new holder H6 known through other issuer
    XK = K[18]
    settle([(XK, H6, D, 0, 0)])  # registers X and H6 (zero receipt)
    rec("settle: first receipt of a pair (+day, +pair-week), accounts registered, issuer-week known",
        settle([(ROOTK, H6, D, 4 * CR, 0)]))

    # payments: payer P funded by root
    P, PK = A[7], K[7]
    settle([(ROOTK, P, D, 500 * CR, 0)])
    rec("settle: first receipt of pair+day that takes (payer P, holder H4 new)", settle([(PK, H4, D, 0, 2 * U)]))
    rec("settle: later receipt that takes (no hold)", settle([(PK, H4, D, 0, 2 * U)]))
    rec("settle: later receipt that takes (no hold), again", settle([(PK, H4, D, 0, 2 * U)]))
    settle([(PK, H5, D, 0, 1 * U)])
    send(PK, L, "hold(address,uint256)", H5, str(20 * U))
    rec("settle: later receipt that takes and uses a hold up in part", settle([(PK, H5, D, 0, 3 * U)]))
    rec("settle: later receipt that takes and uses the rest of the hold (to 0)", settle([(PK, H5, D, 0, 17 * U)]))
    settle([(PK, H6, D, 0, 1 * U)])
    gs = settle([(PK, H4, D, 0, 2 * U), (PK, H4, D, 0, 2 * U)])
    gdh = settle([(PK, H4, D, 0, 2 * U), (PK, H6, D, 0, 2 * U)])
    rec("settle: 2 paying receipts, same holder", gs)
    rec("settle: 2 paying receipts, two holders", gdh)
    rec("settle: first receipt of a holder in a tx vs later (difference)", gdh - gs)

    # ───────── holds ─────────
    P2, P2K = A[8], K[8]
    H7, H7K = A[9], K[9]
    settle([(ROOTK, P2, D, 500 * CR, 0)])
    rec("hold: first for a payee", send(P2K, L, "hold(address,uint256)", H7, str(50 * U)))
    at(T[0] + 3600)
    rec("hold: renewal, same amount, no decay due", send(P2K, L, "hold(address,uint256)", H7, str(50 * U)))
    rec("hold: raise, no decay due", send(P2K, L, "hold(address,uint256)", H7, str(60 * U)))
    rec("hold: second payee, first", send(P2K, L, "hold(address,uint256)", A[10], str(10 * U)))
    rec("release (payee, live hold)", send(H7K, L, "release(address)", P2))
    rec("release (nothing held)", send(H7K, L, "release(address)", P2))

    # holdFor: payer P3 (no tx of its own), relay = caller
    P3, P3K = A[11], K[11]
    settle([(ROOTK, P3, D, 500 * CR, 0)])
    dl = T[0] + 600
    s = hold_sig(P3K, P3, H7, 40 * U, 1000, dl)
    rec("holdFor: first for (payer, payee)", send(caller, L, "holdFor(address,address,uint256,uint256,uint256,bytes)",
                                                  P3, H7, str(40 * U), "1000", str(dl), s))
    s = hold_sig(P3K, P3, H7, 40 * U, 1001, dl)
    rec("holdFor: later (renewal, same payer+payee)", send(caller, L,
        "holdFor(address,address,uint256,uint256,uint256,bytes)", P3, H7, str(40 * U), "1001", str(dl), s))
    s = hold_sig(P3K, P3, A[12], 10 * U, 5, dl)
    rec("holdFor: first for a second payee", send(caller, L,
        "holdFor(address,address,uint256,uint256,uint256,bytes)", P3, A[12], str(10 * U), "5", str(dl), s))

    # ───────── transfers (day D, no decay) ─────────
    T1, T1K = A[13], K[13]
    T2 = A[14]
    settle([(ROOTK, T1, D, 1000 * CR, 0)])
    settle([(ROOTK, T2, D, 10 * CR, 0)])
    rec("transfer: to an existing account", send(T1K, L, "transfer(address,uint256)", T2, str(10 * U)))
    rec("transfer: to an existing account (again)", send(T1K, L, "transfer(address,uint256)", T2, str(10 * U)))
    rec("transfer: to a new address", send(T1K, L, "transfer(address,uint256)",
                                           "0x000000000000000000000000000000000000bEEF", str(10 * U)))
    send(T1K, L, "approve(address,uint256)", A[0], str(100 * U))
    rec("transferFrom: finite allowance", send(caller, L, "transferFrom(address,address,uint256)", T1, T2, str(10 * U)))
    send(T1K, L, "approve(address,uint256)", A[0], str(2**256 - 1))
    rec("transferFrom: unlimited allowance", send(caller, L, "transferFrom(address,address,uint256)", T1, T2, str(10 * U)))
    rec("approve (new non-zero → non-zero)", send(T1K, L, "approve(address,uint256)", A[1], str(5 * U)))

    # sweep: P2's hold for A[10] runs out
    at(T[0] + 6 * 3600 + 60)
    rec("sweep (hold ran out)", send(caller, L, "sweep(address,address)", P2, A[10]))

    # decay set-up for later: T3..T6 funded today, untouched until D+8
    T3, T3K, T4, T5, T5K = A[15], K[15], A[16], A[17], K[17]
    settle([(ROOTK, T3, D, 100 * CR, 0)])
    settle([(ROOTK, T4, D, 100 * CR, 0)])
    settle([(ROOTK, T5, D, 100 * CR, 0)])
    # P4: hold placed late on D+7, renewed after midnight
    P4, P4K = A[10], K[10]
    settle([(ROOTK, P4, D, 100 * CR, 0)])

    # ───────── D+1: first receipt of a pair+day in a known pair-week ─────────
    at((D + 1) * DAY + 3600)
    g1f = settle([(ROOTK, H1, D + 1, 4 * CR, 0)])
    rec("settle: first receipt of a pair+day, pair-week known (new day)", g1f)
    rec("settle: later receipt same pair+day (D+1)", settle([(ROOTK, H1, D + 1, 4 * CR, 0)]))
    g3f = settle([(ROOTK, H2, D + 1, 4 * CR, 0), (ROOTK, H3, D + 1, 4 * CR, 0), (ROOTK, H6, D + 1, 4 * CR, 0)])
    rec("settle: batch of 3 first receipts of pair+day, 3 holders (each earns a reward)", g3f)
    rec("settle: per extra first receipt of pair+day in a batch, other holder ((g3f-g1f)/2)", (g3f - g1f) // 2)
    # ───────── D+6: keep H1, H3 active ─────────
    at((D + 6) * DAY + 3600)
    settle([(ROOTK, H1, D + 6, 4 * CR, 0)])
    settle([(ROOTK, H3, D + 6, 4 * CR, 0)])
    # ───────── D+7: new week ─────────
    at((D + 7) * DAY + 3600)
    rec("settle: first receipt of pair+week AND issuer+week (accounts known)", settle([(ROOTK, H1, D + 7, 4 * CR, 0)]))
    rec("settle: first receipt of a pair+week (issuer-week known, issuer counted)",
        settle([(ROOTK, H3, D + 7, 4 * CR, 0)]))
    rec("settle: later receipt same pair+day (D+7)", settle([(ROOTK, H3, D + 7, 4 * CR, 0)]))
    # hold renewal across a step: P4 last active D → steps due; place at 23:00, renew at 01:00
    at((D + 7) * DAY + 23 * 3600)
    rec("hold: first for a payee, writes back decay of several steps", send(P4K, L, "hold(address,uint256)", A[12], str(10 * U)))
    at((D + 8) * DAY + 3600)
    rec("hold: renewal that writes back one step of decay", send(P4K, L, "hold(address,uint256)", A[12], str(10 * U)))
    rec("hold: renewal right after (no decay due)", send(P4K, L, "hold(address,uint256)", A[12], str(10 * U)))
    # transfers with decay write-back
    rec("transfer: decay write-back on both sides (T3 → T4, both idle 8 days)",
        send(T3K, L, "transfer(address,uint256)", T4, str(5 * U)))
    rec("transfer: same pair right after (no decay due)", send(T3K, L, "transfer(address,uint256)", T4, str(5 * U)))
    rec("transfer: decay write-back on sender only (T5 → T4)", send(T5K, L, "transfer(address,uint256)", T4, str(5 * U)))
    rec("transfer: no decay due, sender T5 → T4 again", send(T5K, L, "transfer(address,uint256)", T4, str(5 * U)))
    rec("poke (no decay due)", send(caller, L, "poke(address)", T4))
    rec("poke (decay due, H2 idle since D+1)", send(caller, L, "poke(address)", H2))

    # ───────── reward variants ─────────
    D8 = D + 8
    H9, H9K = A[3], K[3]
    rec("settle: first receipt of pair+day, pair-week known, holder is the caller (one minter call)",
        settle([(ROOTK, H9, D8, 4 * CR, 0)], sender=H9K))
    rec("settle: first receipt of pair+day, pair-week known, mints < 1 credit (no reward call)",
        settle([(ROOTK, H1, D8, 512, 0)]))
    L0 = json.loads(sh(FORGE, "create", "--rpc-url", RPC, "--private-key", dep, "--broadcast", "--json",
                       "../../contracts/CreditLedger.sol:CreditLedger", "--constructor-args", ops, "0"))["deployedTo"]
    nonlocal_L[0] = L0
    settled.clear()
    rec("gasReward=0: first receipt of pair+day, 2 new accounts", settle([(ROOTK, H1, D8, 4 * CR, 0)]))
    rec("gasReward=0: later receipt same pair+day", settle([(ROOTK, H1, D8, 4 * CR, 0)]))
    rec("gasReward=0: later receipt same pair+day (again)", settle([(ROOTK, H1, D8, 4 * CR, 0)]))


if __name__ == "__main__":
    main()
