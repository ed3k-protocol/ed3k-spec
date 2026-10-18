# Extension: anonymous paths (`hops`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1. **The key agreement and the ciphers of §2.3 and §3.2 are a first design and need a review by people who do this for a living before anyone relies on them.**
**Extends:** [`Protocol.md`](../Protocol.md) §2, §5.1, §9.4, §10.1, §10.3 and §10.7. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 14 `ANON`, bit 15 `HOP`. Message types 26 to 34 and 39. Record kinds 11 and 12. One field in `Record`, one in `Store`, two in `KadAnswer`.
**Required of every `ROUTER`:** storing and returning the record kinds `ANON` and `GATE`, and accepting `signer` in a `Store` (§4). The rest is optional.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Hop keys* and *Hop session*.

In ed3k every peer sees the address of the peer it talks to, and every storing node sees who
stores and who searches. This extension lets a node put **0 to 3 other nodes** between itself
and the rest. The number is the choice of each node, for itself.

- A node that hides builds **paths**. It picks every node of a path itself, and wraps what it
  sends in one layer of encryption per node. A node on a path, a **hop**, knows the node before
  it and the node after it and nothing else.
- **Each side hides itself.** A downloader builds a path outwards. A source that hides builds a
  path inwards and publishes its far end, the **gate**, in place of an address. Between two
  hidden nodes there are the hops of both.
- A hop forwards **reliable streams**. Each link of a path is an exchange of the connection
  between two neighbours, so loss is repaired on the link where it happens, and a hop that
  cannot pass data on stops reading.
- Being a hop is **voluntary**. A hop gives a small rate to everyone and more to the
  neighbours that forward for it.

```
 downloader ── hop ── hop ── hop ─────── gate ── hop ── hop ── source
 |<------ path of the downloader ------>|<----- path of the source ----->|
 |<----------------------------- session ------------------------------->|
```

NeoLoader's NeoShare was the starting point: a number of hops that the user chooses. What was
kept and what was changed is in §7.

---

## 1. Roles

| Bit | Name | Meaning |
|---|---|---|
| 14 | `ANON` | The node accepts sessions that arrive through a path (§3) and understands anonymous sources (§4) |
| 15 | `HOP` | The node forwards for the paths of others (§2, §5). Only a `ROUTER` sets it, and a node that sets it sets `ANON` |

**The anonymous key.** A node that hides has a second Ed25519 key pair, the *anonymous key*,
with a node ID of its own (`Protocol.md` §6.1). Everything it does through a path, it does
under that key.

- The anonymous key MUST NOT be the node key, and nothing may be signed with both.
- A node under its anonymous key is a leaf. It has no address, so it has no routing ID, is in
  no routing table, and never sets `ROUTER`, `RELAY` or `HOP`.
- A node MAY take a new anonymous key at any time. It then loses its places in queues and what
  uploaders remember of it.

**One machine, two nodes.** The same machine MAY be a `ROUTER` and a hop under its node key
and share and download under its anonymous key. That is the recommended way to run it: the
traffic of others covers its own.

- A file MUST NOT be offered under both keys. Two source records for the same rare files
  connect the keys.
- What is done under the anonymous key MUST go through a path: no direct connection, no HTTP
  source ([`http-source.md`](http-source.md)), no eD2K (`Protocol.md` §14), no answer in the
  local network ([`lan-discovery.md`](lan-discovery.md)).
- A client MUST show its user which files are shared under which key, and MUST ask before it
  moves a file from one to the other.

**How many hops.** A user setting, 0 to 3, for each direction. 0 is ed3k as it is.

| Hops | A single hop knows | It takes to connect address and deed |
|---|---|---|
| 0 | | the peer sees it |
| 1 | both ends | that one hop |
| 2 | one end each | both hops together |
| 3 | one end, or neither | the first and the last together |

---

## 2. Paths

A path has an **owner** and one to three hops. The owner knows all of them. A hop knows its
neighbours. The hop furthest from the owner is the **last hop**.

### 2.1 Choosing hops

- The owner keeps a list of nodes that set `HOP`, with their keys. It fills it from the nodes
  it meets in Kad lookups for random targets and SHOULD hold at least 20 *(initial)* before
  it builds a path.
- The hops of one path, the owner, and the node a stream goes to MUST be in different **sites**
  and SHOULD be in different **blocks** ([`ip-diversity.md`](ip-diversity.md) §1).
- **Entry hops.** The owner picks 3 *(initial)* hops and uses one of them as the first hop of
  every path, for 30 days *(initial)*. With a new first hop for every path an owner would,
  sooner or later, pick a hostile first and a hostile last hop at once. Tor does this for the
  same reason.
- The other hops are picked at random for each path.
- A node keeps up to 4 *(initial)* paths outwards and up to 4 gates (§3.1). It measures the
  rate of each path and replaces the slowest when it is far below the others.

### 2.2 Building

A path is built one hop at a time. The owner asks the first hop directly. It asks every
further hop through the path it has so far, so that hop never sees the owner.

```protobuf
message HopCreate {           // type 26. First frame of an exchange between two neighbours
  bytes path = 1;             // 8 B, random, chosen by the sender; names the path on this connection only
  bytes e = 2;                // 32 B, X25519 public value made by the owner for this hop alone
}

message HopCreated {          // type 27
  bytes f = 1;                // 32 B, X25519 public value made by the hop for this path
  bytes proof = 2;            // 32 B (§2.3)
}

message HopExtend {           // type 28. From the owner to the last hop, on the control stream
  bytes node = 1;             // compact node, 22 or 34 B: the hop to add
  HopCreate create = 2;       // `path` is left empty; the last hop fills it in
}
```

1. The owner opens an exchange to the first hop and sends `HopCreate`. The hop answers
   `HopCreated`, or `Error{BUSY}`. The exchange stays open. It is the **control stream** of the
   path on this link, and its stream number is 0 (§2.4).
2. To add a hop, the owner sends `HopExtend` on the control stream. The last hop connects to
   `node` as to any node, opens an exchange, sends `HopCreate` with a `path` of its own, and
   passes the `HopCreated` back to the owner as a frame on the control stream, in its own layer
   and those of the hops before it (the new hop has no layer yet). From then on it forwards. A path MUST NOT be extended while
   it carries a stream other than the control stream: a hop added later has no layer on those
   streams. The owner closes them first, or builds a new path.
3. The owner checks `proof` (§2.3). A wrong proof means that the new hop is not the node the
   owner chose; the owner closes the path. The owner knows its first hop by the key of the
   connection (`Protocol.md` §4.1), and checks its `proof` all the same.

- **`node`**, here and in `HopConnect` (§3.1), MUST be a global unicast address on the public
  network (`Protocol.md` §8.2 (7)). A private address is a dial target only in a named
  network (`net_id` not empty, `Protocol.md` §4.3). Any other `node` is answered
  `Error{MALFORMED}` and is not dialled.
- A hop that cannot reach `node` within 10 s *(initial)* answers `Error{NOT_FOUND}`. A last
  hop that has no answer to its `HopCreate` 10 s *(initial)* after it sent it closes that
  exchange and answers `Error{NOT_FOUND}`.
- The owner closes the path when it has no answer 10 s *(initial)* after a `HopCreate` or a
  `HopGate`, or 30 s *(initial)* after a `HopExtend`.
- Everything on the control stream after `HopCreated` is in layers (§2.3).
- **The control stream is a sequence of requests and answers.** The owner sends one request
  frame (`HopExtend`, `HopGate`) and waits for its answer before it sends the next. An `Error`
  on the control stream does not close it: the path stays as it was, and the hop that
  answered is still the last hop. When the new hop answers the `HopCreate` of a `HopExtend`
  with an `Error`, the last hop closes that exchange and passes the `Error` back as the
  answer. An `Error` that answers the first `HopCreate` ends the exchange: there is no path.
- A `HopCreate` whose `path` is not 8 B or is in use on that connection, whichever neighbour
  chose it, or whose `e` is not 32 B, a `HopExtend` whose `create.path` is not empty, and a
  `HopExtend` while the path carries a stream other than the control stream, are answered
  `Error{MALFORMED}`.
- **Keeping a path alive.** Both neighbours of a link of a path send a QUIC PING every 20 s
  *(initial)* while the control stream is open (`Protocol.md` §4.1: `max_idle_timeout` is
  30 s, and whoever wants a connection kept sends the PING). A gate lives as long as its path.
- A path ends when its control stream is closed, or when the connection to a neighbour is
  lost. A hop then closes the control stream and every stream of the path on its other side.
  The owner builds another.

### 2.3 Keys and layers

For each hop the owner makes a fresh X25519 pair `(x, e)`. The hop makes `(y, f)`. The hop's
node key is used for X25519 through the conversion of `Protocol.md` §6.4.

```
ss1   = X25519(x, converted node key of the hop)        the hop computes X25519(its converted private key, e)
ss2   = X25519(x, f)                          the hop computes X25519(y, e)
k     = H("ed3k v1 hop", ss1 ‖ ss2 ‖ e ‖ f ‖ node key of the hop)        32 bytes
proof = H("ed3k v1 hop proof", k)
k_out = H("ed3k v1 hop out", k)               for bytes that move away from the owner
k_in  = H("ed3k v1 hop in", k)                for bytes that move towards the owner
```

- The node key of the hop in the derivation of `k` is the Ed25519 key as it is sent in the
  handshake (32 B), not the converted one. The converted key is used for `ss1` alone.
- Both sides MUST refuse an `ss1` or `ss2` that is all zero.
- `ss1` proves that the hop holds the key the owner chose. `ss2` makes `k` safe when that key
  is taken later. The hop learns nothing about the owner.
- The owner MUST NOT use one `e` for two hops. The hop deletes `y` and `k` when the path ends.

**Layers.** Every stream of a path (§2.4) is encrypted once per hop, in each direction:

```
ChaCha20(key = k_out or k_in of the hop, nonce = 0x00000000 ‖ stream number(8, big-endian), counter from 0)      RFC 8439
```

The key stream runs on over the whole stream in that direction. On the control stream the
stream number is 0, and the layer of a hop starts, with its counter at 0, at the first byte
after that hop's `HopCreated`; on any other stream a layer starts at the first byte after
`HopStream` in the direction of the opener, and at the first byte after the status frame of
the link (§2.4) in the other direction.

- Away from the owner: the owner applies the layer of the last hop first and the layer of the
  first hop last. Each hop removes its own.
- Towards the owner: each hop adds its own. The owner removes all of them.
- **There is no tag on a layer.** A hop that changes bytes cannot be noticed by the next hop.
  It is noticed at the end: a new hop fails its `proof`, and a session checks every record
  (§3.3). A last hop that reads a frame it cannot parse closes the path.
- A stream MUST be closed before 256 GiB have passed in one direction, the limit of the cipher
  for one nonce.

### 2.4 Streams

A stream of a path is one exchange on each link, joined end to end by the hops.

```protobuf
message HopStream {           // type 32. First frame of an exchange between two neighbours
  bytes  path = 1;            // as in HopCreate, for this connection
  uint64 number = 2;          // never 0 (the control stream); the opener counts up, and a number is never used twice on a path. Even: opened by the owner. Odd: opened at the last hop
}

message HopOk {}              // type 39. Status frame: accepted, raw bytes follow
```

- After `HopStream` the exchange carries bytes without frames, in layers, until it is closed.
  It is, like `Block`, `HopConnect` and `HopJoin` (§3.1), a message that is followed by raw
  bytes (`Protocol.md` §5.1).
- **Status frame.** The first thing in the other direction of an exchange that starts with
  `HopStream` is one frame, before any raw byte: `HopOk`, or an `Error`. The neighbour that
  received the `HopStream` sends it, without a layer, as soon as it has accepted or refused
  for itself; it does not wait for the next link. The neighbour that opened the exchange
  reads it and does not pass it on. After `HopOk` come the bytes of the stream, in layers.
  An opener that has no status frame 10 s *(initial)* after `HopStream` or `HopJoin`, or
  30 s *(initial)* after `HopConnect` (§3.1), resets the exchange.
- A hop that receives `HopStream` opens an exchange on the other side of the path with the
  `path` value of that link and the same `number`, and copies in both directions. A
  half-close and a reset are passed on. A hop that receives an `Error` as status frame, or a
  reset, on one side of a stream resets the other side.
- An unknown `path` is answered `Error{NOT_FOUND}`.
- **Numbers never repeat on a path**, so that a layer's key stream is never used twice. Each
  opener MUST use a number greater than every number it used before on the path. The
  exchanges of a link arrive in any order, so the receiver does not ask for that order. A
  hop, and the owner for the streams that reach it, accepts a number that it has not seen
  before on that path from that side, and that is above the highest number it has seen from
  that side minus 1,024 *(initial)*. A `HopStream` with `number = 0`, with a number seen
  before, or with one that is not above that bound is answered `Error{MALFORMED}`.
- **Parity.** An odd number from the owner side, or an even number from the far side, is
  answered `Error{MALFORMED}` as status frame; a node that cannot send one resets the
  exchange. The nonce of a layer rests on it.
- **A hop keeps little.** It holds at most 256 KiB *(initial)* per stream and direction. When
  the next link does not take data, it stops reading from the link before. The flow control of
  the exchanges carries this back to the sender. Nothing is acknowledged twice and nothing is
  dropped.
- A path carries at most 64 *(initial)* streams at a time.
- What the last hop does with a stream is told by its first bytes (§3.1, §4.1).

---

## 3. Sessions

A **session** is what two nodes have with each other when one of them, or both, is behind a
path. It gives what a connection gives (`Protocol.md` §3, T1, T2 and T4): each side has
proof of the other's key, everything is encrypted from end to end, and either side opens
exchanges. No address is proven and none is needed: an answer goes back the way the request
came.

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

**Outwards.** The owner opens a stream and sends `HopConnect`. The last hop connects to
`node` (an address as §2.2 asks; else `Error{MALFORMED}`), opens an exchange, sends `HopJoin`
with the same `gate`, and copies in both directions. The dials that `HopExtend` and
`HopConnect` cause are limited per path, per neighbour, per neighbour host and per
destination host (§5). A node that hides behind 0 hops sends `HopJoin` itself.

- **A node that is not open.** The owner copies `relays4` and `relays6` from the `Peer`
  record of `node`, at most 2 *(initial)* each (`Protocol.md` §10.3); more, or a length that
  is not a multiple of 22 or 34 B, is `Error{MALFORMED}`. When `node` has relays in a
  family, the last hop reaches it there as any node reaches a leaf: it sends `Punch` through
  one of them and dials (`Protocol.md` §9.4, §9.5). A punch that fails is a dial that fails
  (table below). One `HopConnect` counts as one dial for §5, whether `node` is dialled directly
  or through a punch; each further punch attempt counts as one more. Each relay is an address as
  §2.2 asks of `node`; else `Error{MALFORMED}`. A dial to a relay counts only in the row per
  destination host, at the relay's host; a relay the last hop is already connected to is no
  dial. A punch that the relay refuses counts as a failed dial of the neighbour host that
  asked, not of the destination host (§5).

**Inwards.** An owner that wants to be reached sends `HopGate` on the control stream of a
path. The last hop answers with a gate value and is from then on a **gate** of the owner. A
second `HopGate` on a path is answered with the same gate value. A `HopExtend` on a path
that has a gate is answered `Error{MALFORMED}`: the gate is the last hop.

- A node that receives `HopJoin` with a gate value it gave opens a stream with an odd number
  on that path and copies. With an unknown value it answers `Error{NOT_FOUND}`.
- A node that receives `HopJoin` without a gate value, and sets `ANON`, is itself the other end
  of the session.
- The owner publishes its gates in a `GATE` record (§4.3).
- A gate accepts at most 10 *(initial)* `HopJoin` per minute from one host and answers
  `Error{BUSY}` above that.

**Status frame.** As after `HopStream` (§2.4), the first thing in the other direction after
`HopConnect` and after `HopJoin` is one frame, `HopOk` or an `Error`, before any raw byte.
Both are, like `HopStream`, followed by raw bytes.

| After | Sent by | `HopOk` when | `Error` |
|---|---|---|---|
| `HopStream` | the neighbour that received it, on that link, without a layer | it accepted the stream for itself | §2.4 |
| `HopConnect` | the last hop, on the stream towards the owner. The frame is in the layers of the path, as every byte of that stream | `node` answered its `HopJoin` with `HopOk` | `NOT_FOUND` when `node` cannot be reached within 10 s *(initial)*; `MALFORMED` for a `node` that §2.2 refuses; `BUSY` above a dial limit (§5); else the `Error` that `node` gave |
| `HopJoin` | the node that received it, without a layer | with a gate value: its neighbour on the path answered the `HopStream` with `HopOk`. Without one: at once | `NOT_FOUND` for an unknown gate value; `BUSY` above the limit; a failed stream on the path is a reset |

The last hop reads the status frame of the `HopJoin` it sent and does not copy it: the owner
gets the one of `HopConnect`. After `HopOk` the two ends speak to each other. The hops and
gates in between copy bytes.

### 3.2 Opening a session

The side that opens is A, the other is B. The first stream of a session starts with

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

- A key is the anonymous key of a node that hides, and the node key of one that does not.
- `sig_B` and `sig_A` are Ed25519 signatures by the key that the same message carries. `nonce 0`
  is 12 zero bytes; each key seals one message only.
- A MUST compute the node ID from the key of B and compare it with the node it meant to
  reach. On a mismatch it closes the stream.
- B learns the key of A only inside the seal. A hop or gate between them sees neither key.
- A session ends 10 min *(initial)* after its last stream closed. A session with an open
  stream does not end.

**Further streams** of a session start with

```
0x02 ‖ sid(16) ‖ number(8, big-endian)        number: even when A opens the stream, odd when B does; never used twice
```

and need no signature. They MAY use another path than the first. **A session therefore
outlives a path**: when a path breaks, its streams end, and the session goes on over the next
path.

- Each side MUST use a number greater than every number it used before in the session.
- The receiver resets a stream whose number it has already seen from that side in the
  session, whose number is not above the highest it has seen from that side minus 1,024
  *(initial)*, or whose number has the parity of the receiver's own side. A gate or a last
  hop that sends a stream a second time is refused so.
- A stream with an unknown `sid`, or whose first byte is neither 0x01 nor 0x02, is reset. An
  opener whose further stream is reset before a record came back on it drops the session and
  opens a new one with the three messages above. A reset carries no reason (`Protocol.md`
  §5.2), so the opener does the same whether the `sid` was unknown, the number was seen or
  the path broke.

### 3.3 What a stream of a session carries

```
key     = H("ed3k v1 hop session stream", k_s ‖ number(8, big-endian) ‖ dir(1))     dir = 0x00 from A, 0x01 from B
record  = len(2) ‖ ChaCha20-Poly1305(key, nonce = number of the record(12, big-endian), plaintext of at most 16,384 bytes, empty AAD)
```

`len` is the length of the ciphertext with its 16-byte tag, big-endian, 16 to 16,400. Records
are numbered from 0 per stream and direction; the number is the nonce and is not sent. The
first stream has the number 0, and its records follow the three messages of §3.2.

- A record whose plaintext is empty (`len` = 16) is the end of a direction, and is
  authenticated like any record. A direction that ends without it was cut; the receiver treats
  the exchange as reset.
- A record with `len` below 16 or above 16,400, or that fails its check, ends the stream.
- A session has no observed address, so a `Store` with `source` over it is answered `REJECTED`
  (`Protocol.md` §10.3); a node that hides publishes `ANON` and `GATE` records through a
  `HopLookup` (§4).
- **One stream is one exchange** (`Protocol.md` §5.2). The records carry its frames. Every
  message of ed3k works unchanged.
- The first exchange is `Hello`, on the first stream: A sends it and B answers, as in
  `Protocol.md` §5.5. A node under its anonymous key sends no `observed`, `ep4`,
  `ep6` and no `nat`, an empty `nick`, and as `agent` one fixed string that its implementation
  uses for every hidden node; a `Peer` that names it carries its node ID alone, and its
  `FileRequest`s carry no `relays4` or `relays6`. Anything else would tie the two keys. A
  receiver that sets `ANON` treats a `Peer` with a node ID alone as an anonymous source and
  looks up its `GATE` record under the limits of §4.3; any other receiver drops it.
- A node MUST limit new sessions per node ID to the node-ID rate of `Protocol.md` §4.6, and
  per gate or per node that sent the `HopJoin` to its host-address rate. Nothing is banned: a
  session over a limit is refused by resetting its first stream; the opener does not open
  another session to that node ID for 60 s *(initial)*. B knows the key of A only after the
  third message (§3.2), so it checks the node-ID rate then, and resets the first stream at that
  point when it is over.
- B can open a stream to A only through a gate of A. An uploader that offers a slot later
  (`Protocol.md` §12.5) looks up the `GATE` record of the downloader.

---

## 4. Kad for a node that hides

### 4.1 Lookups by the last hop

A lookup connects to 8 to 12 nodes. Through three hops each of them would cost seconds. The
owner hands the lookup to the last hop of a path.

```protobuf
message HopLookup {           // type 30. First frame on a stream, read by the last hop
  KadRequest request = 1;
}

message HopLookupAnswer {     // type 31
  repeated KadAnswer answers = 1;   // those of the closest set (Protocol.md §10.4), closest first
}
```

- The last hop runs the lookup of `Protocol.md` §10.4 with the request as it is, under its own
  key, and returns the answers of the closest set.
- It answers `Error{DENIED}` to a `HopLookup` whose `store` carries `source`, `also`, `http`,
  `links` or `sealed`: those records are bound to the connection (`Protocol.md` §10.3) and
  would name the last hop as a source. It refuses in the same way a `store` with a signed
  record and no `signer`, which would be counted under its own key.
- It allows 20 *(initial)* lookups per minute and path, and answers `Error{BUSY}` above that.
- **The last hop sees what is searched and stored. It does not see by whom.** An owner SHOULD
  spread its lookups over its paths.
- The last hop can leave out or invent answers. Signed records are checked by the owner as
  always. A false source costs a failed session.

### 4.2 Storing under another key

A storing node takes the key of a record from the connection (`Protocol.md` §10.7). A record
that a last hop stores for someone else is signed with a key that is not the one of the
connection.

- A `Store` of any signed kind MAY carry `signer` (`Protocol.md` §10.7). The storing node then
  checks the signature under `signer`, and counts the record for that key.
- Its limits per address ([`ip-diversity.md`](ip-diversity.md) §2.4) count the address the
  request came from. For `ANON` records that is 4 per host and 8 per site for each target
  *(initial)*; a record beyond a limit is answered `REJECTED`. Many nodes that store through
  one last hop share its allowance, so an owner uses several paths.

### 4.3 The two records

```protobuf
// in enum Kind (§10.3 of Protocol.md):
ANON = 11;
GATE = 12;

// in Record (§10.7 there):
Gates gates = 10;

message Gates {
  uint64 issued = 1;          // Unix time; the later record replaces the earlier
  repeated Gate gates = 2;    // at most 4
}

message Gate {
  bytes node = 1;             // compact node, 22 or 34 B
  bytes gate = 2;             // 8 B
}

// in Store:
Signed anon = 10;             // ANON or GATE; `signer` is always present

// in KadAnswer:
bytes  anon = 16;             // SOURCE search: n × 16 B node IDs of anonymous sources
Signed gate = 17;             // GATE search
```

| | `ANON`: an anonymous source of a file | `GATE`: where an anonymous node is reached |
|---|---|---|
| Target | `root[0..16)`, as for `SOURCE` | `H("ed3k v1 gate", node ID)[0..16)` |
| Signed by | the anonymous key | the anonymous key; the target of the request MUST follow from `signer` |
| Content | `expires` and nothing else | `expires` and `gates` |
| Lifetime | 5 h *(initial)*, as `SOURCE` | 1 h *(initial)* |
| Count | one per key and target; at most 64 *(initial)* per target, a further one answered `FULL`; at most 4 per host and 8 per site *(initial)* (§4.2), a further one answered `REJECTED`. The address limits are checked first: a record over them is `REJECTED` before the target is looked at ([`ip-diversity.md`](ip-diversity.md) §2.4) | one per target; replaced by a later `issued`, else answered `OLDER` |
| Returned | in `anon`, with every search of kind `SOURCE` at that target | in `gate`, with the key in `keys` |

**Checks**, in addition to those of `Protocol.md` §10.7

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| `signer` | `Store.anon` carries `signer`, and the signature is valid under it | `gate` is valid under `keys[key]`, and that key gives the node ID that was looked up |
| Target | `GATE`: follows from `signer` | the same |
| Content | `ANON`: `expires` and nothing else. `GATE`: `expires` and `gates`, nothing else | the same for `gate` |
| `gates` | at most 4 entries | the same |
| `Gate.node` | 22 or 34 B, and an address that §2.2 accepts as `node` | the same |
| `Gate.gate` | 8 B | the same |

A record that fails a check is answered `REJECTED` by the storing node and dropped by the
searcher.

- **Two steps on purpose.** Gates change whenever a path breaks. A node with 10,000 files
  then stores one `GATE` record again, and its 10,000 `ANON` records stay as they are.
- `anon` holds node IDs without key or signature. As for any source, reaching it is the proof:
  the session shows the key behind the ID.
- A searcher that wants an anonymous source looks up its `GATE` record, checks that the key
  gives the node ID, picks a gate at random, and opens a session through it. It makes at most
  4 *(initial)* failed `GATE` lookups or session attempts per file and hour, as
  [`ip-diversity.md`](ip-diversity.md) §2.5 limits failed source dials: `anon` is unsigned.
  One session attempt is one gate dialled.
- An owner stores its `GATE` record again after 45 min *(initial)*, and at once when a gate
  was lost.
- The batch of `Protocol.md` §10.4 does not carry `ANON` records: each is signed for its
  target. A node with very many files pays one lookup per file (§8).

---

## 5. Being a hop

A hop spends its own upload on the data of strangers. It decides how much.

**Limits a hop sets** *(all initial)*

| | |
|---|---|
| Paths from one neighbour | 8 |
| Paths in total, and the share of its upload | the user's choice. A client SHOULD offer the share as a setting and start with a quarter |
| Streams per path | 64 |
| Exchanges on the connection to one neighbour | 64 opened by the peer (`Protocol.md` §4.1), and 65 more for each path between them, in both directions: its 64 streams and the control stream |
| Held per stream and direction | 256 KiB |
| Lookups per path | 20 per minute |
| Dials (`HopExtend`, `HopConnect`) per path / per neighbour | 4 / 16 per minute |
| Dials caused by `HopExtend`/`HopConnect` whose QUIC handshake did not complete within 10 s, or that are still pending, relay dials and punches included, per destination host (IPv4 /32, IPv6 /64), over all paths and neighbours | 8 per hour |
| Paths and dials per neighbour host address | 16 paths; 32 dials per minute |
| Failed dials per neighbour host address: a wrong node ID, or a punch the relay refused | 8 per hour |
| `HopJoin` per gate and host | 10 per minute |

A path counts for the neighbour that sent its `HopCreate`, and for that neighbour's host, from
that `HopCreate` until the path ends, whatever its state, also while it is being extended.

A dial whose QUIC handshake completed within those 10 s does not count in the row per
destination host, whatever node ID answered. A hop gives up a dial after 10 s (§2.2); it then
counts for the hour. A wrong node ID, and a punch the relay refused, count as failed
dials of the neighbour host that asked; above its limit that neighbour host gets `BUSY`.

Above a limit it answers `Error{BUSY}`. A hop that takes no new paths clears `HOP` in its
`Hello`.

**Give and take.** A hop counts, for each neighbour, two numbers: the bytes it forwarded on
paths that this neighbour asked it for with `HopCreate`, and the bytes this neighbour forwarded
on paths the hop asked *it* for.

- Every neighbour gets a **base rate**, 16 KiB/s *(initial)*, shared over its paths.
- What the hop gives to paths beyond the base rates, it SHOULD divide in proportion to what
  each neighbour has forwarded for it in the last 7 days *(initial)*.
- Rate that nobody has earned MAY go to anyone.
- A hop slows a path down by reading less (§2.4). It never drops data.

The count needs only what two neighbours see of each other, so it unmasks nobody. It also
reaches only one link far: a hop in the middle is charged by the next hop for data that is not
its own, and has to earn that by forwarding. A node that cannot be a hop, a leaf, gets the base
rate and what is free.

**What a hop is not.** A hop never sends anything out of ed3k. It connects to ed3k nodes and
speaks `HopCreate`, `HopJoin` and Kad to them. It cannot be used to reach a web server or any
other service.

---

## 6. What it does and does not give

| | |
|---|---|
| A peer, or a storing node | sees a gate or a last hop in place of the address |
| A single hop | sees its two neighbours and how much flows. It cannot read it and does not know its place in the path, except a last hop, which sees where a stream goes, and a gate |
| A last hop | reads the lookups it runs: targets, search words, the records stored. Not for whom |
| A last hop, or a gate | reads `sid` on every further stream of a session (§3.2), so it can tell which paths carry one session and link their last hops |
| The first and the last hop of a path together | match a path by its timing and volume. This is the limit of every design of this kind, and file sharing makes it easy: the flows are large and long |
| Whoever runs many hops | is picked as first and last more often. Hops are found through Kad, where nodes are cheap; there is no list that someone vouches for. Entry hops and the site rule make this slower, not impossible |
| Someone who watches the links of a node | sees that it takes part, and how much. Sizes and times are not hidden; there is no padding and no delay |
| Someone who watches for a long time | learns from which files appear and vanish together with which node being online |
| The crowd | is the nodes that use paths. A small network hides little |
| A gate | is known to everyone and can be flooded. The owner replaces it |
| The anonymous key, taken later | names the node's past records. The keys of paths and sessions are gone |
| With 1 hop | the hop knows everything. It is a relay for a pair that cannot be hole-punched (`Protocol.md` §9.4), not anonymity |
| Speed | each hop uploads every byte again. A path is as fast as its slowest hop is willing to be |

---

## 7. NeoShare, and alternatives considered

NeoShare was read in its source (`NeoKad/Kad/KadRouting`, `NeoLoader/FileTransfer/NeoShare`);
nothing was run. The broader comparison is in
[`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md) §8.

| | NeoShare | This extension |
|---|---|---|
| The setting | levels 0 to 4. In the code levels 0 to 2 give the same direct route; 3 and 4 add 1 and 2 random nodes | 0 to 3 hops for each direction |
| Who picks the nodes | each relay picks its next ones, towards a random area of the DHT; the route is a tree with several branches | the owner picks every hop |
| What a relay sees | the IDs of both ends, a frame counter, offsets, and the whole handshake | its neighbours and the volume |
| Encryption | end to end; none per relay beyond the link | end to end, and one layer per hop |
| The receiver | registers with the nodes near a random ID | publishes gates in a signed record |
| Transport | 4 KB frames, acknowledged by each hop and from end to end, with windows and timers of its own | the exchanges of QUIC on every link; a session of records on top |
| A slow relay | routed around: next hop by measured delay and loss | holds its path; the owner measures paths and replaces the slowest |
| Queue at a relay | without limit | 256 KiB per stream and direction |
| Limits | 100 relayed operations in total; no limit per peer, no quota, cannot be turned off | a capability; limits per neighbour; a share the user sets |
| Reward | none | give and take between neighbours |
| Kad through others | lookups and stores handed to a proxy that sees the requester | handed to the last hop, which does not |

**Decided**

| Choice | Pro | Con |
|---|---|---|
| **The owner picks, with layers**, in place of relays that pick | no hop sees both ends; the owner spreads the hops over networks and sets their number | the owner needs a list of hops; a path that loses a hop is gone |
| **Reliable streams on every link**, in place of unreliable packets with a QUIC connection inside | needs nothing that version 1 lacks; loss is repaired where it happens; a full hop slows the sender instead of dropping | one slow hop holds its path; a hop keeps state per stream; a session is a second secure channel to get right |
| **Each side its own half**, in place of a meeting node | the number of hops is each node's own choice and cost; one hop also serves pairs that cannot be punched | up to 6 hops between two hidden nodes; a record and a lookup more per hidden source |
| **Voluntary, give and take**, in place of a duty | nobody carries strangers' data against its will | capacity depends on users who turn it on |

**Alternatives**

| Alternative | Pro | Con |
|---|---|---|
| **A binding over Tor or I2P** | a large crowd and years of study | a dependency; streams only for Tor, which asks not to be used for bulk transfer; a separate network |
| **One exchange for all streams of a path**, with a multiplexer | fewer exchanges per link | one lost packet holds every stream; flow control has to be written again |
| **A tag on every layer** | a changed byte is noticed at the next hop | 16 bytes per record and per hop, and records in place of a running stream |
| **Lookups walked by the owner**, one session per node | the last hop reads less | 8 to 12 sessions through the path for every lookup |
| **Gates in every source record**, with no `GATE` record | one lookup less | every source record is stored again when a gate changes |
| **Signed entries in `anon`** | a false entry is seen without a session | about 100 bytes per source in place of 16 |
| **A new first hop for every path** | no hop sees a node for long | a hostile first and last hop at once becomes a matter of time |
| **Padding and delays** | sizes and times say less | throughput, which this extension is for |

---

## 8. Open points

- **Review of the cryptography** (§2.3, §3.2, §3.3). The key agreement follows the pattern Tor
  uses for its hops and the session a signed key exchange, but the text here is new.
- Layers have no tag, and the frames a last hop reads (`HopExtend`, `HopConnect`, `HopLookup`)
  can be changed by an earlier hop that guesses their content. A new hop and a session are
  safe by `proof` and by the check of the node ID. A lookup is not.
- Where a node gets hops it can trust more than a sample from Kad: signed lists, as for the
  first contact ([`bootstrap.md`](bootstrap.md)), or the key puzzle
  ([`key-puzzle.md`](key-puzzle.md)).
- Anonymous keys cost nothing, and the limits of a storing node count the last hop. The key
  puzzle would give a key a price.
- `ANON` records for very many files: one lookup and one signature per file. The batch form of
  a signature (`Protocol.md` §10.7) would allow a batch as for `SOURCE`.
- `ANON` stays optional, also for a `ROUTER`: a node that hides can only use sources that set
  it. Making it a duty would put the session of §3 into the core. To be looked at again when
  it is known how many nodes set it.
- Give and take reaches one link far and starts at nothing between two nodes that never met.
- Live streams, publish and subscribe, and friends under an anonymous key are not looked at.
- All numbers are first values.
