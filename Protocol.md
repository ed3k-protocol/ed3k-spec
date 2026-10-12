# ed3k Protocol Specification

**Status:** draft 0.1 (2026-10-10). Nothing in this document is implemented yet.
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
dual-protocol client tells eD2K peers that it also speaks ed3k. §14 is the only part of this
document that concerns eD2K, and it is OPTIONAL.

---

## 0. Conventions

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, RECOMMENDED, MAY and OPTIONAL are to be
read as in RFC 2119.

- Protobuf is **proto3**. `bytes` fields with a fixed layout state it in a comment; integers inside
  such layouts are **big-endian** (network order).
- `H(ctx, data)` is BLAKE3 in `derive_key` mode with context string `ctx`, 32 bytes of output
  unless a length is given. `H(ctx, data)[a..b)` is a byte slice.
- `varint` is the protobuf base-128 varint.
- Integer fields are `uint32` or `uint64`; both are a varint on the wire, and the type states the
  range a reader accepts. A field that holds a random 64-bit value is `fixed64`
  ([`docs/wire-format-comparison.md`](docs/wire-format-comparison.md) section 2.4).
- Numbers marked *(initial)* are tuning values. They are collected in Appendix A and may change
  without a wire version bump.

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
| 8 | `LEVELS` | Reserved for keyword levels and ranking (§10.8). MUST be 0 in version 1 |
| 9 | `HTTP` | Optional extension: understands HTTP sources ([`extensions/http-source.md`](extensions/http-source.md)) |
| 10 | `SETS` | Optional extension: understands file sets ([`extensions/file-set.md`](extensions/file-set.md)) |
| 11 | `LIVE` | Optional extension: takes part in live streams ([`extensions/live.md`](extensions/live.md)) |
| 12 | `PUBSUB` | Optional extension: passes on new records of things it follows ([`extensions/pubsub.md`](extensions/pubsub.md)) |
| 13 | `LAN` | Optional extension: answers nodes of its local network ([`extensions/lan-discovery.md`](extensions/lan-discovery.md)) |
| 14 | `ANON` | Optional extension: accepts sessions that arrive through a path of other nodes ([`extensions/hops.md`](extensions/hops.md)) |
| 15 | `HOP` | Optional extension: forwards for the paths of others ([`extensions/hops.md`](extensions/hops.md) §5) |
| 16 | `CREDIT` | Optional extension: signs receipts for what it downloads and understands bids for upload slots ([`extensions/credit.md`](extensions/credit.md)) |
| 17 | `LINKS` | Optional extension: understands links at file hosters as sources ([`extensions/hoster.md`](extensions/hoster.md)) |

`caps` travels in `Hello` only (§5.5). Compact Kad contacts and source records do not carry it,
so a node learns a peer's capabilities after it has connected.

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
  `NO_KEY`.
- The initiator knows whom it is dialling. After the handshake it MUST compute the peer's node ID
  (§6.1) and compare it with the expected ID; on mismatch it closes with `WRONG_PEER`.
- An **exchange** (T4) is one bidirectional QUIC stream. Unidirectional streams are not used.
- Connection migration SHOULD be enabled. It is what lets a session survive an address change and
  move between IPv4 and IPv6 (§8.3).
- 0-RTT MAY be used, and only for `Hello` and `KadRequest` without a `store` (replay-safe).
- Idle timeout: 30 s *(initial)* for a connection with no open exchange; relay reservations and
  file streams keep a connection alive with QUIC PING every 20 s *(initial)*.
- Congestion control is the sender's choice. BBR is RECOMMENDED.
- A node MUST accept a Retry packet (RFC 9000 §8.1.2) and repeat its Initial with the token. A
  node MAY answer an Initial with Retry at any time (§4.6).

### 4.2 Why obfuscation is required

QUIC encrypts its payload but not its shape. The Initial packet is protected with a key derived
from a public salt, so any observer can decrypt it; it is padded to 1200 bytes; and short-header
packets repeat a connection ID. The Great Firewall has decrypted and filtered QUIC Initials since
April 2024 and blocks the flow for about 180 s on a match. A UDP datagram of random bytes does not
trigger its fully-encrypted-traffic heuristic, which is limited to TCP. The obfuscation layer
therefore has two jobs: remove every fixed byte, and blur the 1200-byte handshake size.

This is obfuscation, not secrecy. The key is derived from the receiver's node ID, which is public
to anyone who takes part in the network. Confidentiality comes from TLS.

### 4.3 Datagram format

Every UDP datagram of this binding is:

```
salt(8) ‖ mask( magic(4) ‖ kp(1) ‖ pad(pad_len) ‖ body )
```

| Field | Meaning |
|---|---|
| `salt` | 8 random bytes, sent in clear. `salt[0]` MUST NOT be `0xE3`, `0xE4`, `0xE5`, `0xC5`, `0xD4`, `0xA3` or `0xB2` (re-roll). These are the plaintext headers of eD2K/eMule UDP; the rule keeps a stray ed3k datagram from being parsed as one by a legacy node at a stale address |
| `magic` | `0x65 0x33 0x6B 0x01` (`"e3k"`, version 1). Checked after unmasking |
| `kp` | bits 7-6 `kind`, bits 5-0 `pad_len` (0–63) |
| `pad` | `pad_len` random bytes |
| `body` | depends on `kind` |

| `kind` | Name | `body` |
|---|---|---|
| 0 | `DATA` | one QUIC datagram whose first packet has a short header |
| 1 | `HANDSHAKE` | one QUIC datagram containing a long-header packet |
| 2 | `FRAGMENT` | `msg_id(1) ‖ idx:4 total:4 (1) ‖ piece` of a `HANDSHAKE` datagram |
| 3 | `PROBE` | `nonce(8)`; used for hole punching and reachability tests (§9) |

**Key and mask.**

```
obf_id   = NodeID[0..12)                    of the node that ACCEPTED the connection
net_id   = H("ed3k v1 net", name)[0..16)                   empty in the public network
key      = H("ed3k v1 obfs", net_id ‖ obf_id ‖ dir ‖ salt) dir = 0x00 to that node, 0x01 from it
stream   = BLAKE3 keyed XOF (key), as many bytes as masked
mask(x)  = x XOR stream
```

`obf_id` is 12 bytes, not 16, so that it can be learned from an eD2K user hash (§14.2). A `PROBE`
is always keyed to its receiver with `dir = 0x00`.

**Networks.** The public network has no name, and `net_id` is empty there. Any other network
has a `name` of 1 to 32 bytes that all its nodes are configured with.

- A datagram from a node of another network does not unmask to the `magic` and is dropped
  like any stray datagram (§4.5). The two networks never exchange a message, a contact or a
  record.
- A **test network** uses a name that is published.
- A **closed network** uses 32 random bytes that only its members know. Whoever has the name
  is a member; there is no way to remove one except a new name. A client MUST NOT write such a
  name to a log or show it in an error message.
- A node is in one network per UDP port. Node lists and their sources
  ([`extensions/bootstrap.md`](extensions/bootstrap.md)) belong to one network.
- The reasoning is in [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §2.2.

For `HANDSHAKE`, `FRAGMENT` and `PROBE` the whole datagram after the salt is masked. For `DATA`
only the header, the padding and the first 64 bytes of the QUIC packet are masked: that covers the
QUIC flags byte and connection ID, and the rest is AEAD ciphertext already. This keeps the layer's
CPU cost on the bulk path at one BLAKE3 block per packet.

**Overhead** is 13 bytes per datagram plus padding, about 1 % of a full data packet.

### 4.4 Size shaping

- A sender SHOULD split each `HANDSHAKE` datagram into 2–4 `FRAGMENT`s of random sizes, each with
  random padding, and MAY send it whole with random padding instead. Receivers MUST support both.
- Fragments are reassembled per (source address, `msg_id`). A receiver keeps at most 4
  *(initial)* incomplete messages per source address and drops them after 3 s *(initial)*.
- `DATA` datagrams SHOULD carry `pad_len = 0` once the packet is larger than 256 bytes; smaller
  ones SHOULD be padded randomly.
- **Anti-amplification.** The layer only ever adds bytes, so a reassembled datagram is never
  larger than what arrived on the wire, and QUIC's 3× limit (RFC 9000 §8.1) stays conservative.
  A receiver MUST NOT answer a `PROBE` and MUST NOT emit anything for an incomplete fragment set.
- The QUIC maximum datagram size MUST be set 13 bytes below the path limit. An unsplit Initial is
  then 1213 bytes, which fits the IPv6 minimum MTU (1232 bytes of UDP payload).

This follows the approach of Hysteria 2's Salamander (salted XOR mask) and Gecko (fragmenting
long-header packets), with a real keystream instead of a repeating 32-byte pad and without a
pre-shared password. Censors adapt; the datagram format is versioned by `magic` so the shaping can
change without touching QUIC or the messages.

### 4.5 Receiving

1. If the source address belongs to a live connection, unmask with that connection's key.
2. Otherwise unmask the first 5 bytes with the node's own `obf_id` and `dir = 0x00`. If `magic`
   matches, process it as a new flow.
3. Otherwise the datagram is not ed3k. Drop it, or hand it to another protocol sharing the socket
   (§14.1).

A random datagram passes step 2 with probability 2⁻³².

### 4.6 Connection budget

A node MUST limit the new connections it accepts from one peer.

| Counted per | New connections per minute *(initial)* | When exceeded |
|---|---|---|
| node ID | 10 | the node ID is banned for 24 h |
| host address (IPv4 /32, IPv6 /64) | 30 | the address is banned for 1 h |

- **Only completed handshakes count.** The address is then validated and the key proven (T1,
  T3), so a forged source address cannot get another node banned. A resumed connection counts
  like a new one. Nothing is counted, and nobody is banned, on packets before that point.
- While a ban lasts, the node closes a handshake of that node ID as soon as the key is known,
  and drops datagrams of that address that would start a new flow. Connections that are already
  open are closed.
- The address budget is wider than the node ID budget because several nodes share one address
  behind a NAT. A node with the `INDEX` capability MAY use a wider address budget.
- Private, loopback, link-local and unique-local addresses are exempt.
- To stay within the budget of others, a node MUST NOT open a second connection to a peer it
  already has one with, and SHOULD keep a connection it will use again open for the idle
  timeout.

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

`Block` (§12.3) is the only message followed by raw bytes outside the frame.

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
| 21 | `LiveGet` | request | optional, §13.1 |
| 22 | `TopicJoin` | subscriber → peer | optional, §13.1 |
| 23 | `TopicState` | both | optional, §13.1 |
| 24 | `LanQuery` | request | optional, §13.1 |
| 25 | `LanAnswer` | answer | optional, §13.1 |
| 26–34 | `HopCreate`, `HopCreated`, `HopExtend`, `HopGate`, `HopLookup`, `HopLookupAnswer`, `HopStream`, `HopConnect`, `HopJoin` | between the nodes of a path | optional, §13.1 |
| 35 | `Receipt` | downloader → uploader | optional, §13.1 |
| 36 | `CreditRelay` | request | optional, §13.1 |
| 37 | `CreditRelayAnswer` | answer | optional, §13.1 |
| 48–63 | reserved | | §10.6 |

### 5.2 Exchange patterns

- **Request/answer:** the initiator opens an exchange, sends one request and half-closes. The
  responder sends one answer (or `Error`) and closes. There are no request IDs; the exchange is
  the correlation.
- **File stream:** one exchange per (downloader, uploader), open while blocks are transferred
  or while the uploader asks the downloader to stay (§12).
- An answer is never fragmented: the exchange is reliable, so a large answer is simply a long
  frame. A frame body MUST NOT exceed 1 MiB after decompression.
- A responder that does not know a request type answers `Error{UNSUPPORTED}` and closes that
  exchange only.

### 5.3 Compression

Every node MUST be able to **decompress** zstd (RFC 8878) frames in the profile below. A sender
compresses a body only when the result is smaller, and SHOULD NOT try for bodies under 64 bytes
*(initial)*. File data is not part of a frame body; it follows a `Block` frame and has its own
rule and profile (§12.3).

Profile:
- magicless zstd frames, no content checksum, no content size field
- window size at most 1 MiB
- dictionary `ed3k-v1`, dictionary ID 1, shipped with this specification (`dict/ed3k-v1.dict`,
  trained on a corpus of real messages before the format is frozen)
- each body is compressed on its own; no context is carried between messages
- a receiver MUST stop and answer `Error{MALFORMED}` when the output exceeds 1 MiB

**Later dictionaries.** A dictionary never changes once published; a better one gets the next ID.
Every zstd frame carries the ID of the dictionary it needs, so a receiver always knows which one
to use. A sender learns what the peer holds from `Hello.dict` (§5.5):

- A node that announces `dict = n` MUST hold every dictionary from 1 to n.
- A sender MUST NOT use a dictionary with an ID above the peer's `dict`. Until the peer's `Hello`
  has arrived it uses dictionary 1 or sends uncompressed.
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

```protobuf
syntax = "proto3";
package ed3k.v1;

// An endpoint is a bytes field: ip ‖ port(2, big-endian). 6 bytes = IPv4, 18 bytes = IPv6.
// A compact node is node_id(16) ‖ endpoint: 22 bytes (IPv4) or 34 bytes (IPv6).

message Peer {
  bytes id = 1;               // 16 B node ID
  bytes ep4 = 2;              // endpoint, 6 B
  bytes ep6 = 3;              // endpoint, 18 B
  bytes relays4 = 4;          // n × 22 B compact nodes; only for a leaf
  bytes relays6 = 5;          // n × 34 B compact nodes; only for a leaf
}

message FileInfo {
  bytes  root = 1;            // 32 B BLAKE3 root (§7)
  uint64 size = 2;
  string name = 3;
  bytes  ed2k = 4;            // 16 B eD2K MD4 hash; see §7.4
  uint32 sources = 5;         // complete sources known to the sender
  uint32 duration = 6;        // media length in seconds, 0 = unknown
  uint32 bitrate = 7;         // kbit/s, 0 = unknown
  // fields 8 and 9 belong to the optional file set extension (§13.1)
}
```

Compact byte strings replace `repeated` sub-messages where a list is long and its elements have a
fixed size: a compact IPv4 node is 22 bytes against about 30 as a sub-message (eMule Kad: 25).

### 5.5 `Hello`

The first exchange on every connection. The initiator sends `Hello`, the responder answers with
its own. Other exchanges MAY be opened without waiting for the answer.

```protobuf
message Hello {
  uint64 caps = 1;            // capability bits, §2
  string agent = 2;           // "eMuleQt 0.7.0"
  bytes  observed = 3;        // your endpoint as I see it
  bytes  ep4 = 4;             // my advertised endpoints; absent = none in that family
  bytes  ep6 = 5;
  uint32 nat = 6;             // 0 unknown, 1 open, 2 cone, 3 symmetric (§9.2)
  string nick = 7;            // optional
  // 8 and 9 are reserved (§10.6, §6.3)
  uint32 dict = 10;           // highest zstd dictionary ID held (§5.3); absent = 1
}
```

There is no user hash, no key and no signature in `Hello`: the transport has already delivered and
verified the key (T1), and the node ID is computed from it.

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
    NOT_RESPONSIBLE = 6;      // Kad store refused: target outside this node's zone
  }
  Code   code = 1;
  uint32 retry_after = 2;     // seconds; 0 = do not retry
}
```

An `Error` is never answered with an `Error`. Unknown codes read as `UNKNOWN`.

---

## 6. Identity

### 6.1 Key and node ID

A node has one long-lived **Ed25519** key pair.

```
NodeID = H("ed3k v1 node id", public_key)[0..16)
```

The node ID is the only identifier of a node. It is its position seed in Kad (§6.3), the key of
credit records, the source ID in source records and the thing a dialler authenticates. The eD2K
user hash has no counterpart.

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
       m[0] = m[0] OR (r << 5)
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

Private, loopback and link-local addresses are exempt, for LAN and test networks.

**Test vectors** (CRC32C of the masked address; the first three match BEP 42):

| Address | r | CRC32C | top 21 bits |
|---|---|---|---|
| 124.31.75.21 | 1 | `5fbfbdb2` | `0bf7f7` |
| 21.75.31.124 | 6 | `5a3ce9b0` | `0b479d` |
| 65.23.51.170 | 6 | `a5d4344a` | `14ba86` |
| 2001:db8:1234:5678::1 | 3 | `32b2d72e` | `06565a` |

**Limits, stated plainly**

- The position moves when the address changes. The identity does not: credits, source records
  and sessions follow the node ID.
- On IPv4 an attacker needs on the order of 2¹⁶ addresses to place one node within a chosen
  19-bit region (each address reaches 8 of 2²¹ prefixes). On IPv6 the holder of a /32 controls
  11 of the 21 bits, so a few dozen /32 allocations, or one /28, do the same. IPv6 positions are
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
/16 for IPv4; /64, /48 and /32 for IPv6. The limits are local policy and need no wire change,
which is why they can be required from the first version.

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

---

## 7. Files

### 7.1 Identity

```
root = BLAKE3(file contents)          32 bytes, default (unkeyed) mode
```

A file is identified by `root`. The size travels with it in every record but is not part of the
identity. The root is what the standard `b3sum` tool prints.

### 7.2 Verification

BLAKE3 is a Merkle tree over 1 KiB chunks. Any aligned range of 2ᵏ chunks is a subtree with its
own 32-byte chaining value, so parts of a file can be verified against the root without the rest.

- **Block:** 256 KiB (2⁸ chunks). The unit of verification and of hash exchange.
- **Piece:** 4 MiB (16 blocks). The unit of availability bitmaps.

A downloader fetches block chaining values with `HashRequest` (§12.4), verifies them against the
root, and then verifies each received block on its own. A corrupt block is detected after 256 KiB
and its sender is known exactly. The hash data is 32 bytes per block, 0.012 % of the file.
A file of at most one block needs no hash exchange.

This replaces both the eD2K part hashes (MD4 over 9.28 MB) and AICH (SHA-1 tree over 180 KiB
blocks, trusted by majority vote because the eD2K hash does not commit to it). In ed3k the root
commits to every block; there is nothing to vote on.

### 7.3 Choice of hash function

One function is used for files, node IDs, Kad targets, key derivation and the obfuscation
keystream. BLAKE3 is chosen because the hash tree is part of the function: the specification
does not have to define one, and the root is something a standard tool computes. It also
supplies keyed hashing, key derivation with context strings and an extendable output, so no
second primitive is needed for §4 and §6. The cost is a thinner round margin than the
alternatives and one more dependency.

SHA-256 with a tree defined on top is the fallback if BLAKE3 is ever weakened: the root field is
32 bytes either way and the hash is named in every context string, so it can be replaced with a
wire version bump.

The comparison with the alternatives is in
[`docs/hash-comparison.md`](docs/hash-comparison.md).

### 7.4 The eD2K alias

`FileInfo.ed2k` carries the file's eD2K hash (MD4 root of the 9.28 MB part hashes). Together with
`size` it is the eD2K identity, which lets a dual-protocol client resolve an eD2K link on ed3k and
download one file from both networks at once.

- In version 1 a sender that knows the eD2K hash MUST include it.
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
5. The endpoint of the family a connection arrived on is **observed** and therefore trusted for
   routing. An endpoint of the other family is **claimed** until dialled.
6. Abuse limits count per IPv4 address and per IPv6 /64.
7. An address is acceptable only if it is global unicast. For IPv6 this excludes at least
   `::/128`, `::1`, `fe80::/10`, `fc00::/7`, `::ffff:0:0/96`, `64:ff9b::/96`, `2001::/32`
   (Teredo), `2002::/16` (6to4), `2001:db8::/32` and `ff00::/8`. An IPv4-mapped address is
   normalised to IPv4 on receipt.

### 8.3 Dialling

When a peer has endpoints in both families and so does the dialler, IPv6 is tried first and IPv4
250 ms *(initial)* later; the first handshake to complete wins. An established connection MAY
migrate to the other family with QUIC path validation.

A node SHOULD advertise a stable IPv6 address rather than a temporary one (RFC 4941), and send
from the address it advertises.

---

## 9. NAT traversal

**NAT traversal is REQUIRED.** Every node MUST implement address observation (§9.2), act as punch
initiator and punch target (§9.5), and, when it is a `ROUTER`, offer the relay service (§9.4).
A node behind a stateful IPv6 firewall is in the same position as a node behind an IPv4 NAT and
uses the same procedure.

### 9.1 Before traversal

A node SHOULD first try to become directly reachable with a port mapping (PCP, NAT-PMP, UPnP IGD;
for IPv6 a firewall pinhole). Traversal is for when that fails.

### 9.2 Address observation and classification

Every `Hello` reports the sender's view of the receiver's endpoint (`observed`). A node collects
these reports per local socket:

| Class | Evidence |
|---|---|
| `open` (1) | a `ReachTest` (§9.3) arrived from a node it had never sent to, or unasked connections arrived (below) |
| `cone` (2) | one endpoint has the lead (below), and there is no evidence for `open` |
| `symmetric` (3) | at least 3 *(initial)* reports name the same address, and no port has the lead |
| `unknown` (0) | anything else: too few reports, or reports that name different addresses |

**Which reports count.** A node that dials in can say anything in `observed`, and can do so from
many node IDs. A report is therefore counted only when all of this holds:

- It arrived on a direct connection that the node opened itself. A connection made by hole
  punching (§9.5) counts for its initiator only.
- It is the newest report from its site, where a site is the peer's /24 or /48
  ([`extensions/ip-diversity.md`](extensions/ip-diversity.md) §1). Peers are never told apart
  by node ID.
- It is at most 60 min *(initial)* old.
- It names a public address. A report of a private, loopback, link-local or unique-local
  address, or from a peer reached at one, is not counted.

**The lead.** An endpoint has the lead when at least 3 *(initial)* counted reports name it, and
at least twice as many as name any other endpoint. Only an endpoint with the lead is adopted as
the node's own. One peer that reports a false port can therefore neither change the address nor
turn `cone` into `symmetric`.

**Unasked connections.** A connection that completes from a host the node has sent nothing to
within the last 60 min *(initial)* could not have passed a NAT or a stateful firewall. Two such
connections from different sites within 60 min *(initial)* are evidence for `open` in that
family, the same as a `PROBE`. Relayed and punched connections do not count. Two are asked for
because one can come through a mapping that an earlier exchange left open.

**Testing again.** A class can go stale: a port mapping expires, a firewall rule changes.

- A node asks for a `ReachTest` every 60 min *(initial)* per address family, and at once when
  its local address or its observed endpoint changes.
- When no prober was asked (`ok` not set, or no answer), it tries again with another peer after
  30 s *(initial)*, doubling the wait each time up to 60 min.
- An `open` node that was asked a prober and received no `PROBE` in two tests in a row is no
  longer `open`, unless unasked connections arrived since the first of the two. It classifies
  itself from its address reports and takes relays (§9.4).
- An `open` node MAY skip a test that is due while it has evidence from unasked connections
  that is less than 60 min old. A test after an address change is never skipped.

### 9.3 `ReachTest`

```protobuf
message ReachTest {
  bytes  target_id = 1;       // set when forwarded: node to probe
  bytes  target_ep = 2;       // set when forwarded: endpoint to probe
  fixed64 nonce = 3;          // random; goes into the PROBE as 8 B, big-endian
  bool   ok = 4;              // answer: a prober was asked
}
```

A node asks a peer A to have it tested. A picks a `ROUTER` contact B that has no connection with
the asker, and forwards the request with `target_id` and `target_ep` filled in; `target_ep` MUST
be the asker's observed endpoint as A sees it, never a value from the asker. B sends one `PROBE`
datagram (§4.3) with the nonce to that endpoint. If it arrives, the asker is `open` in that
family. A and B rate-limit these to 1 per asker address per minute *(initial)*.

### 9.4 Relays

A node keeps reservations at 2 *(initial)* relays in total while it has an address family in
which it is not `open`. That includes a `ROUTER` that is open in one family only.

```protobuf
message Relay {
  bool   reserve = 1;         // request: true = hold a slot, false = release
  uint32 ttl = 2;             // answer: seconds the slot is held without traffic
}
```

A relay holds the connection open and forwards `Punch` messages to the leaf. A leaf lists its
relays in every `Peer` record it publishes (`relays4`, `relays6`).

**Listing.** The list a relay is in says how an initiator reaches the relay, not which family
the punch uses: `Punch` carries the endpoints of both. A relay is listed with an endpoint of
its own in that family, so a node prefers relays that have both.

| The node is not open in | `relays4` | `relays6` |
|---|---|---|
| IPv4 only, or it has no IPv6 | both relays | none |
| IPv6 only, or it has no IPv4 | none | both relays |
| both families | both relays | one of them |

Why 2, with a comparison to one relay, to two per family and to libp2p (2 by default), is in
[`docs/relay-count.md`](docs/relay-count.md).

**Finding relays.** A leaf takes its relays from the `ROUTER` nodes it meets in its own Kad
lookups.

- It asks nodes whose `Hello` has `RELAY` set, with `Relay{reserve = true}`. A node that
  answers `Error{BUSY}` is skipped and not asked again for 30 min *(initial)*.
- Its relays SHOULD be in different sites ([`extensions/ip-diversity.md`](extensions/ip-diversity.md)
  §1), so that one network going away does not take all of them.
- A relay that is lost is replaced at once. The new list goes out with the next refresh of
  each record. Only when a published list has no working relay left does the leaf publish its
  `Peer` records again at once.

**Capacity.** The two numbers are the two sides of the service: a leaf holds a reservation at
2 relays, and a relay serves many leaves. A `ROUTER` SHOULD be able to serve at
least 8 *(initial)* leaves at the same time, one reservation per leaf node ID, and MAY serve
more. When all are taken it answers `Error{BUSY}` and clears `RELAY` in its `Hello` until one
is free.

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
the limits of that extension.

### 9.5 Hole punching

```protobuf
message Punch {
  bytes  peer_id = 1;         // to relay: the target. to target: the initiator
  bytes  ep4 = 2;             // sender's observed endpoints
  bytes  ep6 = 3;
  uint32 nat = 4;             // sender's class, §9.2
  fixed64 nonce = 5;          // random; goes into each PROBE as 8 B, big-endian
}
```

```
initiator I                        relay R                          target T
  ── Punch{peer_id=T, eps(I)} ──►   forward over T's reservation
                                    ── Punch{peer_id=I, eps(I)} ──►
                                                                     start PROBEs to eps(I)
                                    ◄── Punch{peer_id=I, eps(T)} ──
  ◄── Punch{peer_id=T, eps(T)} ──
  dial QUIC to eps(T)
```

1. I opens an exchange to one of T's relays and sends `Punch`.
2. R forwards it to T on a new exchange, with `peer_id` replaced by I's ID. R answers
   `Error{NOT_FOUND}` if T holds no reservation.
3. T answers with its own endpoints, and at once starts sending `PROBE` datagrams to each of I's
   endpoints in the families both share, at random intervals of 10–200 ms for 5 s *(initial)*.
   These open T's mapping.
4. R returns T's answer to I. I dials T with a normal QUIC handshake; its retransmitted Initials
   open I's mapping. I is the QUIC client, T the server.
5. I tries up to 3 times *(initial)*. After that the source is marked unreachable for
   30 min *(initial)*.

Pairs that cannot work are skipped without trying: `symmetric` on both sides, in a family both
need. IPv6 is preferred when both sides have it, since a stateful firewall has no port mapping to
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
| `ALIAS` | `H("ed3k v1 alias", len(scheme) ‖ scheme ‖ id)[0..16)`, with the kind of name and the name ([`extensions/alias.md`](extensions/alias.md) §1) |
| `SUB` | `H("ed3k v1 topic", type(1) ‖ key ‖ id)[0..16)`, with the type, the key and the `id` of the topic ([`extensions/pubsub.md`](extensions/pubsub.md) §1) |
| `KEYWORD` | `H("ed3k v1 keyword", level(1) ‖ word)[0..16)`, `word` being one keyword of the file name in UTF-8, as defined below. `level` is one byte, 0 in version 1 (§10.8) |

File hashes, node IDs and keyword keys all come from the same function and are cut to the same
128 bits, so they share one keyspace without conversion.

**Words and keywords.** Publisher, searcher and storing node MUST split a name the same way:

1. **Normalise.** Apply Unicode NFKC, then full case folding, then NFKC again. Remove code
   points of general category Cf (soft hyphen, zero-width joiners).
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
   all of category L or N, it is the extension of the name. Otherwise the name has none.

The **words** of a name are the distinct results of steps 1 to 3. Its **keywords** are those
words that are at least 2 bytes long in UTF-8, without the word that step 4 made the extension.
Targets are derived from keywords only, and a storing node refuses a record under any other
target (§10.7). `Query` (§10.3) matches against words.

| Name | Words | Keywords |
|---|---|---|
| `The.Big_Sleep (1946) [1080p].mkv` | `the` `big` `sleep` `1946` `1080p` `mkv` | `the` `big` `sleep` `1946` `1080p` |
| `Ça-va_ÉTÉ.MP3` | `ça` `va` `été` `mp3` | `ça` `va` `été` |
| `Straße.ﬁnal.txt` | `strasse` `final` `txt` | `strasse` `final` |
| `東京物語 Tokyo Story.avi` | `東京` `京物` `物語` `tokyo` `story` `avi` | `東京` `京物` `物語` `tokyo` `story` |

- The Unicode stability policy keeps normalisation and case folding of an assigned character
  fixed across Unicode versions. A code point that an implementation does not know is treated
  as a separator.
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
Only `ROUTER` nodes are inserted. A contact is dropped when a connection attempt fails twice.
Insertion is subject to the per-bucket and per-table limits of
[`extensions/ip-diversity.md`](extensions/ip-diversity.md).

**Contacts that dialled in.** A contact is *dialled-in* when the node first verified it on a
connection that the contact opened. At most half of a bucket, 5 of K = 10 *(initial)*, may be
dialled-in contacts; a further one is not inserted. The mark stays while the contact is in the
table, also after the node has connected to it itself. Opening connections to a node is the
cheapest way to enter its table, and the contacts a node finds by its own lookups are not chosen
by the other side.

### 10.3 The one request

There is a single Kad request. It always asks for closer nodes and optionally carries an
operation that the receiver performs if it is responsible for the target.

```protobuf
enum Kind { NONE = 0; SOURCE = 1; KEYWORD = 2; NOTE = 3; HEAD = 4; MEMBER = 5; LIVE = 6; SUB = 7; HIDDEN = 8; PART = 9; FRIEND = 10; ANON = 11; GATE = 12; ALIAS = 13; }

message KadRequest {
  bytes  target = 1;          // 16 B
  uint32 want = 2;            // bit 0: IPv4 contacts, bit 1: IPv6 contacts
  Kind   kind = 3;            // NONE = find nodes only
  Query  search = 4;          // search for records of `kind`
  Store  store = 5;           // store records of `kind`
  uint32 level = 6;           // KEYWORD: level the target was computed for (§10.1); 0 in version 1
}

message Query {
  repeated string all = 1;    // every word must be a word of the name (§10.1)
  repeated string none = 2;   // no word may be a word of the name
  uint64 min_size = 3;
  uint64 max_size = 4;
  uint32 min_sources = 5;
  string ext = 6;             // extension of the name (§10.1)
  uint32 offset = 7;          // skip this many results
}

message Store {
  Peer     source = 1;        // SOURCE: my endpoints and relays; `id` MUST be empty
  repeated Signed files = 2;  // KEYWORD: files whose name contains the keyword (§10.7)
  Signed   note = 3;          // NOTE (§10.7)
  // field 4 belongs to the optional HTTP source extension (§13.1)
  // fields 5 to 7 belong to the optional file set extension (§13.1)
  bytes    also = 8;          // SOURCE: n × 16 B further targets that get the same record (§10.4); at most 256
  // field 9 belongs to the optional extension for finding a known node (§13.1)
  // field 10 belongs to the optional extension for anonymous paths (§13.1)
  // field 11 belongs to the optional extension for other names of a file (§13.1)
  // field 12 belongs to the optional extension for links at file hosters (§13.1)
}

message Note {
  uint32 rating = 1;          // 1..5, 0 = none
  string text = 2;
}

message KadAnswer {
  bytes  nodes4 = 1;          // n × 22 B compact nodes, closest first
  bytes  nodes6 = 2;          // n × 34 B
  bool   responsible = 3;     // this node considers the target inside its zone
  uint32 load = 4;            // 0..100: files held under this target, in percent of the level size (§10.8)
  repeated Peer     sources = 5;
  repeated Signed   files = 6;  // §10.7
  repeated Signed   notes = 7;  // §10.7
  bool   more = 8;            // further results exist; ask again with `offset`
  repeated bytes    keys = 9; // 32 B publisher keys, referenced by Signed.key
  repeated StoreResult results = 10;  // one per record of the Store, in order; absent = all stored
  bool   spill = 11;          // this node answered FULL for this target within the last record lifetime
  repeated uint32   sites = 12;       // one per entry of `files`: publisher sites counted; 0 = not counted
  // field 13 belongs to the optional HTTP source extension (§13.1)
  // field 14 belongs to the optional file set extension (§13.1)
  // field 15 belongs to the optional extension for finding a known node (§13.1)
  // fields 16 and 17 belong to the optional extension for anonymous paths (§13.1)
  // field 18 belongs to the optional extension for other names of a file (§13.1)
  // field 19 belongs to the optional extension for links at file hosters (§13.1)
}

enum StoreResult {
  STORED = 0;
  FULL = 1;                   // no room under this target (§10.8)
  BUSY = 2;                   // no room on this node; says nothing about the target
  REJECTED = 3;               // failed a check (§10.7) or a flood limit
  FAR = 4;                    // a target of `also` this node is not responsible for
}
```

This one pair replaces 12 Kad2 opcodes: `REQ`/`RES`, three search requests and their answer, three
publish requests with answer and acknowledgement, and bootstrap (a `KadRequest` for one's own ID;
where the first address comes from is in [`extensions/bootstrap.md`](extensions/bootstrap.md)).
`Hello` (§5.5) replaces the Kad hello pair, ping/pong and the firewall check packets; §9 replaces
the buddy packets.

Rules:

- A node returns up to 16 *(initial)* verified contacts per requested family that are closer to
  the target than itself. It returns none when it knows none closer. "Itself" is its routing ID
  for the address the request arrived at.
- **The requester checks the answer.** It computes the routing ID of each returned contact from
  the contact's address and node ID (§6.3). It drops a contact that is not closer to the target
  than the answering node, and every contact after the 16th of a family. An answer with such a
  contact counts as one failed attempt for the answering node (§10.2); a correct node cannot
  produce one. This check comes before the filters of
  [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.2.
- **Responsibility.** A node is responsible for a target when it knows fewer than
  K verified contacts closer to the target than one of its own routing IDs. It sets
  `responsible`, executes a `search`, and accepts a `store`.
- **`HEAD` and `MEMBER`** are the record kinds of file sets. Every `ROUTER` MUST store and return
  them, whether or not it sets `SETS`. Their content and checks are in
  [`extensions/file-set.md`](extensions/file-set.md) §5 and §6.
- **`LIVE`** is the record kind for the peers of a live stream. It is stored and returned like
  `SOURCE`, with a shorter lifetime. Every `ROUTER` MUST store and return it, whether or not it
  sets `LIVE` ([`extensions/live.md`](extensions/live.md) §5).
- **`SUB`** is the record kind for the subscribers of a topic. It is stored and returned like
  `SOURCE`, with a shorter lifetime. Every `ROUTER` MUST store and return it, whether or not it
  sets `PUBSUB` ([`extensions/pubsub.md`](extensions/pubsub.md) §5).
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
- **`ANON`** and **`GATE`** are the record kinds of a node that hides its address behind other
  nodes: `ANON` names it as a source of a file, by its node ID alone, and `GATE` says through
  which nodes it is reached. Both are signed. A search of kind `SOURCE` also returns the `ANON`
  records of the target. A `Store` of kind `KEYWORD`, `NOTE`, `ANON` or `GATE` MAY carry
  `signer`; the record is then checked under that key in place of the key of the connection.
  Every `ROUTER` MUST do all of this, whether or not it sets `ANON`
  ([`extensions/hops.md`](extensions/hops.md) §4).
- A node that does not know the `kind` of a request answers as a node that is not responsible.
- A node that is not responsible ignores `search` and `store` and only returns contacts. It
  answers a `store` with contacts and `responsible = false`, never with stored data.
- **The requester follows closer nodes.** Whenever an answer contains contacts closer than the
  answering node, the requester MUST continue to them, whether or not the answer said
  `responsible`. Only the closest verified nodes hold the payload; an answer from a node further
  out is a waypoint.
- **Source records are bound to the connection.** The storing node takes the source's node ID
  from the authenticated connection and its endpoint from the observed address. A node can
  therefore publish only itself, and source lists cannot be filled with invented peers.
- **Source records are not signed.** A searcher has only the storing node's word for a source
  until it dials it. Dialling is the verification: the handshake proves that the node at that
  address holds the key behind the node ID in the record (T1), and a source that fails it is
  dropped. A signature would add nothing to that, because it cannot prove that an address
  belongs to a key. What a false record can still cause is a wasted dial, which is bounded by
  the dial limits of [`extensions/ip-diversity.md`](extensions/ip-diversity.md) §2.5.
- **Keyword and note records are signed** by their publisher (§10.7). Their content is still a
  claim; the signature fixes who made it.
- Lifetimes *(initial)*: `SOURCE`, `ANON` and `FRIEND` 5 h, `GATE` 1 h, `KEYWORD` 24 h, `NOTE` 24 h, `MEMBER` and `ALIAS` 24 h, `LIVE` 10 min, `SUB` 60 min, `HEAD`, `HIDDEN` and `PART` up to
  7 days. Republish before expiry.
- Flood limits are per source address and per `kind`.
- **Keyword search.** The searcher splits its search text into words as in §10.1 (steps 1 to 3).
  It derives the target from one of them that is a keyword, and sends the others in `all`. A
  word in `all` or `none` matches a name when it equals one of the name's words; there is no
  substring match. A search text without a keyword cannot be searched.
- **Source records for many targets.** A `store` of kind `SOURCE` MAY name further targets in
  `also`. The node treats each as a store of the same record under that target: it checks its
  responsibility and its limits per target, and answers in `results` with one entry for the
  target of the request followed by one per target of `also`. A target it is not responsible
  for is answered `FAR`, and every target after the 256th *(initial)* `REJECTED`. `nodes4` and
  `nodes6` refer to the target of the request. Fields that belong to one file, such as HTTP
  sources, apply to the target of the request only. How a publisher uses this is in §10.4.
- A `store` is answered per record in `results`. What a keyword target holds, and what a
  publisher does when it is full, is in §10.8.

### 10.4 Lookup

1. Seed the candidate set with the closest contacts from the routing table.
2. Query the α = 3 *(initial)* closest candidates not yet queried, carrying the operation in every
   request.
3. Add returned contacts to the candidates. A contact becomes verified when its connection
   completes (§6.3).
4. Stop when the R closest verified candidates have all answered and none of them returned a
   closer node. R = 4 for a search, 8 for a store *(initial)*.

A queried node that has not answered within 5 s *(initial)*, counted from the first packet sent
to it, has failed for this lookup and counts one failed attempt (§10.2). A lookup ends after
60 s *(initial)* at the latest, with the closest set it has by then.

Which returned contacts become candidates, which candidates are queried and which R nodes form
the final set are further restricted by [`extensions/ip-diversity.md`](extensions/ip-diversity.md)
§2.2 and §2.3.

**Storing many source records.** A node that shares many files does not look up every root.

1. It sorts its targets.
2. It looks up the first target that is due, with the store in the request as always. The
   lookup ends with a closest set.
3. To each node of that set it sends one more request for the same target, with the following
   targets in `also`: those whose distance to the target is smaller than the distance from the
   target to the furthest node of the set.
4. A target that fewer than 4 *(initial)* nodes answered `STORED` gets a lookup of its own.
   Otherwise the node goes on with the first target behind the batch.

- One pass over all targets takes 4 h *(initial)* and is spread evenly over that time, so that
  every record is renewed before its 5 h end without a burst.
- A file that is new is stored at once and joins the pass afterwards.
- The number of lookups per pass is then bounded by the size of the network, not by the number
  of files. The estimate and the comparison with IPFS, where the method comes from, are in
  [`docs/ipfs-freenet-comparison.md`](docs/ipfs-freenet-comparison.md) §2.1.

**Searching sources for many files.** There is no batch for searches; every target is looked
up. A client that has many targets to search SHOULD take them in sorted order, and SHOULD add
the closest set of the lookup just ended to the candidates of the next one (step 1). Where two
targets share their closest nodes, the second lookup then ends after one round on connections
that are still open. Each target is still a request of its own and counts against the limits
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
| No speculative queries | A candidate is queried only if it is among the α closest known. Every wasted query costs 2.5 KB, so α stays small and is not raised to hide timeouts |
| First hop is warm | Connections to routing-table contacts stay open while idle (30 s) and resume with 0-RTT afterwards, which saves the round trip though not the padded Initial |
| One connection per node, many exchanges | Lookups for different targets that pass through the same node share its connection; a client publishing or searching many files orders them by target so neighbouring walks overlap (§10.4) |
| Keyword records batched per node | All files sharing a keyword go in one `Store` |
| Source records batched per region | Targets that share their closest nodes go in one `Store` per node (§10.4) |
| Index nodes | A node logged in to an index node (§11) publishes and searches there over one connection and needs Kad for keywords only as a fallback |
| Long record lifetimes | Publishing is the dominant Kad load; it happens every 5 h and 24 h, not per search |

Expected cost of a lookup with operation: 3–4 hops and 8–12 nodes contacted, of which 5–9 are new
connections, so roughly 15–25 KB against 3–5 KB in eMule Kad. That is the price of having one
authentication and anti-spoofing mechanism instead of two, and of the verified addresses that §6.3
depends on.

### 10.6 Reserved: sessionless Kad datagrams

If measurements on the live network show the cost above to be a problem, Kad requests can move to
single sealed datagrams, as eMule Kad and Ethereum's discv5 do. Version 1 reserves for this:

- capability bit 5 (`KAD_DGRAM`)
- message types 48–63
- a second `magic` value for the datagram format of §4.3
- field 8 of `Hello`

Such a binding has to provide on its own what QUIC provides here: proof of the sender's key,
proof of its address and replay safety. It carries the same `KadRequest` and `KadAnswer`.
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
}

message Signature {
  uint32 alg = 1;             // 1 = Ed25519
  bytes  sig = 2;
  // field 3 is reserved for a batch form: one signature over many records
}

message Signed {
  bytes  body = 1;            // a serialised Record, verified over exactly these bytes
  uint32 key = 2;             // index into KadAnswer.keys; absent in a Store
  repeated Signature sigs = 3;
}
```

Each signature is computed over

```
"ed3k v1 record" ‖ kind(1) ‖ target(16) ‖ body
```

`kind` and `target` are those of the request, so they are signed without being sent, and a
record cannot be replayed under another keyword or file.

**Signatures**

- The publisher signs with its identity key (§6.1). In a `Store` the key is the one of the
  connection. In a `KadAnswer` it is `keys[key]`, sent once per publisher.
- `sigs` is a list from version 1 on. It holds at most one signature per algorithm.
- A record is **valid** when every signature of an algorithm the verifier knows verifies, and
  every *required* algorithm is present. In version 1 the required set is Ed25519 alone. The
  set changes by network upgrade (§16).
- Signatures of unknown algorithms are ignored by the verifier and kept by a storing node.
- A `Signed` MUST NOT exceed 1 KiB *(initial)*. This bounds what unknown signatures can add.
- How a key of another algorithm is tied to a node identity is **not defined** in version 1.
  The specification that registers an algorithm has to define it. Signatures of the sizes that
  post-quantum algorithms have today (2 KB and more) will need the reserved batch form.

**Checks**

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| Signature | valid under the connection's key | valid under `keys[key]` |
| `expires` | later than now, and at most the lifetime of the kind plus 1 h *(initial)* ahead | later than now minus 1 h |
| Content | `file` for `KEYWORD`, `note` for `NOTE`, nothing else | the same |
| Keyword | one keyword of `file.name` (§10.1) gives the target of the request at its `level` | one keyword of `file.name` gives the target that was searched |
| Count | one record per publisher and file under a keyword; one note per publisher and file | duplicates by publisher and file are merged |

A record that fails a check is rejected (`Error{MALFORMED}`) or dropped. There are no unsigned
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
- While a target is full, a record for a file the node already holds is renewed (`STORED`), and
  a record for a new file is refused (`FULL`).
- `BUSY` means the node as a whole is out of room. It says nothing about the target.
- `load` and `spill` MUST be set as defined in §10.3. `sites` MAY be left at 0.

**Publisher**

- A file is published under every keyword of its name.
- A refused record is offered again at the next refresh, not earlier. There is no longer
  back-off.
- The outcome of a store is judged over all answers from responsible nodes, ignoring `BUSY`.
  One node cannot make a publisher give up on a keyword.

**Searcher**

- `load` and `spill` tell the searcher that the results are a part of what was published under
  that word.

**Levels.** The keyword target includes a level (§10.1), and version 1 uses level 0 only: a node
MUST send `level = 0` and MUST NOT publish or search at another level. A storing node treats a
target of level 1 to 15 like any other keyword target, so that upgraded publishers can use those
levels on nodes that have not upgraded; it rejects a record with `level` above 15. A later network upgrade
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
| Get sources | `KadRequest{kind=SOURCE}` |
| Callback request | `Punch` (an index node is a relay) |
| Server list | `KadRequest{want}` returns nodes; index nodes are recognised by `Hello.caps` |

Differences from a Kad node: an index node is `responsible` for **every** target, never needs the
iterative walk, MAY keep source records for as long as the publishing connection stays open
instead of the fixed lifetime, and MAY apply its own admission policy with `Error{DENIED}` or
`{BUSY}`. Signed records (§10.7) expire on an index node as anywhere else; the publisher renews
them.

A client finds index nodes from a configured list, from a link (§13.2) or from an eD2K server
that advertises one ([`docs/ed2k-tags.md`](docs/ed2k-tags.md)). Index nodes are optional for the
network and optional for a client.

---

## 12. File transfer

### 12.1 The file stream

A downloader opens one exchange per uploader for the file it wants from it. Everything about
that file between the two peers happens on it. The file is named once, in the first message.
The stream stays open during a transfer; while the downloader waits in the queue, the uploader
decides whether it stays open (§12.5).

```protobuf
message FileRequest {
  bytes  root = 1;            // 32 B; only in the first FileRequest of the stream
  bytes  have = 2;            // my piece bitmap, LSB first; empty = nothing or not told
  uint32 want_sources = 3;    // max number of sources wanted, 0 = none
  bytes  relays4 = 4;         // n × 22 B compact nodes: my relays, if I am a leaf (§12.5)
  bytes  relays6 = 5;         // n × 34 B
  bool   brief = 6;           // I know size, name, ed2k and note; leave them out
  // field 7 belongs to the optional HTTP source extension (§13.1)
  // field 8 belongs to the optional file set extension (§13.1)
  // field 9 belongs to the optional credit extension (§13.1)
}

message FileStatus {
  uint64 size = 1;
  string name = 2;
  bytes  ed2k = 3;            // §7.4
  bytes  have = 4;            // uploader's piece bitmap; empty = complete file
  uint32 rank = 5;            // queue position; 0 = upload slot granted, send BlockRequest
  repeated Peer sources = 6;  // source exchange
  Note   note = 7;            // uploader's rating/comment on the file
  uint32 recheck = 8;         // queued: 0 = stay connected, else ask again after this many seconds (§12.5)
  // field 9 belongs to the optional HTTP source extension (§13.1)
  // field 10 belongs to the optional file set extension (§13.1)
  // field 11 belongs to the optional credit extension (§13.1)
  // field 12 belongs to the optional extension for links at file hosters (§13.1)
}

message SlotOffer {
  bytes  root = 1;            // the file the downloader is queued for
}
```

- The uploader answers the first `FileRequest` with a full `FileStatus`, or `Error{NOT_FOUND}`.
- Later `FileStatus` messages carry only changed fields; `size`, `name` and `ed2k` are sent
  once. When they are sent is limited by §12.5.
- The downloader sends a new `FileRequest` to update `have` or to ask for sources again.
- Either side ends by closing the exchange. There is no cancel message.
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
  repeated uint64 offset = 1; // packed
  repeated uint32 length = 2; // packed, same count; each ≤ 256 KiB
  bool   raw = 3;             // send these ranges as they are, never compressed (§12.3)
}
```

Allowed only while `rank = 0`. Up to 16 *(initial)* ranges may be outstanding.

### 12.3 `Block`

```protobuf
message Block {
  uint64 offset = 1;
  uint32 size = 2;            // number of bytes that follow this frame
  uint32 plain = 3;           // set when those bytes are zstd: length of the file data they hold
}
```

The frame is followed by exactly `size` bytes. They are not inside the protobuf body, so a block
is never copied through the serializer. Without `plain` they are file data as it is.

Per-block overhead is about 8 bytes on 256 KiB.

**Compressed blocks.** Most shared files are compressed media and archives and gain nothing.
Text, databases, disk images and file set manifests gain a lot. A sender MAY therefore compress
a block, and every node MUST be able to receive one.

- Profile: one magicless zstd frame per block as in §5.3, without a dictionary. Each block is
  compressed on its own, so blocks can be asked for in any order and from any source.
- `plain` is the length of the block after decompression and is at most 256 KiB. A receiver
  stops when the output would exceed `plain`. Output of another length, or input that does not
  decode, is a bad block (below).
- Hashes are over the file data, never over the compressed bytes (§7.2).
- **When to compress.** A sender sends a block compressed only when that saves at least 1/8 of
  its length *(initial)*. After a block that did not, it SHOULD NOT try the next 15 blocks
  *(initial)* of that file. It MAY remember the result per file, since a file never changes, and
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

**A block that fails.** Every block is verified on its own (§7.2), so the sender of a bad one
is known exactly. The downloader discards the block and asks another source for it.

- After 1 bad block a downloader SHOULD stop using that node as a source for that file for
  4 h *(initial)*.
- After a bad block in a second file, or 2 bad blocks in total, within 24 h *(initial)*, it
  SHOULD stop using the node as a source for any file for 24 h *(initial)*.
- Both are counted per node ID and are local: nothing is reported to other nodes. Uploads to
  that node and its credits stay local policy (§12.1).

### 12.4 Hashes

```protobuf
message HashRequest {
  uint64 first = 1;           // first block index
  uint32 count = 2;           // number of blocks
}

message HashAnswer {
  bytes cvs = 1;              // count × 32 B block chaining values
  bytes proof = 2;            // n × 32 B sibling values linking the range to the root
}
```

`cvs` are the BLAKE3 chaining values of the 256 KiB subtrees `first .. first+count-1`. `proof`
holds the sibling chaining values needed to recompute the root from that range, ordered from the
lowest tree level upwards and left to right within a level. The receiver recomputes parent nodes
up to the root and compares with `root`; the root node is finalised with BLAKE3's `ROOT` flag, all
others without. A request for all blocks has an empty proof.

### 12.5 Waiting in the queue

An uploader's queue can hold thousands of downloaders. Keeping a connection and a stream of
updates for each would cost far more than eMule's reask, so the uploader sets how each one
waits.

**One entry per downloader.** An uploader keeps one queue entry per node ID, for one file. A
downloader has at most one file stream open to an uploader. Opening a file stream for another
file replaces the request and keeps the position, or the slot if one is granted (see "Several
files in one turn" below).

**Stay or come back.** Every `FileStatus` to a queued downloader carries `recheck`:

| `recheck` | Meaning |
|---|---|
| 0 | Stay connected. The uploader expects to serve this downloader soon |
| 1800 to 14400 | Close the file stream and open a new one after this many seconds |

- The uploader chooses the value per downloader, shorter near the front of the queue. How many
  downloaders it keeps connected is local policy.
- **The uploader closes the stream** after a `FileStatus` with a `recheck` above 0, and the
  connection if nothing else uses it. A downloader cannot stay connected against the uploader's
  choice.
- The downloader comes back by opening a new file stream; `brief` keeps the answer small.
- It MAY come back earlier to report changed endpoints or relays, at most once per 30 min
  *(initial)*, or to ask for another file, which keeps its position as in eMule. Any other
  early `FileRequest` MAY be answered with `Error{BUSY}` and `retry_after`. The
  position is not affected.
- The entry is kept until 30 min *(initial)* after the time the downloader was told to come
  back. Then it is removed.

**While connected.** The uploader sends a `FileStatus` only

- when a slot is granted (`rank` = 0) or taken away,
- to change `recheck`,
- while the downloader holds a slot, for a changed `have`, at most once per 5 min *(initial)*,
- every 30 min *(initial)* as a **ping**: with `rank` and `have` if they changed, empty
  otherwise.

The downloader answers a ping with a `FileRequest` within 30 s, empty unless its own `have`
changed. If it does not, or the connection is lost, the uploader closes the stream. The entry is
then kept for 30 min so that the downloader can come back, and removed after that.

**A free slot.** The uploader contacts the next downloader at once.

- Connected: `FileStatus` with `rank` = 0 on the open stream.
- Not connected: the uploader dials the downloader at the endpoints of its `Hello`, or by
  `Punch` through the relays of its last `FileRequest` (§9.5). It opens an exchange and sends
  `SlotOffer`. The downloader answers on that exchange with a `FileRequest`, and the exchange
  continues as a file stream with the slot granted. A downloader that no longer wants the file
  answers `Error{NOT_FOUND}` and its entry is removed. It answers `Error{DENIED}` to an offer
  from a node it is not queued at.
- The uploader waits 30 s *(initial)* from the start of the dial. After that the slot goes to
  the next downloader.
- A downloader that could not be reached keeps its position and is offered the next free slot.
  After a second failure it gets no further offers.
- In both cases the entry stays until its normal end, 30 min after the time the downloader was
  told to come back. A `FileRequest` from the downloader before then clears the failures.

**Length of a slot.** A slot is one turn at an uploader.

- An uploader SHOULD transfer 16 MiB (4 pieces) to a downloader before it takes the slot away,
  as long as the downloader keeps requesting.
- An uploader SHOULD take the slot away after 1 h even if less was transferred, so that a
  downloader with little bandwidth does not hold it. Bytes count from the `FileStatus` that
  granted the slot.
- After 16 MiB the uploader MAY go on while nobody else is waiting.
- A downloader that loses its slot stays in the queue. Its new position is local policy.

**Several files in one turn.** A slot belongs to the downloader, not to the file.

- A downloader that holds a slot (`rank` = 0) MAY open a file stream for another file. It
  replaces the open one, and the uploader MUST keep the slot: the bytes and the time of the
  turn go on counting across files.
- The uploader answers the new `FileRequest` with `rank` = 0, or with `Error{NOT_FOUND}`.
  After `NOT_FOUND` it holds the slot for 30 s *(initial)* for a further `FileRequest`, and
  then takes it away.
- The downloader SHOULD set `brief` when it knows size and name.
- A turn stays 16 MiB or 1 h, so several files give no more than one.

**Number of slots.** An uploader SHOULD choose the number of its upload slots so that 16 MiB
take 5 to 30 min in each, which is about 9 to 55 KiB/s per slot. It SHOULD offer at least 2
slots; below about 19 KiB/s of upload that minimum comes first and a turn takes longer.

The comparison with eMule's reask and with the alternatives is in
[`docs/queue-status-comparison.md`](docs/queue-status-comparison.md).

---

## 13. Other peer messages

### 13.1 Optional messages

Browsing a peer's shared files, chat, HTTP sources, file sets, live streams, publish and subscribe, and finding nodes in the local network are optional extensions. A node announces
each with a capability bit and need not implement any of them. Hidden sets, sets with
several authors, other names of a file, torrents and Usenet posts as file sets, shared filter lists, first
contact and finding a known node are optional too. They add file formats, links and four record
kinds, and have no bit of their own. Links at file hosters are a further kind of source, with
a bit of their own.

**Other networks and data sources are joined under the root, never beside it.** A name from
outside leads to a root through one record kind, `ALIAS`, and the rules that every such
extension follows are in [`extensions/alias.md`](extensions/alias.md) §5.
Anonymous paths, which put other nodes between a node and its peers, are optional as well.
So are credits that count at every uploader; they need a ledger outside ed3k.

| Extension | Capability | Wire elements | Document |
|---|---|---|---|
| Browse | bit 3, `BROWSE` | message types 14, 15 | [`extensions/browse.md`](extensions/browse.md) |
| Chat | bit 4, `CHAT` | message types 16, 18 | [`extensions/chat.md`](extensions/chat.md) |
| HTTP sources | bit 9, `HTTP` | fields in `FileRequest`, `FileStatus`, `Store` and `KadAnswer`; `ws` and `wh` in links; `ed3k:cache` links | [`extensions/http-source.md`](extensions/http-source.md) |
| File sets | bit 10, `SETS` | a manifest file format; record kinds 4 and 5; fields in `FileInfo`, `FileRequest`, `FileStatus`, `Record`, `Store` and `KadAnswer`; `path` in links; `ed3k:set` links | [`extensions/file-set.md`](extensions/file-set.md) |
| Live streams | bit 11, `LIVE` | message types 19 to 21; record kind 6; a channel file format; `ed3k:live` links | [`extensions/live.md`](extensions/live.md) |
| Publish and subscribe | bit 12, `PUBSUB` | message types 22 and 23; record kind 7; a field in `Record` | [`extensions/pubsub.md`](extensions/pubsub.md) |
| Finding nodes in the local network | bit 13, `LAN` | message types 24 and 25; a multicast datagram outside any connection | [`extensions/lan-discovery.md`](extensions/lan-discovery.md) |
| Anonymous paths | bits 14 and 15, `ANON` and `HOP` | message types 26 to 34; record kinds 11 and 12; a field in `Record` and `Store`, two in `KadAnswer` | [`extensions/hops.md`](extensions/hops.md) |
| Credits | bit 16, `CREDIT` | message types 35 to 37; a field in `FileRequest` and in `FileStatus`; a ledger on a chain of its own, reached by RPC | [`extensions/credit.md`](extensions/credit.md) |
| Hidden sets | none (`SETS`) | record kind 8; a field in `Record` and in a manifest; `s` in `ed3k:set` links | [`extensions/hidden-set.md`](extensions/hidden-set.md) |
| Sets with several authors | none (`SETS`) | record kind 9; two fields in `Record`; fields in a manifest | [`extensions/multi-author-set.md`](extensions/multi-author-set.md) |
| Links at file hosters | bit 17, `LINKS` | a field in `FileStatus`, `Store` and `KadAnswer`; three result codes; a field in a manifest; `hl` in `ed3k:file` links; two schemes of `ALIAS` | [`extensions/hoster.md`](extensions/hoster.md) |
| Other names of a file | none | record kind 13; a field in `Record`, `Store` and `KadAnswer` | [`extensions/alias.md`](extensions/alias.md) |
| Torrents as file sets | none (`SETS`) | two schemes of `ALIAS`; a field in a manifest; `bt` in `ed3k:file` links | [`extensions/torrent.md`](extensions/torrent.md) |
| Usenet posts as file sets | none (`SETS`) | two schemes of `ALIAS`; a field in a manifest; an article list file format | [`extensions/usenet.md`](extensions/usenet.md) |
| Shared filter lists | none | a list file format | [`extensions/filter-list.md`](extensions/filter-list.md) |
| First contact | none | a node list file format; `ed3k:nodes` links | [`extensions/bootstrap.md`](extensions/bootstrap.md) |
| Finding a known node | none | record kind 10; a field in `Store` and in `KadAnswer`; `ed3k:friend` links | [`extensions/friend.md`](extensions/friend.md) |

A node without the capability answers the message types with `Error{UNSUPPORTED}` (§5.2) and
ignores the fields (§16).

### 13.2 Links

```
ed3k:file?b3=<root, base32 lower case, no padding>&sz=<size>&dn=<url-encoded name>[&ed2k=<32 hex>][&ws=<url-encoded URL>][&wh=<url-encoded URL>][&hl=<url-encoded link>]
ed3k:node?id=<node ID, 32 hex>&ep=<ip:port>[&ep=<[ip6]:port>]
```

`ws` names an HTTP source for the whole file and may occur several times. `wh` names a URL of
the file's hash tree. A client without the HTTP source extension ignores both. That extension
also defines `ed3k:cache`, a link to a cache server. The file set extension adds `path` to
`ed3k:file` and defines `ed3k:set`. The live stream extension defines `ed3k:live`. The hidden
set extension adds `s` to `ed3k:set`, the torrent extension adds `bt` and the extension for links at file hosters adds `hl` to `ed3k:file`, the first contact extension defines `ed3k:nodes`, and the
extension for finding a known node defines `ed3k:friend`.

### 13.3 Not carried over

Preview requests, PeerCache, the eD2K server message, comment packets separate from file status,
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
  datagram the receiver runs §4.5 first: a live ed3k flow from that source address, else the
  4-byte `magic` under its own key. Only if both fail does the datagram go to the eD2K trial
  decryption, which has its own 32-bit magic. A datagram is misrouted with probability 2⁻³²
  in either direction.
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
  `WRONG_PEER`

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
  pass under the other, the pair is false; the client drops it and the peers that supplied it.
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
| Block header | 32 B per ≤ 10 KiB packet | ~8 B per 256 KiB | QUIC segments the stream |
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
- Anything that is ever signed MUST travel as `bytes` and be verified over exactly those bytes.
  Protobuf serialisation is not canonical.

Reserved for later: further signature algorithms and the batch form (§10.7), the key puzzle of §6.3, the Kad datagram
binding of §10.6, keyword levels and ranking (§10.8), a data relay and symmetric-NAT probing (§9.4), dropping the eD2K alias (§7.4).
A hybrid post-quantum key exchange for the QUIC binding is evaluated in
[`docs/hybrid-key-exchange.md`](docs/hybrid-key-exchange.md); version 1 does not use it.

---

## Appendix A — initial constants

| Constant | Value |
|---|---|
| Idle timeout without open exchange | 30 s |
| Keep-alive on reservations and file streams | 20 s |
| Fragment reassembly: messages per source, timeout | 4, 3 s |
| Compression threshold | 64 B |
| Maximum frame body | 1 MiB |
| Address reports: needed for the lead, counted for, lead over any other endpoint | 3, 60 min, 2 to 1 |
| Unasked connections that show `open`: number, from hosts not sent to for, within | 2 from different sites, 60 min, 60 min |
| Reach tests per asker address | 1 per minute |
| Reach test: repeated every, retry after a test without prober | 60 min, 30 s doubling up to 60 min |
| Reach test: tests without `PROBE` before `open` is dropped | 2 |
| Relays per node that is not open in a family | 2 |
| Reservations a router holds, recommended minimum | 8 |
| Relay that answered `BUSY`: not asked again for | 30 min |
| `Punch` forwards per minute: per leaf, per initiator address and leaf | 30, 3 |
| Punch: probe interval, duration, attempts, back-off | 10–200 ms, 5 s, 3, 30 min |
| Dual-stack head start for IPv6 | 250 ms |
| Kad: K, α, contacts per answer, R (search / store) | 10, 3, 16, 4 / 8 |
| Kad: dialled-in contacts per bucket | 5 |
| Kad: time for one node to answer, longest lookup | 5 s, 60 s |
| Source records: targets per request, pass over all targets, nodes that must store before a target needs its own lookup | 256, 4 h, 4 |
| Diversity, per host / site / block: bucket, answer, lookup | 1 / 2 / 4 |
| Diversity, per host / site / block: whole routing table | 1 / 10 / 20 |
| Diversity: new node ID per host group | 1 per hour |
| Diversity: `SOURCE` records per file, per host / site | 8 / 32 |
| Diversity: failed source dials per file and hour, per host / site / block | 2 / 4 / 8 |
| Record lifetimes: source (also anonymous) and friend, keyword, note, member, live, subscriber, head and hidden head and part | 5 h, 24 h, 24 h, 24 h, 10 min, 60 min, up to 7 days |
| Keyword: files per target (level size), levels in use, highest level accepted | 100,000, 1, 15 |
| Keyword: minimum length (protocol / suggested default), keywords published per file | 2 bytes / 3 bytes, 40 |
| Signed record: maximum size, clock tolerance | 1 KiB, 1 h |
| Block, piece | 256 KiB, 4 MiB |
| Outstanding block ranges | 16 |
| Block compression: least saving, blocks skipped after a miss | 1/8, 15 |
| Bad block: source not used for that file | 4 h |
| Bad block in a second file, or 2 in total, within 24 h: source not used at all | 24 h |
| Queue: time to come back (`recheck`) | 30 min to 4 h |
| Queue: entry kept after the time to come back, or after a lost connection | 30 min |
| Queue: ping interval, answer time | 30 min, 30 s |
| Queue: `have` update to a downloader with a slot | 1 per 5 min |
| Queue: early return for an address or relay change | 1 per 30 min |
| Slot offer: wait, failures before offers stop | 30 s, 2 |
| Upload slots, recommended minimum | 2 |
| Slot: data per turn, longest turn | 16 MiB, 1 h |
| Slot: time aimed at for one turn | 5–30 min |
| Slot: held after `NOT_FOUND` for another file | 30 s |
| New connections accepted per minute: per node ID, per host address | 10, 30 |
| Ban after exceeding the connection budget: node ID, host address | 24 h, 1 h |
| Source exchange freshness | 1 h |
| Anonymous paths: hops per direction, entry hops, kept for, paths outwards, gates | 0–3, 3, 30 days, 4, 4 |
| Anonymous paths, a hop: paths per neighbour, streams per path, held per stream and direction, base rate, lookups per path | 8, 64, 256 KiB, 16 KiB/s, 20 per minute |
| Anonymous paths: `ANON` records per target, `GATE` lifetime and renewal, session idle time | 64, 1 h and 45 min, 10 min |
| Other names: records per root and scheme, roots per target, records per answer, roots tried per name | 4, 64, 16, 3 |
| Usenet: article list at most, sets tried per NZB, `nzb-file` records per post | 32 MiB, 3, 256 |
| Links at hosters: per `FileStatus`, per source record, per Kad answer, per member of a set; results until a link is dropped; streams per hoster and account | 8, 3, 16, 4; 3; 1 |
| Credits: tranche, priority slots at most, time for a payment, bid change, ledger read valid for | 4 MiB, 2/3, 30 s, 1 per 30 min, 10 min |
| Credits: a bid ends a turn that has more left than, rate measured over; uploader's share of a payment | 60 s, 10 s; 10 % |
| Credits, ledger: grace, decay, floor, burned on transfer, settle within | 3 days, 10 % per day, 10 credits, 10 %, 48 h |

## Appendix B — open points

- BLAKE3 test vectors for node ID, routing ID, keyword target, user hash and obfuscation key are
  to be added under `tests/` with the first implementation.
- The zstd dictionary does not exist yet; it needs a message corpus.
- Raw-public-key support has to be confirmed in the TLS library chosen by each implementation.
- The numbers in §10.5 and §15 are estimates.

## Appendix C — references

- [RFC 9000](https://www.rfc-editor.org/rfc/rfc9000),
  [RFC 9001](https://www.rfc-editor.org/rfc/rfc9001),
  [RFC 9002](https://www.rfc-editor.org/rfc/rfc9002) (QUIC);
  [RFC 7250](https://www.rfc-editor.org/rfc/rfc7250) (raw public keys);
  [RFC 8878](https://www.rfc-editor.org/rfc/rfc8878) (zstd);
  [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032) (Ed25519)
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
  [RFC 8439](https://www.rfc-editor.org/rfc/rfc8439) (ChaCha20), used by private channels and hidden sets
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
