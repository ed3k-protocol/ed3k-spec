# Extension: key puzzle (`key-puzzle`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Not part of wire version 1; reserved for a network upgrade.
**Extends:** [`Protocol.md`](../Protocol.md) §6.3. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 7, `PUZZLE`.

The routing ID of `Protocol.md` §6.3 limits *where* a node may sit, and the diversity limits
([`ip-diversity.md`](ip-diversity.md)) limit how many nodes of one network count. Neither puts a
price on a node ID itself: a key costs microseconds. This extension adds one. A `ROUTER` node
must show a solution to a memory-hard puzzle before other nodes treat it as a router.

It is a recurring price per identity, per network location and per month. It is a modest one
(§7), and it does not replace the other two measures.

## 1. Overview

- The puzzle is a **nonce search**. The key, and with it the node ID, never changes because of
  the puzzle.
- The function is **Argon2id** (RFC 9106) with 64 MiB and one pass. A try costs tens of
  milliseconds and cannot be made much cheaper with a GPU or custom hardware.
- A solution is bound to the **key**, the node's **site group** (IPv4 /24, IPv6 /48) and a
  30-day **epoch**. It is of no use at another address or two months later.
- Only **`ROUTER`** nodes need one. Leaves pay nothing.
- Finding a solution takes minutes. **Checking one costs a single try**: about 35–100 ms and
  64 MiB, once per node, site and epoch.

## 2. The puzzle

```
out = Argon2id(
        password = public_key(32) ‖ family(1) ‖ site(6) ‖ epoch(4) ‖ nonce(8),
        salt     = "ed3k v1 puzzle"                    (14 ASCII bytes)
        t = 1,  m = 65536 KiB,  p = 1,  tag length = 32,  version = 0x13 )

valid  ⇔  the first n bits of out are zero            n = 13 (initial)
```

The first n bits are counted from the most significant bit of `out[0]`: with n = 13, `out[0]`
is 0 and the top 5 bits of `out[1]` are 0.

| Field | Value |
|---|---|
| `public_key` | the node's Ed25519 public key (`Protocol.md` §6.1) |
| `family` | one byte: `0x04` for IPv4, `0x06` for IPv6 |
| `site` | IPv4: the first 3 bytes of the address, then 3 zero bytes. IPv6: the first 6 bytes |
| `epoch` | `floor(unix_time / 2 592 000)`, big-endian. One epoch is 30 days |
| `nonce` | any 8 bytes; the node searches this |

- A solution with more than `n` zero bits is valid. A node MAY keep the best one it has found.
- The site is the one of the **observed** address (`Protocol.md` §9.2), the same address the routing ID is
  checked against.
- A dual-stack router needs one solution per family.
- In a named network a router at a private address (`Protocol.md` §8.2 (8)) needs no
  solution, as it needs no routing-ID check there (`Protocol.md` §6.3); on the public network
  such addresses never become contacts.

### 2.1 Epochs

A verifier accepts a solution for the current epoch and for the previous one. In the last 24
hours of an epoch it also accepts the next one, to tolerate clock error.

A solution found at any time in epoch *e* therefore stays valid until the end of epoch *e* + 1,
between 30 and 60 days. A router solves for the new epoch in the background at some point
during it; there is no moment at which all routers must solve at once.

## 3. Carrying the solution

`Hello` (`Protocol.md` §5.5) gains one field:

```protobuf
  bytes puzzle = 9;           // epoch(4, big-endian) ‖ nonce(8), for the family of this connection
```

- A node that sets `ROUTER` and `PUZZLE` sends the solution for the site it believes the peer
  sees. It knows its public address from earlier `observed` reports.
- The field is 12 bytes. Compact contacts in `KadAnswer` do not carry solutions: a contact is
  verified by connecting to it (`Protocol.md` §6.3), and the `Hello` of that connection brings the solution.
- With the datagram binding ([`kad-datagram.md`](kad-datagram.md)) there is no `Hello`. The
  solution then follows the responder's public key in `CHALLENGE`, 12 bytes more; an `INIT` is
  at least 70 bytes so that this fits ([`kad-datagram.md`](kad-datagram.md) §2.1). The
  solution is the one for the family of the datagram, as `Hello` carries it for the family of
  its connection, and is sent only while the node is `ROUTER` and has a valid one.
- A `REQUEST` carries no solution. A node is therefore never puzzle-verified by an exchange in
  which it is the requester, only by one in which it is the responder, or by a `Hello`.

## 4. Verification

A router contact is **puzzle-verified** when its solution is valid for its node key, the site of
its observed address, and an accepted epoch. After the enforcement date (§6), a contact that is
not puzzle-verified is treated as a leaf: it is not inserted into a routing table, not handed
out in answers, not counted towards lookup termination and not used as a storage target. Its
requests are still served. It is not banned.

Verification costs one Argon2id run, so the rules keep that cost under the verifier's control:

- **Lazily.** A node verifies a solution only at the point where the contact would otherwise be
  inserted, counted or stored on. It does not verify on receipt of `Hello`.
- **Bounded for peers that dial in.** A peer that dials in chooses when it comes and which key
  it shows, so its runs are rationed. A contact that dialled in is verified only when it would
  be inserted as a dialled-in contact (`Protocol.md` §10.2), and stays out until then. A node
  verifies at most 1 *(initial)* dialled-in contact per host group
  ([`ip-diversity.md`](ip-diversity.md) §1) per hour, whatever the result. A failed result is
  kept as below.
- **Own lookups first.** Contacts that the node found by its own lookups are verified before
  any contact that dialled in. A dialled-in contact is checked only while none of the others
  waits.
- **One at a time.** A node runs at most one verification at once, so the extension costs 64 MiB
  of memory, not 64 MiB per peer. Pending verifications queue; a lookup does not wait for a
  contact whose check is still queued, it simply does not count that contact yet.
- **A capped queue.** The queue holds at most 64 *(initial)* contacts. When it is full, the
  newest one is dropped: it stays unverified and is queued again the next time it would be
  inserted, counted or stored on.
- **Cached.** The result is kept per (node ID, family, site, `puzzle` value) and SHOULD be
  written to disk with the routing table. A valid result is kept while its epoch is accepted
  (§2.1), so a known router costs nothing until then. A failed result is kept until the end of
  the current epoch, so the same solution is not checked twice. Another `puzzle` value of that
  node ID, family and site is checked again at most once per hour *(initial)*:
  a router that moved shows its old solution once before it learns its new address.
- **Datagram binding.** A node that only sent `REQUEST`s has shown no solution (§3). It is
  puzzle-verified only by an exchange in which it is the responder, or by a `Hello`.
- **One value per connection.** A connection carries the value of its `Hello` for its whole
  life. A contact whose value leaves the accepted epochs while its connection stays open is
  not puzzle-verified until its next connection; its requests are still served.
- **The newest value counts.** A contact whose new value fails is not puzzle-verified, also
  when an older value of the same site was valid.

A lookup that meets 10 to 20 routers for the first time costs roughly 0.4 to 2 seconds of one
core in total. A warm node pays close to nothing.

## 5. Generation

A router starts the search when it first learns its public address in a family, when the site
of that address changes, and once in every epoch. It runs in the background at low priority.
The node MAY use several threads, 64 MiB each.

While a family has no valid solution, the node acts as a leaf in that family after the
enforcement date: it clears `ROUTER` on connections of that family, or sends no `puzzle` and is
treated as a leaf by its peers.

| | Tries | Time on one core |
|---|---|---|
| Average | 2¹³ = 8192 | about 5 min at 35 ms per try, about 14 min at 100 ms |
| Median | 5678 | about 0.7 × the average |
| 1 search in 20 | more than 24 500 | about 3 × the average |

One try took about 35 ms with OpenSSL 3.6 on an Apple M3 Max, process start included. 100 ms is
an estimate for an older desktop core. Low-power routers may need an hour.

**What an honest router pays**

| Situation | Cost |
|---|---|
| Fixed address | one search per family every 30 days |
| Address changes within the same /24 or /48 | nothing |
| Address changes daily to another site | one search per family per day, and that many minutes a day as a leaf |
| Leaf (firewalled node, phone) | nothing to generate; verification as in §4 |

The third row is the price of binding the solution to the site. It is the reason `n` cannot go
much higher.

## 6. Deployment

The puzzle is a network rule (`Protocol.md` §16) and comes in two steps.

1. **Announce.** Nodes that implement the extension set `PUZZLE`, generate solutions and send
   them. Nothing is enforced; nodes without the bit are treated as before.
2. **Enforce.** From a date fixed in the specification, the rule of §4 applies. Routers without
   a valid solution become leaves in the eyes of upgraded nodes.

Because the puzzle is a nonce search, step 2 changes nobody's node ID. Credits, friends and
source records keep working across the upgrade.

`n` and the Argon2id parameters are raised the same way: a new value and a date. Nodes can
search for the harder solution before that date, and a solution that already has enough zero
bits stays valid.

## 7. What it buys

Costs at n = 13 with 35–100 ms per try. One ID is one key at one site for one or two epochs.

| Attacker goal | IDs needed | Cost |
|---|---|---|
| Hold the closest set of one key | 8–16 | 0.6–3.6 core-hours; minutes on a 16-core machine with 1 GiB of memory |
| 1,000 router identities | 1,000 | 3–10 core-days per month |
| 100,000 router identities | 100,000 | 0.9–2.6 core-years per month |

- **A targeted attack on one key stays cheap.** It needs few identities. The addresses are the
  expensive part of that attack, and the routing ID and the diversity limits are the defence.
- **Mass creation becomes a standing cost.** A Sybil population as large as the network has to
  be paid for again every month, and for every site it is to appear at.
- **Churn is slowed.** A new identity at an address takes minutes, in addition to the one-hour
  rule of the diversity limits.
- **No stockpile across time.** A solution is dead after at most 61 days. Epoch numbers are
  predictable, so an attacker can prepare solutions for a *chosen* future epoch well in advance.
  The epoch limits how long an identity lasts, not how early it can be made.
- **Site binding.** One solution is one network location. Without it, one key could be shown
  from many addresses at once and take a different routing position at each.

## 8. Comparison with S/Kademlia

S/Kademlia (Baumgart and Mies, 2007) defines two puzzles with a plain hash function.

| | S/Kademlia static | S/Kademlia dynamic | This extension |
|---|---|---|---|
| What is searched | the key pair | a value X beside the node ID | a nonce beside the key |
| Condition | `H(H(public_key))` has c₁ leading zero bits | `H(NodeID ⊕ X)` has c₂ leading zero bits | Argon2id output has n leading zero bits |
| Purpose | the ID cannot be chosen | many IDs are costly | many IDs are costly |
| Identity survives a harder puzzle | no | yes | yes |
| Bound to address or time | no | no | site and epoch |
| Verification | two hashes | one hash | one Argon2id run, 64 MiB |
| Advantage of GPU or custom hardware | very large | very large | small |

- The static puzzle is not needed. It stops a node choosing its position, which the IP-bound
  routing ID of `Protocol.md` §6.3 already does.
- The dynamic puzzle is what this extension is. The function is replaced because a plain hash
  is searched thousands of times faster on a GPU than on the CPU of an honest node, so its
  difficulty prices honest users and not attackers. The price of the replacement is that
  verification is no longer free (§4).

## 9. Alternatives considered

- **More memory (300 MiB and up, as disk encryption uses).** The attacker's cost is memory ×
  time × tries, however it is split. Memory only has to exceed CPU caches and limit how many
  instances fit on a graphics card; 64 MiB does. More would be paid by every verifier and would
  not fit on small devices.
- **Key search instead of a nonce** (the earlier text of `Protocol.md` §6.3). Enforcing or raising it later
  would give every node a new ID.
- **Bound to the key only.** No re-solving when the address changes, but one solution would work
  from any number of addresses at once.
- **No expiry.** Solve once, no clock dependency, but identities would accumulate without limit.
- **Every node solves.** Would also price fake sources and publishers, at the cost of a search on
  every phone and a verification for every publisher.
- **A puzzle that is cheap to verify** (Equihash, Cuckoo Cycle). Memory-hard to solve and
  verified in microseconds, which would remove §4 altogether. They are more complex, have fewer
  reviewed implementations and no standard, and Argon2id is already in OpenSSL 3.2 and later.
  Worth revisiting if verification load proves to be a problem.
- **An unpredictable epoch value**, so that solutions cannot be prepared early. A peer-to-peer
  network has no source for one that all nodes agree on.

## 10. Open points

- `n` = 13 is set from one measurement and one estimate. It needs timings on low-power routers
  before a date is fixed.
- Routers whose address moves to another site every day lose several minutes a day as routers.
  Whether that is acceptable needs data on how common such routers are.
- Whether a node may answer as a router to peers that have not upgraded while it has no
  solution. This draft leaves that to the announce step, where nothing is enforced.
- Test vectors, one output that is not valid and one solution, are in
  [`tests/vectors-v1.md`](../tests/vectors-v1.md).
