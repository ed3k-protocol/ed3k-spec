# Extension: credits (`credit`)

**Status:** draft 0.2 (2026-10-12). Not implemented. Optional in wire version 1. **Ledger contract ([`contracts/CreditLedger.sol`](../contracts/CreditLedger.sol)) compiles with solc 0.8.37 and passes unit, fuzz and invariant tests ([`tests/contracts`](../tests/contracts/README.md), 2026-10-18). Stake token of §6.7 ([`contracts/StakeToken.sol`](../contracts/StakeToken.sol)) and the ledger's burn pass their own unit, fuzz and invariant tests (2026-10-27). Neither audited; numbers not simulated against an attack model; no chain deployed.**
**Extends:** [`Protocol.md`](../Protocol.md) §2, §5.1, §12.1 and §12.5. § = this document unless a document is named.
**Capability:** bit 16, `CREDIT`. Message types 35 to 37. One field in `FileRequest`, one in `FileStatus`.
**Needs:** JSON-RPC access to a credit-ledger node (§7). The ledger is its own chain, outside ed3k.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Credit signatures*.

In ed3k credits are local policy (`Protocol.md` §12.1): an uploader remembers who uploaded to it,
and nobody else can use that. Here credits count at **every** uploader.

- Downloader signs a **receipt** for what it received; uploader hands receipts to a **ledger**,
  which turns them into credits: 1 credit per 1 MiB.
- Downloader can **spend** credits for an upload slot ahead of the queue, naming a price per
  MiB; highest bid gets the slot, pays just above the next bid.
- Ledger = contract on an EVM chain, run as an **Avalanche L1** by a set of operators. Holds
  balances, applies rules, so no client can skip them.
- Nothing here sells/buys credits. Transferable; lose 10 % a day when holder stops uploading,
  and 10 % on every transfer.
- Credits can be **burned for a stake token** (§6.7), which does not decay and trades on
  exchanges, so **credits have a price** (§10).

Why a chain, why this one, who must run a node, what was rejected:
[`docs/credit-ledger.md`](../docs/credit-ledger.md).

**A node without this extension is still served,** from the normal queue. But up to two thirds
of an uploader's slots can go to bidders (§5.1) and a bid can end its turn early (§5.3), so it
waits longer at uploaders selling priority.

## 1. Overview

```
 Downloader D                          Uploader U                      Ledger (own chain)
 ────────────                          ──────────                      ──────────────────
 FileRequest{credit: payer, bid} ───▶
            ◀─── FileStatus{rank, credit: payee, challenge, floor, clearing}
 Receipt{counters as they stand,
         proof} ────────────────────▶  verify `proof`: `payer` is D's
 hold(payee, 16 × bid) ──────────────────────────────────────────────▶ locked for 6 h
                                       bid wins a priority slot:
                                       eth_call holdOf(payer, payee) ▶
            ◀─── FileStatus{rank = 0, credit: price, due}
 Receipt{kib, spent = due} ─────────▶  1 tranche paid ahead
            ◀═══ 4 MiB of blocks ═══
            ◀─── FileStatus{credit: due}
 Receipt{kib = 4 MiB, spent = due} ─▶
            ◀═══ 4 MiB of blocks ═══
            …                          before the hold runs out:
                                       settle(receipts) ─────────────▶ burn `spent` at D
                                                                        mint `kib` to U
```

Without a bid: same minus the hold, `spent` = 0; downloader signs what it received, uploader
earns credits, settling within 48 h.

| Term | Meaning |
|---|---|
| **credit address** | 20 bytes, address of a secp256k1 key as in Ethereum; one per node, apart from its node key (§2) |
| **issuer** | downloader signing a receipt |
| **holder** | uploader a receipt is signed for |
| **µ** | 10⁻⁶ credit, unit of every amount on wire and ledger |
| **tranche** | each 4 MiB of verified file data of a turn, counted from turn start (piece size, `Protocol.md` §7.2, not aligned to one); a turn's last tranche can be shorter |
| **day** | `floor(unix_time / 86400)` |
| **mature** | account whose receipts mint credits (§6.3) |

## 2. The credit key

Node key (`Protocol.md` §6.1) is Ed25519; the EVM verifies secp256k1 signatures for about 3,000 gas and has no
Ed25519 function → a participating node has a second key.

- **Credit key** = secp256k1 key. Address = last 20 bytes of Keccak-256 of the uncompressed
  public key without `0x04` prefix (64 bytes), as in Ethereum.
- Signs receipts (§3) and the node's ledger transactions (§7).
- **No signature ties it to the node ID.** Credit address is stated on a file stream; the
  connection is authenticated (`Protocol.md` §6.2), so it comes from that node; the first receipt
  with a `proof` (§5.2) proves the downloader holds the key. Naming an address not held gains
  nothing: as uploader it gives credits away, as downloader its receipts fail.
- Ledger never sees a node ID (§9).
- A credit address is used by **one node**. Two nodes under one address keep separate counters,
  so their receipts fail at an uploader both use (§4.2) and only one can bid there (§5.2).
- A node MAY change its credit address; new one starts without balance and immature (§6.3);
  credits movable by transfer, at its price (§6.5).
- Key file is as private as the node key. A client MUST NOT write the private key to a log.

## 3. Receipts

The only signed object of this extension; carries both what was delivered and what was spent.

```
Receipt(address issuer, address holder, uint32 day, uint64 kib, uint64 spent)
```

| Field | Meaning |
|---|---|
| `issuer` | downloader's credit address |
| `holder` | uploader's credit address |
| `day` | day of the issuer's clock = day the counters belong to; holder accepts today, yesterday, tomorrow by its own clock (§4.2) |
| `kib` | KiB of file data issuer received from holder **on that day**, verified, over all files; cumulative |
| `spent` | µ issuer spent at holder **on that day**; cumulative |

- Both counters only grow within a day, start at 0 on a new day.
- Signature: EIP-712 by issuer's credit key, ledger domain
  `{name: "ed3k credit", version: "1", chainId, verifyingContract}` (§7.1). 65 bytes on the
  wire, `r ‖ s ‖ v`, `s` in lower half, `v` 27 or 28.
- Holder keeps the **latest** receipt per issuer and day; an older one is worthless once a newer
  one is settled.
- Issuer keeps its two counters per holder and day and MUST NOT lose them within the day; if it
  does, holder returns the last receipt (§4.1).
- `kib` counts bytes of blocks that passed verification (`Protocol.md` §7.2), before
  compression. 1 KiB = 1024 bytes; a rest below 1 KiB carries to the next receipt.

**Value.** A receipt mints `floor(kib × 15625 / 16)` µ = 1 credit per MiB, computed by the ledger
on the cumulative value, so rounding does not add up.

### 3.1 When a downloader signs

A downloader with `CREDIT` signs for an uploader that named a `payee` (§4.1):

- once it learns `payee` and `challenge`, if it bids: receipt with counters as they stand plus
  `proof` (both counters 0 for a pair without a receipt that day). Mints/pays nothing; proves
  downloader holds the key of `payer` (§5.2),
- after every verified **tranche**,
- at turn end (`Protocol.md` §12.5), for the rest, and
- at its own midnight, if holding a slot there: a **closing receipt** for the old day with all
  verified so far, then a new-day receipt with `kib` = 0 and `spent` = 0. Later verification
  counts under the new day.

Sent on the file stream as message type 35 (§4.2). Signing costs nothing.

An uploader MAY rank a non-signing downloader lower in its queue; local policy, like credits of
`Protocol.md` §12.1. It MUST still serve nodes without `CREDIT`.

### 3.2 When a receipt carries a payment

In a priority slot (§5) the downloader pays **one tranche ahead**. Uploader states its need as
`due` in `CreditOffer` (§4.1); downloader signs that as `spent` if within its own bound.

```
uploader:
due   = spent at the start of the turn
      + Σ ceil(price of each tranche delivered in this turn × its bytes / 2²⁰)
      + ceil(price now × 4 MiB / 2²⁰)                      prices in µ per MiB, as in §5.2
      − carry                                               paid by the previous priority turn at this uploader today and not used

downloader:
bound = spent at the start of the turn
      + ceil(max_price × bytes it verified in this turn / 2²⁰)
      + 4 × max_price                                       one tranche ahead
```

- Amounts are whole µ, rounded up per tranche. A partly delivered tranche (stream live) counts bytes delivered so far.
  `carry` ≤ one tranche, 0 without earlier turn. `due` ≥ 0: if `carry` exceeds the rest of the
  sum, `due` is 0. Sides need not compute the same number: `due` is the uploader's; the bound
  protects the downloader.
- `due` goes in the slot-granting `FileStatus` and in another whenever it changes: on sending the
  last byte of the paid tranches (paid mark), with a new `price`, and after the first receipt of a
  new day. At the paid mark `due` covers delivered tranches whole and the next one ahead, so the
  last tranche's receipt can pay it at once; late-payment 30 s (§5.2) count from that
  `FileStatus`.
- First receipt of a priority turn pays the first tranche. Uploader serves `BlockRequest`s only
  up to the end of the paid tranche; a range beyond is held, counting among the 16 outstanding
  (`Protocol.md` §12.2), until its tranche is paid; not refused.
- Each further receipt adds the tranche just received to `kib` and carries the last `due` as
  `spent`.
- A downloader MUST NOT sign a `spent` above `due`, nor above its bound. `due` above the bound →
  downloader signs `kib` with `spent` unchanged and waits for the next `due`. None within the bound → uploader
  takes the slot away as for a late payment (§5.2); downloader SHOULD NOT bid there again.
- Downloader stops paying when it stops receiving; loss ≤ one tranche.
- An unused payment (turn ended inside a tranche, or ended by a higher bid, §5.3) counts towards
  the next priority turn at the same uploader the same day.
- **A new day inside a turn.** Counters restart at 0 (§3), but paid stays paid: uploader carries
  the paid byte mark across the day boundary. From the new day's first receipt (§3.1), `due`
  and bound count in that day: "spent at the start of the turn" reads as 0, the `due` sum runs
  from the next unpaid tranche, the bound's bytes from the closing receipt. A `due` that crossed
  with that receipt still counts in the old day; the bound keeps what the downloader signs for it
  below one tranche, and that is `carry`.

## 4. Wire

### 4.1 Fields on the file stream

```protobuf
// in FileRequest (Protocol.md §12.1):
  CreditBid credit = 9;

// in FileStatus:
  CreditOffer credit = 11;

message CreditBid {
  bytes  payer = 1;           // 20 B, my credit address; in the first FileRequest of the stream
  optional uint64 max_price = 2; // µ per MiB I pay at most for a priority slot; absent = unchanged; 0 = bid withdrawn
}

message CreditOffer {
  bytes  payee = 1;           // 20 B, the uploader's credit address; sent once per stream
  optional uint64 floor = 2;  // µ per MiB, lowest bid taken; absent = no priority slots
  uint64 clearing = 3;        // µ per MiB a bid has to reach to get the next priority slot
  uint32 used = 4;            // priority slots in use
  uint32 slots = 5;           // priority slots at most (§5.1)
  optional uint64 price = 6;  // µ per MiB charged from the next tranche on; with rank = 0 only; absent = unchanged
  Receipt last = 7;           // the receipt of the newest `day` I hold from `payer`, if any
  uint64 due = 8;             // cumulative `spent` the uploader needs before it serves the next tranche, in µ; with rank = 0 only (§3.2)
  bytes  challenge = 9;       // 16 B, random, new per queue entry
}
```

- Downloader sends `credit` only to a peer with `CREDIT`, only if willing to sign receipts; `payer` alone (no bid)
  says so. If sending `credit`, the stream's first `FileRequest` goes after reading the peer's
  `Hello`; peer answers nothing before that anyway (`Protocol.md` §5.5).
- Uploader answers with `credit` only if the `FileRequest` carried one. `payee` alone, no
  `floor` = I take receipts, sell no priority.
- Uploader with a `floor` sends `challenge` with `payee`: random at queue-entry creation, same on
  every stream of the entry, new per entry. `challenge` counts for the rest of the stream once a
  `CreditOffer` of the stream carried it. `floor` set later on a stream → `challenge` goes with the first
  `CreditOffer` having `floor`. Until a `CreditOffer` of the stream carried `challenge`, `floor`
  makes no bid eligible; downloader treats uploader as selling no priority.
- **The bid is given blind,** in the queue entry's first `FileRequest`. Price charged is set by
  the next bid, not one's own (§5.2), so downloader names the most it would pay, need not see
  `clearing` first. `clearing` is info for the user.
- A bid stays with the queue entry (`Protocol.md` §12.5), over all its streams. `max_price`
  absent = unchanged; in the queue entry's first `FileRequest` that is no bid. `max_price` = 0
  withdraws it.
- Bid change = `FileRequest` with another `max_price`. A **raise** at most once per 10 min
  *(initial)*, counted from the last raise. A **lower** `max_price`, `0` included, any time, not
  counted, so a bidder can stop paying at once. Same value or none = no change, not counted. Over
  the limit: on an open stream ignored (an `Error` would end it). On a new stream's first
  `FileRequest` a raise counts as an early return of `Protocol.md` §12.5 and is answered per
  §12.5; a lower bid is no reason to return early (an entry away pays nothing).
- A `FileRequest` for another file inside a turn (`Protocol.md` §12.5) keeps the bid; a priority
  turn goes on at the same `price`, `due` keeps counting.
- A `payer` other than the queue entry's **voids the bid** (§5.2): entry keeps its normal-queue
  place, no bid until a `max_price` and a receipt with `proof` for the new `payer` arrive. Not
  one of the two void bids of §5.2.
- `last` = receipt of newest `day` held from `payer`, w/o `proof`; lets a downloader that lost
  its counters go on. Downloader MUST verify the signature is its own, uses values only if `day`
  is its own day, continues from the higher of own and these. Whoever names `payer` gets `last`,
  so a receipt alone proves nothing about the sender (§5.2).
- Uploader with a `floor` SHOULD NOT close a bidding entry's stream (`Protocol.md` §12.5,
  `recheck` above 0) before the §3.1 receipt arrived or 30 s *(initial)* passed. Only a receipt
  with verifying `proof` ends the wait early; 30 s count from the `CreditOffer` that carried
  `challenge` on this stream. Downloader knowing `payee` and `challenge` from an earlier stream
  of the same entry MAY send that receipt right after its `FileRequest`.

Both ledger parameters (chain ID, contract address) are fixed per network (§7.1), not sent.

### 4.2 `Receipt`

```protobuf
message Receipt {             // type 35, downloader → uploader, on the file stream
  uint32 day = 1;
  uint64 kib = 2;
  uint64 spent = 3;
  bytes  sig = 4;             // 65 B
  bytes  proof = 5;           // 65 B
}
```

`issuer`/`holder` = the stream's `payer`/`payee`. Uploader verifies the signature and that

- `day` is today, yesterday or tomorrow by its own clock,
- `kib` growth over the receipt held for that `day` (0 if none) ≤ bytes delivered to this issuer
  not yet covered by a receipt. Uploader keeps **one** such counter per issuer, over all days,
  lowered by what a receipt covers; so the accepted day a byte is booked under does not matter,
  and a closing receipt and a new day's first (§3.1) both pass,
- `kib` ≤ 52,428,800 (50 GiB, §6.2); above is refused, since the contract would skip it,
- `kib` not below the held receipt's for that `day`: counters only grow (§3),
- `spent` not below the held receipt's for that `day`.

A failing receipt is dropped. Receipts are not answered. **Whether `spent` covers `due` (§3.2)
does not make a receipt valid or not.** It decides only whether the uploader serves the next
tranche: a last receipt paying less than due still counts for its `kib` and what it pays.

**`proof`**: EIP-712 signature by issuer's credit key, domain and 65 bytes as for a receipt (§3),
over struct hash
`keccak256(abi.encode(keccak256("Bid(address payer,address payee,bytes16 challenge)"), payer, payee, challenge))`.
`payer`/`payee` the stream's, `challenge` from this queue entry's `CreditOffer`. As in EIP-712,
each address left-padded and `challenge` right-padded with zeros to 32 bytes. Contract has no
type hash for it: only the uploader checks `proof`. It makes an entry eligible (§5.2), needed
once per queue entry, in the receipt of §3.1; later receipts MAY omit it. A non-verifying
`proof` does not void the receipt: it counts as one without `proof`. `proof` is not part of the
signed receipt nor of what `settle` verifies (§6.2); omitted there and in `SignedReceipt`
(§4.3).

**A receipt dated tomorrow pays nothing yet.** Its `spent` counts as payment (towards `due` and
the next tranche) only once its `day` is not above uploader's today: the ledger cannot settle a
day not yet begun. Its `kib` counts at once, as above. Such a payment covering `due` is not late
(§5.2): uploader holds the next tranche, and the late-payment 30 s start at its own midnight.

### 4.3 `CreditRelay`

Sending a transaction needs a little native coin of the ledger chain (§7.3). A node with none,
or a hiding one (§9.1), asks a peer to send its receipts and holds.

```protobuf
message CreditRelay {         // type 36, request
  repeated SignedReceipt receipts = 1;   // at most 32 (initial)
  repeated SignedHold holds = 2;         // at most 8 (initial)
}

message SignedReceipt {
  bytes  issuer = 1;          // 20 B
  bytes  holder = 2;          // 20 B
  uint32 day = 3;
  uint64 kib = 4;
  uint64 spent = 5;
  bytes  sig = 6;
}

message SignedHold {          // the arguments of holdFor (§6.5)
  bytes  payer = 1;           // 20 B
  bytes  payee = 2;           // 20 B
  uint64 amount = 3;          // µ
  uint64 nonce = 4;           // above every nonce this payer used for this payee
  uint64 deadline = 5;        // unix time in seconds; the ledger refuses the hold after it
  bytes  sig = 6;             // 65 B, by the credit key of `payer`
}

message CreditRelayAnswer {   // type 37, answer
  bytes  tx = 1;              // 32 B, hash of the `settle` transaction that was sent; absent if none was. A `holdFor` is not named; the asker reads `holdOf`
}
```

- Request with more than 32 receipts or 8 holds *(initial)* → `Error{MALFORMED}`.
- Any node with `CREDIT` MAY relay; one that does not → `Error{BUSY}` w/o `retry_after`, asker
  tries another. A non-verifying receipt or hold → `Error{DENIED}` (`MALFORMED` is
  for an unparsable body, or an address or `sig` of wrong length, `Protocol.md` §5.6). Same for
  one the ledger would refuse: receipt with zero-address `issuer` or `holder`, `issuer` = its
  `holder`, `kib` above 52,428,800 (50 GiB), or `day` after relay's today or more than 7 days
  back; hold with zero-address `payer` or `payee` or passed `deadline`, and a hold that
  `eth_estimateGas` shows would revert. If any receipt or hold fails, the whole request →
  `Error{DENIED}` and relay sends none of it.
- Relay calls `settle` (§6.2) from its own account. Receipts say who is credited; relay cannot
  change that. Paid by the gas reward of §6.6.
- Relay sends the call when ≥ 1 receipt, by itself, would mint or take at least 1 credit for a
  pair and day not yet rewarded (the reward threshold, §6.6). Computed with §7.3 views:
  `settled()` of pair and day, `accountOf()` of issuer, `issuedKib()` and `pairKib()` of the
  receipt's week. An issuer not mature, or `overdrawn`, counts as minting nothing; what it pays
  is still taken (§6.2). Minted stays within what is left of both weekly limits (§6.3). Taken ≤
  issuer's balance, `earned` + `received` of `accountOf()`. A receipt at most 1 day old whose
  `spent` growth exceeds that balance sets `overdrawn` before its mint (§6.2), so mints nothing
  either. Holds for other payees cannot be read, so this is an upper bound. `settle` returns
  nothing, so nothing to simulate.
- **Nothing worth sending** (verifying receipts all below threshold, no hold the relay sends) →
  `Error{BUSY}` with `retry_after` above 0. Relay keeps nothing; asker returns with newer
  receipts. Until `retry_after` passes it sends those receipts to no relay: every relay computes
  the same.
- **A hold** = one `holdFor` call each (§6.5). Amount in µ, fits `uint64` like `spent`. No ledger
  reward for holds: a hold burns and mints nothing, so a reward would pay for filling blocks. Its
  `payee` has the reason to send it: earns the uploader's share (§6.2) of the enabled turn. A node SHOULD send a hold naming its own
  credit address as `payee`, and MAY refuse any other with `Error{BUSY}`.
- **Whom to ask** *(SHOULD)*: hold → its `payee` (above); receipts → connected peer with
  `CREDIT`, receipts' issuer first. A hiding node asks only peers of its sessions (§9.1), never
  its own main node; the main node asks no peer over a path. Receipts and holds in separate
  requests, since one failing item fails the request.
- Asker checks the outcome itself: `settle` by tx hash or `settled()`, hold by `holdOf()`, or,
  if it cannot read (§9.1), a hold by the slot offered and a `settle` by a second relay (§9.1).
- Part of each reward goes to the receipt's holder, so a node needs a relay for its first
  settlements only.

## 5. Priority slots

### 5.1 The lane

An uploader with a `floor` gives some upload slots to bidders.

- **At most two thirds** of its slots, rounded down; 1 if it has 2. At least one slot always
  serves the normal queue. An uploader SHOULD NOT exceed this.
- A priority slot no bid asks for serves the normal queue; a later bid can end that turn (§5.3).
- A priority slot is a **turn** like any other: 16 MiB or 1 h (`Protocol.md` §12.5).
- After 16 MiB the uploader MAY go on while no other bid waits and the downloader keeps paying.
- A bid does not move the entry in the normal queue, either way. A downloader whose bid never
  wins is served as if it had not bid.
- `floor` SHOULD be at least 1,000,000 µ per MiB (1 credit) *(initial)*.

### 5.2 Who gets the slot, and at what price

When a priority slot is free, uploader takes the waiting entry with highest `max_price` that is
at least `floor` and whose payer is **eligible**:

- a valid receipt (§4.2) for (`payer`, `payee`) arrived on this queue entry with a verifying
  `proof` (§4.2); need not add to `kib` or `spent` (§3.1). `payer` is unsigned and receipts can
  be replayed, so otherwise anybody could bid under another node's address. A receipt without
  valid `proof` counts for its `kib` and `spent` but makes no entry eligible,
- no other queue entry still queued or served sent a valid `proof` for this `payer` before: one
  node per credit address (§2),
- `holdOf(payer, payee)` (§6.5) returns `amount` ≥ 16 × `max_price` µ (a whole 16 MiB turn at the
  bid) and an `until` at least 2 h *(initial)* ahead: a turn, the read's age, and time to settle
  after the turn,
- account not `overdrawn` (§6.4),
- both read via the RPC of §7 no more than 10 min *(initial)* ago.

A **hold** makes the bid honest: downloader locks that amount for this uploader for 6 h
*(initial)*, and cannot meanwhile move it or lower the hold.

- Bidder places one hold per uploader bid at, each one turn at its bid, renewed every 3 h *(initial; half the
  time of a hold)* while bidding. Sum of holds = what it can lose; ledger refuses holds beyond
  90 % of the balance, at every renewal.
- A settled payment uses the hold up (§6.5); for a further turn the bidder raises it again, at
  the latest with its next renewal.
- A bidder that stops lets the hold run out, then releases it (`hold` with `amount` = 0).
  Uploader MAY release it earlier (`release`, §6.5).
- **The uploader MUST settle a receipt whose `spent` grew, and do so before the `until` it
  read** (§6.2). It MAY settle inside a turn. Serving for payments it cannot settle before
  `until` is at its own risk.
- A hiding node (§9.1) sends no transaction of its own; it signs its hold and sends it via a
  relay, as a rule the uploader it bids at (§4.3).

Equal bids go by queue position. Slot offered as any slot (`rank` = 0 on the open stream, or
`SlotOffer`), with `price` set:

```
price = min( own max_price,  max( floor,  highest other waiting eligible bid + tick ) )
tick  = 10,000 µ per MiB (initial)
```

- Second-price rule: winner pays just above the next bidder, not its own maximum; naming the
  true maximum is the best bid.
- **The price holds for a tranche.** Waiting bids change → uploader recomputes the price and sends it in a
  `FileStatus`; it applies from the next tranche paid for. Never above `max_price`. Binding price of
  a tranche = the one sent before that tranche's receipt arrived; a receipt at the previous price
  for the first tranche after a change is accepted (they cross on the stream).
- Downloader sends the turn's first receipt (§3.2) within 30 s *(initial)* of the `FileStatus`
  carrying `price`, sent on an open stream. Else uploader takes the slot away and the bid is
  **void**: entry stays in the normal queue, no priority slot until a valid receipt from it
  arrives; after two void bids none for 24 h *(initial)*.
- A failed `SlotOffer` (`Protocol.md` §12.5) is no void bid (no `price` was sent); the failure
  counts as that section says.
- Void bids counted **per node ID, never per `payer`**. A `payer` not one's own is never
  eligible, so naming it gains nothing and costs its owner nothing.
- A later payment 30 s late (from the `FileStatus` carrying its `due`) → uploader takes the slot
  away; entry stays in the normal queue with its bid. A tomorrow-dated receipt covering `due` is
  not late (§4.2).
- `clearing` in a `FileStatus` = price a new bid must reach now to be first among waiting bids:
  `floor` if no eligible bid waits, else highest waiting eligible bid + `tick`.

### 5.3 A bid ends a running turn

When an eligible bid waits and no slot is free, uploader picks one running turn:

| Bidders hold | Turn it picks |
|---|---|
| fewer slots than the lane has (§5.1) | a turn **from the normal queue**: of those with more than 60 s left, the one that received most of its 16 MiB |
| every slot of the lane | priority turn with lowest `max_price`, if the bid is above that `max_price` |

A bid never takes the lane above two thirds, so at least one normal turn always runs on. For the
picked turn uploader computes

```
remaining = bytes left of that turn / its rate over the last 10 s (initial)
```

| `remaining` | The uploader |
|---|---|
| more than 60 s *(initial)* | ends the turn after the block being sent, gives the slot to the bidder |
| 60 s or less | lets the turn run out; bidder gets the slot then |

- Bytes left = rest of the 16 MiB or rest of what the downloader still needs from this
  uploader, whichever is less. A turn sending nothing counts as more than 60 s.
- If every normal turn has ≤ 60 s left, the bidder gets the first of them that ends.
- A bid above a running priority turn's price but not above its `max_price` ends nothing; it
  raises the price (§5.2).
- **A bidder whose turn was ended** keeps normal-queue entry and bid; paid-ahead counts for its
  next priority turn that day (§3.2); if none, it lost up to one tranche.
- **A normal-queue downloader whose turn was ended** keeps its entry; uploader SHOULD give it
  the next slot freed for the normal queue, before anyone else. Its turn bytes restart at 0.
- Uploader tells it via `FileStatus` with its `rank` and the new `clearing`.
- A downloader SHOULD write a block that arrived whole, and SHOULD ask another source for the
  rest of the tranche.

### 5.4 The downloader's side

- A client SHOULD bid per file, with a user-set limit, and SHOULD offer a limit for credits spent
  per day.
- It SHOULD NOT bid at an uploader whose `floor` is above its limit.
- Exposure: one tranche per uploader where it holds a priority slot. Locked: one turn at its bid
  per uploader bid at, for 6 h from last renewal (§5.2); these holds sum within 90 % of its
  balance (§6.5).
- A downloader that signed more `spent` in total than it has is **overdrawn** once receipts are
  settled (§6.4). A client MUST keep its open payments summed at or below 80 % *(initial)* of
  its balance as last read, so two days of decay (§6.5) cannot leave it short. Open payments =
  `spent` signed for today and yesterday, over all holders: only those can set `overdrawn`
  (§6.2). A receipt settled more than 1 day after its `day` takes what is there and does not flag
  the account (§6.2).

## 6. The ledger

One contract holds every balance; immutable, no owner. The operator contract (§7.2) can set two
values, the stake token parameters (§6.7), nothing else. This section says what the ledger
does; source: [`contracts/CreditLedger.sol`](../contracts/CreditLedger.sol). Where the two differ,
that is a fault to report, and this text is meant.

### 6.1 Accounts

```solidity
struct Account {
    uint128 earned;         // µ, from receipts
    uint128 received;       // µ, from transfers and from the uploader's share of payments (§6.2)
    uint32  lastActive;     // day of the newest settlement that counted as activity
    uint64  decayedUntil;   // decay is applied up to this time
    uint64  registeredAt;   // first appearance
    uint64  inflow;         // KiB received credit for, from mature issuers
    uint16  issuers;        // distinct mature issuers among them
    bool    overdrawn;
}
```

- `balanceOf` is `earned + received` after decay (§6.5).
- An account is **registered** on first appearing, as issuer or holder, in a receipt passing
  every check of §6.2, whether or not it mints.

### 6.2 Settlement

```solidity
function settle(Receipt[] calldata receipts, bytes[] calldata sigs) external;
```

Anyone may call it. Per receipt the contract recovers the signer and skips the receipt unless

- signer is `issuer`, and `issuer` is not `holder`,
- `day` not in the future and not more than 7 days *(initial)* back,
- `kib` ≤ 52,428,800 (50 GiB in a day) *(initial)*,
- `kib` and `spent` not below what is settled for (issuer, holder, day).

A non-verifying signature recovers the zero address, so a zero `issuer` or `holder` is skipped
too: a zero-issuer receipt with a bad signature would otherwise pass the first rule.

Then, in order:

1. **Take.** Growth of `spent` taken from issuer, `received` first, after decay. Short balance →
   what is there is taken. Account set `overdrawn` only if the receipt is at
   most 1 day *(initial)* old, today − `day` ≤ 1 by the chain's clock; an older receipt takes
   what is there w/o the flag, rest no longer owed. A payment does not dip into the issuer's holds for
   other payees (§6.5): takes its own hold as far as balance less those holds covers it, beyond
   its hold only what is left above them ÷ 0.9; the missing rest is short like any other. Taken
   also comes out of the issuer's hold for this holder (§6.5). **10 % of what is taken goes to the uploader**, into the holder's `received`; the rest
   is burned ([`docs/credit-ledger.md`](../docs/credit-ledger.md) §9).
2. **Mint.** If issuer is mature (§6.3) and not overdrawn, growth of `kib` is minted to holder's
   `earned`, up to what is left of the issuer's weekly limit and the pair limit (§6.3). Excess is
   lost.
3. **Activity.** If ≥ 1 credit was minted, holder's decay is written up to now and `lastActive`
   becomes today by the chain's clock. Grace runs from the settlement day.
4. **Gas reward:** receipt earns the reward of §6.6 if first of its pair and day to mint or take
   at least 1 credit.

- Issuer **not mature** → step 1 only. Settled `kib` stays put, so the receipt can be settled
  again once the issuer matured, within the 7 days. A holder reads `isMature` (or `accountOf`)
  before settling a receipt taking nothing, and settles an immature issuer's only once it is
  mature and not `overdrawn` (`accountOf`); an overdrawn issuer
  never mints (§6.4). A holder that cannot read tries once per 48 h round.
- A holder MUST settle a receipt whose `spent` grew, and do so **before the `until` of the
  hold** it read for that turn (§5.2). After `until` the issuer can move the credits away.
- A holder MUST settle its other receipts **at least every 48 h** while it has new ones, at a
  random time, not right after a transfer (§9). The 7 days are what the contract takes.
- A holder settling a payment late cannot brand the issuer `overdrawn` (§6.4) for a balance
  decayed since signing: step 1 flags only while the receipt is at most 1 day old.

### 6.3 Maturity and limits

Signing is free, so who may mint is the whole question. A receipt mints only if its issuer is
**mature**:

| | Rule *(initial, not simulated)* |
|---|---|
| Age | registered ≥ 30 days |
| Contribution | credit minted to it for ≥ 1 GiB, from ≥ 3 distinct mature issuers |
| Roots | operators' credit addresses (§7.2) mature from the start, one per operator |

| Limit | Value *(initial, not simulated)* |
|---|---|
| Minted from one issuer, over all holders, per week (fixed bins of 7 days, `day / 7`; not a rolling window) | 100 GiB |
| Minted from one issuer to one holder, per week (same bins) | 10 GiB |
| `kib` of one receipt | 50 GiB |
| Age of a receipt at settlement | 7 days |

- A new node downloads, earns, spends from day one; only receipts it *signs* do not mint yet. Its
  uploaders are credited later, if it matures within the 7 days a receipt can be settled in
  (§6.2).
- Making one more key a mature issuer: attacker uploads 1 GiB to three mature issuers through it
  and waits 30 days. What the key can then mint is bounded per week.
- **Pair limit** keeps one issuer from pouring its week onto one holder: using its 100 GiB a key
  must sign for 10 holders, so a mature user signing for a friend gives it 10 GiB a week, no
  more. A real download >10 GiB from one uploader in a week mints 10 GiB; rest lost to the
  uploader.
- **A bound, not a proof.** A group of keys can mint to each other up to each mature key's
  weekly limit, when the group has at least 11 of them. The rules make that cost real upload
  first.
- **Not bounded by the limits:** how fast such a group grows; three mature keys can sign the
  contribution of any number of new keys (§13).

### 6.4 Overdrawn

Spent more than it had → flagged for good: no priority slot (§5.2), receipts mint nothing, no
transfer, no hold. Such a node starts a new credit address, immature and empty. Only a receipt
settled by the end of the day after its `day` *(initial)* sets the flag (§6.2).

Bounds what one address takes by signing payments to many uploaders at once: one tranche each.
A new address costs the 10 % transfer burn (§6.5), so the bound is per address, not per node.

### 6.5 Decay and transfers

**Decay.** No timer; balance computed on read, written back at the next change.

| | Value |
|---|---|
| Grace | 3 days after `lastActive`, no decay |
| Then | × 0.9 per full 24 h, on current balance, rounded up (x − ⌊x/10⌋) |
| `earned` | stops at the first step that takes it below 10 credits |
| `received` | goes on; below 1 credit → 0, except while the account holds something (then stops below 10 µ) |
| Activity | a settlement that minted ≥ 1 credit, on its settlement day. Nothing else |

An account that stops uploading keeps about 31 % after two weeks, ends just below 10 credits.
`poke(address)` writes an account's decay back; anyone can call it.

**Transfers.** ERC-20 interface (`name`, `symbol`, `decimals` = 6, `balanceOf`, `transfer`,
`transferFrom`, `approve`, `allowance`, `totalSupply`) plus `permit` of EIP-2612.

- **10 % of every transfer is burned** *(initial)*; recipient gets the rest.
- Takes from `received` first, then `earned`; always arrives in `received`. So splitting over
  many addresses cannot keep a balance under the floor of 10.
- A transfer to one's own address burns 10 % like any other.
- Receiving a transfer is **never** activity, never counts for maturity.
- Every address decays, contracts too; an exchange pool is an inactive account.
- `balanceOf` falls with time without an event. `totalSupply` = supply as last written, an upper
  bound.
- A transfer costs gas, earns no reward (§6.6).

**Holds.**

```solidity
function hold(address payee, uint256 amount) external;
function holdFor(address payer, address payee, uint256 amount, uint256 nonce, uint256 deadline, bytes calldata sig) external;
function release(address payer) external;
function sweep(address payer, address payee) external;
function holdOf(address payer, address payee) external view returns (uint256 amount, uint64 until);
```

- `hold` locks `amount` µ of caller's balance for `payee` up to `until` = 6 h *(initial)* after
  the call. One hold per payer and payee; a later call replaces it and resets `until`.
- **While a hold runs, its payer can only raise or extend it.** Lower `amount` refused before
  `until`. After `until` the payer may lower it (checked as a new hold, below); `amount` = 0
  releases it.
- Payee gives a hold up with `release(payer)`, any time.
- On placing, raising or renewing, the sum of the payer's holds (incl. expired ones not released
  or swept) MUST fit 90 % *(initial)* of its balance after decay, else the call fails. A hold
  lasts 6 h, so ≤ one decay step falls within it; every hold stays covered until its `until`.
  `amount` = 0 is unchecked, so a release never fails.
- The margin stays with the holds. A payment beyond the payee's own hold, and a transfer, take
  only what is left above the other holds ÷ 0.9; while the account holds something, `received`
  below 1 credit is not cut to 0.
- `holdFor` = `hold` for a payer sending no transaction; anyone may send it. EIP-712 signature
  by payer's credit key over
  `Hold(address payer,address payee,uint256 amount,uint256 nonce,uint256 deadline)`, domain and
  65 bytes as for a receipt (§3). `nonce` MUST be above every nonce the payer used for that payee
  before (`holdNonces(payer, payee)`), so holds for different uploaders land in any order; not
  necessarily the next number, so ms time serves, no read needed. Refused after
  `deadline` (unix seconds). A signer SHOULD set it at most 10 min *(initial)* ahead: how long a
  relay can keep a hold back.
- `holdOf` returns both values as stored. Reader compares `until` with the time: a hold protects
  nothing once `until` passed.
- A transfer is refused if it would leave the account below the sum of its holds ÷ 0.9, rounded
  up. An expired hold still counts there, and for the cut of `received` below 1 credit, until
  the payer releases or replaces it, or anyone removes it with `sweep(payer, payee)`.
- Settlement (§6.2) is not refused: the receipts of `payee` are what the hold is for. What a
  settlement takes from `payer` for `payee` comes out of that hold, down to 0.
- A hold is why an uploader can serve a priority turn before settling (§5.2).

### 6.6 Gas reward

Gas has a fixed, very low price (§7.2), in a native coin otherwise useless. The contract mints
that coin for useful work, via the chain's native minter:

- per receipt in a `settle` that minted or took ≥ 1 credit, once per (issuer, holder, day): a
  later receipt of the same pair and day earns nothing, in this call or a later one, so
  splitting a delivery does not pay. Amount fixed at deployment *(initial: twice the gas of a
  `settle` of the first receipt of a pair and day, accounts and weeks known, minter included,
  §11)*;
- 60 % to the caller, 40 % to the receipt's holder; all to the holder if it is the caller.

A receipt that changes less, a failed call, a transfer and a hold (`hold`, `holdFor`, `release`,
`sweep`) earn nothing and cost gas. Settling own receipts gains a little coin each; other
transactions use it up.

### 6.7 Stake token

Credits can be burned for a second token, the **stake token** (`E3S`,
[`contracts/StakeToken.sol`](../contracts/StakeToken.sol)), meant as stake for a possible later
staking contract not part of this spec. Traded freely.

```solidity
function burnForStake(uint256 value, address to) external returns (uint256 id);
function claimStake(uint256 id) external;
function setStakeParams(uint256 rate, uint256 delay) external;   // the operator contract only
function stakeClaims(uint256 id) external view returns (address to, uint64 readyAt, uint256 amount);
function stakeRate() external view returns (uint128);
function stakeDelay() external view returns (uint64);
function stakeToken() external view returns (address);
```

- `burnForStake` burns `value` µ of caller's balance (after decay, `received` first), records a
  **claim** of `value × stakeRate / 10⁶` stake units for `to`, ready at now + `stakeDelay`,
  returns its `id`. Ids start at 1; `nextClaimId` = id of the next burn. Burn to `to` = 0
  refused.
- **Rate and delay are fixed at the burn.** Later change of either does not touch a waiting claim.
- `claimStake` mints a claim to its `to` once `readyAt` passed. Anyone may call it; paid once.
- A burn never dips into holds: like a transfer it leaves the holds ÷ 0.9 (§6.5). An `overdrawn`
  account cannot burn. A burn coming to 0 stake units is refused.
- A burn is not activity, does not count for maturity, earns no gas reward (§6.6).
- Irreversible; nothing turns stake tokens back into credits.
- A burn's `id` is in its log `StakeBurned(uint256 indexed id, address indexed from,
  address indexed to, uint256 credits, uint256 amount, uint64 readyAt)` in the receipt, from the
  ledger's address. `claimStake` logs `StakeClaimed(uint256 indexed id, address indexed
  to, uint256 amount)`. A paid claim reads in `stakeClaims` as `to` = 0.

| Parameter | Initial *(not simulated)* | Bound |
|---|---|---|
| `stakeRate` | 10¹⁸ stake units per credit: 1 E3S for 1 credit | at most 2¹²⁸ − 1; 0 turns burning off |
| `stakeDelay` | 30 days | at most 365 days |

**Who sets them.** Operator contract (§7.2), after a two-thirds vote, calls `setStakeParams`; no
other account can. Values at deployment are constructor arguments.

**The token.** E3S: plain ERC-20, `permit` of EIP-2612 and 18 decimals. No decay, transfers burn
nothing, no owner. Ledger creates it at deployment, sole minter. EIP-712 domain
`{name: "ed3k stake", version: "1",
chainId, verifyingContract}` of the token, not the ledger.

**Where it trades.** Only the operator contract deploys on the L1 (§7.2): no exchange there.
Home is the L1: operators deploy the Avalanche Interchain Token Transfer (ICTT) home contract,
locking E3S on the L1, minting a remote E3S on Avalanche's C-Chain, where exchanges trade it. Bridge contracts not part of this spec (§13).

Why burning not wrapping, why a delay, why operators set the values:
[`docs/credit-ledger.md`](../docs/credit-ledger.md) §10.

## 7. The chain and the RPC

### 7.1 Parameters of a network

| Parameter | Public network |
|---|---|
| Chain ID | *to be assigned at deployment* |
| Ledger contract address | *to be assigned at deployment* |
| RPC endpoints | a list in client configuration; none built into the protocol |

A named network (`Protocol.md` §4.3) has its own ledger or none; a node of it MUST NOT use the
public network's ledger. Until both parameters are assigned no receipt can be signed or verified
on the public network. A client takes both from its network, has no default, sets `CREDIT` only
when it has them. Test vectors use a test domain: chain ID 31337 and contract address
`0x5fbdb2315678afecb367f032d93f642f64180aa3`.

### 7.2 The chain

- An **Avalanche L1** running the EVM (Subnet-EVM), own validators.
- **Validators are operators**, admitted by vote: an operator contract owns the L1's validator
  manager; two thirds of operators add or remove one. No stake, no project key after genesis. An
  operator runs a validator and SHOULD run a public RPC endpoint.
- **Gas**: static price 1 wei, the lowest the chain accepts.
- **Only the operator contract may deploy contracts**, so the chain carries only the ledger, its
  stake token (§6.7), the operator contract and the bridge's home contract.
- Operator contract sets the stake token parameters (§6.7).
- Ledger contract is the one account allowed to mint the native coin (§6.6).
- An accepted block is final. Clients read at tag `latest`.

What operators can and cannot do: [`docs/credit-ledger.md`](../docs/credit-ledger.md) §3.

### 7.3 What the ed3k node does over RPC

The ledger node is a separate program; the ed3k node is its client over the chain's Ethereum
JSON-RPC (`https://<host>/ext/bc/<blockchain ID>/rpc`).

| Use | Method |
|---|---|
| Is this the right chain | `eth_chainId` |
| Balance, `overdrawn`, maturity, what is settled, holds (views below); own coin | `eth_call`, `eth_getBalance` |
| Send `settle`, `hold`, `holdFor`, `release`, `sweep`, `transfer`, `poke`, `burnForStake`, `claimStake` | `eth_getTransactionCount`, `eth_estimateGas`, `eth_sendRawTransaction` |
| Outcome; a burn's `id`, from its `StakeBurned` log | `eth_getTransactionReceipt` |

Views this document relies on, as in the source:

```solidity
function balanceOf(address account) external view returns (uint256);              // µ, after decay
function accountOf(address account) external view returns (AccountView memory);
function isOverdrawn(address account) external view returns (bool);
function isMature(address account) external view returns (bool);
function settled(address issuer, address holder, uint32 day) external view returns (uint64 kib, uint64 spent, bool rewarded);
function issuedKib(address issuer, uint32 week) external view returns (uint64);    // week = day / 7
function pairKib(address issuer, address holder, uint32 week) external view returns (uint64);
function holdOf(address payer, address payee) external view returns (uint256 amount, uint64 until);
function holdNonces(address payer, address payee) external view returns (uint256);  // highest nonce used in holdFor
function stakeClaims(uint256 id) external view returns (address to, uint64 readyAt, uint256 amount);  // §6.7
function stakeRate() external view returns (uint128);                                 // §6.7
function stakeDelay() external view returns (uint64);                                 // §6.7
function stakeToken() external view returns (address);                                // §6.7
function nextClaimId() external view returns (uint256);                               // §6.7, id of the next burn

struct AccountView {        // the account of §6.1 with decay applied as of now
    uint256 earned;
    uint256 received;
    uint32  lastActive;
    uint64  registeredAt;
    uint64  inflow;
    uint16  issuers;
    bool    overdrawn;
    bool    mature;
}
```

`isMature` asks the operator contract (§7.2) for the roots. If it is missing or the call fails,
no root is mature by that rule; the call does not fail, `settle` goes on.

- Transactions signed by the credit key; the credit address's account holds the coin.
- **A local node** needs nothing more; follows the L1 without being a validator.
- **A remote node** is believed. A client using remote nodes SHOULD ask two of different
  operators for a read it acts on, acting only if they agree. An `eth_estimateGas`
  deciding a refusal (§4.3) is such a read. A relay unable to get two to agree does not refuse:
  answers `Error{BUSY}` with a `retry_after`. A transaction goes to one node.
- A lying node's power is bounded: make an uploader give one priority turn for nothing, or make
  a node pay gas for a `settle` minting nothing. Cannot move or mint credits; that takes
  signatures.
- A remote node sees the client's address and every credit address asked about. A client SHOULD
  send RPC via the user-configured proxy, if any.
- No RPC endpoint reachable → node keeps signing and collecting receipts, sells no priority, settles
  later.

## 8. What a modified client can do

| Cheat | Effect | Bound |
|---|---|---|
| Uploader takes first payment, does not serve | downloader loses one tranche | 4 MiB at the price; downloader does not bid there again |
| Uploader lies about `clearing` or queue | none on the price | as above |
| Uploader does not settle payments | downloader keeps credits it spent | uploader loses the mint of the same receipt: both one object |
| Downloader does not sign | uploader earns no credits | one tranche; local ranking (§3.1) |
| Downloader stops paying in a turn | none | payment is one tranche ahead |
| Downloader signs payments to many uploaders | each gives up to a tranche for nothing | `overdrawn`, for good (§6.4); fresh address costs a 10 % burn |
| Uploader sends data failing verification | none: only verified blocks count in `kib` | downloader bans it after 2 bad blocks within 24 h (`Protocol.md` §12.3) |
| Two keys of one person sign for each other | credits without upload | maturity, 10 GiB per pair and week, weekly limit (§6.3) |
| Mature user signs large receipts for friends | the same | 10 GiB per friend and week |
| Group of mature keys makes new keys mature | more minting keys, each up to the weekly limit | 30 days and 1 GiB of the group's limits per key; **not bounded in number** (§13) |
| Uploader overcharges to earn its 10 % | downloader pays up to its maximum | never above `max_price`; gain = a tenth of the difference |
| Bidder's turn ended by a higher bid | loses what it paid ahead, if no other turn that day | one tranche |
| Farming credits to sell them | credits get a price (§10) | decay, 10 % on transfer, the limits |
| Node bids under another's `payer` | none | not eligible w/o a `proof` by that key over its own queue entry's `challenge` (§5.2); a replayed receipt makes nothing eligible; void bids count per node ID |
| Uploader settles a payment late | issuer's balance has decayed | receipt older than 1 day sets no flag (§6.2); uploader gets what is there |
| Bidder moves credits away after the turn | none while the hold runs | hold cannot be lowered for 6 h; uploader settles before `until` (§5.2) |
| Relay drops a request | delay | asker asks another |
| Relay keeps a signed hold back, sends it later | credits locked 6 h the payer no longer wanted locked | the `deadline`, 10 min; a higher-nonce hold landing first |
| RPC node lies | §7.3 | one turn, or gas |
| Farming credits to burn for E3S | farming pays in a token with a market price | limits of §6.3, decay up to the burn, delay of §6.7; **the limits now guard a token with a price** (§13) |
| Burn, then move credits away before the claim | none | credits gone at the burn |
| Operators change rate or delay | later burns get more/less, or wait longer | never a waiting claim; delay at most 365 days (§6.7) |

## 9. What others learn

**The ledger is public and stays so.** Per pair of credit addresses: how much one downloaded from
the other, on which day, what was paid. No file, no host address, no node ID.

**A credit address is not secret from peers**: a node tells it to every peer with `CREDIT` opening a file
stream to it. Whoever does that with many nodes maps node IDs and host addresses to credit
addresses, exposing each node's whole trade history on the ledger. The price of a public ledger,
and the main reason to leave the extension off.

- A client MUST leave the extension **off by default** and MUST tell the user the above when it is
  switched on.
- A hiding node MUST NOT send its node's credit address in a session through an anonymous path
  ([`hops.md`](hops.md)); §9.1 says what it may use there. A non-hiding node answers in such a
  session as on any connection.
- A new credit address breaks the link to the past, starts immature.
- Random settle time within 48 h hides single transfers from block order; only for receipts
  without a payment.
- **A hold is public at once**: who bids at whom, how much, when, before a byte moves; and a
  receipt with a payment is settled within hours (§6.2).
- A remote RPC node learns more than the ledger shows (§7.3).
- **A burn is public** (§6.7): ties credit address to address `to`, via the bridge to a C-Chain
  address, where an exchange may know a name. To avoid that link, burn to a new address and move
  nothing back to the credit address.

### 9.1 Through an anonymous path

A hiding node acts under its anonymous key ([`hops.md`](hops.md) §1). It MAY use credits there,
with a credit address of its own:

- Under its anonymous key it sets `CREDIT` whenever its implementation has the extension, like
  every hidden node of it ([`hops.md`](hops.md) §3.3). Credit use shows on the file stream only:
  without an anonymous credit address it sends/answers no `credit` field.
- The **anonymous credit address** belongs to the anonymous key. It MUST NOT be used outside a
  path, and the node's other credit address MUST NOT be used inside one.
- Replaced when the anonymous key is, not more often (see the table).
- **No transfer between the two addresses,** either direction, via no third address the node
  controls; the ledger shows every transfer for good. A client MUST NOT make one on its own and
  MUST warn a user who does.
- **No direct RPC.** Transactions via `CreditRelay` (§4.3) to a session peer. Reads from a ledger
  node on the same machine, via a proxy the user set up for that, or not at all. A node that cannot read sells no priority and does not bid: a bid needs the balance of
  §5.4. It can still sign and earn. Checks a `settle` by a second relay: 48 h after the first relay's
  answer, same receipts, own request, to another. Only a relay answering with `tx` is checked;
  `Error{BUSY}` is no claim. `Error{BUSY}` with `retry_after` at the second relay = they are settled, or below
  threshold (§4.3). If the second sends them, the first did not: the node sends the first no receipts
  again; a hold still goes to its `payee` (§4.3).
- **It bids with a signed hold** (`holdFor`, §6.5), sent by `CreditRelay` to the uploader bid at,
  renewed the same way; the signature needs no read. Ledger pays nothing for a hold; uploader
  sends it for its share of the turn (§4.3).
- Reward gas coin (§6.6) sent to this address stays unused: the address sends no own transaction; the
  relay's part of the reward pays the relay. Gas coin MUST NOT be
  sent to it from the other address.
- **No burn** (§6.7): a burn is its own transaction; `CreditRelay` carries none. A stake token
  claimed for the anonymous address would be a public link like a transfer: no address the node
  controls names it as `to` of a burn, and the node calls no `claimStake` for it. E3S reaching it anyway stays unused, like gas coin.
  A node claims the ids of its own burns, `to` known from their `StakeBurned` log; another id (one
  the user typed) only after reading `stakeClaims(id)`, and not when `to` is the anonymous address.

**A temporary address** is what a hidden node starts with; the rules keeping credits honest
limit it:

| | With a new address |
|---|---|
| Signing receipts | works, mints nothing: address immature. Uploaders earn nothing from it |
| Earning | works at once, if the downloaders are mature |
| Bidding | only with what was earned under this address; hold goes out signed, via a relay (§4.3) |
| Becoming mature | 30 days and 1 GiB to three mature issuers (§6.3), as for anyone |
| One address per session | nothing to spend; earnings stranded: moving them out is a transfer, a transfer is a public link |

So the unit that works is **one address per anonymous key**, kept as long as that key: a
pseudonym with a public history, not a temporary identity.

**What it gives away.** Ledger shows which credit addresses the pseudonym traded with, how much,
which day; non-hiding peers can be mapped to hosts (§9). So an observer learns which hosts a hidden node
exchanged data with, not who it is nor which files; adds to what long observation shows anyway
([`hops.md`](hops.md) §6). A node for which that is too much leaves
credits off under its anonymous key.

## 10. Risks that are not technical

- **Credits have a price.** A credit buys E3S at the rate of §6.7, and E3S trades on exchanges. So
  uploading earns something priced, and farming receipts pays. Decay and transfer burn make a
  credit poor to hold; they do not stop it being burned for a token that keeps its value.
- **The operators set the price of conversion** (§6.7): two thirds decide how much E3S a credit
  buys. They also run chain and bridge.
- Whether credits and E3S are crypto-assets under a country's law, what payment in them means for
  uploading users, and what setting the rate and running the bridge means for operators **was
  not checked by a lawyer**. An implementation MAY say so where the feature is switched
  on.
- The chain's native coin is transferable like any EVM chain's; no use but gas at 1 wei.

## 11. Cost

| | Cost |
|---|---|
| Receipt on the wire | about 80 bytes per 4 MiB, 0.002 % |
| Signing, verifying | one secp256k1 operation per tranche and side |
| RPC reads of an uploader | two calls per bidder per 10 min |
| Transactions of an uploader | one `settle` per 48 h with that period's receipts; one more for receipts with a payment, before their hold runs out; a `holdFor` per signed hold it relays |
| Transactions of a bidder | one `hold` per uploader bid at, every 3 h while bidding; none if its holds are relayed |
| Ledger state | one entry per settled (issuer, holder, day), one per (issuer, week) and one per (issuer, holder, week) that minted, one per (payer, payee) that placed a hold and one per (payer, payee) that used `holdFor`, kept for good; one per burn until claimed |

Gas **measured** 2026-10-18: solc 0.8.37, optimizer on (200 runs), EVM cancun, one transaction
per call, receipt `gasUsed` (21,000 and calldata included, after refunds), with
[`tests/contracts/gas/measure.py`](../tests/contracts/gas/measure.py). Native minter was a stub,
so on a real chain a reward-paying call adds the minter's own cost: about 30,000 per
`mintNativeCoin`, two per reward unless caller is the holder. Where the precompile refuses
(ledger not allowed to mint), each such call burns its 60,000 gas cap. Chain size after a year
**not measured**.

| Call | Gas |
|---|---|
| `settle`, first receipt of a pair and day, two new accounts | 244,000 |
| `settle`, first receipt of a pair and day, accounts and weeks known | 113,800 |
| `settle`, first of a pair and week / of a pair and issuer week | 130,900 / 148,000 |
| `settle`, later receipt of the same pair and day (no reward) | 91,900 |
| `settle`, each further later receipt in the same call: same pair and day / another holder | 30,000 / 53,200 |
| `settle`, each further first receipt of a pair and day in the same call, another holder (with its reward) | 72,700 |
| `settle`, later receipt that takes / that also uses a hold up | 81,400 / 87,900 |
| `hold`, first for a payee / for a further payee | 75,900 / 58,900 |
| `hold`, renewal; one that writes back a step of decay | 36,500–38,900; 52,500 |
| `holdFor`, first for (payer, payee) / later | 105,500 / 48,700 |
| `release` / `sweep` | 29,900 / 28,300 |
| `transfer` to an account / to a new address | 54,900 / 92,200 |
| `transfer`, decay written back on one side / both | 63,300 / 71,000 |
| `transferFrom`, finite / unlimited allowance | 60,800 / 57,600 |
| `poke` | 29,900–43,800 |

Stake token (§6.7), measured the same way 2026-10-27 (Foundry 1.8.6):

| Call | Gas |
|---|---|
| `burnForStake` | 96,500 |
| `claimStake`, first E3S of the address / to a holder of E3S | 73,800 / 39,600 |
| `setStakeParams`, through the operator contract | 31,900 |
| E3S `transfer` to a new address / to a holder | 51,200 / 34,100 |

No hold call earns a reward (§6.6). A bidder pays holds with coin earned settling receipts as
holder; a never-uploading node has none and has the uploader send its holds (§4.3), paying gas
from what it earns from the turn. The §6.6 reward is set from the second
`settle` row above, plus the minter's cost.

## 12. Alternatives considered

Choice of chain, key, gas and transfers: [`docs/credit-ledger.md`](../docs/credit-ledger.md). On
the wire:

| Alternative | Pro | Con |
|---|---|---|
| **One receipt for delivery and payment** (chosen) | one signature, one state per pair; uploader cannot take the mint and leave out the burn | a payment is only settled with a receipt |
| Two objects, receipt and payment voucher | each settleable alone | uploader gains nothing settling a payment, so a modified one never does |
| All of a payment to the uploader | selling priority earns credits | upload capacity sold, not shared; uploader gains from every false bid |
| All of a payment burned | uploader gains nothing from a false bid | no reason to settle a payment whose receipt mints nothing |
| **A tenth of a payment to the uploader** (chosen) | a reason to settle and to sell | small gain from false bids ([`docs/credit-ledger.md`](../docs/credit-ledger.md) §9) |
| Bid after seeing `clearing` | user sees the price first | second round on a stream the uploader may have closed (`Protocol.md` §12.5); with a second-price rule changes nothing |
| A running turn is never ended | no half-sent tranche, no paid-ahead loss | best bidder waits up to 1 h for a slot it outbid |
| A bid ends priority turns only | a running normal-queue turn is safe | a bid waits up to 1 h while the lane has room |
| **A bid ends any turn with more than 60 s left** (chosen) | a bid acts within a minute | a downloader without credits can lose its turn after a long wait; an ended bidder can lose a paid-ahead tranche |
| Credit key signed by the node key | third parties can check the tie | nobody needs to: session is authenticated; and the tie would be a public proof |
| Relay over HTTPS to operators | no message type | a second service and its addresses; operators become a gate |

On the bounds against keys signing for each other (§6.3):

| Alternative | Pro | Con |
|---|---|---|
| **A limit per issuer and holder** (chosen, 10 GiB per 7 days) | a key cannot give its week to one friend; receipts must spread | a real download above the limit from one uploader mints only the limit; one more stored value per pair and week |
| Sponsoring cap: an issuer counts towards the maturity of at most 3 new accounts per 30 days | a colluding group at most doubles per month | the honest network cannot grow faster either; one more counter per account |
| Lower weekly limit, dearer maturity (25 GiB; 4 GiB from 5 issuers) | each colluding key worth less, costs more, no new mechanism | heavy downloaders mint less for their uploaders; newcomers mature slower |
| Weekly limit following what the issuer itself earned | a group uploading to nobody outside mints a fifth | uploader earns less from pure downloaders; most complex in the contract |

On the stake token (§6.7):

| Alternative | Pro | Con |
|---|---|---|
| **Rate and delay fixed at the burn** (chosen) | burner knows what it gets | operators cannot cut a waiting claim when they find farming |
| Rate read at the claim | operators can react during the delay | burner does not know what it gets |
| **Set by the operator contract** (chosen) | no new key; same vote that admits validators | two thirds of operators set the price of conversion |
| Fixed at deployment | nobody can change them | a wrong rate stays for good; contract not replaceable without a new ledger |
| An admin key | simplest | a project key after genesis, which [`docs/credit-ledger.md`](../docs/credit-ledger.md) §3 avoids |
| **Bridge to the C-Chain** (chosen) | existing exchanges and liquidity | a bridge to deploy and trust |
| An exchange on the L1, deployed by the operators | no bridge | liquidity apart from every other market; operators run it |

## 13. Open points

- **A group of colluding keys grows without a bound.** Three mature keys can sign the 1 GiB
  contribution for any number of new keys, each minting up to 100 GiB a week after 30 days. Pair
  limit slows (a new key needs 10 GiB a week from each of 10 others to use its week), does not
  stop it. A sponsoring cap (§12) would; not taken.
- **Every number of §6.3** needs simulation against a Sybil model: age, contribution, weekly
  limit, pair limit. So does the gas reward of §6.6, which must not pay for filling blocks.
- **Bootstrap.** Initially only operators are mature; maturity spreads only as fast as operators
  download from new nodes. A rule for the first months may be needed.
- **`overdrawn` for good** is harsh for a client bug. A way back, at a price, not designed.
- **An expired hold still shields its amount** from payments to other holders until swept
  (§6.5). Anyone can sweep it, the holder awaiting payment first of all.
- **A hold earns no reward,** so a coinless bidder depends on the uploader to send its holds,
  and an uploader pays gas for bids that never win.
- **Registration is free.** A receipt with `kib = 0`, valid signature, day in the window registers
  both addresses (§6.1) before minting or taking anything, so the 30 days of §6.3 can be pre-aged
  for any number of addresses for one `settle`; contribution and the three issuers are the only
  price.
- **State growth.** Settled entries are never deleted. Whether entries older than 7 days can be
  dropped by a later call is open.
- **The validator manager** contract and operator vote are named, not specified. Ledger has tests
  ([`tests/contracts`](../tests/contracts/README.md)), no external audit.
- **Two thirds for bidders, and bid-endable turns,** leave a node without credits one slot in
  three at a selling uploader, and no certainty of finishing a turn. Whether the network stays
  usable for newcomers at that share was not modelled.
- **RPC endpoints** are found only via configuration. An operator-run index node
  (`Protocol.md` §11) could name one.
- **Checking a remote node** without trusting it, via validators' signatures, not looked at.
- **eD2K transfers** earn nothing: eD2K has no receipt.
- **Staking E3S** not specified (§6.7): who may stake, for what (validator seats, §7.2), relation
  to the operator vote. May come later as its own contract.
- **The bridge** (ICTT home contract on the L1, remote token on the C-Chain) is named, not
  specified, nor who deploys it and pays its relayers.
- **Credits have a price now** (§10). Every §6.3 number guards a traded token, and the unbounded
  growth of colluding keys (above) is paid. Simulation more urgent.
- **The rate.** How operators set it, from what data, how often: not designed. 1:1 and 30 days
  are initial.
