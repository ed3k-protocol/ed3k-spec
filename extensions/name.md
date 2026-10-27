# Extension: readable names (`name`)

**Status:** draft 0.2 (2026-10-27, after the audit of that day). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §2, §5.1, §10.1, §10.3, §10.7 and §13.2, and [`file-set.md`](file-set.md) §5. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 20, `NAMES`: answers `NameQuery` (§4.3). Message types 44 and 45; record kinds 14 `NAME` and 15 `CLAIM`; two fields in `Record`, one in `Store`, one in `KadAnswer`; a name list file format; `ed3k:name` links; a DNS record and a file at a web server.
**Required of every `ROUTER`:** storing and returning the record kinds `NAME` and `CLAIM` (§3.2, §4.2). The rest is optional.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Names*.

Every thing in ed3k that has an author is named by a key: a set that changes, a channel, a room,
a friend. A key is 52 characters of base32, which nobody remembers or reads out. This extension
gives such things names that people can read: `blog`, `shop.blog` under a key, `alice.ed3k`,
`example.com`.

No name can be readable, the same for everyone, and safe from being taken all at once
(Zooko's triangle). ed3k gives up a different side in each form, and says in each place which
one it gave up:

| Form | Example | Readable | Same for everyone | Who decides | Section |
|---|---|---|---|---|---|
| **Name under a key** | `shop.blog` under the key of a zone | yes, below the key | yes | the holder of the key | 3 |
| **Global name** | `alice.ed3k` | yes | no: what a user's own names, lists and friends say | the user, then friends, then the network | 4 |
| **Name from the web** | `example.com`, `alice@example.com` | yes | yes | whoever holds the domain | 5 |
| **Petname** | `Mum`, `my band` | yes | no: only the user has it | the user | 4.1 |

What a name leads to is an **ed3k link**: an `ed3k:set` that changes (a website, §6), an
`ed3k:friend` link, an `ed3k:room`, an `ed3k:live` channel, an `ed3k:file`, an `ed3k:node`.
A name never leads to a secret: a link with `s`, an `ed3k:pass` with `k` and an `ed3k:cache`
link are refused wherever a name record or a domain carries one.

A website is a set that changes ([`file-set.md`](file-set.md) §5): a signed list of files
under a key, with a head that leads to the newest version. A client shows it by handing it to
the user's web browser from a local port (§6). ed3k carries no code and the client runs
none.

---

## 1. Names

### 1.1 Labels

A **label** is 1 to 63 bytes of `a`–`z`, `0`–`9` and `-`. It does not begin or end with `-`, and
it does not begin with `xn--`. A **name** is labels joined by `.`, at most 8 *(initial)* labels
and at most 253 bytes, the limit of DNS.

- Names are in lower case on the wire and in every record; a storing node or a reader refuses a
  name with another byte. A client turns what the user types into lower case first.
- Only ASCII. A label in another script, also in the `xn--` form of IDNA, would let two names
  look alike that are not (`pаypal` with a Cyrillic `а`). Names in any script are petnames
  (§4.1), which only their user sees. This holds for domains and handles as well: a domain
  with an `xn--` label has no ed3k name.
- A **global label** (§4) is a label of 3 to 63 bytes.

### 1.2 Three kinds of name

How a client reads a name it was given without a key:

| Name | Kind | Resolved by |
|---|---|---|
| ends in `.ed3k` | global name: the label before `.ed3k` is the global label, the labels in front of it a name in that zone | §4, then §3 |
| has an `@` | a handle at a domain: `alice@example.com` | §5 |
| any other name of two labels or more | a domain: `example.com`, `blog.example.com` | §5 |

A name given with a key (§1.3) is a name in the zone of that key (§3).

`.ed3k` is not a top-level domain of DNS. Should one of that name be delegated there, a client
still reads `.ed3k` as this document says.

### 1.3 Links

```
ed3k:name?key=<root key of the zone, base32 lower case, no padding>[&n=<name>][&path=<url-encoded path>][&dn=<url-encoded display name>]
ed3k:name?n=<global name ending in .ed3k, domain or handle>[&path=<url-encoded path>][&dn=<url-encoded display name>]
```

- With `key`, `n` is a name in that zone; without `n` the link names the zone itself (its apex,
  §3.1).
- Without `key`, `n` is required and is one of the forms of §1.2.
- `path` selects a page of a website, or a member or directory of a set, as `path` of
  `ed3k:set` does ([`file-set.md`](file-set.md) §5.4).
- `dn` is shown while the name is being resolved; it decides nothing.
- A link is invalid when `key` does not decode to 32 bytes, when `n` is not a name of §1.1 (a
  handle: its local part as in §5.1 and its domain a domain of §1.2, not ending in `.ed3k`;
  `n=` empty is not a name), when a link without `key` has no
  `n`, or when `key`, `n`, `path` or `dn` is repeated. Base32 is as `Protocol.md` §13.2 fixes.
  Unknown parameters are ignored.
- A link without `key` is not a secure name. A client shows how it resolved it (§4.4) before it
  opens it for the first time.
- What a user types as one label without `.ed3k` (`alice`) is a petname if the user has one by
  that text, else `alice.ed3k`. In a link a name of one label without `key` is invalid.

---

## 2. Root key and signing key

A name lives for years; a key on a machine that is online every day can be lost or taken. A
zone therefore MAY keep its key offline. That **root key** names the zone and signs a
**delegation**: a short-lived **signing key** that signs the records.

```protobuf
message Delegation {
  bytes  key = 1;             // 32 B Ed25519 signing key
  uint64 issued = 2;          // Unix time in seconds
  uint64 expires = 3;         // Unix time in seconds
  uint64 scope = 4;           // bit 0: NAME records (§3); every other bit is reserved
  repeated Signature sigs = 5;    // by the root key, over the layout below
}
```

Each signature is computed over

```
"ed3k v1 delegation" ‖ root key(32) ‖ key(32) ‖ issued(8) ‖ expires(8) ‖ scope(8)
```

with the integers big-endian. The rules for `sigs` are those of `Protocol.md` §10.7.

A delegation is **valid** when:

- `key` is 32 bytes and not the root key;
- its signatures verify under the root key;
- `issued` is at most 1 h *(initial)* ahead of the reader's clock, `expires` is later than
  `issued` and at most 90 days *(initial)* after it, and `expires` has not passed;
- `scope` has the bit of the record that carries it.

A root key MUST NOT sign two delegations with the same `issued`.

**Replacing a signing key.** The root key signs a new delegation with a later `issued`, and the
owner stores the apex record (§3.1) under it first. A record under the newer delegation wins over
every record under an older one (§3.2), whatever its `seq`, and the apex sets the **floor** of
the zone: a searcher refuses every record of the zone under an older delegation (§3.3). A
signing key that was taken is therefore replaced by one signature of the root key and one
store, and every record it signed, also for names the zone never used, is refused from then on.
A signing key that was lost is replaced the same way.

**What it does not cover.** A root key that is taken is the end of the zone: the thief can
sign delegations as well as the owner. A root key that is lost leaves the zone with the last
delegation until it expires. Pins (§4.4) tell the users of a zone that moved to a new root key
that it changed; they cannot tell them that it is still the same owner.

A zone that does not want two keys signs its records with the root key itself. Such a record
loses against every valid record under a delegation (§3.2), so a zone can start with one key
and move to a signing key at any time. The way back is not open: a zone that used a delegation
signs under a delegation until the last one it signed has expired, up to 90 days.

`scope` is a list of bits so that other records signed for a key that stays offline can use
the same message later: the heads of sets, live channels and rooms each name the successor of
their key as an open point ([`file-set.md`](file-set.md) §10, [`live.md`](live.md),
[`room.md`](room.md)).

---

## 3. Names under a key

### 3.1 Zones

A **zone** is the set of names under one root key. Its owner gives each name one record. The
name `""`, the **apex**, is the zone itself: a website at the apex is what `ed3k:name?key=…`
opens. A zone that signs under a delegation MUST keep an apex record under its newest
delegation, with neither links nor `sub` if the apex is not in use: it is the floor of §2.

A name may have dots (`shop.blog`), and a record may hand every name below it to another zone
(`sub`, below): `x.shop.blog` is then the name `x` in that zone. This is how a zone gives a
name to someone else without giving them its key. Before it stores a record with `sub`, the zone
takes back every name below it that it gave a record (a record with neither links nor `sub`,
§3.2), and keeps those until the records they replace have expired.

### 3.2 The record

```protobuf
// in enum Kind (Protocol.md §10.3):
NAME = 14;

// in Record (Protocol.md §10.7):
Name name = 12;

message Name {
  bytes  zone = 1;            // 32 B root key of the zone
  string label = 2;           // the name in the zone (§1.1); empty = the apex
  uint64 seq = 3;             // version, higher = newer
  repeated string links = 4;  // the ed3k links the name stands for; at most 4
  bytes  sub = 5;             // optional: 32 B root key of the zone that holds the names below this one
  Delegation delegation = 6;  // present when a signing key signed the record (§2)
}

// in Store (Protocol.md §10.3):
repeated Signed names = 13;   // NAME or CLAIM

// in KadAnswer:
repeated Signed names = 20;   // answer to a search of kind NAME or CLAIM
```

```
target = H("ed3k v1 name", zone(32) ‖ label)[0..16)
```

The record is a `Signed` of kind `NAME` (`Protocol.md` §10.7), signed by the root key, or by
the key of `delegation`.

- **Size.** The record is a `Signed` of at most 1 KiB (`Protocol.md` §10.7), as every record.
  The links of a record are at most 512 B *(initial)* together, so that a record with a label of
  253 B and a delegation still fits.
- **Links.** Each is an ed3k link of `Protocol.md` §13.2 or of an extension, beginning with
  `ed3k:`. At most one per kind of link (`ed3k:set`,
  `ed3k:friend`, …): a name can be a website and a person at once, and a client takes the link
  that fits what the user does with it. Of two links of one kind, a reader takes the first. A link of a kind the reader does not know is ignored,
  and so is a link that carries a secret (see the introduction). A publisher MUST NOT put such a
  link into a record.
- **`sub`.** The names below this one are in the zone of `sub`. A record can have links and
  `sub` together: `blog` is a website, and `x.blog` is asked of another zone.
- **A record with neither** links nor `sub` says that the name is not in use. It is how a name
  is taken back before the record that gave it has expired.
- `Name.links` is not checked against the kinds of link by a storing node: it checks sizes
  only, and stores links of kinds it does not know.

**Storing node**

| Check | Rule |
|---|---|
| Signature, `expires` | as for a keyword record (`Protocol.md` §10.7); `expires` at most 7 days *(initial)* plus 1 h ahead |
| Signer | without `delegation`, the record is signed by `zone`, and `Store.signer` (or the key of the connection) is `zone`. With `delegation`, the delegation is valid under `zone` (§2) with bit 0 of `scope`, the record is signed by `delegation.key`, the signer of the `Store` is that key, and `expires` is not after `delegation.expires` |
| Content | `name`, nothing else. `zone` is 32 B, `label` a name of §1.1 or empty, `links` at most 4, each at least 6 B and starting with `ed3k:`, 512 B together at most, `sub` absent or 32 B; else `REJECTED` |
| Target | `H("ed3k v1 name", zone ‖ label)[0..16)` is the target of the request |
| Count | one record per target, whichever key signed it. It is replaced by a record that is newer by the order below; any other valid record is answered `OLDER` |

**Order.** Of two records for one target, the newer is the one with:

1. the higher **rank**: `delegation.issued` for a record under a delegation, 0 for one signed by
   the root key;
2. then the higher `seq`;
3. then, when both have the same `links`, `sub` and delegation, the later `expires` (a renewal);
4. then the lower `Signed.body`, compared as bytes as it was received and signed, not encoded again.

An owner raises `seq` whenever it changes the content; a renewal keeps it.

- A storing node deletes a record at its `expires`.
- A storing node counts the `NAME` records it holds over all targets per host and site, as it
  does `HEAD` records ([`ip-diversity.md`](ip-diversity.md) §2.4).

**Searcher.** It checks every record as the storing node does, and as the target it searched.
Of all valid records the answers hold, it takes the newest by the order above. A record that is
older than one it already holds for the name, and that one still valid (not expired, its
delegation not expired), is dropped, as a follower of a set does
([`file-set.md`](file-set.md) §5.3).

**Who stores it.** The owner stores a record when it signs it and renews it every 24 h
*(initial)*, with an `expires` of up to 7 days. A node whose user opened a name SHOULD store each
record on its way (§3.3: the apexes and the records that resolved it) once per 24 h, as long as
it keeps the name, with `signer` set to the key the record verifies under, as a follower stores
a head ([`file-set.md`](file-set.md) §5.2). A zone whose owner is offline then keeps its names
until their `expires`.

### 3.3 Resolving a name in a zone

The labels are taken from the right, as DNS does:

1. Search the apex of the zone, `H("ed3k v1 name", zone)[0..16)`. The rank of the newest valid apex
   record is the zone's **floor**; no apex record found: the floor is 0. A record of the zone
   whose rank is below the floor is refused (§2).
2. For each suffix of the name, from the shortest (`blog` of `x.shop.blog`) to the whole name,
   search `NAME` at `H("ed3k v1 name", zone ‖ suffix)[0..16)`. The apex is not a suffix: a `sub` in
   the apex record stands for the apex alone.
3. The first suffix whose record has `sub`, and that is shorter than the name, hands the rest
   on: the client goes on with the labels in front of it in the zone of `sub`, from step 1.
4. Otherwise the record of the whole name is the answer. A record of the whole name that has
   `sub` and no links stands for the apex of that zone: the client goes on in that zone with the
   name `""` (step 1), which counts as one more zone and one more lookup.
5. No record of the whole name: the name does not exist. The name `""` is answered by step 1
   alone.

- The searches of steps 1 and 2 are independent targets, and a client MAY send them at once.
  Most names have no `sub` on their way, so the whole name is known after one round of lookups.
- At most 8 *(initial)* zones are followed, the first one counted, and at most 24 *(initial)*
  lookups made per name, counted as a search one at a time would make them: per zone the apex
  and each suffix up to the one that hands on or answers. Searches sent at once beyond these
  are not counted, and so every client reaches the same answer. A zone that comes up twice ends
  the resolution with no answer.
- A record of a name below a `sub` (in the zone that handed it on) is ignored: what the other
  zone says decides, as in DNS. The zone takes such names back before it hands them on (§3.1),
  so that a lookup of the `sub` record that is lost or withheld finds no stale answer below.

---

## 4. Global names

`alice.ed3k` has no key in it. Nobody owns it. A client finds the zone that a name stands for
in the places its user trusts, nearest first, and only then asks the network. What the network
holds is a count of the sites that make a claim, not a proof.

### 4.1 Where a client looks

| # | Source | What it is | Shown as |
|---|---|---|---|
| 1 | **Petnames** | the user's own zones, under the names it chose for them, and names the user gave keys; any text, also in other scripts; never sent anywhere, unless the user marks one that is a global label as shareable (§4.3) | yours |
| 2 | **Pins** | the zone a name led to when the user first opened it (§4.4) | pinned, with the date |
| 3 | **Name lists** the user follows | files of names and zones, published by their authors as signed sets (§4.5) | the title of the list |
| 4 | **Friends** | the friends' own zones, the names they vouched for, their shareable petnames and their followed lists, asked by `NameQuery` (§4.3), and the lists they publish | *k* of *n* friends |
| 5 | **The network** | `CLAIM` records in Kad (§4.2) | network, *s* sites |

- The first source that names a zone for the name decides. A client that has no pin for the
  name also asks the sources after it, and shows a warning when one of them names another zone.
- Two name lists the user follows that disagree: the one the user put first wins, and the
  client warns.
- Friends that disagree: the zone that most of them name. In *k of n*, *n* is the friends
  that answered the `NameQuery` for the name, with a hit or without; an error or no answer is
  not counted. Equal counts: no answer from this
  source; the client shows the zones and lets the user choose.
- **Before a name that only the network knows is opened for the first time**, the client shows
  that no friend and no list knows it, and the number of sites. The user confirms.
- A pinned name is asked of friends again at most once per 24 h *(initial)*. When most friends
  that answer name another zone, the client warns. It never changes the pin itself.

### 4.2 Claims in Kad

```protobuf
// in enum Kind (Protocol.md §10.3):
CLAIM = 15;

// in Record (Protocol.md §10.7):
Claim claim = 13;

message Claim {
  string name = 1;            // a global label (§1.1), without ".ed3k"
  bytes  zone = 2;            // 32 B root key of the zone the publisher says it stands for
}
```

```
target = H("ed3k v1 name claim", name)[0..16)
```

A claim says: *the publisher calls this zone by this name.* It is signed by its publisher, any
key, like a keyword record (`Protocol.md` §10.7). Lifetime 24 h *(initial)*, stored again every
20 h *(initial)*. It travels in `Store.names` and `KadAnswer.names`.

**Publisher**

- A node publishes a claim for its own zone, under the name its user chose for it, and for a
  name its user has **vouched** for: a name the user pinned or listed and then chose to vouch
  for, one by one. A client MUST NOT publish claims for every name it pinned or saw.
- At most 2 *(initial)* zones per name and publisher key.
- A node that hides behind other nodes ([`hops.md`](hops.md)) publishes claims under its
  anonymous key only, so that a claim does not tie its zone to its node. Such claims arrive as
  stores with `signer` (hops.md §4.3) and count no site, as for `ALIAS`.

**Storing node.** As for `ALIAS` ([`alias.md`](alias.md) §3), with the zone in the place of the
root:

| Check | Rule |
|---|---|
| Signature, `expires` | as for a keyword record (`Protocol.md` §10.7) |
| Content | `claim`, nothing else. `name` a global label, `zone` 32 B; else `REJECTED` |
| Target | `H("ed3k v1 name claim", name)[0..16)` is the target of the request |
| Count | one record per publisher, target and zone. At most 2 *(initial)* zones per publisher key and 8 *(initial)* per publisher site under a target ([`ip-diversity.md`](ip-diversity.md) §2.4); a further one is `REJECTED`, checked before the limit of 64 zones per target. At most 64 *(initial)* zones per target (a full target, below). At most 3 *(initial)* records kept per zone, by the rule of `ALIAS` |

- **A full target.** Unlike `ALIAS`, a record for a new zone at a target that holds 64 zones
  takes the place of the zone with the fewest sites, of those the one first stored earliest
  (the earliest time a record still held for it was first stored, renewals not counted; in one
  second the lower zone as bytes), when that zone has at most 1 site; else it is `FULL`. A
  `Store` with `signer` never takes the place of a zone: where it would, it is `REJECTED`, as in
  `Protocol.md` §10.8; where no zone gives way, it is `FULL`, as any store.
- **Refused zones.** Per target the node remembers up to 64 *(initial)* zones it answered
  `FULL`, each with its site count: each site whose store for the zone was refused, until that
  record would have expired. A `Store` with `signer` adds no site, and no zone to the list. When
  a 65th zone is refused,
  the remembered zone with the fewest sites is forgotten, of those the one remembered earliest.
  A store that makes a refused zone's count higher than that of the weakest held zone is
  answered `STORED`: the zone takes that place and keeps its count, and the zone it displaces is
  remembered as refused.
- So a zone that more sites name than the weakest held zone gets in, whoever filled the target
  first. A squatter that filled a name early holds zones of 1 site, which give way to any new
  zone. To keep out a zone that *n* sites name, it must hold 64 zones of *n* sites or more,
  from at least 8 × *n* sites.

- Sites per zone are counted as `ALIAS` counts them per root: each site that stored a valid
  record for the zone, until that record would have expired, a `Store` with `signer` not
  counted, saturating at 255 *(initial)*.
- A search returns up to 16 *(initial)* records in `names`, one per zone, the zones with the
  most publisher sites first, then by `H("ed3k v1 page", secret ‖ zone)`; of the records kept
  for a zone the one with the lowest `H("ed3k v1 page", secret ‖ zone ‖ publisher key)`.
  `KadAnswer.sites` has one entry per entry of `names`. Paging is by `offset` and `more`, along
  that order.

**Searcher.** It checks each record as the storing node does, merges the answers by zone, and
takes as the site count of a zone the lower median of the non-zero `sites` reported for it, as
for `ALIAS`. The zone with the highest count is the answer of source 5. Two zones with the
highest count: no answer from this source; the client shows them, as for friends (§4.1).

### 4.3 Asking friends

```protobuf
message NameQuery {           // type 44
  repeated string names = 1;  // global labels, without ".ed3k"; at most 16
}

message NameAnswer {          // type 45
  repeated NameHit hits = 1;  // at most one per name asked
  string list = 2;            // optional: the ed3k:set link of the name list this node publishes (§4.5)
}

message NameHit {
  string     name = 1;
  bytes      zone = 2;        // 32 B
  NameSource source = 3;
  uint64     since = 4;       // Unix time since when the node holds it; 0 = not told
}

enum NameSource {
  NAME_NONE = 0;
  NAME_OWN = 1;               // the node's own zone
  NAME_PETNAME = 2;           // a petname its user marked as shareable, a global label
  NAME_PINNED = 3;            // a pin its user vouched for (§4.4)
  NAME_LISTED = 4;            // a name list its user follows
}
```

`NameQuery` and `NameAnswer` are a request and its answer (`Protocol.md` §5.2).

- **No hearsay.** A node answers only from its own zones, the names its user vouched for (§4.4),
  the petnames its user marked as shareable, and the name lists its user follows. A pin alone is
  not answered: it may have come from friends or the network. Trust is not passed on from
  friend to friend, and the network's answer does not come back dressed as a friend's.
- A node answers its friends ([`friend.md`](friend.md) §1), and answers `Error{DENIED}` to any
  other node unless its user chose otherwise. A node without `NAMES` answers
  `Error{UNSUPPORTED}`.
- A name that is not a global label is `Error{MALFORMED}`, and so are more than 16 names. A node
  answers one `NameQuery` per peer and 10 s *(initial)*; a sooner one is `Error{BUSY}` with
  `retry_after`.
- A name the node does not hold gets no hit. `zone` is 32 bytes; a reader drops a hit that is
  not, or whose `name` was not asked.
- A client asks the friends that are online for the names its user opens, not for names it
  merely shows.

### 4.4 Pins and what the user sees

- When the user opens a name for the first time, the client **pins** it: name, zone, how it was
  resolved and when. From then on the pin decides (§4.1). Pins are for global names and for
  names from the web (§5), not for names under a key, whose key is in the link. A pin is kept
  per global label, domain or handle; `x.alice.ed3k` uses the pin of `alice.ed3k`.
- An answer that names another zone than the pin is a **warning**, with both zones, both
  sources and the date of the pin. Only the user changes a pin.
- Wherever the client shows a name it shows how it was resolved: *yours*, *pinned*, the list,
  *k of n friends*, *network*, *domain-verified*, *DNS, unverified* (§5.3). A name under a key
  shows the first 8 characters of the key's base32.
- A user may vouch for a pinned name; the client then publishes a claim for it (§4.2) and
  answers it to friends as `NAME_PINNED`.

### 4.5 Name lists

A name list is a file of names and zones that its author vouches for. It is published as a
member of a signed set that changes and followed by its `ed3k:set` link, as filter lists are
([`filter-list.md`](filter-list.md) §2).

```
"ed3knam" ‖ 0x01 ‖ NameList
```

```protobuf
message NameList {
  string title = 1;
  string comment = 2;
  uint64 created = 3;         // Unix time in seconds; 0 = not told
  repeated NameEntry entries = 4;
}

message NameEntry {
  string name = 1;            // a global label, without ".ed3k"
  bytes  zone = 2;            // 32 B
  string note = 3;            // at most 256 B of UTF-8
}
```

- A member of a followed set is a name list when it is at the top level of the set, is not an
  inner set, has a name ending in `.ed3knam`, starts with the 8 bytes above and is at most 4 MiB
  *(initial)*. Of a set, the first 64 *(initial)* such members in the order of `files` are read,
  at most 64 MiB *(initial)* together, as for filter lists.
- A reader refuses a list with more than 100,000 *(initial)* entries, a `name` that is not a
  global label, or a `zone` that is not 32 bytes. Of two entries for one name the first counts.
  A `note` above 256 B does not refuse the list: the reader cuts it at a character boundary.
- A client follows at most 32 *(initial)* sets of name lists, apart from the sets of filter
  lists, and only those its user chose, in an order the user sets and that is kept across
  restarts (§4.1: the first one wins). A list
  a friend names in `NameAnswer.list` is a list of that friend: it counts for source 4, and the
  client follows it as source 3 only when the user chooses so.
- A user's shareable petnames and the names it vouched for can be published as a list of its
  own, which is what `NameAnswer.list` points to.

---

## 5. Names from the web

A domain of the web can say which ed3k link it stands for. This joins a name that people
already know, and that its holder pays for, to a key.

### 5.1 The DNS record

```
_ed3k.<domain>          TXT "v=ed3k1 <link>"
<local>._ed3k.<domain>  TXT "v=ed3k1 <link>"          for the handle <local>@<domain>
```

```
_ed3k.example.com.        3600 IN TXT "v=ed3k1 ed3k:name?key=25njqamcweflpvkl73j4szahhihoc4xt3ktcgjnpaingr5yhkena"
_ed3k.blog.example.com.   3600 IN TXT "v=ed3k1 ed3k:set?key=25njqamcweflpvkl73j4szahhihoc4xt3ktcgjnpaingr5yhkena&id=mjwg6zy"
alice._ed3k.example.com.  3600 IN TXT "v=ed3k1 ed3k:friend?key=hvabpq7iioevvevxbktu2g36xsojqlgpf3cjndgazvk7ckxumyga&dn=Alice"
```

- The strings of one TXT record are joined without a separator, as SPF and DKIM do, and the
  result is read. It is `v=ed3k1`, one space and one link; anything else is not a record of
  this document and is ignored. The joined record is at most 1,024 B *(initial)*.
- `<local>` is 1 to 63 bytes of `a`–`z`, `0`–`9`, `-` and `_`. It is not `_` alone, which
  stands for the domain itself in `ed3k.json` (§5.2).
- The link is one that names a key or a node ID: `ed3k:name` with `key`, `ed3k:set`, `ed3k:friend`,
  `ed3k:room`, `ed3k:live`, `ed3k:node`, or an `ed3k:file`. An `ed3k:name` without `key`, or a
  link that carries a secret, is ignored: a domain does not point at another name it cannot
  vouch for, and what DNS holds is public. The `r` of an `ed3k:friend` link is not refused: a
  user that publishes it lets anyone who reads the record send it friend requests
  ([`friend.md`](friend.md)), which is what a public handle is for.
- A domain may have several records, one per kind of link. A client uses at most 4
  *(initial)*. Two records of one kind whose links differ in more than `dn` make that kind unusable, and the
  client says so.
- The client asks DNS through the resolver the user configured, DNS over HTTPS included, and
  keeps an answer for its TTL, at most 24 h *(initial)*. An answer that has not come within
  10 s *(initial)* is no answer.

### 5.2 The file at the web server

```
GET https://<domain>/.well-known/ed3k.json
```

```json
{
  "names": {
    "_":     ["ed3k:name?key=25njqamcweflpvkl73j4szahhihoc4xt3ktcgjnpaingr5yhkena"],
    "alice": ["ed3k:friend?key=hvabpq7iioevvevxbktu2g36xsojqlgpf3cjndgazvk7ckxumyga&dn=Alice"]
  }
}
```

- `_` is the domain itself; any other member is a `<local>` of a handle, as in §5.1. Each value
  is a list of at most 4 links, with the rules of §5.1. Members the reader does not know are
  ignored.
- Only `https`, with a certificate valid for `<domain>`. Redirects are not followed. The answer
  is at most 64 KiB *(initial)* and arrives within 10 s *(initial)*. The address rules of
  [`http-source.md`](http-source.md) §5 apply to the address the domain resolves to.
- `Cache-Control` is honoured up to 24 h *(initial)*.

### 5.3 What a client believes

A record in DNS can be forged on the path to the resolver unless DNSSEC protects it; that is
why [`bootstrap.md`](bootstrap.md) does not take node lists from DNS. The file at the web server
is protected by TLS. A client compares the **key** of the links (the key of a zone, a set, a
friend, …; the root of an `ed3k:file`; the node ID of an `ed3k:node`), not the link text.
Another `id` under the same key is another set of the same author; the client does not warn
about it.

Before a name from the web is opened for the first time, the client waits for both the TXT
answer and `ed3k.json`, each up to 10 s *(initial)*, and then judges by the table below.

| Evidence | Shown as |
|---|---|
| `ed3k.json` names the key, with or without a TXT record that agrees | domain-verified |
| A TXT record that a validating resolver returned as secure (DNSSEC) | domain-verified |
| A TXT record only, not validated | DNS, unverified: the client asks the user before the first open |
| `ed3k.json` and TXT name different keys | `ed3k.json` decides; the client warns |

- A name from the web is pinned like a global name (§4.4). A domain whose key changes later is
  a warning, also when the new key is domain-verified: a domain can change hands, be taken or
  lapse.
- The client asks DNS and the web server only for a name its user opens, never for a name it
  found in a record or a page, so that a page cannot make it reveal the user's address to a
  server of its choice.

### 5.4 From a web page to ed3k *(informative)*

A web server that also offers its site in ed3k MAY say so to browsers:

```
Link: <ed3k:name?key=25njqamcweflpvkl73j4szahhihoc4xt3ktcgjnpaingr5yhkena>; rel="alternate"
<link rel="alternate" href="ed3k:name?key=25njqamcweflpvkl73j4szahhihoc4xt3ktcgjnpaingr5yhkena">
```

A browser extension or the client can then offer the same site from ed3k. The set can also be
laid out so that the web server serves it as it is ([`chaintube.md`](chaintube.md) §2).

---

## 6. Opening a website

### 6.1 What is a website

A name, or an `ed3k:set` link, leads to a website when it leads to a set whose manifest has a
member `index.html`, or else `index.htm`, at its top level. The client follows its head
([`file-set.md`](file-set.md) §5.3). Any other set is shown as a set, for download
([`file-set.md`](file-set.md) §2.3).

### 6.2 The local server

The client serves the website to the user's web browser over HTTP on a local port, and opens
the browser there.

- It listens on `127.0.0.1` and `::1` only, never on another address.
- **One origin per set.** A set is served at

  ```
  http://<id>.<key>.localhost:<port>/
  ```

  with `key` and `id` of the set in base32 as in links (52 characters for a key, at most 52
  for an `id`). Browsers keep cookies, storage and permissions per origin, so one site cannot
  read another's. The origin follows the key and not the name: a name that comes to lead to
  another key does not hand the new site what the old one stored.
- The port is part of the origin. The client picks it once and keeps it across restarts, else
  every site loses what it stored.
- A client MAY also answer a readable host, `<name>.ed3k.localhost` or `<domain>.web.localhost`,
  with nothing but a redirect (302) to the origin of the set the name leads to. It serves no
  content under such a host.
- Where `*.localhost` does not reach the loopback address in a browser, the client gives each
  set a loopback address of its own (`127.x.y.z`) where the system routes them, as Linux and
  Windows do. Elsewhere it gives each set a port of its own, and tells the user that sites then
  share cookies: browsers keep cookies per host, not per port.
- **Only GET and HEAD**, else 405. The checks go in this order: `Host` (421), method (405),
  `Sec-Fetch-*` (403). A request whose `Host` is not a host the client serves on
  that port is answered 421, so that a page on the web that makes a browser resolve its own
  name to the loopback address (DNS rebinding) reaches nothing. The port serves websites and
  nothing else: no part of the client's own interface is reachable on it, and no answer carries
  a CORS header.
- **Who may make it work.** Resolving a name and downloading a set happen only for the user. The
  client reads `Sec-Fetch-Site`, `Sec-Fetch-Mode` and `Sec-Fetch-User`:

  | Request | Answer |
  |---|---|
  | `Sec-Fetch-Site: none` (the user typed it, or the client opened it), or a navigation with `Sec-Fetch-User: ?1` (the user followed a link) | served; a readable host is resolved (§4, §5), a set not held is fetched |
  | `same-origin`, or `same-site` on a host `<id>.<key>.localhost` (another set of the same key: the browser's site is `<key>.localhost`; under a fallback address or port per set, `same-site` is answered as anything else) | served; members not held are fetched |
  | anything else, or no `Sec-Fetch-Site` | 403, also for what the client holds |

  A page on the web, or a site in ed3k of another key, then cannot make the client ask DNS, a
  web server or friends for a name (§5.3), fetch a set, or tell from an answer which sets the
  client holds. A browser that sends no `Sec-Fetch-Site` gets nothing; the client tells the user
  that websites need a browser that sends it.

### 6.3 Paths and bytes

- A request path, up to the first `?` (the query is ignored), is percent-decoded and split at `/`; its components follow the rules of
  [`file-set.md`](file-set.md) §2.2, else 400. A path that names a directory, or ends in `/`,
  is `index.html` in it, then `index.htm`. A path the set does not hold is answered with the
  member `404.html` at the top level and status 404, or a plain 404.
- **Only verified bytes are sent.** A member the client does not have yet is downloaded at
  once, before other downloads, and sent as its blocks pass under the root, in file order, as
  the window of `Protocol.md` §12.6 does for media. Range requests are answered from verified blocks. A member that is not
  there within 60 s *(initial)* is answered 504. So is a set that is fetched (§6.2) and whose
  manifest is not there within 60 s. A set that is not held and not fetched, under the rules
  of §6.2, is answered 503.
- Headers: `Content-Type` by the extension of the member's name from a fixed table, and
  `application/octet-stream` for any other; `X-Content-Type-Options: nosniff`;
  `Referrer-Policy: no-referrer`; `ETag` with the member's root in base32; `Cache-Control:
  no-cache`.
- **Content-Security-Policy** `default-src 'self' data: blob:; form-action 'self';
  frame-ancestors 'self'; webrtc 'block'` by default. A page then loads no images, scripts,
  styles or frames from the internet, which would tell a server of the page's choice that this
  user is reading it, and from which address. The user MAY relax this per site, and MAY turn
  scripts off per site (`script-src 'none'`).
- What the policy does not stop, while scripts run: a script that navigates the page or opens a
  window to an address on the web, and WebRTC in browsers that do not know `webrtc 'block'`.
  Both reveal the reader's address to that server (§8).
- A new version of the set is served from the first request after its manifest is complete.

**Code.** The client runs nothing it received ([`alias.md`](alias.md) §5, rule 10). A page may
hold scripts; the user's browser runs them in its own sandbox, under the origin of the set, as
it runs the scripts of any web page. The client's part is the bytes and the headers above.

### 6.4 Visitors serve the site *(informative)*

- A client keeps the members it fetched for a website and shares them, and shares the
  manifest, as for any download ([`file-set.md`](file-set.md) §3). Whether it publishes source
  records for each member is its normal policy; the sources of the manifest are asked first.
- The space this takes is the user's choice, 2 GiB by default. The sites viewed longest ago go
  first; a set the user follows goes only when the user removes it.
- **Short slots.** A website is many small files, and an upload queue makes each one wait. The
  order of the queue and the number of slots are local policy (`Protocol.md` §12.1, §12.5), so
  an uploader SHOULD keep up to 4 *(initial)* short slots beside its regular ones: a
  `FileRequest` for a member of at most one block (256 KiB) of a set it shares is answered with
  a `FileStatus` of `rank` = 0 at once when a short slot is free.
- A short slot is the exception to the rule of `Protocol.md` §12.5 that a slot belongs to the
  downloader: it belongs to that one file, ends when its stream closes, without the hold of
  §12.5, and is not kept for a `FileRequest` of another file, which goes into the queue (or
  takes another short slot). A downloader gets at most 32 *(initial)* short slots per minute from
  one uploader.

---

## 7. What it shows

| To whom | What |
|---|---|
| A node that stores a `NAME` record | the zone, the name and its links, in clear |
| A node that stores a `CLAIM` | that the publisher calls that zone by that name |
| The nodes of a lookup | that someone at the searcher's address asks for that zone and name, or that global name |
| Friends | the names the user opens (§4.3), and with `NameAnswer.list` the names the user vouches for |
| The resolver of DNS and the web server of a domain | that this address asks for an ed3k name at that domain. DNS is in clear unless the user configured DNS over HTTPS |
| The browser | the origins of the sites visited, in its history |

A name in a zone is as public as a set's `id`. A zone that wants names only some can find is an
open point (§11).

---

## 8. What an attacker can do

- **Claim a global name** from many sites. The network then shows the attacker's zone to users
  who have no pin, no list and no friend that knows the name. They see *network only* and
  confirm before the first open (§4.1). Users with a pin get a warning, and the pin decides.
- **Fill a `CLAIM` target** with 64 zones, from at least 8 sites. Each of those zones has one
  site, and gives way to a new zone (§4.2). After an eviction the squatter can store its zone
  again and push out the next zone of 1 site, so zones of 1 site rotate. Zones of 2 sites or
  more are safe from this. To keep out a zone that *n* sites name, the squatter needs 64 zones
  of *n* sites or more, from at least 8 × *n* sites. The order by publisher sites puts the
  zones that honest nodes of many sites name first.
- **Hide a newer `NAME` record.** A storing node can withhold a record but cannot change one; a
  searcher asks R nodes (`Protocol.md` §10.4) and takes the newest. Old versions live until
  their `expires`.
- **Take a signing key.** The thief signs records for any name of the zone, also names it never
  used. The owner signs a new delegation and stores the apex under it; searchers then refuse
  every record under the old one (§2, §3.3). Until then the thief's records count. Withholding
  the new apex from a searcher needs every node it asks.
- **Take a root key.** The zone is the thief's. Nothing in the protocol can tell users; a zone
  that keeps its root key offline makes this hard.
- **Lie as a friend.** One friend is one voice of *k of n*. A friend's answer is never passed
  on (§4.3).
- **Take a domain**, or forge DNS. A pin warns users that opened the name before. A first visit
  through an unvalidated TXT record alone is marked and confirmed (§5.3).
- **Imitate a site**: a site that looks like another one, at a name that looks alike. Names are
  ASCII (§1.1), and the client shows how it resolved each name, but the browser shows only the
  origin.
- **Abuse the local server.** A page on the web cannot read it (no CORS, a check of `Host`), nor
  make it resolve names or fetch sets (`Sec-Fetch-*`); a site in ed3k cannot read another (one
  origin per set) or reach the client's interface (§6.2).
- **Learn who reads a site**, as its author: a script that navigates to the web, or WebRTC in a
  browser without `webrtc 'block'`, reveals the reader's address (§6.3). A user that turns
  scripts off for the site closes both.

---

## 9. What it gives and what it costs

| | |
|---|---|
| A name under a key | one round of Kad lookups (the apex and each suffix, at once); one record of about 200 B per name, 320 B with a delegation, 1 KiB at most |
| A global name | nothing when pinned; else a `NameQuery` to online friends and one lookup of claims |
| A name from the web | one DNS query and one HTTPS fetch, outside ed3k |
| Records per zone | one per name and the apex, stored by the owner and by those who opened it once per 24 h |
| Claims | one per vouched name and publisher, about 120 B, once per 20 h |
| Trust added | none that a user did not choose: own names, lists, friends, a domain. The network's count is shown as such |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Global names first come, first served, priced by work** (an Argon2id proof per claim) | one owner per name, no chain | work favours whoever has the most machines; a name is gone to whoever claimed it first and renews it; disputes have no remedy |
| **Registrars**, k of n keys that ship with the client and sign names | simple to check | an authority that the network would have to agree on, and that every client trusts |
| **A contract on a chain**, as for credits ([`credit.md`](credit.md)) | one owner per name, transferable | every name lookup depends on the chain and its operators; a second network that each client would need |
| **The zone as one file** in a set, in place of a record per name | no new record kind; the zone is a name list | a lookup is a head, a manifest and a file, each perhaps behind an upload queue |
| **Labels in any script** (IDNA with a check of confusable characters) | names in every language | every node needs the same tables of confusables; mixed scripts still pass; petnames serve other scripts |
| **A key-blinded zone**, as in GNS | storing nodes do not learn names | daily signing with derived keys, which [`hidden-set.md`](hidden-set.md) §6 left out for the same reason |
| **Friends that ask their friends** | more answers | trust that nobody chose; a lie spreads with every hop |
| **DNS alone**, without the file at the web server | no web server needed | forged by anyone on the path without DNSSEC |
| **An authority form of link**, `ed3k://shop.blog.<key>/a.html` | looks like a web address | breaks the form `ed3k:<kind>?…` of `Protocol.md` §13.2 |
| **The readable name as the origin** in the browser | a readable address bar | a name that comes to lead to another key would give it the old site's storage |
| **No scripts**, enforced by a policy the client sets | nothing runs | many sites do not work; the browser's sandbox is made for this |
| **A viewer inside the client** | no browser, no port | a browser engine in the client, and its updates |

---

## 11. Open points

- Zones whose names only holders of a secret can find, with daily targets as for hidden sets.
- Delegations for the keys of sets, live channels and rooms (bits of `Delegation.scope`), which
  would close the open point of a successor key there ([`file-set.md`](file-set.md) §10).
- A root key that is lost or taken: a successor statement signed in advance and kept offline.
- Hidden and paid sets as websites: their origin, and how the local server obtains keys.
- Whether `NameQuery` should also carry names under a key, for a friend's petnames of zones.
- All numbers are first values.
