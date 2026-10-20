# Extension: finding a known node (`friend`)

**Status:** draft 0.2 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §9, §10.1, §10.3 and §13.2. Section numbers (§) refer to that document unless this one is named.
**Capability:** none. Record kind 10. One field in `Store`, one in `KadAnswer`, one link.
**Required of every `ROUTER`:** storing and returning the record kind `FRIEND` (§4 of this document). The rest is optional.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Friend records*.

A node is found through what it shares: its source records. A node that a user *knows*, a
friend, cannot be found at all once its address has changed. Chat
([`chat.md`](chat.md)) and browsing ([`browse.md`](browse.md)) need a connection, and a node
behind a NAT is in no routing table.

A node that wants to be found stores a small **record** of its addresses and relays in Kad. The
record is encrypted, and its place **changes every day**. Who can find and read it depends on
the form:

| Form | Who finds the node | What the link holds | Section |
|---|---|---|---|
| **Friends** | one other node, which it has added and which has added it | the node key | 1 |
| **Open link** | everyone the link was given to | the node key and a secret | 2 |
| **Request** | nobody yet: a node asks another to become its friend | the node key and a request value | 3 |

The forms differ only in how a key `k` is made. The record and its handling (§4 of this
document) are the same.

RetroShare and GNUnet find known nodes through a DHT
([`docs/gnunet-retroshare-comparison.md`](../docs/gnunet-retroshare-comparison.md) §2.1). The
pairwise keys are Briar's, the request is Tox's
([`docs/tox-briar-comparison.md`](../docs/tox-briar-comparison.md) §2).

---

## 1. Friends

```
ed3k:friend?key=<node key, base32 lower case, no padding>[&r=<request value>][&dn=<url-encoded name>]
```

Two nodes are friends when each has added the key of the other. Nothing else is exchanged.

```
ss  = X25519(my private key, the friend's public key)
k   = H("ed3k v1 friend pair", ss ‖ key of the publishing node ‖ key of the reading node)       32 bytes
```

- The Ed25519 node keys are used for X25519 through the conversion of `Protocol.md` §6.4. A
  node MUST refuse an `ss` that is all zero. The keys in `k` are the 32-byte Ed25519 node
  keys, not the converted ones.
- Both nodes compute the same `ss`. Each direction has a `k` of its own: one for the record A
  stores for B, one for the record B stores for A.
- **The link is not a secret.** Whoever has a node's key and was not added by that node can
  compute nothing.
- **Dropping a friend** is removing its key. The node stops storing the record for it. Nobody
  else is affected.
- A node stores records for at most 64 *(initial)* friends.
- A link that was changed on its way leads to another node. That the key is the right one is
  for the two users to check; ed3k has no means for it.
- `key`, `s` and `r` are base32 as `Protocol.md` §13.2 fixes: written lower case without
  padding, read either way.
- A link is invalid when `key` is missing, repeated or not 32 bytes, when `s` is not 32 bytes,
  when `r` is not 8 bytes, or when `s`, `r` or `dn` is repeated.

---

## 2. Open link

```
ed3k:friend?key=<node key>&s=<secret, 32 bytes, base32>[&dn=<url-encoded name>]
```

```
k   = H("ed3k v1 friend", secret ‖ key)                                   32 bytes
```

- Everyone who has the link finds the node. The node need not know who they are.
- It works in one direction.
- A node has at most 4 *(initial)* open links in use. It takes one back by no longer storing
  under it, and everyone who should still find it needs a new link.
- A holder of the link can store false records at the place (§4 of this document). Each costs
  the others one failed dial.
- A link with `s` is an open link. An `r` in it, valid as §1 of this document requires, is
  ignored: an open link carries no request.

**This link is a credential.** A client MUST NOT write the secret to a log or show it in an
error message, and SHOULD mark such a link as private wherever it shows it.

---

## 3. Request

A friend link MAY carry a **request value** `r`: 8 random bytes in base32. With it, a node that
was given the link can ask to be added, without a second link going back by hand.

```
target = H("ed3k v1 friend request target", key ‖ r(8) ‖ period(8))[0..16)    key and r of the node that is asked; r enters as its 8 raw bytes, the base32 text is the link grammar only; period = the day number of §4 of this document, big-endian

plain  = len(2) ‖ serialised FriendRequest ‖ zero bytes up to 324 B          len = size of the serialised message, big-endian
sealed = e(32) ‖ nonce(12) ‖ ChaCha20-Poly1305(k_req, nonce, plain)           RFC 8439, 16-byte tag, empty associated data; 384 B
k_req  = H("ed3k v1 friend request", X25519(e's private key, the asked node's public key) ‖ e ‖ key)     key = the asked node's key
```

The padding is that of every `sealed` (§4 of this document). A serialised `FriendRequest` is at
most 322 bytes.

```protobuf
message FriendRequest {
  bytes  key = 1;             // 32 B node key of the asking node
  string text = 2;            // at most 96 B
  uint64 issued = 3;          // Unix time in seconds
  repeated Signature sigs = 4;  // by the asking node, over "ed3k v1 friend request signature" ‖ asked key(32) ‖ issued(8, big-endian) ‖ text
}
```

`e` is an X25519 key made for this one seal. Only the asked node can open it.

**The asking node** adds the key of the link first, and then stores the request under the
target of the current day. It stores it again every 4 h *(initial)*, until it finds the other
node's record for it (§1 of this document) or 7 days *(initial)* have passed. Each store seals
the request anew, with a new `e` and nonce, under the target of the day of that store; `issued`
and the signature stay those of the first store. After the 7 days without an answer the key
stays added. The node stops looking for the other's record, except when its user wants to
reach it, and tells its user that the request went unanswered.

**The asked node** searches its request place when it starts and then once per 30 min
*(initial)*: the targets of the day before, the current day and the day after, so that an
asking node whose clock is ahead is found.

- It drops a request that is not 384 bytes, whose tag fails, whose `len` is above 322, whose
  `key` is not 32 bytes, whose signature does not verify under `key`, whose `text` is above
  96 bytes, or whose `issued` is more than 7 days old or more than 1 h *(initial)* ahead of
  its clock.
- A request from a key the node has already added is not shown: the two are friends already.
  The node MAY tell its user.
- It shows the rest to the user, each key once: a key the user declined is not shown again
  while its request lasts. How long a declined key is remembered is local. A request is never
  accepted without the user.
- When the user accepts, the node adds the key. Both are then friends by §1 of this document.
- When the user declines, nothing is sent.

**The request value keeps strangers out.** Without `r` the place cannot be computed. A node
that gets unwanted requests makes a new `r`: links with the old one can no longer ask.
Friends are not affected, because `r` is in none of their keys. A link without `r` cannot be
used to ask at all.

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

For §1 and §2 of this document:

```
k_rec   = H("ed3k v1 friend record", k)                                   32 bytes
target  = H("ed3k v1 friend target", k ‖ period(8))[0..16)                period = Unix time / 86400, rounded down, big-endian
plain   = len(2) ‖ serialised FriendRecord ‖ zero bytes up to 356 B          len = size of the serialised message, big-endian
sealed  = nonce(12) ‖ ChaCha20-Poly1305(k_rec, nonce, plain)                  RFC 8439, 16-byte tag, empty associated data; 384 B
```

A request (§3 of this document) has its own target and its own `sealed`, and travels in the
same fields.

**Every `sealed` is exactly 384 bytes.** The message is padded before it is sealed, so the
padding is encrypted and a record and a request have one length on the wire:

| | Bytes before the ciphertext | `plain` | Tag | `sealed` | Message at most |
|---|---|---|---|---|---|
| Record (§1, §2 of this document) | `nonce` 12 | 356 | 16 | 384 | 354 B |
| Request (§3 of this document) | `e` 32, `nonce` 12 | 324 | 16 | 384 | 322 B |

- `len` is 2 bytes, big-endian: the size of the serialised message that follows it. The rest
  of `plain` is zero bytes.
- A reader opens `sealed`, reads `len` and parses that many bytes. It drops a `sealed` that is
  not 384 bytes, and one whose `len` is larger than the room `plain` has. It ignores the
  bytes after the message.
- A node that has more relays or endpoints than fit leaves some out. A record is never
  stored unpadded.
- The associated data of the AEAD is empty in both forms.
- `nonce` is 12 random bytes, new for every seal. `k_rec` stays the same for a pair or a link
  for ever, so a nonce MUST NOT be used twice under it. The key of a request is new with
  every `e` (§3 of this document).

**The storing node** treats every `FRIEND` record like a source record (§10.3):

- It is bound to the connection: one record per publishing node under a target, and a new one
  replaces the old.
- It checks the address limits first: at most 8 records per host and 16 per site
  ([`ip-diversity.md`](ip-diversity.md) §1, §2.4) under one target, `REJECTED` beyond, before
  the target is looked at. Then it holds at most 32 *(initial)* records under one target and
  answers `FULL` after that. A holder of the link thus needs several sites to lock the real
  node out of its place.
- The lifetime is 5 h *(initial)*. An index node does not keep it past its lifetime
  ([`Protocol.md`](../Protocol.md) §11), so that a dropped friend or a taken-back link stops
  finding the node.
- Of the content it checks the size and nothing else: a `sealed` of any length other than 384 bytes is
  `REJECTED`. It cannot
  read the record. All three forms have one length, so it cannot tell them apart by their
  size. It can tell a request from a record with some effort: a request begins with an
  X25519 public value, a record with random bytes. It returns the records of a target without saying who stored them.
- A search of kind `FRIEND` returns the records of the target in `friends`, at most 32
  *(initial)*, in the order sources are paged ([`Protocol.md`](../Protocol.md) §10.3,
  **Paging**): by `H("ed3k v1 page", secret ‖ node ID)` of the publishing node. Of a `Query`
  only `offset` is read. Over QUIC the 32 always fit and `more` is not set; a binding that
  cuts the answer ([`kad-datagram.md`](kad-datagram.md) §4.1) sets `more`, and `offset` pages
  on.

**The node that wants to be found**

- stores a record for each friend and each open link, under the target of the current day:
  when it starts, when it accepts a friend or makes a link, when its addresses or relays
  change, and every 4 h *(initial)*;
- names in `peer` what it would put into a source record: its endpoints, or its relays when it
  is a leaf (§9.4).

**The node that looks**

1. Searches `FRIEND` at the targets of the current day and of the day before, and merges the
   answers: a false record under today's target must not hide the real one under
   yesterday's. Only when both give no record that opens, it searches the day after (the
   friend's clock may be ahead).
2. Opens every record with `k_rec`. One whose tag fails is dropped. An answer holds at most
   32 `friends`, the records of one target; the node ignores those beyond the first 32.
3. Tries the records in descending order of `issued`, each once, leaving out those whose
   `issued` is more than 1 h *(initial)* in the future, the clock tolerance of `Protocol.md`
   §10.7. A record with a false `issued` costs one failed dial and no more, since each is
   tried once. For each it dials: directly, or
   through the relays as in §9.4 and §9.5.
4. **The handshake is the proof.** The node at that address has to hold `key` (T1). A record
   whose dial or handshake fails is dropped, and the next one is tried. A record
   is not signed for the same reason a source record is not: no signature can prove an
   address, and dialling does.

- A node looks when its user wants to reach the other, and otherwise at most once per 30 min
  *(initial)* per friend or link.
- For a look-up its user asks for, a node SHOULD search under the pair key (§1 of this
  document) first, and use an open link of the other only when that finds nothing. Only the
  two friends can write at the pair place; any holder of an open link can write at its place.
- A node opens at most 256 *(initial)* sealed values of one search, records and requests
  alike; further ones are ignored.
- Once connected, the two nodes are ordinary peers. What a friend is allowed, in chat, in
  browsing or in the upload queue, is local policy.

---

## 5. What it shows

| To whom | What |
|---|---|
| A friend, or a holder of an open link | the node's addresses and relays whenever it is online, and so whether it is online |
| A node that stores a record | the node ID and address of the publisher, as for any record, and that it publishes a `FRIEND` record. Not for whom. Whether it is a request or a record it can find out (§4 of this document) |
| The same node | the addresses of those who search that target. It can conclude that they know the publisher. The target moves every day, so no node sees this for long |
| Someone who has a node's key only | nothing |
| Someone who has a link with `r` | can leave requests. Cannot read the requests of others |
| Someone who gets an open link later | the node's current address. There is no forward secrecy |
| Someone who takes a node's private key | every place of that node, past and future |

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **An open record under the node ID** | any node ID seen anywhere can be reached; no link | anyone can follow a node from address to address for as long as it exists |
| **Only the open link** (draft 0.1) | one derivation | the link is a secret that many hold; one friend cannot be dropped alone; friends can plant false records for each other |
| **Only pairwise keys** | no secret anywhere | no way to let many people find a node with one link |
| **A target a storing node can check**, made from the node key as for a hidden set ([`hidden-set.md`](hidden-set.md) §2) | one record per target, no false records | every peer the node ever connected to knows its key, so each could watch the place |
| **A signed record** | a false record is seen without dialling | the handshake proves more than a signature could |
| **A fixed target** | nothing to store again at midnight | the same nodes see who looks for whom, for years |
| **A request sealed with a key from `r`** | no key agreement per request | every holder of the link reads who else asked |
| **A request sent on a connection**, as a message | no stored request | the asking node has to find the other first, which is what it cannot do |
| **Keys that move forward every day**, the old ones deleted | a key taken later opens no earlier place | both sides have to keep step; a record only holds the current address anyway |

---

## 7. Open points

- A list of friends is not part of the protocol: there is no message that tells a friend one
  is online, and friends are not kept connected.
- An introduction: one node passing the keys of two of its friends to each other.
- A friend that is a leaf behind a symmetric NAT can be reached only as far as §9.5 reaches
  it.
- All numbers are first values.
