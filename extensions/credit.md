# Extension: credits (`credit`)

**Status:** draft 0.2 (2026-10-12). Not implemented. Optional in wire version 1. **The ledger contract ([`contracts/CreditLedger.sol`](../contracts/CreditLedger.sol)) compiles and passed a smoke test; it is not audited. Its numbers were not simulated against an attack model, and no chain has been deployed.**
**Extends:** [`Protocol.md`](../Protocol.md) §2, §5.1, §12.1 and §12.5. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 16, `CREDIT`. Message types 35 to 37. One field in `FileRequest`, one in `FileStatus`.
**Needs:** JSON-RPC access to a node of the credit ledger (§7). The ledger is a chain of its own, outside ed3k.

In ed3k, credits are local policy (`Protocol.md` §12.1): an uploader remembers who uploaded to
it, and nobody else can use that. This extension adds credits that count at **every** uploader.

- A downloader signs a **receipt** for what it received. The uploader hands receipts to a
  **ledger**, which turns them into credits: 1 credit for 1 MiB.
- A downloader can **spend** credits to get an upload slot ahead of the queue. It names a
  price per MiB; the highest bid gets the slot and pays just above the next bid.
- The ledger is a contract on an EVM chain, run as an **Avalanche L1** by a set of operators.
  It holds the balances and applies the rules, so that no client can skip them.
- Credits **are not money**. Nothing here sells or buys them. They are transferable, lose
  10 % a day when their holder stops uploading, and lose 10 % on every transfer.

Why a chain, why this one, who has to run a node, and what was rejected is in
[`docs/credit-ledger.md`](../docs/credit-ledger.md).

**A node without this extension is still served,** from the normal queue. But up to two
thirds of an uploader's slots can go to bidders (§5.1), and a bid can end its turn early
(§5.3), so it waits longer at uploaders that sell priority.

## 1. Overview

```
 Downloader D                          Uploader U                      Ledger (own chain)
 ────────────                          ──────────                      ──────────────────
 FileRequest{credit: payer, bid} ───▶
            ◀─── FileStatus{rank, credit: payee, floor, clearing}
                                       bid wins a priority slot:
                                       eth_call balanceOf(payer) ────▶
            ◀─── FileStatus{rank = 0, credit: price}
 Receipt{kib, spent = 1 tranche} ───▶  verify signature
            ◀═══ 4 MiB of blocks ═══
 Receipt{kib = 4 MiB, spent = 2 tr.} ▶
            ◀═══ 4 MiB of blocks ═══
            …                          within 48 h:
                                       settle(receipts) ─────────────▶ burn `spent` at D
                                                                        mint `kib` to U
```

A transfer without a bid looks the same with `spent` = 0: the downloader signs what it
received, and the uploader earns credits for it.

| Term | Meaning |
|---|---|
| **credit address** | 20 bytes, the address of a secp256k1 key as in Ethereum. A node has one, apart from its node key (§2) |
| **issuer** | the downloader that signs a receipt |
| **holder** | the uploader a receipt is signed for |
| **µ** | 10⁻⁶ credit, the unit of every amount on the wire and in the ledger |
| **tranche** | each 4 MiB of verified file data of a turn, counted from the start of the turn (the size of a piece, `Protocol.md` §7.2, not aligned to one); the last tranche of a turn can be shorter |
| **day** | `floor(unix_time / 86400)` |
| **mature** | an account whose receipts mint credits (§6.3) |

## 2. The credit key

The node key of `Protocol.md` §6.1 is Ed25519. The EVM verifies secp256k1 signatures for about
3,000 gas and has no Ed25519 function. So a node that takes part has a second key.

- The **credit key** is a secp256k1 key. Its address is the last 20 bytes of the Keccak-256
  hash of the uncompressed public key without its `0x04` prefix (64 bytes), as in Ethereum.
- It signs receipts (§3) and the node's transactions on the ledger (§7).
- **There is no signature that ties it to the node ID.** A node states its credit address on a
  file stream. The connection is authenticated (`Protocol.md` §6.2), so the statement comes
  from that node, and the first receipt proves that the downloader holds the key. A node that
  names an address it does not hold gains nothing: as uploader it gives its credits away, as
  downloader its receipts fail.
- The ledger never sees a node ID (§9).
- A node MAY change its credit address. The new one starts without balance and immature
  (§6.3); credits can be moved with a transfer, at its price (§6.5).
- The key file is as private as the node key. A client MUST NOT write the private key to a log.

## 3. Receipts

A receipt is the only signed object of this extension. It carries both what was delivered and
what was spent.

```
Receipt(address issuer, address holder, uint32 day, uint64 kib, uint64 spent)
```

| Field | Meaning |
|---|---|
| `issuer` | credit address of the downloader |
| `holder` | credit address of the uploader |
| `day` | the day of the signer's clock; the holder accepts today, yesterday and tomorrow by its own clock (§4.2) |
| `kib` | KiB of file data the issuer received from the holder **on that day**, verified, over all files; cumulative |
| `spent` | µ the issuer spent at the holder **on that day**; cumulative |

- Both counters only grow within a day and start at 0 on a new day.
- The signature is an EIP-712 signature by the issuer's credit key, with the domain
  `{name: "ed3k credit", version: "1", chainId, verifyingContract}` of the ledger (§7.1). It
  travels as 65 bytes, `r ‖ s ‖ v`, with `s` in the lower half and `v` 27 or 28.
- The holder keeps the **latest** receipt per issuer and day. An older one is worth nothing
  once a newer one is settled.
- The issuer keeps its two counters per holder and day, and MUST NOT lose them within the day.
  If it does, the holder returns the last receipt (§4.1).
- `kib` counts bytes of blocks that passed verification (`Protocol.md` §7.2), before
  compression. 1 KiB is 1024 bytes; a rest below 1 KiB is carried to the next receipt.

**Value.** A receipt mints `floor(kib × 15625 / 16)` µ, which is 1 credit per MiB. The ledger
computes this on the cumulative value, so rounding does not add up.

### 3.1 When a downloader signs

A downloader with `CREDIT` signs for an uploader that named a `payee` (§4.1):

- after every **tranche** it has verified, and
- when its turn ends (`Protocol.md` §12.5), for the rest.

It sends the receipt on the file stream as message type 35 (§4.2). Signing costs it nothing.

An uploader MAY rank a downloader that does not sign lower in its queue; that is local policy,
like the credits of `Protocol.md` §12.1. It MUST still serve nodes without `CREDIT`.

### 3.2 When a receipt carries a payment

In a priority slot (§5) the downloader pays **one tranche ahead**:

```
due = Σ ceil(price of each tranche delivered in this turn × its bytes / 2²⁰)
    + ceil(price now × 4 MiB / 2²⁰)                        prices in µ per MiB, as in §5.2
spent ≥ spent at the start of the turn + due
```

- Amounts are whole µ, rounded up per tranche, so both sides compute the same `due`.
- The counters start at 0 on a new day (§3), but what was paid stays paid: the uploader carries
  the byte mark up to which the turn is paid across the day boundary, and the new day's `spent`
  covers the tranches from the next unpaid one on. For the first receipt after a day boundary
  inside a turn, "spent at the start of the turn" reads as 0 and the sum runs from the first
  tranche after the boundary, so both sides compute the same `due`.

- The first receipt of a priority turn carries the payment for the first tranche. The uploader
  serves `BlockRequest`s only up to the end of the tranche that is paid.
- Each further receipt adds the tranche just received to `kib` and the next tranche to `spent`.
- The downloader stops paying when it stops receiving. Its loss is at most one tranche.
- A payment that was not used up, because the turn ended inside a tranche or was ended by a
  higher bid (§5.3), counts towards the next priority turn at the same uploader on the same
  day.
- A downloader MUST NOT sign a `spent` above what is due.

## 4. Wire

### 4.1 Fields on the file stream

```protobuf
// in FileRequest (Protocol.md §12.1):
  CreditBid credit = 9;

// in FileStatus:
  CreditOffer credit = 11;

message CreditBid {
  bytes  payer = 1;           // 20 B, my credit address; in the first FileRequest of the stream
  uint64 max_price = 2;       // µ per MiB I pay at most for a priority slot; 0 = no bid
}

message CreditOffer {
  bytes  payee = 1;           // 20 B, the uploader's credit address; sent once per stream
  optional uint64 floor = 2;  // µ per MiB, lowest bid taken; absent = no priority slots
  uint64 clearing = 3;        // µ per MiB a bid has to reach to get the next priority slot
  uint32 used = 4;            // priority slots in use
  uint32 slots = 5;           // priority slots at most (§5.1)
  optional uint64 price = 6;  // µ per MiB charged from the next tranche on; with rank = 0 only; absent = unchanged
  Receipt last = 7;           // the latest receipt of today I hold from `payer`, if any
}
```

- A downloader sends `credit` only to a peer with `CREDIT`, and only if it is willing to sign
  receipts. `payer` alone, without a bid, says so.
- An uploader answers with `credit` only if the `FileRequest` carried one. `payee` alone,
  without `floor`, says: I take receipts and sell no priority.
- **The bid is given once, blind, in the first `FileRequest`.** The price charged is set by
  the next bid, not by the own one (§5.2), so a downloader names the most it would pay and
  need not see `clearing` first. `clearing` is information for the user.
- A bid stays with the queue entry (`Protocol.md` §12.5). A downloader changes it with a new
  `FileRequest`, at most once per 30 min *(initial)*; this counts as an early return of
  `Protocol.md` §12.5.
- `last` lets a downloader that lost its counters go on. It MUST verify that the signature is
  its own before it takes the values. It continues from the higher of its own and these.

Both ledger parameters, chain ID and contract address, are fixed for a network (§7.1) and are
not sent.

### 4.2 `Receipt`

```protobuf
message Receipt {             // type 35, downloader → uploader, on the file stream
  uint32 day = 1;
  uint64 kib = 2;
  uint64 spent = 3;
  bytes  sig = 4;             // 65 B
}
```

`issuer` and `holder` are `payer` and `payee` of the stream. The uploader verifies the
signature and that

- `day` is today, yesterday or tomorrow by its own clock,
- `kib` is not above what it delivered to this node on that day, and not below the receipt
  before,
- `spent` is not below the receipt before, and covers what is due (§3.2) in a priority turn.

A receipt that fails is dropped. A receipt is not answered.

### 4.3 `CreditRelay`

A node needs a little native coin of the ledger chain to send a transaction (§7.3). A node
that has none asks a peer to send its receipts for it.

```protobuf
message CreditRelay {         // type 36, request
  repeated SignedReceipt receipts = 1;   // at most 32 (initial)
}

message SignedReceipt {
  bytes  issuer = 1;          // 20 B
  bytes  holder = 2;          // 20 B
  uint32 day = 3;
  uint64 kib = 4;
  uint64 spent = 5;
  bytes  sig = 6;
}

message CreditRelayAnswer {   // type 37, answer
  bytes  tx = 1;              // 32 B, hash of the transaction that was sent
}
```

- Any node with `CREDIT` MAY relay. It answers `Error{BUSY}` if it does not, and
  `Error{MALFORMED}` if a receipt does not verify.
- The relay calls `settle` (§6.2) from its own account. The receipts say who is credited; the
  relay cannot change that. It is paid by the gas reward of §6.6.
- A relay SHOULD simulate the call first (`eth_call`) and send it only if the reward covers
  the gas.
- The asker checks the outcome itself, by transaction hash or by `settled()`.
- Part of each reward goes to the holder of the receipt, so a node needs a relay for its first
  settlements only.

## 5. Priority slots

### 5.1 The lane

An uploader with a `floor` gives some of its upload slots to bidders.

- **At most two thirds** of its slots, rounded down, and 1 if it has 2. At least one slot
  always serves the normal queue. An uploader SHOULD NOT exceed this.
- A priority slot that no bid asks for serves the normal queue. A bid that comes later can
  end the turn in it (§5.3).
- A priority slot is a **turn** as any other: 16 MiB or 1 h (`Protocol.md` §12.5).
- After 16 MiB the uploader MAY go on while no other bid waits and the downloader keeps
  paying.
- A bid does not move the entry in the normal queue, forwards or backwards. A downloader
  whose bid never wins is served as if it had not bid.
- The `floor` SHOULD be at least 1,000,000 µ per MiB (1 credit) *(initial)*.

### 5.2 Who gets the slot, and at what price

When a priority slot is free, the uploader takes the waiting entry with the highest
`max_price` that is at least `floor` and whose payer is **eligible**:

- `balanceOf(payer)` on the ledger covers a whole turn at its bid,
- the account is not `overdrawn` (§6.4),
- both read through the RPC of §7 no more than 10 min *(initial)* ago.

Equal bids go by queue position. The slot is offered as any slot (`rank` = 0 on the open
stream, or `SlotOffer`), with `price` set:

```
price = min( own max_price,  max( floor,  highest other waiting eligible bid + tick ) )
tick  = 10,000 µ per MiB (initial)
```

- This is a second-price rule: the winner pays just above the next bidder, not its own
  maximum. Naming the true maximum is then the best bid.
- **The price holds for a tranche.** When the waiting bids change, the uploader computes it
  again and sends it in a `FileStatus`; it applies from the next tranche the downloader pays
  for. It is never above `max_price`.
- The downloader sends the first receipt (§3.2) within 30 s *(initial)*. If it does not, or a
  later payment is 30 s late, the uploader takes the slot away. The entry stays in the normal
  queue.
- `clearing` in a `FileStatus` is the price a new bid would have to exceed now.

### 5.3 A bid ends a running turn

When an eligible bid waits and no slot is free, the uploader picks one running turn:

| Bidders hold | Turn it picks |
|---|---|
| fewer slots than the lane has (§5.1) | a turn **from the normal queue**: of those with more than 60 s left, the one that has received the most of its 16 MiB |
| every slot of the lane | the priority turn with the lowest `max_price`, if the bid is above that `max_price` |

A bid never takes the lane above two thirds, so at least one normal turn always runs on.
For the turn it picked, the uploader computes

```
remaining = bytes left of that turn / its rate over the last 10 s (initial)
```

| `remaining` | The uploader |
|---|---|
| more than 60 s *(initial)* | ends the turn after the block it is sending, and gives the slot to the bidder |
| 60 s or less | lets the turn run out; the bidder gets the slot then |

- Bytes left are the rest of the 16 MiB, or the rest of what the downloader still needs from
  this uploader, whichever is less. A turn that sends nothing counts as more than 60 s.
- If every normal turn has 60 s or less left, the bidder gets the first of them that ends.
- A bid that is above the price of a running priority turn but not above its `max_price` ends
  nothing. It raises the price (§5.2).
- **A bidder whose turn was ended** keeps its entry in the normal queue and its bid. What it
  paid ahead counts for its next priority turn that day (§3.2); if it gets none, it has lost
  up to one tranche.
- **A downloader from the normal queue whose turn was ended** keeps its entry, and the
  uploader SHOULD give it the next slot that becomes free for the normal queue, before anyone
  else. The bytes of its turn start again at 0.
- The uploader tells it with a `FileStatus` that carries its `rank` and the new `clearing`.
- A downloader SHOULD write a block that arrived whole, and SHOULD ask another source for
  the rest of the tranche.

### 5.4 The downloader's side

- A client SHOULD bid per file, with a limit the user sets, and SHOULD offer a limit for
  credits spent per day.
- It SHOULD NOT bid at an uploader whose `floor` is above its limit.
- Its exposure is one tranche per uploader it holds a priority slot at.
- A downloader that signed more `spent` in total than it has is **overdrawn** once the
  receipts are settled (§6.4). A client MUST keep the sum of its open payments at or below
  80 % *(initial)* of its balance as last read, so that two days of decay (§6.5) and the 48 h
  a holder has to settle (§6.2) cannot leave it short.

## 6. The ledger

One contract holds every balance. It is immutable and has no owner. This section says what it
does; the source is [`contracts/CreditLedger.sol`](../contracts/CreditLedger.sol). Where the two
differ, that is a fault to be reported, and this text is meant.

### 6.1 Accounts

```solidity
struct Account {
    uint128 earned;         // µ, from receipts
    uint128 received;       // µ, from transfers
    uint32  lastActive;     // day of the newest receipt that counted as activity
    uint64  decayedUntil;   // decay is applied up to this time
    uint64  registeredAt;   // first appearance
    uint64  inflow;         // KiB received credit for, from mature issuers
    uint16  issuers;        // distinct mature issuers among them
    bool    overdrawn;
}
```

- `balanceOf` is `earned + received` after decay (§6.5).
- An account is **registered** when it first appears in a receipt with a valid signature, as
  issuer or as holder, whether or not the receipt mints.

### 6.2 Settlement

```solidity
function settle(Receipt[] calldata receipts, bytes[] calldata sigs) external;
```

Anyone may call it. For each receipt the contract recovers the signer and skips the receipt
unless

- the signer is `issuer`, and `issuer` is not `holder`,
- `day` is not in the future and not more than 60 days *(initial)* back,
- `kib` is at most 52,428,800 (50 GiB in a day) *(initial)*,
- `kib` and `spent` are not below what is settled for (issuer, holder, day).

A signature that does not verify recovers the zero address. That is why a zero `issuer` or
`holder` is skipped as well: a receipt with a zero issuer and a bad signature would otherwise
pass the first rule.

Then, in this order:

1. **Take.** The growth of `spent` is taken from the issuer, from `received` first, after its
   decay is applied. If the balance is short, what is there is taken and the account is set
   `overdrawn`. **10 % of what is taken goes to the uploader** and arrives in the holder's
   `received`; the rest is burned
   ([`docs/credit-ledger.md`](../docs/credit-ledger.md) §9).
2. **Mint.** If the issuer is mature (§6.3) and not overdrawn, the growth of `kib` is minted to
   the holder's `earned`, up to what is left of the issuer's weekly limit and of the limit of
   the pair (§6.3). What exceeds
   the limit is lost.
3. **Activity.** If at least 1 credit was minted and `day` is newer than the holder's
   `lastActive`, the holder's decay is first written up to the start of `day`, and then
   `lastActive` becomes `day`. So the days from `day` on, up to the grace, do not decay, and
   the time before does.
4. **Gas reward** (§6.6).

- If the issuer is **not mature**, step 1 is done and step 2 is not. The settled `kib` stays
  where it was, so the same receipt can be settled again when the issuer has matured, within
  the 60 days.
- A holder SHOULD settle **at least every 48 h** while it has new receipts, at a random time
  and not right after a transfer (§9). Decay that was already written is not undone by a late
  receipt.

### 6.3 Maturity and limits

Signing a receipt is free, so who may mint is the whole question. A receipt mints only if its
issuer is **mature**:

| | Rule *(initial, not simulated)* |
|---|---|
| Age | registered for at least 30 days |
| Contribution | credit minted to it for at least 1 GiB, from at least 3 distinct mature issuers |
| Roots | the credit addresses of the operators (§7.2) are mature from the start, one per operator |

| Limit | Value *(initial, not simulated)* |
|---|---|
| Minted from one issuer, over all holders, per 7 days | 100 GiB |
| Minted from one issuer to one holder, per 7 days | 10 GiB |
| `kib` of one receipt | 50 GiB |
| Age of a receipt at settlement | 60 days |

- A new node downloads, earns and spends from the first day. Only the receipts it *signs* do
  not mint yet. The uploaders it took from are credited later, if it matures within 60 days.
- To make one more key an accepted issuer, an attacker has to upload 1 GiB to three accepted
  issuers through it and wait 30 days. What that key can then mint is bounded per week.
- **The limit of a pair** keeps one issuer from pouring its week onto one holder. To use its
  100 GiB a key has to sign for 10 holders, so a mature user that signs for a friend gives
  it 10 GiB a week and no more. A real download of more than 10 GiB from one uploader in a
  week mints 10 GiB; the rest is lost to the uploader.
- **This is a bound, not a proof.** A group of keys can mint to each other up to the weekly
  limit of each mature key, when the group has at least 11 of them. The rules make that cost
  real upload first.
- **What the limits do not bound** is how fast such a group grows: three mature keys can
  sign the contribution of any number of new keys (§13).

### 6.4 Overdrawn

An account that spent more than it had is flagged for good. An uploader does not give it a
priority slot (§5.2), its receipts mint nothing, and it cannot transfer. A node in that state
starts a new credit address, immature and empty.

This bounds what a downloader can take by signing payments to many uploaders at once: one
tranche at each, once per address.

### 6.5 Decay and transfers

**Decay.** Nothing runs on a timer; the balance is computed when it is read and written back
at the next change.

| | Value |
|---|---|
| Grace | 3 days after `lastActive`, no decay |
| Then | × 0.9 per full 24 h, on the current balance |
| `earned` | stops at the first step that takes it below 10 credits |
| `received` | goes on; below 1 credit it becomes 0 |
| Activity | a settled receipt that minted at least 1 credit, by its `day`. Nothing else |

An account that stops uploading keeps about 31 % after two weeks and ends just below 10
credits. `poke(address)` writes the decay of an account back and can be called by anyone.

**Transfers.** The contract has the ERC-20 interface (`name`, `symbol`, `decimals` = 6,
`balanceOf`, `transfer`, `transferFrom`, `approve`, `allowance`, `totalSupply`) and `permit`
of EIP-2612.

- **10 % of every transfer is burned** *(initial)*. The recipient gets the rest.
- A transfer takes from `received` first, then from `earned`, and always arrives in
  `received`. So a balance cannot be kept under the floor of 10 by splitting it over many
  addresses.
- A transfer to one's own address burns 10 % like any other.
- Receiving a transfer is **never** activity and never counts for maturity.
- Every address decays, contracts too. An exchange pool is an inactive account.
- `balanceOf` falls with time without an event. `totalSupply` is the supply as last written,
  an upper bound.
- A transfer costs gas and earns no reward (§6.6).

### 6.6 Gas reward

The chain charges gas at a fixed, very low price (§7.2), in a native coin that is otherwise of
no use. The contract mints that coin for useful work, through the native minter of the chain:

- for each receipt in a `settle` that minted or took at least 1 credit, an amount fixed at
  deployment *(initial: twice the gas such a receipt costs, §11)*;
- 60 % to the caller, 40 % to the holder of the receipt; all of it to the holder if the holder
  is the caller.

A receipt that changes less, a failed call and a transfer earn nothing and cost gas. A node
that settles its own receipts thus gains a little coin with each, and one that sends other
transactions uses it up.

## 7. The chain and the RPC

### 7.1 Parameters of a network

| Parameter | Public network |
|---|---|
| Chain ID | *to be assigned at deployment* |
| Ledger contract address | *to be assigned at deployment* |
| RPC endpoints | a list in the client's configuration; none is built into the protocol |

A network with a name (`Protocol.md` §4.3) has its own ledger or none. A node of such a
network MUST NOT use the ledger of the public network. Until both parameters are assigned no
receipt can be signed or verified, and the test vectors of this extension wait on that.

### 7.2 The chain

- An **Avalanche L1** that runs the EVM (Subnet-EVM), with its own validators.
- **Validators are operators**, admitted by vote: an operator contract owns the validator
  manager of the L1, and two thirds of the operators add or remove one. There is no stake and
  no project key after genesis. An operator runs a validator and SHOULD run a public RPC
  endpoint.
- **Gas** has a static price of 1 wei, the lowest the chain accepts.
- **Only the operator contract may deploy contracts**, so the chain carries the ledger and
  nothing else.
- The ledger contract is the one account allowed to mint the native coin (§6.6).
- A block that is accepted is final. A client reads at the tag `latest`.

What the operators can and cannot do is in
[`docs/credit-ledger.md`](../docs/credit-ledger.md) §3.

### 7.3 What the ed3k node does over RPC

The ledger node is a separate program. The ed3k node is its client, over the Ethereum JSON-RPC
of the chain (`https://<host>/ext/bc/<blockchain ID>/rpc`).

| Use | Method |
|---|---|
| Is this the right chain | `eth_chainId` |
| Balance, `overdrawn`, maturity, what is settled, own coin | `eth_call`, `eth_getBalance` |
| Send `settle`, `transfer`, `poke` | `eth_getTransactionCount`, `eth_estimateGas`, `eth_sendRawTransaction` |
| Outcome | `eth_getTransactionReceipt` |

- Transactions are signed by the credit key. The account of the credit address holds the coin.
- **A local node** needs nothing more. It follows the L1 without being a validator.
- **A remote node** is believed for what it says. A client that uses remote nodes SHOULD ask
  two of different operators for a read it acts on, and act only if they agree.
- What a lying node can do is bounded: it can make an uploader give one priority turn for
  nothing, or make a node pay gas for a `settle` that mints nothing. It cannot move or mint
  credits; that takes signatures.
- A remote node sees the address of the client and every credit address it asks about. A
  client SHOULD send RPC through the proxy the user configured, if any.
- A node that reaches no RPC endpoint goes on signing and collecting receipts, sells no
  priority, and settles later.

## 8. What a modified client can do

| Cheat | Effect | Bound |
|---|---|---|
| Uploader takes the first payment and does not serve | the downloader loses one tranche | 4 MiB at the price; the downloader does not bid there again |
| Uploader lies about `clearing` or the queue | none on the price | as above |
| Uploader does not settle payments | a downloader keeps credits it spent | the uploader loses the mint of the same receipt: both are one object |
| Downloader does not sign | the uploader earns no credits | one tranche; local ranking (§3.1) |
| Downloader stops paying in a turn | none | payment is one tranche ahead |
| Downloader signs payments to many uploaders | each gives up to a tranche for nothing | `overdrawn`, for good (§6.4) |
| Uploader sends data that fails verification | none: only verified blocks count in `kib` | it is banned by the downloader after 2 blocks (`Protocol.md` §12.3) |
| Two keys of one person sign for each other | credits without upload | maturity, 10 GiB per pair and week, and the weekly limit (§6.3) |
| A mature user signs large receipts for friends | the same | 10 GiB per friend and week |
| A group of mature keys makes new keys mature | more keys that mint, each up to the weekly limit | 30 days and 1 GiB of the group's limits per key; **not bounded in number** (§13) |
| Uploader charges more to earn its 10 % | the downloader pays up to its maximum | never above `max_price`; the gain is a tenth of the difference |
| A bidder's turn is ended by a higher bid | it loses what it paid ahead, if it gets no other turn that day | one tranche |
| Farming credits to sell them | credits get a price (§10) | decay, the 10 % on transfer, the limits |
| A relay drops a request | delay | the asker asks another |
| An RPC node lies | §7.3 | one turn, or gas |

## 9. What others learn

**The ledger is public and stays so.** For every pair of credit addresses it shows how much
one downloaded from the other, on which day, and what was paid. It shows no file, no address
of a host and no node ID.

**A credit address is not secret from peers.** A node tells it to every peer with `CREDIT`
that opens a file stream to it. Whoever does that with many nodes can map node IDs and host
addresses to credit addresses, and then read each node's whole trade history from the ledger.
This is the price of a public ledger, and it is the main reason to leave the extension off.

- A client MUST leave the extension **off by default** and MUST tell the user the above when
  it is switched on.
- A node MUST NOT send its credit address in a session that runs through an anonymous path
  ([`hops.md`](hops.md)). §9.1 says what it may use there.
- A new credit address breaks the link to the past, and starts immature.
- Settling at a random time within 48 h keeps single transfers from showing in the order of
  blocks.
- A remote RPC node learns more than the ledger shows (§7.3).

### 9.1 Through an anonymous path

A node that hides acts under its anonymous key ([`hops.md`](hops.md) §1). It MAY use credits
there, with a credit address of its own:

- The **anonymous credit address** belongs to the anonymous key. It MUST NOT be used outside
  a path, and the node's other credit address MUST NOT be used inside one.
- It is replaced when the anonymous key is replaced, and not more often (see the table).
- **No transfer between the two addresses,** in either direction and through no third address
  the node controls. The ledger shows every transfer for good. A client MUST NOT make one on
  its own and MUST warn a user who does.
- **No direct RPC.** A transaction goes out by `CreditRelay` (§4.3) to a peer of a session.
  A read comes from a ledger node on the same machine, or through a proxy the user set up
  for that purpose, or not at all. A node that cannot read sells no priority; it can still
  sign, earn and bid.
- Gas coin comes from the holder's part of the reward (§6.6). It MUST NOT be sent from the
  other address.

**A temporary address** is what a hidden node starts with, and what it can do is limited by
the rules that keep credits honest:

| | With a new address |
|---|---|
| Signing receipts | works, and mints nothing: the address is immature. Uploaders earn nothing from it |
| Earning | works at once, if the downloaders are mature |
| Bidding | only with what was earned under this address |
| Becoming mature | 30 days and 1 GiB to three mature issuers (§6.3), as for anyone |
| One address per session | has nothing to spend, and what it earns is stranded: moving it out is a transfer, and a transfer is a public link |

So the unit that works is **one address per anonymous key**, kept as long as that key. It is
a pseudonym with a public history, not a temporary identity.

**What it gives away.** The ledger shows with which credit addresses the pseudonym traded,
how much and on which day. Those peers that do not hide can be mapped to hosts (§9). An
observer thus learns which hosts a hidden node exchanged data with, though not who it is and
not which files. This adds to what long observation shows anyway ([`hops.md`](hops.md) §6). A
node for which that is too much leaves credits off under its anonymous key.

## 10. Risks that are not technical

- **Transferable credits can be sold.** Then uploading earns something with a price, and
  farming receipts is paid. The decay and the transfer burn make a credit a poor thing to
  hold; they do not make it worthless.
- Whether such credits are a crypto-asset under the law of a country, and what being paid in
  them means for a user who uploads, **was not checked by a lawyer**. An implementation SHOULD
  say so where the feature is switched on.
- The native coin of the chain is transferable like that of any EVM chain. It has no use but
  gas at 1 wei.

## 11. Cost

| | Cost |
|---|---|
| Receipt on the wire | about 80 bytes per 4 MiB, 0.002 % |
| Signing, verifying | one secp256k1 operation per tranche and side |
| RPC reads of an uploader | two calls per bidder per 10 min |
| Transactions of an uploader | one `settle` per 48 h, with the receipts of that time |
| Ledger state | one entry per (issuer, holder, day) that was settled and one per (issuer, holder, week) that minted, kept for good |

Gas, from one run on a test chain without the native minter: about 195,000 for the first
receipt of a pair and day with two new accounts, about 65,000 for a later one of the same pair
and day, plus 21,000 per transaction. These were measured before the limit of a pair was
added; that limit writes one more value per receipt that mints, so the numbers are now
higher, by about 22,000 for the first receipt of a pair and week by the gas schedule, **not
measured**. The size of the chain after a year was **not measured**.

## 12. Alternatives considered

The choice of chain, key, gas and transfers is argued in
[`docs/credit-ledger.md`](../docs/credit-ledger.md). On the wire:

| Alternative | Pro | Con |
|---|---|---|
| **One receipt for delivery and payment** (chosen) | one signature and one state per pair; an uploader cannot take the mint and leave out the burn | a payment is only settled with a receipt |
| Two objects, receipt and payment voucher | each can be settled alone | the uploader gains nothing by settling a payment, so a modified one never does |
| All of a payment goes to the uploader | selling priority earns credits | upload capacity is sold, not shared; the uploader gains from every false bid |
| All of a payment is burned | the uploader gains nothing from a false bid | no reason to settle a payment whose receipt mints nothing |
| **A tenth of a payment goes to the uploader** (chosen) | a reason to settle and to sell | a small gain from false bids ([`docs/credit-ledger.md`](../docs/credit-ledger.md) §9) |
| Bid after seeing `clearing` | the user sees the price first | a second round on a stream the uploader may have closed (`Protocol.md` §12.5); with a second-price rule it changes nothing |
| A running turn is never ended | no half-sent tranche, nothing paid ahead is lost | the best bidder waits up to 1 h for a slot it outbid |
| A bid ends priority turns only | a turn from the normal queue is safe once it runs | a bid waits up to 1 h while the lane has room |
| **A bid ends any turn that has more than 60 s left** (chosen) | a bid acts within a minute | a downloader without credits can lose its turn after a long wait; a bidder that is ended can lose a tranche paid ahead |
| The credit key signed by the node key | third parties can check the tie | nobody needs to: the session is authenticated; and the tie would be a public proof |
| Relay over HTTPS to operators | no message type | a second service and its addresses; operators become a gate |

On the bounds against keys that sign for each other (§6.3):

| Alternative | Pro | Con |
|---|---|---|
| **A limit per issuer and holder** (chosen, 10 GiB per 7 days) | a key cannot give its week to one friend; receipts have to spread | a real download above the limit from one uploader mints only the limit; one more stored value per pair and week |
| A cap on sponsoring: an issuer counts towards the maturity of at most 3 new accounts per 30 days | a group of colluding keys can at most double per month | the honest network cannot grow faster either; one more counter per account |
| A lower weekly limit and a dearer maturity (25 GiB; 4 GiB from 5 issuers) | each colluding key is worth less and costs more, with no new mechanism | heavy downloaders mint less for their uploaders; newcomers mature more slowly |
| A weekly limit that follows what the issuer earned itself | a group that uploads to nobody outside mints a fifth | an uploader earns less from pure downloaders; the most complex in the contract |

## 13. Open points

- **A group of colluding keys grows without a bound.** Three mature keys can sign the 1 GiB of
  contribution for any number of new keys, and each new key mints up to 100 GiB a week after
  30 days. The limit of a pair slows this (a new key needs 10 GiB a week from each of 10
  others to use its week) and does not stop it. A cap on sponsoring (§12) would; it was not
  taken.
- **Every number of §6.3** needs a simulation against a Sybil model: age, contribution, the
  weekly limit and the limit of a pair. So does the gas reward of §6.6, which must not pay for filling blocks.
- **Bootstrap.** At the start only the operators are mature. Maturity spreads only as fast as
  operators download from new nodes. A rule for the first months may be needed.
- **`overdrawn` for good** is harsh for a client bug. A way back, at a price, was not designed.
- **State growth.** Settled entries are never deleted. Whether entries older than 60 days can
  be dropped by a later call is open.
- **The validator manager** contract and the operator vote are named, not specified. The
  ledger has no test suite and no audit.
- **Two thirds for bidders, and turns that bids can end,** leave a node without credits one
  slot in three at an uploader that sells, and no certainty of finishing a turn. Whether the network stays usable for newcomers at that share was not modelled.
- **RPC endpoints** have no way to be found other than configuration. An index node
  (`Protocol.md` §11) run by an operator could name one.
- **Checking a remote node** without trusting it, by the validators' signatures, was not
  looked at.
- **eD2K transfers** earn nothing: there is no receipt in eD2K.
