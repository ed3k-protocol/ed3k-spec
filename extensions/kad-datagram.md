# Extension: sessionless Kad datagrams (`kad-dgram`)

**Status:** draft 0.1 (2026-10-10). Not implemented. Not part of wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.6. Section numbers (§) refer to that document.
**Capability:** bit 5, `KAD_DGRAM`.

A Kad exchange in version 1 costs a QUIC connection per node: about 2.5 KB and two round trips
for a request that is 20 bytes and an answer that is a few hundred. This extension carries the
same `KadRequest` and `KadAnswer` in one UDP datagram each way, without a connection, and keeps
the three guarantees the QUIC binding gives Kad:

| Guarantee | Needed for | QUIC binding | This binding |
|---|---|---|---|
| Proof of the peer's key | low 107 bits of the routing ID (§6.3) | TLS handshake | static Diffie-Hellman; an answer or request that decrypts was made by the key holder |
| Proof of the peer's address | top 21 bits of the routing ID; no reflection | QUIC address validation | cookie for the requester; fresh transaction ID for the responder |
| Confidentiality and no fixed bytes | T2, T5 | TLS + §4 obfuscation | AEAD + the same mask |

It is a second binding for Kad messages only. Everything else stays on QUIC, and QUIC remains the
fallback for any Kad message that does not fit.

---

## 1. Overview

```
requester I                                              responder R (ROUTER, KAD_DGRAM)

 first contact only:
  ── INIT       {txid} ───────────────────────────────►
  ◄─ CHALLENGE  {txid, cookie, R's public key} ───────

 every exchange:
  ── REQUEST    {txid, I's public key, cookie, sealed KadRequest} ──►
  ◄─ ANSWER     {txid, sealed KadAnswer} ─────────────
```

- The responder keeps **no state** per requester. The cookie is recomputed, the shared key is
  derived from the two public keys.
- A requester that already holds R's public key and a valid cookie sends `REQUEST` at once: one
  round trip. Both can come from an earlier datagram exchange or from a QUIC session (§6 below).
- A stale cookie is answered with a fresh `CHALLENGE`, and the requester repeats the `REQUEST`.

---

## 2. Datagram format

The datagram reuses the layout of §4.3 with a second `magic`:

```
salt(8) ‖ mask( magic(4) ‖ kp(1) ‖ pad(pad_len) ‖ type(1) ‖ txid(4) ‖ payload )
```

| Field | Meaning |
|---|---|
| `salt` | 8 random bytes; same first-byte rule as §4.3 |
| `magic` | `0x65 0x33 0x6B 0x02` |
| `kp` | bits 7-6 MUST be 0, and a datagram in which they are not is dropped; bits 5-0 `pad_len` |
| `type` | 48 `INIT`, 49 `CHALLENGE`, 50 `REQUEST`, 51 `ANSWER` (from the range reserved in §5.1) |
| `txid` | 4 random bytes chosen by the requester, echoed by the responder |

**Mask.** As in §4.3, over the whole datagram after the salt, keyed with the **responder's**
`obf_id`: `dir = 0x00` for `INIT` and `REQUEST`, `dir = 0x01` for `CHALLENGE` and `ANSWER`.
The responder does not need to know who the requester is to mask its reply.

**Receiving.** Step 2 of §4.5 accepts either magic. This binding adds a step 2a to §4.5,
between steps 2 and 3: for a datagram from an address and port with a pending exchange, unmask with
that responder's `obf_id` and `dir = 0x01`; on a match hand it to the exchange. A datagram
from an address with a live QUIC connection that fails the `magic` check under the connection
key goes on to the next step of §4.5, as any datagram does; step 2a is what catches a
`CHALLENGE` or `ANSWER` from a responder that is at the same time the client of a QUIC
connection to this node.

A datagram of this binding MUST NOT exceed 1200 bytes of UDP payload.

### 2.1 `INIT` (48)

No payload. `pad_len` MUST be at least 52, so the datagram is at least 70 bytes: a `CHALLENGE` that
carries a key-puzzle solution ([`key-puzzle.md`](key-puzzle.md) §3) is then never longer than
the `INIT` it answers. A receiver drops an `INIT` with `pad_len` below 52 or with bytes after
`txid`, as §2.2 of this document drops a wrong payload length.

### 2.2 `CHALLENGE` (49)

```
cookie(8) ‖ responder_public_key(32)
```

The payload after `txid` is 40 bytes, or 52 when a key-puzzle solution follows the key
([`key-puzzle.md`](key-puzzle.md) §3): the payload length tells which, and a datagram with any
other payload length is dropped. The datagram is then 58 or 70 bytes plus padding.
It MUST NOT be longer than the datagram it answers. Sent in reply to an `INIT`, and in reply
to a `REQUEST` whose cookie is not valid.

The requester MUST check that `H("ed3k v1 node id", responder_public_key)[0..16)` equals the node
ID it meant to contact, and drop the datagram otherwise.

### 2.3 `REQUEST` (50)

```
requester_public_key(32) ‖ cookie(8) ‖ seal_req( nf(1) ‖ tag ‖ body )
```

### 2.4 `ANSWER` (51)

```
seal_ans( nf(1) ‖ [cookie(8)] ‖ tag ‖ body )
```

### 2.5 Sealed content

| Field | Meaning |
|---|---|
| `nf` | bit 0: sender is `ROUTER`; bit 1: sender is `INDEX`; bit 7 (`ANSWER` only): a fresh cookie follows. Other bits MUST be 0 |
| `tag` | varint `(type << 1) \| z` exactly as in §5.1. Allowed types: 3 `KadRequest` in a `REQUEST`; 4 `KadAnswer` or 2 `Error` in an `ANSWER` |
| `body` | the protobuf message, to the end of the plaintext. There is no length field. zstd as in §5.3 when `z = 1` |

A sealed content whose `nf` has another bit set, or whose `tag` is not allowed for its datagram
type, is dropped after opening, without an answer.

`nf` is information only. It does not make a contact `ROUTER` or `INDEX` for a routing table;
that takes a `Hello` (§3.4 of this document).

---

## 3. Cryptography

### 3.1 Keys

Each node's Ed25519 identity key is used for X25519 through the conversion of `Protocol.md`
§6.4.

```
ss            = X25519(own x_private, peer x_public)        reject an all-zero result
k_req ‖ k_ans = H("ed3k v1 kad dgram", ss ‖ requester_public_key ‖ responder_public_key)   64 bytes
```

The two keys in the derivation are the Ed25519 keys as they are sent, not the converted ones.
Both sides compute the same `ss`; the two keys depend on who asks, so the two directions of an
exchange, and the two roles a pair of nodes can take towards each other, never share a key.
A node SHOULD cache `k_req ‖ k_ans` per peer key and role.

### 3.2 Sealing

ChaCha20-Poly1305 (RFC 8439), 16-byte tag.

```
nonce = salt(8) ‖ txid(4)                       of the datagram being sealed
aad   = type(1) ‖ txid(4) ‖ everything in the payload before the ciphertext
        for ANSWER also ‖ req_hash, so that an answer is bound to its request
req_hash = H("ed3k v1 kad req", type(1) ‖ txid(4) ‖ payload)[0..8)    of the REQUEST being answered
seal_req uses k_req, seal_ans uses k_ans
```

`req_hash` is taken over the **unmasked** bytes of the `REQUEST`: its `type` (50), its `txid`
and its whole payload, that is `requester_public_key ‖ cookie ‖` the ciphertext with its tag
(§2.3 of this document). `salt`, `magic`, `kp` and the padding are not part of it.

Every copy of a `REQUEST` has its own `req_hash`: a retransmission has a new salt and so a new
ciphertext, and the copy sent after a `CHALLENGE` has a new cookie as well. A requester keeps
the `req_hash` of every copy it sent for a `txid`, at most three (§4 of this document), and accepts an `ANSWER`
that opens under any of them. The responder uses the hash of the copy it answers.

`salt` is fresh for every datagram, retransmissions included, so a nonce never repeats under a
key in practice: 64 random bits, and the `txid` tells exchanges apart. A retransmission is
sealed again under its new salt, never sent as the same bytes.

### 3.3 Cookie

```
cookie = BLAKE3_keyed(secret_e, ip ‖ port(2, big-endian))[0..8)
```

`secret_e` is a random 32-byte value the responder replaces every hour *(initial)*. A cookie made
with the current or the previous secret is valid, so it lives between one and two hours. `ip` is
4 or 16 bytes.

When a `REQUEST` carried a cookie from the previous secret, the responder SHOULD set bit 7 of
`nf` and include a current one, which saves the requester a `CHALLENGE` round trip later.

### 3.4 What each side has proven

| After | Proven |
|---|---|
| Responder accepts a `REQUEST` | The cookie matches the source address: the requester receives at that address. The seal opens under `k_req`: the requester holds the private key of `requester_public_key`. Its node ID and routing ID are therefore **verified** in the sense of §6.3 |
| Requester accepts an `ANSWER` | The seal opens under `k_ans` with the `txid` of a pending request: the answer was made by the holder of the responder's key, after seeing a request that was sent to that address. The responder is **verified** |

A verified contact from this binding has the same standing as one verified by a QUIC handshake,
with one exception: a datagram carries no `Hello`, so it does not show `nat = 1`. A contact
first seen through a Kad datagram is not inserted until a `Hello` on a connection of that family
says `ROUTER` and `nat = 1` (`Protocol.md` §10.2). A contact first verified by a `REQUEST` it
sent is a dialled-in contact, as one that opened a connection is.

### 3.5 Properties and limits

- **No forward secrecy.** Keys are static per pair. A stolen identity key lets its thief read
  recorded Kad datagrams of that node. Kad requests are not secret from the network: any node
  can be asked the same question. Peer sessions and file transfers stay on QUIC with full TLS.
- **Replay.** A recorded `REQUEST` can be replayed only with the same source address, forged
  or not, and only while its cookie is valid, one to two hours. A replay changes nothing on
  the responder, because no `REQUEST` stores anything (§4 of this document). It makes the responder send one
  more answer, up to 1200 bytes for about 96, to the address of the real requester; the
  `REQUEST` rate of §5 of this document bounds that. A replayed `ANSWER` matches no pending `txid` and is
  dropped.
- **One key, two uses.** Using an Ed25519 key for X25519 after conversion is established
  practice; the context strings keep the DH output apart from every other use of the key.
- **The requester's key is visible.** `REQUEST` carries `requester_public_key` under the mask
  only (§2.3 of this document). Anyone who knows the responder's node ID unmasks it and learns which node
  asked; the QUIC binding keeps the key inside TLS.
- **Not a general transport.** No streams, no ordering, no retransmission beyond §4 of this
  document, no congestion control beyond the limits in §5 of this document.

---

## 4. Rules for an exchange

**Requester**

1. Without the responder's public key: send `INIT`. With key but without cookie: also `INIT`.
   Then wait for the `CHALLENGE` (step 3) with the timer of step 2; `REQUEST` is not sent
   before it arrives. For a contact that is not known to set `KAD_DGRAM`, 300 ms *(initial)*
   replace the timer of step 2: when they pass, the `INIT` is not retransmitted, no failure
   is counted, and the requester opens a QUIC connection (§9 of this document). A `CHALLENGE` with that `txid`
   that arrives later only fills the cache with key and cookie; no `REQUEST` follows it.
2. With key and cookie: send `REQUEST`. Start a timer of 1 s *(initial)*; a requester that
   knows the round-trip time to the responder MAY use twice that instead. Every `INIT` or
   `REQUEST` sent starts this timer anew.
3. On `CHALLENGE` with the same `txid`: store cookie and key, and send the `REQUEST` with the
   same `txid` and a new `salt`. This is done once per `txid`; a later `CHALLENGE` for it only
   fills the cache.
4. On timeout: send the last datagram again with a new `salt` and the same `txid`, the `INIT`
   at most once and the `REQUEST` at most once. A `txid` has at most 3 copies of its `REQUEST`
   (§3.2 of this document). When the timer runs out and nothing may be sent again, the exchange
   has failed; the contact counts one failure as in §10.2. This step does not apply to the
   `INIT` for a contact not known to set `KAD_DGRAM` (step 1).
5. Accept the first valid `ANSWER` for a `txid` and ignore later ones. An `ANSWER` is valid
   when it opens under the `req_hash` of any copy sent for that `txid` (§3.2 of this document).
6. Never send a `KadRequest` with `store` in a `REQUEST`. A lookup for a store MAY walk by
   datagram with the same request without `store`; the request with `store` goes over QUIC. A node sets `responsible` in its answer whether or
   not the request carries an operation (`Protocol.md` §10.3), so the walk ends as it does
   over QUIC.
   On `Error{UNSUPPORTED}` for a request, repeat it over QUIC.

**Responder**

1. `INIT`: reply `CHALLENGE`, subject to the limits in §5 of this document. Do nothing else.
2. `REQUEST`: check the cookie first. If it is not valid, reply `CHALLENGE` and stop; no
   Diffie-Hellman is computed for an unvalidated address.
3. Open the seal. On failure drop silently.
4. A `KadRequest` with `store` is not accepted over datagrams. Nothing is stored, and the
   `ANSWER` carries `Error{UNSUPPORTED}` (§5.6), which tells the requester to repeat the
   request over QUIC.
5. Process any other `KadRequest` exactly as §10.3 prescribes. Where §10.3 names the family
   or the address of the connection, the family and the source address of the datagram take
   their place.
6. Reply with one `ANSWER`.

Only a `ROUTER` node answers. A leaf uses the binding as a requester only, and drops `INIT`
and `REQUEST` without an answer.

### 4.1 Fitting one datagram

There is no fragmentation. A sealed `ANSWER` leaves about 1150 bytes for the body, a `REQUEST`
about 1110.

- A responder builds the `KadAnswer` in this order and stops when the next item would not fit:
  contacts (closest first, both families interleaved by distance), then records in the paging
  order of §10.3. Records are left out from the end of that order only, and `offset` of the
  next request counts the records carried. `http` and `links` go in only for carried sources,
  and are left out before any record. A requester cannot see that riders were left out; it
  gets them over QUIC, or from the source itself ([`http-source.md`](http-source.md) §2.1,
  [`hoster.md`](hoster.md) §2). If records were left out it sets `more`. `sites`
  holds one entry per record carried, and `keys` every key that a carried record references.
  zstd is applied before measuring.
- A requester that sees `more` either asks again with `Query.offset`, or opens a QUIC connection
  for the rest.
- If an answer sets `more` and carries no record at all, the contacts left no room for one. The
  requester asks again with `want = 0`, so that the answer carries no contacts. If that answer
  carries no record either, one record does not fit a datagram, and the requester sends the
  request over QUIC. It does not ask a third time by datagram for that request. A request that
  already had `want = 0` goes over QUIC at once when its answer sets `more` and carries no
  record.
- A `KadRequest` that does not fit, in practice a search with many words, MUST be sent over
  QUIC. A request with `store` goes over QUIC whatever its size (§4 of this document).

16 IPv4 contacts take 352 bytes and 16 IPv6 contacts 544, so a find-node answer for both families
always fits.

---

## 5. Abuse limits

| Risk | Rule |
|---|---|
| Reflection | A `CHALLENGE` is never longer than the datagram that caused it, and nothing else is sent to an address without a valid cookie. A responder SHOULD send at most 4 *(initial)* `CHALLENGE`s per second to one host ([`ip-diversity.md`](ip-diversity.md) §1) |
| Amplification | An `ANSWER` goes only to a cookie-validated address |
| CPU | Cookie check (one keyed hash) precedes everything. Derived keys are cached per peer key; a responder limits new key derivations per host ([`ip-diversity.md`](ip-diversity.md) §1) to 8 per second *(initial)* |
| Request floods and replay | A responder processes at most 16 / 64 / 256 *(initial)* `REQUEST`s per second per host / site / block ([`ip-diversity.md`](ip-diversity.md) §1), counted on `REQUEST`s with a valid cookie. A `REQUEST` above a limit is dropped without an answer |
| Flooding of records | A `KadRequest` with `store` is not accepted over datagrams and is answered `Error{UNSUPPORTED}` (§4 of this document). The per-address, per-kind policy of §10.3 applies unchanged to searches |
| Requester load | At most 16 *(initial)* exchanges outstanding in total, and 2 per responder |

A private address (§8.2 (8)) is exempt from the limits of the rows *Reflection*, *CPU* and
*Request floods and replay* where §4.6 exempts it from the host-address row: in a named network
(§4.3).

---

## 6. Getting key and cookie without `INIT`

`Hello` (§5.5) gains one field:

```protobuf
message Hello {
  // ...
  bytes dgram_cookie = 8;     // 8 B; sent by a ROUTER node with KAD_DGRAM: the cookie it accepts from your endpoint
}
```

A node that has had any QUIC session with a `ROUTER` node that sets `KAD_DGRAM` holds its
public key (from the handshake) and a cookie, and can use datagrams with it at once. The cookie is tied to the
peer's endpoint on that connection: its source when the peer dialled, the endpoint dialled when
the `KAD_DGRAM` node did. After an address change it fails and is replaced by a `CHALLENGE`.

---

## 7. Cost

Sizes are whole UDP payloads without padding, estimated from the layouts above.

| Datagram | Fixed part | Typical total |
|---|---|---|
| `INIT` | 18 (70 with the pad of §2.1 of this document) | 70–81 |
| `CHALLENGE` | 58 (70 with a key-puzzle solution) | 58–70 |
| `REQUEST` | 76 + body | ~96 for a find-node, ~140 for a search |
| `ANSWER` | 36 + body (+8 with cookie) | ~390 with 16 IPv4 contacts |

| | QUIC binding (§10.5) | This binding | eMule Kad |
|---|---|---|---|
| Exchange with a known node | ~2.5 KB handshake + messages, 2 round trips (1 with 0-RTT) | ~0.5 KB, 1 round trip | ~0.3–0.4 KB, 1 round trip |
| First contact with a node | same | + ~120 B and 1 round trip | + verify-key round trip in some cases |
| Lookup with a search, 8–12 nodes | 15–25 KB | 5–7 KB | 3–5 KB |

A store is not sent by datagram (§4 of this document). A lookup for a store saves the connections to the
waypoints only; each node that stores still costs a QUIC connection (§10.5).

The remaining gap to eMule Kad is the requester's 32-byte key and the 16-byte tags, which buy
authenticated node IDs.

`KadAnswer` carries no `observed` endpoint. A node that uses datagrams for all of Kad still
learns its own address from its relay and peer sessions (§9.2).

---

## 8. Alternatives considered

| Design | Why not |
|---|---|
| **Signatures instead of DH.** Requester signs the request, responder signs the answer | 64-byte signature plus 32-byte key in each direction, no confidentiality beyond the mask, and a signature check is no cheaper than a cached DH key |
| **discv5-style sessions.** WHOAREYOU challenge, ephemeral handshake, per-peer session keys | Forward secrecy, at the price of session state on both sides, a handshake packet type with its own retransmission rules, and key rotation. Kad traffic does not warrant it |
| **Noise IK or a WireGuard-like 1-RTT handshake** | Needs the responder's static key before the first packet just as this design does, and then keeps per-peer state for a channel that carries one request |
| **eMule-style verify keys only.** 32-bit keys echoed between nodes, no key proof | Proves the address, not the key. Node IDs would no longer be bound to keys and §6.3 would lose its lower 107 bits |
| **Public key in every contact** to skip `INIT` | +32 bytes on every contact in every answer, paid whether or not the contact is ever used; `INIT` costs ~120 bytes once per node actually contacted |
| **Stripping the QUIC Initial padding in the obfuscation layer** | The padding is inside the protected packet in the candidate library, and removing it gives up QUIC's amplification bound |

---

## 9. Deployment

1. A node that implements this sets `KAD_DGRAM`. While it is `ROUTER` it answers datagrams
   and sends `dgram_cookie`; a leaf drops `INIT` and `REQUEST` without an answer (§4 of this
   document). It keeps answering Kad over QUIC.
2. A requester uses datagrams with nodes it knows to have the bit (from `Hello`). For a contact
   it knows only from a `KadAnswer` it sends `INIT`, and if no `CHALLENGE` arrives within
   300 ms *(initial)* it opens a QUIC connection instead. The 300 ms replace the timer of §4
   step 2 of this document: the `INIT` is not retransmitted, and the missing `CHALLENGE` is
   not a failure in the sense of §10.2. A contact whose `INIT` got no `CHALLENGE` within the
   300 ms is then asked over QUIC until a `Hello` from it sets `KAD_DGRAM`. A
   `CHALLENGE` that arrives after the 300 ms only fills the cache.
3. Network upgrade (§16): from a date fixed in the specification, a node MUST set `KAD_DGRAM` to
   be accepted as `ROUTER`. From then on step 2 needs no fallback.

Compact contacts are not extended with a capability flag. The fallback in step 2 costs one small
datagram and is temporary.

---

## 10. Open points

- Test vectors for the key conversion, key derivation, cookie and one sealed exchange are in
  [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- Whether `INIT` should be allowed to piggyback on a `PROBE` during hole punching. Leaves do not
  answer datagrams, so probably not needed.
- Measured sizes, and measured loss behaviour of 1200-byte answers on real paths.
