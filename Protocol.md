# ed3k Protocol Specification

**Status:** draft 0.2 (2026-10-13). Nothing here is implemented yet.
**Wire version:** 1 (ALPN `ed3k/1`).

ed3k: P2P file sharing in the tradition of eD2K and Kad. Keeps their model (hash-identified
files, Kademlia DHT, optional index servers, upload queues with credits); replaces wire format,
transport and identity:

- every message is **protobuf**; exactly one message per purpose
- transport **UDP only**; mandatory binding **obfuscated QUIC**
- **NAT traversal is a required feature**, not an add-on
- **IPv4 and IPv6** both first-class, one overlay
- identity is an **Ed25519 key**; its hash is the node ID in Kad and everywhere else
- files identified by a **BLAKE3** root hash

**ed3k does not require eD2K.** A compliant implementation needs no eD2K, eMule-Kad or TCP code.
Reference implementation eMuleQt speaks both; §14 specifies how such a dual-protocol client tells
eD2K peers it also speaks ed3k. Apart from a file's optional `ed2k` alias (§7.4), §14 is the only
eD2K part, and it is OPTIONAL.

---

## 0. Conventions

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, RECOMMENDED, MAY and OPTIONAL are to be
read as in RFC 2119.

- Protobuf is **proto3**. Fixed-layout `bytes` fields state it in a comment; integers in such
  layouts, and in inputs of `H` and of signatures, are **big-endian** (network order).
- `H(ctx, data)`: BLAKE3 `derive_key` mode, context string `ctx`, 32-byte output unless a length
  is given; a longer output is the extended output of the same `derive_key` call, so its first
  32 bytes are the 32-byte value. `H(ctx, data)[a..b)` is a byte slice.
- `varint` is the protobuf base-128 varint.
- Integer fields are `uint32` or `uint64`; both varint on the wire, type states the range a
  reader accepts. A field holding a random 64-bit value is `fixed64`
  ([`docs/wire-format-comparison.md`](docs/wire-format-comparison.md) section 2.4).
- Numbers marked *(initial)* are tuning values, collected in Appendix A; may change w/o a wire
  version bump. Appendix A also lists (and marks) a few values fixed by the wire format; those
  change only with the wire version.
- A field declared `optional` has explicit presence: a reader tells an absent field from one set to its default.
  Where a document gives an absent optional field a meaning, it is "unchanged" or "not told". A
  non-`optional` field that is absent reads as its default (0, empty), as in proto3, and MUST
  NOT be given a meaning differing from its default's.
- **Canonical serialisation.** Where a document requires it: fields in ascending field number,
  each once, repeated fields in order, every varint shortest, default-valued fields left out, no
  unknown fields. Elsewhere not canonical; a reader MUST accept any valid encoding.
- A fixed-layout `bytes` field of a length the layout does not allow makes its message malformed
  (`Error{MALFORMED}`; `REJECTED` for a record), unless its document names another outcome. A
  field that may be absent is not a wrong length when empty.
- Protobuf fragments here and in extensions are excerpts. [`proto/ed3k.proto`](proto/ed3k.proto)
  is the normative definition of every message, field and enum of wire version 1, extensions
  included.

---

## 1. Design decisions

Decisions taken while drafting; each links to the section with reasoning and rejected
alternatives.

| # | Decision | Section |
|---|---|---|
| D1 | Messages specified against an abstract transport contract; obfuscated QUIC is the one binding every peer MUST implement | §3, §4 |
| D2 | One overlay for IPv4 and IPv6; records carry one endpoint per family | §8, §10.2 |
| D3 | Node ID: 128 bits, truncated hash of the public key, in Kad and outside | §6.1 |
| D4 | Routing position IP-bound (BEP 42 style) in top 21 bits, key-bound in the rest | §6.3 |
| D5 | Dual-protocol client listens for ed3k on the UDP port numbered as its eD2K TCP port | §14.1 |
| D6 | eD2K peers learn of ed3k support from a user hash derived from the node ID | §14.2 |
| D7 | zstd decompression required; senders compress only when it helps, messages and file blocks alike | §5.3, §12.3 |
| D8 | Files identified by BLAKE3 root; eD2K hash is an alias, droppable later | §7 |
| D9 | Kad runs over QUIC in version 1; sessionless datagram binding reserved | §10.6 |
| D10 | Relays forward rendezvous signalling only, never file data | §9.4 |
| D11 | A server is an ordinary ed3k node with INDEX capability; no separate server message set | §11 |
| D12 | Keyword and note records signed by publisher; source records not, proven by dialling | §10.3, §10.7 |
| D13 | Every signed object carries a list of signatures, so algorithms can be added | §10.7 |
| D14 | Keyword target includes a level, store answered per record, so crowded keywords can later spill to further levels w/o wire change | §10.1, §10.8 |
| D15 | One Kad request may store source records for many targets: publishing costs lookups by network size, not file count | §10.3, §10.4 |
| D16 | A non-public network has a name entering the obfuscation key; nodes of two networks cannot read each other's datagrams | §4.3 |

---

## 2. Roles and capabilities

Every participant is a **node**, announcing what it does with capability bits in `Hello` (§5.5)
(Ext. = optional extension):

| Bit | Name | Meaning |
|---|---|---|
| 0 | `ROUTER` | Reachable w/o prior signalling on ≥ 1 address; may be placed in routing tables; stores Kad records |
| 1 | `RELAY` | Forwards rendezvous signalling for firewalled nodes (§9.4). A `ROUTER` node MUST set it unless out of relay slots |
| 2 | `INDEX` | Index node ("server"): accepts and answers records for every target (§11) |
| 3 | `BROWSE` | Ext.: answers `Browse` ([`extensions/browse.md`](extensions/browse.md)) |
| 4 | `CHAT` | Ext.: accepts `Chat` ([`extensions/chat.md`](extensions/chat.md)) |
| 5 | `KAD_DGRAM` | Reserved for sessionless Kad binding (§10.6). MUST be 0 in version 1 |
| 6 | `ED2K` | Also speaks eD2K (§14). Informational |
| 7 | `PUZZLE` | Reserved for key puzzle (§6.3). MUST be 0 in version 1 |
| 8 | `LEVELS` | Reserved for keyword levels and ranking on Kad (§10.8). MUST be 0 in version 1; levels at an index node need no bit (§11) |
| 9 | `HTTP` | Ext.: understands HTTP sources ([`extensions/http-source.md`](extensions/http-source.md)) |
| 10 | `SETS` | Ext.: understands file sets ([`extensions/file-set.md`](extensions/file-set.md)) |
| 11 | `LIVE` | Ext.: takes part in live streams ([`extensions/live.md`](extensions/live.md)) |
| 12 | `PUBSUB` | Ext.: passes on new records of things it follows ([`extensions/pubsub.md`](extensions/pubsub.md)) |
| 13 | `LAN` | Ext.: answers nodes of its local network ([`extensions/lan-discovery.md`](extensions/lan-discovery.md)) |
| 14 | `ANON` | Ext.: accepts sessions arriving through a path of other nodes ([`extensions/hops.md`](extensions/hops.md)) |
| 15 | `HOP` | Ext.: forwards for others' paths ([`extensions/hops.md`](extensions/hops.md) §5) |
| 16 | `CREDIT` | Ext.: signs receipts for what it downloads, understands bids for upload slots ([`extensions/credit.md`](extensions/credit.md)) |
| 17 | `LINKS` | Ext.: understands file hoster links as sources ([`extensions/hoster.md`](extensions/hoster.md)) |
| 18 | `PAID` | Ext.: hands out epoch keys of paid channels and sets to pass holders ([`extensions/subscription.md`](extensions/subscription.md)) |
| 19 | `ROOM` | Ext.: takes part in rooms with many members ([`extensions/room.md`](extensions/room.md)) |
| 20 | `NAMES` | Ext.: answers friends asking what a name stands for ([`extensions/name.md`](extensions/name.md)) |

`caps` travels in `Hello` only (§5.5), not in compact Kad contacts or source records: a node
learns a peer's capabilities after connecting. A bit or policy changing during a connection
(e.g. `RELAY` when relay slots fill, §9.4) is not announced on it; the next request's answer
signals it: `Error{BUSY}` full resource, `Error{DENIED}` refusal, `Error{UNSUPPORTED}`
capability turned off. A bit turned on shows on the next connection.

A node sets `ROUTER` while `open` in ≥ 1 family (§9.2), also when operator-declared open there,
and clears it on new connections when no longer so. `ROUTER` alone does not say which family;
`Hello.nat` does (§10.2). Two nodes MAY leave `ROUTER` unset while open: an index node (§11) and
a second node for friends ([`extensions/friend.md`](extensions/friend.md) §5).

A non-`ROUTER` node is a **leaf**: uses Kad as client, never put in a routing table, reached via
relays and hole punching.

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

Messages never refer to QUIC; they rely on a **transport contract** every binding has to meet:

| | Property |
|---|---|
| T1 | **Mutual authentication.** Each side learns the other's Ed25519 public key with proof of possession |
| T2 | **Confidentiality and integrity** of everything above the binding |
| T3 | **Validated remote address.** Peer's source address proven by a round trip before more than a bounded reply is sent to it |
| T4 | **Exchanges.** Either side can open any number of independent, ordered, reliable, flow-controlled byte channels at negligible cost. An exchange has an initiator; each side can half-close it |
| T5 | **Unlinkability to a protocol.** A passive observer not knowing the peer's node ID cannot classify traffic by fixed bytes, fixed sizes or a decryptable handshake. A binding whose transport lacks this MUST add obfuscation; one whose transport already hides itself MUST NOT add a second layer |

Unreliable datagrams are not in the version 1 contract (see §10.6).

**Why not gRPC or gRPC-Web over UDP.** Usual way to expose protobuf services, so considered.
gRPC assumes HTTP: per call pseudo-headers, 5-byte message prefix, trailers block; over UDP it
needs HTTP/3 with QPACK; as of this draft neither grpc-core nor QtGrpc ships an HTTP/3 channel. Calls here are
symmetric (either peer may ask), long-lived (file stream), carry raw file blocks: streams fit
better than RPC. HTTP/3 is also one more recognisable layer. §5 framing gives gRPC's one useful
property, typed request and answer on its own stream, for 2 bytes.

**Other bindings.** WebTransport, TLS-over-TCP with a stream multiplexer, or an in-memory test
binding can meet T1–T5; none specified. A future binding gets its own name and ALPN-like
identifier and changes no message. Version 1 cannot advertise a second binding; gap and path to
close it: [`docs/transport-bindings.md`](docs/transport-bindings.md).

---

## 4. The `quic-obfs` binding

### 4.1 QUIC profile

- QUIC version 1 (RFC 9000/9001/9002) with TLS 1.3. ALPN `ed3k/1`. No SNI.
- **Mutual authentication with raw public keys** (RFC 7250), Ed25519 only. No X.509: a
  certificate adds several hundred bytes per handshake and carries nothing the key does not.
  Both sides MUST send their key; handshake w/o client key is closed with `NO_KEY`. A server MAY
  fail such a handshake inside TLS instead (`certificate_required`); client treats both alike. Same for a
  non-Ed25519 key.
- Initiator knows whom it dials; after the handshake it MUST compute the peer's node ID (§6.1)
  and compare with the expected one; mismatch → close with `WRONG_PEER`. The expected ID's first
  12 bytes key the datagrams (§4.3): a node differing there cannot unmask them; dialler sees a
  timeout.
- A side closes with `NO_KEY` or `WRONG_PEER` only after its last handshake flight is out: an
  application close in an Initial or Handshake packet loses its code (RFC 9000 §10.2.3).
- An **exchange** (T4) is one bidirectional QUIC stream. Unidirectional streams unused.
- Connection migration SHOULD be enabled: session survives a client-side address change. Server
  side does not migrate (RFC 9000 §9): connection whose server changed address is lost and
  reopened, by a new hole punch where needed (§9.5). A connection does not move between
  IPv4 and IPv6 (§8.3).
- 0-RTT MAY be used, and only for `Hello` and `KadRequest` without a `store` (replay-safe). A
  server MUST NOT act on any other message arriving in 0-RTT before handshake completion; it
  holds the request until then.
  A resumed handshake carries no key. Both sides MUST keep the peer's key with the session
  ticket and take the node ID from it. A server unable to recover the client's key from a ticket
  MUST refuse resumption and do a full handshake, also rejecting the 0-RTT data. A client MUST
  use a ticket only towards the node ID whose key it was issued under.
- Idle timeout: both sides send `max_idle_timeout` = 30 s *(initial)*. A side wanting the
  connection kept sends a QUIC PING every 20 s *(initial)*: for a file stream open or queued
  (§12.5), a relay reservation (§9.4), and where an extension says so. Otherwise it sends none
  and lets the timeout run, or closes with `CLOSED`. Towards an index node both values are wider
  (§11).
- A node allows its peer at most 64 *(initial)* peer-opened open exchanges per connection
  (`initial_max_streams_bidi`, raised as exchanges close). Limit is per direction; file streams and
  extension exchanges count. The two ends of a path link allow each other 65 more per path
  between them ([`extensions/hops.md`](extensions/hops.md) §5). A responder resets (§5.2) an
  exchange whose request has not arrived whole within 30 s *(initial)* of opening; a file
  stream, once its first `FileRequest` arrived, follows §12.5 instead.
- Congestion control is the sender's choice. BBR is RECOMMENDED.
- A node MUST accept a Retry packet (RFC 9000 §8.1.2) and repeat its Initial with the token. A
  node MAY answer an Initial with Retry at any time
  ([`extensions/flood-protection.md`](extensions/flood-protection.md) §2).

**Close codes.** An application close (`CONNECTION_CLOSE` with an application error code,
RFC 9000 §10.2) uses one of:

| Code | Name | When |
|---|---|---|
| 0 | `CLOSED` | normal end: idle, or node shuts down |
| 1 | `NO_KEY` | handshake carried no client key |
| 2 | `WRONG_PEER` | peer's node ID not the expected one (§6.1, §14.2) |
| 3 | `DENIED` | peer banned (§4.6) or refused by a network's policy |
| 4 | `PROTOCOL` | first frame on initiator's first stream not `Hello` or malformed (after the `Error`, §5.5); or answer on that stream not a `Hello` or malformed; or no peer `Hello` on that stream within 30 s (§5.5); or a frame header (`tag`, `len`) did not parse, incl. `len` above 1 MiB, so the exchange cannot be read on (§5.1). A body that does not parse, or exceeds 1 MiB after decompression, is answered `Error{MALFORMED}` on its exchange instead (§5.2, §5.3, §5.6) |
| 5 | `BUSY` | node out of connections, or peer's host address over its budget (§4.6) |

A code not in this table reads as `PROTOCOL`.

### 4.2 Why obfuscation is required

QUIC encrypts payload, not shape: the Initial's key derives from a public salt (any observer can
decrypt it), it is padded to 1,200 bytes, short-header packets repeat a connection ID. The Great
Firewall has decrypted and filtered QUIC Initials since April 2024 and blocks the flow ~180 s on a
match. Random-byte UDP datagrams don't trigger its fully-encrypted-traffic heuristic (TCP only).
So the layer's two jobs: remove every fixed byte, blur the 1,200-byte handshake size.

Obfuscation, not secrecy: the key derives from the receiver's node ID, public to anyone in the
network. Confidentiality comes from TLS. Unmasking only demultiplexes: that a datagram unmasks
proves nothing about its sender. Evidence about a peer, e.g. reachability (§9), rests on what a
datagram carries and on the transport (T1, T3), never on the mask.

### 4.3 Datagram format

Every UDP datagram of this binding is:

```
salt(8) ‖ mask( magic(4) ‖ kp(1) ‖ pad(pad_len) ‖ body )
```

| Field | Meaning |
|---|---|
| `salt` | 8 random bytes in clear, fresh per datagram (resent datagram re-masked under new salt). `salt[0]` MUST NOT be `0xE3`, `0xE4`, `0xE5`, `0xC5`, `0xD4`, `0xA3` or `0xB2` (re-roll): eD2K/eMule UDP plaintext headers, so a legacy node at a stale address won't parse a stray ed3k datagram as one. Receiver does not check it |
| `magic` | `0x65 0x33 0x6B 0x01` (`"e3k"`, version 1). Checked after unmasking |
| `kp` | bits 7-6 `kind`, bits 5-0 `pad_len` (0–63) |
| `pad` | `pad_len` random bytes |
| `body` | depends on `kind` |

| `kind` | Name | `body` |
|---|---|---|
| 0 | `DATA` | one QUIC datagram holding one short-header packet; such a packet has no length field and is always last in its datagram (RFC 9000 §12.2), so nothing follows |
| 1 | `HANDSHAKE` | one QUIC datagram containing a long-header packet, plus whatever QUIC coalesced after it |
| 2 | `FRAGMENT` | `msg_id(1) ‖ frag(1) ‖ piece`. `piece`: slice of the QUIC datagram a `HANDSHAKE` body would hold (§4.4). `frag`: `idx` in bits 7-4, pieces counted from 0, `total` in bits 3-0; sender uses 2 to 4 pieces, receiver accepts 2 to 15 |
| 3 | `PROBE` | `nonce(8)`; for hole punching and reachability tests (§9). A `PROBE` with a body not 8 bytes is dropped |

Receiver takes `kind` as given; it decides how much is masked (below). Dropped: a `DATA` datagram
whose QUIC packet has a long header; a `HANDSHAKE` whose body starts with a short-header packet.

**Key and mask.**

```
obf_id   = NodeID[0..12)                    of the node that ACCEPTED the connection
net_id   = H("ed3k v1 net", name)[0..16)                   empty in the public network
key      = H("ed3k v1 obfs", net_id ‖ obf_id ‖ dir ‖ salt) dir = 0x00 to that node, 0x01 from it
stream   = BLAKE3 keyed XOF (key, empty message), as many bytes as masked
mask(x)  = x XOR stream
```

`obf_id` is 12 bytes, not 16, so it can be learned from an eD2K user hash (§14.2). A `PROBE` is
always keyed to its receiver with `dir = 0x00`. Keystream: extendable output of BLAKE3 keyed
mode, `key` as 32-byte key, empty message. The seven excluded `salt[0]` values are a statistical
fingerprint, weak but real; the list is as short as port sharing with eD2K allows (§14.1), and a
later `magic` can change it.

**Networks.** Public network has no name; `net_id` empty there. Any other network has a `name`
of 1 to 32 bytes configured on all its nodes.

- A datagram from another network's node does not unmask to the `magic`; dropped like any stray
  datagram (§4.5). The two networks never exchange a message, contact or record.
- A **test network** uses a published name.
- A **closed network** uses 32 random bytes only members know. Whoever has the name is a
  member; removal only by a new name. Its `net_id` opens the network as the name does. A client
  MUST NOT write such a name, or its `net_id`, to a log or show it in an error message.
- A node is in one network per UDP port. Node lists and their sources
  ([`extensions/bootstrap.md`](extensions/bootstrap.md)) belong to one network; a list names it
  by `H("ed3k v1 nodes net", net_id)[0..16)`, never by `net_id` or name.
- Reasoning: [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §2.2.

`HANDSHAKE`, `FRAGMENT`, `PROBE`: whole datagram after the salt is masked. `DATA`: only header,
padding and first 64 bytes of the QUIC packet (all when shorter), covering the QUIC flags byte
and connection ID; the rest is AEAD ciphertext already. Keeps bulk-path CPU cost at a few BLAKE3
compressions per packet (key derivation, ~130 bytes of keystream). Enough because a `DATA`
datagram holds one packet (above); a multi-packet datagram is a `HANDSHAKE`, masked whole.

**Overhead**: 13 bytes per datagram plus padding, ~1 % of a full data packet; `FRAGMENT` 2 more,
15 plus padding.

### 4.4 Size shaping

- A sender SHOULD split each `HANDSHAKE` datagram into 2–4 `FRAGMENT`s of random sizes, each
  randomly padded, and MAY send it whole with random padding instead. Receivers MUST support
  both.
- Layer adds 13 bytes plus padding: `13 + pad_len + QUIC datagram` MUST NOT exceed the UDP
  payload the path carries. So an unsplit `HANDSHAKE` carrying an Initial (1,200 bytes) uses
  `pad_len` of at most 19 while path MTU is unknown (1,232 − 13 − 1,200).
- **Fragments.** A message's pieces, joined in `idx` order, are one QUIC datagram; receiver hands
  it to QUIC as if it arrived in a `HANDSHAKE` from that source address.
  - Sender picks `msg_id` at random per message, no reuse towards one address within 3 s
    *(initial)*. A datagram QUIC resends is a new message.
  - Dropped: `FRAGMENT` with `total` below 2, or `idx` not below `total`. First arriving piece
    fixes `total`; a piece of that message with another `total` is dropped. Of two pieces with
    one `idx` the first counts.
  - Messages kept per (source address and port, `msg_id`). Receiver keeps ≤ 4 *(initial)*
    incomplete messages per source address and port, drops one after 3 s *(initial)*; when a
    fifth starts, oldest dropped. A message whose pieces total more than 1,500 bytes
    *(initial)* is dropped. All incomplete messages together hold ≤ 4 MiB *(initial)* and
    number ≤ 65,536 *(initial)*; beyond either, oldest dropped. Also bounds messages of empty
    pieces.
  - Anyone knowing both addresses can fill the 4 places with forged pieces, so a sender whose
    handshake got no answer after 2 *(initial)* fragmented attempts sends the next unsplit.
    Attempts counted per connection and long-header packet type, never reset: 3rd flight of a
    type and all later go unsplit. An arriving datagram is not an answer (a path dropping
    fragments can still carry other packets).
- `DATA` datagrams SHOULD carry `pad_len = 0` once the packet exceeds 256 bytes; smaller ones
  SHOULD be padded randomly. A `DATA` datagram whose QUIC datagram is at maximum size MUST carry
  `pad_len = 0`.
- **Anti-amplification.** Reassembled datagram never exceeds what arrived on the wire, so
  QUIC's received-bytes count stays conservative; its sent-bytes count doesn't (this layer adds
  header and padding to every datagram). Until the peer's address is validated, bytes a node
  puts on the wire towards it, this layer's header and padding included, MUST NOT exceed 3× the
  bytes that arrived from it on the wire (RFC 9000 §8.1); this holds again for a new client address after
  migration until QUIC validated it (RFC 9000 §9.3). A receiver MUST NOT answer a `PROBE` and
  MUST NOT emit anything for an incomplete fragment set. A punch initiator's dial to a `PROBE`'s
  source (§9.5 step 4) is not an answer: it needs the nonce of the initiator's own punch. With
  these bytes counted, a server whose 1,200-byte flight is lost twice may lack budget for a
  third (its padding and fragment overhead exceeding the client's); it resends on the client's
  next Initial.
- QUIC maximum datagram size MUST be set 13 bytes below the path limit. QUIC starts at 1,200
  bytes (Initials are 1,200 bytes), raising it only by path MTU discovery. Unsplit unpadded
  Initial = 1,213 bytes, fitting the IPv6 minimum MTU (1,232 bytes UDP payload); padded fits up to
  `pad_len = 19`.
- Binding needs a path carrying 1,213 bytes of UDP payload. A fragmented handshake never puts
  such a datagram on the path, so QUIC's padded-Initial test (RFC 9000 §14.1) is missing. Each
  side checks the path once per connection, after the handshake (handshake datagrams don't
  count). The check passes when any datagram ≥ 1,213 bytes on the wire, sent by the side after the
  handshake, is acknowledged, whatever it carries. Until then: send a PING padded to ≥ 1,213
  bytes and ≤ path limit; hold back file data (an exchange's bytes from its first `Block` on,
  §12.3), not other frames. 3 *(initial)* such probes lost → close with `CLOSED`. A side unable
  to send such a PING (its QUIC maximum, or peer's `max_udp_payload_size`, is 1,200 bytes, so no
  probe exceeds its datagrams) holds back nothing; its full file-data datagrams are the probes,
  passing as above; QUIC loss detection and idle timeout end a path dropping them. Check exists
  for file data: a side sending no file data on the connection MAY skip it, starting it before
  its first `Block` there. Lost probes count from the first sent on the connection, also if the
  check started before it had to; nothing resets the count. A path MTU probe above the path
  limit is no such probe. Index node sends none. Check not repeated after migration: QUIC path
  validation expands its datagrams to 1,200 bytes (RFC 9000 §8.2.1), 1,213 on the wire.

Follows Hysteria 2's Salamander (salted XOR mask) and Gecko (fragmenting long-header packets),
but real keystream instead of a repeating 32-byte pad, no pre-shared password. Censors adapt;
`magic` versions the datagram format so shaping can change w/o touching QUIC or messages.

### 4.5 Receiving

Steps tried in order; a datagram failing one goes to the next. Each step unmasks only the first
5 bytes and checks `magic`.

1. Source address and port belong to a live connection or dial in progress → unmask with that
   connection's key: `obf_id` and `dir` the peer uses towards this node (salt enters the key per
   datagram). Two connections can share source address and port a while after both nodes
   dialled at once (§4.6); different keys, each tried. On match, hand to that connection, unless
   `kind` is `PROBE`: it falls through to step 2 whatever key unmasked it (kind byte is among the
   5 unmasked bytes; free).
2. Else unmask with own `obf_id` and `dir = 0x00`. On match, process as new flow or `PROBE`. A
   `PROBE` is always keyed so, also from an address with a connection (hence step 1 passes it
   on). A packet of an existing connection whose client side changed address lands here too;
   QUIC's connection ID migrates it, within its family: one from an address of the other family
   is dropped (§8.3).
3. Otherwise the datagram is not ed3k. Drop it, or hand it to another protocol sharing the socket
   (§14.1).

A random datagram passes step 2 with probability 2⁻³² (2⁻³¹ with the Kad datagram binding,
[`extensions/kad-datagram.md`](extensions/kad-datagram.md)).

A datagram from a connection's server arriving from an address unknown to the client matches no
step and is dropped, as QUIC would (RFC 9000 §9): a connection's server side, also the NATed side
of a punched one, cannot change address.

A QUIC stateless reset (RFC 9000 §10.3) works towards a connection's client only: a server that
lost state masks it under own `obf_id`, `dir = 0x01`; client unmasks it in step 1. A client that
lost state has no key for the server's datagrams, sends nothing; server waits out its idle
timeout. An implementation MUST NOT rely on a reset reaching a server.

### 4.6 Connection budget

A node MUST limit the new connections it accepts from one peer.

| Counted per | New connections per minute *(initial)* | When exceeded |
|---|---|---|
| node ID | 10 | node ID banned 24 h *(initial)* |
| host address (IPv4 /32, IPv6 /64) | 30 | further new connections refused; nothing banned |

- **Only completed handshakes count.** Address then validated, key proven (T1, T3), so a forged
  source address cannot get another node banned. A resumed connection counts as new. Nothing
  counted, nobody banned, on packets before that point.
- During a ban, the node closes a handshake of that node ID once the key is known. Open
  connections of a banned node ID are closed.
- **Host address never banned for its budget.** Several nodes share one address behind a
  carrier-grade NAT or in one /64, and a ban would lock all out for what one did. Connection
  beyond the address budget: closed with `BUSY` once its handshake completes; ones already open
  from that address stay; address admitted again as the minute moves on. For the same reason the address
  budget is wider than the node ID one. A node with `INDEX` capability MAY use a wider address
  budget.
- **Open connections.** A node keeps ≤ 64 *(initial)* open connections with peers at one host address and
  ≤ 256 *(initial)* with peers in one site
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1). A further connection from
  there is closed with `BUSY` once its handshake completes. Both directions count, but only
  accepted ones are closed: dialling beyond the limits is local policy. Such a connection still
  counts in the rows above. Private addresses exempt where exempt from the host-address row
  (below). A node with `INDEX` capability MAY use wider limits.
- An address ban, caused only by bad blocks (§12.3), drops that address's datagrams that would
  start a new flow while it lasts.
- Private addresses (§8.2 (8)) are exempt from the host-address row and address bans (§12.3)
  where acceptable: in a named network (§4.3) and for a peer found in the local network
  (§8.2 (8)). The node-ID row applies to them.
- At most one connection per peer and address family. To stay within others' budgets a node
  MUST NOT open a second connection to a peer in a family where it already has one with it, and
  SHOULD keep a connection it will reuse open for the idle timeout. One per family is allowed:
  reach tests (§9.3), verifying a contact in each family (§6.3) and bootstrap (§10.3) need one
  per family.
- When two nodes dial each other at once in one family, two connections complete. Both keep the
  one whose client (its opener) has the lower node ID compared as bytes, open nothing new on the
  other, and close it with `CLOSED` once no exchange is open on it and ≥ 5 s *(initial)* passed
  since it lost: exchanges the peer opened on it before it knew can still be on their way.

A node that sent bad blocks is banned likewise, for other times, and so is its host address
(§12.3).

Protection against floods of packets never completing a handshake is optional and local:
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

Only `Block` (§12.3), `HopStream` ([`extensions/hops.md`](extensions/hops.md) §2.4), `HopConnect`
and `HopJoin` (§3.1 there) are followed by raw bytes outside the frame. After each of the three
hop messages, the first thing in the other direction is one status frame, `HopOk` or `Error`,
before any raw byte. An extension adding such a message says so here; a node not supporting it
answers `Error{UNSUPPORTED}` and closes the exchange (§5.2), so following bytes are never read
as frames.

A frame header does not parse when `tag` or `len` is a varint > 10 bytes, `len` is above 1 MiB
(§5.2), or the exchange ends inside it. An exchange ending, or reset, before a frame's first
byte holds no header: it failed (§5.2); connection stays open. A varint longer
than its value needs is accepted (§0).

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

- **Request/answer:** initiator opens an exchange, sends one request, half-closes. Responder
  sends one answer (or `Error`) and closes. No request IDs; the exchange is the correlation.
- **File stream:** one exchange per (downloader, uploader), open while blocks are transferred or
  while the uploader asks the downloader to stay (§12).
- An answer is never fragmented: the exchange is reliable, so a large answer is just a long
  frame. A frame body MUST NOT exceed 1 MiB after decompression.
- A `len` above 1 MiB is an unparseable header: connection closed with `PROTOCOL` (§4.1,
  §5.1). Once the peer half-closed, a receiver also checks before reading a body that `len` fits
  the remaining bytes; failing frame → `Error{MALFORMED}` w/o reading its body, exchange closed.
- A responder not knowing a request type answers `Error{UNSUPPORTED}` and closes that exchange
  only.
- **Reset.** A side giving up an exchange w/o answer resets it: `RESET_STREAM` (sending part),
  `STOP_SENDING` (receiving part), both application code 0. No other stream code; receiver reads
  any code as 0. An initiator unable to read an answer resets the exchange (it half-closed, can't
  send an `Error`). A reset exchange has failed; no answer to expect on it.
- On a request/answer exchange, what the initiator sends after its request is ignored.

### 5.3 Compression

Every node MUST be able to **decompress** zstd (RFC 8878) frames in the profile below. A sender
compresses a body only when the result is smaller, and SHOULD NOT try for bodies under 64 bytes
*(initial)*. File data is not part of a frame body; it follows a `Block` frame with its own rule
and profile (§12.3).

Profile:
- magicless zstd frames, no content checksum, no content size field
- window size at most 1 MiB
- no dictionary (no `Dictionary_ID` in the frame header; a receiver also reads an ID of 0 so, and a
  sender never writes one), or dictionary `ed3k-v1`, dictionary ID 1, shipped with this spec
  (`dict/ed3k-v1.dict`, trained on a corpus of real messages before the format is frozen)
- each body compressed on its own; no context carried between messages
- a receiver MUST stop and answer `Error{MALFORMED}` when output exceeds 1 MiB (an initiator
  reading an answer resets the exchange instead, §5.2)

A sender MUST write this profile. A receiver MUST still accept a frame with content size or
checksum; it answers `Error{MALFORMED}` to a body not exactly one zstd frame or with
window > 1 MiB. Dictionary IDs count from 1: RFC 8878 keeps IDs below 32,768 for a public registry,
irrelevant to magicless frames never leaving ed3k.

**Dictionaries.** A published dictionary never changes; a better one gets the next ID. A zstd
frame using a dictionary carries its ID in the frame header, so receiver always knows which; a
frame w/o dictionary is always accepted. A sender learns what the peer holds from
`Hello.dict` (§5.5):

- A node announcing `dict = n` MUST hold every dictionary from 1 to n. A node SHOULD hold
  dictionary 1 as soon as published; `dict` absent or 0 says it holds none, expected of an
  incomplete implementation only.
- A sender MUST NOT use a dictionary with an ID above the peer's `dict`. Until the peer's `Hello`
  arrived it sends w/o dictionary or uncompressed.
- A receiver answers a frame needing a dictionary it lacks with `Error{UNSUPPORTED}`.

So a new dictionary needs no flag day, capability bit or new wire version: nodes having it use it
with each other, everyone else keeps an older one.

Mandatory *support* removes negotiation and every "does the peer compress" variant. Mandatory
*use* would enlarge the many messages that are mostly hashes and keys.

Alternatives considered (zstd w/o dictionary, deflate, Brotli, LZ4, none) are compared in
[`docs/compression-comparison.md`](docs/compression-comparison.md).

**Side channels.** Compress-then-encrypt leaks length, relevant when a secret and
attacker-chosen text share one compression context. ed3k messages carry no secrets next to
third-party content and no context spans messages: CRIME/BREACH does not apply. An extension
adding secrets to a message MUST send that message uncompressed.

### 5.4 Common types

Fragments below and in every extension are excerpts of [`proto/ed3k.proto`](proto/ed3k.proto),
which is normative (§0).

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

Compact byte strings replace `repeated` sub-messages where a list is long with fixed-size
elements: a compact IPv4 node is 22 bytes vs ~30 as a sub-message (eMule Kad: 25).

### 5.5 `Hello`

First exchange on every connection: `Hello` travels only on the first bidirectional stream the
initiator opens (QUIC stream 0). Initiator sends `Hello`, responder answers with its own. Either
side MAY open other exchanges w/o waiting. Streams can arrive in any order, so neither side answers a
peer-opened exchange before reading the peer's `Hello` (responder: the one on stream 0;
initiator: the answer to its own). A `Hello` on another
exchange, or a second one, is answered `Error{MALFORMED}`. Malformed `Hello` on stream 0:
responder answers `Error{MALFORMED}`, then nothing more on that connection, and closes it with
`PROTOCOL` once the `Error` is acknowledged, at the latest when the 30 s below end. Close with
`PROTOCOL` also: initiator whose `Hello` is answered by a malformed one; a side with no peer
`Hello` on stream 0 within 30 s *(initial)* of the handshake; initiator getting a non-`Hello` answer on stream 0. A malformed `Hello` does not
count as a `Hello` for this 30 s rule.

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

No user hash, key or signature in `Hello`: the transport already delivered and verified the key
(T1), and the node ID is computed from it.

- `agent` and `nick` at most 64 bytes each; longer makes the `Hello` malformed.
- A `nat` above 3 reads as 0.
- A receiver ignores capability bits this version reserves or does not define.
- `ep4`/`ep6` are what the node advertises. Absent → nothing advertised in its family, the
  connection's included: observed endpoint does not replace it. In the connection's family a
  present endpoint's address is the observed one, whatever `Hello` says; the port is `Hello`'s. A port differing from the observed one is *claimed* until the
  node is dialled at it (§8.2 (5)). Which endpoint a node advertises: §9.2.

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
hint: receiver does not ask again sooner. 0 = no hint: the concerned section's wait applies
(30 min for a relay, §9.4), or 60 s *(initial)* where it names none. A receiver reads a value above 24 h
*(initial)* as 24 h. `MALFORMED` is for a body that does not parse, or breaks a size or layout
rule of §0, §5 or the concerned document; a frame header that does not parse, incl. a `len`
above 1 MiB, closes the connection (§4.1, §5.1). A record failing a content check is answered per
record (§10.3, §10.7), not with an `Error`.

---

## 6. Identity

### 6.1 Key and node ID

A node has one long-lived **Ed25519** key pair.

```
NodeID = H("ed3k v1 node id", public_key)[0..16)
```

Node ID is a node's only identifier: Kad position seed (§6.3), credit record key, source ID in
source records, what a dialler authenticates. The eD2K user hash has no
counterpart. As text (link, file or directory name, display) a node ID is 32 lower-case hex
digits; a reader accepts either case (§13.2).

**Why 128 bits.** Impersonating a node = finding a key whose ID equals the victim's: second
preimage, 2¹²⁸ work. The 2⁶⁴ birthday bound only lets an attacker collide two of their *own*
keys, gaining nothing. IDs are the most frequent field (every contact, every source), so 16 bytes
instead of 32 is the largest single saving available.

**Why truncate, not fold.** For a hash w/o structure, XOR-folding 256 bits to 128 is exactly as
strong as truncating. Truncation keeps the short ID a prefix of the full hash, leaving room to
lengthen IDs later w/o renumbering the network.

**Alternatives considered:** 256-bit IDs everywhere (16 more bytes per contact and source); 256
bits in Kad and a folded 128 outside (two forms, and the saving misses Kad contact lists, where
most IDs are).

### 6.2 Secure identification

eD2K's secure identification (RSA-384, SHA-1, a challenge packet pair) is replaced by the
transport handshake (T1); no separate message. A node is identified from the first byte of the
first exchange, both directions.

Ed25519 chosen for 32-byte keys, 64-byte signatures, fast verification, universal library
support. Key type fixed in version 1; a future type gets a new node-ID context string and a new
wire version. Signed records (§10.7) already carry a list of signatures, so a second algorithm
can be added w/o a new record format.

### 6.3 Routing ID: IP-bound, key-bound

Were keyspace position freely chosen, an attacker would generate keys until one lands next to a
target, then eclipse it. BitTorrent's BEP 42 prevents this by deriving the ID's top bits from
the node's IP address, but its IDs aren't tied to a key. ed3k combines both:
position fixed by the address in the top 21 bits, by the key in the remaining 107.

```
r          = NodeID[15] & 0x07

IPv4:  m = ip[0..4)  AND  03 0F 3F FF
IPv6:  m = ip[0..8)  AND  01 03 07 0F 1F 3F 00 00

both:  m[0] = m[0] OR (r << 5)
       c    = CRC32C(m)                              (Castagnoli, as in BEP 42)

RoutingID(address) = top 21 bits of c  ‖  bits 21..127 of NodeID
```

- A node has **one node ID** and one routing ID **per public address**. A dual-stack node sits at
  two positions with the same low 107 bits.
- `r` comes from the key, adding nothing to the wire. As in BEP 42 an address can reach 8
  prefixes; an attacker picks among them by generating keys (cheap), gaining nothing beyond
  those 8.
- IPv4 mask is BEP 42's. IPv6 mask deliberately differs: BEP 42 hashes the top 64 bits, so one
  /48 reaches 2¹⁸ prefixes. Here only the top 48 bits count (1+2+3+4+5+6 = 21 bits, exactly the
  prefix length), so a /48 site reaches 8.
- Kad distance is XOR over routing IDs. Targets (§10.1) are plain 128-bit values.

**Verification.** A contact is *claimed* on arrival in a `KadAnswer`, *verified* once a
connection to it completed: handshake proves the key (low 107 bits), QUIC address validation the
source address (top 21). Only verified contacts are inserted into a routing table, handed out in
answers, counted towards lookup termination or used as storage targets. Costs nothing extra:
every queried node is connected to anyway.

Private addresses (§8.2 (8)) are exempt in a named network (§4.3), as a LAN or test network is:
acceptable there although §8.2 (7) rejects them elsewhere, top-21-bit check skipped, and routing
ID of a node at such an address = its node ID. Diversity
limits exempt them too ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1).

**Test vectors** (CRC32C of the masked address; first three match BEP 42):

| Address | r | CRC32C | top 21 bits |
|---|---|---|---|
| 124.31.75.21 | 1 | `5fbfbdb2` | `0bf7f7` |
| 21.75.31.124 | 6 | `5a3ce9b0` | `0b479d` |
| 65.23.51.170 | 6 | `a5d4344a` | `14ba86` |
| 2001:db8:1234:5678::1 | 3 | `32b2d72e` | `06565a` |

The IPv6 row uses a documentation prefix that §8.2 (7) rejects; only its CRC is a vector.

**Limits, stated plainly**

- Position moves when the address changes; identity does not: credits, source records and
  sessions follow the node ID.
- On IPv4 an attacker needs ~2¹⁶ addresses to place one node within a chosen 19-bit region (each
  address reaches 8 of 2²¹ prefixes). On IPv6 a /32 holder controls 11 of the 21 bits, so a few
  dozen /32 allocations, or one /23, do the same. IPv6 positions are thus weaker than IPv4.
- Far fewer bits are needed in practice. On a 100,000-node network ~13 matching bits put a node
  among the closest to a key, and one IPv6 /29 or one IPv4 /16 reaches 131,072 prefixes. The
  routing ID alone does not stop such a block's holder; hence the diversity limits below are
  required.
- Nodes behind one carrier-grade NAT address share 8 prefixes. In practice they are leaves, not
  in routing tables.
- BEP 42 itself was never enforced by default in the BitTorrent DHT. ed3k can enforce it from the
  first version: no installed base to break.

**IP diversity limits (REQUIRED).** A node MUST apply the limits of
[`extensions/ip-diversity.md`](extensions/ip-diversity.md) wherever it builds a set of contacts:
per bucket, whole routing table, per answer, per lookup, closest set, among a record's
publishers. Three groups per family: address, /24, /16 (IPv4); /64, /48, /32 (IPv6). Each node
applies them to what it accepts: no wire change, hence requirable from the first version.

**Extension reserved for a network upgrade.** Not part of version 1.

- *Key puzzle*: a `ROUTER` node shows a solution to a memory-hard puzzle (Argon2id, 64 MiB) bound
  to its key, its /24 or /48 and a 30-day epoch. A nonce search, so the node ID does not change;
  puts a recurring price on every router identity. Designed in
  [`extensions/key-puzzle.md`](extensions/key-puzzle.md); introduced with a capability bit and a
  date after which routers w/o a solution are treated as leaves. Capability bit 7 and field 9 of
  `Hello` are reserved for it.

**Alternatives considered:** pure key hash with diversity limits only (attacker with a handful of
subnets can sit next to any target); key grinding until the hash matches the address (~2²¹ key
generations per address change, and identity would change with the address); a signed
short-lived routing key under the identity key (second key, a certificate and a grind per address
change).

### 6.4 The identity key in a key agreement

Where a document uses a node's Ed25519 key for X25519 (finding a known node, anonymous paths, the
datagram binding), the key is converted the standard way:

```
x_private = clamp( SHA-512(ed25519_seed)[0..32) )
x_public  = (1 + y) / (1 − y)  mod 2²⁵⁵−19          y = the Ed25519 public key's y coordinate
```

(libsodium: `crypto_sign_ed25519_sk_to_curve25519`, `crypto_sign_ed25519_pk_to_curve25519`.)
A node MUST refuse a public key whose y is 1 or −1 before converting it (y = 1 has no inverse,
y = −1 gives x = 0), and MUST refuse an all-zero shared secret. One key for signatures and, after
conversion, X25519 is established practice; each use's context strings keep results apart.

---

## 7. Files

### 7.1 Identity

```
root = BLAKE3(file contents)          32 bytes, default (unkeyed) mode
```

A file is identified by `root`. The size travels with it in every record but is not part of the
identity: a size in a record or from a peer is a claim until a block has been verified under it
(§12.3). The root is what the standard `b3sum` tool prints.

### 7.2 Verification

BLAKE3 is a Merkle tree over 1 KiB chunks. Any aligned range of 2ᵏ chunks is a subtree with its
own 32-byte chaining value, as is a range starting aligned and cut short by end of file. So parts
of a file can be verified against the root w/o the rest.

- **Block:** 256 KiB (2⁸ chunks). Unit of verification and of hash exchange.
- **Piece:** 4 MiB (16 blocks). Unit of availability bitmaps.

A downloader fetches block chaining values with `HashRequest` (§12.4), verifies them against the
root, then each received block on its own. A corrupt block is detected after 256 KiB, its sender
known exactly. Hash data is 32 bytes per block, 0.012 % of the file. A file of at
most one block needs no hash exchange.

Replaces both eD2K part hashes (MD4 over 9.28 MiB) and AICH (SHA-1 tree over 180 KiB blocks,
trusted by majority vote since the eD2K hash does not commit to it). In ed3k the root commits to
every block; nothing to vote on.

### 7.3 Choice of hash function

One function for files, node IDs, Kad targets, key derivation and obfuscation keystream. BLAKE3
because the hash tree is part of the function: spec need not define one; root is what a standard
tool computes. It also supplies keyed hashing, key derivation with context
strings and extendable output, so no second primitive is needed for §4 and the node ID (§6.1).
Cost: a thinner round margin than the alternatives, and one more dependency.

SHA-256 with a tree defined on top is the fallback if BLAKE3 is ever weakened: root field is 32
bytes either way and the hash is named in every context string, so it can be replaced with a
wire version bump.

Comparison with alternatives: [`docs/hash-comparison.md`](docs/hash-comparison.md).

### 7.4 The eD2K alias

`FileInfo.ed2k` carries the file's eD2K hash (MD4 root of the 9.28 MiB part hashes). With `size`
it is the eD2K identity, letting a dual-protocol client resolve an eD2K link on ed3k and download
one file from both networks at once.

- In version 1 a sender that computed the eD2K hash over its own complete, verified copy of the
  file MUST include it. A sender having the pair only from others does not pass it on.
- A receiver MUST accept records without it and MUST NOT use it as a key in any ed3k message.
  Every request names a file by `root`.
- The pairing (`root`, `ed2k`) in a record is a **claim** until the receiver verified data under
  both hashes. See §14.4.

A node MAY also publish the pairing as an `ALIAS` record of scheme `ed2k`
([`extensions/alias.md`](extensions/alias.md) §2), so an eD2K link resolves w/o keyword search.

**The alias is expected to be dropped.** Nothing in ed3k depends on it: it is one optional field.
A later revision turns the MUST above into a MAY, and field 4 is then left reserved.

---

## 8. IPv4 and IPv6

### 8.1 One overlay

One keyspace, one record store per node, one node ID per node. Addresses of both families live
in the same routing table. A node with public addresses in both families holds two positions
(§6.3) and bridges single-stack nodes.

**Alternative considered:** two separate overlays as in BitTorrent (BEP 32). Every table contact
is then dialable, but dual-stack nodes maintain and publish twice and the IPv6 overlay starts
small. Fully separate instances per family were rejected outright.

### 8.2 The dual-stack contract

1. A record (`Peer`, compact node) carries **at most one endpoint per family**.
2. A node MUST use the same UDP port number in both families when it can, and MUST advertise the
   real ports when it cannot.
3. Requests returning contacts state which families the asker can dial (`KadRequest.want`). A
   node answers with contacts of those families only.
4. A single-stack node inserts only contacts of its own family into its routing table.
5. In the family a connection arrived on, an advertised endpoint's address is **observed**, thus
   trusted for routing; a port differing from the observed one, and an endpoint of the other
   family, are **claimed** until dialled.
6. Abuse limits count per IPv4 address and per IPv6 /64; a private address in a named network
   counts as IPv4 /32 or IPv6 /128 (§9.2).
7. An address is acceptable only if global unicast. For IPv4 this excludes at least
   `0.0.0.0/8`, `10.0.0.0/8`, `100.64.0.0/10`, `127.0.0.0/8`, `169.254.0.0/16`,
   `172.16.0.0/12`, `192.0.0.0/24`, `192.0.2.0/24`, `192.168.0.0/16`, `198.18.0.0/15`,
   `192.88.99.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `224.0.0.0/4` and `240.0.0.0/4`.
   For IPv6 the address MUST be inside `2000::/3`, and outside at least `2001::/32` (Teredo),
   `2001:2::/48`, `2001:10::/28`, `2002::/16` (6to4), `2001:db8::/32` and `3fff::/20`.
   That leaves out everything else, incl. `::/128`, `::1`, `::/8`, `fe80::/10`, `fec0::/10`,
   `fc00::/7`, `::ffff:0:0/96`, `64:ff9b::/96`, `100::/64`, `5f00::/16` and `ff00::/8`. A
   library's "global unicast" test, e.g. Go's `IsGlobalUnicast`, is too loose: it accepts
   `::81.2.3.4`, `4000::1` and `fec0::1`. A source address a socket reports as IPv4-mapped is
   treated as the IPv4 address it holds. Not acceptable: an 18-byte wire endpoint holding an
   IPv4-mapped address, and an endpoint with port 0. For an HTTP fetch target,
   [`extensions/http-source.md`](extensions/http-source.md) §5 judges an address of
   `64:ff9b::/96` or `2002::/16` by the IPv4 address inside it.
8. **Private addresses** are `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`,
   `169.254.0.0/16`, `127.0.0.0/8`, `::1`, `fe80::/10` and `fc00::/7`. "Private, loopback,
   link-local and unique-local" in a document means this list. `100.64.0.0/10` is neither
   private nor global unicast. A private address is acceptable, as a contact and as an address
   to dial, only in a named network (`net_id` not empty, §4.3; §6.3), and for a peer found in
   the local network ([`extensions/lan-discovery.md`](extensions/lan-discovery.md) §3). On the
   public network it is never inserted into a routing table, stored in a record or handed out.

### 8.3 Dialling

Peer and dialler both have endpoints in both families → IPv6 first, IPv4 250 ms *(initial)*
later; first completed handshake wins. Dialler closes the other with `CLOSED` at once, also if it
completed, unless a rule needs a connection in that family too (§4.6); it counts in the §4.6
budget like any connection. A dialler trying several addresses of
a peer in one family, e.g. a `PROBE`'s source and `eps(T)` (§9.5), runs one handshake per address
and closes the others with `CLOSED` once one completes; until then they are not a second
connection in the sense of §4.6. A connection stays in the family it was opened in: a side does not
let it migrate to the other family (§4.5); routing table entries do not change with it (§10.2).

A node SHOULD advertise a stable IPv6 address rather than a temporary one (RFC 4941), and send
from the address it advertises.

---

## 9. NAT traversal

**NAT traversal is REQUIRED.** Every node MUST implement address observation (§9.2), act as punch
initiator and target (§9.5), act as A of a `ReachTest` (§9.3), and, when `ROUTER`, offer the
relay service (§9.4). Index node offers relay as §11 says. A node behind a stateful IPv6 firewall is like one behind an IPv4
NAT; same procedure.

### 9.1 Before traversal

A node SHOULD first try direct reachability via port mapping (PCP, NAT-PMP, UPnP IGD; IPv6:
firewall pinhole). Traversal is the fallback.

### 9.2 Address observation and classification

Every `Hello` reports sender's view of receiver's endpoint (`observed`); a node collects these per local
socket and address family:

| Class | Evidence |
|---|---|
| `open` (1) | `PROBE`s of two `ReachTest`s asked of peers in different sites arrived within 60 min *(initial)* (§9.3), or unasked connections arrived (below), or operator declared node open (below) |
| `cone` (2) | one endpoint has the lead (below); no evidence for `open` |
| `symmetric` (3) | one address has the lead (below); no endpoint has it |
| `unknown` (0) | else: too few reports, or reports naming different addresses |

Rows tried top-down; first fit wins.

**Which reports count.** A dialling-in node can put anything in `observed`, from many node IDs.
A report counts only if all hold:

- Arrived on a direct connection the node opened itself. A hole-punched connection (§9.5) counts
  for its initiator only.
- Newest report from its site; site = peer's /24 or /48
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1). Peers never told apart by
  node ID.
- ≤ 60 min *(initial)* old. An open connection does not refresh it: report ages from the `Hello`
  carrying it.
- Names a public address. Not counted: report of a private, loopback, link-local or unique-local
  address, or from a peer reached at one.

In a named network a private address is its own site: sites of this section, §9.3 and §9.4 are
host addresses (IPv4 /32, IPv6 /128). Likewise a private address's host address is IPv4 /32 and IPv6 /128
wherever ed3k counts per host address: reservations of §9.4, punch and `PROBE` limits of §9.5,
extensions. §9.4's site limit then does not bind (site = host).

**The lead.** An endpoint has the lead when ≥ 3 *(initial)* counted reports name it and at least
twice as many as any other endpoint. Only an endpoint with the lead is adopted as own. So one
peer's false port can neither change the address nor turn `cone` into `symmetric`; that needs two
sites agreeing on one false port, and two suffice against 3 honest ones (true endpoint then lacks
the lead). An address has the lead by the same rule, counting reports
naming it whatever their port.

While no endpoint has the lead, a node adopts its own socket's address and port where no NAT can
be in front: if address acceptable (§8.2 (7)) and operator declared node open in that family
(below), or a private address in a named network (§6.3), where no report counts. Else a
network's first routers have no routing ID and answer every `KadRequest` without contacts
(§10.3).

**Advertised endpoint.** `cone` node advertises in `Hello` its adopted endpoint with the mapped
port (a punch aims there). Other classes: adopted endpoint if any; else best-known address
(named by most counted reports), else socket's address if acceptable, with a port mapping's
external port (§9.1) if any, else socket's port. No known address in a family → family left out.

**The first nodes of a network.** A reach test needs a `ROUTER` B (§9.3), so in a network with
none, nobody becomes `ROUTER` by tests. An operator MAY therefore declare a node open per family:
node then `open` there without evidence, asks no `ReachTest` there, sets `ROUTER` by §2.

**First lookups.** No adopted endpoint in a family → no routing ID there to look up (§10.3);
adopting needs reports from 3 sites. After the first `Hello` of a `ROUTER` the node dialled in that
family, it takes the routing ID of the `observed` endpoint in it, if acceptable, and looks up
that ID and the two IDs differing from it in the first and in the second bit; these lead to
further sites. Done once per family.

**Unasked connections.** A connection completing from a host the node sent nothing to in the last
60 min *(initial)* cannot have passed a NAT or stateful firewall. Two such from different sites within
60 min *(initial)* = evidence for `open` in that family, like the `PROBE`s of two tests (§9.3).
Relayed and punched connections don't count. Two, since one can come through a mapping an
earlier exchange left open.

**Testing again.** A class can go stale (mapping expires, firewall rule changes).

- `ReachTest` every 60 min *(initial)* per family, and at once on local address or observed
  endpoint change; asked of peers in different sites in turn, so no single peer decides.
- No prober asked (`ok` not set, or no answer): retry with another peer after 30 s *(initial)*,
  doubling up to 60 min *(initial)*.
- An `open` node whose tests had a prober asked (`ok = true`) and got no `PROBE` in two
  *(initial)* such tests in a row is no longer `open`, unless unasked connections arrived since
  the first of the two. It then classifies itself from address reports and takes relays (§9.4).
- An `open` node MAY skip a due test while holding unasked-connection evidence < 60 min
  *(initial)* old. A test after an address change is never skipped.

### 9.3 `ReachTest`

```protobuf
message ReachTest {
  bytes  target_id = 1;       // set when forwarded: node to probe
  bytes  target_ep = 2;       // set when forwarded: endpoint to probe
  fixed64 nonce = 3;          // chosen by the asker; goes into the PROBE as 8 B, big-endian
  bool   ok = 4;              // answer: a prober was asked
}
```

1. Asker sends `ReachTest{nonce}` to peer A on a connection the asker opened in the tested family.
   `nonce`: random, asker-chosen, to recognise the `PROBE`. Asker SHOULD NOT dial a routing-table
   contact to ask it as A (its NAT may already map that host). A connection the asker already
   opened MAY serve as A, also to a table contact; step 5 judges the `PROBE`'s source host.
2. A picks a `ROUTER` contact B of its routing table in that family, not the asker, that A
   believes has no connection with the asker; forwards to B on a new exchange with `target_id`,
   `target_ep` and the asker's `nonce`. `target_ep` MUST be the asker's observed endpoint as A
   sees it, never asker-supplied. A request arriving with `target_id` set is never re-forwarded.
3. B sends one `PROBE` datagram (§4.3) with the nonce to `target_ep`, which MUST be an acceptable
   address (§8.2 (7), and (8) in a named network), and answers A `ReachTest{ok = true}`;
   `ok = false` if it does not probe; `Error{BUSY}` with `retry_after` at its limit.
4. A answers asker `ReachTest{ok}`: `ok = true` if a B agreed to probe, `ok = false` if no B
   found or B declined. On a B declining or `Error{BUSY}`, A first tries up to 2 *(initial)*
   more. A waits ≤ 3 s *(initial)* per B; silent B = declined. Asker waits 10 s *(initial)* for
   A (covers A's 3 waits); no answer = no prober asked (§9.2).
5. A `PROBE` with the nonce counts from sending the request until 5 s *(initial)* after the
   answer, or 10 s *(initial)* after the request if none comes (B probes before answering). It is
   evidence of `open` in that family only if from a host the asker sent nothing to in the last
   60 min *(initial)*, as for unasked connections (§9.2): a mapping left open by an earlier
   exchange would pass it. `PROBE` with an unasked-for nonce: ignored. With few `ROUTER`s a node
   may have sent to all while bootstrapping; its tests then show nothing until 60 min pass.

A `PROBE` is evidence only as far as A is honest: A knows nonce, asker's `obf_id` and endpoint,
and could send the `PROBE` itself. So a non-`open` node becomes `open` only on the
`PROBE`s of two tests asked of peers in different sites
([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1) within 60 min *(initial)* (as
§9.2 asks two sites of unasked connections); after the first `PROBE` it asks the second at once,
of a peer in another site. An `open` node stays so by §9.2.

A limits these to 1 per asker address per minute *(initial)*; further → `Error{BUSY}` with
`retry_after`. B limits to 2 per `target_ep` address per minute *(initial)*, whichever A asks:
an asker's second test may reach the same B.

B cannot check `target_ep` is the asker's; a false A can name any acceptable address. Accepted:
a `PROBE` is small, never answered (§4.4), and the limit above bounds what one A causes.

### 9.4 Relays

A node keeps reservations at ≥ 2 *(initial)* relays (more where this section says) while it has
an address in a family where it is not `open` — incl. a `ROUTER` open in one family only.

```protobuf
message Relay {
  bool   reserve = 1;         // request: true = hold a slot, false = release
  uint32 ttl = 2;             // answer: seconds the slot is held without traffic
}
```

Relay holds the connection open, forwards `Punch` messages to the leaf. Leaf lists its relays in
every `Peer` record it publishes (`relays4`, `relays6`).

**A reservation** lasts as long as its connection; not carried over. Leaf sends the PING of §4.1,
at an index node at the §11 interval; any packet counts as traffic. `ttl` informational: time
the relay keeps the slot without traffic, ≤ idle timeout (§4.1). Slot ends with the connection. Leaf does not
repeat its request. `Relay{reserve = false}` returns the slot, answered `Relay{ttl = 0}`.

**One slot per leaf.** A leaf connected to the relay in each family (§4.6) MAY reserve on both.
Relay keeps one slot per leaf node ID, held by both connections: `Reserve` on the second is
answered like the first, without checking the limits below; neither connection is told of the
other. Slot ends when the last holding connection closes or returns it. A forwarded `Punch`
(§9.5) goes over the connection that reserved last and is still open, and over the other when
that fails (exchange cannot be opened, or fails within the 5 s of §9.5 step 2). A timeout is not
retried.

**Listing.** The list says how an initiator reaches the relay, not which family the punch uses
(`Punch` carries both endpoints). A relay is listed with its own endpoint in that family, so
nodes prefer relays having both.

| The node is not open in | `relays4` | `relays6` |
|---|---|---|
| IPv4 only, or it has no IPv6 | both relays | none |
| IPv6 only, or it has no IPv4 | none | both relays |
| both families | both relays | one of them |

A relay with no endpoint in a list's family is left out of it; a node whose list would then be empty takes a
further relay having one. List ≤ 2 relays (§10.3). A node holding more relays than a list takes
lists, of relays with an endpoint in the list's family, those with endpoints in both families
first and, among equals, the ones it chose last.

Why 2, compared to one relay, two per family and libp2p (2 by default): [`docs/relay-count.md`](docs/relay-count.md).

**Finding relays.** Leaf takes relays from `ROUTER` nodes met in own Kad lookups, and from connected index nodes
(§11), which lookups miss when they leave `ROUTER` unset. Index node offers the service as §11
says.

- Ask nodes whose `Hello` has `RELAY` set, with `Relay{reserve = true}`. One answering
  `Error{BUSY}` is skipped, not asked again for 30 min *(initial)*.
- Relays SHOULD be in different sites ([`extensions/ip-diversity.md`](extensions/ip-diversity.md)
  §1), so losing one network does not take all.
- Lost relay replaced at once; new list goes out with each record's next refresh. Only when a
  published list has no working relay left does the leaf republish its `Peer` records at once.

**Capacity.** Leaf: reservations at ≥ 2 relays; relay: many leaves. A `ROUTER` MUST be able to
serve at least 1 leaf and SHOULD be able to serve at least 8 *(initial)* at once, one reservation
per leaf node ID, and MAY serve more. It holds ≤ 2 *(initial)* reservations per host address and
4 *(initial)* per site ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1), so cheap
keys from one network cannot take every slot; a further one from that network → `Error{BUSY}`.
A node with the `INDEX` capability MAY use wider per-host and per-site limits, as for connections
(§4.6): behind carrier-grade NAT many clients share one address; it SHOULD serve far more than 8
leaves. Forward limits below are per leaf, not widened. When all slots taken, answers `Error{BUSY}` and
clears `RELAY` in its `Hello` until one is free. An already-connected peer doesn't see the
cleared bit; for it `Error{BUSY}` is the signal (§2).

**Forward limit.** Each forwarded `Punch` makes the leaf probe for 5 s (§9.5), so the relay
limits forwarding:

| Counted per | Limit *(initial)* |
|---|---|
| leaf | 30 forwards per minute |
| initiator host address and leaf | 3 forwards per minute |

Over a limit relay answers initiator `Error{BUSY}` with `retry_after`, forwards nothing. Second limit
matches §9.5's 3 attempts. A leaf whose relays often hit the first limit SHOULD take a further
relay.

**A relay forwards signalling only.** Never carries file data or any other peer-to-peer exchange: keeps it cheap
enough to oblige every reachable node, useless as a proxy. Consequence: a pair whose punch fails
cannot transfer.

**Alternatives considered:** rate-limited data relay as last resort (connects every pair, but
relays pay bandwidth for strangers, need quota and abuse rules); multi-port "birthday" probe for
symmetric NATs (hundreds of probes per attempt, trips router session limits, looks like a scan).
Both addable later as capabilities without changing §9.5. A one-hop path
([`extensions/hops.md`](extensions/hops.md)) carries data for such a pair, at that extension's
cost and limits; its last hop reaches a leaf via the leaf's relays (§9.5), named in `HopConnect`.

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

1. I dials T's endpoints first, if any (§8.3). If no handshake completed 2 s *(initial)* after the
   first direct attempt, or at once if T has no endpoint I can dial, I opens an exchange to one of
   T's relays and sends `Punch` with a random `nonce` of its own; direct attempts go on. If that
   relay is unreachable, silent for 7 s *(initial)*, or answers anything but `Error{BUSY}` or a
   `Punch` for T, I asks T's next relay in the same try. Such an answer: any other `Error`
   (`NOT_FOUND`, `UNSUPPORTED`, `DENIED`, `MALFORMED`, unknown codes included), or a `Punch` whose
   `peer_id` is not T's or that breaks a rule of this section. An endpoint at a non-acceptable
   address (§8.2 (7), and (8) in a named network) breaks no rule: only left out; answer declines
   if no endpoint is left. `Error{BUSY}` ends the try: may come from R or T (step 4).
2. R forwards to T on a new exchange: `peer_id` → I's ID; endpoint of the family the `Punch`
   arrived on → I's observed endpoint as R sees it (as §9.3 for `target_ep`) if I's `Punch` holds
   one there (a family I left out stays declined; the other stays as I claimed); `seen` = family
   the `Punch` arrived on, also if I declined it; rest unchanged. `seen` also marks a forwarded
   `Punch`, so never 0 here. I's `Punch` has `seen` 0; one with `seen` set is no request to R as
   relay (**Who may ask**). R answers `Error{NOT_FOUND}` if T holds no reservation; waits 5 s
   *(initial)* for T, then answers `Error{BUSY}`.
3. T answers with its own endpoints, those it advertises (§9.2); in its reservation's family, if
   it advertises none there, the `observed` of the relay's `Hello`. Its `nonce` is 0, and I
   ignores it. T at once sends `PROBE` datagrams carrying I's `nonce` to each of I's endpoints in
   families both share and T does not decline (**Class and family**), at random 100–300 ms
   *(initial)* intervals per endpoint for 5 s *(initial)*, opening T's mapping. `PROBE` only to an
   acceptable address (§8.2 (7), and (8) in a named network), and ≤ 3 *(initial)* to an endpoint
   `seen` does not mark: that is I's claim, and a false one would aim probes at a third party.
4. R returns T's answer to I: `peer_id` = T's ID; endpoint of T's reservation family → T's
   observed endpoint if T's answer holds one there (a family T left out stays declined); `seen` =
   that family, also if T left it out. I dials T with a normal QUIC handshake; retransmitted
   Initials open I's mapping. I = QUIC client, T = server. A `PROBE` with I's `nonce` reaching I
   before the answer or ≤ 5 s *(initial)* after tells I the address T's packets leave from: if
   acceptable, I dials it at once, beside the dials to `eps(T)`. One handshake per address; an
   attempt already dialling an answer address restarts when the answer arrives, so its Initials
   fall into T's 5 s of `PROBE`s. `PROBE` with a nonce I did not choose: ignored (§9.3). An
   `Error` from T is returned to I unchanged, `retry_after` included.
5. A try failed when no handshake with T completed 8 s *(initial)* after the relay's answer, or it
   ended at `Error{BUSY}`, or no relay of T is left to ask. I tries up to 3 times *(initial)*,
   preferring for the next try a relay the last try did not ask. Then T is marked unreachable for 30 min
   *(initial)*; at once after a declining answer of T (**Class and family**).

**Who may ask.** A forwarded `Punch` (`seen` not 0) whose `seen` is not 1 or 2 is
`Error{MALFORMED}`, wherever it arrives. T acts on any other forwarded `Punch` only if it arrives
on the connection to a relay where T holds a reservation; elsewhere `Error{DENIED}`. `Punch` with
`seen` 0 = request to the node as relay; a node offering no relay service (§9.4) answers
`Error{UNSUPPORTED}`. A node can be a peer's relay and a leaf at it at once, told apart by `seen`
(only a relay sets it). T starts ≤ 30 *(initial)* punches per minute, 3 *(initial)* per minute
towards one host address, `Error{BUSY}` beyond. It sends ≤ 50 *(initial)* `PROBE`s per punch, shared
evenly over I's endpoints, and ≤ 100 *(initial)* per minute to one destination host address over
all punches; beyond that none to that host. Relay limits (§9.4) bound an honest relay only.

**Class and family.** `nat` in `Hello` = sender's class in the connection's family. `nat` in
`Punch` = sender's class in its own hop's family (the connection it sent the `Punch` to the relay
on); relay forwards it unchanged; receiver learns that family from `seen` (steps 2 and 4). T
declines a family by leaving its endpoint out of its answer; no endpoint = punch declined. A
declining answer ends the try, also if T's `PROBE`s arrived, and I does not punch T again for
30 min *(initial)* (step 5); an already completed handshake stands. An answer whose acceptable
endpoints are all in families I cannot dial is no decline but leaves no family to try: ends the
try the same way.

Pairs that cannot work (`symmetric` on both sides in a family both need) are not punched. I
doesn't know T's class before T answers, and `nat` holds for one connection's family only, so the
sides decide in turn. T declines such a family, judging I by its `nat` in the family `seen`
marks. I judges T by the answer's `nat` in the family `seen` marks; if no family is left to try,
no punch of T again for 30 min *(initial)*. IPv6 preferred when both have it: a stateful firewall
has no port mapping to guess.

libp2p measurements of the same procedure (DCUtR): success near 70 % of attempts across all NAT
types, above 80 % for port-restricted cone NATs.

### 9.6 Why a self-contained design

Traversal above = four messages on a QUIC library letting the application own the UDP socket.
Existing stacks (libp2p, iroh, ICE/STUN/TURN) not adopted: none leaves room for the obfuscation
of §4, and each brings its own socket, runtime or servers. §9.5 is libp2p's DCUtR; §9 stays close
to the QUIC drafts for address discovery and NAT traversal, to be replaceable by the standard
frames in a later wire version.

Comparison: [`docs/nat-traversal-comparison.md`](docs/nat-traversal-comparison.md).

---

## 10. Kad

### 10.1 Targets

| Record kind | Target |
|---|---|
| `SOURCE`, `NOTE`, `MEMBER` | `root[0..16)` |
| `HEAD` | `H("ed3k v1 set head", key ‖ id)[0..16)`; author's key, set's `id` ([`extensions/file-set.md`](extensions/file-set.md) §5.1) |
| `LIVE` | `H("ed3k v1 live", key ‖ id)[0..16)`; channel's key and `id` ([`extensions/live.md`](extensions/live.md) §1.1). Private channel: target changes hourly (§10.4 there) |
| `HIDDEN` | `H("ed3k v1 hidden head target", base ‖ period(8))[0..16)`; value from set's key and day number ([`extensions/hidden-set.md`](extensions/hidden-set.md) §2) |
| `PART` | `H("ed3k v1 parts", key ‖ id ‖ issuer key)[0..16)`; owner key, set's `id`, key of the issuer whose certificate the record carries, or no issuer key ([`extensions/multi-author-set.md`](extensions/multi-author-set.md) §3.4) |
| `FRIEND` | `H("ed3k v1 friend target", k ‖ period(8))[0..16)`; value from two friends' keys, or a friend link, and day number ([`extensions/friend.md`](extensions/friend.md) §4). Friend request has own target (§3 there) |
| `ANON` | `root[0..16)`, as `SOURCE`: source reached through other nodes, no address ([`extensions/hops.md`](extensions/hops.md) §4.3) |
| `GATE` | `H("ed3k v1 gate", node ID)[0..16)`; node ID of such a source (§4.3 there) |
| `ALIAS` | `H("ed3k v1 alias", len(scheme) as 1 byte ‖ scheme ‖ id)[0..16)`; kind of name, and name ([`extensions/alias.md`](extensions/alias.md) §1) |
| `SUB` | `H("ed3k v1 topic", type(1) ‖ key ‖ id)[0..16)`; topic's type, key, `id` ([`extensions/pubsub.md`](extensions/pubsub.md) §1). A room stores `SUB` records under own target ([`extensions/room.md`](extensions/room.md) §1.2) |
| `NAME` | `H("ed3k v1 name", zone ‖ label)[0..16)`; zone's root key, a name in it ([`extensions/name.md`](extensions/name.md) §3.2) |
| `CLAIM` | `H("ed3k v1 name claim", name)[0..16)`; a global name ([`extensions/name.md`](extensions/name.md) §4.2) |
| `KEYWORD` | `H("ed3k v1 keyword", level(1) ‖ word)[0..16)`; `word` = one keyword of the file name in UTF-8, defined below. `level`: one byte, 0 on Kad in version 1, 0 to 15 at an index node (§10.8, §11) |

File hashes, node IDs and keyword keys come from one function, cut to the same 128 bits: one
keyspace, no conversion.

**Words and keywords.** Publisher, searcher and storing node MUST split a name the same way:

1. **Normalise.** NFKC, then full case folding, then NFKC again. Remove general category Cf code points
   (soft hyphen, zero-width joiners). A text is normalised once; a word from this split is never
   re-normalised: Cf is removed after the last NFKC, so a Cf between a letter and a combining mark
   leaves them uncomposed, and a second pass would compose them into other bytes.
2. **Split.** Word = maximal run of code points of general categories L (letter), M (mark), N
   (number). Any other code point separates.
3. **Scripts written without spaces.** In a word, a maximal run of *unspaced* code points is
   replaced by its overlapping pairs: run of n ≥ 2 → n − 1 two-character words; run of 1 → that
   character. Parts before and after the run are own words. Unspaced code points:

   | Script | Code points |
   |---|---|
   | Han, Hiragana, Katakana | U+3005–3007, U+3040–30FF, U+31F0–31FF, U+3400–4DBF, U+4E00–9FFF, U+20000–323AF |
   | Thai | U+0E00–0E7F |
   | Lao | U+0E80–0EFF |
   | Khmer | U+1780–17FF |
   | Myanmar | U+1000–109F, U+A9E0–A9FF, U+AA60–AA7F |

   Pairs are of code points. In Thai, Lao, Khmer, Myanmar a vowel sign or tone mark is its own
   code point, so a pair may begin or end with one. Tibetan needs no pair rule: syllables are
   divided by a punctuation mark (U+0F0B), so step 2 already gives one word per syllable.
4. **Extension.** If text after the last `.` of the normalised name is 1 to 8 code points, all
   category L or N, it is the extension. Else none; likewise for a name without `.`.

**Words** of a name = distinct results of steps 1 to 3. **Keywords** = words ≥ 2 bytes in UTF-8,
minus the words steps 2 and 3 make of the extension (a word, the pairs of an unspaced run, or
both), also where such a word occurs elsewhere in the name: words are distinct, so
`Avi.Love.avi` has the keyword `love` alone. Targets derive from keywords only; a storing node
refuses a record under any other target (§10.7). `Query` (§10.3) matches against words.

| Name | Words | Keywords |
|---|---|---|
| `The.Big_Sleep (1946) [1080p].mkv` | `the` `big` `sleep` `1946` `1080p` `mkv` | `the` `big` `sleep` `1946` `1080p` |
| `Ça-va_ÉTÉ.MP3` | `ça` `va` `été` `mp3` | `ça` `va` `été` |
| `Straße.ﬁnal.txt` | `strasse` `final` `txt` | `strasse` `final` |
| `東京物語 Tokyo Story.avi` | `東京` `京物` `物語` `tokyo` `story` `avi` | `東京` `京物` `物語` `tokyo` `story` |

- An implementation MUST use the tables of Unicode 16.0.0 or later; examples are checked against
  that version. A code point's general category, where 16.0.0 assigns it, is taken from 16.0.0,
  also with later tables; step 3 ranges = the table above only. Unicode 17.0.0 changes no category
  this split distinguishes (L, M, N, Cf, other) for a code point 16.0.0 assigns, nor
  normalisation or case folding: 17.0.0 tables comply as is. Code points 17.0.0 adds after
  U+323AF (CJK Extension J) are outside step 3's table on purpose, so all versions split them
  alike. The Unicode stability policy fixes normalisation and case folding of an assigned
  character across versions. A code point unknown to an implementation = separator, so a storing
  node with older tables than a publisher can refuse a record whose keyword uses a later-assigned
  character; current tables bound that.
- **2 bytes is the limit of the protocol; 3 bytes is the suggested default.** Two-byte words are
  few and very common (`of`, `to`, `la`, `de`): the most crowded targets. By default a publisher
  SHOULD publish a file under its keywords of 3 bytes and more, and under 2-byte ones only when
  the name has no longer one (`AC/DC`). A searcher SHOULD derive the target from a keyword of
  3 bytes or more when its search text has one.
- A name whose words are all 1 byte long has no keyword; not findable by keyword search.
- A publisher SHOULD publish a file under at most 40 *(initial)* keywords, taken in name order.

### 10.2 Routing table

Buckets of K = 10 *(initial)* verified contacts, ordered by XOR distance between routing IDs.
Entry = one (node ID, endpoint) pair; a dual-stack node gives two entries at two positions. A
family's buckets are by distance to the node's own routing ID of that family (§6.3); when that routing ID
changes with an address change, the family's entries are re-bucketed under the new one. A node
enters a family's table only if it set `ROUTER` and its `Hello` on a connection of that family
says `nat = 1`. A contact learned otherwise, from a `KadAnswer` or a Kad datagram
([`extensions/kad-datagram.md`](extensions/kad-datagram.md) §3.4), waits for such a `Hello`. A
contact whose `Hello` on a later connection of that family no longer has `ROUTER` set and
`nat = 1` is removed from that family's table.

Contact dropped after two failed attempts in a row: failed connection attempt, or an answer
counting as failed attempt (§10.3, §10.4). Counted per contact. Only a `KadAnswer` counting no
failed attempt resets the count; an `Error` neither counts nor resets; a completed connection
does not reset. Second attempt 5 min *(initial)* after the first failed.

**Maintenance.** A bucket no lookup touched for 60 min *(initial)* gets a lookup for a random ID
in its range. A contact unverified for 60 min *(initial)* is reconnected and sent a `KadRequest`;
failure counts toward the two above. Both run in background, spread over the hour, so a quiet
node neither keeps dead contacts nor bursts. Insertion obeys the per-bucket and per-table limits
of [`extensions/ip-diversity.md`](extensions/ip-diversity.md), which hold for table contacts at
all times: if they ever tighten, the node rechecks the table and removes what no longer fits
(§2.1 there).

**Contacts that dialled in.** A contact is *dialled-in* when the node first verified it on a connection the contact opened.
At most half a bucket, 5 of K = 10 *(initial)*, may be dialled-in; a further one is not inserted.
A dialled-in contact is inserted with the endpoint observed on its connection, address and port, not its `Hello`
port (§5.5), as a source record is (§10.3). Mark stays while in the table, also after the node
connected to it itself. Opening connections to a node is the cheapest way into its table;
contacts found by own lookups are not chosen by the other side.

### 10.3 The one request

One Kad request: always asks for closer nodes; optionally carries an operation the receiver
performs if responsible for the target.

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

This pair replaces 13 Kad2 opcodes: `REQ`/`RES`, three search requests and their answer, three
publish requests with answer and acknowledgement, and bootstrap (a `KadRequest` for own routing
ID, one lookup per public address (§6.3); source of the first address: [`extensions/bootstrap.md`](extensions/bootstrap.md)).
`Hello` (§5.5) replaces the Kad hello pair, ping/pong and firewall check packets; §9 replaces
buddy packets.

Rules:

- A node returns up to 16 *(initial)* verified contacts per requested family closer to the target than
  itself; none if it knows none closer. "Itself" = its routing ID for the public endpoint it adopted
  (§9.2) in the connection's family, for `nodes4` and `nodes6` alike. None adopted in that family
  → no contacts, `responsible` not set, unless index node: an index node is responsible for every
  target whether or not it adopted an endpoint (§11); only its contacts need the routing ID.
- Every node, leaf too, answers a `KadRequest` with contacts it knows. Only a `ROUTER` `open` in
  the connection's family (§9.2), or an index node (§11), is responsible.
- **The requester checks the answer.** Computes each returned contact's routing ID from its
  address and node ID (§6.3); drops a contact not closer to the target than the answerer, and
  every contact after a family's 16th. Closer is judged, in `nodes4` and `nodes6` alike, against
  the answerer's routing ID in the connection's family (what the answerer judged by, above;
  requester computes it from the answerer's address on that connection), never one in the other
  family. An answer with a contact after the 16th counts as one failed attempt for the answerer (§10.2); a correct node
  cannot produce one. A not-closer contact is only dropped: a node whose address just changed can
  return one honestly. Check precedes the filters of
  [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.2.
- **Responsibility.** A node is responsible for a target when, in the family of the connection the request
  arrived on, it knows fewer than K verified contacts of that family closer to the target than its
  own routing ID of that family; other family not considered. Then it sets `responsible`,
  executes a `search`, accepts a `store`.
- **Search or store.** Search = `search` present, also empty; store = `store` present; both →
  `Error{MALFORMED}`; neither → contacts only, whatever `kind` says, still setting `responsible`
  as below and, for a kind it stores, `load` and `spill` (§10.8), so a lookup learns them without
  a search. `all` and `none` of a `Query`: at most 64 *(initial)* terms together; more →
  `Error{MALFORMED}`.
- **Order of checks.** In this order; first deciding step gives the answer:
  1. Parses and keeps the size and layout rules of §0 and this section, else `Error{MALFORMED}`.
     An `also` not n × 16 bytes in a request of kind `SOURCE` breaks such a rule.
  2. `search` and `store`: both → `Error{MALFORMED}`; neither → contacts only.
  3. `kind` and responsibility. `kind = NONE` with `search` or `store` → contacts only; likewise
     an unknown `kind`, or a non-responsible node (below). `level` ignored for kinds other than
     `KEYWORD`; a `KEYWORD` search with `level` above 15 is answered as by a non-responsible
     node. A responsible node answers `Error{MALFORMED}` to a `Store` holding no record of `kind`.
  4. Each record, answered in `results` (below, §10.7).
- `want = 0` asks for records only: no contacts returned. For a requester already knowing the
  responsible nodes, e.g. an index node's client (§11).
- **Paging.** Each node pages search results in its own order, not choosable by a publisher:
  records by `H("ed3k v1 page", secret ‖ root ‖ publisher key)` (a note by
  `H("ed3k v1 page", secret ‖ publisher key)`), sources by `H("ed3k v1 page", secret ‖ node ID)`,
  compared as bytes. `secret` = 32 random bytes kept private, replaced every 24 h *(initial)*. It
  MAY move targets to a new secret one at a time: a target pages in old order until its new one is
  complete; all moved before the next secret. A searcher MUST NOT expect two nodes to page alike.
  An answer holds at most 100 *(initial)* `files`, 100 `notes`, or 100 `sources` and `anon`
  together; `offset` counts records in that order, `more` = records remain. Records in several
  fields are paged as one sequence: fields by field number, each in own order; `anon` ordered as
  `sources`, by `H("ed3k v1 page", secret ‖ node ID)`. `offset` counts in that sequence; records
  left out from its end only. `http` and `links` are no sequence records: they belong to the
  source record they were stored with; an answer carries those of its `sources`, in `sources`
  order, up to their per-answer limits. A `KEYWORD` search uses every `Query` field; any other
  version-1 kind uses `offset` only, ignoring the rest, unless the kind's document says otherwise.
  No state kept between pages; in-between changes can be missed or seen twice; searcher merges by
  publisher and file. An extension paging its own records states order and count per answer.
  Pages asked of one node: local policy, unless the kind's document says otherwise.
- **Every `ROUTER` stores every record kind of version 1**, extensions' included, whether or not it
  sets the extension's capability: a kind coming into use later needs no network upgrade. Each
  kind's content and storing-node checks are in its defining document; a `ROUTER` implements
  those checks for every kind (§13.1). A `ROUTER` answering a version-1 kind as not responsible
  does not conform, though a lookup goes on past it; the unknown-kind rule (below) is for kinds
  added after version 1. An index node without `ROUTER` stores the kinds it chooses (§11).
- **`HEAD` and `MEMBER`**: file-set record kinds. Every `ROUTER` MUST store and return them,
  whether or not it sets `SETS`. Content, checks: [`extensions/file-set.md`](extensions/file-set.md)
  §5 and §6.
- **`LIVE`**: peers of a live stream; stored/returned like `SOURCE`, shorter lifetime. Every
  `ROUTER` MUST store and return it, whether or not it sets `LIVE`
  ([`extensions/live.md`](extensions/live.md) §5).
- **`SUB`**: a topic's subscribers; stored/returned like `SOURCE`, shorter lifetime. Every `ROUTER`
  MUST store and return it, whether or not it sets `PUBSUB`
  ([`extensions/pubsub.md`](extensions/pubsub.md) §5). Rooms use it for members
  ([`extensions/room.md`](extensions/room.md) §5).
- **`HIDDEN`**: head of a hidden set, which the storing node verifies but cannot read. Every
  `ROUTER` MUST store and return it, whether or not it sets `SETS`. Content, checks:
  [`extensions/hidden-set.md`](extensions/hidden-set.md) §3.
- **`PART`**: parts other authors add to a set; a `HEAD` may be a joint head, signed by several
  keys under an owner grant. Every `ROUTER` MUST store, check and return both, whether or not it
  sets `SETS` ([`extensions/multi-author-set.md`](extensions/multi-author-set.md) §1.3 and §3.4).
- **`FRIEND`**: by which friends find a node, and a node asks another to become its friend.
  Stored/returned like `SOURCE`, but content is a sealed byte string the storing node cannot
  read. Every `ROUTER` MUST store and return it ([`extensions/friend.md`](extensions/friend.md) §4).
- **`ALIAS`**: leads from another name of a file to its root: eD2K hash, torrent infohash,
  file-hoster link, Usenet post's articles, magnet-link hash. Kind of name is a string in the
  record, so a network joined later needs no new record kind. Every `ROUTER` MUST store and return
  it, for names of every kind ([`extensions/alias.md`](extensions/alias.md) §3).
- **`NAME`** and **`CLAIM`**: readable names. `NAME`: a name under a zone's root key, one record per
  name, signable only by that key or a key it delegated to; `CLAIM`: its publisher calls a zone by
  a global name, held like `ALIAS`. Every `ROUTER` MUST store and return both, whether or not it
  sets `NAMES` ([`extensions/name.md`](extensions/name.md) §3.2, §4.2).
- **`ANON`** and **`GATE`**: a node hiding its address behind other nodes. `ANON` names it as a file
  source by node ID alone; `GATE` says through which nodes it is reached. Both signed. A `SOURCE`
  search also returns node IDs of the target's `ANON` records, in `anon`, paged after `sources`
  (**Paging** above). A `Store` of a signed kind MAY carry `signer` (§10.7); the record is then
  checked under that key instead of the connection's. For `ANON` and `GATE` `signer` is required
  ([`extensions/hops.md`](extensions/hops.md) §4.3). Every `ROUTER` MUST do all of this, whether or
  not it sets `ANON` ([`extensions/hops.md`](extensions/hops.md) §4).
- A node not knowing a request's `kind` answers as non-responsible: a kind added after version 1,
  or one an index node without `ROUTER` does not store (§11).
- A non-responsible node ignores `search` and `store`, only returns contacts; a `store` gets
  contacts and `responsible = false`, never stored data. A requester MUST ignore `sources`,
  `files`, `notes`, `results` and every extension record field in an answer whose `responsible`
  is false.
- **The requester follows closer nodes.** Whenever an answer has contacts closer than the answerer,
  the requester MUST continue to them, whether or not it said `responsible`. Only the closest
  verified nodes hold the payload; a farther node's answer is a waypoint.
- **Source records are bound to the connection.** Storing node takes the source's node ID from the
  authenticated connection, and the endpoint of the connection's family as observed (address and
  port), replacing what `source` says for that family; the `Hello` port (§5.5) never enters a
  source record. `source` with non-empty `id` → `REJECTED`. Other family's endpoint is stored as
  claimed (§8.2 (5)); its address MUST be acceptable (§8.2 (7), and (8) in a named network), else
  `REJECTED`. `relays4` and `relays6`: at most 2 *(initial)* compact nodes each, at acceptable
  addresses, else `REJECTED`. So a node can publish only its own node ID, and source lists cannot
  be filled with invented peers; claimed endpoint and relays are addresses a searcher dials within
  the limits of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.5. A dial through a
  relay whose `Punch` is refused (`Error{DENIED}`, `Error{UNSUPPORTED}` from a non-relay, or
  `Error{NOT_FOUND}` for want of a reservation, §9.5) counts as a failed dial at the relay's
  address ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.5). A connection with no
  observed address, e.g. a session through a path ([`extensions/hops.md`](extensions/hops.md) §3),
  cannot store a source record: `REJECTED`; a hiding node publishes `ANON` records instead.
- **The searcher checks `sources`** by the same rules: drops a `Peer` whose `id` is not 16 bytes,
  whose endpoints or relays are not at acceptable addresses, with more than 2 relays in a family,
  or with neither endpoint nor relay.
- **Partial files are sources.** A node MAY store a `SOURCE` record for a file it holds in part;
  its `FileStatus.have` tells a downloader what it has (§12.1). `FileInfo.sources` counts complete
  sources, `FileInfo.partial` the others.
- **Source records are not signed.** Until dialling, a searcher has only the storing node's word.
  Dialling is the verification: the handshake proves the node at that address holds the key
  behind the record's node ID (T1); a source failing it is dropped. A signature adds nothing: it
  cannot prove an address belongs to a key. A false record can still cause a wasted dial, bounded
  by the dial limits of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.5.
- **Keyword and note records are signed** by their publisher (§10.7). Content is still a claim;
  the signature fixes who made it.
- Lifetimes *(initial)*: `SOURCE`, `ANON` and `FRIEND` 5 h, `GATE` 1 h, `KEYWORD` 24 h, `NOTE` 24 h, `MEMBER`, `ALIAS` and `CLAIM` 24 h, `LIVE` 10 min, `SUB` 60 min, `HEAD`, `HIDDEN`, `PART`
  and `NAME` up to 7 days. Republish before expiry; how long before is local policy where the kind
  names no interval.
- A `Store` holds at most 256 *(initial)* records, all record fields together; every record after
  the 256th is `REJECTED`, yet still counts as held: a `Store` whose only record of `kind` comes
  after the 256th is answered per record, not `Error{MALFORMED}`. Requests per minute accepted per
  source address and `kind`: local policy; beyond it a store gets `BUSY` for every record, a
  search `Error{BUSY}` with `retry_after`.
- **Keyword search.** Searcher splits its search text into words as in §10.1 (steps 1 to 3);
  derives target from the longest in UTF-8 bytes with 3 bytes or more (the first if equally long), from a
  2-byte word only if no longer one. Sends the others in `all`, and `all`, `none` and `ext`
  normalised as step 1 of §10.1; a storing node compares them as bytes with the name's words and
  extension. `ext` is sent and compared without the `.`. A word in `all` or `none` matches a name
  when it
  equals one of the name's words; no substring match. So a term = one word as steps 1–3 give it: a
  term with a separator inside, or an unspaced run of 3 or more, never matches; searcher splits
  its terms with steps 1 to 3 before sending. Search text without a keyword can't be
  searched. Searcher doesn't know names' extensions, and extension words aren't keywords
  (§10.1): a search text whose longest word is one (`mkv` in `ab.mkv`) finds nothing under that
  target.
- **Source records for many targets.** A `store` of kind `SOURCE` MAY name further targets in
  `also`. Each = store of the same record under that target: responsibility and limits checked
  per target; `results` has one entry for the request's target, then one per `also` target. Target the node isn't responsible for → `FAR`; every `also` target after its
  256th *(initial)* → `REJECTED`. `also` not n × 16 bytes makes a request of kind `SOURCE`
  `Error{MALFORMED}`. Other `kind`: `also` ignored, gets no entries. `nodes4`/`nodes6` refer to the
  request's target. Per-file fields (e.g. HTTP sources): request's target only. Publisher use: §10.4.
- `store` answered per record in `results`, ordered by `Store` field number, within a repeated
  field by its order; then one per `also` target. Record fields:
  `source` (1), `files` (2), `note` (3), `head` (5), `members` (7), `sealed` (9), `anon` (10),
  `aliases` (11), `names` (13). `http` (4) and `links` (12) belong to the `source` record, no
  entry; nor `signer` and `also` themselves. Shorter list reads `STORED` for missing
  entries, like an absent list; extra entries ignored. `sites` shorter than the list it counts
  reads 0 for missing entries. Record failing a check (§10.7) or not of `kind` →
  `REJECTED`; `Store` with no record for `kind` → `Error{MALFORMED}`; record not newer than the
  held one (§10.7) → `OLDER`. Keyword target contents, and what a publisher does when full: §10.8.

### 10.4 Lookup

1. Seed candidate set with closest contacts from the routing table.
2. Query the α = 3 *(initial)* closest not-yet-queried candidates, carrying the operation in
   every request.
3. Add returned contacts to candidates. A contact becomes verified when its connection
   completes (§6.3).
4. Stop when both hold:
   - the R closest verified candidates have all answered and no still-queryable candidate is
     closer to the target than the furthest of them. R = 4 for a search, 8 for a store
     *(initial)*. These R nodes = **closest set**, whether or not they said `responsible`; a
     store is judged over those that did, ignoring `BUSY` ones (§10.8). Every queried responsible
     node performs the operation, so more nodes than the closest set hold a store; it's still judged
     over the closest set;
   - at least M nodes answered in this lookup. M = 5 for a search, 9 for a store *(initial)*.
     Lookup without operation uses the search M.

   R and M count distinct node IDs: a dual-stack node's two entries are one node.

**Minimum walk.** While the first condition holds and the second not, requester queries the
closest not-yet-queried candidate, one at a time, though further from the target than the
closest set. Its returned contacts become candidates like any other; one closer than the
furthest of the closest set is followed as in step 2. Failed nodes don't count towards M. A
lookup with no candidate left to query ends with fewer than M answers.

- Without M a lookup ends once R nodes say they know nothing closer. Colluding nodes can say so
  of each other, and nodes that would name honest ones near the target are never asked.
- Filters of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.2 apply to these
  queries too. Candidate whose groups are full in the closest set is skipped: extra query goes to a node of
  another host, site or block than the nodes that ended the walk.
- M doesn't change the closest set. A node asked for this reason joins it only if among the R
  closest.

Queried node not answering within 5 s *(initial)*, counted from the first packet sent to it: failed for
this lookup, counts one failed attempt (§10.2). Node answering with an `Error`: failed for this
lookup too; doesn't count towards M, counts no failed attempt (it did answer), doesn't reset
the count (§10.2). Malformed `KadAnswer` (doesn't parse or breaks a size/layout rule, §0,
§10.3): failed for this lookup, counts one failed attempt. Lookup ends after 60 s *(initial)*
at latest, with the closest set it has by then.

Which contacts become candidates, which are queried and which R nodes form the final set: further
restricted by [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.2
and §2.3. A lookup that can't reach R nodes within those limits ends with fewer.

**Storing many source records.** A node sharing many files doesn't look up every root.

1. Sorts its targets.
2. Looks up the target whose deadline (below) comes first, store in the request as always.
   Lookup ends with a closest set.
3. To each node of that set sends one more request for the same target, with the following
   targets in `also`: those whose distance to the target is smaller than the target's distance
   to the set's furthest node, at most 256 *(initial)*, in sorted order. Next batch starts after
   the last of them.
4. Target that fewer than 4 *(initial)* nodes answered `STORED` gets its own lookup, unless the
   others answered `REJECTED` or `OLDER` (any other node would too). `BUSY` is transient: no
   reason for own lookup; target keeps its deadline (below). Otherwise go on with next target.

- **Deadline.** Each target is stored again at most 4 h *(initial)* after its last successful
  store, so every record is renewed before its 5 h end. Store succeeded when 4 nodes answered
  `STORED`, or the others `REJECTED` or `OLDER`, or its own lookup ended; a target held back
  only by `BUSY` has not. Lookups spread evenly over the 4 h: no burst. Batching is an
  optimisation: a target a batch stored has been stored; no deadline is passed to wait for a
  batch.
- A new file is stored at once, out of turn; its deadline runs from that store.
- Lookups per 4 h then bounded by network size, not file count. Estimate, IPFS comparison
  (method's origin):
  [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §2.1.

**Searching sources for many files.** No batch for searches; every target is looked up. Client
with many targets SHOULD take them in sorted order, and SHOULD add the closest set of the lookup
just ended to the next one's candidates (step 1). Where two targets share closest nodes, the
second lookup then asks them on still-open connections and needs one node more to reach M.
Each target is still its own request and counts against the answering node's limits on its own.
Why no batch: [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §4.8. For
set members a client asks the manifest's sources first
([`extensions/file-set.md`](extensions/file-set.md) §3).

### 10.5 What a lookup costs over QUIC, and what bounds it

New QUIC connection ≈ 2.5 KB (both Initial flights padded to 1,200 bytes) and one round trip; an
eMule Kad exchange is a few hundred bytes. Iterative walk can't be skipped: payload lives only
on the closest nodes, found via not-yet-close ones. Connections per lookup = distinct nodes on
the walk + responsible set; no protocol trick removes that. Design keeps every other cost off it:

| Measure | Effect |
|---|---|
| Operation rides in the lookup request | Responsible nodes contacted **once**. eMule Kad walks with `KADEMLIA2_REQ`, then sends search/publish to the same nodes again; over QUIC that second pass is free only while the connection is open; here it doesn't exist |
| 16 contacts per answer instead of 11 | Fewer hops. A contact costs 22 bytes, a hop a handshake |
| Few queries beyond the closest | Candidate queried only if among the α closest known, or one at a time while fewer than M answered (§10.4). Each further query costs 2.5 KB, so α stays small, not raised to hide timeouts; M is R + 1 |
| First hop is warm | Connections to routing-table contacts stay open while idle (30 s), then resume with 0-RTT: saves the round trip, not the padded Initial |
| One connection per node and family, many exchanges | Lookups for different targets via the same node share its connection; a client publishing/searching many files orders them by target so neighbouring walks overlap (§10.4) |
| Keyword records batched per node | All files sharing a keyword go in one `Store` |
| Source records batched per region | Targets sharing closest nodes go in one `Store` per node (§10.4) |
| Index nodes | Node logged in to an index node (§11) publishes/searches there over one connection; Kad for keywords only as fallback |
| Long record lifetimes | Publishing is the dominant Kad load; a source re-stored every 4 h, a keyword record before its 24 h end, not per search |

Expected cost of a lookup with operation: 3–4 hops, 8–12 nodes contacted (≥ M = 9 for a store),
5–9 of them new connections: ≈ 15–25 KB vs 3–5 KB in eMule Kad. Price of one authentication/anti-spoofing
mechanism instead of two, and of the verified addresses §6.3 needs. Minimum walk adds nothing:
such a lookup has > M answers before the closest set completes. M costs a query only in a lookup
that would otherwise end on its first R answers (its purpose) and on a network of few nodes.

### 10.6 Reserved: sessionless Kad datagrams

If live measurements show the cost above to be a problem, Kad requests can move to single
sealed datagrams, as eMule Kad and Ethereum's discv5 do. Version 1 reserves:

- capability bit 5 (`KAD_DGRAM`)
- message types 48–63
- a second `magic` value for the datagram format of §4.3
- field 8 of `Hello`

Such a binding must itself provide what QUIC provides here: proof of sender's key, proof of its
address, replay safety. Carries the same `KadRequest` and `KadAnswer`, except a request with
`store` isn't accepted by datagram and goes over QUIC. Specified as an extension in
[`extensions/kad-datagram.md`](extensions/kad-datagram.md); not part of version 1.

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

`kind` and `target` are the request's: signed without being sent, so a record can't be replayed
under another keyword or file. One kind signs, in place of `target`, a value not changing with
the day: `HIDDEN` ([`extensions/hidden-set.md`](extensions/hidden-set.md) §3).

**Signatures**

- Publisher signs with its identity key (§6.1). In a `Store` the key is the connection's, or
  `signer` if the `Store` carries it: a `Store` of any signed kind MAY carry `signer`; its
  records are then checked and counted under that key. So a follower re-stores another's head
  ([`extensions/file-set.md`](extensions/file-set.md) §5.2) and a last hop stores for a hiding
  node ([`extensions/hops.md`](extensions/hops.md) §4.2). Source records, bound to the
  connection, never use it: `signer` in a `Store` of `SOURCE`, of a kind stored like it (`LIVE`,
  `SUB`, §10.3), or of `FRIEND` is ignored. In a `KadAnswer` the key is `keys[key]`, sent once per
  publisher.
- `sigs` is a list from version 1 on. At most one signature per algorithm; a second of the same
  `alg`, or one with `alg` = 0, makes the record invalid.
- **Ed25519** per RFC 8032 §5.1.7 with every choice fixed, so all nodes agree on validity: signature is 64 bytes; `S` MUST be below the group order; encodings of key `A` and
  of `R` MUST be canonical; `A` and `R` MUST NOT be of small order; equation checked without
  cofactor, `[S]B = R + [k]A`. Equals `verify_strict` of ed25519-dalek plus a check that `A`'s
  encoding is canonical. Looser libraries need these checks in front: e.g. Go's
  `crypto/ed25519.Verify` accepts small-order `R` and `A` and a non-canonical `A`. Rejected
  forms are test vectors ([`tests/vectors-v1.md`](tests/vectors-v1.md)).
- Record is **valid** when every signature of an algorithm the verifier knows verifies and every
  *required* algorithm is present. Version 1 required set: Ed25519 alone; changes by network
  upgrade (§16).
- Unknown-algorithm signatures: ignored by verifier, kept by a storing node.
- A `Signed` MUST NOT exceed 1 KiB *(initial)*, counted over `body` and `sigs` as §0's
  canonical serialisation writes them after decoding, without `key`. Unknown fields, long varints
  don't count; re-encoding doesn't change the measure. Bounds what unknown signatures add.
- How a key of another algorithm ties to a node identity is **not defined** in version 1; the
  spec registering an algorithm must define it. Post-quantum signatures of today's sizes (2 KB
  and more) will need the reserved batch form.

**Checks**

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| Form | `body` parses as a `Record`; else record `REJECTED`. `key` ignored | same, and `key` is an index into `keys`; else record dropped |
| Signature | valid under the connection's key, or under `signer` | valid under `keys[key]` |
| `expires` | later than now, and at most the kind's lifetime plus 1 h *(initial)* ahead | later than now minus 1 h *(initial)* |
| Content | `file` for `KEYWORD`, `note` for `NOTE`, and no other of `Record` fields 1–3; extension fields (4 and up, §13.1) and unknown fields allowed and ignored for these kinds | same |
| Size | `file.name` 1 to 255 bytes UTF-8; `file.root` 32 bytes; `file.ed2k` empty or 16 bytes (§0); `note.rating` 0 to 5, `note.text` at most 512 *(initial)* bytes UTF-8; `Signed` at most 1 KiB *(initial)* as measured above | same |
| Keyword | one keyword of `file.name` (§10.1) gives the request's target at its `level` | one keyword of `file.name` gives the searched target |
| Count | one record per publisher and file under a keyword; one note per publisher and target (note carries no root: bound to `root[0..16)`, the target) | duplicates by publisher and file, or publisher and target for a note, merged |

Record failing a check on the storing node → `REJECTED` in `results` (§10.3). `Store.signer`
not 32 bytes makes a request of a kind using it `Error{MALFORMED}` (§0); for `SOURCE`, `LIVE`,
`SUB` and `FRIEND` ignored whatever its length.
**Newer.** Every signed kind keeps one record per publisher and file, target or root (over all
publishers a keyword file, a `MEMBER` set, an `ALIAS` root and a `CLAIM` zone keep at most 3,
§10.8 and their documents), except `HEAD`, `HIDDEN`, `GATE` and `NAME`: one record per target
whichever key signed it. A record replaces the held one only when newer: `HEAD`, `HIDDEN`,
`PART`, `GATE` and `NAME` have their own order, stated in their documents; for every other
signed kind the later `expires` is newer. Valid record not newer than the held one → `OLDER`,
nothing changes. Also holds for a record a third party re-stores with `signer`: an older copy
never replaces a newer one. `Error{MALFORMED}` is for a request that doesn't parse or breaks a
§10.3 rule (§5.6); a `Signed.body` that doesn't parse fails its record only. Searcher drops a
failing record. No unsigned keyword or note records. Storing node deletes a record at
`expires`.

Publisher's node ID, `H("ed3k v1 node id", key)[0..16)`, is the record's author: what a
client counts once, ignores or trusts. Why this helps against spam, and where not:
[`docs/signed-records.md`](docs/signed-records.md).

### 10.8 Keyword load

Some words occur in a large share of file names, and all records of one word go to the same few
nodes. Version 1 bounds that:

**Storing node**

- Keyword target holds at most 100,000 *(initial)* files: the **level size**. Several
  publishers' records for one file count once.
- Per file at most 3 *(initial)* records kept, the freshest of different publishers. Freshness
  = time the node last stored it, renewal included; `expires` (publisher-chosen) doesn't count.
  Least fresh = stored longest ago; of two stored in the same second, the one with lower publisher key as bytes.
- Site of a kept record = site ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1)
  of the address of the `Store` that last stored it, renewal included. For a `Store` with
  `signer`: the sending node's address, not publisher's.
- Valid record not `OLDER` (§10.7) takes a place by these rules, in order, answered `STORED`:
  1. Record of an already-kept publisher replaces that publisher's record, whatever site. Kept
     record then has this `Store`'s site.
  2. Else a record from a site holding a place replaces that site's record (the least fresh,
     should a renewal have left two of one site).
  3. Record from another site is added while fewer than 3 kept; else replaces the least fresh.
- `Store` with `signer` never replaces another publisher's record of another site: where rule 3
  would give it such a place → `REJECTED` (not `FULL`: target not full).
- So each site holds one place; can't keep two by renewing often.
- File target keeps at most 64 *(initial)* notes; note of a further publisher → `FULL`, held one
  renewed. At most 4 *(initial)* of them come from one publisher host address and 8 *(initial)* from one
  publisher site ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1); a further one
  from there → `REJECTED`.
- File target holds at most 10,000 *(initial)* `SOURCE` records; record of a further node →
  `FULL`, held one renewed. A `LIVE` or `SUB` target holds as many of its kind, same rule.
- Of the files under one keyword target, at most 1,000 *(initial)* from one publisher key and
  2,000 *(initial)* from one publisher site
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1). Record for a further file of
  that key or site → `REJECTED`. A file counts for a key and a site while a kept record of it
  has that publisher and site. Record keeping its file under the same publisher and site, or
  same site by rule 2, adds no file to that site; rule 2 adds one to the new publisher key; rule
  1 from another site adds one to that site. So a renewal by rule 1 from a site at its limit,
  holding no record of that file, is `REJECTED`; the kept record stays as it was, with its site,
  until it expires.
- `FULL` = target has no room. Record over a per-address or per-publisher limit → `REJECTED`:
  says nothing about the target, doesn't set `spill`, no reason for own lookup for the target
  (§10.4).
- While a target is full, a record for a file already held is renewed (`STORED`); one for a new
  file refused (`FULL`).
- `BUSY` = node as a whole out of room, or sender over its request rate (§10.3). Says nothing
  about the target.
- `load` and `spill` MUST be set, as §10.3 defines, in every answer of a responsible node to a
  request of kind `KEYWORD`, store answers and contacts-only answers included. Other kinds:
  `load` is 0; `spill` set when the node answered `FULL` for that target within the kind's last
  record lifetime. `sites` MAY be left at 0 for `KEYWORD` in version 1; for `MEMBER`, `ALIAS`
  and `CLAIM` it carries the count the answer is ordered by
  ([`extensions/file-set.md`](extensions/file-set.md) §6,
  [`extensions/alias.md`](extensions/alias.md) §3, [`extensions/name.md`](extensions/name.md)
  §4.2). That count covers sites of stores answered `STORED` for that root or set, each
  remembered until that record would have expired, also after displacement. A `Store` with
  `signer` adds no site. One site counts once; count saturates at 255 *(initial)*, as in
  [`extensions/keyword-load.md`](extensions/keyword-load.md) §2.1.

**Publisher**

- File published under the keywords §10.1 selects: by default those of 3 bytes and more, at
  most 40.
- Refused record offered again at next refresh, not earlier. No back-off longer than that.
- Store outcome judged over closest-set members (§10.4) that said `responsible`, ignoring those
  answering `BUSY`. Stored if one answered `STORED`, else refused.

**Searcher**

- `load` and `spill` tell searcher the results are part of what was published under that
  word.

**Levels.** Keyword target includes a level (§10.1); on Kad version 1 uses level 0 only: a node
MUST send `level = 0` to a Kad router and MUST NOT publish or search there at another level. At
an index node levels are part of version 1 (§11). Storing node treats a level 1 to 15 target
like any other keyword target, so upgraded publishers can use those levels on non-upgraded
nodes; answers a record with `level` above 15 with `REJECTED`. A later network upgrade lets a
full keyword spill to further levels, ranking files by publisher-site count so the most widely shared files
are at level 0. Every field it needs is defined above; capability bit 8 is
reserved for it. Rules and considered alternatives:
[`extensions/keyword-load.md`](extensions/keyword-load.md).

---

## 11. Index nodes

Index node = node with the `INDEX` capability: an eD2K server without a protocol of its own:

| eD2K server function | ed3k |
|---|---|
| Login, ID assignment, "your IP" | `Hello` (`observed`, §9.2). No HighID/LowID: a node is `open` or has relays |
| Offer files | `KadRequest{kind=KEYWORD, store}` and `{kind=SOURCE, store}` |
| Search | `KadRequest{kind=KEYWORD, search}` |
| Get sources | `KadRequest{kind=SOURCE, search}` |
| Callback request | `Punch` (an index node offering the relay service is a relay, §9.4; **Kad duty** below) |
| Server list | none on the wire; client keeps its own index node list, found as below, recognised by `Hello.caps` |

Differences from a Kad node: `responsible` for **every** target, also before adopting an
endpoint (§9.2, §10.3); never needs the iterative walk; MAY keep a `SOURCE` record past its
lifetime while the publishing connection stays open; MAY apply own admission policy with
`Error{DENIED}` or `{BUSY}`. Such a kept `SOURCE` record goes at connection close or lifetime
end, whichever is later; past its lifetime it yields to a further node's record before the
target answers `FULL` (§10.8). Kinds stored like `SOURCE` (`LIVE`, `SUB`, `FRIEND`) aren't kept
so: they end with their lifetime, since the short lifetime is what shows a node left a stream
or topic, or no longer wants to be found. Signed records (§10.7) expire as anywhere; publisher
renews them. Limits of §10.8, of [`extensions/ip-diversity.md`](extensions/ip-diversity.md)
§2.4 and each kind's per-target limits bind an index node as any storing node, unless its own
policy is stricter. Sole exception: level size (§10.8), operator-chosen. Until a keyword target
holds that many files, no `FULL` for it; from then on `FULL` and `spill` per §10.8.

**Keyword levels.** Part of version 1 at an index node, no capability bit. Index node MAY rank
and keep leaving entries per [`extensions/keyword-load.md`](extensions/keyword-load.md) §2; one
that doesn't refuses when full, per §10.8. Client publishing/searching there follows that
document's §3 and §4 by that node's answers: moves down on `FULL`, follows `spill`. Each level is its
own target there. Level remembered per (file, keyword) = that of the index node in use; Kad
answers never move it; moving to another index node restarts at level 0. On Kad, levels wait
for the network upgrade (§10.8).

**Limits over all targets.** Those limits are per target, and an index node holds every target;
keys, names, roots cost nothing, so per-target limits don't bound what one publisher can make
it hold. So an index node MUST also bound records held per publisher host address and per
publisher site ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1), over all
targets and kinds together. The bound is local policy. A record beyond it is `REJECTED`, not
`BUSY`: the node has room.

**Kad duty.** Index node normally open, often by operator's word (§9.2); MAY leave `ROUTER`
unset (§2), and is then no Kad router:

- Not in routing tables (§10.2), keeps no table, answers without contacts. Need not dial at all: on a connection it accepted every datagram is masked with its own `obf_id`
  (§4.3), and as A of a `ReachTest` it answers `ok = false` (§9.3).
- Stores record kinds of its choice; answers other kinds as a non-responsible node (§10.3), so
  clients fall back to Kad for those.
- It SHOULD still offer the relay service (§9.4), the eD2K server callback. If it does, it sets
  `RELAY` while open and with a free slot.

An index node setting `ROUTER` has every duty of one, also storing every record kind.

**Keepalive.** Index node holds many idle connections, keeps none for its own sake. Sends
`max_idle_timeout` = 300 s *(initial)* and no PING of its own. Client dialing a node known as
index node (below) sends 300 s too, since QUIC takes the smaller of the two. Client wanting such a
connection kept PINGs every 60 s *(initial)* while holding a relay reservation there or not
`open` in the connection's family (so its NAT keeps the mapping), else every 240 s
*(initial)*; each interval randomly varied by up to 10 % *(initial)*. Mapping lapsing between
PINGs is mended by the next PING via client-side migration (§4.1); a `Punch` lost in the gap is
retried (§9.5).

**Load** *(informative)*. Signed records cost an index node one strict Ed25519 verification each
(§10.7). A client sharing 10,000 files under ~5 keywords each sends some 50,000 a day, a few
seconds of one core; 50,000 such clients ≈ 1.5 cores. Batch verification doesn't help: it
checks the equation with the cofactor, which §10.7 doesn't. A node MAY answer a `Signed`
byte-identical to the one it holds `OLDER` without re-verifying.

Index nodes found from a configured list, a link (§13.2) or an eD2K server advertising
one ([`docs/ed2k-tags.md`](docs/ed2k-tags.md)). Index nodes are optional for the network and for
a client. An eD2K server running an index node MAY list some of its ed3k sources on eD2K too
([`docs/ed2k-tags.md`](docs/ed2k-tags.md) §6).

**One index node at a time.** A client SHOULD publish and search at one index node of its list
at a time, over one connection, moving to another of the list when it's unreachable. Records
sent to every listed index node multiply each one's load (**Load**). A client MAY also hold a
relay reservation (§9.4) at a second listed index node, and opens no other exchange there.

- **Redial.** No redial of an index node sooner than 30 s *(initial)* after the connection
  ended or a dial failed; wait doubles after each failed try, up to 10 min *(initial)*. A
  connection open 60 s *(initial)* resets the wait to 30 s; one ending sooner, even after a
  completed handshake, is a failed try. A longer `retry_after`
  (§5.6) wins. That keeps it well within the budget of §4.6.
- **Moving.** Client moving to another index node stores its records there at once; the old one
  keeps them until their lifetimes end (above). After redialing the same index node nothing is
  re-stored: records outlive the connection, renewed before lifetime end as anywhere (§10.4); a
  relay lost with the connection is replaced per §9.4.
- **What a searcher sees.** Index node answers with what was published there, like an eD2K
  server without global search; Kad has what was published in Kad. A searcher MAY also ask
  up to 3 *(initial)* other index nodes of its list per search, over connections it doesn't
  hold: sends no PING on them (**Keepalive**).

An index node stores and answers records. A service with its own catalogue (e.g. video site)
isn't thereby an index node; it runs a node beside it, which may be one
([`extensions/chaintube.md`](extensions/chaintube.md) §5).

---

## 12. File transfer

### 12.1 The file stream

Downloader opens one exchange per uploader for the file it wants from it; everything about that
file between the two peers happens on it. File named once, in the first message. Stream stays
open during a transfer; while downloader is queued, uploader decides whether it stays
open (§12.5).

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

- First `FileRequest` → full `FileStatus`, or `Error{NOT_FOUND}`. Full = `size`, `have`,
  `rank`, `recheck` present, plus `name`, `ed2k`, `note` as far as uploader knows them and
  `brief` didn't omit them.
- `root` is 32 bytes (§0) and MUST be present in a stream's first `FileRequest`; a first one
  without it → `Error{MALFORMED}`; `root` in a later `FileRequest` is ignored. `have` bits beyond the
  file's last piece ignored, in both messages. Of `relays4` and `relays6` uploader uses the
  first 2 *(initial)* compact nodes of each, as a source record holds (§10.3); rest ignored.
- **The size is a claim.** Downloader with a size differing from uploader's
  `FileStatus.size` doesn't use that source with the hash values it holds: two sizes, two
  trees (§12.3).
- Later `FileStatus` messages carry only changed fields: `have`, `rank`, `recheck` are
  `optional`, absent = unchanged (§0); `size`, `name`, `ed2k` sent once. When sent: limited by
  §12.5. Same for `have` in a later `FileRequest`.
- Downloader sends a new `FileRequest` to update `have` or re-ask for sources. A later
  `FileRequest` is answered only when its `want_sources` is above 0 (with a `FileStatus`
  carrying `sources` alone, at most once per 10 min *(initial)*), or when it asks for a set's
  member bitmap ([`extensions/file-set.md`](extensions/file-set.md) §4.1); any other isn't.
- Either side ends by closing the exchange. No cancel message.
- An `Error` on a file stream ends it: its sender closes the stream. Exceptions: `NOT_FOUND`
  and `BUSY` in answer to a `HashRequest` (§12.4).
- Sources in `FileStatus` MUST be peers uploader itself had a verified connection with for this
  file in the last hour *(initial)*.

These two messages replace `OP_HELLO`'s file fields, `OP_REQUESTFILENAME`, `OP_SETREQFILEID`,
`OP_FILESTATUS`, the three multipacket variants and their two answers, `OP_STARTUPLOADREQ`,
`OP_ACCEPTUPLOADREQ` (with `SlotOffer`), `OP_QUEUERANKING`, `OP_REQUESTSOURCES2`/`OP_ANSWERSOURCES2`,
`OP_REASKFILEPING`/`OP_REASKACK`, `OP_FILEDESC`, `OP_CANCELTRANSFER` and `OP_END_OF_DOWNLOAD`.
No large-file variant; every offset is 64 bits.

Queueing, scoring, credits: local policy as in eMule; credits keyed by node ID. Slot duration
and count: §12.5. Credits counting at every uploader, and slot bids: optional extension (§13.1).

### 12.2 `BlockRequest`

```protobuf
message BlockRequest {
  repeated uint64 offset = 1; // packed; each a multiple of 256 KiB
  repeated uint32 length = 2; // packed, same count; the length of the block at that offset
  bool   raw = 3;             // send these ranges as they are, never compressed (§12.3)
}
```

Allowed only while `rank = 0`; up to 16 *(initial)* ranges may be outstanding. A
`BlockRequest` while `rank` isn't 0, or leaving more than 16 outstanding, isn't served:
uploader answers `Error{DENIED}`, closes the file stream. Queue entry kept 30 min as after a lost connection
(§12.5).

- **A range is one block.** `offset` a multiple of 256 KiB; `length` that block's length:
  256 KiB, or the rest of the file. `offset` and `length` have equal entry counts, every
  `offset` is below file size, each block named once. A request breaking this, or naming an
  outstanding block → `Error{MALFORMED}`, stream closed; queue entry kept 30 min, as after
  `DENIED`.
- A block of a piece uploader doesn't hold isn't sent; it sends a `FileStatus` with its
  `have` at once, outside the 1 per 5 min limit (§12.5), and the range is no longer
  outstanding. An uploader unable to read a block of a piece its `have` holds first clears that
  piece from `have`, then does the same; an unchanged `have` would leave the range outstanding.
- **When a slot is taken away** (`FileStatus` with `rank` above 0), all outstanding ranges are
  void on both sides. That `FileStatus` MUST carry `recheck`. Uploader finishes at most the
  `Block` it is sending; frames don't interleave on an exchange (§5.1), so that `Block` arrives
  before the `FileStatus`, its range still outstanding, and is used as any other. A
  `Block` after that `FileStatus` is a protocol fault (§12.3). With `recheck` above 0 the
  downloader doesn't answer, and the stream closes (§12.5). With `recheck` = 0 downloader
  answers with an empty `FileRequest`, and uploader ignores every `BlockRequest` until it
  arrives: one that crossed the `FileStatus` is neither served nor an offence. With a new slot
  ranges are counted from zero.

### 12.3 `Block`

```protobuf
message Block {
  uint64 offset = 1;
  uint32 size = 2;            // number of bytes that follow this frame; the range length when not compressed; at most 256 KiB
  uint32 plain = 3;           // set when those bytes are zstd: length of the file data they hold
}
```

Frame is followed by exactly `size` bytes, outside the protobuf body, so a block is never
copied through the serializer. Without `plain` they are raw file data. Per-block overhead ≈ 12
bytes on 256 KiB.

**Compressed blocks.** Most shared files (compressed media, archives) gain nothing; text,
databases, disk images, file set manifests gain a lot. So a sender MAY compress a
block, and every node MUST be able to receive one.

- **One `Block` per range.** A `Block` answers exactly one range of a `BlockRequest`, with its
  `offset` and length. Uploader MUST NOT split or merge ranges. Compression covers that
  `Block`'s bytes alone. A `Block` whose `offset` matches no outstanding range → protocol fault (below).
- **Send order.** Uploader SHOULD send outstanding ranges in order asked: earlier
  `BlockRequest`'s before later, within one request by `offset` entry order. So downloader
  decides what comes first (§12.6). A `Block` out of that order is no protocol fault.
- Profile: one magicless zstd frame per `Block` as in §5.3, no dictionary. Each range compressed
  alone, so blocks can be asked for in any order from any source. Frame naming a dictionary →
  protocol fault (below).
- `plain` = block length after decompression, at most 256 KiB. Receiver stops when output would
  exceed `plain`. Output of another length, or undecodable input → protocol fault (below).
- **Length.** `plain` (compressed) or `size` (not compressed) MUST equal the length of the range
  the `Block` answers, and `size` MUST NOT exceed 256 KiB. Breaking either → protocol fault
  (below).
- Hashes are over file data, never compressed bytes (§7.2).
- **When to compress.** Compressed only if it saves at least 1/8 of the length *(initial)*. After
  a block that didn't, a sender SHOULD NOT try the next 15 blocks *(initial)* of that file;
  blocks asked for with `raw` count among the 15. It MAY remember the result per file (a file
  never changes) and MAY skip, untried, files it expects to gain nothing.
- No file type list: a name isn't part of a file's identity (§7.1), need not match content;
  measured saving decides.
- A turn's data (§12.5) is counted in file bytes, so a turn is 4 pieces whether or not
  compressed.
- **What it shows.** Compressed block sizes depend on content: an observer with a copy can
  compress it the same way and compare sizes with a transfer it sees; encryption doesn't hide
  them. Uncompressed blocks all look alike. So either side can turn compression off:
  a sender MAY be set never to compress, and a sender MUST NOT compress a range asked for with
  `BlockRequest.raw`.

Codecs and alternatives compared in
[`docs/compression-comparison.md`](docs/compression-comparison.md) §5.

**A protocol fault** is a `Block` downloader refuses without a hash: wrong length,
undecodable bytes, an `offset` not asked for. Downloader reads no further, closes the stream,
doesn't use the node as a source of that file for 4 h *(initial)*. No `BadBlock`; fault counts
towards no ban. Only a block of the right form whose hash fails is a **bad
block**.

**A block that fails.** Each block is verified alone (§7.2) and a range is one block (§12.2), so
a bad one's sender is known exactly. Downloader discards it and asks another source.

| Bad blocks from one node ID | What downloader does |
|---|---|
| 1 | MUST NOT use the node as a source for that file for 4 h *(initial)* |
| 2 within 24 h *(initial)*, in any files | MUST ban the node ID for 24 h *(initial)*, and the host address the second one came from for 1 h *(initial)* |
| 1 more within 30 days *(initial)* after a ban ended | MUST ban the node ID for 30 days *(initial)*, and the host address for 1 h *(initial)* |

- **A ban of the node ID is that of §4.6:** open connections closed, a handshake of that node ID
  closed once the key is known, nothing downloaded from or uploaded to it. **A ban of the host
  address** closes only the banned node ID's connections; for 1 h it drops datagrams from the
  address that would start a new flow, so no new connection comes from there. Other node IDs' connections from the address stay open. Host address = IPv4 /32 or
  IPv6 /64; private addresses exempt where §4.6 exempts them.
- **One bad block is not a ban.** Disk/memory faults at honest nodes give one; two in a day = a
  node that shouldn't serve.
- **No ban is for ever.** Node IDs cost nothing to replace: a permanent ban would stop no attacker,
  would only keep out a since-repaired node. Address ban is short since several nodes share an
  address behind NAT; it exists so a new key from that address gains nothing for that hour.
- A block counts only when the chaining value it was checked against was itself verified under
  the root (§12.4). Then the fault is the block's, and its sender certain.
- Verified values can still belong to a false size: a shorter file's tree is part of the true
  one. So a bad block counts against its sender only once one block of the file passed under
  that size. Before that downloader drops size and values, counts nothing, sends no `BadBlock`, uses neither the
  block's sender nor the sources of hash values under that size as sources of that file
  for 4 h *(initial)*.
- Counts and bans are kept across restarts, counted per node ID, and local: nothing reported to
  other nodes.
- Non-node sources follow their extension's rules (§13.1). eD2K sources follow §14.4: a block of
  eD2K data failing under the BLAKE3 tree marks the (`root`, `ed2k`) pair false and counts
  against no peer.

**Telling the sender.** Downloader tells sender once per bad block, on the file stream, before closing stream or
connection. If the block leads to a ban, `BadBlock` is sent, then the ban closes the
connection, so it may not arrive:

```protobuf
message BadBlock {            // type 38, downloader → uploader, on the file stream
  uint64 offset = 1;          // start of the block that failed, in the file of the stream
}
```

- Not answered.
- The sender MUST verify that block of its own copy before serving it again, to anyone. If it
  fails, its piece is no longer complete: sender clears its `have` bit and re-fetches the
  piece, or stops sharing the file.
- If it passes, sender does nothing more. A `BadBlock` proves nothing; a node MUST NOT ban a
  peer because of one.
- Sender acts on a `BadBlock` only for a block it sent to that peer on that stream in the last
  10 min *(initial)*, on at most one per block and hour and 16 per peer and day *(initial)*;
  rest ignored. Each costs a read and a hash of 256 KiB.
- This is how an honest node with a damaged file finds out and stops collecting bans.

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

`cvs` = BLAKE3 chaining values of blocks `first .. first+count−1`, in order; a block is 256 KiB,
except the file's last, which may be shorter. `count` is at most 32,000 *(initial)*, so `cvs`, a
proof of any depth and the frame fit §5.2's 1 MiB; a downloader asks for more in several
requests. `proof` holds the sibling values needed to recompute the root from that range.

**The tree.** Let n = ceil(size / 256 KiB) = number of blocks. Level 0 holds their n chaining
values. Each level above halves the count:

```
count(0) = n                    count(k) = ceil(count(k−1) / 2)
node(k, j) = parent( node(k−1, 2j), node(k−1, 2j+1) )     if both exist
           = node(k−1, 2j)                                 if it is the last and has no partner
```

`parent` = BLAKE3's parent node function. The one-node level is the root, level L; finalised
with the `ROOT` flag it equals the file's root; every other parent computed without that flag.
This is BLAKE3's own tree (§7.2): a partnerless node is carried up unchanged, putting the
largest power of two on the left. A block's chaining value is
that of its subtree inside the file: its chunks keep their file position as BLAKE3's chunk
counter, so block i starts at chunk 256 i.

**The proof.** The range covers nodes `a(k) .. b(k)` of level k, with `a(0) = first`,
`b(0) = first + count − 1`, `a(k) = floor(a(k−1) / 2)` and `b(k) = floor(b(k−1) / 2)`. For
k = 0, 1, … up to the level below the root, `proof` holds, in this order:

- `node(k, a(k) − 1)` when `a(k)` is odd (left sibling of the range's first node),
- `node(k, b(k) + 1)` when `b(k)` is even and that node exists (right sibling of its last).

A level the range spans whole contributes nothing. Receiver recomputes level by level to the
root, compares with `root`. Request for all blocks → empty proof. A one-block file
(n = 1) needs no hash exchange: its root is the block's own output finalised as root (§7.2). A
file of size 0 has the root of the empty input, no block and no hash exchange; complete once
created. `HashRequest` → `Error{MALFORMED}` when `count` is 0, `first + count` exceeds n,
`count` exceeds its limit, or n is at most 1.

- `HashRequest` sent on the file stream, at any `rank`, one at a time. Unanswered within 30 s
  *(initial)* → failed; downloader asks another source for those values. One arriving while
  one is unanswered → `Error{BUSY}` (doesn't close the stream); a downloader whose request timed
  out sends no further `HashRequest` on that stream.
- Uploader answers at most 2 n values per node ID and file within 24 h *(initial)*, all streams
  together; `Error{BUSY}` beyond. A new stream doesn't renew the count. Values not held (as a
  partial source may not) → `Error{NOT_FOUND}`. Neither closes the stream.
- A `HashAnswer` whose `cvs` or `proof` length differs from what the request calls for, or not
  leading to the root: discarded; downloader closes the stream, doesn't use the node as a
  source of that file for 4 h *(initial)*. Not a bad block; counts towards no ban.

### 12.5 Waiting in the queue

An uploader's queue can hold thousands of downloaders; a connection and update stream each
would cost far more than eMule's reask, so uploader sets how each waits.

**One entry per downloader.** One queue entry per node ID, for one file, across all of the
downloader's downloads. A downloader has at most one file stream open to an uploader; several
files wanted from one uploader are asked for in turn, within that one entry. A file stream for another file replaces the request,
keeping the position, or the slot if granted ("Several files in one turn" below). Uploader
closes the old stream only after answering the new one's first `FileRequest` with a
`FileStatus`, first finishing the `Block` it is sending on the old one. Old stream's outstanding
ranges are then void; ranges count from 0 on the new one. A `Block` cut short on a replaced
stream isn't a protocol fault (§12.3). If uploader answers the new stream with an `Error`
(`NOT_FOUND`, or `MALFORMED` for a bad bid), the entry stays as it was, with the old file and
its stream.

**Stay or come back.** The first `FileStatus` to a queued downloader carries `recheck`; a later
one carries it when it changes:

| `recheck` | Meaning |
|---|---|
| 0 | Stay connected. Uploader expects to serve this downloader soon |
| 1,800 to 14,400 | Close the file stream and open a new one after this many seconds |

A value from 1 to 1,799 reads as 1,800, above 14,400 as 14,400.

- Uploader chooses the value per downloader, shorter near the queue front. How many downloaders
  it keeps connected is local policy.
- **The uploader closes the stream** after a `FileStatus` with `recheck` above 0, and the
  connection if unused otherwise. Uploader selling priority first waits up to 30 s *(initial)* for
  the receipt of a bidding entry
  ([`extensions/credit.md`](extensions/credit.md) §4.1). A downloader cannot stay connected
  against uploader's choice.
- Downloader comes back by opening a new file stream; `brief` keeps the answer small.
- It MAY come back earlier to report changed endpoints or relays (what uploader dials by,
  below), or ask for another file (keeps position, as in eMule); these reasons and a changed
  bid ([`extensions/credit.md`](extensions/credit.md) §4.1) together at most once per
  30 min *(initial)*. Any other early `FileRequest` MAY be answered with `Error{BUSY}` and
  `retry_after`. Position not affected.
- Entry kept until 30 min *(initial)* after the come-back time it was told, then removed.

**While connected.** Uploader sends a `FileStatus` only

- when a slot is granted (`rank` = 0) or taken away (`rank` above 0),
- to change `recheck`,
- while downloader holds a slot, for a changed `have`, at most once per 5 min *(initial)*,
- at once, outside that limit, for a block of a piece it doesn't hold (§12.2),
- every 30 min *(initial)* as a **ping**: `FileStatus` with `ping` set, and with `rank` and
  `have` if changed,
- in answer to a `FileRequest` asking for sources (§12.1),
- in a priority turn of the credit extension, for a changed `price` or `due`
  ([`extensions/credit.md`](extensions/credit.md) §3.2).

Downloader answers a `FileStatus` with `ping` set, and no other except the one taking a slot
away with `recheck` of 0 (§12.2), with a `FileRequest` within 30 s *(initial)*, empty unless its
own `have` changed. If not, or the connection is lost, uploader closes the stream; entry then
kept 30 min *(initial)* for downloader to come back, then removed.

**A free slot.** Uploader contacts the next downloader at once.

- Connected: `FileStatus` with `rank` = 0 on the open stream.
- Not connected: uploader dials downloader at the observed endpoint of its last connection,
  and at a claimed endpoint (§8.2 (5)) only if acceptable (§8.2 (7), and (8) in a named
  network), or by `Punch` through the relays its `FileRequest`s last named (§9.5); of `relays4`
  and `relays6` it uses only those at acceptable addresses, as for a source record (§10.3). Opens
  an exchange, sends `SlotOffer`. Downloader answers there with a first `FileRequest` naming a
  file by `root`: the queued one, or another of this uploader's it now wants (slot belongs to
  downloader, below). Uploader answers with a full `FileStatus`, `rank` = 0; exchange
  continues as a file stream with the slot granted. Downloader wanting nothing more of this
  uploader answers `Error{NOT_FOUND}`; entry removed. It answers `Error{DENIED}` to an offer
  from a node it isn't queued at; that entry removed too.
- A downloader with its own file stream open to uploader when the offer arrives keeps it:
  closes the offer exchange unanswered; uploader grants the slot on the open stream. An offer
  exchange ending unanswered while such a stream is open is read so, not as a failed offer.
- Uploader waits 30 s *(initial)* from the start of the dial; then the slot goes to the next
  downloader.
- An unreachable downloader keeps its position and is offered the next free slot. After a
  second failure it gets no further offers.
- In both cases the entry stays until its normal end, 30 min after the come-back time it was told. An entry told to stay connected (`recheck` = 0) has no such time; it ends
  30 min *(initial)* after the failed offer. A `FileRequest` from downloader before then
  clears the failures.

**Length of a slot.** A slot is one turn at an uploader.

- An uploader SHOULD transfer 16 MiB *(initial)* (4 pieces) to a downloader before taking the
  slot away, as long as downloader keeps requesting.
- An uploader SHOULD take the slot away after 1 h *(initial)* even if less was sent, so a
  low-bandwidth downloader doesn't hold it. Bytes count from the `FileStatus` that granted the
  slot.
- After 16 MiB, or after 1 h, uploader MAY go on while nobody else is waiting.
- A slot with no outstanding range for 30 s *(initial)* is taken away.
- A slot outlives its stream only by a hold: when downloader closes or resets the stream
  holding its slot, or uploader answers the `FileRequest` on a `SlotOffer` exchange with
  `NOT_FOUND`, uploader holds the slot 30 s *(initial)* for a new file stream, then takes it
  away. A hold isn't restarted while it runs, and the no-outstanding-range rule doesn't apply
  during it; the turn's bytes and time go on counting. A slot whose connection is lost is taken
  away at once.
- A downloader losing its slot stays queued. New position is local policy; the `FileStatus`
  taking the slot carries it as `rank` ≥ 1. Where none was chosen (e.g. slot taken for no
  outstanding range), `rank` = last one above 0 downloader was given, else 1.

**Several files in one turn.** A slot belongs to downloader, not the file.

- A downloader holding a slot (`rank` = 0) MAY open a file stream for another file. It replaces
  the open one, and uploader MUST keep the slot: the turn's bytes and time go on counting
  across files. Sole exception: a short slot for a member of a website
  ([`extensions/name.md`](extensions/name.md) §6.4).
- Uploader answers the new `FileRequest` with `rank` = 0, or `Error{NOT_FOUND}`; after
  `NOT_FOUND` the slot stays on the old stream, or is held as above when there is none.
- Downloader SHOULD set `brief` when it knows size and name.
- A turn stays 16 MiB or 1 h, so several files give no more than one.

**Number of slots.** An uploader SHOULD choose its upload slot count so 16 MiB take 5 to 30 min
*(initial)* in each, ~9 to 55 KiB/s per slot. It SHOULD offer at least 2 *(initial)* slots;
below ~18 KiB/s of upload that minimum comes first and a turn takes longer.

Comparison with eMule's reask and alternatives:
[`docs/queue-status-comparison.md`](docs/queue-status-comparison.md).

### 12.6 Playing while downloading *(informative)*

A client MAY play a video or sound while downloading it, and seek anywhere. Wire needs nothing
more: any block from any source in any order (§12.2), each block verified alone (§7.2),
uploader sends in the order asked (§12.3). Left: downloader's block choice, which stays local. A playing client SHOULD choose as follows.

- **Only verified bytes are played.** A block goes to the decoder after passing under the root
  (§12.3). For a seek the client asks for the hash values of blocks around the new position,
  with their proof (§12.4), not the whole file's. `HashRequest` is allowed at any `rank`, so
  this works while queued.
- **The index first.** Many containers keep their index at file end: MP4 with `moov` last,
  Matroska cues, AVI `idx1`. Client fetches the first block and index blocks before playing, to
  map time to offset.
- **A window ahead of the play position.** Blocks of the next ~30 s of media to be played are
  asked for in file order, before any other block. Media rate from the container or
  `FileInfo.bitrate` (§5.4). Blocks outside the window chosen as for any download, rare pieces
  first, so a swarm of viewers doesn't hold only the starts of a file.
- **Few ranges outstanding per source.** Ranges already asked for are sent before later ones
  (§12.3). So a playing client keeps only enough outstanding per source to keep it busy, 2
  rather than 16, and asks its fastest sources for the window.
- **After a seek** the window moves. Its blocks go to the sources with fewest ranges
  outstanding. The window's first block MAY be asked for from two sources at once; the copy
  arriving second is dropped. Ranges still outstanding for the old position are taken as they
  arrive.
- **Keeping the slots.** A slot with no outstanding range for 30 s is taken away (§12.5); a
  paused client keeps downloading outside the window, keeping its slots.
- **What limits it.** Playback needs as many slots as media rate / one slot's rate. At 9 to
  55 KiB/s per slot (§12.5), 1080p at 5 Mbit/s needs some 11 to 70 sources at once. A seek waits
  at least for one 256 KiB block from one source, 5 to 28 s at those rates. Before holding slots
  a client is queued as for any download; HTTP sources
  ([`extensions/http-source.md`](extensions/http-source.md)), links at hosters
  ([`extensions/hoster.md`](extensions/hoster.md)), local-network peers and the priority slots
  of [`extensions/credit.md`](extensions/credit.md) §5 start playback sooner.
- No message takes back outstanding ranges, to keep the protocol small; few ranges
  outstanding per source (above) serves the same end.

A media layout made for playing, fragmented MP4 with its index at the start:
[`extensions/chaintube.md`](extensions/chaintube.md) §2.2.

---

## 13. Other peer messages

### 13.1 Optional messages

Optional, each announced by a capability bit, none required: browsing a peer's shared files, chat,
rooms, HTTP sources, file sets, live streams, publish and subscribe, finding nodes in the local
network. Optional w/o bit of their own (adding file formats, links, four record kinds): hidden
sets, sets with several authors, other names of a file, torrents and Usenet posts as file sets,
videos named on a chain, shared filter lists, first contact, finding a known node. Readable names
for keys (websites among things named) are optional: two record kinds, a link, a bit. Links at file hosters are
a further source kind, with own bit.

**Other networks and data sources are joined under the root, never beside it.** An outside name
leads to a root through one record kind, `ALIAS`; rules for every such extension:
[`extensions/alias.md`](extensions/alias.md) §5.
Also optional: anonymous paths (other nodes between a node and its peers); credits counting at
every uploader (need a ledger outside ed3k); paid subscriptions, giving each viewer of a channel
or set own access for the time paid (ed3k carries passes and keys, not payment).
A video site with its catalogue on a chain keeps media in ed3k as file sets; the site is a
separate program next to an ordinary node, no further role (§11).

| Extension | Capability | Wire elements | Document |
|---|---|---|---|
| Browse | bit 3, `BROWSE` | message types 14, 15 | [`extensions/browse.md`](extensions/browse.md) |
| Chat | bit 4, `CHAT` | message types 16, 18 | [`extensions/chat.md`](extensions/chat.md) |
| Rooms | bit 19, `ROOM` | message types 42, 43; `ed3k:room` links; uses record kind 7 | [`extensions/room.md`](extensions/room.md) |
| HTTP sources | bit 9, `HTTP` | fields in `FileRequest`, `FileStatus`, `Store`, `KadAnswer`; `ws`, `wh` in links; `ed3k:cache` links | [`extensions/http-source.md`](extensions/http-source.md) |
| File sets | bit 10, `SETS` | manifest file format; record kinds 4, 5; fields in `FileInfo`, `FileRequest`, `FileStatus`, `Record`, `Store`, `KadAnswer`; `path` in links; `ed3k:set` links | [`extensions/file-set.md`](extensions/file-set.md) |
| Live streams | bit 11, `LIVE` | message types 19 to 21; record kind 6; channel file format; `ed3k:live` links | [`extensions/live.md`](extensions/live.md) |
| Publish and subscribe | bit 12, `PUBSUB` | message types 22, 23; record kind 7; field in `Record` | [`extensions/pubsub.md`](extensions/pubsub.md) |
| Finding nodes in the local network | bit 13, `LAN` | message types 24, 25; multicast datagram outside any connection | [`extensions/lan-discovery.md`](extensions/lan-discovery.md) |
| Anonymous paths | bits 14 and 15, `ANON` and `HOP` | message types 26 to 34 and 39; record kinds 11, 12; field in `Record` and `Store`, two in `KadAnswer` | [`extensions/hops.md`](extensions/hops.md) |
| Credits | bit 16, `CREDIT` | message types 35 to 37; field in `FileRequest` and in `FileStatus`; ledger on own chain, via RPC | [`extensions/credit.md`](extensions/credit.md) |
| Paid subscriptions | bit 18, `PAID` | message types 40, 41; fields in `Channel`, `LiveSegment`, `LiveJoin`, `LiveState`, `FileSet`, `Member`, `Sealed`, `TopicJoin`, `TopicState`, `TopicItem`; `paid` in `ed3k:live`, `ed3k:set` links; `ed3k:pass` links | [`extensions/subscription.md`](extensions/subscription.md) |
| Hidden sets | none (`SETS`) | record kind 8; field in `Record` and in manifest; `s` in `ed3k:set` links | [`extensions/hidden-set.md`](extensions/hidden-set.md) |
| Sets with several authors | none (`SETS`) | record kind 9; two fields in `Record`; fields in manifest | [`extensions/multi-author-set.md`](extensions/multi-author-set.md) |
| Links at file hosters | bit 17, `LINKS` | field in `FileStatus`, `Store`, `KadAnswer`; three result codes; field in manifest; `hl` in `ed3k:file` links; two schemes of `ALIAS` | [`extensions/hoster.md`](extensions/hoster.md) |
| Other names of a file | none | record kind 13; field in `Record`, `Store`, `KadAnswer` | [`extensions/alias.md`](extensions/alias.md) |
| Torrents as file sets | none (`SETS`) | two schemes of `ALIAS`; field in manifest; `bt` in `ed3k:file` links | [`extensions/torrent.md`](extensions/torrent.md) |
| Usenet posts as file sets | none (`SETS`) | two schemes of `ALIAS`; field in manifest; article list file format | [`extensions/usenet.md`](extensions/usenet.md) |
| Videos named on a chain | none (`SETS`) | field in manifest; `b3`, `sz`, `ipfs` in `ed3k:set` links, `ipfs` in `ed3k:file` links; set layout an IPFS gateway or web server can serve as is; `ed3k:set`, `ed3k:file` links for attachments of other entries; contracts on own chain, via RPC | [`extensions/chaintube.md`](extensions/chaintube.md) |
| Shared filter lists | none | list file format | [`extensions/filter-list.md`](extensions/filter-list.md) |
| First contact | none | node list file format; `ed3k:nodes` links | [`extensions/bootstrap.md`](extensions/bootstrap.md) |
| Finding a known node | none | record kind 10; field in `Store` and in `KadAnswer`; `ed3k:friend` links | [`extensions/friend.md`](extensions/friend.md) |
| Readable names | bit 20, `NAMES` | message types 44, 45; record kinds 14, 15; two fields in `Record`, one in `Store` and `KadAnswer`; name list file format; `ed3k:name` links; DNS record and file at a web server; local web server for websites | [`extensions/name.md`](extensions/name.md) |

A node without the capability answers the message types with `Error{UNSUPPORTED}` (§5.2) and
ignores the fields (§16); one exception (§10.3): a `ROUTER` stores, checks and returns records of
every kind in this table, and `Store` fields of a source record (HTTP sources, links at hosters),
whether or not it sets the bit. So version-1 record kinds are 1 to 15; the documents above define
their checks.

### 13.2 Links

```
ed3k:file?b3=<root, base32 lower case, no padding>&sz=<size>[&dn=<url-encoded name>][&ed2k=<32 hex>][&ws=<url-encoded URL>][&wh=<url-encoded URL>][&hl=<url-encoded link>]
ed3k:node?id=<node ID, 32 hex>&ep=<ip:port or [ip6]:port>[&ep=<endpoint of the other family>]
```

Hex tokens written lower case; base32 tokens (RFC 4648 §6 alphabet) lower case, no padding. Reader
also accepts upper case and padding, padding only in RFC 4648 form (to a multiple of 8
characters). Holds for every link here and in the extensions.

- Scheme `ed3k` read in either case (RFC 3986 §3.1). Link kind (`file`, `node`, …) and parameter
  names are case-sensitive: `ed3k:File` is a kind and `B3` a parameter nobody knows.

- Parameters in any order. Value is UTF-8, percent-encoded per RFC 3986; `+` is a plus sign, not
  a space. Reader ignores unknown parameters. Writer writes `ep` as the template shows, with `[`,
  `]` and `:` as is; reader also accepts them percent-encoded.
- `dn` may be absent; a link without it is valid.
- Link invalid when a parameter occurring once by meaning (`b3`, `sz`, `dn`, `ed2k`, `id`) is
  repeated, when `b3` or `sz` of an `ed3k:file` link is missing, or when bits of a base32 token
  beyond its length are not 0. Also invalid when:
  - `b3` does not decode to 32 bytes;
  - hex token length differs from what its parameter calls for (32 digits for `id` of an
    `ed3k:node` link and for `ed2k`; `ed3k:set` defines own `id`, in base32,
    [`extensions/file-set.md`](extensions/file-set.md) §5.4);
  - base32 or hex token holds a character outside its alphabet;
  - `sz` not a decimal number, has a sign or leading zero (`0` itself is a size), or is above
    2⁶⁴ − 1;
  - `ed3k:node` link has no `id`, no `ep`, or two `ep` of one family;
  - `ep` not `ip:port` or `[ip6]:port` as the template shows, has port 0, holds an IPv4-mapped
    IPv6 address, or has an address §8.2 (7) does not accept (and (8) when the node using the
    link is in a named network; a link names no network, so a reader not yet knowing the network
    applies this check when it uses the link).

  `ws`, `wh`, `hl` and `ep` are lists and may repeat. An extension adding a parameter states how
  often it may occur; e.g. `bt` at most twice, once per hash length
  ([`extensions/torrent.md`](extensions/torrent.md)).

`ws` names an HTTP source for the whole file, may occur several times; `wh` names a URL of the
file's hash tree. Client without the HTTP source extension ignores both; that extension also
defines `ed3k:cache` (link to a cache server). Other extensions:
file set adds `path` to `ed3k:file`, defines `ed3k:set`; live stream defines `ed3k:live`;
hidden set adds `s` to `ed3k:set`; torrent adds `bt` and links at file hosters add `hl`, both
to `ed3k:file`; first contact defines `ed3k:nodes`; finding a known node defines `ed3k:friend`;
videos named on a chain add `b3`, `sz`, `ipfs` to `ed3k:set`, and `ipfs` to `ed3k:file`;
paid subscription adds `paid` to `ed3k:live` and `ed3k:set`, defines `ed3k:pass`; room defines
`ed3k:room`; readable names define `ed3k:name`.

### 13.3 Not carried over

No ed3k equivalent: preview requests (file is played while downloading instead, §12.6),
PeerCache, eD2K server message, comment packets separate from file status, separate packet type
for compressed parts (a flag on `Block`, §12.3), Kad v1, every "version N of packet X".

---

## 14. Coexistence with eD2K (OPTIONAL)

Only for a client speaking **both** eD2K and ed3k. Specifies, followable by an eD2K implementer
without the rest, how such a client lets eD2K peers know before any connection that it is also
reachable over ed3k. Changes no byte a legacy eD2K client or server interprets.

### 14.1 The port rule

> A dual-protocol client listens for ed3k on the **UDP port whose number equals its advertised
> eD2K TCP port**, on every address it advertises.

Every eD2K source record (from server, source exchange, Kad or link) carries address and TCP port;
no server or source-exchange format carries the UDP port. So the ed3k endpoint of any eD2K source
is known without one extra byte.

**Can ed3k share the eMule UDP socket?** Yes, when the user's eD2K UDP port number equals the TCP
port; else ed3k has own socket. Sharing:

- Not distinguishable by first byte: obfuscated eD2K datagram starts with a random marker byte,
  ed3k datagram with a random salt.
- **Can** be told reliably by a keyed check. Plaintext eD2K datagrams start with one of
  `0xE3 0xE4 0xE5 0xC5 0xD4 0xA3 0xB2`, which an ed3k salt never does (§4.3). For every other
  datagram receiver runs §4.5 first: live ed3k flow from that source address and port, else the
  4-byte `magic` under own key (steps 1 and 2). Only if both fail does it go to eD2K trial
  decryption (own 32-bit magic). Misrouted with probability 2⁻³² per key tried, either direction.
- Legacy client receiving a stray ed3k datagram fails trial decryption and drops it, like any
  noise.

**Alternatives considered:** always sharing the eD2K UDP port (one mapping, but servers and source
exchange don't tell a peer that port); dedicated port carried in tags (no demultiplexing, but
invisible through legacy servers and source exchange relayed by legacy clients).

### 14.2 The derived user hash

A dual-protocol client derives its eD2K user hash from its ed3k node ID `N`:

```
byte   0  1  2  3  4  5   6  7  8  9  10 11 12  13  14   15
       N0 N1 N2 N3 N4 14  N5 N6 N7 N8 N9 N10 N11 c0  111  c1

(c0, c1) = H("ed3k v1 ed2k user hash", N[0..12))[0..2)
```

Bytes 5 and 14 are eMule's own client markers (14 and 111), kept as is, so every eD2K client still
classifies the peer as eMule.

**Recognising it.** For any user hash `u`: take `n = u[0..5) ‖ u[6..13)`, compute the two check
bytes, compare with `u[13]` and `u[15]`, with `u[5] = 14` and `u[14] = 111`. On match:

- peer *probably* speaks ed3k version 1, at the UDP port of §14.1
- `n` = first 12 bytes of its node ID, exactly the `obf_id` of §4.3, so the first ed3k datagram
  can be built at once
- after the ed3k handshake, the peer's real node ID MUST start with `n`; otherwise close with
  `WRONG_PEER`. A peer whose node ID doesn't start with `n` cannot unmask the datagrams, so a
  false hint mostly shows as an unanswered handshake (§4.1)

It is a **hint**: one legacy hash in 65,536 matches by accident; a false hint costs one unanswered
handshake. An implementation MUST fall back to eD2K when ed3k does not answer and MUST NOT drop a
source for it.

**Where the hint arrives before connecting**

| Channel | User hash present? |
|---|---|
| Kad source search result | always: source ID of a Kad source record is the publisher's user hash |
| Source exchange v2, v3, v4 (`OP_ANSWERSOURCES`, `OP_ANSWERSOURCES2`) | always |
| Extended source exchange | when the `CT_EMULE_USERHASH` tag is sent |
| Server `OP_FOUNDSOURCES_OBFU` | when the per-source option byte has `0x80` set |
| Server `OP_CALLBACKREQUESTED` with the crypt extension | yes |
| Server `OP_FOUNDSOURCES`, UDP `OP_GLOBFOUNDSOURCES`, source exchange v1 | no |

LowID source: hash known, address not; the hint then says the file is worth looking up on ed3k.

**Precedents.** eD2K clients always carried signals in the user hash: eMule's 14/111, older
13/110. Other clients mark their hashes with own bytes likewise.

**Costs**

- User hash changes once on adoption. Other peers key credits by user hash, so credits earned at
  them are lost. An implementation SHOULD tell the user before switching.
- eD2K and ed3k identities become linkable by anyone; that is the purpose.
- Same marks tell an eD2K server or peer, before any ed3k message, that the client speaks ed3k;
  it may refuse the client for that. A client that must not be recognised as ed3k node on eD2K
  keeps a non-derived user hash.
- 96 of 112 free bits remain random-looking; nothing in eD2K depends on more.

### 14.3 Other signals

- **Explicit tags.** A tag in eD2K hello, Kad source record, extended source exchange, server
  login and eD2K link can state ed3k support outright (user hash is only a hint). Not part of
  this spec, as a dual-protocol client works without them. Proposal with tag numbers checked
  against existing clients: [`docs/ed2k-tags.md`](docs/ed2k-tags.md).
- **Signals not used.** Considered: free bits in connect-options byte, hello option tags and Kad
  option tag; fixed marker bytes in the user hash; Kad record mapping user hash to node ID; blind
  port probing. None reaches a peer before first connection, through legacy servers and legacy
  relaying clients, and carries the node ID. Comparison:
  [`docs/ed2k-signal-comparison.md`](docs/ed2k-signal-comparison.md).
- **eD2K callbacks as a punch signal.** An eD2K server's rendezvous gives two firewalled
  dual-protocol clients each other's mapped UDP endpoint and user hash, as §9.5 needs; Kad buddy
  callback does not. Not part of this spec; [`docs/ed2k-tags.md`](docs/ed2k-tags.md) section 5
  has comparison and a procedure.

### 14.4 Using both networks for one file

- A dual-protocol client learns (`root`, `ed2k`) pairs from `FileInfo` and `FileStatus`.
- Pair trusted once data verified under both: a block under the BLAKE3 tree and the part
  containing it under the eD2K part hash. If a part fails under one hash while its blocks pass
  under the other, the pair is false; client drops the pair, not the peers that supplied it (may
  have had it from a record). A block of eD2K data failing under the BLAKE3 tree marks the pair
  false likewise and counts against no peer (§12.3).
- A file's sources from both networks feed one download. Availability kept internally as byte
  ranges; eD2K part bitmaps and ed3k piece bitmaps are both projections of it.
- A peer reachable over both protocols SHOULD be used over ed3k only.

---

## 15. Overhead summary

Sizes = message bodies estimated from encoding rules, to be replaced by measured values once an
implementation exists. eD2K figures from eMule sources.

| Purpose | eD2K / Kad | ed3k | Why |
|---|---|---|---|
| Client hello | 103 B, plus 2 secure-ident packets (~150 B) | 30–45 B | no hash, key, ports, version tags or server fields |
| Kad contact (IPv4) | 25 B | 22 B | no TCP port, no version byte |
| Kad find + search or publish | 2 requests, 2 answers | 1 request, 1 answer | §10.3 |
| File request with part status | 55 B + bitmap | 36 B + bitmap, once per stream | hash sent once, 4 MiB pieces |
| Source entry (IPv4) | 29 B (v4) | 26 B | `Peer` with 16 B ID |
| Keyword result, note | unsigned | about 80 B more per record, 34 B per publisher | signature, expiry, key once per answer (§10.7) |
| Block header | 32 B per ≤ 10 KiB packet | ~12 B per 256 KiB | QUIC segments the stream |
| Reask while queued | 1 UDP packet each way per 29 min | 1 resumed connection (~2 KB) per 30 min to 4 h, as uploader sets; a ping per 30 min for the few kept connected | §12.5 |
| Per-datagram cost | 8–16 B obfuscation | 13 B obfuscation + QUIC header and 16 B AEAD tag | §4.3 |
| New contact | 0 (connectionless UDP) or a TCP handshake | ~2.5 KB QUIC handshake | §10.5 |

Removed to get there: generic tag lists (every field typed, absent when empty), request IDs,
fragment markers, user hash, ports equal to another port, in-message version negotiation, all
packets existing only as a second way to do the same thing.

---

## 16. Versioning and extensions

- A change an old node would misread gets a new ALPN (`ed3k/2`) and, if the datagram format
  changes, a new `magic`.
- Additive changes use new fields (unknown fields MUST be ignored), new message types (answered
  with `UNSUPPORTED`) and new capability bits.
- **Network rules** (who counts as verified, record lifetimes, an ID puzzle) change by upgrade: a
  capability bit announces support, a date fixed in the spec switches enforcement.
- A signed message MUST travel as `bytes` and be verified over exactly those bytes; a signature a
  document defines over a fixed field layout is computed over that layout. Serialisation not
  canonical, except where a document requires the canonical form of §0.
- [`proto/ed3k.proto`](proto/ed3k.proto) holds every message; an extension adds its fields and
  messages there, at the numbers its document names.

Reserved for later: further signature algorithms and batch form (§10.7), key puzzle of §6.3, Kad
datagram binding of §10.6, keyword levels and ranking (§10.8), data relay and symmetric-NAT probing
(§9.4), dropping the eD2K alias (§7.4).
Hybrid post-quantum key exchange for the QUIC binding evaluated in
[`docs/hybrid-key-exchange.md`](docs/hybrid-key-exchange.md); version 1 does not use it.

---

## Appendix A — initial constants

Rows marked **(wire)** are fixed in wire version 1 and are not tuning values (§0).

| Constant | Value |
|---|---|
| Idle timeout (`max_idle_timeout`) | 30 s |
| Exchanges peer may have open on connection, more per path between the two (both directions); request must arrive whole within | 64, 65; 30 s |
| Keep-alive (PING) by side that wants connection kept | 20 s |
| Towards index node: `max_idle_timeout` (both sides); client PING with relay reservation there or not `open` / otherwise, jitter | 300 s; 60 s / 240 s, ±10 % |
| Index node, client: redial after connection ended or dial failed; open time that resets wait; further index nodes asked per search max | 30 s doubling up to 10 min; 60 s; 3 |
| `Hello` of peer awaited on stream 0 | 30 s |
| Fragment reassembly: messages per source, timeout, one message max, all messages max (bytes / messages), attempts in fragments per packet type before unsplit one; `msg_id` not used again towards one address within | 4, 3 s, 1,500 B, 4 MiB / 65,536, 2; 3 s |
| Full-size path probes lost before connection is closed | 3 |
| `agent`, `nick` max **(wire)**; `retry_after` read as max, 0 read as where no section names wait | 64 B; 24 h, 60 s |
| Compression threshold | 64 B |
| Maximum frame body **(wire)** | 1 MiB |
| Padding of unsplit Initial, max **(wire)** | 19 B |
| Address reports: needed for lead (of endpoint or address), counted for, lead over any other endpoint or address | 3, 60 min, 2 to 1 |
| Unasked connections that show `open`: number, from hosts not sent to for, within | 2 from different sites, 60 min, 60 min |
| Reach tests whose `PROBE`s show `open`: number, within | 2 asked of different sites, 60 min |
| Reach tests: per asker address at A, per target address at B, further probers A tries; A's wait for each B, asker's wait for A; `PROBE` awaited after answer, without answer | 1 per minute, 2 per minute, 2; 3 s, 10 s; 5 s, 10 s |
| Reach test: repeated every, retry after test without prober; due test MAY be skipped on evidence from unasked connections less old than | 60 min, 30 s doubling up to 60 min; 60 min |
| Reach test: tests without `PROBE` before `open` is dropped | 2 |
| Relays per node that is not open in family, min | 2 |
| Reservations router holds, min / recommended | 1 / 8 |
| Reservations relay holds per host / site, max (index node MAY use wider) | 2 / 4 |
| Relay that answered `BUSY`: not asked again for | 30 min |
| Relay: `ttl` (informational) max, wait for target's `Punch` answer | the idle timeout, 5 s |
| Punches target starts per minute: in all, towards one host address; `PROBE`s per punch (shared evenly over I's endpoints), per destination host address and minute | 30, 3; 50, 100 |
| `Punch` forwards per minute: per leaf, per initiator address and leaf | 30, 3 |
| Punch: probe interval per endpoint, duration, probes to claimed endpoint max, attempts, back-off (also after answer that leaves no family to try) | 100–300 ms, 5 s, 3, 3, 30 min |
| Punch, initiator: direct dials before relay is asked, relay's answer awaited, handshake awaited after answer, `PROBE` used after answer | 2 s, 7 s, 8 s, 5 s |
| Dual-stack head start for IPv6 | 250 ms |
| Kad: K, α, contacts per answer, R (search / store) | 10, 3, 16, 4 / 8 |
| Kad: nodes that must answer before lookup may stop, M (search / store) | 5 / 9 |
| Kad: dialled-in contacts per bucket | 5 |
| Kad: bucket refreshed after, contact verified again after, second attempt after failed one | 60 min, 60 min, 5 min |
| Kad: time for one node to answer, longest lookup | 5 s, 60 s |
| Kad: records in one `Store` max; terms in `all` and `none` of `Query` together max | 256; 64 |
| Source records: targets in `also` per request and per batch, stored again after last successful store, max, nodes that must store before target needs its own lookup | 256, 4 h, 4 |
| Diversity, per host / site / block: bucket, answer | 1 / 2 / 4 |
| Diversity, per host / site / block: lookup, counted over its closest set | 1 / 2 / half of R (2 for a search, 4 for a store) |
| Diversity, closest set | as the lookup ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.3) |
| Diversity, per host / site / block: whole routing table | 1 / 10 / 20 |
| Diversity: host group whose contact was removed is not admitted under another node ID for; removed host groups remembered for | 1 h; 1 h |
| Diversity: `SOURCE` records per file and `LIVE` and `SUB` records per target, `ANON` records per target, `KEYWORD` and `NOTE` publishers counted per entry, per host / site | 8 / 32, 4 / 8, 1 / 1 |
| Diversity, over all targets, per host / site: `HEAD` and `HIDDEN` records held, `GATE` records held, `NAME` records held | 1,024 / 4,096, 64 / 256, 1,024 / 4,096 |
| Diversity: failed or pending dials per file and hour (source endpoints of both families, relays and gates), per host / site / block; per host and hour over all files | 2 / 4 / 8; 8 |
| Record lifetimes: source (also anonymous) and friend, keyword, note, member and alias and claim, live, subscriber, gate, head and hidden head and part and name | 5 h, 24 h, 24 h, 24 h, 10 min, 60 min, 1 h, up to 7 days |
| Search answer: keyword records, notes, sources and anonymous sources together max; paging secret replaced every | 100, 100, 100; 24 h |
| Keyword target: records kept per file, files per publisher key / per publisher site; notes per file target, in all / per publisher host / per publisher site | 3, 1,000 / 2,000; 64 / 4 / 8 |
| Source record and `FileRequest`: relays per family max (used); records per file target (and per `LIVE` or `SUB` target) max; note text max | 2; 10,000; 512 B |
| Keyword: files per target (level size), levels in use, highest level accepted **(wire)** | 100,000, 1, 15 |
| Keyword: minimum length (protocol / suggested default), keywords published per file | 2 bytes / 3 bytes, 40 |
| Signed record: maximum size, clock tolerance | 1 KiB, 1 h |
| Block, piece **(wire)** | 256 KiB, 4 MiB |
| Hash exchange: block values per request max, per node ID and file within 24 h max; answer awaited | 32,000, 2 × the blocks of the file; 30 s |
| Outstanding block ranges | 16 |
| Block compression: least saving, blocks skipped after miss | 1/8, 15 |
| Bad block, protocol fault or failed `HashAnswer`: source not used for that file | 4 h |
| 2 bad blocks within 24 h: ban of node ID, of host address | 24 h, 1 h |
| Bad block within 30 days after ban ended: ban of node ID, of host address | 30 days, 1 h |
| `BadBlock` acted on, max: per block, per peer; only for block sent within | 1 per hour, 16 per day; 10 min |
| Queue: time to come back (`recheck`) **(wire)** | 30 min to 4 h |
| Queue: entry kept after time to come back, or after lost connection | 30 min |
| Queue: ping interval, answer time | 30 min, 30 s |
| Queue: `have` update to downloader with slot | 1 per 5 min |
| Queue: early return for address or relay change, for another file, or for changed bid | 1 per 30 min, all together |
| Slot offer: wait, failures before offers stop; entry told to stay connected kept after failed offer | 30 s, 2; 30 min |
| Upload slots, recommended minimum | 2 |
| Slot: data per turn, longest turn | 16 MiB, 1 h |
| Slot: time aimed at for one turn | 5–30 min |
| Slot: held without stream (old stream closed, or `NOT_FOUND`), taken away when no range is outstanding for | 30 s, 30 s |
| Later `FileRequest` for sources answered max; sources per `FileStatus` max | 1 per 10 min; 50 |
| New connections accepted per minute: per node ID, per host address | 10, 30 |
| Open connections with peers at one host address / in one site, max | 64 / 256 |
| Ban after exceeding connection budget: node ID; host address (further connection is refused instead) | 24 h; none |
| Simultaneous dial: connection that lost is kept, min | 5 s |
| Source exchange freshness | 1 h |
| Anonymous paths: hops per direction, entry hops, kept for, paths outwards, gates | 0–3, 3, 30 days, 4, 4 |
| Anonymous paths, hop: paths per neighbour, streams per path, further exchanges per path, held per stream and direction, base rate, lookups per path, dials per path / per neighbour | 8, 64, 65, 256 KiB, 16 KiB/s, 20 per minute, 4 / 16 per minute |
| Anonymous paths: `ANON` records per target (in total / per host / per site), `GATE` lifetime and renewal, session idle time, failed `GATE` lookups or gates dialled per file and hour | 64 / 4 / 8, 1 h and 45 min, 10 min, 4 |
| Anonymous paths: dials caused by `HopExtend`/`HopConnect` (relay dials and punches included) whose handshake did not complete within 10 s, or still pending, per destination host (IPv4 /32, IPv6 /64) and hour; per neighbour host address: paths, dials per minute, failed dials (wrong node ID, refused punch) per hour | 8; 16, 32, 8 |
| Anonymous paths: hops known before path is built, reach new hop or `HopConnect` node within, status frame awaited (after `HopStream` or `HopJoin` / after `HopConnect`), `HopJoin` per gate and host, give and take measured over, relays per family in `HopConnect` | 20, 10 s, 10 s / 30 s, 10 per minute, 7 days, 2 |
| Anonymous paths: answer awaited by last hop to its `HopCreate`; by owner after `HopCreate` or `HopGate` / after `HopExtend`; refused session not opened again to that node ID for | 10 s; 10 s / 30 s; 60 s |
| Anonymous paths: next message of session handshake awaited; `HopLookup` answered by last hop at latest / awaited by owner | 10 s; 60 s / 70 s |
| Anonymous paths: replay window for stream numbers of path and of session, PING on path link every | 1,024, 20 s |
| Browse: entries per answer; pages and entries requester takes from one node, max | 1,000; 100 and 10,000 |
| Chat: message ids of sender remembered for, max; senders remembered max, of them per host address; resends after lost connection max, within | 10 min, 1,024; 256, 16; 3, 10 min |
| Rooms: item max (room info, `KeyDrop`); clock tolerance; moderators, admits, bans and lifts, presences per room; permanent bans; ban max; lift kept for; ban window of moderator kept after ban (when no longer in `mods`), windows per key | 8 KiB (72 KiB); 1 h; 8, 1,024, 4,096, 4,096; 4,096; 30 days; 30 days; 30 days, 64 |
| Rooms: presence renewed every, own presences at least apart, `expires` max after `issued`, `issued` remembered after `expires`; history (age, posts), `since` slack, dropped post ids kept; passed on per author and minute (posts, presences), per signer and minute (admits, bans, keys); taken per neighbour and minute | 5 min, 60 s, 15 min, 1 h; 24 h, 1,000, 1 h, 1 h; 20, 1 (+1 `left`); 60; 600 |
| Rooms: neighbours aimed at / kept, rooms per node, bad neighbour refused for, item awaiting its room info (time, items per room); member proof clock tolerance; `SUB` target of private room changes every, neighbour hours searched in first / last | 3 / 2–4, 16, 24 h, 60 s, 64; 10 min; 1 h, 10 min / 10 min |
| Rooms, generations: started again after (with posts), post of older one dropped after, `KeyDrop`s held, wraps per `KeyDrop`; post awaiting its `KeyDrop` (time, posts per room); moderator starts new generation once current one has `KeyDrop`s; key forgotten after later generation (no post held / at latest) | 24 h, 2 min, 64, 1,032; 60 s, 64; 32; 1 h / 48 h |
| HTTP sources: per `FileStatus`, per source record, per Kad answer; results until source is dropped; redirects followed; sealed source's `expires` ahead min | 8, 3, 16; 3; 3; 120 s |
| HTTP sources: live sealed sources before no further chunk is published; failed or pending fetches per hour, per host / site / block, counted per file and, over all files, per namer of URL; over all files and namers together, per host; host is exempt over all files after verified block for | 2; 2 / 4 / 8; 8; 1 h |
| File sets, manifest max: size, `files`, `dirs`, path component, components in path, depth, files with all inner sets | 4 MiB, 65,536, 65,536, 1–255 B, 32, 8, 1,000,000 |
| File sets: head `expires` max, follower stores head every, check followed set max / suggested | 7 days (+1 h), 24 h, 1 per hour / 1 per 6 h |
| File sets: `MEMBER` sets per file and publisher, sets per target (in all / per publisher key / per site), records kept per set, records per answer, site count saturates at | 4, 64 / 4 / 16, 3, 16, 255 |
| Hidden sets: head `expires` max, follower stores within first, sealed `data`, record dropped after its day | 7 days (+1 h), 6 h, 12–256 B, 48 h |
| Sets with several authors: grant keys, signatures required, mounts per manifest / over all levels of set, issuers, changes, part (size, files, manifest), `PART` records per target (per host / per site) / per answer, parts read per set | 8, 4, 64 / 256, 8, 4,096, 64 MiB / 256 / 64 KiB, 1,024 (8 / 32) / 16, 1,024 |
| Live: `start` max ahead, `end` kept, `have` update, sign of life, neighbour gone after, `peers` seen within, give up on request after, second exchange of stream reset after | 1 h, 30 min, 1 per s, 10 s, 30 s, 10 min, 4 s plus 1 s per MiB asked for, 10 s |
| Live: join behind newest, neighbours aimed at / kept, interrupted after, leave after, `LIVE` record stored every, `window`; cut-off broadcaster continues its `seq` when it returns within; neighbours broadcaster takes min | 3 segments, 6 / 3–8, 60 s, 10 min, 5 min, 8 to 240 (0 or absent = 30); 10 min; 3 |
| Live, rating: outcomes kept, urgent within, probe after, steady (age, outcomes, rate), core (age, outcomes, rate), requests at a time new / steady / core | 20, 3 segments, 60 s, 2 min / 5 / 80 %, 5 min / 10 / 90 %, 1 / 2 / 4 |
| Live, rating: down (rate, outcomes), out (second down within), `LIVE_BLOCKED` for, core per site / block, sudden loss (core neighbours min, share, within, hold), returning within | 50 % / 5, 10 min, 30 min, 1 / 2, 3 / more than 1/3 / 2 min / 2 min, 30 min |
| Live, private channel: look under previous hour during first, under next hour during last | 10 min, 10 min |
| Publish and subscribe: `peers` seen within, bad neighbour refused for, items passed on per topic, neighbours aimed at / kept, topics pushed, second exchange of topic reset after | 1 h, 24 h, 1 per 10 s, 3 / 2–4, 16, 10 s |
| Publish and subscribe: on-air notice `expires` after `issued` max, sent every, highest `issued` remembered after `expires`; `SUB` record stored every | 15 min, 5 min, 1 h; 30 min |
| Local network: group IPv4 / IPv6, port, announce every, multicast announces per interface max, announce datagram max, reply wait up to, replies per node, replies per minute, LAN peer kept, LAN peers max, new per minute | 239.255.101.51 / ff02::6533:6b, 6533, 5 min, 1 per minute, 512 B, 1 s, 1 per 5 min, 10, 15 min, 256, 10 |
| Local network: `LanQuery` per peer, once more on new download, roots per query | 1 per 10 min, 1 per minute, 256 |
| Names: labels per name, label, name max; zones followed, lookups per name; links per record, links of record together max; `NAME` `expires` ahead max, stored again every; delegation from `issued` to `expires` max, `issued` ahead max | 8, 63 B, 253 B; 8, 24; 4, 512 B; 7 days (+1 h), 24 h; 90 days, 1 h |
| Names, global: label; zones per target (per key / per site / in all), records kept per zone, records per answer, site count saturates at, claim stored again every; refused zones remembered per `CLAIM` target | 3–63 B; 2 / 8 / 64, 3, 16, 255, 20 h; 64 |
| Names, friends and lists: names per `NameQuery`, queries answered per peer, pinned name asked of friends again; sets of lists followed, list file max, entries per list, `note` max | 16, 1 per 10 s, 1 per 24 h; 32, 4 MiB, 100,000, 256 B |
| Names from the web: TXT record max, its query, links used per domain, `ed3k.json` max, its fetch, kept max | 1,024 B, 10 s, 4, 64 KiB, 10 s, 24 h |
| Websites: member awaited, cache by default; short slots per uploader, member max, short slots per downloader and minute | 60 s, 2 GiB; 4, 256 KiB, 32 |
| Other names: records per root and scheme per publisher; roots per target (per key / per site / in all); records kept per root; records per answer; roots tried per name; `entry`, `password` max; site count saturates at | 4; 2 / 8 / 64; 3; 16; 3; 255 B, 128 B; 255 |
| Torrents: `info_size` max, sets tried per torrent | 32 MiB, 3 |
| Usenet: article list max, sets tried per NZB, `nzb-file` records per post, `nzb-file` lookups per NZB | 32 MiB, 3, 256, 256 |
| Videos on chain: files per upload set, subtitles, picture and playlist max, link max, blocks per log read, gateways per video | 64, 1 MiB, 512 B, 2,048, 3 |
| Videos on chain, shown max: title, tag, tags, comment; avatar picture max | 256 B, 64 B, 16, 8,192 B; 1 MiB |
| Videos on chain, attachments: `media` value max, files per set, gateways per file; fetched unasked: manifest max, picture max, pictures per entry; picture refused above (side, pixels) | 512 B, 64, 3; 64 KiB, 1 MiB, 4; 16,384 px, 64 million |
| Filter lists: list file max, sets of lists followed, list files per set, list files read in all | 4 MiB, 32, 64, 64 MiB |
| First contact: saved contacts, fetch lists when fewer than N answered / max per first contact, list max, redirects followed, `created` max old, nodes per family, sources shipped / fetched, nodes taken per list, self-lookup after N answered (max half from one list, for max), listed nodes reachable for; nodes taken per list from one site / block; first LAN round over after first announces | 200, 10 / 1 per 6 h, 32 KiB, 3, 30 days, 200, 4 / 2, 50, 4 (30 s), 24 h; 2 / 8; 2 s |
| Finding known node: friends, open links, request stored every / for, request place searched every, records per target (per host / site), `sealed` **(wire)**, `issued` of record or request max ahead, record stored every, lookup of friend max, sealed values opened per search max | 64, 4, 4 h / 7 days, 30 min, 32 (8 / 16), 384 B, 1 h, 4 h, 1 per 30 min, 256 |
| Links at hosters: per `FileStatus`, per source record, per Kad answer, per member of set; results until link is dropped; streams per hoster and account | 8, 3, 16, 4; 3; 1 |
| Credits: tranche, priority slots max, time for payment (from `FileStatus` with `price` or `due`), bid change, ledger read valid for | 4 MiB, 2/3, 30 s, 1 per 30 min, 10 min |
| Credits: void bids before priority is withheld (per node ID), withheld for | 2, 24 h |
| Credits: bid ends turn that has more left than, rate measured over; uploader's share of payment | 60 s, 10 s; 10 % |
| Credits, ledger: grace, decay, floor, burned on transfer, settle within (no payment / with payment) | 3 days, 10 % per day, 10 credits, 10 %, 48 h / before the hold's `until` |
| Credits: receipts / holds per `CreditRelay`, `floor` min, `tick`, gas reward (paid once per (issuer, holder, day) entry, to its first receipt that earns); open payments of client max | 32 / 8, 1,000,000 µ per MiB, 10,000 µ per MiB, 2 × the gas of a `settle` of the first receipt of a pair and day (accounts and weeks known), minter included; 80 % of the balance |
| Credits, ledger: receipt age at settlement, hold for bid, `kib` per receipt, minted per issuer and 7 days, minted per issuer and holder and 7 days, maturity (age, contribution, issuers) | 7 days, 6 h, 50 GiB, 100 GiB, 10 GiB, 30 days / 1 GiB / 3 |
| Credits, hold: amount min, `until` ahead min, renewed every, deadline of signed hold max, stream kept open for bidder's first receipt | 16 × `max_price` µ, 2 h, 3 h, 10 min, 30 s |
| Credits, ledger: short payment sets `overdrawn` up to | 1 day after the receipt's `day` |
| Paid subscriptions: season length, horizon on online nodes, pass length max (recommended), pass `issued` ahead max, offer `issued` ahead max, proof clock tolerance, span of sighting minutes in share report, requests for missing tip per scope | about 30 days, 7 days, 7 days, 1 h, 1 h, 10 min, 80 min, 1 per day |
| Paid subscriptions: next epoch counts as current in last, key requests answered per holder key, `devices` when 0, proofs counted over, reports per holder key and scope | 10 % of an epoch (at most 10 min), 6 per hour, 3, 1 h, 1 per day |
| Paid subscriptions, seasons: chain steps per message max, chain `base` ahead of clock max, planned season's chain accepted before epoch before its `base` max; node whose key failed not used for | 2^20, 1 h (plus 1 epoch), 1 h; 24 h |
| Paid subscriptions, purchase: poll `pass_url` every / for, renewal polled in last / every | 15 s / 30 min, 48 h / 6 h |
| Flood protection: Retry token, return token valid for; per second: handshakes without token per host / site / block (Retry above), Retry packets per host (dropped above) | 10 s, 24 h; 4 / 16 / 64, 4 |
| Flood protection: validated handshakes per minute per host / site / block while guarded; Kad datagrams per second: `CHALLENGE`s per host, key derivations per host / site / block | 30 / 120 / 480; 4, 8 / 32 / 128 |
| Keyword levels (reserved on Kad, version 1 at index node): records per entry, site count saturates at, candidate files per target, leaving entries per target | 3, 255, S/4, S/4 |
| Kad datagrams (reserved): `INIT` min, cookie secret replaced every, request timer, `CHALLENGE`s per second per host, key derivations per second per host, `REQUEST`s per second per host / site / block (host, site and block of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1; private address in named network exempt), exchanges outstanding (total / per responder), `INIT` fallback to QUIC after | 70 B, 1 h, 1 s, 4, 8, 16 / 64 / 256, 16 / 2, 300 ms |
| Key puzzle (reserved): leading zero bits, memory, epoch, dialled-in contacts verified per host group (whatever the result), verification queue max; another `puzzle` value of one node ID, family and site rechecked max | 13, 64 MiB, 30 days, 1 per hour, 64; 1 per hour |


## Appendix B — open points

- Test vectors are in [`tests/vectors-v1.md`](tests/vectors-v1.md): node ID, routing ID, record
  and keyword targets with tokeniser edge cases, one signed record with the signatures and forms
  §10.7 rejects, user hash, obfuscation keys and whole datagrams of each kind, the §12.4 tree, the
  key conversion of §6.4, of the extensions: hop keys, signatures of a set and its grant, one
  canonical torrent manifest, sealed forms of a hidden set and of a friend record, LAN announce,
  one live segment record, one exchange of Kad datagrams, key puzzle, a Usenet post, the
  `canonical` of hoster links, the hash body; of paid subscriptions: scopes, a season with its
  tip, an offer, a pass, a proof, a key exchange, sealed forms and a share report; the node list
  tag, keys of private channels, hop session, pair key of friend records; of rooms: names,
  proofs, keys and a sealed post; of readable names: targets, a delegation, a `NAME` and a
  `CLAIM` record; and credit signatures of a bid and a receipt, under a test domain: chain ID and
  contract of the public network not assigned yet
  ([`extensions/credit.md`](extensions/credit.md) §7.1).
- The zstd dictionary does not exist yet; needs a message corpus. Until published, every node
  announces `dict = 0` and sends frames without a dictionary (§5.3).
- Field numbers of every message are collected in `proto/ed3k.proto`; a change in a document has
  to be made there too.
- Raw-public-key support has to be confirmed in the TLS library each implementation chooses.
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
