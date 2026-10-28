# Extension: finding a known node (`friend`)

**Status:** draft 0.2 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §9, §10.1, §10.3, §13.2. § = that document unless this one is named; "§n here" = this document.
**Capability:** none. Record kind 10. One field in `Store`, one in `KadAnswer`, one link.
**Required of every `ROUTER`:** storing and returning the record kind `FRIEND` (§4 here). The rest is optional.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Friend records*.

Nodes are found via their source records. A node a user *knows* (a friend) can't be found once
its address changed. Chat ([`chat.md`](chat.md)) and browsing
([`browse.md`](browse.md)) need a connection; a node behind a NAT is in no routing table.

A node wanting to be found stores a small encrypted **record** of its addresses and relays in
Kad, at a place **changing daily**. Who finds and reads it depends on the form:

| Form | Who finds the node | What the link holds | Section |
|---|---|---|---|
| **Friends** | one other node, which it added and which added it | the node key | 1 |
| **Open link** | everyone given the link | node key and a secret | 2 |
| **Request** | nobody yet: a node asks another to become its friend | node key and a request value | 3 |

Forms differ only in how key `k` is made; record and handling (§4 here) same.

RetroShare and GNUnet find known nodes via a DHT
([`docs/gnunet-retroshare-comparison.md`](../docs/gnunet-retroshare-comparison.md) §2.1).
Pairwise keys are Briar's, the request Tox's
([`docs/tox-briar-comparison.md`](../docs/tox-briar-comparison.md) §2).

---

## 1. Friends

```
ed3k:friend?key=<node key, base32 lower case, no padding>[&r=<request value>][&dn=<url-encoded name>]
```

Friends = each node added the other's key. Nothing else exchanged.

```
ss  = X25519(my private key, the friend's public key)
k   = H("ed3k v1 friend pair", ss ‖ key of the publishing node ‖ key of the reading node)       32 bytes
```

- Ed25519 node keys used for X25519 via the conversion of `Protocol.md` §6.4. A node MUST
  refuse an all-zero `ss`. Keys in `k` = 32-byte Ed25519 node keys, not converted.
- Same `ss` both sides; own `k` per direction: A's record for B, B's for A.
- **The link is not a secret.** A node holding another's key without being added by it computes nothing.
- **Dropping a friend** = removing its key; node stops storing its record. Nobody else affected.
- A node stores records for at most 64 *(initial)* friends.
- Altered link leads to another node; checking the key is up to the two users, ed3k has no
  means.
- `key`, `s`, `r`: base32 per `Protocol.md` §13.2: written lower case without padding, read
  either way.
- Link invalid if `key` missing, repeated or not 32 bytes; `s` not 32 bytes; `r` not 8 bytes;
  or `s`, `r` or `dn` repeated.

---

## 2. Open link

```
ed3k:friend?key=<node key>&s=<secret, 32 bytes, base32>[&dn=<url-encoded name>]
```

```
k   = H("ed3k v1 friend", secret ‖ key)                                   32 bytes
```

- Everyone with the link finds the node; node needn't know who. One direction only.
- A node has at most 4 *(initial)* open links in use. Taking one back = stop storing under it; whoever should
  still find the node needs a new link.
- A holder can store false records at the place (§4 here); each costs others one failed dial.
- Link with `s` = open link. An `r` in it, valid per §1 here, is ignored: open links carry no
  request.

**This link is a credential.** A client MUST NOT write the secret to a log or show it in an
error message, and SHOULD mark such a link private wherever shown.

---

## 3. Request

A friend link MAY carry a **request value** `r`: 8 random bytes, base32. With it a link holder
can ask to be added, no second link sent back by hand.

```
target = H("ed3k v1 friend request target", key ‖ r(8) ‖ period(8))[0..16)    key and r of the node that is asked; r enters as its 8 raw bytes, the base32 text is the link grammar only; period = the day number of §4 of this document, big-endian

plain  = len(2) ‖ serialised FriendRequest ‖ zero bytes up to 324 B          len = size of the serialised message, big-endian
sealed = e(32) ‖ nonce(12) ‖ ChaCha20-Poly1305(k_req, nonce, plain)           RFC 8439, 16-byte tag, empty associated data; 384 B
k_req  = H("ed3k v1 friend request", X25519(e's private key, the asked node's public key) ‖ e ‖ key)     key = the asked node's key
```

Padding as every `sealed` (§4 here). Serialised `FriendRequest` ≤ 322 bytes.

```protobuf
message FriendRequest {
  bytes  key = 1;             // 32 B node key of the asking node
  string text = 2;            // at most 96 B
  uint64 issued = 3;          // Unix time in seconds
  repeated Signature sigs = 4;  // by the asking node, over "ed3k v1 friend request signature" ‖ asked key(32) ‖ issued(8, big-endian) ‖ text
}
```

`e`: X25519 key made for this one seal; only the asked node can open it.

**Asking node** adds the link's key first, then stores the request under the current day's
target; it stores it again every 4 h *(initial)* until it finds the other's record for it (§1 here) or 7 days
*(initial)* have passed. Each store reseals (new `e`, nonce) under that store's day target; `issued`
and signature stay the first store's. After 7 days unanswered: key stays added; node stops
looking for the other's record unless its user wants to reach it, and tells the user the request
went unanswered.

**Asked node** searches its request place at start, then every 30 min *(initial)*: previous,
current and next day's targets, so an asker with a clock ahead is found.

- Drops a request with any of: size not 384 bytes, tag fails, `len` above 322, `key` not 32 bytes, signature
  fails under `key`, `text` above 96 bytes, or `issued` more than 7 days old or more than 1 h
  *(initial)* ahead of its clock.
- Request from an already-added key: not shown (already friends); node MAY tell its user.
- Rest shown to user, each key once: declined key not reshown while its request lasts; how long
  declined keys are remembered is local. A request is never accepted without the user.
- Accept → node adds the key; friends per §1 here. Decline → nothing sent.

**The request value keeps strangers out.** No `r` → place can't be computed. Unwanted requests →
new `r`; old-`r` links can no longer ask. Friends unaffected (`r` in none of their keys). Link
without `r` can't ask at all.

---

## 4. The record

```protobuf
// in enum Kind (§10.3):
FRIEND = 10;

// in Store (§10.3):
bytes sealed = 9;             // FRIEND: exactly 384 B, padded before sealing; any other length: REJECTED

// in KadAnswer (§10.3):
repeated bytes friends = 15;  // answer to a search of kind FRIEND

message FriendRecord {
  Peer   peer = 1;            // my endpoints, and relays if I am a leaf; `id` empty
  uint64 issued = 2;          // Unix time in seconds
}
```

For §1 and §2 here:

```
k_rec   = H("ed3k v1 friend record", k)                                   32 bytes
target  = H("ed3k v1 friend target", k ‖ period(8))[0..16)                period = Unix time / 86400, rounded down, big-endian
plain   = len(2) ‖ serialised FriendRecord ‖ zero bytes up to 356 B          len = size of the serialised message, big-endian
sealed  = nonce(12) ‖ ChaCha20-Poly1305(k_rec, nonce, plain)                  RFC 8439, 16-byte tag, empty associated data; 384 B
```

A request (§3 here) has its own target and `sealed`, same fields.

**Every `sealed` is exactly 384 bytes.** Padded before sealing, so padding is encrypted and
record and request share one wire length:

| | Bytes before the ciphertext | `plain` | Tag | `sealed` | Message at most |
|---|---|---|---|---|---|
| Record (§1, §2 here) | `nonce` 12 | 356 | 16 | 384 | 354 B |
| Request (§3 here) | `e` 32, `nonce` 12 | 324 | 16 | 384 | 322 B |

- `len`: 2 bytes big-endian, size of the following serialised message; rest of `plain` zeros.
- Reader opens `sealed`, reads `len`, parses that many bytes. Drops `sealed` not 384 bytes or
  with `len` beyond the room in `plain`. Ignores bytes after the message.
- Too many relays/endpoints → leave some out. Never stored unpadded.
- AEAD associated data empty in both forms.
- `nonce`: 12 random bytes, new per seal. `k_rec` fixed for a pair or link for ever, so a nonce
  MUST NOT be used twice under it. A request's key is new with every `e` (§3 here).

**Storing node** treats every `FRIEND` record like a source record (§10.3):

- Connection-bound: one record per publisher per target; new replaces old.
- Address limits first: at most 8 records per host, 16 per site
  ([`ip-diversity.md`](ip-diversity.md) §1, §2.4) under one target, `REJECTED` beyond, before
  the target is looked at. Then at most 32 *(initial)* per target, `FULL` after. A link holder
  thus needs several sites to lock the real node out of its place.
- Lifetime 5 h *(initial)*. Index node doesn't keep it past lifetime
  ([`Protocol.md`](../Protocol.md) §11), so a dropped friend or taken-back link stops finding
  the node.
- Checks size only: `sealed` of any length other than 384 bytes → `REJECTED`. Can't read it;
  all three forms share one length, so size can't tell them apart. With some effort it can tell
  request from record: request starts with an X25519 public value, record with random bytes. Returns a target's records without saying who stored them.
- `FRIEND` search returns the target's records in `friends`, at most 32 *(initial)*, in source
  paging order ([`Protocol.md`](../Protocol.md) §10.3, **Paging**): by
  `H("ed3k v1 page", secret ‖ node ID)` of the publisher. Of a `Query` only `offset` read. Over
  QUIC the 32 always fit, `more` not set; a binding that cuts the answer
  ([`kad-datagram.md`](kad-datagram.md) §4.1) sets `more`, `offset` pages on.

**Node wanting to be found**

- stores one record per friend and per open link under the current day's target: at start, on
  accepting a friend or making a link, on address or relay change, and every 4 h *(initial)*;
- names in `peer` what a source record would hold: its endpoints, or relays when a leaf (§9.4).

**Node that looks**

1. Searches `FRIEND` at current and previous day's targets, merges answers: a false record
   under today's must not hide the real one under yesterday's. Only if neither gives an opening
   record: search next day (friend's clock may be ahead).
2. Opens each record with `k_rec`; tag fails → dropped. An answer holds at most 32 `friends`
   (one target's records); ignore those beyond the first 32.
3. Tries records by descending `issued`, each once, skipping `issued` more than 1 h *(initial)*
   in the future, the clock tolerance of `Protocol.md` §10.7. A false `issued` costs one failed
   dial, no more, as each is tried once. Dials directly, or via relays per §9.4 and §9.5.
4. **The handshake is the proof.** Node at that address must hold `key` (T1). Dial or handshake
   fails → record dropped, next tried. A record is unsigned for the same reason as a source record: no
   signature proves an address; dialling does.

- Looks when its user wants to reach the other, else at most once per 30 min *(initial)* per
  friend or link.
- User-requested look-up: SHOULD search under the pair key (§1 here) first, the other's open
  link only if that finds nothing. Only the two friends write at the pair place; any open-link
  holder at its place.
- Opens at most 256 *(initial)* sealed values per search, records and requests alike; rest
  ignored.
- Connected → ordinary peers; a friend's rights in chat, browsing, upload queue: local policy.

---

## 5. What it shows

| To whom | What |
|---|---|
| A friend, or a holder of an open link | node's addresses and relays whenever online, hence whether online |
| A node that stores a record | publisher's node ID and address, as for any record, and that it publishes a `FRIEND` record; not for whom. Can find out request vs record (§4 here) |
| The same node | addresses of those searching that target, so it can infer they know the publisher. Target moves daily, so not for long |
| Someone who has a node's key only | nothing |
| Someone who has a link with `r` | can leave requests; can't read others' |
| Someone who gets an open link later | current address. No forward secrecy |
| Someone who takes a node's private key | every place of that node, past and future |

**A friend learns the node ID.** Link key = node key, proven by handshake (T1): a friend or
open-link holder knows the node ID, hence its source records and every other trace.
*(Informative.)* A user not wanting this runs a **second node** for friends and chat: own key,
without `ROUTER` also while open (`Protocol.md` §2), so in no routing table and not competing
with the first node for the one entry per host group ([`ip-diversity.md`](ip-diversity.md)
§1). Publishes no source records, shares no credits with the first; names its relays in its
records when it has no own port. To the network an ordinary leaf; protocol unchanged. Doesn't
hide:

- address: once connected a friend sees it, may find it in the first node's records; behind a
  shared address a hint, not a proof;
- timing: two nodes coming and going together can be matched;
- budgets: both count against the same host address (§4.6).

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **An open record under the node ID** | any node ID seen anywhere reachable; no link | anyone can follow a node across addresses for its lifetime |
| **Only the open link** (draft 0.1) | one derivation | a secret many hold; one friend can't be dropped alone; friends can plant false records for each other |
| **Only pairwise keys** | no secret anywhere | can't let many find a node with one link |
| **A target a storing node can check**, made from the node key as for a hidden set ([`hidden-set.md`](hidden-set.md) §2) | one record per target, no false records | every peer the node ever connected to knows its key, so could watch the place |
| **A signed record** | false record seen without dialling | handshake proves more than a signature could |
| **A fixed target** | no re-store at midnight | same nodes see who looks for whom, for years |
| **A request sealed with a key from `r`** | no key agreement per request | every link holder reads who else asked |
| **A request sent on a connection**, as a message | no stored request | asker must first find the other, exactly what it can't do |
| **Keys that move forward every day**, the old ones deleted | a key taken later opens no earlier place | both sides must keep step; a record holds only the current address anyway |

---

## 7. Open points

- No friend list in the protocol: no online notice to friends; friends not kept connected.
- Introduction: a node passing the keys of two of its friends to each other.
- A friend that is a leaf behind a symmetric NAT is reachable only as far as §9.5 reaches.
- All numbers are first values.
