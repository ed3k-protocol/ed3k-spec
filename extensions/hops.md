# Extension: anonymous paths (`hops`)

**Status:** draft 0.2 (2026-11-09: leaves as hops, §2.5; path built in one message, §2.2; deniable 0 hops, §2.6; 0.1 2026-10-12). Not implemented. Optional in wire version 1. **The key agreement and ciphers of §2.3 and §3.2 are a first design and need review by professionals before anyone relies on them.**
**Extends:** [`Protocol.md`](../Protocol.md) §2, §5.1, §9.4, §9.5, §10.1, §10.3 and §10.7. § = this document unless a document is named.
**Capability:** bit 14, `ANON`; bit 15, `HOP`. Message types 26, 27, 29 to 34 and 39 (28 unused). Record kinds 11 and 12. One field in `Record`, one in `Store`, one in `KadRequest`, one in `Relay`, four in `KadAnswer`.
**Required of every `ROUTER`:** storing and returning the record kinds `ANON` and `GATE`, and accepting `signer` in a `Store` (§4). The rest is optional.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Hop keys* (build keys and a slot included) and *Hop session*.

In ed3k every peer sees its peer's address, and every storing node sees who stores and who
searches. This extension lets a node put **0 to 3 other nodes** between itself and the rest; each
node chooses the number for itself. At 0 it still looks like a hop forwarding for someone (§2.6).

- A hiding node builds **paths**. It picks every node of a path itself and wraps what it sends in
  one encryption layer per node. A node on a path, a **hop**, knows the node before and after it
  and nothing else.
- **Each side hides itself.** A downloader builds a path outwards. A hiding source builds a path
  inwards and publishes its far end, the **gate**, instead of an address. Between two hidden nodes
  are the hops of both.
- A hop forwards **reliable streams**. Each path link is an exchange of the connection between two
  neighbours, so loss is repaired on the link where it happens, and a hop that can't pass data on
  stops reading.
- Being a hop is **voluntary**. A hop gives a small rate to everyone and more to neighbours that
  forward for it.

```
 downloader ── hop ── hop ── hop ─────── gate ── hop ── hop ── source
 |<------ path of the downloader ------>|<----- path of the source ----->|
 |<----------------------------- session ------------------------------->|
```

Starting point was NeoLoader's NeoShare: a user-chosen number of hops. What was kept and changed:
§7.

---

## 1. Roles

| Bit | Name | Meaning |
|---|---|---|
| 14 | `ANON` | Node accepts sessions arriving through a path (§3) and understands anonymous sources (§4). Set only when it does |
| 15 | `HOP` | Node forwards for others' paths (§2, §5). Set by a `ROUTER`, or by a leaf that is `cone` where it holds relay reservations (§2.5); a node setting it sets `ANON` |

**Anonymous key.** A hiding node has a second Ed25519 key pair, the *anonymous key*, with its
own node ID (`Protocol.md` §6.1). Everything it does through a path it does under that key.

- Anonymous key MUST NOT be node key, and nothing may be signed with both.
- A node under its anonymous key is a leaf: no address, so no routing ID, in no routing table, and
  never sets `ROUTER`, `RELAY` or `HOP`.
- A node MAY take a new anonymous key at any time; it then loses its queue places and what
  uploaders remember of it.

**One machine, two nodes.** The same machine MAY be a hop under its node key (a `ROUTER`, or a
leaf of §2.5) and share and download under its anonymous key. A hiding node SHOULD be a hop
where it can: others' traffic covers its own, and a node not setting `HOP` is known as owner by its
first hop (§6). A client turns `HOP` on by default while hiding is on and the node is open or `cone`
(§2.5), before its first own path.

- A file MUST NOT be offered under both keys: two source records for the same rare files connect
  the keys. Records naming a file shared under anonymous key (keywords, notes, aliases,
  `MEMBER`) are published only under that key, through a path (at 0 hops, §2.6). A `MEMBER` record is published only
  when file and set are shared under the same key. An own set MUST NOT list a file the node shares
  under the other key: client refuses it, offers sharing the set under the file's key.
- A file shared under anonymous key SHOULD NOT also be shared or downloaded by the machine on
  another network (eD2K/Kad1, BitTorrent, HTTP): sources there show its address and tie the key to
  it. Client SHOULD warn the user who does so, or moves such a file to anonymous key.
- What is done under anonymous key MUST go through a path, or at 0 hops as §2.6 says: no direct
  connection, no HTTP
  source ([`http-source.md`](http-source.md)), no eD2K (`Protocol.md` §14), no answer in the local
  network ([`lan-discovery.md`](lan-discovery.md)).
- Hiding off (no path): nothing under anonymous key is served or asked for; nothing falls back to
  node key.
- A client MUST show its user which files are shared under which key, and MUST ask before moving
  a file from one to the other.

**How many hops.** User setting per direction: off, or 0 to 3. Off = ed3k as is, under node key.
0 = no hop, but under anonymous key, node being its own last hop and gate (§2.6).

- A client MAY let the user pick it per use instead (lookups, downloads, shares, rooms, …). All
  uses share one anonymous key, so the key is as hidden as its shortest path in use; client SHOULD
  tell the user. A use set off outwards runs under node key, as below; a download's lookups go under
  the download's key.
- With outwards off and inwards on, a node searches and downloads under its node key, as
  ed3k is; under its anonymous key it only shares. For that key's Kad requests it keeps one path
  without a gate, of as many hops as inwards, used for `HopLookup` alone (§2.1, §4.1); at 0
  inwards it runs them itself (§2.6). Sharing is
  serving: under that key it sends no `FileRequest`, no search, and no `SlotOffer`
  (`Protocol.md` §12.5), having no path to reach a downloader. Its downloaders stay connected or
  come back when `recheck` says.

| Hops | A single hop knows | It takes to connect address and deed |
|---|---|---|
| off | | peer sees it |
| 0 | | peer sees a last hop or gate and can only guess it is the owner (§2.6) |
| 1 | both ends | that one hop |
| 2 | one end each | both hops together |
| 3 | one end, or neither | the first and the last together |

---

## 2. Paths

A path has an **owner** and one to three hops. Owner knows all; a hop knows its neighbours. The
hop furthest from owner is the **last hop**.

### 2.1 Choosing hops

- Owner keeps a list of `HOP` nodes with their keys, filled from nodes it dials itself in Kad
  lookups for random targets and whose `Hello` shows `HOP` and `ROUTER`, and from leaf hops those
  lookups list (§2.5), and SHOULD hold at least 20 *(initial)* before building a path. A node
  that connected to it is not taken from that connection. Random targets are drawn uniformly over the whole ID space; bucket refreshes
  (sampling near own routing ID) and first-contact lookups don't count.
- Hops of one path, owner, and a stream's destination node MUST be in different **sites** and
  SHOULD be in different **blocks** ([`ip-diversity.md`](ip-diversity.md) §1).
- A last hop MAY refuse a `HopConnect` whose `node` is in its own site, with `Error{DENIED}`; a
  hop, a build record whose `next` is, with `status` `DENIED` (§2.2).
- **Entry hops.** Owner picks 3 *(initial)* `ROUTER` hops and uses one of them as first hop of every path,
  for 30 days *(initial)*. With a new first hop per path an owner would sooner or later pick a
  hostile first and last hop at once. Tor does this for the same reason.
- Other hops picked at random per path. A leaf hop is never first hop and never last hop of a path
  with a gate (§2.5).
- A node keeps up to 4 *(initial)* outward paths and up to 4 gates (§3.1). It measures each
  path's rate and replaces the slowest when far below the others; how far is local policy.
- An owner MUST NOT send `HopLookup` (§4.1) on a path with a gate, and SHOULD NOT open a
  `HopConnect` there: those streams would tie the gate to the owner's outward activity. A path
  built to become a gate counts as one from its first `HopCreate`, before its `HopGate`.
- A node with inwards off has no gate. At 0 inwards a `ROUTER` is its own gate (§2.6); a leaf
  can't be one (§2.5) and has none either. Without a gate, B can't open a stream to a hidden A (§3.3): an uploader can't
  offer it a slot (`Protocol.md` §12.5), and A comes back by itself when `recheck` says.

### 2.2 Building

A path is built in one message. Owner seals one record per hop into a fixed-size **build**. Each
hop opens its own record and passes the build on; on the way back each adds its answer. Every hop
sees one build of one size, whatever the path's length and its place in it, so no hop learns its
place from the build, except last hop, whose record says so.

```protobuf
message HopCreate {           // type 26. First frame of an exchange between two neighbours
  bytes path = 1;             // 8 B, random, chosen by the sender; names the path on this connection only
  bytes build = 3;            // 768 B: 3 slots of 256 B (§2.3)
  // field 2 unused (was `e`, draft 0.1)
}

message HopCreated {          // type 27
  bytes build = 3;            // 768 B: the build on its way back (§2.3)
  // fields 1 and 2 unused (were `f` and `proof`, draft 0.1)
}

// type 28 unused (was `HopExtend`, draft 0.1)
```

**Slots.** A build has 3 slots, one per hop. Owner puts each hop's record in a slot drawn at
random and fills unused slots with random bytes. Record and answer: §2.3.

1. Owner opens an exchange to first hop and sends `HopCreate` with the build. Exchange stays
   open: it is the path's **control stream** on this link, stream number 0 (§2.4).
2. A hop finds its record, the slot starting with its node ID, and opens it (§2.3). It overwrites its own slot with random bytes and runs its build layer over the other
   two (§2.3).
3. Record says *pass on*: hop connects to `next` as to any node, through a punch where the record
   names relays (§3.1), opens an exchange and sends `HopCreate` with a `path` of its own and the
   build. Record says *end*: hop is last hop.
4. **On the way back.** A hop writes its answer into its own slot (§2.3) and sends the build back
   in `HopCreated`. Last hop does so after step 2, holding it back a random 0 to 1 s *(initial)*
   so the wait tells a hop little about the rest of path (§6); any other hop when next hop's
   `HopCreated` arrives or that link failed. A failed next link goes into the answer as `status`;
   the hop answers all the same.
5. Owner opens every answer (§2.3), from first hop on, up to the first with `status` not 0: hops
   after it wrote none. All `status` 0 = path built. Else no path: owner closes
   control stream, replaces the hop the failed link leads to, and builds anew. An answer that
   doesn't open: that hop isn't the node owner chose, or a hop before it changed the slot; owner
   closes path and builds anew with other hops after the first. Owner knows its first hop by
   connection key (`Protocol.md` §4.1), and opens its answer all the same.

- **A hop refusing the build** answers `Error` instead of `HopCreated`, which ends the exchange:
  `BUSY` above a limit (§5); `MALFORMED` for a `path` not 8 B or in use on that connection,
  whichever neighbour chose it, a `build` not 768 B, no slot naming it, a record that doesn't open
  or breaks §2.3, a `time` more than 10 min *(initial)* from its clock, an `e` it saw before, or an
  all-zero `ss1` or `ss2`. Previous hop puts the code in its own answer as `status`; owner,
  refused by first hop, has no path.
- **Replays.** A hop keeps the `e` of every record it opened for 20 min *(initial)*. A replayed
  build would build the same path again and show its hops.
- **`next`** and its relays, here and in `HopConnect` (§3.1), MUST be global unicast addresses on
  the public network (`Protocol.md` §8.2 (7)). A private address is a dial target only in a named
  network (`net_id` not empty, `Protocol.md` §4.3). Any other: `status` `MALFORMED`, not dialled.
  A `next` in the hop's own site MAY get `status` `DENIED` (§2.1).
- **`status`**, an `ErrorCode` (`Protocol.md` §5.6): 0 = built; `NOT_FOUND` = `next` not reached
  within 10 s *(initial)*, 20 s *(initial)* when punching, or no `HopCreated` from it 30 s
  *(initial)* after the `HopCreate`; `BUSY` = a dial limit (§5); `MALFORMED`, `DENIED` as above;
  else the `Error` next hop answered, unchanged; one reading as `UNKNOWN` (`Protocol.md` §5.6),
  which would read as built, goes in as `NOT_FOUND`. A timeout can name an earlier link than the one
  that stalled; owner replaces that hop all the same.
- Owner closes path with no `HopCreated` 60 s *(initial)* after its `HopCreate`, or no answer
  10 s *(initial)* after a `HopGate`.
- **Length capped by the format.** A build holds 3 records and a hop without one refuses it: no
  path is longer than 3 hops, none is extended later. A path wanting another hop is built anew.
- Everything on control stream after `HopCreated` is in layers (§2.3).
- **Control stream is a sequence of requests and answers.** After `HopCreated` owner's only
  request is `HopGate` (§3.1); it waits for the answer before the next. An `Error` on control
  stream doesn't close it. Another frame type from owner is answered `Error{UNSUPPORTED}`, except
  an `Error`, which gets no answer (`Protocol.md` §5.6); an unparseable frame closes path (§2.3).
- **Keeping a path alive.** Both neighbours of a path link send a QUIC PING every 20 s *(initial)*
  while control stream is open (`Protocol.md` §4.1: `max_idle_timeout` is 30 s, and whoever
  wants a connection kept sends the PING). The PINGs also keep a leaf hop's NAT mapping (§2.5). A
  gate lives as long as its path.
- A path ends when its control stream closes or a neighbour connection is lost. A hop then closes
  control stream and every stream of path on its other side. Owner builds another.

### 2.3 Keys and layers

Per hop owner makes a fresh X25519 pair `(x, e)`; hop makes `(y, f)` when it opens its record.
Hop's node key is used for X25519 via the conversion of `Protocol.md` §6.4.

```
ss1          = X25519(x, converted node key of the hop)         the hop computes X25519(its converted private key, e)
kr ‖ ka ‖ kb = H("ed3k v1 hop build", ss1 ‖ e ‖ node key of the hop)          96 bytes
ss2          = X25519(x, f)                                     the hop computes X25519(y, e)
k            = H("ed3k v1 hop", ss1 ‖ ss2 ‖ e ‖ f ‖ node key of the hop)      32 bytes
k_out        = H("ed3k v1 hop out", k)                          for bytes that move away from the owner
k_in         = H("ed3k v1 hop in", k)                           for bytes that move towards the owner
```

**A slot**, 256 B:

```
record (owner → hop)   to(16) ‖ e(32) ‖ seal(kr, body(192))                  to = the hop's node ID
answer (hop → owner)   random(48) ‖ seal(ka, status(1) ‖ f(32) ‖ zero(159))
seal(k, m)             = ChaCha20-Poly1305(k, nonce 0, m, empty AAD)          as §3.2: RFC 8439, 16-byte tag
```

| Body bytes | Field | |
|---|---|---|
| 0 | `end` | 1 = hop is last hop, 0 = pass on |
| 1–8 | `time` | Unix seconds, big-endian, when owner made the build |
| 9–43 | `next` | length(1) ‖ compact node of 22 or 34 B, zero-padded to 34. Length 0 when `end` = 1 |
| 44–183 | `relays` | 4 × (length(1) ‖ compact node zero-padded to 34): relays of `next` (§3.1, §2.5), ≤ 2 per family; length 0 = none |
| 184–191 | | zero |

A record breaks this when `end` is neither 0 nor 1, a length is not 0, 22 or 34, `end` = 0 has
no `next`, `end` = 1 has a `next` or a relay, or a padding byte isn't 0 (§2.2: `MALFORMED`).

**Build layer.** A hop runs over each slot but its own

```
ChaCha20(key = kb, nonce = 0x00000000 ‖ slot index(8, big-endian), counter from 0)      slot index 0 to 2
```

Owner runs the build layers of hops 1 to j−1 over hop j's slot before sending, so hop j finds its
record as sealed. On the way back no hop touches a slot but its own; owner reads every answer as
written.

- Hop's node key in `kr ‖ ka ‖ kb` and `k` is the Ed25519 key as sent in the handshake (32 B), not
  the converted one; the converted key is used for `ss1` alone.
- Both sides MUST refuse an all-zero `ss1` or `ss2`.
- Only the holder of the hop's node key opens the record and seals the answer: an answer that
  opens proves the hop and its `f`. `ss2` keeps `k` safe if that key is taken later. Hop learns
  nothing about owner.
- **The record is not forward secret.** It is sealed under `ss1` alone. Whoever recorded a build
  and later takes a hop's node key reads that hop's `next` and relays: the path's layout, not its
  traffic.
- Owner MUST NOT use one `e` for two hops. `kr`, `ka` each seal one message, `kb` runs over one
  build. Hop deletes `kr ‖ ka ‖ kb` after answering, `y` and `k` when path ends.

**Layers.** Every path stream (§2.4) is encrypted once per hop, in each direction:

```
ChaCha20(key = k_out or k_in of the hop, nonce = 0x00000000 ‖ stream number(8, big-endian), counter from 0)      RFC 8439
```

Key stream runs over the whole stream in that direction. On control stream, stream number is 0,
and every hop's layer starts, counter at 0, at the first byte after `HopCreated` on each link; on
any other stream a layer starts at the first byte after `HopStream` in the opener's direction,
and at the first byte after the link's status frame (§2.4) in the other. At last hop that is the
first byte of the status frame of `HopConnect`, or of the `HopLookupAnswer`.

- Away from owner: owner applies the last hop's layer first and the first hop's last; each hop
  removes its own.
- Towards owner: each hop adds its own; owner removes all.
- **There is no tag on a layer.** The next hop can't notice a hop changing bytes; it's noticed at
  the end: a session checks every record (§3.3). Record and answer of a build are sealed. A last
  hop reading an unparseable frame closes path.
- A stream MUST be closed before 256 GiB have passed in one direction, the cipher's limit for one
  nonce.

### 2.4 Streams

A path stream is one exchange per link, joined end to end by the hops.

```protobuf
message HopStream {           // type 32. First frame of an exchange between two neighbours
  bytes  path = 1;            // as in HopCreate, for this connection
  uint64 number = 2;          // never 0 (the control stream); the opener counts up, and a number is never used twice on a path. Even: opened by the owner. Odd: opened at the last hop
}

message HopOk {}              // type 39. Status frame: accepted, raw bytes follow
```

- After `HopStream` exchange carries unframed bytes, in layers, until closed. Like `Block`,
  `HopConnect` and `HopJoin` (§3.1), it's a message followed by raw bytes (`Protocol.md` §5.1).
- **Status frame.** First in the other direction of an exchange starting with `HopStream`:
  one frame, before any raw byte, `HopOk` or an `Error`. Neighbour that received the
  `HopStream` sends it, without a layer, as soon as it accepted or refused for itself; it doesn't
  wait for the next link. The opening neighbour reads it and doesn't pass it on. After `HopOk`
  come stream bytes, in layers. An opener with no status frame 10 s *(initial)* after
  `HopStream` or `HopJoin`, or 30 s *(initial)* after `HopConnect` (§3.1), resets exchange.
- A hop receiving `HopStream` opens an exchange on the path's other side with that link's `path`
  value and same `number`, and copies both ways. Half-close and reset are passed on. A hop
  receiving an `Error` as status frame, or a reset, on one side of a stream resets the other side.
- Unknown `path` → `Error{NOT_FOUND}`.
- **Numbers never repeat on a path**, so a layer's key stream is never reused. Each opener MUST use
  a number greater than every number it used before on path. A link's exchanges arrive in any
  order, so receiver doesn't require that order. A hop, and owner for streams reaching it,
  accepts a number not seen before on that path from that side and above the highest seen from
  that side minus 1,024 *(initial)*. A `HopStream` with `number = 0`, a seen number, or one not
  above that bound → `Error{MALFORMED}`.
- **Parity.** Odd number from owner side, or even from far side → `Error{MALFORMED}` as status
  frame; a node unable to send one resets exchange. Layer nonce rests on it.
- **A hop keeps little.** ≤ 256 KiB *(initial)* per stream and direction. When the next link
  takes no data, it stops reading from the previous link. Bytes handed to the next link's
  transport no longer count. Exchange flow control carries this back to sender. Nothing is
  acknowledged twice, nothing dropped.
- A path carries ≤ 64 *(initial)* streams at a time.
- What last hop does with a stream is told by its first frame: `HopConnect` (§3.1) or
  `HopLookup` (§4.1). Another type → `Error{UNSUPPORTED}`, in the layers, and that stream is
  closed; an unparseable frame closes path (§2.3).

### 2.5 Leaf hops

A hop needn't be open. A leaf (`Protocol.md` §2) behind a cone NAT is reached by a punch through
its relays, as any leaf is (`Protocol.md` §9.5). So it can forward, and earn give and take (§5),
instead of only using paths.

```protobuf
// in KadRequest (Protocol.md §10.3):
bool hops = 7;                // list leaf hops holding a relay reservation here

// in KadAnswer:
bytes leaves4 = 21;           // n × 22 B compact nodes: leaf hops with a reservation here, endpoint as this node observes it
bytes leaves6 = 22;           // n × 34 B; likewise

// in Relay (Protocol.md §9.4):
bool hop = 3;                 // request: the leaf sets HOP
```

**Who.** A leaf MAY set `HOP` while `cone` (`Protocol.md` §9.2) in a family where it holds relay
reservations (§9.4 there).

- Not `symmetric`: a hop meets neighbours of every class, and two symmetric sides aren't punched
  (`Protocol.md` §9.5). Not `unknown` either.
- A leaf no longer `cone` in such a family clears `HOP` on new connections, as `ROUTER` is cleared
  (`Protocol.md` §2). Its paths end when its mapping does (§2.2).
- **Telling its relays.** Its reservation connections predate `HOP`, and `caps` shows only on new
  connections (`Protocol.md` §2). So a leaf sends `Relay{reserve, hop}` again on each reservation
  connection whenever its `HOP` changes: `hop` set when it turns `HOP` on, clear when off. Relay
  answers it as a repeat of the reservation (§9.4 there: one slot per leaf, no new limit check)
  and keeps the last `hop` with the slot.

**Listed by its relays.** A leaf is in no routing table, so lookups never meet it. Its relays name
it instead.

- A node answering a `KadRequest` with `hops` set lists, in `leaves4` and `leaves6`, leaves that
  hold a reservation with it in that family and whose last `Relay` on it set `hop`.
  ≤ 4 *(initial)* per family, picked at random per answer. Endpoint = what it observes on that
  connection, never the leaf's claim.
- Listing is optional. A relay having this extension SHOULD list; a node without it ignores
  `hops` (`Protocol.md` §16).
- Requester drops a list whose length isn't a multiple of 22 or 34 B, and an entry whose address
  §2.2 refuses as `node`.

**Taking one.** In its lookups for random targets (§2.1) owner sets `hops`.

- It takes ≤ 1 *(initial)* leaf per answer, so no single relay fills the list. It punches the leaf
  through the answering node (`Protocol.md` §9.5) and keeps it when its `Hello` shows `HOP` and
  `nat` = 2. The answering node is noted as the leaf's relay.
- Owner already connected to the leaf in that family (e.g. as its relay): no punch, and that
  connection's `Hello` predates `HOP`. Owner keeps the leaf on the listing alone; as its relay,
  on the leaf's `Relay{hop}`. A build the leaf refuses, or whose link to it fails, drops it.
- Leaves make up ≤ half *(initial)* of the hop list. A leaf whose punch failed, or whose relay
  refused it, leaves the list.

**Where in a path.** Middle hop, or last hop of a path without a gate. Never:

- first hop: entry hops are kept 30 days (§2.1), a leaf's relays and mapping are not;
- gate: `Gate.node` (§4.3) is dialled at its address, with no relays.

A leaf hop therefore sits only on paths of 2 hops or more.

**Reaching it.** Owner names the leaf's relays in `relays` of the record of the hop before it
(§2.3); that hop punches through one as §3.1 says. Site and block rules (§2.1) take the leaf's
address from its entry.

**Limits.** A leaf hop is a punch target once per new neighbour. Its relays forward ≤ 30 punches a
minute for it, and it starts ≤ 30 a minute (`Protocol.md` §9.4, §9.5). That bounds its new
neighbours; §5 bounds the rest, as for any hop.

### 2.6 Zero hops

At 0 hops a node puts no hop in between but still acts under its anonymous key: outwards it is its
own last hop, inwards its own gate. A peer sees what it sees of a hop forwarding for an owner
behind it, and from one session can't tell the two apart. Deniable, not hidden: nobody stands in
between.

- **Only while it sets `HOP`** (§1) and forwards for others: without paths through it, every
  session from it would be its own. A client offers 0 only then, and tells the user it is
  deniable, not hidden. While node doesn't set `HOP` (§2.5, §5), its 0-hop uses go over 1 hop.
- **Outwards** it does for itself what a last hop does on `HopConnect` and `HopLookup`: sends
  `HopJoin` itself (§3.1) and opens the session (§3.2) under anonymous key; runs its lookups as
  §4.1 says of last hop, with its limits and refusals, storing with `signer`. Its own Kad requests
  then look like those it runs for owners.
- **Inwards**, `ROUTER` only (a leaf hop is never a gate, §2.5). A leaf setting `HOP` has no gate
  at 0 inwards (§2.1) and so shares nothing under its anonymous key; client tells its user so. A
  `ROUTER` picks a gate value for itself
  as a last hop does on `HopGate` (§3.1) and lists itself with that value as the one gate of its
  `GATE` record (§4.3). A `HopJoin` with that value makes node itself the other end. The limits of
  §3.1 apply as to any gate.
- **Hold.** A peer can time a session against a direct connection to the node; a real last hop or
  gate adds at least one link. So node holds back every byte it sends in a 0-hop session, and the
  status frame of a `HopJoin` to its own gate, by a delay drawn once per session from round-trip
  times it measures to its `HOP` neighbours. Costs latency, not rate.
- **Left over** (§6): the key keeps one last hop and one gate for as long as it uses 0 hops, where
  an owner's change with its paths and its `GATE` record lists up to 4; whoever watches the node's
  links sees no path carrying its sessions.

---

## 3. Sessions

A **session** is what two nodes have when one or both is behind a path. It gives what a
connection gives (`Protocol.md` §3, T1, T2 and T4): each side has proof of the other's key,
everything is encrypted end to end, and either side opens exchanges. No address is proven or
needed: an answer goes back the way the request came.

### 3.1 Reaching a node, and gates

```protobuf
message HopConnect {          // type 33. First frame on a stream, read by the last hop
  bytes node = 1;             // compact node, 22 or 34 B
  bytes gate = 2;             // 8 B; absent = the node itself is meant
  bytes relays4 = 3;          // n × 22 B compact nodes: relays of `node`, from its Peer record; empty = none
  bytes relays6 = 4;          // n × 34 B; likewise
}

message HopJoin {             // type 34. First frame of an exchange from outside a path
  bytes gate = 1;             // 8 B; absent = this node itself is meant
}

message HopGate {             // type 29. Request and answer on the control stream
  bytes gate = 1;             // answer: 8 B, random, chosen by the last hop
}
```

**Outwards.** Owner opens a stream and sends `HopConnect`. Last hop connects to `node` (an address
as §2.2 asks; else `Error{MALFORMED}`), opens an exchange, sends `HopJoin` with the same `gate`,
and copies both ways. Dials caused by builds (§2.2) and `HopConnect` are limited per path, per
neighbour, per neighbour host and per destination host (§5). A `HopConnect` to a `node` the last
hop already has a connection with in that family is no dial, as an already-connected relay is
none (below). A node at 0 hops sends `HopJoin` itself (§2.6).

- **A node that is not open**, in `HopConnect` and as `next` of a build record (§2.3). Owner copies
  `relays4` and `relays6` from `node`'s `Peer` record, or for a leaf hop from its hop list (§2.5),
  ≤ 2 *(initial)* each (`Protocol.md` §10.3); more, or a length not a multiple of 22 or 34 B, is
  `Error{MALFORMED}` (in a record: `status` `MALFORMED`). When `node` has relays in a family, last hop reaches it there as any node
  reaches a leaf: sends `Punch` through one of them and dials (`Protocol.md` §9.4, §9.5). A failed
  punch is a failed dial (table below). One `HopConnect` or `next` counts as one dial for §5, whether `node`
  is dialled directly or via a punch; each further punch attempt counts one more. Each relay is an
  address as §2.2 asks of `next`; else `Error{MALFORMED}`. A relay dial counts only in the row per
  destination host, at the relay's host; a relay last hop is already connected to is no dial.
  A punch the relay refuses (`Error{NOT_FOUND}`, `Error{UNSUPPORTED}` or `Error{DENIED}` on the
  `Punch` exchange; `Error{BUSY}` is no refusal) counts as a failed dial of the asking neighbour
  host, not of the destination host (§5).

**Inwards.** An owner wanting to be reached sends `HopGate` on a path's control stream. Last hop
answers with a gate value and is from then on a **gate** of owner. A second `HopGate` on a path
gets the same gate value. A path is never extended (§2.2), so the gate stays its last hop.

- A node receiving `HopJoin` with a gate value it gave opens a stream with an odd number on that
  path and copies. Unknown value → `Error{NOT_FOUND}`.
- A node receiving `HopJoin` without a gate value, and setting `ANON`, is itself the other end of
  session. A node not setting `ANON` answers `Error{UNSUPPORTED}` as status frame.
- Owner publishes its gates in a `GATE` record (§4.3).
- A gate accepts ≤ 10 *(initial)* `HopJoin` per minute from one host and answers `Error{BUSY}`
  above that, always with `retry_after` above 0. A gate whose path has no room for a further
  stream (§2.4) answers the same `BUSY`. `retry_after` value is local policy.

**Status frame.** As after `HopStream` (§2.4), first thing in the other direction after
`HopConnect` and after `HopJoin` is one frame, `HopOk` or an `Error`, before any raw byte. Both
are, like `HopStream`, followed by raw bytes.

| After | Sent by | `HopOk` when | `Error` |
|---|---|---|---|
| `HopStream` | neighbour that received it, on that link, without a layer | it accepted stream for itself | §2.4 |
| `HopConnect` | last hop, on stream towards owner. The frame is in the path's layers, as every byte of that stream | `node` answered its `HopJoin` with `HopOk` | `NOT_FOUND` when `node` can't be reached within 10 s *(initial)*, 20 s *(initial)* when it punches; `MALFORMED` for a `node` §2.2 refuses; `DENIED` for a `node` in its own site (§2.1); `BUSY` without `retry_after` above a dial limit (§5); else the `Error` `node` gave, unchanged |
| `HopJoin` | node that received it, without a layer | with a gate value: its path neighbour answered the `HopStream` with `HopOk`. Without one: at once. At 0 hops held as §2.6 says | `NOT_FOUND` for an unknown gate value; `BUSY` with `retry_after` above 0 above the limit; a failed stream on path is a reset |

Last hop reads the status frame of the `HopJoin` it sent and doesn't copy it: owner gets the one of
`HopConnect`. After `HopOk` the two ends talk; hops and gates in between copy bytes.

### 3.2 Opening a session

Opening side is A, the other B. The first stream of a session starts with

```
A → B   0x01 ‖ e_A(32)
B → A   e_B(32) ‖ seal(k_b, key of B(32) ‖ sig_B(64))
A → B   seal(k_a, key of A(32) ‖ sig_A(64))
```

```
ss              = X25519(a, e_B) = X25519(b, e_A)          fresh pairs (a, e_A) and (b, e_B); refuse all zero
k_b ‖ k_a ‖ k_s = H("ed3k v1 hop session", ss ‖ e_A ‖ e_B)                      96 bytes
sig_B           = signature over "ed3k v1 hop session sig" ‖ 0x02 ‖ e_A ‖ e_B
sig_A           = signature over "ed3k v1 hop session sig" ‖ 0x01 ‖ e_A ‖ e_B ‖ key of B
sid             = H("ed3k v1 hop session id", k_s)[0..16)
seal(k, m)      = ChaCha20-Poly1305(k, nonce 0, m, empty AAD)                    RFC 8439, 16-byte tag
```

- Key = anonymous key of a hiding node, node key of a non-hiding one.
- `sig_B` and `sig_A` are Ed25519 signatures by the key the same message carries. `nonce 0` is 12
  zero bytes; each key seals one message only.
- A MUST compute node ID from B's key and compare it with node it meant to reach; on
  mismatch it closes stream.
- B learns A's key only inside the seal. A hop or gate between them sees neither key.
- A side without the next handshake message 10 s *(initial)* after sending its own resets the
  stream.
- A session ends 10 min *(initial)* after its last stream closed; one with an open stream doesn't
  end.

**Further streams** of a session start with

```
0x02 ‖ sid(16) ‖ number(8, big-endian)        number: even when A opens the stream, odd when B does; never used twice
```

and need no signature. They MAY use another path than the first. **A session therefore outlives a
path**: when a path breaks its streams end, and session goes on over the next path.

- Each side MUST use a number greater than every number it used before in session.
- Receiver resets a stream whose number it already saw from that side in session, whose number
  isn't above the highest seen from that side minus 1,024 *(initial)*, or whose number has the
  parity of the receiver's own side. A gate or last hop sending a stream a second time is refused
  so.
- A stream with unknown `sid`, or first byte neither 0x01 nor 0x02, is reset. Either side, as
  opener of a further stream, drops session when that stream is reset before a record came
  back on it, and opens a new one with the three messages above. A reset carries no reason
  (`Protocol.md` §5.2), so opener does the same whether the `sid` was unknown, the number seen
  or path broken.

### 3.3 What a stream of a session carries

```
key     = H("ed3k v1 hop session stream", k_s ‖ number(8, big-endian) ‖ dir(1))     dir = 0x00 from A, 0x01 from B
record  = len(2) ‖ ChaCha20-Poly1305(key, nonce = number of the record(12, big-endian), plaintext of at most 16,384 bytes, empty AAD)
```

`len` = ciphertext length incl. 16-byte tag, big-endian, 16 to 16,400. Records numbered from 0
per stream and direction; the number is the nonce, not sent. The first stream has number 0, its
records following the three messages of §3.2.

- Record with empty plaintext (`len` = 16) ends a direction, authenticated like any record. A
  direction ending without it was cut; receiver treats exchange as reset. Anything after the
  empty record ends stream, and exchange counts as reset.
- Record with `len` below 16 or above 16,400, or failing its check, ends stream.
- A session has no observed address, so a `Store` with `source` over it is answered `REJECTED`
  (`Protocol.md` §10.3), its `also` targets too; so is `sealed` (friends under an anonymous key:
  §8). A hiding node publishes `ANON` and `GATE` records through a `HopLookup` (§4).
- Nor has a session an address family. A `KadRequest` over it is answered as on a connection in a
  family where node adopted no endpoint (`Protocol.md` §10.3): no contacts and not
  `responsible`, so only an index node answers with records. Limits counting a host address count
  the host of node that sent the `HopJoin`, per gate; limits count per node ID as well. A
  session node opened itself counts at the address its last hop dialled: the endpoint of
  `node` in its `HopConnect`, or the gate's `Gate.node`. A session via one of the node's own
  gates counts under that gate, as one host, site and block. A hiding node uses `HopLookup` for Kad (§4.1).
- Where `Protocol.md` §5.5 closes a connection with `PROTOCOL`, a side ends session: resets
  the session's other streams, except a direction whose empty record it already sent, reads none
  of them further, and forgets `sid`. Records aren't acknowledged end to end, so a responder
  answering a malformed `Hello` with `Error{MALFORMED}` ends session once it sent the `Error`
  and the empty record; it doesn't reset that stream, so hops still deliver both (§2.4). After a
  session ended so, by either side, its opener opens no other session to that node ID for 60 s
  *(initial)*.
- **One stream is one exchange** (`Protocol.md` §5.2). Records carry its frames; every ed3k
  message works unchanged.
- First exchange is `Hello`, on the first stream: A sends, B answers, as in `Protocol.md` §5.5. A
  node under its anonymous key sends no `observed`, `ep4`, `ep6` and no `nat`, an empty `nick`,
  and as `agent`, `caps` and `dict` the values its implementation sends for every hidden node;
  `caps` never holds `ROUTER`, `RELAY` or `HOP` (§1). An implementation having an extension sets
  its bit there, also when user turned it off: a node under its anonymous key taking no part
  answers as a node taking part in nothing (`TOPIC_LEFT`, `LIVE_LEFT`, `ROOM_LEFT`, no `credit`
  field), not `Error{UNSUPPORTED}`. A `Peer` naming it carries its node ID alone, and its
  `FileRequest`s carry no `relays4` or `relays6`. Anything else would tie the two keys. A receiver
  setting `ANON` treats a `Peer` with a node ID alone as an anonymous source and looks up its
  `GATE` record under the limits of §4.3; any other receiver drops it.
- **Source exchange.** A bare node ID names only a node met under an anonymous key, i.e. one that
  sent a `Hello` of the form above. A node reached at an address is passed on with that address. A
  peer met over a session that is neither is left out of source exchange.
- A node MUST limit new sessions per node ID to the node-ID rate of `Protocol.md` §4.6, and per
  gate or per node that sent the `HopJoin` to its host-address rate. Nothing is banned: a session
  over a limit is refused by resetting its first stream; opener opens no other session to that
  node ID for 60 s *(initial)*. B knows A's key only after the third message (§3.2), so checks the
  node-ID rate then and resets the first stream at that point when over. A first stream reset
  before the first record from B came back on it counts as such a refusal. A wrong key (A's node-ID
  check fails, §3.2) is no refusal; it counts as a failed attempt (§4.3). A gate's `Error{BUSY}` is
  no refusal either (§4.3).
- For a hiding A, B can open a stream to A only through a gate of A. An uploader offering a slot
  later (`Protocol.md` §12.5) looks up the downloader's `GATE` record. To a non-hiding A, B opens
  through a path of its own with `HopConnect`, `gate` absent, to an endpoint and relays from A's
  `Peer` record, else from the `Hello` A sent in session. Those are then claims
  (`Protocol.md` §8.2 (5)), dialled within the limits of §5 and
  [`ip-diversity.md`](ip-diversity.md) §2.5. Such an attempt counts there as pending until B has
  checked A's key (§3.2), as in §4.3; a wrong key is a failed attempt.

---

## 4. Kad for a node that hides

### 4.1 Lookups by the last hop

A lookup connects to 8 to 12 nodes; through three hops each would cost seconds. Owner hands the
lookup to a path's last hop.

```protobuf
message HopLookup {           // type 30. First frame on a stream, read by the last hop
  KadRequest request = 1;
}

message HopLookupAnswer {     // type 31
  repeated KadAnswer answers = 1;   // one per closest-set member that said responsible (Protocol.md §10.4), closest first; the page of request.offset
}
```

- Last hop runs the lookup of `Protocol.md` §10.4 with the request, under its own key and with its
  own address families; the owner's `want` doesn't steer the walk. It asks no index node. It
  answers when the lookup ends (`Protocol.md` §10.4), at the latest 60 s *(initial)* after the
  `HopLookup`, with what it has, possibly no answers. Never `Error{NOT_FOUND}` for a lookup that
  found nothing.
- `answers` holds one `KadAnswer` per closest-set member that said `responsible`, closest first:
  the page the request's `offset` asks for. Owner asks for further pages by resending the request
  with the next `offset`; each node pages in its own order (`Protocol.md` §10.3). Answers are left
  out from the far end so the frame fits 1 MiB.
- **Paging across nodes.** An answer doesn't say which node gave it, and last hop walks again
  per page, so owner pages over records, not nodes. Next `offset` = request's `offset` plus the
  smallest record count among answers setting `more`; an answer with `more` and no record is
  ignored. Owner merges records of all pages as a searcher merges those of several nodes
  (`Protocol.md` §10.3): by publisher and file, sources by node ID. A record seen on several pages
  keeps the per-answer values (`load`, `spill`, `sites`) of the first page holding it. An answer
  whose records were all seen on earlier pages adds none but still counts for `more` and the next
  `offset`. Paging ends when no answer of a page sets `more`.
- Owner waits 70 s *(initial)* for the `HopLookupAnswer`, then resets stream.
- It answers `Error{DENIED}` to a `HopLookup` whose `store` carries `source`, `also`, `http`,
  `links` or `sealed`: those records are bound to connection (`Protocol.md` §10.3) and would
  name last hop as source. Likewise refuses a `store` with a signed record and no `signer`,
  which would be counted under its own key. Searches of any kind go over a path; which stores do
  follows from these refusals: every other store passes, e.g. `HEAD`, `HIDDEN`, `PART` and
  `MEMBER` with `signer`.
- It answers `Error{DENIED}` to a `HopLookup` of kind `NONE`. A node under its anonymous key keeps
  no routing table (§1) and takes its hops from its own lookups (§2.1).
- It allows 20 *(initial)* lookups per minute and path; `Error{BUSY}` above that.
- **Last hop sees what is searched and stored. It does not see by whom.** An owner SHOULD
  spread its lookups over its paths.
- Last hop can omit or invent answers. Owner checks signed records as always; a false source costs
  a failed session.

### 4.2 Storing under another key

A storing node takes a record's key from connection (`Protocol.md` §10.7). A record a last hop
stores for someone else is signed with a key other than the connection's.

- A `Store` of any signed kind MAY carry `signer` (`Protocol.md` §10.7). Storing node then checks
  signature under `signer` and counts the record for that key.
- Its per-address limits ([`ip-diversity.md`](ip-diversity.md) §2.4) count the request's source
  address. For `ANON` records: 4 per host and 8 per site per target *(initial)*; a record beyond a
  limit → `REJECTED`. Many nodes storing through one last hop share its allowance, so an owner uses
  several paths.

### 4.3 The two records

```protobuf
// in enum Kind (§10.3 of Protocol.md):
ANON = 11;
GATE = 12;

// in Record (§10.7 there):
Gates gates = 10;

message Gates {
  uint64 issued = 1;          // Unix time; the later record replaces the earlier
  repeated Gate gates = 2;    // 1 to 4
}

message Gate {
  bytes node = 1;             // compact node, 22 or 34 B
  bytes gate = 2;             // 8 B
}

// in Store:
Signed anon = 10;             // ANON or GATE; MUST carry `signer`, else REJECTED

// in KadAnswer:
bytes  anon = 16;             // SOURCE search: n × 16 B node IDs of anonymous sources
Signed gate = 17;             // GATE search
```

| | `ANON`: an anonymous source of a file | `GATE`: where an anonymous node is reached |
|---|---|---|
| Target | `root[0..16)`, as for `SOURCE`; `topic`, a room's target or `stream` in place of a `SUB` or `LIVE` record ([`pubsub.md`](pubsub.md) §5, [`room.md`](room.md) §5, [`live.md`](live.md) §5.1) | `H("ed3k v1 gate", node ID)[0..16)` |
| Signed by | anonymous key | anonymous key; the request's target MUST follow from `signer` |
| Content | `expires` and nothing else | `expires` and `gates` |
| Lifetime | 5 h *(initial)*, as `SOURCE` | 1 h *(initial)* |
| Count | one per key and target; ≤ 64 *(initial)* per target, further one answered `FULL`; ≤ 4 per host and 8 per site *(initial)* (§4.2), further one answered `REJECTED`. Address limits checked first: a record over them is `REJECTED` before target is looked at ([`ip-diversity.md`](ip-diversity.md) §2.4) | one per target; replaced by a later `issued`, else answered `OLDER` |
| Returned | in `anon`, paged after `sources` in a `SOURCE` search at that target (`Protocol.md` §10.3) | in `gate`, with the key in `keys` |

**Checks**, in addition to those of `Protocol.md` §10.7

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| `signer` | `Store.anon` carries `signer`, signature valid under it | `gate` valid under `keys[key]`, and that key gives the looked-up node ID |
| Target | `GATE`: follows from `signer` | the same |
| Content | `ANON`: `expires` and nothing else. `GATE`: `expires` and `gates`, nothing else | the same for `gate` |
| `gates` | 1 to 4 entries; else `REJECTED` | the same |
| `Gate.node` | 22 or 34 B, an address §2.2 accepts as `node` | the same |
| `Gate.gate` | 8 B | the same |

A record failing a check is answered `REJECTED` by storing node and dropped by searcher. A
`Store` of `ANON` or `GATE` without `signer` is `REJECTED`, per record.

A `GATE` search returns the target's one record in `gate`. An `ANON` search is answered as a
`SOURCE` search at that target: `ANON` records come in `anon`, after `sources`
(`Protocol.md` §10.3). Of a `Query`, both use `offset` only.

- **Two steps on purpose.** Gates change whenever a path breaks; a node with 10,000 files then
  re-stores one `GATE` record, and its 10,000 `ANON` records stay as they are.
- `anon` holds node IDs without key or signature. As for any source, reaching it is proof: the
  session shows the key behind the ID.
- A searcher wanting an anonymous source looks up its `GATE` record, checks the key gives node
  ID, picks a gate at random, and opens a session through it. ≤ 4 *(initial)* failed `GATE`
  lookups or session attempts per file and hour, as [`ip-diversity.md`](ip-diversity.md) §2.5
  limits failed source dials: `anon` is unsigned. A topic, a live stream and a room
  ([`room.md`](room.md) §5) each count as a file for this limit, and a dialled gate counts there
  too, in the per-file row and the all-files row. One session attempt = one gate dialled. An
  attempt counts until searcher, as A, has checked B's key (§3.2); one still pending counts as
  failed.
- A gate's `Error{BUSY}` on a `HopJoin` is no failed attempt. A gate always sends it with
  `retry_after` above 0 (§3.1), and that gate waits `retry_after`; other gates of node MAY be
  tried meanwhile. A `BUSY` without `retry_after` answering `HopConnect` comes from last hop
  at its own dial limits (§5): no failed attempt, no wait on the gate; owner retries over another
  path. How many paths it tries, and how long it avoids a busy one, is local policy. A `BUSY`
  without `retry_after` on a `HopJoin` a node sent itself (§3.1, 0 hops) breaks §3.1; that gate is
  waited for 60 s, as `Protocol.md` §5.6 says where no section names a wait. The 60 s waits per
  node ID of §3.3 are for a reset of the first stream, the node's own refusal, and for a session
  ended with `PROTOCOL`.
- A node stores its `GATE` record before its first `ANON` record. No `GATE` record while it has no
  gate: `gates` is never empty. While gateless it stores no `ANON` record either; stored ones run
  out.
- Owner re-stores its `GATE` record after 45 min *(initial)*, and at once when a gate was lost.
- The batch of `Protocol.md` §10.4 doesn't carry `ANON` records: each is signed for its target. A
  node with very many files pays one lookup per file (§8).

---

## 5. Being a hop

A hop spends its own upload on strangers' data and decides how much.

**Limits a hop sets** *(all initial)*

| | |
|---|---|
| Paths from one neighbour | 8 |
| Paths in total, and the share of its upload | user's choice. A client SHOULD offer the share as a setting and start with a quarter |
| Streams per path | 64 |
| Exchanges on connection to one neighbour | 64 opened by peer (`Protocol.md` §4.1), plus 65 per path between them, in both directions: its 64 streams and control stream. Hop raises it on sending `HopCreated`, neighbour that sent `HopCreate` on reading it. When path ends each lowers it again by granting no new exchange until the peer's open ones are below the limit |
| Held per stream and direction | 256 KiB |
| Lookups per path | 20 per minute |
| Dials (`next` of a build, `HopConnect`) per path / per neighbour; a `node` already connected is none (§3.1) | 4 / 16 per minute |
| Dials caused by a build or `HopConnect` whose QUIC handshake didn't complete within 10 s, or still pending, relay dials and punches included, per destination host (IPv4 /32, IPv6 /64), over all paths and neighbours | 8 per hour |
| Paths and dials per neighbour host address | 16 paths; 32 dials per minute |
| Failed dials per neighbour host address: a wrong node ID, or a punch the relay refused | 8 per hour |
| `HopJoin` per gate and host | 10 per minute |

A path counts for neighbour that sent its `HopCreate`, and that neighbour's host, from that
`HopCreate` until path ends, whatever its state, also while being built.

A dial whose QUIC handshake completed within those 10 s doesn't count in the per-destination-host
row, whatever node ID answered. A hop gives up a dial after 10 s (§2.2); it then counts for the
hour. A wrong node ID, and a punch the relay refused, count as failed dials of the asking
neighbour host; above its limit that neighbour host gets `BUSY`.

Above a limit it answers `Error{BUSY}`. A hop taking no new paths clears `HOP` in its `Hello`.

**Give and take.** Per neighbour a hop counts two numbers: bytes it forwarded on paths this
neighbour asked it for with `HopCreate`, and bytes this neighbour forwarded on paths hop asked
*it* for.

- Every neighbour gets a **base rate**, 16 KiB/s *(initial)*, shared over its paths. A hop whose
  share can't give every neighbour its base rate takes no new paths.
- What hop gives paths beyond base rates it SHOULD divide in proportion to what each
  neighbour forwarded for it in the last 7 days *(initial)*.
- Unearned rate MAY go to anyone.
- A hop slows a path by reading less (§2.4); never drops data.

The count needs only what two neighbours see of each other, so it unmasks nobody. It also reaches
only one link: a middle hop is charged by the next hop for data not its own, and must earn that by
forwarding. A node that can't be a hop, a leaf behind a `symmetric` or `unknown` NAT (§2.5), gets the
base rate and what is free.

**What a hop is not.** A hop never sends anything out of ed3k. It connects to ed3k nodes and speaks
`HopCreate`, `HopJoin` and Kad to them; can't be used to reach a web server or any other service.

---

## 6. What it does and does not give

| | |
|---|---|
| A peer, or a storing node | sees a gate or a last hop instead of the address |
| A single hop | sees its two neighbours and how much flows. Can't read it. Learns from the build (§2.2) only whether it is last hop; a last hop also sees where a stream goes, and a gate. Its place otherwise follows only from the rows below |
| A hop asked by a node not setting `HOP` | knows that node is the owner: only an owner or a hop sends `HopCreate`. A hiding node that can be a hop sets `HOP` by default (§1); left are leaves behind a `symmetric` or `unknown` NAT (§2.5) |
| A hop that isn't last, asked by a leaf hop | knows the leaf is the owner: a leaf is never first hop (§2.5), and a path has ≤ 3 hops |
| An entry hop | sees one neighbour build all its paths through it for 30 days (§2.1). A neighbour that builds often but forwards little for others is likely an owner. Tor's guards know their clients the same way |
| A hop timing the build | the answer comes back after the rest of the path and last hop's random hold of up to 1 s (§2.2). One build tells little; over many, the shortest waits still hint at hops after it, most to an entry hop, which sees an owner's builds for 30 days |
| A last hop | reads the lookups it runs: targets, search words, records stored. Not for whom |
| A last hop, or a gate | reads `sid` on every further stream of a session (§3.2), so it can tell which paths carry one session and link their last hops |
| The first and last hop of a path together | match a path by timing and volume. Limit of every design of this kind, and file sharing makes it easy: flows are large and long |
| Whoever runs many hops | is picked as first and last more often. Hops are found via Kad, where nodes are cheap; no vouched-for list. Entry hops and the site rule make this slower, not impossible |
| Leaf hops | cheaper to run than `ROUTER` hops: no open address needed. Bounded by their relays (≤ 2 reservations per host, 4 per site, `Protocol.md` §9.4), one leaf per answer, half the hop list, and never first hop or gate (§2.5) |
| Someone who watches the links of a node | sees that it takes part, and how much. Sizes and times not hidden; no padding, no delay |
| Someone who watches for a long time | learns from which files appear and vanish together with which node being online |
| The crowd | is the nodes using paths. A small network hides little |
| A gate | is known to everyone and can be flooded. Owner replaces it |
| Anonymous key, taken later | names the node's past records. Path and session keys are gone |
| A hop's node key, taken later | with a recorded build, shows that hop's next hop and relays (§2.3). Layer and session keys are gone |
| At 0 hops (§2.6) | a peer sees a last hop or gate and, from one session, bytes or timing, can't tell it from an owner behind one. Over time: the key keeps one last hop and one gate, an owner's change with its paths; one gate in its `GATE` record where an owner lists up to 4; no path feeding its sessions to whoever watches its links. Deniable, not hidden |
| With 1 hop | hop knows everything. It's a relay for a pair that can't be hole-punched (`Protocol.md` §9.4), not anonymity |
| Speed | each hop re-uploads every byte. A path is as fast as its slowest hop is willing to be |

---

## 7. NeoShare, and alternatives considered

NeoShare was read in source (`NeoKad/Kad/KadRouting`, `NeoLoader/FileTransfer/NeoShare`); nothing
was run. Broader comparison: [`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md) §8. Tribler's onion routing: [`docs/tribler-comparison.md`](../docs/tribler-comparison.md).

| | NeoShare | This extension |
|---|---|---|
| The setting | levels 0 to 4. In the code levels 0 to 2 give the same direct route; 3 and 4 add 1 and 2 random nodes | 0 to 3 hops per direction |
| Who picks the nodes | each relay picks its next ones, towards a random DHT area; route is a tree with several branches | owner picks every hop |
| What a relay sees | IDs of both ends, a frame counter, offsets, and the whole handshake | its neighbours and the volume |
| Encryption | end to end; none per relay beyond link | end to end, plus one layer per hop |
| Receiver | registers with the nodes near a random ID | publishes gates in a signed record |
| Transport | 4 KB frames, acknowledged by each hop and end to end, with own windows and timers | QUIC exchanges on every link; a session of records on top |
| A slow relay | routed around: next hop by measured delay and loss | holds its path; owner measures paths and replaces the slowest |
| Queue at a relay | unlimited | 256 KiB per stream and direction |
| Limits | 100 relayed operations in total; no per-peer limit, no quota, can't be turned off | a capability; per-neighbour limits; a user-set share |
| Reward | none | give and take between neighbours |
| Kad through others | lookups and stores handed to a proxy that sees the requester | handed to last hop, which doesn't |

**Decided**

| Choice | Pro | Con |
|---|---|---|
| **Owner picks, with layers**, in place of relays that pick | no hop sees both ends; owner spreads hops over networks and sets their number | owner needs a hop list; a path losing a hop is gone |
| **Reliable streams on every link**, in place of unreliable packets with a QUIC connection inside | needs nothing version 1 lacks; loss repaired where it happens; a full hop slows sender instead of dropping | one slow hop holds its path; a hop keeps per-stream state; a session is a second secure channel to get right |
| **Each side its own half**, in place of a meeting node | hop count is each node's own choice and cost; one hop also serves unpunchable pairs | up to 6 hops between two hidden nodes; one more record and lookup per hidden source |
| **Path built in one message**, in place of one hop at a time (Tor's extend) | every hop sees one build of one size; slots in random order: no hop but the last learns its place from it, an owner from its neighbour's view no more than a hop. One round trip. Length capped at 3 by the format | record readable later by whoever takes a hop's node key (§2.3); one failed hop fails the build; a hop keeps seen `e` against replays |
| **Deniable 0 hops**, in place of 0 = node key | a node paying for no hop still can't be named from one session: it looks like a last hop or gate; its own lookups like those it runs for owners | needs `HOP`; one last hop and gate for the key over time; held bytes cost latency |
| **Voluntary, give and take**, in place of a duty | nobody carries strangers' data against its will | capacity depends on users turning it on |
| **Leaves behind a cone NAT as hops**, in place of `ROUTER`s only | NATed nodes add capacity and earn give and take instead of only using it; Tribler lets every punchable peer relay ([`docs/tribler-comparison.md`](../docs/tribler-comparison.md) §4.2) | one punch per leaf hop of a build; a path dies with a leaf's mapping; leaves are found through their relays' listings |

**Alternatives**

| Alternative | Pro | Con |
|---|---|---|
| **A binding over Tor or I2P** | large crowd, years of study | a dependency; streams only for Tor, which asks not to be used for bulk transfer; a separate network |
| **One exchange for all streams of a path**, with a multiplexer | fewer exchanges per link | one lost packet holds every stream; flow control must be rewritten |
| **A tag on every layer** | a changed byte is noticed at the next hop | 16 bytes per record and hop, and records instead of a running stream |
| **Lookups walked by owner**, one session per node | last hop reads less | 8 to 12 sessions through path per lookup |
| **Gates in every source record**, with no `GATE` record | one lookup less | every source record re-stored when a gate changes |
| **Signed entries in `anon`** | a false entry is seen without a session | ~100 bytes per source instead of 16 |
| **One hop at a time, with dummy extends** | record forward secret; a failed hop replaced in place | an entry hop sees the same owner for 30 days and learns its place from the average count; slower |
| **A new first hop for every path** | no hop sees a node for long | a hostile first and last hop at once becomes a matter of time |
| **Padding and delays** | sizes and times say less | throughput, which this extension is for |

---

## 8. Open points

- **Review of the cryptography** (§2.3, §3.2, §3.3). Key agreement follows the pattern Tor uses
  for its hops, the session a signed key exchange, but the text here is new.
- Layers have no tag, and the frames a last hop reads (`HopConnect`, `HopLookup`) can be changed by
  an earlier hop guessing their content. A build is sealed per hop, a session safe by the node-ID
  check. A lookup is not.
- **The build record is not forward secret** (§2.3): sealed under the hop's node key alone. Forward
  secrecy for it needs the hop's fresh key first, i.e. one hop at a time, which a hop can count.
  To revisit with the review.
- Where a node gets hops more trustworthy than a Kad sample: signed lists, as for first contact
  ([`bootstrap.md`](bootstrap.md)), or the key puzzle ([`key-puzzle.md`](key-puzzle.md)).
- Anonymous keys cost nothing, and a storing node's limits count last hop. The key puzzle
  would give a key a price.
- `ANON` records for very many files: one lookup and one signature per file. The batch form of a
  signature (`Protocol.md` §10.7) would allow a batch as for `SOURCE`.
- Capability `ANON` (bit 14, accepting sessions through a path, §1) stays optional, also for a
  `ROUTER`; storing and returning record kinds `ANON` and `GATE` does not (above,
  `Protocol.md` §10.3). A hiding node can only use sources setting the capability. Making it a duty
  would put session of §3 into the core. To revisit once it's known how many nodes set it.
- Give and take reaches one link and starts at nothing between two nodes that never met.
- **Leaf hops** (§2.5): never first hop or gate for now. To revisit with measured punch success
  and path lifetime through leaf hops; their share of the hop list (half) is a first value.
- Friends under an anonymous key not looked at.
- All numbers are first values.
