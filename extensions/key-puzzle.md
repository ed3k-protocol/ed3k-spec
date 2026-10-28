# Extension: key puzzle (`key-puzzle`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Not part of wire version 1; reserved for a network upgrade.
**Extends:** [`Protocol.md`](../Protocol.md) §6.3. § = this document unless a document is named.
**Capability:** bit 7, `PUZZLE`.

Routing ID (`Protocol.md` §6.3) limits *where* a node may sit; diversity limits
([`ip-diversity.md`](ip-diversity.md)) limit how many nodes of one network count. Neither prices
the node ID: a key costs microseconds. Here a `ROUTER` node must show a solution to a memory-hard
puzzle before others treat it as router. Price: recurring, per identity, network location and
month; modest (§7); doesn't replace the other two measures.

## 1. Overview

- A **nonce search**: key, hence node ID, never changes because of the puzzle.
- **Argon2id** (RFC 9106), 64 MiB, one pass. A try costs tens of ms; GPU/custom hardware can't
  make it much cheaper.
- Bound to **key**, node's **site group** (IPv4 /24, IPv6 /48) and a 30-day **epoch**; useless
  at another address or two months later.
- Only **`ROUTER`** nodes need one; leaves pay nothing.
- Finding takes minutes. **Checking costs a single try**: ~35–100 ms, 64 MiB, once per node,
  site and epoch.

## 2. The puzzle

```
out = Argon2id(
        password = public_key(32) ‖ family(1) ‖ site(6) ‖ epoch(4) ‖ nonce(8),
        salt     = "ed3k v1 puzzle"                    (14 ASCII bytes)
        t = 1,  m = 65536 KiB,  p = 1,  tag length = 32,  version = 0x13 )

valid  ⇔  the first n bits of out are zero            n = 13 (initial)
```

Bits counted from MSB of `out[0]`: n = 13 → `out[0]` is 0, top 5 bits of `out[1]` are 0.

| Field | Value |
|---|---|
| `public_key` | node's Ed25519 public key (`Protocol.md` §6.1) |
| `family` | one byte: `0x04` IPv4, `0x06` IPv6 |
| `site` | IPv4: first 3 address bytes + 3 zero bytes. IPv6: first 6 bytes |
| `epoch` | `floor(unix_time / 2 592 000)`, big-endian; 30 days |
| `nonce` | any 8 bytes; node searches this |

- More than `n` zero bits is valid. Node MAY keep the best found.
- Site = that of the **observed** address (`Protocol.md` §9.2), same one the routing ID is
  checked against.
- Dual-stack router: one solution per family.
- In a named network a router at a private address (`Protocol.md` §8.2 (8)) needs no solution,
  as it needs no routing-ID check there (`Protocol.md` §6.3); on the public network such
  addresses never become contacts.

### 2.1 Epochs

Verifier accepts current and previous epoch; in an epoch's last 24 hours also the next one
(clock error). So a solution found any time in epoch *e* is valid until end of *e* + 1, 30–60
days. Router solves for the new epoch in background some time during it; no moment where all
routers must solve at once.

## 3. Carrying the solution

`Hello` (`Protocol.md` §5.5) gains one field:

```protobuf
  bytes puzzle = 9;           // epoch(4, big-endian) ‖ nonce(8), for the family of this connection
```

- Node setting `ROUTER` and `PUZZLE` sends the solution for the site it believes the peer sees
  (public address known from earlier `observed` reports).
- 12 bytes. Compact contacts in `KadAnswer` carry none: a contact is verified by connecting to
  it (`Protocol.md` §6.3), and that connection's `Hello` brings it.
- Datagram binding ([`kad-datagram.md`](kad-datagram.md)) has no `Hello`: solution follows
  responder's public key in `CHALLENGE`, 12 bytes more; an `INIT` is ≥ 70 bytes so it fits
  ([`kad-datagram.md`](kad-datagram.md) §2.1). It is the one for the datagram's family (as
  `Hello` for its connection's), sent only while node is `ROUTER` and has a valid one.
- `REQUEST` carries none, so a node is never puzzle-verified by an exchange where it is
  requester; only as responder, or by a `Hello`.

## 4. Verification

Router contact is **puzzle-verified** when its solution is valid for its node key, its observed
address's site, and an accepted epoch. After enforcement date (§6), a contact not
puzzle-verified is treated as leaf: not inserted into a routing table, not handed out in
answers, not counted towards lookup termination, not a storage target. Requests still served;
not banned.

Verification = one Argon2id run, so rules keep cost under verifier's control:

- **Lazily.** Verified only where the contact would otherwise be inserted, counted or stored on; not on
  receipt of `Hello`.
- **Bounded for peers that dial in.** Dialler picks when it comes and which key it shows, so its
  runs are rationed. Verified only when it would be inserted as dialled-in contact
  (`Protocol.md` §10.2), out until then. At most 1 *(initial)* dialled-in contact verified per host group
  ([`ip-diversity.md`](ip-diversity.md) §1) per hour, whatever the result. Failed result kept as
  below.
- **Own lookups first.** Contacts from own lookups verified before any dialled-in one; a dialled-in one
  checked only while none of the others waits.
- **One at a time.** At most one verification at once: 64 MiB total, not per peer. Pending ones
  queue; a lookup doesn't wait for a still-queued contact, just doesn't count it yet.
- **A capped queue.** At most 64 *(initial)* contacts. When full, newest dropped: stays
  unverified, re-queued next time it would be inserted, counted or stored on.
- **Cached.** Result kept per (node ID, family, site, `puzzle` value); SHOULD be written to disk with the
  routing table. Valid result kept while its epoch is accepted (§2.1): known router costs
  nothing until then. Failed result kept until end of current epoch, so same solution isn't
  checked twice. Another `puzzle` value for that node ID, family and site re-checked at most
  once per hour *(initial)*: a moved router shows its old solution once before learning its new
  address.
- **Datagram binding.** Node that only sent `REQUEST`s showed no solution (§3); puzzle-verified
  only by an exchange as responder, or by a `Hello`.
- **One value per connection.** Connection carries its `Hello` value for its whole life. Contact
  whose value leaves the accepted epochs while connection stays open is not puzzle-verified
  until next connection; requests still served.
- **The newest value counts.** New value fails → not puzzle-verified, even if an older value of
  the same site was valid.

Lookup meeting 10 to 20 routers for the first time: roughly 0.4 to 2 s of one core total. Warm
node pays close to nothing.

## 5. Generation

Router starts the search on first learning its public address in a family, when that address's
site changes, and once every epoch. Background, low priority. Node MAY use several threads,
64 MiB each.

While a family has no valid solution, after enforcement date node acts as leaf in that family:
clears `ROUTER` on that family's connections, or sends no `puzzle` and peers treat it as leaf.

| | Tries | Time on one core |
|---|---|---|
| Average | 2¹³ = 8192 | ~5 min at 35 ms/try, ~14 min at 100 ms |
| Median | 5678 | ~0.7 × average |
| 1 search in 20 | > 24 500 | ~3 × average |

One try ~35 ms with OpenSSL 3.6 on Apple M3 Max, process start included; 100 ms estimated for
an older desktop core. Low-power routers may need an hour.

**What an honest router pays**

| Situation | Cost |
|---|---|
| Fixed address | one search per family every 30 days |
| Address changes within same /24 or /48 | nothing |
| Address changes daily to another site | one search per family per day, that many minutes/day as leaf |
| Leaf (firewalled node, phone) | nothing to generate; verification as in §4 |

Third row = price of site binding, and why `n` can't go much higher.

## 6. Deployment

Network rule (`Protocol.md` §16), two steps:

1. **Announce.** Implementing nodes set `PUZZLE`, generate and send solutions. Nothing
   enforced; nodes w/o the bit treated as before.
2. **Enforce.** From a date fixed in the specification, §4 applies. Routers w/o valid solution
   become leaves to upgraded nodes.

Nonce search → step 2 changes nobody's node ID; credits, friends, source records survive the
upgrade. `n` and Argon2id parameters raised the same way (new value and date). Nodes can search
for the harder solution beforehand; one already with enough zero bits stays valid.

## 7. What it buys

n = 13, 35–100 ms per try. One ID = one key at one site for one or two epochs.

| Attacker goal | IDs needed | Cost |
|---|---|---|
| Hold closest set of one key | 8–16 | 0.6–3.6 core-hours; minutes on 16 cores with 1 GiB memory |
| 1,000 router identities | 1,000 | 3–10 core-days per month |
| 100,000 router identities | 100,000 | 0.9–2.6 core-years per month |

- **Targeted attack on one key stays cheap**: few identities needed. Addresses are the expensive
  part; routing ID and diversity limits are the defence.
- **Mass creation becomes a standing cost**: a network-sized Sybil population must be paid again
  every month, and per site it appears at.
- **Churn slowed**: new identity at an address takes minutes, on top of the diversity limits'
  one-hour rule.
- **No stockpile across time**: solution dead after at most 61 days. Epoch numbers are
  predictable, so an attacker can prepare solutions for a *chosen* future epoch well ahead.
  Epoch limits how long an identity lasts, not how early it can be made.
- **Site binding**: one solution = one network location. Without it one key could be shown from
  many addresses at once, a different routing position at each.

## 8. Comparison with S/Kademlia

S/Kademlia (Baumgart and Mies, 2007) defines two puzzles with a plain hash function.

| | S/Kademlia static | S/Kademlia dynamic | This extension |
|---|---|---|---|
| Searched | key pair | value X beside node ID | nonce beside key |
| Condition | `H(H(public_key))` has c₁ leading zero bits | `H(NodeID ⊕ X)` has c₂ leading zero bits | Argon2id output has n leading zero bits |
| Purpose | ID can't be chosen | many IDs costly | many IDs costly |
| Identity survives a harder puzzle | no | yes | yes |
| Bound to address or time | no | no | site and epoch |
| Verification | two hashes | one hash | one Argon2id run, 64 MiB |
| GPU/custom hardware advantage | very large | very large | small |

- Static puzzle not needed: stops a node choosing its position, which the IP-bound routing ID
  (`Protocol.md` §6.3) already does.
- Dynamic puzzle = this extension. Function replaced since a plain hash is searched thousands of
  times faster on GPU than on an honest node's CPU, so difficulty prices honest users, not
  attackers. Cost: verification no longer free (§4).

## 9. Alternatives considered

- **More memory (300 MiB and up, as disk encryption uses).** Attacker cost = memory × time ×
  tries, however split. Memory need only exceed CPU caches and limit instances per graphics
  card; 64 MiB does. More is paid by every verifier, won't fit small devices.
- **Key search instead of a nonce** (earlier text of `Protocol.md` §6.3). Enforcing/raising
  later would give every node a new ID.
- **Bound to key only.** No re-solve on address change, but one solution works from any number
  of addresses at once.
- **No expiry.** Solve once, no clock dependency, but identities accumulate without limit.
- **Every node solves.** Also prices fake sources and publishers, at cost of a search on every
  phone and a verification per publisher.
- **A puzzle cheap to verify** (Equihash, Cuckoo Cycle). Memory-hard to solve, verified in
  microseconds; would remove §4. More complex, fewer reviewed implementations, no standard;
  Argon2id already in OpenSSL 3.2+. Revisit if verification load proves a problem.
- **An unpredictable epoch value**, against early preparation. P2P network has no source all
  nodes agree on.

## 10. Open points

- `n` = 13 from one measurement and one estimate; needs low-power router timings before a date
  is fixed.
- Routers moving to another site daily lose several minutes a day as routers; acceptable?
  Needs data on how common they are.
- May a node answer as router to non-upgraded peers while it has no solution? Draft leaves that
  to the announce step, where nothing is enforced.
- Test vectors (one invalid output, one solution): [`tests/vectors-v1.md`](../tests/vectors-v1.md).
