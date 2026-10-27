# ed3k Protocol Specification

**Status:** draft 0.2 (2026-10-13). Nothing in this document is implemented yet.
**Wire version:** 1 (ALPN `ed3k/1`).

ed3k is a peer-to-peer file sharing protocol in the tradition of eD2K and Kad. It keeps their
model (hash-identified files, a Kademlia DHT, optional index servers, upload queues with credits)
and replaces the wire format, transport and identity:

- every message is a **protobuf** message; there is exactly one message per purpose
- transport is **UDP only**; the mandatory binding is **obfuscated QUIC**
- **NAT traversal is a required feature**, not an add-on
- **IPv4 and IPv6** are both first-class, in one overlay
- identity is an **Ed25519 key**; its hash is the node ID in Kad and everywhere else
- files are identified by a **BLAKE3** root hash

**ed3k does not require eD2K.** A compliant ed3k implementation needs no eD2K, eMule-Kad or TCP
code at all. The reference implementation, eMuleQt, speaks both protocols; §14 specifies how such a
dual-protocol client tells eD2K peers that it also speaks ed3k. Apart from the optional `ed2k`
alias of a file (§7.4), §14 is the only part of this document that concerns eD2K, and it is
OPTIONAL.

---

## 0. Conventions

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, RECOMMENDED, MAY and OPTIONAL are to be
read as in RFC 2119.

- Protobuf is **proto3**. `bytes` fields with a fixed layout state it in a comment; integers inside
  such layouts, and inside the inputs of `H` and of signatures, are **big-endian** (network order).
- `H(ctx, data)` is BLAKE3 in `derive_key` mode with context string `ctx`, 32 bytes of output
  unless a length is given; a longer output is the extended output of the same `derive_key`
  call, so its first 32 bytes are the 32-byte value. `H(ctx, data)[a..b)` is a byte slice.
- `varint` is the protobuf base-128 varint.
- Integer fields are `uint32` or `uint64`; both are a varint on the wire, and the type states the
  range a reader accepts. A field that holds a random 64-bit value is `fixed64`
  ([`docs/wire-format-comparison.md`](docs/wire-format-comparison.md) section 2.4).
- Numbers marked *(initial)* are tuning values. They are collected in Appendix A and may change
  without a wire version bump. Appendix A also lists a few values that are fixed by the wire
  format; it marks them, and they change only with the wire version.
- A field declared `optional` has explicit presence: a reader tells an absent field from one
  that is set to its default. Where a document gives an absent optional field a meaning, it is
  "unchanged" or "not told". A field without `optional` that is absent reads as its default
  (0, empty), as proto3 does, and MUST NOT be given a meaning that differs from its default's.
- **Canonical serialisation.** Where a document requires it, a message is written with its
  fields in ascending field number, each once, repeated fields in order, every varint in its
  shortest form, a field at its default value left out, and no unknown fields. Everywhere else
  serialisation is not canonical, and a reader MUST accept any valid encoding.
- A `bytes` field with a fixed layout whose length is not one the layout allows makes its
  message malformed (`Error{MALFORMED}`; `REJECTED` for a record), unless its document names
  another outcome. A field that may be absent is not a wrong length when it is empty.
- The protobuf fragments in this document and in the extensions are excerpts.
  [`proto/ed3k.proto`](proto/ed3k.proto) is the normative definition of every message, field
  and enum of wire version 1, extensions included.

---

## 1. Design decisions

Decisions taken while drafting. Each links to the section that carries the reasoning and the
alternatives that were rejected.

| # | Decision | Section |
|---|---|---|
| D1 | Messages are specified against an abstract transport contract; obfuscated QUIC is the one binding every peer MUST implement | §3, §4 |
| D2 | One overlay for IPv4 and IPv6; records carry one endpoint per family | §8, §10.2 |
| D3 | Node ID is 128 bits, the truncated hash of the public key, in Kad and outside it | §6.1 |
| D4 | Routing position is IP-bound (BEP 42 style) in its top 21 bits and key-bound in the rest | §6.3 |
| D5 | A dual-protocol client listens for ed3k on the UDP port whose number equals its eD2K TCP port | §14.1 |
| D6 | eD2K peers learn about ed3k support from a user hash derived from the node ID | §14.2 |
| D7 | zstd decompression is required; senders compress only when it helps, messages and file blocks alike | §5.3, §12.3 |
| D8 | Files are identified by a BLAKE3 root; the eD2K hash is an alias that can be dropped later | §7 |
| D9 | Kad runs over QUIC in version 1; a sessionless datagram binding is reserved | §10.6 |
| D10 | Relays forward rendezvous signalling only, never file data | §9.4 |
| D11 | A server is an ordinary ed3k node with the INDEX capability; no separate server message set | §11 |
| D12 | Keyword and note records are signed by their publisher; source records are not, they are proven by dialling | §10.3, §10.7 |
| D13 | Every signed object carries a list of signatures, so algorithms can be added | §10.7 |
| D14 | A keyword target includes a level, and a store is answered per record, so crowded keywords can spill to further levels later without a wire change | §10.1, §10.8 |
| D15 | One Kad request may store source records for many targets, so that publishing costs lookups by the size of the network and not by the number of files | §10.3, §10.4 |
| D16 | A network other than the public one has a name that enters the obfuscation key; nodes of two networks cannot read each other's datagrams | §4.3 |

---

## 2. Roles and capabilities

Every participant is a **node**. A node announces what it does with capability bits in `Hello`
(§5.5):

| Bit | Name | Meaning |
|---|---|---|
| 0 | `ROUTER` | Reachable without prior signalling on at least one address; may be placed in routing tables and stores Kad records |
| 1 | `RELAY` | Forwards rendezvous signalling for firewalled nodes (§9.4). A `ROUTER` node MUST set it unless it is out of relay slots |
| 2 | `INDEX` | Index node ("server"): accepts and answers records for every target (§11) |
| 3 | `BROWSE` | Optional extension: answers `Browse` ([`extensions/browse.md`](extensions/browse.md)) |
| 4 | `CHAT` | Optional extension: accepts `Chat` ([`extensions/chat.md`](extensions/chat.md)) |
| 5 | `KAD_DGRAM` | Reserved for the sessionless Kad binding (§10.6). MUST be 0 in version 1 |
| 6 | `ED2K` | Also speaks eD2K (§14). Informational |
| 7 | `PUZZLE` | Reserved for the key puzzle (§6.3). MUST be 0 in version 1 |
| 8 | `LEVELS` | Reserved for keyword levels and ranking on Kad (§10.8). MUST be 0 in version 1; levels at an index node need no bit (§11) |
| 9 | `HTTP` | Optional extension: understands HTTP sources ([`extensions/http-source.md`](extensions/http-source.md)) |
| 10 | `SETS` | Optional extension: understands file sets ([`extensions/file-set.md`](extensions/file-set.md)) |
| 11 | `LIVE` | Optional extension: takes part in live streams ([`extensions/live.md`](extensions/live.md)) |
| 12 | `PUBSUB` | Optional extension: passes on new records of things it follows ([`extensions/pubsub.md`](extensions/pubsub.md)) |
| 13 | `LAN` | Optional extension: answers nodes of its local network ([`extensions/lan-discovery.md`](extensions/lan-discovery.md)) |
| 14 | `ANON` | Optional extension: accepts sessions that arrive through a path of other nodes ([`extensions/hops.md`](extensions/hops.md)) |
| 15 | `HOP` | Optional extension: forwards for the paths of others ([`extensions/hops.md`](extensions/hops.md) §5) |
| 16 | `CREDIT` | Optional extension: signs receipts for what it downloads and understands bids for upload slots ([`extensions/credit.md`](extensions/credit.md)) |
| 17 | `LINKS` | Optional extension: understands links at file hosters as sources ([`extensions/hoster.md`](extensions/hoster.md)) |
| 18 | `PAID` | Optional extension: hands out epoch keys of paid channels and sets to pass holders ([`extensions/subscription.md`](extensions/subscription.md)) |
| 19 | `ROOM` | Optional extension: takes part in rooms with many members ([`extensions/room.md`](extensions/room.md)) |
| 20 | `NAMES` | Optional extension: answers friends that ask what a name stands for ([`extensions/name.md`](extensions/name.md)) |

`caps` travels in `Hello` only (§5.5). Compact Kad contacts and source records do not carry it,
so a node learns a peer's capabilities after it has connected. A bit or a policy that changes while a
connection is open, such as `RELAY` when the relay slots are full (§9.4), is not announced on
that connection. The answer to the next request is the signal there: `Error{BUSY}` for a full
resource, `Error{DENIED}` for a refusal, `Error{UNSUPPORTED}` for a capability that was turned
off. A bit that was turned on is seen on the next connection.

A node sets `ROUTER` while it is `open` in at least one family (§9.2), also when the operator
declared it open there, and clears it on new connections when it no longer is. `ROUTER` alone
does not say in which family; `Hello.nat` does (§10.2). Two nodes MAY leave `ROUTER` unset
while open: an index node (§11) and a second node for friends
([`extensions/friend.md`](extensions/friend.md) §5).

A node that is not `ROUTER` is a **leaf**: it uses Kad as a client, is never put in a routing
table, and is reached through relays and hole punching.

---

## 3. Layering and the transport contract

```
  messages   (protobuf, §5–§12)             transport-agnostic
  ------------------------------------------------------------
  framing    tag + length, zstd flag (§5)   transport-agnostic
  ------------------------------------------------------------
  binding    quic-obfs (§4)                 the only binding in version 1
  ------------------------------------------------------------
  UDP over IPv4 / IPv6
```

Messages never refer to QUIC. They rely on a **transport contract** that any binding has to meet:

| | Property |
|---|---|
| T1 | **Mutual authentication.** Each side learns the other's Ed25519 public key and has proof of possession |
| T2 | **Confidentiality and integrity** of everything above the binding |
| T3 | **Validated remote address.** The peer's source address is proven by a round trip before more than a bounded reply is sent to it |
| T4 | **Exchanges.** Either side can open any number of independent, ordered, reliable, flow-controlled byte channels at negligible cost. An exchange has an initiator and can be half-closed by each side |
| T5 | **Unlinkability to a protocol.** A passive observer without knowledge of the peer's node ID cannot classify the traffic by fixed bytes, fixed sizes or a decryptable handshake. A binding whose transport does not provide this MUST add obfuscation; a binding whose transport already hides itself MUST NOT add a second layer |

Unreliable datagrams are not part of the contract in version 1 (see §10.6).

**Why not gRPC or gRPC-Web over UDP.** It was considered because protobuf services are usually
exposed that way. gRPC assumes HTTP: per call it adds pseudo-headers, a 5-byte message prefix and a
trailers block, and over UDP it would need HTTP/3 with QPACK. As of this draft neither grpc-core
nor QtGrpc ship an HTTP/3 channel. Calls here are symmetric (either peer may ask), long-lived
(a file stream) and carry raw file blocks, which fits streams better than RPC. HTTP/3 would also
be one more recognisable layer. The framing in §5 gives the one useful property of gRPC, a typed
request and answer on its own stream, for 2 bytes.

**Other bindings.** WebTransport, TLS-over-TCP with a stream multiplexer, or an in-memory binding
for tests can meet T1–T5. None is specified. A future binding gets its own name and ALPN-like
identifier and changes no message. Version 1 has no way to advertise a second binding; the gap
and a path to close it are described in [`docs/transport-bindings.md`](docs/transport-bindings.md).

---

## 4. The `quic-obfs` binding

### 4.1 QUIC profile

- QUIC version 1 (RFC 9000/9001/9002) with TLS 1.3. ALPN `ed3k/1`. No SNI.
- **Mutual authentication with raw public keys** (RFC 7250), Ed25519 only. No X.509 certificates:
  a certificate would add several hundred bytes to each handshake and carries nothing the key does
  not. Both sides MUST send their key; a handshake without a client key is closed with
  `NO_KEY`. A server MAY fail such a handshake inside TLS instead (`certificate_required`); a
  client treats both alike. The same holds for a key that is not Ed25519.
- The initiator knows whom it is dialling. After the handshake it MUST compute the peer's node ID
  (§6.1) and compare it with the expected ID; on mismatch it closes with `WRONG_PEER`. The
  first 12 bytes of the expected ID key the datagrams (§4.3): a node whose ID differs there
  cannot unmask them, and the dialler sees a timeout instead.
- A side closes with `NO_KEY` or `WRONG_PEER` only after its last handshake flight is out: an
  application close in an Initial or Handshake packet loses its code (RFC 9000 §10.2.3).
- An **exchange** (T4) is one bidirectional QUIC stream. Unidirectional streams are not used.
- Connection migration SHOULD be enabled. It lets a session survive an address change of its
  client side. The server side of a connection does not migrate (RFC 9000 §9): a connection
  whose server changed its address is lost and is opened again, by a new hole punch where one
  was needed (§9.5). A connection does not move between IPv4 and IPv6 (§8.3).
- 0-RTT MAY be used, and only for `Hello` and `KadRequest` without a `store` (replay-safe).
  A server MUST NOT act on any other message that arrives in 0-RTT before the handshake is
  complete; it holds the request until then.
  A resumed handshake carries no key. Both sides MUST keep the peer's key with the session
  ticket and take the node ID from it. A server that cannot recover the client's key from a
  ticket MUST refuse the resumption and do a full handshake, which also rejects the 0-RTT data.
  A client MUST use a ticket only towards the node ID whose key it was issued under.
- Idle timeout: both sides send `max_idle_timeout` = 30 s *(initial)*. A side that wants the
  connection kept sends a QUIC PING every 20 s *(initial)*: for a file stream that is open or
  waits in a queue (§12.5), for a relay reservation (§9.4), and where an extension says so. A
  side with no such reason sends none and lets the timeout run, or closes with `CLOSED`.
  Towards an index node both values are wider (§11).
- A node allows its peer at most 64 *(initial)* open exchanges that the peer opened on a
  connection (`initial_max_streams_bidi`, raised as exchanges close). The limit is per
  direction; file streams and the exchanges of extensions count in it. The two ends of a
  path link allow each other 65 more for each path between them
  ([`extensions/hops.md`](extensions/hops.md) §5). A responder resets (§5.2) an exchange whose
  request has not arrived whole within 30 s *(initial)* of its opening; a file stream, once its
  first `FileRequest` arrived, follows §12.5 instead.
- Congestion control is the sender's choice. BBR is RECOMMENDED.
- A node MUST accept a Retry packet (RFC 9000 §8.1.2) and repeat its Initial with the token. A
  node MAY answer an Initial with Retry at any time
  ([`extensions/flood-protection.md`](extensions/flood-protection.md) §2).

**Close codes.** An application close (`CONNECTION_CLOSE` with an application error code,
RFC 9000 §10.2) uses one of:

| Code | Name | When |
|---|---|---|
| 0 | `CLOSED` | normal end: idle, or the node shuts down |
| 1 | `NO_KEY` | the handshake carried no client key |
| 2 | `WRONG_PEER` | the peer's node ID is not the expected one (§6.1, §14.2) |
| 3 | `DENIED` | the peer is banned (§4.6) or refused by the policy of a network |
| 4 | `PROTOCOL` | the first frame on the initiator's first stream was not `Hello` or was a malformed one (after the `Error`, §5.5), or the answer on that stream is not a `Hello` or is a malformed one, or no `Hello` of the peer arrived on that stream within 30 s (§5.5), or a frame header (`tag`, `len`) did not parse, a `len` above 1 MiB included, so that the exchange cannot be read on (§5.1). A body that does not parse, or exceeds 1 MiB after decompression, is answered `Error{MALFORMED}` on its exchange instead (§5.2, §5.3, §5.6) |
| 5 | `BUSY` | the node is out of connections, or the peer's host address is over its budget (§4.6) |

A code this table does not name is read as `PROTOCOL`.

### 4.2 Why obfuscation is required

QUIC encrypts its payload but not its shape. The Initial packet is protected with a key derived
from a public salt, so any observer can decrypt it; it is padded to 1200 bytes; and short-header
packets repeat a connection ID. The Great Firewall has decrypted and filtered QUIC Initials since
April 2024 and blocks the flow for about 180 s on a match. A UDP datagram of random bytes does not
trigger its fully-encrypted-traffic heuristic, which is limited to TCP. The obfuscation layer
therefore has two jobs: remove every fixed byte, and blur the 1200-byte handshake size.

This is obfuscation, not secrecy. The key is derived from the receiver's node ID, which is public
to anyone who takes part in the network. Confidentiality comes from TLS. Unmasking only
demultiplexes: that a datagram unmasks proves nothing about who sent it. Evidence about a peer,
such as its reachability (§9), rests on what a datagram carries and on the transport (T1, T3),
never on the mask.

### 4.3 Datagram format

Every UDP datagram of this binding is:

```
salt(8) ‖ mask( magic(4) ‖ kp(1) ‖ pad(pad_len) ‖ body )
```

| Field | Meaning |
|---|---|
| `salt` | 8 random bytes, sent in clear, fresh for every datagram: a datagram that is sent again is masked again under a new salt. `salt[0]` MUST NOT be `0xE3`, `0xE4`, `0xE5`, `0xC5`, `0xD4`, `0xA3` or `0xB2` (re-roll). These are the plaintext headers of eD2K/eMule UDP; the rule keeps a stray ed3k datagram from being parsed as one by a legacy node at a stale address. A receiver does not check it |
| `magic` | `0x65 0x33 0x6B 0x01` (`"e3k"`, version 1). Checked after unmasking |
| `kp` | bits 7-6 `kind`, bits 5-0 `pad_len` (0–63) |
| `pad` | `pad_len` random bytes |
| `body` | depends on `kind` |

| `kind` | Name | `body` |
|---|---|---|
| 0 | `DATA` | one QUIC datagram holding one short-header packet. Such a packet has no length field and is always the last of its datagram (RFC 9000 §12.2), so nothing follows it |
| 1 | `HANDSHAKE` | one QUIC datagram containing a long-header packet, and whatever QUIC coalesced after it |
| 2 | `FRAGMENT` | `msg_id(1) ‖ frag(1) ‖ piece`. `piece` is a slice of the QUIC datagram that a `HANDSHAKE` body would hold (§4.4). `frag` holds `idx` in bits 7-4, counting the pieces from 0, and `total` in bits 3-0; a sender uses 2 to 4 pieces, a receiver accepts 2 to 15 |
| 3 | `PROBE` | `nonce(8)`; used for hole punching and reachability tests (§9). A `PROBE` whose body is not 8 bytes is dropped |

A receiver takes `kind` as it is given: it decides how much of the datagram is masked (below).
A `DATA` datagram whose QUIC packet has a long header, and a `HANDSHAKE` whose body starts with
a short-header packet, are dropped.

**Key and mask.**

```
obf_id   = NodeID[0..12)                    of the node that ACCEPTED the connection
net_id   = H("ed3k v1 net", name)[0..16)                   empty in the public network
key      = H("ed3k v1 obfs", net_id ‖ obf_id ‖ dir ‖ salt) dir = 0x00 to that node, 0x01 from it
stream   = BLAKE3 keyed XOF (key, empty message), as many bytes as masked
mask(x)  = x XOR stream
```

`obf_id` is 12 bytes, not 16, so that it can be learned from an eD2K user hash (§14.2). A `PROBE`
is always keyed to its receiver with `dir = 0x00`. The keystream is the extendable output of
BLAKE3 in keyed mode, with `key` as the 32-byte key and an empty message. The seven values that
`salt[0]` never takes are a statistical fingerprint, weak but real; the list is as short as
sharing a port with eD2K allows (§14.1), and a later `magic` can change it.

**Networks.** The public network has no name, and `net_id` is empty there. Any other network
has a `name` of 1 to 32 bytes that all its nodes are configured with.

- A datagram from a node of another network does not unmask to the `magic` and is dropped
  like any stray datagram (§4.5). The two networks never exchange a message, a contact or a
  record.
- A **test network** uses a name that is published.
- A **closed network** uses 32 random bytes that only its members know. Whoever has the name
  is a member; there is no way to remove one except a new name. Its `net_id` opens the network
  as the name does. A client MUST NOT write such a name, or its `net_id`, to a log or show it
  in an error message.
- A node is in one network per UDP port. Node lists and their sources
  ([`extensions/bootstrap.md`](extensions/bootstrap.md)) belong to one network. A list names
  its network by `H("ed3k v1 nodes net", net_id)[0..16)`, never by `net_id` or the name.
- The reasoning is in [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §2.2.

For `HANDSHAKE`, `FRAGMENT` and `PROBE` the whole datagram after the salt is masked. For `DATA`
only the header, the padding and the first 64 bytes of the QUIC packet are masked (all of it
when it is shorter): that covers the
QUIC flags byte and connection ID, and the rest is AEAD ciphertext already. This keeps the layer's
CPU cost on the bulk path at a few BLAKE3 compressions per packet (the key derivation and about
130 bytes of keystream). It is enough because a `DATA` datagram
holds one packet (above); a datagram with several packets is a `HANDSHAKE` and is masked whole.

**Overhead** is 13 bytes per datagram plus padding, about 1 % of a full data packet. A
`FRAGMENT` has 2 bytes more, 15 plus padding.

### 4.4 Size shaping

- A sender SHOULD split each `HANDSHAKE` datagram into 2–4 `FRAGMENT`s of random sizes, each with
  random padding, and MAY send it whole with random padding instead. Receivers MUST support both.
- The layer adds 13 bytes and the padding: `13 + pad_len + QUIC datagram` MUST NOT exceed the
  UDP payload the path carries. An unsplit `HANDSHAKE` that carries an Initial (1200 bytes)
  therefore uses `pad_len` of at most 19 while the path MTU is not known (1232 − 13 − 1200).
- **Fragments.** The pieces of a message, put together in the order of `idx`, are one QUIC
  datagram. The receiver hands it to QUIC as if it had arrived in a `HANDSHAKE` from that
  source address.
  - The sender picks `msg_id` at random per message and does not use a value again towards
    one address within 3 s *(initial)*. A datagram that QUIC sends again is a new message.
  - A `FRAGMENT` with `total` below 2, or with `idx` not below `total`, is dropped. The first
    piece that arrives fixes `total`; a piece of that message with another `total` is dropped.
    Of two pieces with one `idx` the first counts.
  - Messages are kept per (source address and port, `msg_id`). A receiver keeps at most 4
    *(initial)* incomplete messages per source address and port and drops one after 3 s
    *(initial)*. When a fifth starts, the oldest is dropped. A message whose pieces come to
    more than 1,500 bytes *(initial)* is dropped. All incomplete messages together hold at
    most 4 MiB *(initial)* and are at most 65,536 *(initial)*; beyond either the oldest are
    dropped. This bounds also messages made of empty pieces.
  - Whoever knows both addresses can fill the 4 places with forged pieces. A sender whose
    handshake got no answer after 2 *(initial)* attempts in fragments therefore sends the
    next attempt unsplit. Attempts are counted per connection and long-header packet type
    and are never reset: the third flight of a type and every later one go unsplit. A
    datagram that arrives is not an answer, since a path that drops fragments can still
    carry other packets.
- `DATA` datagrams SHOULD carry `pad_len = 0` once the packet is larger than 256 bytes; smaller
  ones SHOULD be padded randomly. A `DATA` datagram whose QUIC datagram is at the maximum size
  MUST carry `pad_len = 0`.
- **Anti-amplification.** A reassembled datagram is never larger than what arrived on the
  wire, so QUIC's count of the bytes it received stays conservative. Its count of the bytes
  it sent does not: this layer adds its header and padding to every datagram. Until the
  peer's address is validated, the bytes a node puts on the wire towards it, header and
  padding of this layer included, MUST NOT exceed 3 times the bytes that arrived from it on
  the wire (RFC 9000 §8.1). This holds again for a new client address after a migration,
  until QUIC has validated it (RFC 9000 §9.3). A receiver MUST NOT answer a `PROBE` and MUST
  NOT emit anything for an incomplete fragment set. The dial of a punch initiator to a
  `PROBE`'s source (§9.5 step 4) is not an answer: it needs the nonce of the initiator's own
  punch. With these bytes counted, a server whose flight of 1,200 bytes is lost twice may have
  no budget for a third, when its padding and fragment overhead exceed the client's; it sends
  again when the client's next Initial arrives.
- The QUIC maximum datagram size MUST be set 13 bytes below the path limit. A QUIC stack
  starts at 1,200 bytes, so that its Initials are 1,200 bytes, and raises the size only by
  path MTU discovery. An unsplit Initial without padding is then 1213 bytes, which fits the
  IPv6 minimum MTU (1232 bytes of UDP payload); with padding it fits up to `pad_len = 19`.
- The binding needs a path that carries 1,213 bytes of UDP payload. A handshake sent in
  fragments never puts a datagram of that size on the path, so the test that QUIC makes with
  its padded Initial (RFC 9000 §14.1) is missing. Each side checks the path once per
  connection, after the handshake; datagrams of the handshake do not count. The check passes
  when any datagram of 1,213 bytes or more on the wire that the side sent after the handshake
  is acknowledged, whatever it carries. Until then the side sends a PING padded to at least
  1,213 bytes and at most the path limit, and holds back file data (the bytes of an exchange
  from its first `Block` on, §12.3); other frames are not held back. When 3 *(initial)* such
  probes are lost it closes with `CLOSED`. A side that cannot send such a PING holds back
  nothing. This is the case when its QUIC maximum, or the peer's `max_udp_payload_size`, is
  1,200 bytes, so that no probe is larger than its datagrams. Its full datagrams of file data
  are then the probes. The check passes as above, and QUIC's loss detection and the idle
  timeout end a path that drops them. The check exists for file data: a side that sends no
  file data on the connection MAY skip it, and starts it before it sends its first `Block`
  there. The lost probes count from the first one the side sent on the connection, also when
  it started the check before it had to; nothing resets the count. A path MTU probe above
  the path limit is no such probe. An index node sends none. After a migration the check is not repeated:
  QUIC's path validation expands its datagrams to 1,200 bytes (RFC 9000 §8.2.1), 1,213 on the
  wire.

This follows the approach of Hysteria 2's Salamander (salted XOR mask) and Gecko (fragmenting
long-header packets), with a real keystream instead of a repeating 32-byte pad and without a
pre-shared password. Censors adapt; the datagram format is versioned by `magic` so the shaping can
change without touching QUIC or the messages.

### 4.5 Receiving

The steps are tried in order; a datagram that fails one goes to the next. Each step unmasks
the first 5 bytes only and checks `magic`.

1. If the source address and port belong to a live connection, or to a dial in progress,
   unmask with that connection's key: the `obf_id` and `dir` the peer uses towards this node
   (the salt enters the key per datagram). Two connections can share a source address and
   port for a while, after both nodes dialled at once
   (§4.6); they have different keys, and the key of each is tried.
   On a match, hand the datagram to that connection, unless its `kind` is `PROBE`: a `PROBE`
   falls through to step 2 whatever key it unmasked with. The kind byte is among the 5 bytes
   unmasked, so this costs nothing.
2. Otherwise unmask with the node's own `obf_id` and `dir = 0x00`. On a match, process it as a
   new flow, or as a `PROBE`. A `PROBE` is always keyed this way, also when it comes from an
   address that has a connection, which is why step 1 lets it through. A packet of an existing
   connection whose client side changed its address lands here too; QUIC's connection ID then
   migrates it, within its family: a packet of a connection from an address of the other
   family is dropped (§8.3).
3. Otherwise the datagram is not ed3k. Drop it, or hand it to another protocol sharing the socket
   (§14.1).

A random datagram passes step 2 with probability 2⁻³² (2⁻³¹ with the Kad datagram binding,
[`extensions/kad-datagram.md`](extensions/kad-datagram.md)).

A datagram from the server of a connection that arrives from an address the client does not
know matches no step and is dropped, as QUIC itself would drop it (RFC 9000 §9): the server
side of a connection, also the NATed side of a punched one, cannot change its address.

A QUIC stateless reset (RFC 9000 §10.3) works towards the client of a connection only: a
server that lost its state masks the reset under its own `obf_id` with `dir = 0x01`, which the
client unmasks in step 1. A client that lost its state has no key for the server's datagrams,
sends nothing, and the server waits for its idle timeout. An implementation MUST NOT rely on a
reset reaching a server.

### 4.6 Connection budget

A node MUST limit the new connections it accepts from one peer.

| Counted per | New connections per minute *(initial)* | When exceeded |
|---|---|---|
| node ID | 10 | the node ID is banned for 24 h *(initial)* |
| host address (IPv4 /32, IPv6 /64) | 30 | a further new connection from it is refused; nothing is banned |

- **Only completed handshakes count.** The address is then validated and the key proven (T1,
  T3), so a forged source address cannot get another node banned. A resumed connection counts
  like a new one. Nothing is counted, and nobody is banned, on packets before that point.
- While a ban lasts, the node closes a handshake of that node ID as soon as the key is known.
  The open connections of a banned node ID are closed.
- **The host address is never banned for its budget.** Several nodes share one address behind
  a carrier-grade NAT or in one /64, and a ban would lock all of them out for what one did. A
  connection beyond the address budget is closed with `BUSY` once its handshake completes;
  the connections already open from that address stay, and the address is admitted again as
  the minute moves on.
- The address budget is wider than the node ID budget for the same reason. A node with the
  `INDEX` capability MAY use a wider address budget.
- **Open connections.** A node keeps at most 64 *(initial)* open connections with peers at one
  host address and 256 *(initial)* with peers in one site
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1). A further connection from
  there is closed with `BUSY` once its handshake completes. Connections in both directions
  count, but only accepted ones are closed: whether a node dials beyond the limits is local
  policy. Such a connection still counts in the rows above. Private addresses are exempt where
  they are exempt from the host-address row (below). A node with the `INDEX` capability MAY use
  wider limits.
- An address ban, which only bad blocks cause (§12.3), drops the datagrams of that address
  that would start a new flow while it lasts.
- Private addresses (§8.2 (8)) are exempt from the host-address row and from address bans
  (§12.3) where they are acceptable: in a named network (§4.3) and for a peer found in the
  local network (§8.2 (8)). The node-ID row applies to them.
- A node has at most one connection per peer and address family. To stay within the budget of
  others, it MUST NOT open a second connection to a peer in a family in which it already has
  one with it, and SHOULD keep a connection it will use again open for the idle timeout. A
  connection in each family is allowed: reach tests (§9.3), the verification of a contact in
  each family (§6.3) and bootstrap (§10.3) need one per family.
- When two nodes dial each other at once in one family, two connections complete. Both sides keep the one
  whose client, the node that opened it, has the lower node ID compared as bytes, open nothing
  new on the other, and close it with `CLOSED` once no exchange is open on it and at least
  5 s *(initial)* have passed since it lost: exchanges that the peer opened on it before it
  knew can still be on their way.

A node that sent bad blocks is banned in the same way, for other times, and so is its host
address (§12.3).

Protection against floods of packets that never complete a handshake is optional and local:
[`extensions/flood-protection.md`](extensions/flood-protection.md).

---

## 5. Framing and common messages

### 5.1 Frame

Each message on an exchange is one frame:

```
tag  varint     (type << 1) | z          z = 1: body is zstd-compressed
len  varint     length of body in bytes
body            protobuf message of that type
```

`Block` (§12.3), `HopStream` ([`extensions/hops.md`](extensions/hops.md) §2.4), `HopConnect`
and `HopJoin` (§3.1 there) are the only messages followed by raw bytes outside the frame.
After each of the three hop messages, the first thing in the other direction is one status
frame, `HopOk` or `Error`, before any raw byte. An extension that
adds such a message says so
here; a node that does not support it answers the frame with `Error{UNSUPPORTED}` and closes
the exchange (§5.2), so the bytes that follow are never read as frames.

A frame header does not parse when `tag` or `len` is a varint of more than 10 bytes, when
`len` is above 1 MiB (§5.2), or when the exchange ends inside it. An exchange that ends, or
is reset, before the first byte of a frame holds no header: it has failed (§5.2), and the
connection stays open. A varint that is longer than its value needs is accepted (§0).

Types 1–63 keep the tag at one byte.

| Type | Message | Direction | Section |
|---|---|---|---|
| 1 | `Hello` | both, first exchange | §5.5 |
| 2 | `Error` | answer | §5.6 |
| 3 | `KadRequest` | request | §10 |
| 4 | `KadAnswer` | answer | §10 |
| 5 | `FileRequest` | downloader → uploader | §12 |
| 6 | `FileStatus` | uploader → downloader | §12 |
| 7 | `BlockRequest` | downloader → uploader | §12 |
| 8 | `Block` | uploader → downloader | §12 |
| 9 | `HashRequest` | downloader → uploader | §12 |
| 10 | `HashAnswer` | uploader → downloader | §12 |
| 11 | `Punch` | via relay | §9 |
| 12 | `Relay` | request and answer | §9 |
| 13 | `ReachTest` | request and answer | §9 |
| 14 | `Browse` | request | optional, §13.1 |
| 15 | `BrowseAnswer` | answer | optional, §13.1 |
| 16 | `Chat` | request | optional, §13.1 |
| 17 | `SlotOffer` | uploader → downloader | §12.5 |
| 18 | `ChatAck` | answer | optional, §13.1 |
| 19 | `LiveJoin` | viewer → peer | optional, §13.1 |
| 20 | `LiveState` | both | optional, §13.1 |
| 21 | `LiveGet` | viewer → peer, not answered | optional, §13.1 |
| 22 | `TopicJoin` | subscriber → peer | optional, §13.1 |
| 23 | `TopicState` | both | optional, §13.1 |
| 24 | `LanQuery` | request | optional, §13.1 |
| 25 | `LanAnswer` | answer | optional, §13.1 |
| 26–34 | `HopCreate`, `HopCreated`, `HopExtend`, `HopGate`, `HopLookup`, `HopLookupAnswer`, `HopStream`, `HopConnect`, `HopJoin` | between the nodes of a path | optional, §13.1 |
| 35 | `Receipt` | downloader → uploader | optional, §13.1 |
| 36 | `CreditRelay` | request | optional, §13.1 |
| 37 | `CreditRelayAnswer` | answer | optional, §13.1 |
| 38 | `BadBlock` | downloader → uploader | §12.3 |
| 39 | `HopOk` | status frame after `HopStream`, `HopConnect`, `HopJoin` | optional, §13.1 |
| 40 | `KeyRequest` | request | optional, §13.1 |
| 41 | `KeyAnswer` | answer | optional, §13.1 |
| 42 | `RoomJoin` | member → peer | optional, §13.1 |
| 43 | `RoomState` | both | optional, §13.1 |
| 44 | `NameQuery` | request | optional, §13.1 |
| 45 | `NameAnswer` | answer | optional, §13.1 |
| 48–63 | reserved | | §10.6 |

### 5.2 Exchange patterns

- **Request/answer:** the initiator opens an exchange, sends one request and half-closes. The
  responder sends one answer (or `Error`) and closes. There are no request IDs; the exchange is
  the correlation.
- **File stream:** one exchange per (downloader, uploader), open while blocks are transferred
  or while the uploader asks the downloader to stay (§12).
- An answer is never fragmented: the exchange is reliable, so a large answer is simply a long
  frame. A frame body MUST NOT exceed 1 MiB after decompression.
- A `len` above 1 MiB is a header that does not parse: the connection is closed with
  `PROTOCOL` (§4.1, §5.1). Once the peer has half-closed, a receiver also checks, before it
  reads a body, that `len` fits the bytes that remain; a frame that fails is answered
  `Error{MALFORMED}` without its body being read, and the exchange is closed.
- A responder that does not know a request type answers `Error{UNSUPPORTED}` and closes that
  exchange only.
- **Reset.** A side that gives an exchange up without an answer resets it: `RESET_STREAM` for
  the part it sends and `STOP_SENDING` for the part it receives, both with the application
  code 0. There is no other stream code; a receiver reads any code as 0. An initiator that
  cannot read an answer resets the exchange, since it has half-closed and cannot send an
  `Error`. An exchange that was reset has failed, and no answer is to be expected on it.
- On a request/answer exchange, what the initiator sends after its request is ignored.

### 5.3 Compression

Every node MUST be able to **decompress** zstd (RFC 8878) frames in the profile below. A sender
compresses a body only when the result is smaller, and SHOULD NOT try for bodies under 64 bytes
*(initial)*. File data is not part of a frame body; it follows a `Block` frame and has its own
rule and profile (§12.3).

Profile:
- magicless zstd frames, no content checksum, no content size field
- window size at most 1 MiB
- no dictionary (no `Dictionary_ID` in the frame header; a receiver also reads an ID of 0 so,
  and a sender never writes one), or the dictionary `ed3k-v1`,
  dictionary ID 1, shipped with this specification (`dict/ed3k-v1.dict`, trained on a corpus
  of real messages before the format is frozen)
- each body is compressed on its own; no context is carried between messages
- a receiver MUST stop and answer `Error{MALFORMED}` when the output exceeds 1 MiB (an initiator
  reading an answer resets the exchange instead, §5.2)

A sender MUST write this profile. A receiver MUST all the same accept a frame that carries a
content size or a checksum, and answers `Error{MALFORMED}` to a body that is not exactly one
zstd frame or whose window exceeds 1 MiB. Dictionary IDs are counted from 1: RFC 8878 keeps
the IDs below 32,768 for a public registry, which does not apply to magicless frames that
never leave ed3k.

**Dictionaries.** A dictionary never changes once published; a better one gets the next ID.
Every zstd frame that uses a dictionary carries its ID in the frame header, so a receiver always
knows which one to use; a frame without a dictionary is always accepted. A sender learns what
the peer holds from `Hello.dict` (§5.5):

- A node that announces `dict = n` MUST hold every dictionary from 1 to n. A node SHOULD hold
  dictionary 1 as soon as it is published; `dict` absent or 0 says that it holds none, which is
  expected of an incomplete implementation only.
- A sender MUST NOT use a dictionary with an ID above the peer's `dict`. Until the peer's `Hello`
  has arrived it sends without a dictionary or uncompressed.
- A receiver answers a frame that needs a dictionary it does not hold with `Error{UNSUPPORTED}`.

A new dictionary therefore needs no flag day, no capability bit and no new wire version: nodes
that have it use it with each other, and everyone else keeps using an older one.

Making *support* mandatory removes negotiation and every "does the peer do compression" variant.
Making *use* mandatory would enlarge the many messages that are mostly hashes and keys.

The alternatives considered (zstd without a dictionary, deflate, Brotli, LZ4, none) are compared in
[`docs/compression-comparison.md`](docs/compression-comparison.md).

**Side channels.** Compress-then-encrypt leaks length. It matters when a secret and
attacker-chosen text share one compression context. ed3k messages carry no secrets next to
third-party content and no context spans messages, so the CRIME/BREACH pattern does not apply.
An extension that adds secrets to a message MUST send that message uncompressed.

### 5.4 Common types

The fragments below and in every extension are excerpts of
[`proto/ed3k.proto`](proto/ed3k.proto), which is normative (§0).

```protobuf
syntax = "proto3";
package ed3k.v1;

// An endpoint is a bytes field: ip ‖ port(2, big-endian). 6 bytes = IPv4, 18 bytes = IPv6.
// A compact node is node_id(16) ‖ endpoint: 22 bytes (IPv4) or 34 bytes (IPv6).

message Peer {
  bytes id = 1;               // 16 B node ID
  bytes ep4 = 2;              // endpoint, 6 B
  bytes ep6 = 3;              // endpoint, 18 B
  bytes relays4 = 4;          // n × 22 B compact nodes; only for a node that is not open in a family (§9.4)
  bytes relays6 = 5;          // n × 34 B compact nodes; likewise
  // field 6 is reserved for other transport bindings (docs/transport-bindings.md)
}

message FileInfo {
  bytes  root = 1;            // 32 B BLAKE3 root (§7)
  uint64 size = 2;
  string name = 3;            // at most 255 bytes of UTF-8 (§10.7)
  bytes  ed2k = 4;            // 16 B eD2K MD4 hash; see §7.4
  uint32 sources = 5;         // complete sources known to the sender
  uint32 duration = 6;        // media length in seconds, 0 = unknown
  uint32 bitrate = 7;         // kbit/s, 0 = unknown
  // fields 8 and 9 belong to the optional file set extension (§13.1)
  uint32 partial = 10;        // sources known to the sender that hold the file in part (§10.3)
}
```

Compact byte strings replace `repeated` sub-messages where a list is long and its elements have a
fixed size: a compact IPv4 node is 22 bytes against about 30 as a sub-message (eMule Kad: 25).

### 5.5 `Hello`

The first exchange on every connection: `Hello` travels on the first bidirectional stream that
the connection's initiator opens (QUIC stream 0) and nowhere else. The initiator sends `Hello`,
the responder answers with its own. Either side MAY open other exchanges without waiting.
Streams can arrive in any order, so neither side answers an exchange that the peer opened
before it has read the peer's `Hello`: the responder the one on stream 0, the initiator the
answer to its own. A `Hello` on another exchange, or a second one, is answered
`Error{MALFORMED}`. A malformed `Hello` on stream 0 is answered `Error{MALFORMED}` by the
responder, which then answers nothing on that connection and closes it with `PROTOCOL` once
the `Error` is acknowledged, at the latest when the 30 s below end; an initiator whose
`Hello` is answered by a malformed one closes with `PROTOCOL`. A side closes with `PROTOCOL`
when no `Hello` of the peer has arrived on stream 0 within 30 s *(initial)* of the handshake,
and an initiator does so when the answer on stream 0 is not a `Hello`. A malformed `Hello` does
not count as a `Hello` for this 30 s rule.

```protobuf
message Hello {
  uint64 caps = 1;            // capability bits, §2
  string agent = 2;           // "eMuleQt 0.7.0"; at most 64 B
  bytes  observed = 3;        // your endpoint as I see it
  bytes  ep4 = 4;             // my advertised endpoints; absent = none in that family
  bytes  ep6 = 5;
  uint32 nat = 6;             // my class in the family of this connection: 0 unknown, 1 open, 2 cone, 3 symmetric (§9.2)
  string nick = 7;            // optional; at most 64 B
  // 8 and 9 belong to extensions that are not part of version 1 (§10.6, §6.3); 11 is reserved (docs/transport-bindings.md)
  uint32 dict = 10;           // highest zstd dictionary ID held (§5.3); 0 or absent = none
}
```

There is no user hash, no key and no signature in `Hello`: the transport has already delivered and
verified the key (T1), and the node ID is computed from it.

- `agent` and `nick` are at most 64 bytes each; a longer one makes the `Hello` malformed.
- A `nat` above 3 reads as 0.
- A receiver ignores capability bits that this version reserves or does not define.
- `ep4` and `ep6` are what the node advertises. One that is absent advertises nothing in its
  family, the family of the connection included: the observed endpoint does not take its
  place. In the family of the connection the address of an endpoint that is present is the
  observed one, whatever `Hello` says, and the port is that of `Hello`. A port that differs from the observed one is *claimed* until the node is dialled at it
  (§8.2 (5)). Which endpoint a node advertises is in §9.2.

### 5.6 `Error`

```protobuf
message Error {
  enum Code {
    UNKNOWN = 0;
    UNSUPPORTED = 1;          // message type or option not handled
    MALFORMED = 2;
    NOT_FOUND = 3;            // file not shared
    BUSY = 4;                 // queue full, store full, no relay slot
    DENIED = 5;               // banned, filtered, not allowed
    reserved 6;               // was NOT_RESPONSIBLE; a store at a node that is not responsible is
                              // answered with contacts, §10.3
  }
  Code   code = 1;
  uint32 retry_after = 2;     // seconds, a hint; 0 = no hint
}
```

An `Error` is never answered with an `Error`. Unknown codes read as `UNKNOWN`. `retry_after`
is a hint: the receiver does not ask again sooner. 0 is no hint, and the wait that the section
concerned names applies (30 min for a relay, §9.4); where it names none, 60 s *(initial)*.
A receiver reads a value above 24 h
*(initial)* as 24 h. `MALFORMED` is
for a body that does not parse, or a body that breaks a size or layout rule of §0, §5 or the
document concerned; a frame header that
does not parse, a `len` above 1 MiB included, closes the connection (§4.1, §5.1). A record that fails a check of its
content is answered per record (§10.3, §10.7), not with an `Error`.

---

## 6. Identity

### 6.1 Key and node ID

A node has one long-lived **Ed25519** key pair.

```
NodeID = H("ed3k v1 node id", public_key)[0..16)
```

The node ID is the only identifier of a node. It is its position seed in Kad (§6.3), the key of
credit records, the source ID in source records and the thing a dialler authenticates. The eD2K
user hash has no counterpart. Written as text, in a link, a file or directory name or a
display, a node ID is 32 hex digits, lower case; a reader accepts either case (§13.2).

**Why 128 bits.** Impersonating a node means finding a key whose ID equals the victim's: a second
preimage, 2¹²⁸ work. The 2⁶⁴ birthday bound only lets an attacker make two of their *own* keys
collide, which gains nothing. IDs are the most frequent field in the protocol (every contact,
every source), so 16 bytes instead of 32 is the single largest saving available.

**Why truncate, not fold.** XOR-folding a 256-bit hash to 128 bits is exactly as strong as
truncating it for a hash without structure. Truncation keeps the short ID a prefix of the full
hash, which leaves room to lengthen IDs later without renumbering the network.

**Alternatives considered:** 256-bit IDs everywhere (16 more bytes per contact and source); 256
bits in Kad and a folded 128 outside (two forms, and the saving misses Kad contact lists, where
most IDs are).

### 6.2 Secure identification

eD2K's secure identification (RSA-384, SHA-1, a challenge packet pair) is replaced by the
transport handshake (T1). There is no separate message. A node is identified from the first byte
of the first exchange, in both directions.

Ed25519 was chosen for 32-byte keys, 64-byte signatures, fast verification and universal library
support. The key type is fixed in version 1; a future type gets a new node-ID context string and
a new wire version. Signed records (§10.7) already carry a list of signatures, so a second
algorithm can be added to them without a new record format.

### 6.3 Routing ID: IP-bound, key-bound

If a node could choose its position in the keyspace freely, an attacker would generate keys until
one lands next to a target and then eclipse it. BitTorrent's BEP 42 prevents this by deriving the
top bits of the ID from the node's IP address, but its IDs are not tied to a key. ed3k combines
both: the position is fixed by the address in its top 21 bits and by the key in the remaining 107.

```
r          = NodeID[15] & 0x07

IPv4:  m = ip[0..4)  AND  03 0F 3F FF
IPv6:  m = ip[0..8)  AND  01 03 07 0F 1F 3F 00 00

both:  m[0] = m[0] OR (r << 5)
       c    = CRC32C(m)                              (Castagnoli, as in BEP 42)

RoutingID(address) = top 21 bits of c  ‖  bits 21..127 of NodeID
```

- A node has **one node ID** and one routing ID **per public address**. A dual-stack node sits
  at two positions with the same low 107 bits.
- `r` comes from the key, so nothing is added to the wire. As in BEP 42 an address can reach 8
  prefixes; an attacker picks among them by generating keys, which is cheap, and gains nothing
  beyond those 8.
- The IPv4 mask is BEP 42's. The IPv6 mask deliberately differs: BEP 42 hashes the top 64 bits,
  so one /48 reaches 2¹⁸ prefixes. Here only the top 48 bits count (1+2+3+4+5+6 = 21 bits,
  exactly the prefix length), so a /48 site reaches 8.
- Distance in Kad is XOR over routing IDs. Targets (§10.1) are plain 128-bit values.

**Verification.** A contact is *claimed* when it arrives in a `KadAnswer` and *verified* once a
connection to it has completed: the handshake proves the key (hence the low 107 bits), and QUIC's
address validation proves the source address (hence the top 21). Only verified contacts are
inserted into a routing table, handed out in answers, counted towards lookup termination or
used as storage targets. Verification costs nothing extra, because every node that is queried is
connected to anyway.

Private addresses (§8.2 (8)) are exempt in a named network (§4.3), which is what a LAN or a
test network is: such an address is acceptable there although §8.2 (7) rejects it elsewhere, the
check of the top 21 bits is skipped, and the routing ID of a node at such an address is its
node ID. The diversity limits exempt them too ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1).

**Test vectors** (CRC32C of the masked address; the first three match BEP 42):

| Address | r | CRC32C | top 21 bits |
|---|---|---|---|
| 124.31.75.21 | 1 | `5fbfbdb2` | `0bf7f7` |
| 21.75.31.124 | 6 | `5a3ce9b0` | `0b479d` |
| 65.23.51.170 | 6 | `a5d4344a` | `14ba86` |
| 2001:db8:1234:5678::1 | 3 | `32b2d72e` | `06565a` |

The IPv6 row uses a documentation prefix that §8.2 (7) rejects; only its CRC is a vector.

**Limits, stated plainly**

- The position moves when the address changes. The identity does not: credits, source records
  and sessions follow the node ID.
- On IPv4 an attacker needs on the order of 2¹⁶ addresses to place one node within a chosen
  19-bit region (each address reaches 8 of 2²¹ prefixes). On IPv6 the holder of a /32 controls
  11 of the 21 bits, so a few dozen /32 allocations, or one /23, do the same. IPv6 positions are
  therefore weaker than IPv4 ones.
- Far fewer bits are needed in practice. On a network of 100,000 nodes about 13 matching bits
  put a node among the closest to a key, and one IPv6 /29 or one IPv4 /16 reaches 131,072
  prefixes. The routing ID alone does not stop the holder of such a block; the diversity limits
  below are required for that reason.
- Nodes behind one carrier-grade NAT address share 8 prefixes. They are leaves in practice and do
  not appear in routing tables.
- BEP 42 itself has never been enforced by default in the BitTorrent DHT. ed3k can enforce it
  from the first version because there is no installed base to break.

**IP diversity limits (REQUIRED).** A node MUST apply the limits of
[`extensions/ip-diversity.md`](extensions/ip-diversity.md) wherever it builds a set of contacts:
per bucket, in the whole routing table, per answer, per lookup, in the closest set, and among
the publishers of a record. Contacts are counted in three groups per family: address, /24 and
/16 for IPv4; /64, /48 and /32 for IPv6. Each node applies the limits to what it accepts, so
they need no wire change, which is why they can be required from the first version.

**Extension reserved for a network upgrade.** Not part of version 1.

- *Key puzzle*: a `ROUTER` node shows a solution to a memory-hard puzzle (Argon2id, 64 MiB)
  bound to its key, its /24 or /48 and a 30-day epoch. It is a nonce search, so the node ID does
  not change, and it puts a recurring price on every router identity. Designed in
  [`extensions/key-puzzle.md`](extensions/key-puzzle.md); introduced with a capability bit and a
  date after which routers without a solution are treated as leaves. Capability bit 7 and field 9
  of `Hello` are reserved for it.

**Alternatives considered:** pure key hash with diversity limits only (an attacker with a handful
of subnets can sit next to any target); key grinding until the hash matches the address (about
2²¹ key generations at every address change, and the identity would change with the address);
a signed short-lived routing key under the identity key (a second key, a certificate and a grind
per address change).

### 6.4 The identity key in a key agreement

Where a document uses a node's Ed25519 key for X25519 (finding a known node, anonymous paths,
the datagram binding), the key is converted the standard way:

```
x_private = clamp( SHA-512(ed25519_seed)[0..32) )
x_public  = (1 + y) / (1 − y)  mod 2²⁵⁵−19          y = the Ed25519 public key's y coordinate
```

(libsodium: `crypto_sign_ed25519_sk_to_curve25519`, `crypto_sign_ed25519_pk_to_curve25519`.)
A node MUST refuse a public key whose y is 1 or −1 before it converts it (y = 1 has no
inverse, y = −1 gives x = 0), and MUST refuse a shared secret that is all zero. Using one key for signatures and, after
conversion, for X25519 is established practice; the context strings of each use keep the
results apart.

---

## 7. Files

### 7.1 Identity

```
root = BLAKE3(file contents)          32 bytes, default (unkeyed) mode
```

A file is identified by `root`. The size travels with it in every record but is not part of the
identity: a size in a record or from a peer is a claim until a block has been verified under
it (§12.3). The root is what the standard `b3sum` tool prints.

### 7.2 Verification

BLAKE3 is a Merkle tree over 1 KiB chunks. Any aligned range of 2ᵏ chunks is a subtree with its
own 32-byte chaining value, and so is a range that starts aligned and is cut short by the end of
the file. Parts of a file can therefore be verified against the root without the rest.

- **Block:** 256 KiB (2⁸ chunks). The unit of verification and of hash exchange.
- **Piece:** 4 MiB (16 blocks). The unit of availability bitmaps.

A downloader fetches block chaining values with `HashRequest` (§12.4), verifies them against the
root, and then verifies each received block on its own. A corrupt block is detected after 256 KiB
and its sender is known exactly. The hash data is 32 bytes per block, 0.012 % of the file.
A file of at most one block needs no hash exchange.

This replaces both the eD2K part hashes (MD4 over 9.28 MiB) and AICH (SHA-1 tree over 180 KiB
blocks, trusted by majority vote because the eD2K hash does not commit to it). In ed3k the root
commits to every block; there is nothing to vote on.

### 7.3 Choice of hash function

One function is used for files, node IDs, Kad targets, key derivation and the obfuscation
keystream. BLAKE3 is chosen because the hash tree is part of the function: the specification
does not have to define one, and the root is something a standard tool computes. It also
supplies keyed hashing, key derivation with context strings and an extendable output, so no
second primitive is needed for §4 and the node ID (§6.1). The cost is a thinner round margin than the
alternatives and one more dependency.

SHA-256 with a tree defined on top is the fallback if BLAKE3 is ever weakened: the root field is
32 bytes either way and the hash is named in every context string, so it can be replaced with a
wire version bump.

The comparison with the alternatives is in
[`docs/hash-comparison.md`](docs/hash-comparison.md).

### 7.4 The eD2K alias

`FileInfo.ed2k` carries the file's eD2K hash (MD4 root of the 9.28 MiB part hashes). Together with
`size` it is the eD2K identity, which lets a dual-protocol client resolve an eD2K link on ed3k and
download one file from both networks at once.

- In version 1 a sender that has computed the eD2K hash over its own complete, verified copy
  of the file MUST include it. A sender that has the pair only from others does not pass it on.
- A receiver MUST accept records without it and MUST NOT use it as a key in any ed3k message.
  Every request names a file by `root`.
- The pairing (`root`, `ed2k`) in a record is a **claim** until the receiver has verified data
  under both hashes. See §14.4.

A node MAY also publish the pairing as an `ALIAS` record of the scheme `ed2k`
([`extensions/alias.md`](extensions/alias.md) §2), so that an eD2K link is resolved without a
keyword search.

**The alias is expected to be dropped.** Nothing in ed3k depends on it: it is one optional field.
A later revision turns the MUST above into a MAY, and field 4 is then left reserved.

---

## 8. IPv4 and IPv6

### 8.1 One overlay

There is one keyspace, one record store per node and one node ID per node. Addresses of both
families live in the same routing table. A node with public addresses in both families holds two
positions (§6.3) and is the bridge between single-stack nodes.

**Alternative considered:** two separate overlays as in BitTorrent (BEP 32). Every contact in a
table is then dialable, but dual-stack nodes maintain and publish twice and the IPv6 overlay
starts small. Fully separate instances per family were rejected outright.

### 8.2 The dual-stack contract

1. A record (`Peer`, compact node) carries **at most one endpoint per family**.
2. A node MUST use the same UDP port number in both families when it can, and MUST advertise the
   real ports when it cannot.
3. Requests that return contacts state which families the asker can dial (`KadRequest.want`). A
   node answers with contacts of those families only.
4. A single-stack node inserts only contacts of its own family into its routing table.
5. In the family a connection arrived on, the address of an advertised endpoint is **observed**
   and therefore trusted for routing; a port that differs from the observed one, and an endpoint
   of the other family, are **claimed** until dialled.
6. Abuse limits count per IPv4 address and per IPv6 /64; a private address in a named network
   counts as IPv4 /32 or IPv6 /128 (§9.2).
7. An address is acceptable only if it is global unicast. For IPv4 this excludes at least
   `0.0.0.0/8`, `10.0.0.0/8`, `100.64.0.0/10`, `127.0.0.0/8`, `169.254.0.0/16`,
   `172.16.0.0/12`, `192.0.0.0/24`, `192.0.2.0/24`, `192.168.0.0/16`, `198.18.0.0/15`,
   `192.88.99.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `224.0.0.0/4` and `240.0.0.0/4`.
   For IPv6 the address MUST be inside `2000::/3`, and outside at least `2001::/32` (Teredo),
   `2001:2::/48`, `2001:10::/28`, `2002::/16` (6to4), `2001:db8::/32` and `3fff::/20`.
   That leaves out everything else, among it `::/128`, `::1`, `::/8`, `fe80::/10`,
   `fec0::/10`, `fc00::/7`, `::ffff:0:0/96`, `64:ff9b::/96`, `100::/64`, `5f00::/16` and
   `ff00::/8`. A library's "global unicast" test, such as Go's `IsGlobalUnicast`, is too
   loose: it accepts `::81.2.3.4`, `4000::1` and `fec0::1`. A source
   address that a socket reports as IPv4-mapped is treated as the IPv4 address it holds. An
   18-byte endpoint on the wire that holds an IPv4-mapped address is not acceptable, and
   neither is an endpoint with port 0. For the target of an HTTP fetch,
   [`extensions/http-source.md`](extensions/http-source.md) §5 judges an address of
   `64:ff9b::/96` or `2002::/16` by the IPv4 address inside it.
8. **Private addresses** are `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`,
   `169.254.0.0/16`, `127.0.0.0/8`, `::1`, `fe80::/10` and `fc00::/7`. Where a document says
   "private, loopback, link-local and unique-local", it means this list. `100.64.0.0/10` is
   neither private nor global unicast. A private address is acceptable, as a contact and as an
   address to dial, only in a named network (`net_id` not empty, §4.3; §6.3), and for a peer
   found in the local network ([`extensions/lan-discovery.md`](extensions/lan-discovery.md)
   §3). On the public network it is never inserted into a routing table, stored in a record
   or handed out.

### 8.3 Dialling

When a peer has endpoints in both families and so does the dialler, IPv6 is tried first and IPv4
250 ms *(initial)* later; the first handshake to complete wins. The dialler closes the other
one with `CLOSED` at once, also when it has completed, unless a rule needs a connection in
that family as well (§4.6); it counts in the budget of §4.6 like any connection. A dialler that
tries several addresses of a peer in one family, such as the source of a `PROBE` and `eps(T)`
(§9.5), runs one handshake per address and closes the others with `CLOSED` once one completes;
until then they are not a second connection in the sense of §4.6. A connection stays in the
family it was opened in. A side does not let it migrate to the other family (§4.5); the
entries of the routing table do not change with it (§10.2).

A node SHOULD advertise a stable IPv6 address rather than a temporary one (RFC 4941), and send
from the address it advertises.

---

## 9. NAT traversal

**NAT traversal is REQUIRED.** Every node MUST implement address observation (§9.2), act as punch
initiator and punch target (§9.5), act as A of a `ReachTest` (§9.3), and, when it is a
`ROUTER`, offer the relay service (§9.4). An index node offers it as §11 says.
A node behind a stateful IPv6 firewall is in the same position as a node behind an IPv4 NAT and
uses the same procedure.

### 9.1 Before traversal

A node SHOULD first try to become directly reachable with a port mapping (PCP, NAT-PMP, UPnP IGD;
for IPv6 a firewall pinhole). Traversal is for when that fails.

### 9.2 Address observation and classification

Every `Hello` reports the sender's view of the receiver's endpoint (`observed`). A node collects
these reports per local socket and address family:

| Class | Evidence |
|---|---|
| `open` (1) | the `PROBE`s of two `ReachTest`s the node asked of peers in different sites arrived within 60 min *(initial)* (§9.3), or unasked connections arrived (below), or the operator declared the node open (below) |
| `cone` (2) | one endpoint has the lead (below), and there is no evidence for `open` |
| `symmetric` (3) | one address has the lead (below), and no endpoint has it |
| `unknown` (0) | anything else: too few reports, or reports that name different addresses |

The rows are tried top-down; the first that fits gives the class.

**Which reports count.** A node that dials in can say anything in `observed`, and can do so from
many node IDs. A report is therefore counted only when all of this holds:

- It arrived on a direct connection that the node opened itself. A connection made by hole
  punching (§9.5) counts for its initiator only.
- It is the newest report from its site, where a site is the peer's /24 or /48
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1). Peers are never told apart
  by node ID.
- It is at most 60 min *(initial)* old. A connection that stays open does not refresh it: a
  report ages from the `Hello` that carried it.
- It names a public address. A report of a private, loopback, link-local or unique-local
  address, or from a peer reached at one, is not counted.

In a named network a private address is its own site: there the sites of this section, of
§9.3 and of §9.4 are host addresses (IPv4 /32, IPv6 /128). Likewise the host address of a
private address in a named network is IPv4 /32 and IPv6 /128 wherever ed3k counts per host
address: the reservations of §9.4, the punch and `PROBE` limits of §9.5, and the extensions.
The site limit of §9.4 then does not bind, since a site and a host are one address.

**The lead.** An endpoint has the lead when at least 3 *(initial)* counted reports name it, and
at least twice as many as name any other endpoint. Only an endpoint with the lead is adopted as
the node's own. One peer that reports a false port can therefore neither change the address nor
turn `cone` into `symmetric`; it takes two sites that agree on the same false port. Two such
sites suffice against 3 honest ones: the true endpoint then lacks the lead. An address has the
lead by the same rule, counting the reports that name it whatever their port.

While no endpoint has the lead, a node adopts the address and port of its own socket where no
NAT can be in front of it: when that address is acceptable (§8.2 (7)) and the operator has
declared the node open in that family (below), or when it is a private address in a named
network (§6.3), where no report counts. Without this the first routers of a network have no
routing ID and answer every `KadRequest` without contacts (§10.3).

**Advertised endpoint.** A `cone` node advertises in `Hello` the endpoint it adopted, with
the mapped port, since that is the port a punch aims at. A node of any other class advertises
the endpoint it adopted when it has one; else its best-known address, which is the one that
most counted reports name, else the address of its socket when that is acceptable, with the
external port of a port mapping (§9.1) when it has one, else the port of its socket. A node
that knows no address in a family leaves that family out.

**The first nodes of a network.** A reach test needs a `ROUTER` B (§9.3), so in a network that
has none, nobody becomes `ROUTER` by tests. An operator MAY therefore declare a node open per
family. The node is then `open` in that family without evidence and asks no `ReachTest`
there, and it sets `ROUTER` by §2.

**First lookups.** A node that has adopted no endpoint in a family has no routing ID there to
look up (§10.3), and it needs reports from 3 sites to adopt one. After the first `Hello` of a
`ROUTER` it dialled in that family, it takes the routing ID of the `observed` endpoint in it,
when that is acceptable, and looks up that ID and the two IDs that differ from it in the
first and in the second bit. Those lookups lead it to further sites. It does this once per
family.

**Unasked connections.** A connection that completes from a host the node has sent nothing to
within the last 60 min *(initial)* could not have passed a NAT or a stateful firewall. Two such
connections from different sites within 60 min *(initial)* are evidence for `open` in that
family, the same as the `PROBE`s of two tests (§9.3). Relayed and punched connections do not
count. Two are asked for because one can come through a mapping that an earlier exchange left
open.

**Testing again.** A class can go stale: a port mapping expires, a firewall rule changes.

- A node asks for a `ReachTest` every 60 min *(initial)* per address family, and at once when
  its local address or its observed endpoint changes. It asks its tests of peers in different
  sites in turn, so that no single peer decides its class.
- When no prober was asked (`ok` not set, or no answer), it tries again with another peer after
  30 s *(initial)*, doubling the wait each time up to 60 min *(initial)*.
- An `open` node whose tests had a prober asked (`ok = true`) and that received no `PROBE` in
  two *(initial)* such tests in a row is no longer `open`, unless unasked connections arrived since the first of the two. It classifies
  itself from its address reports and takes relays (§9.4).
- An `open` node MAY skip a test that is due while it has evidence from unasked connections
  that is less than 60 min *(initial)* old. A test after an address change is never skipped.

### 9.3 `ReachTest`

```protobuf
message ReachTest {
  bytes  target_id = 1;       // set when forwarded: node to probe
  bytes  target_ep = 2;       // set when forwarded: endpoint to probe
  fixed64 nonce = 3;          // chosen by the asker; goes into the PROBE as 8 B, big-endian
  bool   ok = 4;              // answer: a prober was asked
}
```

1. The asker sends `ReachTest{nonce}` to a peer A, on a connection the asker opened in the
   family to test. `nonce` is random and chosen by the asker, so that it can recognise the
   `PROBE`. The asker SHOULD NOT dial a contact of its routing table to ask it as A: its NAT may
   already hold a mapping for that host. A connection the asker already opened MAY serve as A,
   also to a contact of its table; step 5 judges the host the `PROBE` comes from.
2. A picks a `ROUTER` contact B of its routing table in that family that is not the asker and
   that A believes has no connection with the asker, and forwards the request to B on a new exchange
   with `target_id`, `target_ep` and the asker's `nonce`. `target_ep` MUST be the asker's
   observed endpoint as A sees it, never a value from the asker. A request that arrives with
   `target_id` set is never forwarded again.
3. B sends one `PROBE` datagram (§4.3) with the nonce to `target_ep`, which MUST be an
   acceptable address (§8.2 (7), and (8) in a named network), and answers A with `ReachTest{ok = true}`. It answers `ok = false` when it
   does not probe, and `Error{BUSY}` with `retry_after` at its limit.
4. A answers the asker with `ReachTest{ok}`: `ok = true` when a B agreed to probe, `ok = false`
   when A found no B or B declined. When a B declines or answers `Error{BUSY}`, A tries up to
   2 *(initial)* further ones first. A waits at most 3 s *(initial)* for the answer of each B;
   a B that has not answered by then has declined. The asker waits 10 s *(initial)* for A's
   answer, which covers the 3 waits of A; without an answer by then no prober was asked (§9.2).
5. A `PROBE` with the nonce counts from the moment the asker sent its request until 5 s
   *(initial)* after the answer, or until 10 s *(initial)* after the request when no answer
   comes: B probes before it answers. It is evidence that the asker is `open` in that family
   only when it comes from a host the asker has sent nothing to within the last 60 min
   *(initial)*, as for unasked connections (§9.2): a mapping that an earlier exchange left
   open would let it through. A `PROBE` with a nonce the node did not ask for is ignored. In a
   network with few `ROUTER`s a node may have sent to all of them while it bootstrapped; its
   tests then show nothing until 60 min have passed.

A `PROBE` is evidence only as far as A is honest: A knows the nonce, the asker's `obf_id` and
its endpoint, and could send the `PROBE` itself. A node that is not `open` therefore becomes
`open` only on the `PROBE`s of two tests asked of peers in different sites
([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1) within 60 min *(initial)*, as
§9.2 asks two sites of unasked connections; after the first `PROBE` it asks the second test at
once, of a peer in another site. A node that is `open` stays so by the rules of §9.2.

A limits these to 1 per asker address per minute *(initial)* and answers a further one
`Error{BUSY}` with `retry_after`. B limits them to 2 per address
of `target_ep` per minute *(initial)*, whichever A asks: the second test of an asker may reach
the same B.

B cannot check that `target_ep` is the asker's endpoint; a false A can name any acceptable
address. That is accepted: a `PROBE` is small and is never answered (§4.4), and the limit above
bounds how many one A can cause.

### 9.4 Relays

A node keeps reservations at no fewer than 2 *(initial)* relays, and at more where this
section says so, while it has an address in a family in which it is not `open`. That includes a `ROUTER` that is open in one family only.

```protobuf
message Relay {
  bool   reserve = 1;         // request: true = hold a slot, false = release
  uint32 ttl = 2;             // answer: seconds the slot is held without traffic
}
```

A relay holds the connection open and forwards `Punch` messages to the leaf. A leaf lists its
relays in every `Peer` record it publishes (`relays4`, `relays6`).

**A reservation** lasts as long as the connection it was made on and is not carried to another
one. The leaf sends the PING of §4.1, at an index node at the interval of §11; any packet
counts as traffic. `ttl` is informational:
the time the relay keeps the slot without traffic, at most the idle timeout (§4.1). The slot
ends with the connection. The leaf does not repeat its request. `Relay{reserve = false}` gives
the slot back and is answered `Relay{ttl = 0}`.

**One slot per leaf.** A leaf with a connection to the relay in each family (§4.6) MAY reserve
on both. The relay keeps one slot per leaf node ID and lets both connections hold it: a
`Reserve` on the second one is answered like the first, without a check of the limits below,
and neither connection is told of the other. The slot ends when the last connection that
holds it closes or gives it back. A forwarded `Punch` (§9.5) goes over the connection that
reserved last and is still open, and over the other one when that fails: when the exchange
cannot be opened there, or fails within the 5 s of §9.5 step 2. A timeout is not tried again.

**Listing.** The list a relay is in says how an initiator reaches the relay, not which family
the punch uses: `Punch` carries the endpoints of both. A relay is listed with an endpoint of
its own in that family, so a node prefers relays that have both.

| The node is not open in | `relays4` | `relays6` |
|---|---|---|
| IPv4 only, or it has no IPv6 | both relays | none |
| IPv6 only, or it has no IPv4 | none | both relays |
| both families | both relays | one of them |

A relay without an endpoint in the family of a list is left out of that list. A node whose
list would then be empty takes a further relay that has one. A list holds at most 2 relays
(§10.3). A node that holds more relays than a list takes lists, of the relays with an
endpoint in the family of the list, those with endpoints in both families first and, among
equals, the ones it chose last.

Why 2, with a comparison to one relay, to two per family and to libp2p (2 by default), is in
[`docs/relay-count.md`](docs/relay-count.md).

**Finding relays.** A leaf takes its relays from the `ROUTER` nodes it meets in its own Kad
lookups, and from the index nodes it is connected to (§11), which it does not meet in a
lookup when they leave `ROUTER` unset. An index node offers the service as §11 says.

- It asks nodes whose `Hello` has `RELAY` set, with `Relay{reserve = true}`. A node that
  answers `Error{BUSY}` is skipped and not asked again for 30 min *(initial)*.
- Its relays SHOULD be in different sites ([`extensions/ip-diversity.md`](extensions/ip-diversity.md)
  §1), so that one network going away does not take all of them.
- A relay that is lost is replaced at once. The new list goes out with the next refresh of
  each record. Only when a published list has no working relay left does the leaf publish its
  `Peer` records again at once.

**Capacity.** The two numbers are the two sides of the service: a leaf holds a reservation at
2 relays or more, and a relay serves many leaves. A `ROUTER` MUST be able to serve at least
1 leaf and SHOULD be able to serve at least 8 *(initial)* leaves at the same time, one reservation per leaf node ID, and MAY serve
more. It holds at most 2 *(initial)* reservations per host address and 4 *(initial)* per site
([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1), so that cheap keys from one
network cannot take every slot; a further one from that network is answered `Error{BUSY}`.
A node with the `INDEX` capability MAY use wider limits per host and per site, as for
connections (§4.6): behind a carrier-grade NAT many of its clients share one address, and it
SHOULD serve far more than 8 leaves. The forward limits below are per leaf and stay as they are.
When all are taken it answers `Error{BUSY}` and clears `RELAY` in its `Hello` until one
is free. A peer that is already connected does not see the cleared bit; for it `Error{BUSY}`
is the signal (§2).

**Forward limit.** Every forwarded `Punch` makes the leaf send probes for 5 s (§9.5), so a
relay limits what it forwards:

| Counted per | Limit *(initial)* |
|---|---|
| leaf | 30 forwards per minute |
| initiator host address and leaf | 3 forwards per minute |

Above a limit the relay answers the initiator with `Error{BUSY}` and `retry_after`, and
forwards nothing. The second limit matches the 3 attempts of §9.5. A leaf whose relays are
often at the first limit SHOULD take a further relay.

**A relay forwards signalling only.** It never carries file data or any other exchange between two
peers. That keeps the service cheap enough to be an obligation for every reachable node and
useless as a proxy. The consequence is that a pair whose punch fails cannot transfer.

**Alternatives considered:** a rate-limited data relay as a last resort (connects every pair, but
relays pay bandwidth for strangers and need quota and abuse rules); a multi-port "birthday" probe
for symmetric NATs (hundreds of probe packets per attempt, which trips router session limits and
looks like a scan). Both can be added later as capabilities without changing §9.5. A path of one hop
([`extensions/hops.md`](extensions/hops.md)) carries data for such a pair, at the cost and under
the limits of that extension. Its last hop reaches a leaf through the leaf's relays (§9.5),
which `HopConnect` names.

### 9.5 Hole punching

```protobuf
message Punch {
  bytes  peer_id = 1;         // to the relay: the target; to the target: the initiator; in the target's answer: the initiator; in the relay's answer to the initiator: the target
  bytes  ep4 = 2;             // sender's endpoints; the relay replaces the one of the family it received the Punch on (steps 2 and 4)
  bytes  ep6 = 3;
  uint32 nat = 4;             // sender's class in the family of its own hop to the relay, which `seen` names to the receiver, §9.2
  fixed64 nonce = 5;          // chosen by I, random; forwarded unchanged; T copies it into each PROBE as 8 B, big-endian; 0 in T's answer
  uint32 seen = 6;            // set by the relay only: bit 0 = ep4, bit 1 = ep6 is the endpoint the relay itself observed; the other is claimed
}
```

```
initiator I                        relay R                          target T
  ── Punch{peer_id=T, eps(I), nonce} ──►   forward over T's reservation
                                    ── Punch{peer_id=I, eps(I), nonce} ──►
                                                                     start PROBEs{nonce} to eps(I)
                                    ◄── Punch{peer_id=I, eps(T)} ──
  ◄── Punch{peer_id=T, eps(T)} ──
  dial QUIC to eps(T)
```

1. I dials the endpoints of T first, where it has any (§8.3). When no handshake has completed
   2 s *(initial)* after the first direct attempt, or at once when T has no endpoint I can
   dial, I opens an exchange to one of T's relays and sends `Punch` with a random `nonce` it
   chose; the direct attempts go on. When that relay cannot be reached, gives no answer within
   7 s *(initial)*, or answers with anything but `Error{BUSY}` or a `Punch` for T, I asks the
   next relay of T in the same try. Such an answer is any other `Error`, `NOT_FOUND`,
   `UNSUPPORTED`, `DENIED`, `MALFORMED` and unknown codes included, or a `Punch` whose
   `peer_id` is not T's or that breaks a rule of this section. An endpoint at an address that is
   not acceptable (§8.2 (7), and (8) in a named network) breaks no rule: it is only left out,
   and the answer declines when no endpoint is left. An `Error{BUSY}` ends the try: it
   can come from R or from T (step 4).
2. R forwards it to T on a new exchange, with `peer_id` replaced by I's ID, the endpoint of
   the family the `Punch` arrived on replaced by I's observed endpoint as R sees it (as §9.3
   does for `target_ep`) when I's `Punch` holds one there (a family I left out stays
   declined; the other family stays as I claimed it), `seen` set to the family the
   `Punch` arrived on, also when I declined that family, and the rest unchanged. `seen` is
   also the mark of a forwarded `Punch`, so it is never 0 here. I's `Punch` has `seen` 0; one with `seen` set is not a
   request to R as a relay (**Who may ask**).
   R answers `Error{NOT_FOUND}` if T holds no reservation. It waits 5 s *(initial)* for T's
   answer and then answers `Error{BUSY}`.
3. T answers with its own endpoints, those it advertises (§9.2); in the family of its
   reservation, when it advertises none there, with the `observed` of the relay's `Hello`. Its
   `nonce` is 0, and I ignores it. T at once starts
   sending `PROBE` datagrams that carry I's `nonce` to each of I's endpoints in the families
   both share and T does not decline (**Class and family**), at random intervals of 100–300 ms *(initial)* per endpoint for 5 s
   *(initial)*. These open T's mapping. T sends a `PROBE` to an acceptable address only (§8.2 (7), and (8) in a named network),
   and at most 3 *(initial)*
   to an endpoint that `seen` does not mark: that one is a claim of I, and a false one would
   aim the probes at a third party.
4. R returns T's answer to I, with `peer_id` set to T's ID, the endpoint of the family of T's
   reservation replaced by T's observed endpoint when T's answer holds one there (a family T
   left out stays declined), and `seen` set to that family, also when T left it out. I dials T
   with a normal QUIC handshake; its retransmitted Initials open I's mapping. I is the QUIC client, T the server. A `PROBE` with I's `nonce` that
   reaches I before the answer, or within 5 s *(initial)* after it, tells I the address T's
   packets leave from: when that address is acceptable, I dials it at once, beside the dials
   to `eps(T)`. One handshake runs per address; an attempt that already dials an address of
   the answer is started again when the answer arrives, so that its Initials fall into T's 5 s
   of `PROBE`s. A `PROBE` with a nonce I did not choose is ignored (§9.3).
   An `Error` from T is returned to I unchanged, `retry_after` included.
5. A try has failed when no handshake with T has completed 8 s *(initial)* after the relay's
   answer, or when it ended at `Error{BUSY}`, or when no relay of T is left to ask. I tries up to 3 times *(initial)*, and for the next
   try prefers a relay that the last try did not ask. After that T is marked unreachable for
   30 min *(initial)*, and at once after an answer of T that declines (**Class and family**).

**Who may ask.** A forwarded `Punch` (`seen` not 0) whose `seen` is not 1 or 2 is
`Error{MALFORMED}`, wherever it arrives. T acts on any other forwarded `Punch` only when it
arrives on the connection to a relay at which T holds a reservation, and answers it
`Error{DENIED}` elsewhere. A
`Punch` with `seen` 0 is a request to the node as a relay; a node that
offers no relay service (§9.4) answers it `Error{UNSUPPORTED}`. A node can be a relay of a
peer and a leaf at it at once and tells the two apart by `seen`, which only a relay sets.
T starts at most 30 *(initial)* punches per minute, and 3 *(initial)* per minute towards one
host address, and answers `Error{BUSY}` beyond that. It sends at most 50 *(initial)* `PROBE`s
per punch, shared evenly over I's endpoints, and at most 100 *(initial)* per minute to one
destination host address over all punches; beyond that it sends none to that host. The limits
of the relay (§9.4) bound an honest relay only.

**Class and family.** `nat` in `Hello` is the sender's class in the family of the connection.
`nat` in `Punch` is the sender's class in the family of its own hop, the connection on which it
sent the `Punch` to the relay; the relay forwards it unchanged, and the receiver learns that
family from `seen` (steps 2 and 4). T declines a family by leaving its endpoint out of its
answer; an answer without an endpoint declines the punch. A declining answer ends the try,
also when `PROBE`s of T arrived, and I does not punch T again for 30 min *(initial)* (step 5);
a handshake that already completed stands. An answer whose acceptable endpoints are all in
families I cannot dial is no decline, but it leaves no family to try: it ends the try the
same way.

Pairs that cannot work, `symmetric` on both sides in a family both need, are not punched. I
does not know T's class before T answers, and `nat` holds for the family of one connection
only, so the two sides decide in turn. T declines such a family, judging I by its `nat` in the
family that `seen` marks. I judges T by the `nat` of the answer in the family that `seen`
marks; when that leaves no family to try, it does not punch T again for 30 min *(initial)*.
IPv6 is preferred when both sides have it, since a stateful firewall has no port mapping to
guess.

Measurements of the same procedure in libp2p (DCUtR) put the success rate near 70 % of attempts
across all NAT types and above 80 % for port-restricted cone NATs.

### 9.6 Why a self-contained design

The traversal above is four messages on top of a QUIC library that lets the application own the
UDP socket. Existing stacks (libp2p, iroh, ICE/STUN/TURN) were not adopted: none leaves room for
the obfuscation of §4, and each brings its own socket, runtime or servers. The procedure of §9.5
is libp2p's DCUtR, and §9 is kept close to the QUIC drafts for address discovery and NAT
traversal, so that it can be replaced by the standard frames in a later wire version.

The comparison is in [`docs/nat-traversal-comparison.md`](docs/nat-traversal-comparison.md).

---

## 10. Kad

### 10.1 Targets

| Record kind | Target |
|---|---|
| `SOURCE`, `NOTE`, `MEMBER` | `root[0..16)` |
| `HEAD` | `H("ed3k v1 set head", key ‖ id)[0..16)`, with the author's key and the `id` of the set ([`extensions/file-set.md`](extensions/file-set.md) §5.1) |
| `LIVE` | `H("ed3k v1 live", key ‖ id)[0..16)`, with the key and the `id` of the channel ([`extensions/live.md`](extensions/live.md) §1.1). A private channel uses a target that changes every hour (§10.4 there) |
| `HIDDEN` | `H("ed3k v1 hidden head target", base ‖ period(8))[0..16)`, with a value made from the key of the set and the number of the day ([`extensions/hidden-set.md`](extensions/hidden-set.md) §2) |
| `PART` | `H("ed3k v1 parts", key ‖ id ‖ issuer key)[0..16)`, with the owner key and the `id` of the set and the key of the issuer whose certificate the record carries, or no issuer key ([`extensions/multi-author-set.md`](extensions/multi-author-set.md) §3.4) |
| `FRIEND` | `H("ed3k v1 friend target", k ‖ period(8))[0..16)`, with a value made from the keys of two friends, or from a friend link, and the number of the day ([`extensions/friend.md`](extensions/friend.md) §4). A request to become a friend has a target of its own (§3 there) |
| `ANON` | `root[0..16)`, as `SOURCE`: a source that is reached through other nodes and has no address ([`extensions/hops.md`](extensions/hops.md) §4.3) |
| `GATE` | `H("ed3k v1 gate", node ID)[0..16)`, with the node ID of such a source (§4.3 there) |
| `ALIAS` | `H("ed3k v1 alias", len(scheme) as 1 byte ‖ scheme ‖ id)[0..16)`, with the kind of name and the name ([`extensions/alias.md`](extensions/alias.md) §1) |
| `SUB` | `H("ed3k v1 topic", type(1) ‖ key ‖ id)[0..16)`, with the type, the key and the `id` of the topic ([`extensions/pubsub.md`](extensions/pubsub.md) §1). A room stores `SUB` records under a target of its own ([`extensions/room.md`](extensions/room.md) §1.2) |
| `NAME` | `H("ed3k v1 name", zone ‖ label)[0..16)`, with the root key of a zone and a name in it ([`extensions/name.md`](extensions/name.md) §3.2) |
| `CLAIM` | `H("ed3k v1 name claim", name)[0..16)`, with a global name ([`extensions/name.md`](extensions/name.md) §4.2) |
| `KEYWORD` | `H("ed3k v1 keyword", level(1) ‖ word)[0..16)`, `word` being one keyword of the file name in UTF-8, as defined below. `level` is one byte, 0 on Kad in version 1 and 0 to 15 at an index node (§10.8, §11) |

File hashes, node IDs and keyword keys all come from the same function and are cut to the same
128 bits, so they share one keyspace without conversion.

**Words and keywords.** Publisher, searcher and storing node MUST split a name the same way:

1. **Normalise.** Apply Unicode NFKC, then full case folding, then NFKC again. Remove code
   points of general category Cf (soft hyphen, zero-width joiners). A text is normalised once,
   and a word from this split is never normalised again: Cf is removed after the last NFKC,
   so a Cf code point between a letter and a combining mark leaves them uncomposed, and a
   second pass would compose them into other bytes.
2. **Split.** A word is a maximal run of code points of the general categories L (letter),
   M (mark) and N (number). Every other code point separates words.
3. **Scripts written without spaces.** Inside a word, a maximal run of *unspaced* code points is
   replaced by its overlapping pairs: a run of n ≥ 2 gives n − 1 two-character words, and a run
   of 1 gives that character. The parts of the word before and after the run are words of their
   own. Unspaced code points are:

   | Script | Code points |
   |---|---|
   | Han, Hiragana, Katakana | U+3005–3007, U+3040–30FF, U+31F0–31FF, U+3400–4DBF, U+4E00–9FFF, U+20000–323AF |
   | Thai | U+0E00–0E7F |
   | Lao | U+0E80–0EFF |
   | Khmer | U+1780–17FF |
   | Myanmar | U+1000–109F, U+A9E0–A9FF, U+AA60–AA7F |

   Pairs are pairs of code points. In Thai, Lao, Khmer and Myanmar a vowel sign or tone mark is a
   code point of its own, so a pair may begin or end with one. Tibetan needs no pair rule: its
   syllables are divided by a punctuation mark (U+0F0B), so step 2 already yields one word per
   syllable.
4. **Extension.** If the text after the last `.` of the normalised name is 1 to 8 code points,
   all of category L or N, it is the extension of the name. Otherwise the name has none, and
   so does a name without a `.`.

The **words** of a name are the distinct results of steps 1 to 3. Its **keywords** are those
words that are at least 2 bytes long in UTF-8, without the words that steps 2 and 3 make of
the extension (a word, the pairs of an unspaced run, or both),
also where such a word occurs elsewhere in the name: words are distinct, so `Avi.Love.avi` has
the keyword `love` alone.
Targets are derived from keywords only, and a storing node refuses a record under any other
target (§10.7). `Query` (§10.3) matches against words.

| Name | Words | Keywords |
|---|---|---|
| `The.Big_Sleep (1946) [1080p].mkv` | `the` `big` `sleep` `1946` `1080p` `mkv` | `the` `big` `sleep` `1946` `1080p` |
| `Ça-va_ÉTÉ.MP3` | `ça` `va` `été` `mp3` | `ça` `va` `été` |
| `Straße.ﬁnal.txt` | `strasse` `final` `txt` | `strasse` `final` |
| `東京物語 Tokyo Story.avi` | `東京` `京物` `物語` `tokyo` `story` `avi` | `東京` `京物` `物語` `tokyo` `story` |

- An implementation MUST use the tables of Unicode 16.0.0 or later; the examples above are
  checked against that version. The general category of a code point that Unicode 16.0.0
  assigns is taken from that version, also by an implementation with later tables, and the
  ranges of step 3 are the table above and nothing else. Unicode 17.0.0 changes no category
  that this split tells apart (L, M, N, Cf, other) for a code point that 16.0.0 assigns, and
  no normalisation or case folding: an implementation with 17.0.0 tables complies as it is.
  The code points that 17.0.0 adds after U+323AF (CJK Extension J) are outside the table of
  step 3 on purpose, so that all versions split them alike. The Unicode stability policy keeps normalisation and case
  folding of an assigned character fixed across Unicode versions. A code point that an
  implementation does not know is treated as a separator, so a storing node with older tables
  than a publisher can refuse a record whose keyword uses a character assigned later; keeping
  the tables current bounds that.
- **2 bytes is the limit of the protocol; 3 bytes is the suggested default.** Two-byte words
  are few and very common (`of`, `to`, `la`, `de`), so they are the most crowded targets. By
  default a publisher SHOULD publish a file under its keywords of 3 bytes and more, and under
  its 2-byte keywords only when the name has no longer one (`AC/DC`). A searcher SHOULD derive
  the target from a keyword of 3 bytes or more when its search text has one.
- A name whose words are all 1 byte long has no keyword and cannot be found by a keyword
  search.
- A publisher SHOULD publish a file under at most 40 *(initial)* keywords, taking them in the
  order they occur in the name.

### 10.2 Routing table

Buckets of K = 10 *(initial)* verified contacts, ordered by XOR distance between routing IDs. Each
entry is one (node ID, endpoint) pair; a dual-stack node contributes two entries at two positions.
The buckets of each family are formed by the distance to the node's own routing ID of that
family (§6.3). When that routing ID changes with an address change, the entries of the family
are bucketed again under the new one. A node is inserted into the table of a family only when
it set `ROUTER` and its `Hello` on a connection of that family says `nat = 1`. A contact
learned another way, from a `KadAnswer` or a Kad datagram
([`extensions/kad-datagram.md`](extensions/kad-datagram.md) §3.4), waits for such a `Hello`. A
contact whose `Hello` on a later connection of that family no longer has `ROUTER` set and
`nat = 1` is removed from that family's table.

A contact is dropped when two attempts fail in a row: a connection attempt that fails, or an
answer that counts as a failed attempt (§10.3, §10.4). Failed attempts are counted per contact.
Only a `KadAnswer` that counts no failed attempt resets the count. An `Error` neither counts
nor resets it, and a completed connection does not reset it. The second attempt is made 5 min *(initial)* after the first
failed.

**Maintenance.** A bucket that no lookup has touched for 60 min *(initial)* gets a lookup for
a random ID in its range. A contact that has not been verified for 60 min *(initial)* is
connected to again and sent a `KadRequest`; a failure counts toward the two above. Both run in the background and are
spread over the hour, so that a quiet node neither keeps dead contacts nor bursts.
Insertion is subject to the per-bucket and per-table limits of
[`extensions/ip-diversity.md`](extensions/ip-diversity.md). The limits hold for the contacts in
the table at all times: if they ever become stricter, the node checks the table again and takes
out what no longer fits (§2.1 there).

**Contacts that dialled in.** A contact is *dialled-in* when the node first verified it on a
connection that the contact opened. At most half of a bucket, 5 of K = 10 *(initial)*, may be
dialled-in contacts; a further one is not inserted. A dialled-in contact is inserted with the
endpoint observed on its connection, address and port, not with the port of its `Hello`
(§5.5), as a source record is (§10.3). The mark stays while the contact is in the
table, also after the node has connected to it itself. Opening connections to a node is the
cheapest way to enter its table, and the contacts a node finds by its own lookups are not chosen
by the other side.

### 10.3 The one request

There is a single Kad request. It always asks for closer nodes and optionally carries an
operation that the receiver performs if it is responsible for the target.

```protobuf
enum Kind { NONE = 0; SOURCE = 1; KEYWORD = 2; NOTE = 3; HEAD = 4; MEMBER = 5; LIVE = 6; SUB = 7; HIDDEN = 8; PART = 9; FRIEND = 10; ANON = 11; GATE = 12; ALIAS = 13; NAME = 14; CLAIM = 15; }

message KadRequest {
  bytes  target = 1;          // 16 B
  uint32 want = 2;            // bit 0: IPv4 contacts, bit 1: IPv6 contacts; 0 = no contacts
  Kind   kind = 3;            // NONE = find nodes only
  Query  search = 4;          // present, also when empty: search for records of `kind`
  Store  store = 5;           // present: store records of `kind`; never together with `search`
  uint32 level = 6;           // KEYWORD: level the target was computed for (§10.1); 0 on Kad in version 1, 0 to 15 at an index node (§11)
}

message Query {
  repeated string all = 1;    // every word must be a word of the name (§10.1); normalised by the sender
  repeated string none = 2;   // no word may be a word of the name; normalised by the sender; at most 64 terms with `all`
  uint64 min_size = 3;
  uint64 max_size = 4;        // 0 = no limit
  uint32 min_sources = 5;     // compared with FileInfo.sources of the record
  string ext = 6;             // extension of the name (§10.1 step 4) without the `.`, normalised by the sender; empty = any
  uint32 offset = 7;          // skip this many matching records, in the order of the paging rule below
}

message Store {
  Peer     source = 1;        // SOURCE, and the kinds stored like it (LIVE, SUB): my endpoints and relays; `id` MUST be empty
  repeated Signed files = 2;  // KEYWORD: files whose name contains the keyword (§10.7); a Store holds at most 256 records (§10.3)
  Signed   note = 3;          // NOTE (§10.7)
  // field 4 belongs to the optional HTTP source extension (§13.1)
  // fields 5 and 7 belong to the optional file set extension (§13.1)
  bytes    signer = 6;        // 32 B key under which the signed records of this Store are checked and counted, when it is not the key of the connection (§10.7)
  bytes    also = 8;          // SOURCE: n × 16 B further targets that get the same record (§10.4); at most 256
  // field 9 belongs to the optional extension for finding a known node (§13.1)
  // field 10 belongs to the optional extension for anonymous paths (§13.1)
  // field 11 belongs to the optional extension for other names of a file (§13.1)
  // field 12 belongs to the optional extension for links at file hosters (§13.1)
  // field 13 belongs to the optional extension for readable names (§13.1)
}

message Note {
  uint32 rating = 1;          // 1..5, 0 = none
  string text = 2;            // at most 512 B of UTF-8
}

message KadAnswer {
  bytes  nodes4 = 1;          // n × 22 B compact nodes, closest first
  bytes  nodes6 = 2;          // n × 34 B
  bool   responsible = 3;     // this node considers the target inside its zone
  uint32 load = 4;            // 0..100: files held under this target, in percent of the level size, rounded down (§10.8)
  repeated Peer     sources = 5;
  repeated Signed   files = 6;  // §10.7
  repeated Signed   notes = 7;  // §10.7
  bool   more = 8;            // further results exist; ask again with `offset`
  repeated bytes    keys = 9; // 32 B publisher keys, referenced by Signed.key
  repeated StoreResult results = 10;  // one per record of the Store, in order, then one per target of `also`; absent = all stored; entries missing from a shorter list read as STORED
  bool   spill = 11;          // this node answered FULL for this target within the last record lifetime
  repeated uint32   sites = 12;       // one per entry of `files`, of `sets` in a MEMBER answer, of `aliases`, or of `names` in a CLAIM answer: publisher sites counted; 0 = not counted
  // field 13 belongs to the optional HTTP source extension (§13.1)
  // field 14 belongs to the optional file set extension (§13.1)
  // field 15 belongs to the optional extension for finding a known node (§13.1)
  // fields 16 and 17 belong to the optional extension for anonymous paths (§13.1)
  // field 18 belongs to the optional extension for other names of a file (§13.1)
  // field 19 belongs to the optional extension for links at file hosters (§13.1)
  // field 20 belongs to the optional extension for readable names (§13.1)
}

enum StoreResult {
  STORED = 0;
  FULL = 1;                   // no room under this target (§10.8)
  BUSY = 2;                   // no room on this node, or the sender is over its request rate; says nothing about the target
  REJECTED = 3;               // failed a check (§10.7), or over a limit per address or per publisher on what is held
  FAR = 4;                    // a target of `also` this node is not responsible for
  OLDER = 5;                  // a newer or winning record is held under this target; nothing changed (§10.7)
}
```

This one pair replaces 13 Kad2 opcodes: `REQ`/`RES`, three search requests and their answer, three
publish requests with answer and acknowledgement, and bootstrap (a `KadRequest` for one's own
routing ID, one lookup per public address (§6.3);
where the first address comes from is in [`extensions/bootstrap.md`](extensions/bootstrap.md)).
`Hello` (§5.5) replaces the Kad hello pair, ping/pong and the firewall check packets; §9 replaces
the buddy packets.

Rules:

- A node returns up to 16 *(initial)* verified contacts per requested family that are closer to
  the target than itself. It returns none when it knows none closer. "Itself" is its routing ID
  for the public endpoint it has adopted (§9.2) in the family of the connection, for `nodes4`
  and `nodes6` alike. A node that has adopted none in that family returns no contacts and does
  not set `responsible`, unless it is an index node: an index node is responsible for every
  target whether or not it has adopted an endpoint (§11), and only its contacts need the
  routing ID.
- Every node answers a `KadRequest` with the contacts it knows, a leaf too. Only a `ROUTER` that is
  `open` in the family of the connection (§9.2), or an index node (§11), is responsible.
- **The requester checks the answer.** It computes the routing ID of each returned contact from
  the contact's address and node ID (§6.3). It drops a contact that is not closer to the target
  than the answering node, and every contact after the 16th of a family. Closer is judged, in
  `nodes4` and `nodes6` alike, against the answerer's routing ID in the family of the
  connection, which the answerer judged by (above) and the requester computes from the
  answerer's address on that connection; the requester never judges by a routing ID of the
  answerer in the other family. An answer with a
  contact after the 16th counts as one failed attempt for the answering node (§10.2); a
  correct node cannot produce one. A contact that is not closer is only dropped: a node whose
  address has just changed can return one honestly. This check comes before the filters of
  [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.2.
- **Responsibility.** A node is responsible for a target when, in the address family of the
  connection the request arrived on, it knows fewer than K verified contacts of that family
  closer to the target than its own routing ID of that family. The other family is not
  considered. It sets `responsible`, executes a `search`, and accepts a `store`.
- **Search or store.** A request is a search when `search` is present, also when it is empty,
  and a store when `store` is present. A request with both is answered `Error{MALFORMED}`.
  With neither, the node returns contacts only, whatever `kind` says. It still sets
  `responsible` as below, and, for a kind it stores, `load` and `spill` (§10.8), so that a
  lookup learns them without a search. `all` and `none` of a
  `Query` hold at most 64 *(initial)* terms together; a search with more is answered
  `Error{MALFORMED}`.
- **Order of checks.** A node checks a `KadRequest` in this order, and the first step that
  decides gives the answer:
  1. It parses and keeps the size and layout rules of §0 and of this section, else
     `Error{MALFORMED}`. An `also` that is not n × 16 bytes in a request of kind `SOURCE`
     breaks such a rule.
  2. `search` and `store`: both is `Error{MALFORMED}`; neither is answered with contacts only.
  3. `kind` and responsibility. `kind = NONE` with a `search` or a `store` is answered with
     contacts only, and so is a `kind` the node does not know, or a node that is not
     responsible (below). `level` is ignored for a kind other than `KEYWORD`; a `KEYWORD`
     search with `level` above 15 is answered as by a node that is not responsible. A
     responsible node answers `Error{MALFORMED}` to a `Store` that holds no record of `kind`.
  4. Each record, answered in `results` (below, §10.7).
- A request with `want = 0` asks for records only: the node returns no contacts. It is for a
  requester that already knows the responsible nodes, such as a client of an index node (§11).
- **Paging.** Each node pages the results of a search in an order of its own, which a
  publisher cannot choose: records by `H("ed3k v1 page", secret ‖ root ‖ publisher key)`
  (a note by `H("ed3k v1 page", secret ‖ publisher key)`), sources by `H("ed3k v1 page", secret ‖ node ID)`,
  compared as bytes. `secret` is 32 random bytes that the node keeps to itself and replaces
  every 24 h *(initial)*. It MAY move its targets to a new secret one at a time: a target pages in
  its old order until its new one is complete, and all have moved before the next secret. A
  searcher MUST NOT expect two nodes to page alike. An answer holds at most 100 *(initial)*
  `files`, 100 `notes`, or 100 `sources` and `anon` together; `offset` counts records in that
  order and `more` says that records remain. Where an answer carries records in several
  fields, they are paged as one sequence: the fields in the order of their field numbers, each
  in its own order; `anon` is ordered as `sources` is, by `H("ed3k v1 page", secret ‖ node ID)`. `offset` counts in that
  sequence, and records are left out from its end only. `http` and `links` are not records of
  the sequence. They belong to the source record they were stored with, and an answer carries
  those of the `sources` it holds, in the order of `sources`, up to their limits per answer.
  Of a `Query`, a search of kind `KEYWORD` uses every field; a search of any other kind of
  version 1 uses `offset` only and ignores the other fields, unless the document of the kind
  says otherwise. The node
  keeps no state between pages; what changes in between can be missed or seen twice, and the
  searcher merges by publisher and file. An extension that pages its own records says in what
  order and how many an answer holds. How many pages a searcher asks of one node is local
  policy, unless the document of the kind says otherwise.
- **Every `ROUTER` stores every record kind of version 1**, those of extensions included,
  whether or not it sets the extension's capability: a kind that comes into use later then
  needs no network upgrade. The content and the storing-node checks of each kind are in the
  document that defines it, and a `ROUTER` implements those checks for every kind (§13.1).
  A `ROUTER` that answers a kind of version 1 as not responsible does not conform, although a
  lookup goes on past it; the rule for a kind the node does not know (below) is for kinds
  added after version 1. An index node without `ROUTER` stores the kinds it chooses (§11).
- **`HEAD` and `MEMBER`** are the record kinds of file sets. Every `ROUTER` MUST store and return
  them, whether or not it sets `SETS`. Their content and checks are in
  [`extensions/file-set.md`](extensions/file-set.md) §5 and §6.
- **`LIVE`** is the record kind for the peers of a live stream. It is stored and returned like
  `SOURCE`, with a shorter lifetime. Every `ROUTER` MUST store and return it, whether or not it
  sets `LIVE` ([`extensions/live.md`](extensions/live.md) §5).
- **`SUB`** is the record kind for the subscribers of a topic. It is stored and returned like
  `SOURCE`, with a shorter lifetime. Every `ROUTER` MUST store and return it, whether or not it
  sets `PUBSUB` ([`extensions/pubsub.md`](extensions/pubsub.md) §5). Rooms use the same kind
  for their members ([`extensions/room.md`](extensions/room.md) §5).
- **`HIDDEN`** is the record kind for the head of a hidden set: a head that the storing node
  verifies but cannot read. Every `ROUTER` MUST store and return it, whether or not it sets
  `SETS`. Its content and checks are in [`extensions/hidden-set.md`](extensions/hidden-set.md) §3.
- **`PART`** is the record kind for the parts that other authors add to a set, and a `HEAD` may
  be a joint head, signed by several keys under a grant of the owner. Every `ROUTER` MUST store,
  check and return both, whether or not it sets `SETS` ([`extensions/multi-author-set.md`](extensions/multi-author-set.md) §1.3 and §3.4).
- **`FRIEND`** is the record kind by which a node is found by its friends, and by which a node asks
  another to become its friend. It
  is stored and returned like `SOURCE`, but its content is a sealed byte string that the storing
  node cannot read. Every `ROUTER` MUST store and return it ([`extensions/friend.md`](extensions/friend.md) §4).
- **`ALIAS`** is the record kind that leads from another name of a file to its root: an eD2K
  hash, the infohash of a torrent, a link at a file hoster, the articles of a Usenet post, a
  hash from a magnet link. The kind
  of name is a string in the record, so that a network joined later needs no new record kind.
  Every `ROUTER` MUST store and return it, for names of every kind
  ([`extensions/alias.md`](extensions/alias.md) §3).
- **`NAME`** and **`CLAIM`** are the record kinds of readable names: `NAME` gives a name
  under the root key of a zone, one record per name, which only that key or a key it delegated
  to can sign; `CLAIM` says that its publisher calls a zone by a global name, and is held like
  `ALIAS`. Every `ROUTER` MUST store and return both, whether or not it sets `NAMES`
  ([`extensions/name.md`](extensions/name.md) §3.2, §4.2).
- **`ANON`** and **`GATE`** are the record kinds of a node that hides its address behind other
  nodes: `ANON` names it as a source of a file, by its node ID alone, and `GATE` says through
  which nodes it is reached. Both are signed. A search of kind `SOURCE` also returns the node IDs
  of the `ANON` records of the target, in `anon`, paged after `sources` (**Paging** above). A `Store` of a signed kind MAY carry `signer` (§10.7); the record is
  then checked under that key in place of the key of the connection. For `ANON` and `GATE`
  `signer` is required ([`extensions/hops.md`](extensions/hops.md) §4.3).
  Every `ROUTER` MUST do all of this, whether or not it sets `ANON`
  ([`extensions/hops.md`](extensions/hops.md) §4).
- A node that does not know the `kind` of a request answers as a node that is not responsible:
  a kind added after version 1, or a kind an index node without `ROUTER` does not store (§11).
- A node that is not responsible ignores `search` and `store` and only returns contacts. It
  answers a `store` with contacts and `responsible = false`, never with stored data. A
  requester MUST ignore `sources`, `files`, `notes`, `results` and every record field of an
  extension in an answer whose `responsible` is false.
- **The requester follows closer nodes.** Whenever an answer contains contacts closer than the
  answering node, the requester MUST continue to them, whether or not the answer said
  `responsible`. Only the closest verified nodes hold the payload; an answer from a node further
  out is a waypoint.
- **Source records are bound to the connection.** The storing node takes the source's node ID
  from the authenticated connection and the endpoint of the connection's family as observed,
  address and port, replacing what `source` says for that family; the port of `Hello` (§5.5)
  does not enter a source record. A `source` whose `id` is not empty is `REJECTED`. An
  endpoint of the other family is stored as claimed (§8.2 (5)); its address MUST be acceptable
  (§8.2 (7), and (8) in a named network), else the record is `REJECTED`. `relays4` and
  `relays6` hold at most 2 *(initial)* compact nodes each, at acceptable addresses, else the
  record is `REJECTED`. A node can therefore publish only
  its own node ID, and source lists cannot be filled with invented peers; the claimed endpoint
  and the relays are addresses that a searcher dials within the limits of
  [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.5. A dial through a relay whose
  `Punch` is refused, with `Error{DENIED}`, with `Error{UNSUPPORTED}` from a node that is no
  relay, or with `Error{NOT_FOUND}` for want of a reservation (§9.5), counts as a failed dial
  at the relay's address ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.5). A connection that has no observed address, such as a session through
  a path ([`extensions/hops.md`](extensions/hops.md) §3), cannot store a source record: the
  answer is `REJECTED`, and a node that hides publishes `ANON` records instead.
- **The searcher checks `sources`** of an answer by the same rules: it drops a `Peer` whose `id`
  is not 16 bytes, whose endpoints or relays are not at acceptable addresses, that has more
  than 2 relays in a family, or that has neither an endpoint nor a relay.
- **Partial files are sources.** A node MAY store a `SOURCE` record for a file it holds in
  part; its `FileStatus.have` tells a downloader what it has (§12.1). `FileInfo.sources`
  counts complete sources and `FileInfo.partial` the others.
- **Source records are not signed.** A searcher has only the storing node's word for a source
  until it dials it. Dialling is the verification: the handshake proves that the node at that
  address holds the key behind the node ID in the record (T1), and a source that fails it is
  dropped. A signature would add nothing to that, because it cannot prove that an address
  belongs to a key. What a false record can still cause is a wasted dial, which is bounded by
  the dial limits of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.5.
- **Keyword and note records are signed** by their publisher (§10.7). Their content is still a
  claim; the signature fixes who made it.
- Lifetimes *(initial)*: `SOURCE`, `ANON` and `FRIEND` 5 h, `GATE` 1 h, `KEYWORD` 24 h, `NOTE` 24 h, `MEMBER`, `ALIAS` and `CLAIM` 24 h, `LIVE` 10 min, `SUB` 60 min, `HEAD`, `HIDDEN`, `PART`
  and `NAME` up to 7 days. Republish before expiry; how long before is local policy where the kind names no
  interval.
- A `Store` holds at most 256 *(initial)* records, all its record fields together; every
  record after the 256th is `REJECTED`. Such a record still counts as held: a `Store` whose
  only record of `kind` comes after the 256th is answered per record, not `Error{MALFORMED}`.
  How many requests per minute a node accepts per
  source address and `kind` is local policy; a store beyond it is answered `BUSY` for every
  record, a search `Error{BUSY}` with `retry_after`.
- **Keyword search.** The searcher splits its search text into words as in §10.1 (steps 1 to 3).
  It derives the target from the longest of them in UTF-8 bytes that has 3 bytes or more, the
  first when several are equally long, and from a 2-byte word only when there is no longer
  one. It sends the others in `all`, and `all`, `none` and `ext` normalised as step 1 of §10.1;
  a storing node compares them as bytes with the words and the extension of the name. `ext`
  is sent and compared without the `.`. A word in `all` or `none` matches a name when it
  equals one of the name's words; there is no substring match. A term is therefore one word
  as steps 1 to 3 give it: a term with a separator inside, or an unspaced run of 3 or more,
  never matches, and a searcher splits its terms with steps 1 to 3 before it sends them. A
  search text without a keyword cannot be searched. The searcher does not know the extension
  of the names it looks for, and extension words are not keywords (§10.1): a search text
  whose longest word is one, as `mkv` in `ab.mkv`, finds nothing under that target.
- **Source records for many targets.** A `store` of kind `SOURCE` MAY name further targets in
  `also`. The node treats each as a store of the same record under that target: it checks its
  responsibility and its limits per target, and answers in `results` with one entry for the
  target of the request followed by one per target of `also`. A target it is not responsible
  for is answered `FAR`, and every target of `also` after its 256th *(initial)* is `REJECTED`. An `also`
  that is not n × 16 bytes makes a request of kind `SOURCE` `Error{MALFORMED}`. With another
  `kind`, `also` is ignored and gets no entries. `nodes4` and
  `nodes6` refer to the target of the request. Fields that belong to one file, such as HTTP
  sources, apply to the target of the request only. How a publisher uses this is in §10.4.
- A `store` is answered per record in `results`: one entry per record, in the order of the
  fields of `Store` by field number and, within a repeated field, in its order, followed by one
  per target of `also`. The record fields are `source` (1), `files` (2), `note` (3), `head` (5),
  `members` (7), `sealed` (9), `anon` (10), `aliases` (11) and `names` (13). `http` (4) and `links` (12)
  belong to the `source` record and get no entry, and neither do `signer` and `also`
  themselves. A list shorter than that reads as `STORED` for the entries it lacks, as
  an absent list does, and entries beyond it are ignored. A `sites` shorter than the list it
  counts reads as 0 for the entries it lacks. A record that fails a check (§10.7) or that does not belong to `kind`
  is `REJECTED`; a `Store` that holds no record for `kind` is answered `Error{MALFORMED}`; a record that is not newer than the one
  held (§10.7) is `OLDER`. What a keyword target holds, and what a publisher does when it is
  full, is in §10.8.

### 10.4 Lookup

1. Seed the candidate set with the closest contacts from the routing table.
2. Query the α = 3 *(initial)* closest candidates not yet queried, carrying the operation in every
   request.
3. Add returned contacts to the candidates. A contact becomes verified when its connection
   completes (§6.3).
4. Stop when both hold:
   - the R closest verified candidates have all answered and no candidate that may still be
     queried is closer to the target than the furthest of them. R = 4 for a search, 8 for a
     store *(initial)*. These R nodes are the **closest set**, whether or not they said
     `responsible`; a store is judged over those of them that did, ignoring those that
     answered `BUSY` (§10.8). Every queried node that
     is responsible performs the operation, so a store is held by more nodes than the
     closest set; it is judged over the closest set all the same;
   - at least M nodes have answered in this lookup. M = 5 for a search, 9 for a store
     *(initial)*. A lookup without an operation uses the M of a search.

   R and M count distinct node IDs: the two entries of a dual-stack node are one node.

**Minimum walk.** While the first condition holds and the second does not, the requester
queries the closest candidate not yet queried, one at a time, although it is further from the
target than the closest set. What that node returns becomes a candidate like any other
contact, and a contact closer than the furthest of the closest set is followed as in step 2.
A node that failed does not count towards M. A lookup that has no candidate left to query
ends with fewer than M answers.

- Without M a lookup ends as soon as R nodes say that they know nothing closer. Nodes that
  work together can say so about each other, and the nodes that would name the honest ones
  near the target are then never asked.
- The filters of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.2 apply to
  these queries too. A candidate whose groups are full in the closest set is skipped, so the
  extra query goes to a node of another host, site or block than the nodes that ended the
  walk.
- M does not change the closest set. A node asked for this reason joins it only if it is
  among the R closest.

A queried node that has not answered within 5 s *(initial)*, counted from the first packet sent
to it, has failed for this lookup and counts one failed attempt (§10.2). A queried node that
answers with an `Error` has failed for this lookup too: it does not count towards M, and it
counts no failed attempt, since it did answer, and does not reset the count (§10.2). A
queried node whose `KadAnswer` is malformed, one that does not parse or breaks a size or
layout rule (§0, §10.3), has failed for this lookup and counts one failed attempt. A lookup
ends after 60 s *(initial)* at the latest, with the closest set it has by then.

Which returned contacts become candidates, which candidates are queried and which R nodes form
the final set are further restricted by [`extensions/ip-diversity.md`](extensions/ip-diversity.md)
§2.2 and §2.3. A lookup that cannot reach R nodes within those limits ends with fewer.

**Storing many source records.** A node that shares many files does not look up every root.

1. It sorts its targets.
2. It looks up the target whose deadline (below) comes first, with the store in the request
   as always. The lookup ends with a closest set.
3. To each node of that set it sends one more request for the same target, with the following
   targets in `also`: those whose distance to the target is smaller than the distance from the
   target to the furthest node of the set, at most 256 *(initial)* of them in sorted order. The
   next batch starts after the last of them.
4. A target that fewer than 4 *(initial)* nodes answered `STORED` gets a lookup of its own,
   unless the others answered `REJECTED` or `OLDER`, which any other node would answer too.
   `BUSY` is transient: it is no reason for a lookup of its own, and the target keeps its
   deadline (below). Otherwise the node goes on with the next target.

- **Deadline.** Each target is stored again at most 4 h *(initial)* after its last successful
  store, so that every record is renewed before its 5 h end. A store of a target has
  succeeded when 4 nodes answered `STORED`, or the others `REJECTED` or `OLDER`, or when its
  lookup of its own has ended; a target that only `BUSY` held back has not. The node
  spreads its lookups evenly over the 4 h, so that there is no burst. Batching is an
  optimisation: a target that a batch stored has been stored, and no deadline is passed to
  wait for a batch.
- A file that is new is stored at once, out of turn; its deadline runs from that store.
- The number of lookups per 4 h is then bounded by the size of the network, not by the number
  of files. The estimate and the comparison with IPFS, where the method comes from, are in
  [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §2.1.

**Searching sources for many files.** There is no batch for searches; every target is looked
up. A client that has many targets to search SHOULD take them in sorted order, and SHOULD add
the closest set of the lookup just ended to the candidates of the next one (step 1). Where two
targets share their closest nodes, the second lookup then asks those nodes on connections
that are still open, and needs one node more to reach M. Each target is still a request of its own and counts against the limits
of the answering node on its own. Why a batch was left out is in
[`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §4.8. For the members of
a set a client asks the sources of the manifest first
([`extensions/file-set.md`](extensions/file-set.md) §3).

### 10.5 What a lookup costs over QUIC, and what bounds it

A new QUIC connection costs about 2.5 KB (both Initial flights are padded to 1200 bytes) and one
round trip; an eMule Kad exchange is a few hundred bytes. The iterative walk cannot be skipped:
the payload lives only on the closest nodes, and those are found by asking nodes that are not yet
close. The number of connections per lookup is therefore the number of distinct nodes on the walk
plus the responsible set, and no protocol trick removes it. What the design does is keep every
other cost off that number:

| Measure | Effect |
|---|---|
| Operation rides in the lookup request | The responsible nodes are contacted **once**. eMule Kad walks with `KADEMLIA2_REQ` and then sends the search or publish to the same nodes again; over QUIC the second pass is free only while the connection is still open, and here it does not exist |
| 16 contacts per answer instead of 11 | Fewer hops. A contact costs 22 bytes, a hop costs a handshake |
| Few queries beyond the closest | A candidate is queried only if it is among the α closest known, or one at a time while fewer than M nodes have answered (§10.4). Every further query costs 2.5 KB, so α stays small and is not raised to hide timeouts, and M is one more than R |
| First hop is warm | Connections to routing-table contacts stay open while idle (30 s) and resume with 0-RTT afterwards, which saves the round trip though not the padded Initial |
| One connection per node and family, many exchanges | Lookups for different targets that pass through the same node share its connection; a client publishing or searching many files orders them by target so neighbouring walks overlap (§10.4) |
| Keyword records batched per node | All files sharing a keyword go in one `Store` |
| Source records batched per region | Targets that share their closest nodes go in one `Store` per node (§10.4) |
| Index nodes | A node logged in to an index node (§11) publishes and searches there over one connection and needs Kad for keywords only as a fallback |
| Long record lifetimes | Publishing is the dominant Kad load; a source is stored again every 4 h and a keyword record before its 24 h end, not per search |

Expected cost of a lookup with operation: 3–4 hops and 8–12 nodes contacted (at least M = 9
for a store), of which 5–9 are new
connections, so roughly 15–25 KB against 3–5 KB in eMule Kad. That is the price of having one
authentication and anti-spoofing mechanism instead of two, and of the verified addresses that §6.3
depends on. The minimum walk adds nothing to that figure: such a lookup has more than M
answers before the closest set is complete. M costs a query only in a lookup that would
otherwise end on its first R answers, which is the case it exists for, and on a network of
only a few nodes.

### 10.6 Reserved: sessionless Kad datagrams

If measurements on the live network show the cost above to be a problem, Kad requests can move to
single sealed datagrams, as eMule Kad and Ethereum's discv5 do. Version 1 reserves for this:

- capability bit 5 (`KAD_DGRAM`)
- message types 48–63
- a second `magic` value for the datagram format of §4.3
- field 8 of `Hello`

Such a binding has to provide on its own what QUIC provides here: proof of the sender's key,
proof of its address and replay safety. It carries the same `KadRequest` and `KadAnswer`,
except that a request with `store` is not accepted by datagram and goes over QUIC.
The design is specified as an extension in
[`extensions/kad-datagram.md`](./extensions/kad-datagram.md); it is not part of version 1.

### 10.7 Signed records

```protobuf
message Record {
  uint64   expires = 1;       // Unix time in seconds
  FileInfo file = 2;          // KEYWORD
  Note     note = 3;          // NOTE
  // fields 4 and 5 belong to the optional file set extension (§13.1)
  // field 6 belongs to the optional publish and subscribe extension (§13.1)
  // field 7 belongs to the optional hidden set extension (§13.1)
  // fields 8 and 9 belong to the optional extension for sets with several authors (§13.1)
  // field 10 belongs to the optional extension for anonymous paths (§13.1)
  // field 11 belongs to the optional extension for other names of a file (§13.1)
  // fields 12 and 13 belong to the optional extension for readable names (§13.1)
}

message Signature {
  uint32 alg = 1;             // 1 = Ed25519; 0 (absent) is invalid
  bytes  sig = 2;
  // field 3 is reserved for a batch form: one signature over many records
}

message Signed {
  bytes  body = 1;            // a serialised Record, verified over exactly these bytes
  uint32 key = 2;             // index into KadAnswer.keys; absent in a Store, and ignored there
  repeated Signature sigs = 3;
}
```

Each signature is computed over

```
"ed3k v1 record" ‖ kind(1) ‖ target(16) ‖ body
```

`kind` and `target` are those of the request, so they are signed without being sent, and a
record cannot be replayed under another keyword or file. One kind signs a value in the place of
`target` that does not change with the day: `HIDDEN`
([`extensions/hidden-set.md`](extensions/hidden-set.md) §3).

**Signatures**

- The publisher signs with its identity key (§6.1). In a `Store` the key is the one of the
  connection, or `signer` when the `Store` carries it: a `Store` of any signed kind MAY carry
  `signer`, and its records are then checked and counted under that key. This is how a
  follower stores another's head again ([`extensions/file-set.md`](extensions/file-set.md)
  §5.2) and how a last hop stores for a node that hides
  ([`extensions/hops.md`](extensions/hops.md) §4.2). Source records, which are bound to the
  connection, never use it: `signer` in a `Store` of `SOURCE` or of a kind stored like it
  (`LIVE`, `SUB`, §10.3), or of `FRIEND`, is ignored. In a `KadAnswer` the key is `keys[key]`, sent once per publisher.
- `sigs` is a list from version 1 on. It holds at most one signature per algorithm; a second
  one of the same `alg`, or one with `alg` = 0, makes the record invalid.
- **Ed25519** is verified as RFC 8032 §5.1.7 says, with every choice fixed, so that all nodes
  agree on what is valid: a signature is 64 bytes; `S` MUST be below the group order; the
  encodings of the key `A` and of `R` MUST be canonical; `A` and `R` MUST NOT be of small
  order; and the equation is checked without the cofactor, `[S]B = R + [k]A`. This is
  `verify_strict` of ed25519-dalek with a check added that the encoding of `A` is canonical.
  Libraries that verify more loosely need these checks in front of them: Go's
  `crypto/ed25519.Verify`, for one, accepts `R` and `A` of small order and an `A` that is not
  canonical. The forms that are rejected are test vectors
  ([`tests/vectors-v1.md`](tests/vectors-v1.md)).
- A record is **valid** when every signature of an algorithm the verifier knows verifies, and
  every *required* algorithm is present. In version 1 the required set is Ed25519 alone. The
  set changes by network upgrade (§16).
- Signatures of unknown algorithms are ignored by the verifier and kept by a storing node.
- A `Signed` MUST NOT exceed 1 KiB *(initial)*, counted over `body` and `sigs` as the
  canonical serialisation of §0 writes them after decoding, without `key`. Unknown fields and
  long varints therefore do not count, and a node that encodes a record again does not change
  its measure. This bounds what unknown signatures can add.
- How a key of another algorithm is tied to a node identity is **not defined** in version 1.
  The specification that registers an algorithm has to define it. Signatures of the sizes that
  post-quantum algorithms have today (2 KB and more) will need the reserved batch form.

**Checks**

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| Form | `body` parses as a `Record`; else the record is `REJECTED`. `key` is ignored | the same, and `key` is an index into `keys`; else the record is dropped |
| Signature | valid under the connection's key, or under `signer` | valid under `keys[key]` |
| `expires` | later than now, and at most the lifetime of the kind plus 1 h *(initial)* ahead | later than now minus 1 h *(initial)* |
| Content | `file` for `KEYWORD`, `note` for `NOTE`, and no other of the fields 1–3 of `Record`; the fields of extensions (4 and up, §13.1) and fields it does not know are allowed and ignored for these kinds | the same |
| Size | `file.name` is 1 to 255 bytes of UTF-8; `file.root` is 32 bytes; `file.ed2k` is empty or 16 bytes (§0); `note.rating` is 0 to 5 and `note.text` at most 512 *(initial)* bytes of UTF-8; the `Signed` is at most 1 KiB *(initial)* as measured above | the same |
| Keyword | one keyword of `file.name` (§10.1) gives the target of the request at its `level` | one keyword of `file.name` gives the target that was searched |
| Count | one record per publisher and file under a keyword; one note per publisher and target (a note carries no root: it is bound to `root[0..16)`, the target) | duplicates by publisher and file, or publisher and target for a note, are merged |

A record that fails a check on the storing node is answered `REJECTED` in `results` (§10.3).
A `Store.signer` that is not 32 bytes makes a request of a kind that uses it `Error{MALFORMED}`
(§0); for `SOURCE`, `LIVE`, `SUB` and `FRIEND` it is ignored whatever its length.
**Newer.** Every signed kind keeps one record per publisher and file, target or root (of all
publishers together a keyword file, a `MEMBER` set, an `ALIAS` root and a `CLAIM` zone keep at most 3, §10.8
and their documents), except `HEAD`, `HIDDEN`, `GATE` and `NAME`, which keep one record per target
whichever key signed it, and a
record replaces the one held only when it is newer: `HEAD`, `HIDDEN`, `PART`, `GATE` and `NAME` have
an order of their own, stated in their documents; for every other signed kind the record with
the later `expires` is the newer one. A valid record that is not newer than the one held is
answered `OLDER`, and nothing changes. This holds also for a record that a third party stores
again with `signer`: an older copy never replaces a newer one. `Error{MALFORMED}` is for a
request that does not parse or breaks a rule of §10.3 (§5.6); a `Signed.body` that does not parse fails its record only. A searcher drops a record that fails. There are no unsigned
keyword or note records. A storing node deletes a record at `expires`.

The publisher's node ID, `H("ed3k v1 node id", key)[0..16)`, is the author of the record. It is
what a client counts once, ignores, or trusts. Why this helps against spam, and where it does
not, is discussed in [`docs/signed-records.md`](docs/signed-records.md).

### 10.8 Keyword load

Some words occur in a large share of all file names, and every record of one word goes to the
same few nodes. Version 1 bounds that as follows.

**Storing node**

- A keyword target holds at most 100,000 *(initial)* files: the **level size**. Records of
  several publishers for one file count once.
- Per file the node keeps at most 3 *(initial)* records, the freshest of different
  publishers. A record is as fresh as the time the node last stored it, a renewal included;
  `expires`, which the publisher chooses, does not count. The least fresh is the one stored
  longest ago, and of two stored in the same second the one whose publisher key is lower as
  bytes.
- The site of a kept record is the site
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1) of the address of the
  `Store` that last stored it, a renewal included. For a `Store` with `signer` that is the
  address of the node that sent it, not of the publisher.
- A valid record that is not `OLDER` (§10.7) takes its place by these rules, in this order,
  and is answered `STORED`:
  1. A record of a publisher already kept takes the place of that publisher's record,
     whatever site it comes from. The kept record then has the site of this `Store`.
  2. Otherwise a record from a site that holds one of the places takes the place of that
     site's record (of the least fresh one, should a renewal have left two of one site).
  3. A record from another site is added while fewer than 3 are kept. Else it takes the
     place of the least fresh record kept.
- A `Store` with `signer` never takes the place of another publisher's record of another
  site: where rule 3 would give it the place of such a record, it is answered `REJECTED`
  (not `FULL`: the target is not full).
- So each site holds one place, and one site cannot keep two by renewing often.
- A file target keeps at most 64 *(initial)* notes; a note of a further publisher is `FULL`,
  and a held one is renewed. At most 4
  *(initial)* of them come from one publisher host address and 8 *(initial)* from one
  publisher site ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1); a further
  one from there is `REJECTED`.
- A file target holds at most 10,000 *(initial)* `SOURCE` records; a record of a further node
  is `FULL`, and a held one is renewed. A target of `LIVE` or `SUB` holds as many records of
  its kind, by the same rule.
- Of the files under one keyword target, at most 1,000 *(initial)* come from one publisher
  key and 2,000 *(initial)* from one publisher site
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1). A record for a further
  file of that publisher key or site is `REJECTED`. A file counts for a key and for a site
  while a kept record of it has that publisher and that site. A record that keeps its file
  under the same publisher and site, or under the same site by rule 2, adds no file to that
  site; rule 2 adds one to the new publisher key, and rule 1 from another site adds one to that
  site. A renewal by rule 1 from a site that is at its limit, and holds no record of that
  file, is therefore `REJECTED`, and the kept record stays as it was, with its site, until it
  expires.
- `FULL` is for a target that has no room. A record that is over a limit per address or per
  publisher is `REJECTED`: that says nothing about the target, does not set `spill`, and is
  no reason for a lookup of its own for the target (§10.4).
- While a target is full, a record for a file the node already holds is renewed (`STORED`), and
  a record for a new file is refused (`FULL`).
- `BUSY` means the node as a whole is out of room, or the sender is over its request rate
  (§10.3). It says nothing about the target.
- `load` and `spill` MUST be set, as §10.3 defines them, in every answer of a responsible node
  to a request of kind `KEYWORD`, a store answer and a contacts-only answer included. For other kinds `load` is 0, and `spill` is set when the node
  answered `FULL` for that target within the last record lifetime of the kind. `sites` MAY be
  left at 0 for `KEYWORD` in version 1; for `MEMBER`, `ALIAS` and `CLAIM` it carries the count the
  answer is ordered by ([`extensions/file-set.md`](extensions/file-set.md) §6,
  [`extensions/alias.md`](extensions/alias.md) §3, [`extensions/name.md`](extensions/name.md) §4.2). That count covers the sites of the stores
  answered `STORED` for that root or set, each remembered until that record would have
  expired, also after it was displaced. A `Store` with `signer` adds no site to it. One site
  counts once, and the count saturates at 255 *(initial)*, as in
  [`extensions/keyword-load.md`](extensions/keyword-load.md) §2.1.

**Publisher**

- A file is published under the keywords that §10.1 selects: by default those of 3 bytes and
  more, at most 40.
- A refused record is offered again at the next refresh, not earlier. There is no longer
  back-off.
- The outcome of a store is judged over the members of the closest set (§10.4) that said
  `responsible`, ignoring those that answered `BUSY`. A record is stored when one of them
  answered `STORED`, and refused otherwise.

**Searcher**

- `load` and `spill` tell the searcher that the results are a part of what was published under
  that word.

**Levels.** The keyword target includes a level (§10.1), and on Kad version 1 uses level 0
only: a node MUST send `level = 0` to a Kad router and MUST NOT publish or search there at
another level. At an index node levels are part of version 1 (§11). A storing node treats a
target of level 1 to 15 like any other keyword target, so that upgraded publishers can use those
levels on nodes that have not upgraded; it answers a record with `level` above 15 with
`REJECTED`. A later network upgrade
lets a full keyword spill to further levels and ranks files by the number of publisher sites, so
that the most widely shared files are at level 0. Every field it needs is defined above;
capability bit 8 is reserved for it. The rules are in
[`extensions/keyword-load.md`](extensions/keyword-load.md), together with the alternatives that
were considered.

---

## 11. Index nodes

An index node is a node with the `INDEX` capability. It is what eD2K calls a server, without a
protocol of its own:

| eD2K server function | ed3k |
|---|---|
| Login, ID assignment, "your IP" | `Hello` (`observed`, §9.2). There is no HighID/LowID: a node is `open` or it has relays |
| Offer files | `KadRequest{kind=KEYWORD, store}` and `{kind=SOURCE, store}` |
| Search | `KadRequest{kind=KEYWORD, search}` |
| Get sources | `KadRequest{kind=SOURCE, search}` |
| Callback request | `Punch` (an index node that offers the relay service is a relay, §9.4; **Kad duty** below) |
| Server list | none on the wire; a client keeps its own list of index nodes, found as said below and recognised by `Hello.caps` |

Differences from a Kad node: an index node is `responsible` for **every** target, also before it
has adopted an endpoint (§9.2, §10.3), never needs the iterative walk, MAY keep a `SOURCE`
record beyond its lifetime for as long as the publishing connection stays open, and MAY apply
its own admission policy with `Error{DENIED}` or `{BUSY}`. A `SOURCE` record kept for the
connection goes when the connection closes or at the end of its lifetime, whichever is later;
once past its lifetime it gives its place to the record of a further node before the target
answers `FULL` (§10.8). The kinds stored like `SOURCE` (`LIVE`, `SUB`, `FRIEND`) are not kept
so: they end with their lifetime as anywhere else, since their short lifetime is what tells
that a node has left a stream or a topic, or no longer wants to be found. Signed records
(§10.7) expire on an index node as anywhere else; the publisher renews them. The limits of
§10.8, of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.4 and the limits per
target in the document of each kind bind an index node as any storing node, unless its own
policy is stricter. The one exception is the level size (§10.8): an index node's operator
chooses it. Until a keyword target holds that many files the node answers no `FULL` for it;
from then on it answers `FULL` and sets `spill` as §10.8 says.

**Keyword levels.** Levels are part of version 1 at an index node, with no capability bit. An
index node MAY rank and keep leaving entries as
[`extensions/keyword-load.md`](extensions/keyword-load.md) §2 says; one that does not refuses
when full, as §10.8 says. A client that publishes or searches at an index node follows that
document's §3 and §4 by that node's answers: it moves down on `FULL` and follows `spill`. Each
level is a target of its own there. The level it remembers per (file, keyword) is that of the
index node it uses; answers from Kad never move it, and on moving to another index node it
starts again at level 0. On Kad, levels wait for the network upgrade (§10.8).

**Limits over all targets.** Those limits are per target, and an index node holds every target;
keys, names and roots cost nothing, so a limit per target does not bound what one publisher
can make it hold. An index node MUST therefore also bound the records it holds per publisher
host address and per publisher site ([`extensions/ip-diversity.md`](extensions/ip-diversity.md)
§1), over all targets and kinds together. The bound is local policy. A record beyond it is
`REJECTED`, not `BUSY`: the node has room.

**Kad duty.** An index node is normally open, often by the operator's word (§9.2), and MAY leave
`ROUTER` unset (§2). It is then no Kad router:

- It is not put in routing tables (§10.2), keeps none of its own and answers without
  contacts. It need not dial at all: on a connection it accepted every datagram is masked
  with its own `obf_id` (§4.3), and as A of a `ReachTest` it answers `ok = false` (§9.3).
- It stores the record kinds it chooses and answers a request of any other kind as a node
  that is not responsible (§10.3). A client therefore falls back to Kad for those kinds.
- It SHOULD still offer the relay service (§9.4), which is the callback of an eD2K server.
  An index node that offers it sets `RELAY` while it is open and has a free slot.

An index node that sets `ROUTER` has every duty of one, also storing every record kind.

**Keepalive.** An index node holds many idle connections, and keeps none for its own sake. It
sends `max_idle_timeout` = 300 s *(initial)* and no PING of its own. A client that dials a node
it knows as an index node (below) sends 300 s as well, since QUIC takes the smaller value of
the two sides. A client that wants such a connection kept sends a PING every 60 s *(initial)*
while it holds a relay reservation there or is not `open` in the family of the connection, so
that its NAT keeps the mapping, and every 240 s *(initial)* otherwise, each interval varied at
random by up to 10 % *(initial)*. A mapping that lapses between two PINGs is mended by the
next one, through the migration of the client side (§4.1); a `Punch` lost in the gap is tried
again (§9.5).

**Load** *(informative)*. Signed records cost an index node one strict Ed25519 verification each
(§10.7). A client that shares 10,000 files under about 5 keywords each sends some 50,000 a
day, a few seconds of one core; 50,000 such clients take about 1.5 cores. Batch verification
does not help: it checks the equation with the cofactor, which §10.7 does not. A node MAY
answer a `Signed` that is byte for byte the one it holds `OLDER` without verifying it again.

A client finds index nodes from a configured list, from a link (§13.2) or from an eD2K server
that advertises one ([`docs/ed2k-tags.md`](docs/ed2k-tags.md)). Index nodes are optional for the
network and optional for a client. An eD2K server that runs an index node MAY list some of its
ed3k sources on eD2K as well ([`docs/ed2k-tags.md`](docs/ed2k-tags.md) §6).

**One index node at a time.** A client SHOULD publish and search at one index node of its list
at a time, over one connection, and move to another of the list when that one cannot be
reached. Records sent to every index node of a list multiply the load of each (**Load**). A
client MAY also hold a relay reservation (§9.4) at a second index node of its list, and opens
no other exchange there.

- **Redial.** A client does not dial an index node again sooner than 30 s *(initial)* after its
  connection there ended or a dial failed, and doubles the wait after each try that fails, up
  to 10 min *(initial)*. A connection that stayed open 60 s *(initial)* resets the wait to
  30 s; one that ended sooner, also after a completed handshake, is a try that failed. A longer
  `retry_after` (§5.6) wins. That keeps it well within the budget of §4.6.
- **Moving.** A client that moves to another index node stores its records there at once. The
  old one keeps them until their lifetimes end (above). After a redial of the same index node
  there is nothing to store again: its records outlive the connection and are renewed before
  their lifetimes end, as anywhere else (§10.4), and a relay lost with the connection is
  replaced as §9.4 says.
- **What a searcher sees.** An index node answers with what was published there, as an eD2K
  server did without global search; Kad has what was published in Kad. A searcher MAY also ask
  up to 3 *(initial)* other index nodes of its list per search, over connections it does not
  hold: it sends no PING on them (**Keepalive**).

An index node stores and answers records. A service that keeps a catalogue of its own, such as
a video site, is not an index node by that; it runs a node next to it, which may be one
([`extensions/chaintube.md`](extensions/chaintube.md) §5).

---

## 12. File transfer

### 12.1 The file stream

A downloader opens one exchange per uploader for the file it wants from it. Everything about
that file between the two peers happens on it. The file is named once, in the first message.
The stream stays open during a transfer; while the downloader waits in the queue, the uploader
decides whether it stays open (§12.5).

```protobuf
message FileRequest {
  bytes  root = 1;            // 32 B; required in the first FileRequest of the stream, ignored in later ones
  optional bytes have = 2;    // my piece bitmap, LSB first; missing bits are 0, so an empty bitmap holds no piece (the opposite of FileStatus.have); absent = unchanged (first message: not told)
  uint32 want_sources = 3;    // max number of sources wanted, 0 = none
  optional bytes relays4 = 4; // n × 22 B compact nodes: my relays, if I am not open (§9.4, §12.5); empty = none; absent = unchanged
  optional bytes relays6 = 5; // n × 34 B; likewise
  bool   brief = 6;           // I know name, ed2k and note; leave them out
  // field 7 belongs to the optional HTTP source extension (§13.1)
  // field 8 belongs to the optional file set extension (§13.1)
  // field 9 belongs to the optional credit extension (§13.1)
}

message FileStatus {
  uint64 size = 1;
  string name = 2;
  bytes  ed2k = 3;            // §7.4
  optional bytes have = 4;    // uploader's piece bitmap, LSB first; missing bits are 1, so an empty bitmap is the complete file; absent = unchanged
  optional uint32 rank = 5;   // queue position; 0 = upload slot granted, send BlockRequest; absent = unchanged
  repeated Peer sources = 6;  // source exchange; at most `want_sources` of the request, and at most 50
  Note   note = 7;            // uploader's rating/comment on the file
  optional uint32 recheck = 8; // queued: 0 = stay connected, else ask again after this many seconds (§12.5); ignored while rank = 0; absent = unchanged
  // field 9 belongs to the optional HTTP source extension (§13.1)
  // field 10 belongs to the optional file set extension (§13.1)
  // field 11 belongs to the optional credit extension (§13.1)
  // field 12 belongs to the optional extension for links at file hosters (§13.1)
  bool   ping = 13;           // answer this message with a FileRequest within 30 s (§12.5)
}

message SlotOffer {
  bytes  root = 1;            // the file the downloader is queued for
}
```

- The uploader answers the first `FileRequest` with a full `FileStatus`, or `Error{NOT_FOUND}`.
  Full means that `size`, `have`, `rank` and `recheck` are present, and `name`, `ed2k` and
  `note` as far as the uploader knows them and `brief` did not leave them out.
- `root` is 32 bytes (§0) and MUST be present in the first `FileRequest` of a stream; a first
  `FileRequest` without it is answered `Error{MALFORMED}`. A `root` in a later `FileRequest`
  is ignored. Bits of `have` beyond the last piece of the file are ignored, in both messages.
  Of `relays4` and
  `relays6` the uploader uses the first 2 *(initial)* compact nodes of each, as a source
  record holds (§10.3), and ignores the rest.
- **The size is a claim.** A downloader that has another size for the file than the
  uploader's `FileStatus.size` does not use that source together with the hash values it
  holds: two sizes give two trees (§12.3).
- Later `FileStatus` messages carry only changed fields: `have`, `rank` and `recheck` are
  `optional` and absent means unchanged (§0); `size`, `name` and `ed2k` are sent once. When
  they are sent is limited by §12.5. The same holds for `have` in a later `FileRequest`.
- The downloader sends a new `FileRequest` to update `have` or to ask for sources again. The
  uploader answers a later `FileRequest` only when its `want_sources` is above 0, with a
  `FileStatus` that carries `sources` alone, and at most once per 10 min *(initial)*, or when
  it asks for the member bitmap of a set ([`extensions/file-set.md`](extensions/file-set.md)
  §4.1). Any other later `FileRequest` is not answered.
- Either side ends by closing the exchange. There is no cancel message.
- An `Error` on a file stream ends it: its sender closes the stream. The exceptions are
  `NOT_FOUND` and `BUSY` in answer to a `HashRequest` (§12.4).
- Sources in `FileStatus` MUST be peers the uploader has itself had a verified connection with
  for this file within the last hour *(initial)*.

These two messages replace `OP_HELLO`'s file fields, `OP_REQUESTFILENAME`, `OP_SETREQFILEID`,
`OP_FILESTATUS`, the three multipacket variants and their two answers, `OP_STARTUPLOADREQ`,
`OP_ACCEPTUPLOADREQ` (with `SlotOffer`), `OP_QUEUERANKING`, `OP_REQUESTSOURCES2`/`OP_ANSWERSOURCES2`,
`OP_REASKFILEPING`/`OP_REASKACK`, `OP_FILEDESC`, `OP_CANCELTRANSFER` and `OP_END_OF_DOWNLOAD`.
There is no large-file variant; every offset is 64 bits.

Queueing, scoring and credits are local policy as in eMule. Credits are keyed by node ID. How
long a slot lasts and how many there are is in §12.5. Credits that count at every uploader,
and bids for a slot, are an optional extension (§13.1).

### 12.2 `BlockRequest`

```protobuf
message BlockRequest {
  repeated uint64 offset = 1; // packed; each a multiple of 256 KiB
  repeated uint32 length = 2; // packed, same count; the length of the block at that offset
  bool   raw = 3;             // send these ranges as they are, never compressed (§12.3)
}
```

Allowed only while `rank = 0`. Up to 16 *(initial)* ranges may be outstanding. A
`BlockRequest` while `rank` is not 0, or one that would leave more than 16 ranges outstanding,
is not served: the uploader answers `Error{DENIED}` and closes the file stream. The queue entry
is kept for 30 min as after a lost connection (§12.5).

- **A range is one block.** `offset` is a multiple of 256 KiB, and `length` is the length of
  that block: 256 KiB, or what is left of the file. `offset` and `length` have the same number
  of entries, every `offset` is below the file's size, and a request names each block once.
  A request that breaks this, or that names a block that is outstanding, is answered
  `Error{MALFORMED}`, and the stream is closed. The queue entry is kept for 30 min, as after
  `DENIED`.
- A block of a piece that the uploader does not hold is not sent. The uploader sends a
  `FileStatus` with its `have` at once, outside the limit of 1 per 5 min (§12.5), and the
  range is no longer outstanding. An uploader that
  cannot read a block of a piece its `have` holds clears that piece from `have` first and
  does the same; a `have` that did not change would leave the range outstanding.
- **When a slot is taken away** (`FileStatus` with `rank` above 0), every outstanding range
  is void on both sides. That `FileStatus` MUST carry `recheck`. The uploader finishes at
  most the `Block` it is sending. Frames do not interleave on an exchange (§5.1), so that
  `Block` arrives before the `FileStatus`, while its range is still outstanding, and is used
  as any other. A `Block` after that `FileStatus` is a protocol fault (§12.3). With `recheck` above 0 the downloader does not answer, and
  the stream closes (§12.5). With `recheck` = 0 the
  downloader answers with an empty `FileRequest`, and the uploader ignores every
  `BlockRequest` until that arrives: one that crossed the `FileStatus` on the way is neither
  served nor an offence. With a new slot the ranges are counted from zero.

### 12.3 `Block`

```protobuf
message Block {
  uint64 offset = 1;
  uint32 size = 2;            // number of bytes that follow this frame; the range length when not compressed; at most 256 KiB
  uint32 plain = 3;           // set when those bytes are zstd: length of the file data they hold
}
```

The frame is followed by exactly `size` bytes. They are not inside the protobuf body, so a block
is never copied through the serializer. Without `plain` they are file data as it is.

Per-block overhead is about 12 bytes on 256 KiB.

**Compressed blocks.** Most shared files are compressed media and archives and gain nothing.
Text, databases, disk images and file set manifests gain a lot. A sender MAY therefore compress
a block, and every node MUST be able to receive one.

- **One `Block` per range.** A `Block` answers exactly one range of a `BlockRequest`, with that
  range's `offset` and length. The uploader MUST NOT split or merge ranges. Compression is
  over the bytes of that `Block` alone. A `Block` whose `offset` is that of no outstanding
  range is a protocol fault (below).
- **Send order.** The uploader SHOULD send the outstanding ranges in the order they were
  asked for: those of an earlier `BlockRequest` before those of a later one, and within one
  request in the order of its `offset` entries. So the downloader decides what comes first
  (§12.6). A `Block` that arrives out of that order is no protocol fault.
- Profile: one magicless zstd frame per `Block` as in §5.3, without a dictionary. Each range is
  compressed on its own, so blocks can be asked for in any order and from any source. A
  `Block` whose frame names a dictionary is a protocol fault (below).
- `plain` is the length of the block after decompression and is at most 256 KiB. A receiver
  stops when the output would exceed `plain`. Output of another length, or input that does not
  decode, is a protocol fault (below).
- **Length.** `plain` (compressed) or `size` (not compressed) MUST equal the length of the
  range the `Block` answers, and `size` MUST NOT exceed 256 KiB. A `Block` that breaks either
  is a protocol fault (below).
- Hashes are over the file data, never over the compressed bytes (§7.2).
- **When to compress.** A sender sends a block compressed only when that saves at least 1/8 of
  its length *(initial)*. After a block that did not, it SHOULD NOT try the next 15 blocks
  *(initial)* of that file; blocks asked for with `raw` count among the 15. It MAY remember the result per file, since a file never changes, and
  it MAY skip files it expects to gain nothing without trying.
- There is no list of file types. A name is not part of a file's identity (§7.1) and need not
  match the content; the measured saving decides.
- The data of a turn (§12.5) is counted in file bytes, so a turn is 4 pieces whether or not
  they were compressed.
- **What it shows.** The sizes of compressed blocks depend on the content. An observer who has
  a copy of a file can compress it the same way and compare the sizes with a transfer it
  sees; encryption does not hide them. Blocks sent as they are all look alike. Either side can
  therefore turn compression off: a sender MAY be set never to compress, and a sender MUST NOT
  compress a range that was asked for with `BlockRequest.raw`.

The codecs and the alternatives are compared in
[`docs/compression-comparison.md`](docs/compression-comparison.md) §5.

**A protocol fault** is a `Block` that the downloader refuses without a hash: a wrong length,
bytes that do not decode, an `offset` that was not asked for. The downloader reads no further,
closes the stream and does not use the node as a source of that file for 4 h *(initial)*. It
sends no `BadBlock`, and the fault counts towards no ban. Only a block of the right form whose
hash fails is a **bad block**.

**A block that fails.** Every block is verified on its own (§7.2), and a range is one block
(§12.2), so the sender of a bad one is known exactly. The downloader discards the block and asks another source for it.

| Bad blocks from one node ID | What the downloader does |
|---|---|
| 1 | MUST NOT use the node as a source for that file for 4 h *(initial)* |
| 2 within 24 h *(initial)*, in any files | MUST ban the node ID for 24 h *(initial)*, and the host address the second one came from for 1 h *(initial)* |
| 1 more within 30 days *(initial)* after a ban ended | MUST ban the node ID for 30 days *(initial)*, and the host address for 1 h *(initial)* |

- **A ban of the node ID is that of §4.6:** its open connections are closed, a handshake of
  that node ID is closed as soon as the key is known, and nothing is downloaded from the node
  or uploaded to it. **A ban of the host address** closes only the connections of the banned
  node ID; for 1 h it drops the datagrams from the address that would start a new flow, so
  that no new connection comes from there. Connections of other node IDs from the address
  stay open. The host address is an IPv4 /32 or an IPv6 /64; private addresses are exempt
  where §4.6 exempts them.
- **One bad block is not a ban.** A disk or a memory fault at an honest node gives one. Two
  within a day is a node that should not be serving.
- **No ban is for ever.** A node ID costs nothing to replace, so a ban for good would stop no
  attacker and would only keep out a node that has since been repaired. The address ban is
  short because several nodes share an address behind a NAT; it is there so that a new key
  from the same address gains nothing for that hour.
- A block counts only when the chaining value it was checked against was itself verified
  under the root (§12.4). The fault is then the block's, and its sender is certain.
- Verified values can still belong to a false size: the tree of a shorter file is a part of
  the true one. A bad block therefore counts against its sender only once one block of the
  file has passed under that size. Before that the downloader drops the size and the values,
  counts nothing and sends no `BadBlock`. It does not use the sender of the block, nor the
  sources of hash values under that size, as sources of that file for 4 h *(initial)*.
- The counts and the bans are kept across restarts. They are counted per node ID and are
  local: nothing is reported to other nodes.
- Sources that are not nodes follow the rules of their extension (§13.1). eD2K sources follow
  §14.4: a block of eD2K data that fails under the BLAKE3 tree marks the (`root`, `ed2k`)
  pair false and counts against no peer.

**Telling the sender.** The downloader tells the sender once per bad block, on the file
stream, before it closes the stream or the connection. When the block leads to a ban, the
`BadBlock` is sent and the ban then closes the connection, so it may not arrive:

```protobuf
message BadBlock {            // type 38, downloader → uploader, on the file stream
  uint64 offset = 1;          // start of the block that failed, in the file of the stream
}
```

- It is not answered.
- The sender MUST verify that block of its own copy before it serves the block again, to
  anyone. If the block fails, the piece it lies in is no longer complete: the sender clears
  its bit in `have` and fetches the piece again, or stops sharing the file.
- If the block passes, the sender does nothing more. A `BadBlock` proves nothing, and a node
  MUST NOT ban a peer because of one.
- A sender acts on a `BadBlock` only for a block that it sent to that peer on that stream
  within the last 10 min *(initial)*, on at most one per block and hour and on 16 per peer
  and day *(initial)*, and ignores the rest. Each one costs it a read and a hash of 256 KiB.
- This is how an honest node with a damaged file finds out, and stops collecting bans.

### 12.4 Hashes

```protobuf
message HashRequest {
  uint64 first = 1;           // first block index
  uint32 count = 2;           // number of blocks; at most 32,000 (1,000 KiB of values)
}

message HashAnswer {
  bytes cvs = 1;              // count × 32 B block chaining values
  bytes proof = 2;            // n × 32 B sibling values linking the range to the root
}
```

`cvs` are the BLAKE3 chaining values of the blocks `first .. first+count−1`, in order; a block
is 256 KiB, except the last of the file, which may be shorter. `count` is at most 32,000
*(initial)*, so that `cvs`, a proof of any depth and the frame fit the 1 MiB of §5.2; a
downloader asks for more in several requests. `proof` holds the sibling values
needed to recompute the root from that range.

**The tree.** Let n = ceil(size / 256 KiB) be the number of blocks. Level 0 holds their n
chaining values. Each level above halves the count:

```
count(0) = n                    count(k) = ceil(count(k−1) / 2)
node(k, j) = parent( node(k−1, 2j), node(k−1, 2j+1) )     if both exist
           = node(k−1, 2j)                                 if it is the last and has no partner
```

`parent` is BLAKE3's parent node function. The level with one node is the root, level L; it is
finalised with the `ROOT` flag and equals the file's root. Every other parent is computed
without that flag. This is the tree BLAKE3 itself builds (§7.2): a node without a partner is
carried up unchanged, which puts the largest power of two on the left. The chaining value of a
block is that of its subtree inside the file: its chunks keep their place in the file as
BLAKE3's chunk counter, so block i starts at chunk 256 i.

**The proof.** The range covers the nodes `a(k) .. b(k)` of level k, with `a(0) = first`,
`b(0) = first + count − 1`, `a(k) = floor(a(k−1) / 2)` and `b(k) = floor(b(k−1) / 2)`. For
k = 0, 1, … up to the level below the root, `proof` holds, in this order:

- `node(k, a(k) − 1)` when `a(k)` is odd (the left sibling of the range's first node),
- `node(k, b(k) + 1)` when `b(k)` is even and that node exists (the right sibling of its last).

A level contributes nothing when the range spans it whole. The receiver recomputes level by
level up to the root and compares with `root`. A request for all blocks has an empty proof. A
file of one block (n = 1) needs no hash exchange: its root is the block's own output finalised
as root (§7.2). A file of size 0 has the root of the empty input, no block and no hash
exchange; it is complete once created. `HashRequest` is answered `Error{MALFORMED}` when
`count` is 0, when `first + count` exceeds n, when `count` exceeds its limit, or when n is at
most 1.

- `HashRequest` is sent on the file stream, at any `rank`, one at a time. A `HashRequest`
  that has no answer within 30 s *(initial)* has failed, and the downloader asks another
  source for those values. A `HashRequest` that arrives while one is unanswered is answered
  `Error{BUSY}`, which does not close the stream; a downloader whose request timed out sends
  no further `HashRequest` on that stream.
- An uploader answers at most 2 n values per node ID and file within 24 h *(initial)*, over
  all streams together, and `Error{BUSY}` beyond that. A new stream does not renew the
  count. Values it does not hold, as a partial source may not, are answered
  `Error{NOT_FOUND}`. Neither closes the stream.
- A `HashAnswer` whose `cvs` or `proof` has another length than the request calls for, or
  that does not lead to the root, is discarded. The downloader closes the stream and does not
  use the node as a source of that file for 4 h *(initial)*. It is not a bad block and counts
  towards no ban.

### 12.5 Waiting in the queue

An uploader's queue can hold thousands of downloaders. Keeping a connection and a stream of
updates for each would cost far more than eMule's reask, so the uploader sets how each one
waits.

**One entry per downloader.** An uploader keeps one queue entry per node ID, for one file. A
downloader has at most one file stream open to an uploader. Opening a file stream for another
file replaces the request and keeps the position, or the slot if one is granted (see "Several
files in one turn" below). The uploader closes the old stream only after it has answered the
first `FileRequest` of the new one with a `FileStatus`, and finishes the `Block` it is sending
on the old stream first. The ranges outstanding on the old stream are then void, and ranges
are counted from 0 on the new one. A `Block` cut short on a stream that the uploader replaced
is not a protocol fault (§12.3). If the uploader answers the new stream with an `Error`
(`NOT_FOUND`, or `MALFORMED` for a bad bid), the entry stays as it was, with the old file and
its stream.

**Stay or come back.** The first `FileStatus` to a queued downloader carries `recheck`, and a
later one carries it when it changes:

| `recheck` | Meaning |
|---|---|
| 0 | Stay connected. The uploader expects to serve this downloader soon |
| 1800 to 14400 | Close the file stream and open a new one after this many seconds |

A value from 1 to 1799 is read as 1800 and a value above 14400 as 14400.

- The uploader chooses the value per downloader, shorter near the front of the queue. How many
  downloaders it keeps connected is local policy.
- **The uploader closes the stream** after a `FileStatus` with a `recheck` above 0, and the
  connection if nothing else uses it. An uploader that sells priority first waits up to 30 s
  *(initial)* for the receipt of an entry that bids ([`extensions/credit.md`](extensions/credit.md) §4.1). A downloader cannot stay connected against the uploader's
  choice.
- The downloader comes back by opening a new file stream; `brief` keeps the answer small.
- It MAY come back earlier to report changed endpoints or relays (what the uploader dials by,
  below), or to ask for another file, which keeps its position as in eMule; these reasons and
  a changed bid ([`extensions/credit.md`](extensions/credit.md) §4.1) together at most once per 30 min
  *(initial)*. Any other
  early `FileRequest` MAY be answered with `Error{BUSY}` and `retry_after`. The
  position is not affected.
- The entry is kept until 30 min *(initial)* after the time the downloader was told to come
  back. Then it is removed.

**While connected.** The uploader sends a `FileStatus` only

- when a slot is granted (`rank` = 0) or taken away (`rank` above 0),
- to change `recheck`,
- while the downloader holds a slot, for a changed `have`, at most once per 5 min *(initial)*,
- at once, outside that limit, for a block of a piece it does not hold (§12.2),
- every 30 min *(initial)* as a **ping**: a `FileStatus` with `ping` set, and with `rank` and
  `have` if they changed,
- in answer to a `FileRequest` that asks for sources (§12.1),
- in a priority turn of the credit extension, for a changed `price` or `due`
  ([`extensions/credit.md`](extensions/credit.md) §3.2).

The downloader answers a `FileStatus` that has `ping` set, and no other but the one that takes
a slot away with a `recheck` of 0 (§12.2), with a `FileRequest` within 30 s *(initial)*, empty unless its own `have` changed. If it does
not, or the connection is lost, the uploader closes the stream. The entry is then kept for
30 min *(initial)* so that the downloader can come back, and removed after that.

**A free slot.** The uploader contacts the next downloader at once.

- Connected: `FileStatus` with `rank` = 0 on the open stream.
- Not connected: the uploader dials the downloader at the observed endpoint of its last
  connection, and at a claimed endpoint (§8.2 (5)) only if it is acceptable (§8.2 (7), and
  (8) in a named network), or by `Punch` through the relays its `FileRequest`s last named (§9.5);
  of `relays4` and `relays6` it uses only those at acceptable addresses, as for a source
  record (§10.3). It opens an exchange and sends `SlotOffer`. The downloader answers on that exchange with a first `FileRequest`, which
  names a file by `root`: the one it is queued for, or another one of this uploader that it
  now wants (a slot belongs to the downloader, below). The uploader
  answers it with a full `FileStatus` whose `rank` is 0, and the exchange continues as a file
  stream with the slot granted. A downloader that wants nothing of this uploader any more
  answers `Error{NOT_FOUND}` and its entry is removed. It answers `Error{DENIED}` to an offer
  from a node it is not queued at; that entry is removed too.
- A downloader that has a file stream of its own open to the uploader when the offer
  arrives keeps that stream: it closes the offer exchange unanswered, and the uploader
  grants the slot on the open stream. An offer exchange that ends unanswered while such a
  stream is open is read so, not as a failed offer.
- The uploader waits 30 s *(initial)* from the start of the dial. After that the slot goes to
  the next downloader.
- A downloader that could not be reached keeps its position and is offered the next free slot.
  After a second failure it gets no further offers.
- In both cases the entry stays until its normal end, 30 min after the time the downloader was
  told to come back. An entry that was told to stay connected (`recheck` = 0) has no such
  time; it ends 30 min *(initial)* after the offer that failed. A `FileRequest` from the
  downloader before then clears the failures.

**Length of a slot.** A slot is one turn at an uploader.

- An uploader SHOULD transfer 16 MiB *(initial)* (4 pieces) to a downloader before it takes
  the slot away, as long as the downloader keeps requesting.
- An uploader SHOULD take the slot away after 1 h *(initial)* even if less was transferred, so
  that a downloader with little bandwidth does not hold it. Bytes count from the `FileStatus` that
  granted the slot.
- After 16 MiB, or after 1 h, the uploader MAY go on while nobody else is waiting.
- A slot with no outstanding range for 30 s *(initial)* is taken away.
- A slot outlives its stream only by a hold: when the downloader closes or resets the stream
  that holds its slot, or the uploader answers the `FileRequest` on a `SlotOffer` exchange with
  `NOT_FOUND`, the uploader holds the slot for 30 s *(initial)* for a new file stream and then
  takes it away. A hold is not started again while it runs, and the rule of no outstanding
  range does not apply during it; the bytes and time of the turn go on counting. A slot whose connection is lost is taken
  away at once.
- A downloader that loses its slot stays in the queue. Its new position is local policy; the
  `FileStatus` that takes the slot carries it as a `rank` of 1 or above. Where no new
  position has been chosen, as when a slot is taken away for having no outstanding range, the
  `rank` is the last one above 0 that the downloader was given, else 1.

**Several files in one turn.** A slot belongs to the downloader, not to the file.

- A downloader that holds a slot (`rank` = 0) MAY open a file stream for another file. It
  replaces the open one, and the uploader MUST keep the slot: the bytes and the time of the
  turn go on counting across files. A short slot for a member of a website is the one
  exception ([`extensions/name.md`](extensions/name.md) §6.4).
- The uploader answers the new `FileRequest` with `rank` = 0, or with `Error{NOT_FOUND}`;
  after `NOT_FOUND` the slot stays on the old stream, or is held as above when there is none.
- The downloader SHOULD set `brief` when it knows size and name.
- A turn stays 16 MiB or 1 h, so several files give no more than one.

**Number of slots.** An uploader SHOULD choose the number of its upload slots so that 16 MiB
take 5 to 30 min *(initial)* in each, which is about 9 to 55 KiB/s per slot. It SHOULD offer
at least 2 *(initial)* slots; below about 18 KiB/s of upload that minimum comes first and a turn takes longer.

The comparison with eMule's reask and with the alternatives is in
[`docs/queue-status-comparison.md`](docs/queue-status-comparison.md).

### 12.6 Playing while downloading *(informative)*

A client MAY play a video or a sound while it downloads it, and seek to any place in it. The
wire needs nothing more for that: any block can be asked for from any source in any order
(§12.2), every block is verified on its own (§7.2), and an uploader sends in the order asked
(§12.3). What is left is the downloader's choice of blocks, which stays local. A client that
plays SHOULD choose as follows.

- **Only verified bytes are played.** A block goes to the decoder after it passed under the
  root (§12.3). For a seek the client asks for the hash values of the blocks around the new
  position, with their proof (§12.4), not for those of the whole file. `HashRequest` is allowed
  at any `rank`, so this can be done while queued.
- **The index first.** Many containers keep their index at the end of the file: MP4 with
  `moov` last, the cues of Matroska, `idx1` of AVI. The client fetches the first block and the
  blocks that hold the index before it plays, so that it can map a time to an offset.
- **A window ahead of the play position.** The blocks of the next seconds to be played, about
  30 s of media, are asked for in file order and before any other block. The media rate comes
  from the container or from `FileInfo.bitrate` (§5.4). Blocks outside the window are chosen as
  for any download, rare pieces first, so that a swarm of viewers does not hold only the
  starts of a file.
- **Few ranges outstanding per source.** Every range already asked for is sent before one asked
  for after it (§12.3). A client that plays therefore keeps only as many ranges outstanding at
  each source as keep it busy, 2 rather than 16, and asks its fastest sources for the window.
- **After a seek** the window moves. Its blocks go to the sources with the fewest ranges
  outstanding. The first block of the window MAY be asked for from two sources at once; the
  copy that arrives second is dropped. Ranges still outstanding for the old position are taken
  as they arrive.
- **Keeping the slots.** A slot with no outstanding range for 30 s is taken away (§12.5). A
  client that pauses goes on downloading outside the window and so keeps its slots.
- **What limits it.** Playback needs as many slots as the media rate divided by the rate of one
  slot. At 9 to 55 KiB/s per slot (§12.5), 1080p at 5 Mbit/s needs some 11 to 70 sources at
  once. A seek waits at least for one block of 256 KiB from one source, 5 to 28 s at those
  rates. Before it holds slots a client waits in the queue as for any download; HTTP sources
  ([`extensions/http-source.md`](extensions/http-source.md)), links at hosters
  ([`extensions/hoster.md`](extensions/hoster.md)), peers in the local network and the
  priority slots of [`extensions/credit.md`](extensions/credit.md) §5 start a playback sooner.
- A message to take back outstanding ranges was left out to keep the protocol small; few
  ranges outstanding per source (above) serves the same end.

A layout of media made for playing, fragmented MP4 with its index at the start, is in
[`extensions/chaintube.md`](extensions/chaintube.md) §2.2.

---

## 13. Other peer messages

### 13.1 Optional messages

Browsing a peer's shared files, chat, rooms, HTTP sources, file sets, live streams, publish and subscribe, and finding nodes in the local network are optional extensions. A node announces
each with a capability bit and need not implement any of them. Hidden sets, sets with
several authors, other names of a file, torrents and Usenet posts as file sets, videos named on a chain, shared filter lists, first
contact and finding a known node are optional too. They add file formats, links and four record
kinds, and have no bit of their own. Readable names for keys, websites among the things they
name, are optional as well; they add two record kinds, a link and a bit. Links at file hosters are a further kind of source, with
a bit of their own.

**Other networks and data sources are joined under the root, never beside it.** A name from
outside leads to a root through one record kind, `ALIAS`, and the rules that every such
extension follows are in [`extensions/alias.md`](extensions/alias.md) §5.
Anonymous paths, which put other nodes between a node and its peers, are optional as well.
So are credits that count at every uploader; they need a ledger outside ed3k.
Paid subscriptions give each viewer of a channel or a set access of their own for the time they
paid; ed3k carries the passes and keys, not the payment.
A video site whose catalogue is on a chain keeps its media in ed3k as file sets; its site is a
program of its own next to an ordinary node, and no further role (§11).

| Extension | Capability | Wire elements | Document |
|---|---|---|---|
| Browse | bit 3, `BROWSE` | message types 14, 15 | [`extensions/browse.md`](extensions/browse.md) |
| Chat | bit 4, `CHAT` | message types 16, 18 | [`extensions/chat.md`](extensions/chat.md) |
| Rooms | bit 19, `ROOM` | message types 42, 43; `ed3k:room` links; uses record kind 7 | [`extensions/room.md`](extensions/room.md) |
| HTTP sources | bit 9, `HTTP` | fields in `FileRequest`, `FileStatus`, `Store` and `KadAnswer`; `ws` and `wh` in links; `ed3k:cache` links | [`extensions/http-source.md`](extensions/http-source.md) |
| File sets | bit 10, `SETS` | a manifest file format; record kinds 4 and 5; fields in `FileInfo`, `FileRequest`, `FileStatus`, `Record`, `Store` and `KadAnswer`; `path` in links; `ed3k:set` links | [`extensions/file-set.md`](extensions/file-set.md) |
| Live streams | bit 11, `LIVE` | message types 19 to 21; record kind 6; a channel file format; `ed3k:live` links | [`extensions/live.md`](extensions/live.md) |
| Publish and subscribe | bit 12, `PUBSUB` | message types 22 and 23; record kind 7; a field in `Record` | [`extensions/pubsub.md`](extensions/pubsub.md) |
| Finding nodes in the local network | bit 13, `LAN` | message types 24 and 25; a multicast datagram outside any connection | [`extensions/lan-discovery.md`](extensions/lan-discovery.md) |
| Anonymous paths | bits 14 and 15, `ANON` and `HOP` | message types 26 to 34 and 39; record kinds 11 and 12; a field in `Record` and `Store`, two in `KadAnswer` | [`extensions/hops.md`](extensions/hops.md) |
| Credits | bit 16, `CREDIT` | message types 35 to 37; a field in `FileRequest` and in `FileStatus`; a ledger on a chain of its own, reached by RPC | [`extensions/credit.md`](extensions/credit.md) |
| Paid subscriptions | bit 18, `PAID` | message types 40, 41; fields in `Channel`, `LiveSegment`, `LiveJoin`, `LiveState`, `FileSet`, `Member`, `Sealed`, `TopicJoin`, `TopicState` and `TopicItem`; `paid` in `ed3k:live` and `ed3k:set` links; `ed3k:pass` links | [`extensions/subscription.md`](extensions/subscription.md) |
| Hidden sets | none (`SETS`) | record kind 8; a field in `Record` and in a manifest; `s` in `ed3k:set` links | [`extensions/hidden-set.md`](extensions/hidden-set.md) |
| Sets with several authors | none (`SETS`) | record kind 9; two fields in `Record`; fields in a manifest | [`extensions/multi-author-set.md`](extensions/multi-author-set.md) |
| Links at file hosters | bit 17, `LINKS` | a field in `FileStatus`, `Store` and `KadAnswer`; three result codes; a field in a manifest; `hl` in `ed3k:file` links; two schemes of `ALIAS` | [`extensions/hoster.md`](extensions/hoster.md) |
| Other names of a file | none | record kind 13; a field in `Record`, `Store` and `KadAnswer` | [`extensions/alias.md`](extensions/alias.md) |
| Torrents as file sets | none (`SETS`) | two schemes of `ALIAS`; a field in a manifest; `bt` in `ed3k:file` links | [`extensions/torrent.md`](extensions/torrent.md) |
| Usenet posts as file sets | none (`SETS`) | two schemes of `ALIAS`; a field in a manifest; an article list file format | [`extensions/usenet.md`](extensions/usenet.md) |
| Videos named on a chain | none (`SETS`) | a field in a manifest; `b3`, `sz` and `ipfs` in `ed3k:set` links, `ipfs` in `ed3k:file` links; a layout for a set that an IPFS gateway or a web server can serve as it is; `ed3k:set` and `ed3k:file` links for the attachments of other entries; contracts on a chain of their own, reached by RPC | [`extensions/chaintube.md`](extensions/chaintube.md) |
| Shared filter lists | none | a list file format | [`extensions/filter-list.md`](extensions/filter-list.md) |
| First contact | none | a node list file format; `ed3k:nodes` links | [`extensions/bootstrap.md`](extensions/bootstrap.md) |
| Finding a known node | none | record kind 10; a field in `Store` and in `KadAnswer`; `ed3k:friend` links | [`extensions/friend.md`](extensions/friend.md) |
| Readable names | bit 20, `NAMES` | message types 44, 45; record kinds 14 and 15; two fields in `Record`, one in `Store` and `KadAnswer`; a name list file format; `ed3k:name` links; a DNS record and a file at a web server; a local web server for websites | [`extensions/name.md`](extensions/name.md) |

A node without the capability answers the message types with `Error{UNSUPPORTED}` (§5.2) and
ignores the fields (§16), with one exception that §10.3 states: a `ROUTER` stores, checks and
returns the records of every kind in this table, and the fields of `Store` that belong to a
source record (HTTP sources, links at hosters), whether or not it sets the bit. The record
kinds of version 1 are therefore 1 to 15, and the documents above define their checks.

### 13.2 Links

```
ed3k:file?b3=<root, base32 lower case, no padding>&sz=<size>[&dn=<url-encoded name>][&ed2k=<32 hex>][&ws=<url-encoded URL>][&wh=<url-encoded URL>][&hl=<url-encoded link>]
ed3k:node?id=<node ID, 32 hex>&ep=<ip:port or [ip6]:port>[&ep=<endpoint of the other family>]
```

Hex tokens are written lower case and base32 tokens (the alphabet of RFC 4648 §6) lower case
without padding; a reader
accepts upper case and padding as well, padding only in the form of RFC 4648 (to a multiple
of 8 characters). This holds for every link here and in the extensions.

- The scheme `ed3k` is read in either case (RFC 3986 §3.1). The kind of a link (`file`,
  `node`, …) and the parameter names are case-sensitive: `ed3k:File` is a kind and `B3` a
  parameter that nobody knows.

- Parameters come in any order. A value is UTF-8, percent-encoded as RFC 3986 says; `+` is a
  plus sign, not a space. A reader ignores a parameter it does not know. A writer writes `ep`
  as the template shows, with `[`, `]` and `:` as they are; a reader accepts them
  percent-encoded as well.
- `dn` may be absent; a link without it is valid.
- A link is invalid when a parameter that occurs once by its meaning (`b3`, `sz`, `dn`,
  `ed2k`, `id`) is repeated, when `b3` or `sz` of an `ed3k:file` link is missing, or when the
  bits of a base32 token beyond its length are not 0. It is invalid too when:
  - `b3` does not decode to 32 bytes;
  - a hex token has another length than its parameter calls for (32 digits for `id` of an
    `ed3k:node` link and for `ed2k`; `ed3k:set` defines an `id` of its own, in base32,
    [`extensions/file-set.md`](extensions/file-set.md) §5.4);
  - a base32 or hex token holds a character outside its alphabet;
  - `sz` is not a decimal number, has a sign or a leading zero (`0` itself is a size), or is
    above 2⁶⁴ − 1;
  - an `ed3k:node` link has no `id`, no `ep`, or two `ep` of one family;
  - an `ep` is not `ip:port` or `[ip6]:port` as the template shows, has port 0, holds an
    IPv4-mapped IPv6 address, or has an address that §8.2 (7) does not accept (and (8) when
    the node that uses the link is in a named network; a link names no network, so a reader
    that does not know the network yet applies this check when it uses the link).

  `ws`, `wh`, `hl` and `ep` are lists and may repeat. An extension that adds a parameter
  states how often it may occur; `bt`, for example, occurs at most twice, once per hash length
  ([`extensions/torrent.md`](extensions/torrent.md)).

`ws` names an HTTP source for the whole file and may occur several times. `wh` names a URL of
the file's hash tree. A client without the HTTP source extension ignores both. That extension
also defines `ed3k:cache`, a link to a cache server. The file set extension adds `path` to
`ed3k:file` and defines `ed3k:set`. The live stream extension defines `ed3k:live`. The hidden
set extension adds `s` to `ed3k:set`, the torrent extension adds `bt` and the extension for links at file hosters adds `hl` to `ed3k:file`, the first contact extension defines `ed3k:nodes`, and the
extension for finding a known node defines `ed3k:friend`. The extension for videos named on a
chain adds `b3`, `sz` and `ipfs` to `ed3k:set`, and `ipfs` to `ed3k:file`. The paid subscription
extension adds `paid` to `ed3k:live` and `ed3k:set`, and defines `ed3k:pass`. The room
extension defines `ed3k:room`. The extension for readable names defines `ed3k:name`.

### 13.3 Not carried over

Preview requests (a file is played while it downloads instead, §12.6), PeerCache, the eD2K server message, comment packets separate from file status,
a separate packet type for compressed parts (a flag on `Block`, §12.3), Kad v1, and every
"version N of packet X" have no ed3k equivalent.

---

## 14. Coexistence with eD2K (OPTIONAL)

This section applies only to a client that speaks **both** eD2K and ed3k. It specifies, in terms
an eD2K implementer can follow without reading the rest, how such a client lets eD2K peers know
before any connection that it can also be reached over ed3k. Nothing here changes a byte that a
legacy eD2K client or server interprets.

### 14.1 The port rule

> A dual-protocol client listens for ed3k on the **UDP port whose number equals its advertised
> eD2K TCP port**, on every address it advertises.

Every eD2K source record, from a server, from source exchange, from Kad or from a link, carries an
address and the TCP port. None of the server or source-exchange formats carries the UDP port.
With this rule the ed3k endpoint of any eD2K source is known without one extra byte.

**Can ed3k share the eMule UDP socket?** Yes, when the user's eD2K UDP port has the same number
as the TCP port; otherwise ed3k has a socket of its own. Sharing works as follows:

- eD2K UDP cannot be told from ed3k by the first byte. An obfuscated eD2K datagram starts with a
  random marker byte, and an ed3k datagram starts with a random salt.
- It **can** be told reliably by a keyed check. Plaintext eD2K datagrams start with one of
  `0xE3 0xE4 0xE5 0xC5 0xD4 0xA3 0xB2`, which an ed3k salt never does (§4.3). For every other
  datagram the receiver runs §4.5 first: a live ed3k flow from that source address and port, else the
  4-byte `magic` under its own key (steps 1 and 2). Only if both fail does the datagram go to the eD2K trial decryption, which has its own
  32-bit magic. A datagram is misrouted with probability 2⁻³² per key tried, in either
  direction.
- A legacy client that receives a stray ed3k datagram fails its trial decryption and drops it, as
  it does with any noise.

**Alternatives considered:** always sharing the eD2K UDP port (one mapping, but servers and source
exchange do not tell a peer what that port is); a dedicated port carried in tags (no demultiplexing
at all, but invisible through legacy servers and through source exchange relayed by legacy
clients).

### 14.2 The derived user hash

A dual-protocol client derives its eD2K user hash from its ed3k node ID `N`:

```
byte   0  1  2  3  4  5   6  7  8  9  10 11 12  13  14   15
       N0 N1 N2 N3 N4 14  N5 N6 N7 N8 N9 N10 N11 c0  111  c1

(c0, c1) = H("ed3k v1 ed2k user hash", N[0..12))[0..2)
```

Bytes 5 and 14 are eMule's own client markers (14 and 111) and stay as they are, so every eD2K
client still classifies the peer as eMule.

**Recognising it.** For any user hash `u`: take `n = u[0..5) ‖ u[6..13)`, compute the two check
bytes, and compare with `u[13]` and `u[15]`, with `u[5] = 14` and `u[14] = 111`. On a match:

- the peer *probably* speaks ed3k version 1, at the UDP port of §14.1
- `n` is the first 12 bytes of its node ID, which is exactly the `obf_id` of §4.3, so the first
  ed3k datagram can be built at once
- after the ed3k handshake, the peer's real node ID MUST start with `n`; otherwise close with
  `WRONG_PEER`. A peer whose node ID does not start with `n` cannot unmask the datagrams, so a
  false hint mostly shows as a handshake that gets no answer (§4.1)

It is a **hint**: one legacy hash in 65,536 matches by accident. The cost of a false hint is one
unanswered handshake. An implementation MUST fall back to eD2K when ed3k does not answer and
MUST NOT drop a source for it.

**Where the hint arrives before connecting**

| Channel | User hash present? |
|---|---|
| Kad source search result | always: the source ID of a Kad source record is the publisher's user hash |
| Source exchange v2, v3, v4 (`OP_ANSWERSOURCES`, `OP_ANSWERSOURCES2`) | always |
| Extended source exchange | when the `CT_EMULE_USERHASH` tag is sent |
| Server `OP_FOUNDSOURCES_OBFU` | when the per-source option byte has `0x80` set |
| Server `OP_CALLBACKREQUESTED` with the crypt extension | yes |
| Server `OP_FOUNDSOURCES`, UDP `OP_GLOBFOUNDSOURCES`, source exchange v1 | no |

For a LowID source the hash is known but no address is. The hint then says that the file is worth
looking up on ed3k.

**Precedents.** eD2K clients have always carried signals in the user hash: eMule's 14/111 and the
older 13/110. Other clients mark their hashes with bytes of their own in the same way.

**Costs**

- The user hash changes once when a client adopts this. Other peers key credits by user hash, so
  credits earned at them are lost. An implementation SHOULD tell the user before switching.
- The eD2K and ed3k identities become linkable by anyone. That is the purpose.
- The same marks tell an eD2K server or peer, before any ed3k message, that the client speaks
  ed3k, and it may refuse the client for that. A client that must not be recognised as an ed3k
  node on eD2K keeps a user hash that is not derived.
- 96 of 112 free bits remain random-looking; nothing in eD2K depends on more.

### 14.3 Other signals

- **Explicit tags.** A tag in the eD2K hello, in the Kad source record, in extended source
  exchange, in the server login and in an eD2K link can state ed3k support outright, where the
  user hash is a hint. They are not part of this specification, since a dual-protocol client
  works without them. A proposal with tag numbers checked against existing clients is in
  [`docs/ed2k-tags.md`](docs/ed2k-tags.md).
- **Signals not used.** Free bits in the connect-options byte, in the hello option tags and in
  the Kad option tag, fixed marker bytes in the user hash, a Kad record mapping user hash to
  node ID, and blind probing of the port were considered. None of them reaches a peer before
  the first connection, through legacy servers and legacy relaying clients, and carries the
  node ID. The comparison is in
  [`docs/ed2k-signal-comparison.md`](docs/ed2k-signal-comparison.md).
- **eD2K callbacks as a punch signal.** The rendezvous of an eD2K server gives two firewalled
  dual-protocol clients each other's mapped UDP endpoint and user hash, which is what §9.5
  needs. The Kad buddy callback does not. It is not part of this specification;
  [`docs/ed2k-tags.md`](docs/ed2k-tags.md) section 5 has the comparison and a procedure.

### 14.4 Using both networks for one file

- A dual-protocol client learns (`root`, `ed2k`) pairs from `FileInfo` and `FileStatus`.
- A pair is trusted once data has been verified under both: a block under the BLAKE3 tree and the
  part containing it under the eD2K part hash. If a part fails under one hash while its blocks
  pass under the other, the pair is false; the client drops the pair. It does not drop the peers that supplied it,
  which may have had it from a record. A block of eD2K data that fails under the BLAKE3 tree
  marks the pair false in the same way and counts against no peer (§12.3).
- A file's sources from both networks feed one download. Availability is kept as byte ranges
  internally; eD2K part bitmaps and ed3k piece bitmaps are both projections of it.
- A peer reachable over both protocols SHOULD be used over ed3k only.

---

## 15. Overhead summary

Sizes are message bodies, estimated from the encoding rules; they are to be replaced by measured
values once an implementation exists. eD2K figures are from the eMule sources.

| Purpose | eD2K / Kad | ed3k | Why |
|---|---|---|---|
| Client hello | 103 B, plus 2 secure-ident packets (~150 B) | 30–45 B | no hash, key, ports, version tags or server fields |
| Kad contact (IPv4) | 25 B | 22 B | no TCP port, no version byte |
| Kad find + search or publish | 2 requests, 2 answers | 1 request, 1 answer | §10.3 |
| File request with part status | 55 B + bitmap | 36 B + bitmap, once per stream | hash sent once, 4 MiB pieces |
| Source entry (IPv4) | 29 B (v4) | 26 B | `Peer` with 16 B ID |
| Keyword result, note | unsigned | about 80 B more per record, 34 B per publisher | signature, expiry, key once per answer (§10.7) |
| Block header | 32 B per ≤ 10 KiB packet | ~12 B per 256 KiB | QUIC segments the stream |
| Reask while queued | 1 UDP packet each way per 29 min | 1 resumed connection (about 2 KB) per 30 min to 4 h, as the uploader sets; a ping per 30 min for the few kept connected | §12.5 |
| Per-datagram cost | 8–16 B obfuscation | 13 B obfuscation + QUIC header and 16 B AEAD tag | §4.3 |
| New contact | 0 (connectionless UDP) or a TCP handshake | ~2.5 KB QUIC handshake | §10.5 |

What was removed to get there: generic tag lists (every field is typed and absent when empty),
request IDs, fragment markers, the user hash, ports that equal another port, version negotiation
inside messages, and all packets that exist only as a second way to do the same thing.

---

## 16. Versioning and extensions

- A change that an old node would misread gets a new ALPN (`ed3k/2`) and, if the datagram format
  changes, a new `magic`.
- Additive changes use new fields (unknown fields MUST be ignored), new message types (answered
  with `UNSUPPORTED`) and new capability bits.
- **Network rules** (who counts as verified, record lifetimes, an ID puzzle) change by upgrade: a
  capability bit announces support, and a date fixed in the specification switches enforcement.
- A signed message MUST travel as `bytes` and be verified over exactly those bytes; a signature
  that a document defines over a fixed layout of fields is computed over that layout.
  Serialisation is not canonical, except where a document requires the canonical form of §0.
- [`proto/ed3k.proto`](proto/ed3k.proto) holds every message; an extension adds its fields and
  messages there, at the numbers its document names.

Reserved for later: further signature algorithms and the batch form (§10.7), the key puzzle of §6.3, the Kad datagram
binding of §10.6, keyword levels and ranking (§10.8), a data relay and symmetric-NAT probing (§9.4), dropping the eD2K alias (§7.4).
A hybrid post-quantum key exchange for the QUIC binding is evaluated in
[`docs/hybrid-key-exchange.md`](docs/hybrid-key-exchange.md); version 1 does not use it.

---

## Appendix A — initial constants

Rows marked **(wire)** are fixed in wire version 1 and are not tuning values (§0).

| Constant | Value |
|---|---|
| Idle timeout (`max_idle_timeout`) | 30 s |
| Exchanges a peer may have open on a connection, more per path between the two (both directions); a request must arrive whole within | 64, 65; 30 s |
| Keep-alive (PING) by the side that wants a connection kept | 20 s |
| Towards an index node: `max_idle_timeout` (both sides); client PING with a relay reservation there or not `open` / otherwise, jitter | 300 s; 60 s / 240 s, ±10 % |
| Index node, client: redial after a connection ended or a dial failed; open time that resets the wait; further index nodes asked per search at most | 30 s doubling up to 10 min; 60 s; 3 |
| `Hello` of the peer awaited on stream 0 | 30 s |
| Fragment reassembly: messages per source, timeout, one message at most, all messages at most (bytes / messages), attempts in fragments per packet type before an unsplit one; `msg_id` not used again towards one address within | 4, 3 s, 1,500 B, 4 MiB / 65,536, 2; 3 s |
| Full-size path probes lost before the connection is closed | 3 |
| `agent`, `nick` at most **(wire)**; `retry_after` read as at most, 0 read as where no section names a wait | 64 B; 24 h, 60 s |
| Compression threshold | 64 B |
| Maximum frame body **(wire)** | 1 MiB |
| Padding of an unsplit Initial, at most **(wire)** | 19 B |
| Address reports: needed for the lead (of an endpoint or an address), counted for, lead over any other endpoint or address | 3, 60 min, 2 to 1 |
| Unasked connections that show `open`: number, from hosts not sent to for, within | 2 from different sites, 60 min, 60 min |
| Reach tests whose `PROBE`s show `open`: number, within | 2 asked of different sites, 60 min |
| Reach tests: per asker address at A, per target address at B, further probers A tries; A's wait for each B, the asker's wait for A; `PROBE` awaited after the answer, without an answer | 1 per minute, 2 per minute, 2; 3 s, 10 s; 5 s, 10 s |
| Reach test: repeated every, retry after a test without prober; a due test MAY be skipped on evidence from unasked connections less old than | 60 min, 30 s doubling up to 60 min; 60 min |
| Reach test: tests without `PROBE` before `open` is dropped | 2 |
| Relays per node that is not open in a family, at least | 2 |
| Reservations a router holds, at least / recommended | 1 / 8 |
| Reservations a relay holds per host / site, at most (an index node MAY use wider) | 2 / 4 |
| Relay that answered `BUSY`: not asked again for | 30 min |
| Relay: `ttl` (informational) at most, wait for the target's `Punch` answer | the idle timeout, 5 s |
| Punches a target starts per minute: in all, towards one host address; `PROBE`s per punch (shared evenly over I's endpoints), per destination host address and minute | 30, 3; 50, 100 |
| `Punch` forwards per minute: per leaf, per initiator address and leaf | 30, 3 |
| Punch: probe interval per endpoint, duration, probes to a claimed endpoint at most, attempts, back-off (also after an answer that leaves no family to try) | 100–300 ms, 5 s, 3, 3, 30 min |
| Punch, initiator: direct dials before a relay is asked, a relay's answer awaited, handshake awaited after the answer, a `PROBE` used after the answer | 2 s, 7 s, 8 s, 5 s |
| Dual-stack head start for IPv6 | 250 ms |
| Kad: K, α, contacts per answer, R (search / store) | 10, 3, 16, 4 / 8 |
| Kad: nodes that must answer before a lookup may stop, M (search / store) | 5 / 9 |
| Kad: dialled-in contacts per bucket | 5 |
| Kad: bucket refreshed after, contact verified again after, second attempt after a failed one | 60 min, 60 min, 5 min |
| Kad: time for one node to answer, longest lookup | 5 s, 60 s |
| Kad: records in one `Store` at most; terms in `all` and `none` of a `Query` together at most | 256; 64 |
| Source records: targets in `also` per request and per batch, stored again at most after the last successful store, nodes that must store before a target needs its own lookup | 256, 4 h, 4 |
| Diversity, per host / site / block: bucket, answer | 1 / 2 / 4 |
| Diversity, per host / site / block: lookup, counted over its closest set | 1 / 2 / half of R (2 for a search, 4 for a store) |
| Diversity, closest set | as the lookup ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.3) |
| Diversity, per host / site / block: whole routing table | 1 / 10 / 20 |
| Diversity: a host group whose contact was removed is not admitted under another node ID for; removed host groups remembered for | 1 h; 1 h |
| Diversity: `SOURCE` records per file and `LIVE` and `SUB` records per target, `ANON` records per target, `KEYWORD` and `NOTE` publishers counted per entry, per host / site | 8 / 32, 4 / 8, 1 / 1 |
| Diversity, over all targets, per host / site: `HEAD` and `HIDDEN` records held, `GATE` records held, `NAME` records held | 1,024 / 4,096, 64 / 256, 1,024 / 4,096 |
| Diversity: failed or pending dials per file and hour (source endpoints of both families, relays and gates), per host / site / block; per host and hour over all files | 2 / 4 / 8; 8 |
| Record lifetimes: source (also anonymous) and friend, keyword, note, member and alias and claim, live, subscriber, gate, head and hidden head and part and name | 5 h, 24 h, 24 h, 24 h, 10 min, 60 min, 1 h, up to 7 days |
| Search answer: keyword records, notes, sources and anonymous sources together at most; paging secret replaced every | 100, 100, 100; 24 h |
| Keyword target: records kept per file, files per publisher key / per publisher site; notes per file target, in all / per publisher host / per publisher site | 3, 1,000 / 2,000; 64 / 4 / 8 |
| Source record and `FileRequest`: relays per family at most (used); records per file target (and per `LIVE` or `SUB` target) at most; note text at most | 2; 10,000; 512 B |
| Keyword: files per target (level size), levels in use, highest level accepted **(wire)** | 100,000, 1, 15 |
| Keyword: minimum length (protocol / suggested default), keywords published per file | 2 bytes / 3 bytes, 40 |
| Signed record: maximum size, clock tolerance | 1 KiB, 1 h |
| Block, piece **(wire)** | 256 KiB, 4 MiB |
| Hash exchange: block values per request at most, per node ID and file within 24 h at most; answer awaited | 32,000, 2 × the blocks of the file; 30 s |
| Outstanding block ranges | 16 |
| Block compression: least saving, blocks skipped after a miss | 1/8, 15 |
| Bad block, protocol fault or failed `HashAnswer`: source not used for that file | 4 h |
| 2 bad blocks within 24 h: ban of the node ID, of the host address | 24 h, 1 h |
| Bad block within 30 days after a ban ended: ban of the node ID, of the host address | 30 days, 1 h |
| `BadBlock` acted on, at most: per block, per peer; only for a block sent within | 1 per hour, 16 per day; 10 min |
| Queue: time to come back (`recheck`) **(wire)** | 30 min to 4 h |
| Queue: entry kept after the time to come back, or after a lost connection | 30 min |
| Queue: ping interval, answer time | 30 min, 30 s |
| Queue: `have` update to a downloader with a slot | 1 per 5 min |
| Queue: early return for an address or relay change, for another file, or for a changed bid | 1 per 30 min, all together |
| Slot offer: wait, failures before offers stop; an entry told to stay connected kept after a failed offer | 30 s, 2; 30 min |
| Upload slots, recommended minimum | 2 |
| Slot: data per turn, longest turn | 16 MiB, 1 h |
| Slot: time aimed at for one turn | 5–30 min |
| Slot: held without a stream (old stream closed, or `NOT_FOUND`), taken away when no range is outstanding for | 30 s, 30 s |
| Later `FileRequest` for sources answered at most; sources per `FileStatus` at most | 1 per 10 min; 50 |
| New connections accepted per minute: per node ID, per host address | 10, 30 |
| Open connections with peers at one host address / in one site, at most | 64 / 256 |
| Ban after exceeding the connection budget: node ID; host address (a further connection is refused instead) | 24 h; none |
| Simultaneous dial: the connection that lost is kept at least | 5 s |
| Source exchange freshness | 1 h |
| Anonymous paths: hops per direction, entry hops, kept for, paths outwards, gates | 0–3, 3, 30 days, 4, 4 |
| Anonymous paths, a hop: paths per neighbour, streams per path, further exchanges per path, held per stream and direction, base rate, lookups per path, dials per path / per neighbour | 8, 64, 65, 256 KiB, 16 KiB/s, 20 per minute, 4 / 16 per minute |
| Anonymous paths: `ANON` records per target (in total / per host / per site), `GATE` lifetime and renewal, session idle time, failed `GATE` lookups or gates dialled per file and hour | 64 / 4 / 8, 1 h and 45 min, 10 min, 4 |
| Anonymous paths: dials caused by `HopExtend`/`HopConnect` (relay dials and punches included) whose handshake did not complete within 10 s, or still pending, per destination host (IPv4 /32, IPv6 /64) and hour; per neighbour host address: paths, dials per minute, failed dials (wrong node ID, refused punch) per hour | 8; 16, 32, 8 |
| Anonymous paths: hops known before a path is built, reach a new hop or a `HopConnect` node within, status frame awaited (after `HopStream` or `HopJoin` / after `HopConnect`), `HopJoin` per gate and host, give and take measured over, relays per family in `HopConnect` | 20, 10 s, 10 s / 30 s, 10 per minute, 7 days, 2 |
| Anonymous paths: answer awaited by the last hop to its `HopCreate`; by the owner after `HopCreate` or `HopGate` / after `HopExtend`; a refused session not opened again to that node ID for | 10 s; 10 s / 30 s; 60 s |
| Anonymous paths: next message of a session handshake awaited; `HopLookup` answered by the last hop at the latest / awaited by the owner | 10 s; 60 s / 70 s |
| Anonymous paths: replay window for stream numbers of a path and of a session, PING on a path link every | 1,024, 20 s |
| Browse: entries per answer; pages and entries a requester takes from one node, at most | 1,000; 100 and 10,000 |
| Chat: message ids of a sender remembered for, at most; senders remembered at most, of them per host address; resends after a lost connection at most, within | 10 min, 1,024; 256, 16; 3, 10 min |
| Rooms: item at most (room info, `KeyDrop`); clock tolerance; moderators, admits, bans and lifts, presences per room; permanent bans; ban at most; lift kept for; ban window of a moderator kept after the ban (when no longer in `mods`), windows per key | 8 KiB (72 KiB); 1 h; 8, 1,024, 4,096, 4,096; 4,096; 30 days; 30 days; 30 days, 64 |
| Rooms: presence renewed every, `expires` at most after `issued`, `issued` remembered after `expires`; history (age, posts), `since` slack, dropped post ids kept; passed on per author and minute (posts, presences), per signer and minute (admits, bans, keys); taken per neighbour and minute | 5 min, 15 min, 1 h; 24 h, 1,000, 1 h, 1 h; 20, 1; 60; 600 |
| Rooms: neighbours aimed at / kept, rooms per node, bad neighbour refused for, item awaiting its room info (time, items per room); member proof clock tolerance; `SUB` target of a private room changes every, neighbour hours searched in the first / last | 3 / 2–4, 16, 24 h, 60 s, 64; 10 min; 1 h, 10 min / 10 min |
| Rooms, generations: started again after (with posts), post of an older one dropped after, `KeyDrop`s held, wraps per `KeyDrop`; post awaiting its `KeyDrop` (time, posts per room); a moderator starts a new generation once the current one has `KeyDrop`s; key forgotten after a later generation (no post held / at the latest) | 24 h, 2 min, 64, 1,032; 60 s, 64; 32; 1 h / 48 h |
| HTTP sources: per `FileStatus`, per source record, per Kad answer; results until a source is dropped; redirects followed; a sealed source's `expires` ahead at least | 8, 3, 16; 3; 3; 120 s |
| HTTP sources: live sealed sources before no further chunk is published; failed or pending fetches per hour, per host / site / block, counted per file and, over all files, per namer of the URL; over all files and namers together, per host; a host is exempt over all files after a verified block for | 2; 2 / 4 / 8; 8; 1 h |
| File sets, a manifest at most: size, `files`, `dirs`, path component, components in a path, depth, files with all inner sets | 4 MiB, 65,536, 65,536, 1–255 B, 32, 8, 1,000,000 |
| File sets: head `expires` at most, follower stores the head every, check a followed set at most / suggested | 7 days (+1 h), 24 h, 1 per hour / 1 per 6 h |
| File sets: `MEMBER` sets per file and publisher, sets per target (in all / per publisher key / per site), records kept per set, records per answer, site count saturates at | 4, 64 / 4 / 16, 3, 16, 255 |
| Hidden sets: head `expires` at most, follower stores within the first, sealed `data`, record dropped after its day | 7 days (+1 h), 6 h, 12–256 B, 48 h |
| Sets with several authors: grant keys, signatures required, mounts per manifest / over all levels of a set, issuers, changes, part (size, files, manifest), `PART` records per target (per host / per site) / per answer, parts read per set | 8, 4, 64 / 256, 8, 4,096, 64 MiB / 256 / 64 KiB, 1,024 (8 / 32) / 16, 1,024 |
| Live: `start` at most ahead, `end` kept, `have` update, sign of life, neighbour gone after, `peers` seen within, give up on a request after, a second exchange of a stream reset after | 1 h, 30 min, 1 per s, 10 s, 30 s, 10 min, 4 s plus 1 s per MiB asked for, 10 s |
| Live: join behind the newest, neighbours aimed at / kept, interrupted after, leave after, `LIVE` record stored every, `window`; a cut-off broadcaster continues its `seq` when it returns within; neighbours a broadcaster takes at least | 3 segments, 6 / 3–8, 60 s, 10 min, 5 min, 8 to 240 (0 or absent = 30); 10 min; 3 |
| Live, rating: outcomes kept, urgent within, probe after, steady (age, outcomes, rate), core (age, outcomes, rate), requests at a time new / steady / core | 20, 3 segments, 60 s, 2 min / 5 / 80 %, 5 min / 10 / 90 %, 1 / 2 / 4 |
| Live, rating: down (rate, outcomes), out (second down within), `LIVE_BLOCKED` for, core per site / block, sudden loss (core neighbours at least, share, within, hold), returning within | 50 % / 5, 10 min, 30 min, 1 / 2, 3 / more than 1/3 / 2 min / 2 min, 30 min |
| Live, private channel: look under the previous hour during the first, under the next hour during the last | 10 min, 10 min |
| Publish and subscribe: `peers` seen within, bad neighbour refused for, items passed on per topic, neighbours aimed at / kept, topics pushed, a second exchange of a topic reset after | 1 h, 24 h, 1 per 10 s, 3 / 2–4, 16, 10 s |
| Publish and subscribe: on-air notice `expires` after `issued` at most, sent every, highest `issued` remembered after `expires`; `SUB` record stored every | 15 min, 5 min, 1 h; 30 min |
| Local network: group IPv4 / IPv6, port, announce every, multicast announces per interface at most, announce datagram at most, reply wait up to, replies per node, replies per minute, LAN peer kept, LAN peers at most, new per minute | 239.255.101.51 / ff02::6533:6b, 6533, 5 min, 1 per minute, 512 B, 1 s, 1 per 5 min, 10, 15 min, 256, 10 |
| Local network: `LanQuery` per peer, once more on a new download, roots per query | 1 per 10 min, 1 per minute, 256 |
| Names: labels per name, label, name at most; zones followed, lookups per name; links per record, links of a record together at most; `NAME` `expires` ahead at most, stored again every; delegation from `issued` to `expires` at most, `issued` ahead at most | 8, 63 B, 253 B; 8, 24; 4, 512 B; 7 days (+1 h), 24 h; 90 days, 1 h |
| Names, global: label; zones per target (per key / per site / in all), records kept per zone, records per answer, site count saturates at, a claim stored again every; refused zones remembered per `CLAIM` target | 3–63 B; 2 / 8 / 64, 3, 16, 255, 20 h; 64 |
| Names, friends and lists: names per `NameQuery`, queries answered per peer, a pinned name asked of friends again; sets of lists followed, list file at most, entries per list, `note` at most | 16, 1 per 10 s, 1 per 24 h; 32, 4 MiB, 100,000, 256 B |
| Names from the web: TXT record at most, its query, links used per domain, `ed3k.json` at most, its fetch, kept at most | 1,024 B, 10 s, 4, 64 KiB, 10 s, 24 h |
| Websites: member awaited, cache by default; short slots per uploader, member at most, short slots per downloader and minute | 60 s, 2 GiB; 4, 256 KiB, 32 |
| Other names: records per root and scheme per publisher; roots per target (per key / per site / in all); records kept per root; records per answer; roots tried per name; `entry`, `password` at most; site count saturates at | 4; 2 / 8 / 64; 3; 16; 3; 255 B, 128 B; 255 |
| Torrents: `info_size` at most, sets tried per torrent | 32 MiB, 3 |
| Usenet: article list at most, sets tried per NZB, `nzb-file` records per post, `nzb-file` lookups per NZB | 32 MiB, 3, 256, 256 |
| Videos on a chain: files per upload set, subtitles, picture and playlist at most, link at most, blocks per log read, gateways per video | 64, 1 MiB, 512 B, 2,048, 3 |
| Videos on a chain, shown at most: title, tag, tags, comment; avatar picture at most | 256 B, 64 B, 16, 8,192 B; 1 MiB |
| Videos on a chain, attachments: `media` value at most, files per set, gateways per file; fetched unasked: manifest at most, picture at most, pictures per entry; picture refused above (side, pixels) | 512 B, 64, 3; 64 KiB, 1 MiB, 4; 16,384 px, 64 million |
| Filter lists: list file at most, sets of lists followed, list files per set, list files read in all | 4 MiB, 32, 64, 64 MiB |
| First contact: saved contacts, fetch lists when fewer than N answered / at most per first contact, list at most, redirects followed, `created` at most old, nodes per family, sources shipped / fetched, nodes taken per list, self-lookup after N answered (at most half from one list, for at most), listed nodes reachable for; nodes taken per list from one site / block; first LAN round over after the first announces | 200, 10 / 1 per 6 h, 32 KiB, 3, 30 days, 200, 4 / 2, 50, 4 (30 s), 24 h; 2 / 8; 2 s |
| Finding a known node: friends, open links, request stored every / for, request place searched every, records per target (per host / site), `sealed` **(wire)**, `issued` of a record or a request at most ahead, record stored every, lookup of a friend at most, sealed values opened per search at most | 64, 4, 4 h / 7 days, 30 min, 32 (8 / 16), 384 B, 1 h, 4 h, 1 per 30 min, 256 |
| Links at hosters: per `FileStatus`, per source record, per Kad answer, per member of a set; results until a link is dropped; streams per hoster and account | 8, 3, 16, 4; 3; 1 |
| Credits: tranche, priority slots at most, time for a payment (from the `FileStatus` with `price` or `due`), bid change, ledger read valid for | 4 MiB, 2/3, 30 s, 1 per 30 min, 10 min |
| Credits: void bids before priority is withheld (per node ID), withheld for | 2, 24 h |
| Credits: a bid ends a turn that has more left than, rate measured over; uploader's share of a payment | 60 s, 10 s; 10 % |
| Credits, ledger: grace, decay, floor, burned on transfer, settle within (no payment / with payment) | 3 days, 10 % per day, 10 credits, 10 %, 48 h / before the hold's `until` |
| Credits: receipts / holds per `CreditRelay`, `floor` at least, `tick`, gas reward (paid once per (issuer, holder, day) entry, to its first receipt that earns); open payments of a client at most | 32 / 8, 1,000,000 µ per MiB, 10,000 µ per MiB, 2 × the gas of a `settle` of the first receipt of a pair and day (accounts and weeks known), minter included; 80 % of the balance |
| Credits, ledger: receipt age at settlement, hold for a bid, `kib` per receipt, minted per issuer and 7 days, minted per issuer and holder and 7 days, maturity (age, contribution, issuers) | 7 days, 6 h, 50 GiB, 100 GiB, 10 GiB, 30 days / 1 GiB / 3 |
| Credits, hold: amount at least, `until` ahead at least, renewed every, deadline of a signed hold at most, stream kept open for a bidder's first receipt | 16 × `max_price` µ, 2 h, 3 h, 10 min, 30 s |
| Credits, ledger: a short payment sets `overdrawn` up to | 1 day after the receipt's `day` |
| Paid subscriptions: season length, horizon on online nodes, pass length at most (recommended), pass `issued` ahead at most, offer `issued` ahead at most, proof clock tolerance, span of sighting minutes in a share report, requests for a missing tip per scope | about 30 days, 7 days, 7 days, 1 h, 1 h, 10 min, 80 min, 1 per day |
| Paid subscriptions: next epoch counts as current in the last, key requests answered per holder key, `devices` when 0, proofs counted over, reports per holder key and scope | 10 % of an epoch (at most 10 min), 6 per hour, 3, 1 h, 1 per day |
| Paid subscriptions, seasons: chain steps per message at most, chain `base` ahead of the clock at most, a planned season's chain accepted before the epoch before its `base` at most; a node whose key failed not used for | 2^20, 1 h (plus 1 epoch), 1 h; 24 h |
| Paid subscriptions, purchase: poll `pass_url` every / for, renewal polled in the last / every | 15 s / 30 min, 48 h / 6 h |
| Flood protection: Retry token, return token valid for; per second: handshakes without token per host / site / block (Retry above), Retry packets per host (dropped above) | 10 s, 24 h; 4 / 16 / 64, 4 |
| Flood protection: validated handshakes per minute per host / site / block while guarded; Kad datagrams per second: `CHALLENGE`s per host, key derivations per host / site / block | 30 / 120 / 480; 4, 8 / 32 / 128 |
| Keyword levels (reserved on Kad, version 1 at an index node): records per entry, site count saturates at, candidate files per target, leaving entries per target | 3, 255, S/4, S/4 |
| Kad datagrams (reserved): `INIT` at least, cookie secret replaced every, request timer, `CHALLENGE`s per second per host, key derivations per second per host, `REQUEST`s per second per host / site / block (host, site and block of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1; a private address in a named network exempt), exchanges outstanding (total / per responder), `INIT` fallback to QUIC after | 70 B, 1 h, 1 s, 4, 8, 16 / 64 / 256, 16 / 2, 300 ms |
| Key puzzle (reserved): leading zero bits, memory, epoch, dialled-in contacts verified per host group (whatever the result), verification queue at most; another `puzzle` value of one node ID, family and site rechecked at most | 13, 64 MiB, 30 days, 1 per hour, 64; 1 per hour |

## Appendix B — open points

- Test vectors are in [`tests/vectors-v1.md`](tests/vectors-v1.md): node ID, routing ID, record
  and keyword targets with the edge cases of the tokeniser, one signed record with the
  signatures and forms §10.7 rejects, user hash,
  obfuscation keys and whole datagrams of each kind, the §12.4 tree, the key conversion of
  §6.4, and of the extensions the hop keys, the signatures of a set and its grant, one
  canonical torrent manifest, the sealed forms of a hidden set and of a friend record, the LAN
  announce, one live segment record, one exchange of Kad datagrams, the key puzzle, a Usenet
  post, the `canonical` of hoster links, the hash body, and of paid subscriptions the scopes,
  a season with its tip, an offer, a pass, a proof, a key exchange, the sealed forms and a
  share report, the node list tag, the keys of private channels, the hop session, the pair key
  of friend records, the names, proofs, keys and a sealed post of rooms, the targets, a delegation and a `NAME` and a `CLAIM` record of readable names, and the credit
  signatures of a bid and a receipt, under a test domain:
  the chain ID and the contract of the public network are not assigned yet
  ([`extensions/credit.md`](extensions/credit.md) §7.1).
- The zstd dictionary does not exist yet; it needs a message corpus. Until it is published
  every node announces `dict = 0` and sends frames without a dictionary (§5.3).
- The field numbers of every message are collected in `proto/ed3k.proto`; a change in a
  document has to be made there as well.
- Raw-public-key support has to be confirmed in the TLS library chosen by each implementation.
- The numbers in §10.5 and §15 are estimates.

## Appendix C — references

- [RFC 9000](https://www.rfc-editor.org/rfc/rfc9000),
  [RFC 9001](https://www.rfc-editor.org/rfc/rfc9001),
  [RFC 9002](https://www.rfc-editor.org/rfc/rfc9002) (QUIC);
  [RFC 7250](https://www.rfc-editor.org/rfc/rfc7250) (raw public keys);
  [RFC 8878](https://www.rfc-editor.org/rfc/rfc8878) (zstd);
  [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032) (Ed25519);
  [RFC 7748](https://www.rfc-editor.org/rfc/rfc7748) (X25519)
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) (requirement words);
  [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986) (URI syntax);
  [RFC 4648](https://www.rfc-editor.org/rfc/rfc4648) (base32);
  [RFC 4941](https://www.rfc-editor.org/rfc/rfc4941) (temporary IPv6 addresses)
- [BLAKE3 specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf) and
  the [Bao verified-streaming specification](https://github.com/oconnor663/bao/blob/master/docs/spec.md)
- [BEP 42](https://www.bittorrent.org/beps/bep_0042.html), *DHT Security Extension*;
  [BEP 32](https://www.bittorrent.org/beps/bep_0032.html), *IPv6 extension for DHT*;
  [BEP 44](https://www.bittorrent.org/beps/bep_0044.html), *Storing arbitrary data in the DHT*;
  [BEP 50](https://www.bittorrent.org/beps/bep_0050.html), *Publish/Subscribe Protocol*;
  [BEP 14](https://www.bittorrent.org/beps/bep_0014.html), *Local Service Discovery*.
  The BEPs as a whole are compared in [`docs/bittorrent-comparison.md`](docs/bittorrent-comparison.md)
- [I2P](https://geti2p.net/) and [ZeroNet](https://github.com/HelloZeroNet/ZeroNet); compared in
  [`docs/i2p-zeronet-comparison.md`](docs/i2p-zeronet-comparison.md).
- [RFC 8439](https://www.rfc-editor.org/rfc/rfc8439) (ChaCha20 and ChaCha20-Poly1305), used by
  private channels, hidden sets and friend records, and by paths, paid subscriptions and Kad
  datagrams
- [IPFS specifications](https://specs.ipfs.tech/), the
  [Kubo configuration reference](https://github.com/ipfs/kubo/blob/master/docs/config.md) and the
  [Hyphanet wiki](https://github.com/hyphanet/wiki/wiki) (Freenet); compared in
  [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md)
- GNUnet: [the GNU Name System](https://lsd.gnunet.org/lsd0001/) and
  [the R5N distributed hash table](https://lsd.gnunet.org/lsd0004/);
  [RetroShare](https://retroshare.cc/); compared in
  [`docs/gnunet-retroshare-comparison.md`](docs/gnunet-retroshare-comparison.md)
- The [Tox protocol specification](https://toktok.ltd/spec.html) and Briar's
  [rendezvous protocol](https://code.briarproject.org/briar/briar-spec/-/blob/master/protocols/BRP.md);
  compared in [`docs/tox-briar-comparison.md`](docs/tox-briar-comparison.md)
- The [Scuttlebutt protocol guide](https://ssbc.github.io/scuttlebutt-protocol-guide/) and the
  [Nostr NIPs](https://github.com/nostr-protocol/nips); compared in
  [`docs/scuttlebutt-nostr-comparison.md`](docs/scuttlebutt-nostr-comparison.md)
- The [Hypercore documentation](https://docs.pears.com/building-blocks/hypercore) and Syncthing's
  [Block Exchange Protocol](https://docs.syncthing.net/specs/bep-v1.html); compared in
  [`docs/hypercore-syncthing-comparison.md`](docs/hypercore-syncthing-comparison.md)
- [Node Discovery Protocol v5](https://github.com/ethereum/devp2p/blob/master/discv5/discv5.md)
  (discv5); compared in [`docs/discv5-comparison.md`](docs/discv5-comparison.md)
- Baumgart, Mies:
  [*S/Kademlia: A Practicable Approach Towards Secure Key-Based Routing*](https://doi.org/10.1109/ICPADS.2007.4447808)
  (2007)
- libp2p specifications: [DCUtR](https://github.com/libp2p/specs/blob/master/relay/DCUtR.md),
  [AutoNAT v2](https://github.com/libp2p/specs/blob/master/autonat/autonat-v2.md),
  [Circuit Relay v2](https://github.com/libp2p/specs/blob/master/relay/circuit-v2.md);
  Trautwein et al., measurement of DCUtR hole punching
  ([arXiv 2510.27500](https://arxiv.org/abs/2510.27500))
- [draft-ietf-quic-address-discovery](https://datatracker.ietf.org/doc/draft-ietf-quic-address-discovery/);
  [draft-seemann-quic-nat-traversal](https://datatracker.ietf.org/doc/draft-seemann-quic-nat-traversal/)
- [*Exposing and Circumventing SNI-based QUIC Censorship of the Great Firewall of China*](https://gfw.report/publications/usenixsecurity25/en/)
  (USENIX Security 2025);
  [*How the Great Firewall of China Detects and Blocks Fully Encrypted Traffic*](https://gfw.report/publications/usenixsecurity23/en/)
  (USENIX Security 2023)
- [Hysteria 2 protocol specification](https://v2.hysteria.network/docs/developers/Protocol/)
  (Salamander, Gecko)
