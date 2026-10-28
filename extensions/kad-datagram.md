# Extension: sessionless Kad datagrams (`kad-dgram`)

**Status:** draft 0.1 (2026-10-10). Not implemented. Not part of wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.6. § = that document; "§n here" = this document.
**Capability:** bit 5, `KAD_DGRAM`. Message types 48 to 51 (of the reserved 48–63), a second `magic`, field 8 of `Hello`.

v1 Kad exchange = a QUIC connection per node: ~2.5 KB, two round trips, for a 20-byte request
and an answer of a few hundred. Here the same `KadRequest`/`KadAnswer` go in one UDP datagram
each way, connectionless, keeping the QUIC binding's three Kad guarantees:

| Guarantee | Needed for | QUIC binding | This binding |
|---|---|---|---|
| Proof of the peer's key | low 107 bits of the routing ID (§6.3) | TLS handshake | static Diffie-Hellman; an answer or request that decrypts was made by the key holder |
| Proof of the peer's address | top 21 bits of the routing ID; no reflection | QUIC address validation | cookie for the requester; fresh transaction ID for the responder |
| Confidentiality and no fixed bytes | T2, T5 | TLS + §4 obfuscation | AEAD + the same mask |

Second binding, Kad messages only; all else on QUIC, which stays fallback for any Kad message
that does not fit.

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

- Responder keeps **no state** per requester: cookie recomputed, shared key from the two public
  keys.
- Holding R's public key and a valid cookie → `REQUEST` at once: one round trip. Both may come from
  an earlier datagram exchange or a QUIC session (§6 here).
- Stale cookie → fresh `CHALLENGE`; requester repeats the `REQUEST`.

---

## 2. Datagram format

§4.3 layout with a second `magic`:

```
salt(8) ‖ mask( magic(4) ‖ kp(1) ‖ pad(pad_len) ‖ type(1) ‖ txid(4) ‖ payload )
```

| Field | Meaning |
|---|---|
| `salt` | 8 random bytes; same first-byte rule as §4.3 |
| `magic` | `0x65 0x33 0x6B 0x02` |
| `kp` | bits 7-6 MUST be 0, else datagram dropped; bits 5-0 `pad_len` |
| `type` | 48 `INIT`, 49 `CHALLENGE`, 50 `REQUEST`, 51 `ANSWER` (from the range reserved in §5.1) |
| `txid` | 4 random bytes chosen by requester, echoed by responder |

**Mask.** As §4.3, over all after the salt, keyed with the **responder's** `obf_id`:
`dir = 0x00` for `INIT`, `REQUEST`; `dir = 0x01` for `CHALLENGE`, `ANSWER`. Responder masks its
reply without knowing the requester.

**Receiving.** §4.5 step 2 accepts either magic. New step 2a in §4.5, between steps 2 and 3:
datagram from an address and port with a pending exchange → unmask with that responder's `obf_id`,
`dir = 0x01`; match → hand to the exchange. Datagram from an address with a live QUIC connection
that fails `magic` under the connection key goes on to the next §4.5 step like any other; step 2a
catches a `CHALLENGE`/`ANSWER` from a responder that is also the client of a QUIC connection to this node.

Datagram of this binding MUST NOT exceed 1200 bytes of UDP payload.

### 2.1 `INIT` (48)

No payload. `pad_len` MUST be at least 52 → datagram ≥ 70 bytes, so a `CHALLENGE` with a
key-puzzle solution ([`key-puzzle.md`](key-puzzle.md) §3) never exceeds the `INIT` it answers.
Receiver drops `INIT` with `pad_len` below 52 or bytes after `txid`, as §2.2 here drops a wrong
payload length.

### 2.2 `CHALLENGE` (49)

```
cookie(8) ‖ responder_public_key(32)
```

Payload after `txid`: 40 bytes, or 52, with a key-puzzle solution after the key
([`key-puzzle.md`](key-puzzle.md) §3); length tells which, any other → dropped. Datagram = 58 or 70
bytes + padding. MUST NOT be longer than the datagram it answers. Replies to `INIT` and to a
`REQUEST` with invalid cookie.

Requester MUST check `H("ed3k v1 node id", responder_public_key)[0..16)` equals the node ID it
meant to contact, else drop.

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
| `nf` | bit 0: sender `ROUTER`; bit 1: sender `INDEX`; bit 7 (`ANSWER` only): fresh cookie follows. Other bits MUST be 0 |
| `tag` | varint `(type << 1) \| z` exactly as §5.1. Allowed: 3 `KadRequest` in `REQUEST`; 4 `KadAnswer` or 2 `Error` in `ANSWER` |
| `body` | protobuf message to end of plaintext; no length field. zstd as §5.3 when `z = 1` |

Other `nf` bit set, or `tag` not allowed for the type → dropped after opening, no answer.

`nf` is informational: doesn't make a contact `ROUTER`/`INDEX` for a routing table; that takes a
`Hello` (§3.4 here).

---

## 3. Cryptography

### 3.1 Keys

Each node's Ed25519 identity key is used for X25519 via the conversion of `Protocol.md` §6.4.

```
ss            = X25519(own x_private, peer x_public)        reject an all-zero result
k_req ‖ k_ans = H("ed3k v1 kad dgram", ss ‖ requester_public_key ‖ responder_public_key)   64 bytes
```

Derivation uses the Ed25519 keys as sent, not converted. Same `ss` on both sides; keys depend
on who asks, so an exchange's two directions, and a node pair's two roles towards each other,
never share a key. A node SHOULD cache `k_req ‖ k_ans` per peer key
and role.

### 3.2 Sealing

ChaCha20-Poly1305 (RFC 8439), 16-byte tag.

```
nonce = salt(8) ‖ txid(4)                       of the datagram being sealed
aad   = type(1) ‖ txid(4) ‖ everything in the payload before the ciphertext
        for ANSWER also ‖ req_hash, so that an answer is bound to its request
req_hash = H("ed3k v1 kad req", type(1) ‖ txid(4) ‖ payload)[0..8)    of the REQUEST being answered
seal_req uses k_req, seal_ans uses k_ans
```

`req_hash` covers the **unmasked** `REQUEST` bytes: `type` (50), `txid`, whole payload, i.e.
`requester_public_key ‖ cookie ‖` ciphertext with tag (§2.3 here). Not `salt`, `magic`, `kp`,
padding.

Each `REQUEST` copy has its own `req_hash`: retransmission → new salt, new ciphertext; copy after
a `CHALLENGE` also new cookie. Requester keeps `req_hash` of every copy sent per `txid` (at most
three, §4 here), accepts an `ANSWER` opening under any. Responder uses the answered copy's hash.

`salt` fresh per datagram, retransmissions included → nonce never repeats under a key in
practice: 64 random bits, `txid` separates exchanges. Retransmission resealed under its new salt,
never resent as same bytes.

### 3.3 Cookie

```
cookie = BLAKE3_keyed(secret_e, ip ‖ port(2, big-endian))[0..8)
```

`secret_e`: random 32 bytes, replaced by the responder hourly *(initial)*. Current- or
previous-secret cookies valid → lifetime one to two hours. `ip` is 4 or 16 bytes.

`REQUEST` with previous-secret cookie: responder SHOULD set `nf` bit 7 and include a current one
(saves a later `CHALLENGE` round trip).

### 3.4 What each side has proven

| After | Proven |
|---|---|
| Responder accepts a `REQUEST` | Cookie matches source address: requester receives there. Seal opens under `k_req`: requester holds the private key of `requester_public_key`. So its node ID and routing ID are **verified** per §6.3 |
| Requester accepts an `ANSWER` | Seal opens under `k_ans` with a pending request's `txid`: made by the responder key's holder, after seeing a request sent to that address. Responder **verified** |

Verified here = same standing as verified by QUIC handshake, except: datagram has no `Hello`, so
no `nat = 1`. Contact first seen via Kad datagram not inserted until a `Hello` on a connection of
that family says `ROUTER` and `nat = 1` (`Protocol.md` §10.2). Contact first verified by a
`REQUEST` it sent is dialled-in, like one that opened a connection.

### 3.5 Properties and limits

- **No forward secrecy.** Static keys per pair; stolen identity key → thief reads that node's
  recorded Kad datagrams. Kad requests aren't secret from the network: any node can be asked the same. Peer
  sessions and file transfers stay on QUIC with full TLS.
- **Replay.** Recorded `REQUEST` replays only from the same source address, forged or not, only
  while its cookie is valid (1–2 h). Changes nothing on the responder: no `REQUEST` stores
  anything (§4 here). Costs one more answer, up to 1200 bytes for ~96, to the real requester;
  bounded by the `REQUEST` rate of §5 here. Replayed `ANSWER` matches no pending `txid`, dropped.
- **One key, two uses.** Ed25519→X25519 conversion is established practice; context strings
  separate DH output from every other key use.
- **Requester's key visible.** `REQUEST` carries `requester_public_key` under the mask only (§2.3
  here): anyone knowing the responder's node ID unmasks it, learns who asked; QUIC keeps it in
  TLS.
- **Not a general transport.** No streams, no ordering, no retransmission beyond §4 here, no
  congestion control beyond the limits of §5 here.

---

## 4. Rules for an exchange

**Requester**

1. No responder key, or key but no cookie: send `INIT`, wait for `CHALLENGE` (step 3) with the
   step 2 timer; no `REQUEST` before it. Contact not known to set `KAD_DGRAM`: 300 ms *(initial)*
   replace the step 2 timer; when they pass: no `INIT` retransmit, no failure counted, open QUIC
   (§9 here). Later `CHALLENGE` with that `txid` only fills the cache (key, cookie); no
   `REQUEST`.
2. Key and cookie: send `REQUEST`, start 1 s *(initial)* timer (requester knowing the RTT to the
   responder MAY use twice that). Each `INIT`/`REQUEST` sent restarts it.
3. `CHALLENGE` with same `txid`: store cookie, key; send `REQUEST`, same `txid`, new `salt`. Once
   per `txid`; later `CHALLENGE` only fills the cache.
4. Timeout: resend last datagram, new `salt`, same `txid`; `INIT` at most once, `REQUEST` at most
   once. At most 3 `REQUEST` copies per `txid` (§3.2 here). Timer out, nothing resendable →
   failed; contact counts one failure as §10.2. Step doesn't apply to the `INIT` to a contact not known to set
   `KAD_DGRAM` (step 1).
5. Accept first valid `ANSWER` per `txid` (opens under the `req_hash` of any copy sent for it,
   §3.2 here), ignore later.
6. Never put a `KadRequest` with `store` in a `REQUEST`. A store lookup MAY walk by datagram with
   the same request minus `store`; the `store` request goes over QUIC. Nodes set `responsible`
   whether or not the request carries an operation (`Protocol.md` §10.3), so the walk ends as
   over QUIC.
   On `Error{UNSUPPORTED}` for a request, repeat it over QUIC.

**Responder**

1. `INIT`: reply `CHALLENGE`, subject to the limits of §5 here. Nothing else.
2. `REQUEST`: cookie first. Invalid → reply `CHALLENGE`, stop; no Diffie-Hellman for an
   unvalidated address.
3. Open seal; failure → drop silently.
4. `KadRequest` with `store` not accepted over datagrams: nothing stored; `ANSWER` carries
   `Error{UNSUPPORTED}` (§5.6) → requester repeats over QUIC.
5. Any other `KadRequest`: exactly as §10.3 prescribes; where it names the connection's family
   or address, use the datagram's family and source address.
6. Reply with one `ANSWER`.

Only a `ROUTER` answers; a leaf only requests, drops `INIT` and `REQUEST` unanswered.

### 4.1 Fitting one datagram

No fragmentation. Sealed `ANSWER` leaves ~1150 bytes for the body, `REQUEST` ~1110.

- Responder fills `KadAnswer` in order, stopping when the next item won't fit: contacts (closest
  first, families interleaved by distance), then records in the paging order of §10.3. Records
  dropped only from the end of that order; next request's `offset` counts carried ones. `http`,
  `links` only for carried sources, dropped before any record; requester can't see that, gets
  them over QUIC or from the source ([`http-source.md`](http-source.md) §2.1,
  [`hoster.md`](hoster.md) §2). Records left out → set `more`. `sites`: one entry per carried
  record; `keys`: every key a carried record references. zstd before measuring.
- On `more`: ask again with `Query.offset`, or open QUIC for the rest.
- `more` with no record → contacts left no room: ask again with `want = 0` (no contacts). Still
  no record → one record doesn't fit: go QUIC, no third datagram try. Request already with
  `want = 0`: QUIC at once when its answer sets `more` with no record.
- A `KadRequest` that does not fit (in practice a many-word search) MUST be sent over QUIC. A
  `store` request goes over QUIC regardless of size (§4 here).

16 IPv4 contacts = 352 bytes, 16 IPv6 = 544, so a find-node answer for both families always fits.

---

## 5. Abuse limits

| Risk | Rule |
|---|---|
| Reflection | `CHALLENGE` never longer than its trigger; nothing else to an address without valid cookie. Responder SHOULD send at most 4 *(initial)* `CHALLENGE`s per second to one host ([`ip-diversity.md`](ip-diversity.md) §1) |
| Amplification | `ANSWER` only to a cookie-validated address |
| CPU | Cookie check (one keyed hash) first. Derived keys cached per peer key; new derivations limited per host ([`ip-diversity.md`](ip-diversity.md) §1) to 8 per second *(initial)* |
| Request floods and replay | Responder processes at most 16 / 64 / 256 *(initial)* `REQUEST`s per second per host / site / block ([`ip-diversity.md`](ip-diversity.md) §1), counting valid-cookie `REQUEST`s. Over a limit → dropped, no answer |
| Flooding of records | `KadRequest` with `store` not accepted over datagrams; answered `Error{UNSUPPORTED}` (§4 here). Searches: §10.3 per-address, per-kind policy unchanged |
| Requester load | At most 16 *(initial)* exchanges outstanding total, 2 per responder |

Private address (§8.2 (8)): exempt from rows *Reflection*, *CPU*, *Request floods and replay*
where §4.6 exempts it from the host-address row: in a named network (§4.3).

---

## 6. Getting key and cookie without `INIT`

`Hello` (§5.5) gains one field:

```protobuf
message Hello {
  // ...
  bytes dgram_cookie = 8;     // 8 B; sent by a ROUTER node with KAD_DGRAM: the cookie it accepts from your endpoint
}
```

After any QUIC session with a `ROUTER` setting `KAD_DGRAM`, a node has its public key (handshake)
and a cookie, usable at once. Cookie is tied to the peer's endpoint on that connection: its source
if the peer dialled, the dialled endpoint if the `KAD_DGRAM` node did. Address change → fails,
replaced via `CHALLENGE`.

---

## 7. Cost

Sizes: whole UDP payloads w/o padding, estimated from the layouts above.

| Datagram | Fixed part | Typical total |
|---|---|---|
| `INIT` | 18 (70 with the pad of §2.1 here) | 70–81 |
| `CHALLENGE` | 58 (70 with a key-puzzle solution) | 58–70 |
| `REQUEST` | 76 + body | ~96 for a find-node, ~140 for a search |
| `ANSWER` | 36 + body (+8 with cookie) | ~390 with 16 IPv4 contacts |

| | QUIC binding (§10.5) | This binding | eMule Kad |
|---|---|---|---|
| Exchange with a known node | ~2.5 KB handshake + messages, 2 round trips (1 with 0-RTT) | ~0.5 KB, 1 round trip | ~0.3–0.4 KB, 1 round trip |
| First contact with a node | same | + ~120 B and 1 round trip | + verify-key round trip in some cases |
| Lookup with a search, 8–12 nodes | 15–25 KB | 5–7 KB | 3–5 KB |

No store by datagram (§4 here): store lookup saves only waypoint connections; each storing node
still costs a QUIC connection (§10.5).

Remaining gap to eMule Kad: requester's 32-byte key, 16-byte tags; they buy authenticated node IDs.

`KadAnswer` has no `observed` endpoint; an all-datagram Kad node still learns its own address
from relay and peer sessions (§9.2).

---

## 8. Alternatives considered

| Design | Why not |
|---|---|
| **Signatures instead of DH.** Requester signs the request, responder signs the answer | 64-byte signature + 32-byte key each way, no confidentiality beyond the mask; signature check no cheaper than a cached DH key |
| **discv5-style sessions.** WHOAREYOU challenge, ephemeral handshake, per-peer session keys | Forward secrecy, at the price of session state both sides, a handshake packet type with own retransmission rules, key rotation. Not warranted for Kad traffic |
| **Noise IK or a WireGuard-like 1-RTT handshake** | Needs responder's static key before the first packet, as here, then per-peer state for a one-request channel |
| **eMule-style verify keys only.** 32-bit keys echoed between nodes, no key proof | Proves address, not key. Node IDs unbound from keys; §6.3 loses its lower 107 bits |
| **Public key in every contact** to skip `INIT` | +32 bytes per contact per answer, used or not; `INIT` costs ~120 bytes once per node actually contacted |
| **Stripping the QUIC Initial padding in the obfuscation layer** | Padding is inside the protected packet in the candidate library; removing it gives up QUIC's amplification bound |

---

## 9. Deployment

1. Implementing node sets `KAD_DGRAM`. As `ROUTER`, answers datagrams, sends `dgram_cookie`; a leaf
   drops `INIT` and `REQUEST` unanswered (§4 here). Still answers Kad over QUIC.
2. Requester uses datagrams with nodes known (from `Hello`) to have the bit. Contact known only from a
   `KadAnswer`: `INIT`; no `CHALLENGE` within 300 ms *(initial)* → QUIC instead. The 300 ms
   replace the timer of §4 step 2 here: no `INIT` retransmit, missing `CHALLENGE` no failure in
   the sense of §10.2. Such a contact is then asked over QUIC until a `Hello` from it sets
   `KAD_DGRAM`. Later `CHALLENGE` only fills the cache.
3. Network upgrade (§16): from a date fixed in the specification, a node MUST set `KAD_DGRAM` to
   be accepted as `ROUTER`. Then step 2 needs no fallback.

No capability flag in compact contacts: step 2 fallback costs one small datagram, temporary.

---

## 10. Open points

- Test vectors (key conversion, key derivation, cookie, one sealed exchange):
  [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- Whether `INIT` may piggyback on a `PROBE` during hole punching. Leaves don't answer datagrams,
  so probably not needed.
- Measured sizes, and measured loss behaviour of 1200-byte answers on real paths.
