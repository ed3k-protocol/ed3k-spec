# Extension: readable names (`name`)

**Status:** draft 0.2 (2026-10-27, after that day's audit). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §2, §5.1, §10.1, §10.3, §10.7 and §13.2, and [`file-set.md`](file-set.md) §5. § = this document unless a document is named.
**Capability:** bit 20, `NAMES`: answers `NameQuery` (§4.3). Message types 44 and 45; record kinds 14 `NAME`, 15 `CLAIM`; two fields in `Record`, one in `Store`, one in `KadAnswer`; a name list file format; `ed3k:name` links; a DNS record and a web-server file.
**Required of every `ROUTER`:** storing and returning the record kinds `NAME` and `CLAIM` (§3.2, §4.2). The rest is optional.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Names*.

Everything in ed3k with an author is named by a key: a changing set, channel, room, friend. A key
is 52 base32 chars nobody remembers or reads out. This extension gives them readable names:
`blog`, `shop.blog` under a key, `alice.ed3k`, `example.com`.

No name can be readable, same for everyone, and safe from being taken all at once (Zooko's
triangle). ed3k gives up a different side per form, and says in each place which:

| Form | Example | Readable | Same for everyone | Who decides | Section |
|---|---|---|---|---|---|
| **Name under a key** | `shop.blog` under a zone's key | yes, below the key | yes | key holder | 3 |
| **Global name** | `alice.ed3k` | yes | no: what user's own names, lists, friends say | user, then friends, then network | 4 |
| **Name from the web** | `example.com`, `alice@example.com` | yes | yes | domain holder | 5 |
| **Petname** | `Mum`, `my band` | yes | no: only the user has it | user | 4.1 |

A name leads to an **ed3k link**: a changing `ed3k:set` (website, §6), `ed3k:friend`,
`ed3k:room`, `ed3k:live` channel, `ed3k:file`, `ed3k:node`. It never leads to a secret: a link with `s`,
an `ed3k:pass` with `k` and an `ed3k:cache` link are refused wherever a name record or domain
carries one.

A website is a changing set ([`file-set.md`](file-set.md) §5): signed file list under a key, head
→ newest version. Client hands it to the user's web browser from a local port (§6). ed3k carries
no code; client runs none.

---

## 1. Names

### 1.1 Labels

**Label**: 1 to 63 bytes of `a`–`z`, `0`–`9`, `-`; not beginning/ending with `-`, not beginning
with `xn--`. **Name**: labels joined by `.`, ≤ 8 *(initial)* labels and ≤ 253 bytes (DNS limit).

- Lower case on wire and in every record; storing node or reader refuses a name with another
  byte. Client lowercases user input first.
- ASCII only. Other scripts, also IDNA `xn--` form, would let different names look alike
  (`pаypal` with Cyrillic `а`). Names in any script are petnames (§4.1), seen only by their user.
  Same for domains and handles: a domain with an `xn--` label has no ed3k name.
- **Global label** (§4): label of 3 to 63 bytes.

### 1.2 Three kinds of name

Reading a name given without a key:

| Name | Kind | Resolved by |
|---|---|---|
| ends in `.ed3k` | global name: label before `.ed3k` = global label, labels in front = name in that zone | §4, then §3 |
| has an `@` | handle at a domain: `alice@example.com` | §5 |
| any other name of ≥ two labels | domain: `example.com`, `blog.example.com` | §5 |

A name given with a key (§1.3) is a name in that key's zone (§3).

`.ed3k` is not a DNS TLD. Should one by that name be delegated, a client still reads `.ed3k` as
here.

### 1.3 Links

```
ed3k:name?key=<root key of the zone, base32 lower case, no padding>[&n=<name>][&path=<url-encoded path>][&dn=<url-encoded display name>]
ed3k:name?n=<global name ending in .ed3k, domain or handle>[&path=<url-encoded path>][&dn=<url-encoded display name>]
```

- With `key`, `n` is a name in that zone; no `n` = the zone itself (apex, §3.1).
- Without `key`, `n` required, one of the forms of §1.2.
- `path` selects a website page, or a set member/directory, as `path` of `ed3k:set`
  ([`file-set.md`](file-set.md) §5.4).
- `dn` shown while resolving; decides nothing.
- A link is invalid when: `key` does not decode to 32 bytes; `n` not a name of §1.1 (handle: local part as
  §5.1, domain a domain of §1.2, not ending in `.ed3k`; `n=` empty is no name); no `key` and no
  `n`; or `key`, `n`, `path` or `dn` repeated. Base32 per `Protocol.md` §13.2. Unknown
  parameters ignored.
- A link without `key` is not a secure name. Client shows how it resolved it (§4.4) before first
  opening.
- User types one label without `.ed3k` (`alice`): petname if user has one by that text, else
  `alice.ed3k`. In a link, one-label name without `key` is invalid.

---

## 2. Root key and signing key

A name lives for years; a key on a daily-online machine can be lost or taken. So a zone MAY keep
its key offline. That **root key** names the zone and signs a **delegation**: a short-lived
**signing key** that signs the records.

```protobuf
message Delegation {
  bytes  key = 1;             // 32 B Ed25519 signing key
  uint64 issued = 2;          // Unix time in seconds
  uint64 expires = 3;         // Unix time in seconds
  uint64 scope = 4;           // bit 0: NAME records (§3); every other bit is reserved
  repeated Signature sigs = 5;    // by the root key, over the layout below
}
```

Each signature is over

```
"ed3k v1 delegation" ‖ root key(32) ‖ key(32) ‖ issued(8) ‖ expires(8) ‖ scope(8)
```

with integers big-endian. `sigs` rules: `Protocol.md` §10.7.

Delegation **valid** when:

- `key` is 32 bytes and not the root key;
- signatures verify under the root key;
- `issued` ≤ 1 h *(initial)* ahead of reader's clock, `expires` later than `issued` and ≤ 90
  days *(initial)* after it, and `expires` not passed;
- `scope` has the bit of the record carrying it.

A root key MUST NOT sign two delegations with the same `issued`.

**Replacing a signing key.** Root key signs a new delegation with later `issued`; owner first
stores the apex record (§3.1) under it. A record under the newer delegation beats every record
under an older one (§3.2), whatever its `seq`, and the apex sets the zone's **floor**: searcher
refuses every zone record under an older delegation (§3.3). A taken signing key is thus replaced
by one root-key signature and one store, and every record it signed, also for names the zone
never used, is refused from then on. A lost one is replaced the same way.

**Not covered.** Taken root key = end of zone: thief can sign delegations as well as owner. Lost
root key: zone keeps the last delegation until it expires. Pins (§4.4) tell a zone's users that
it moved to a new root key; they cannot tell them it is still the same owner.

A zone not wanting two keys signs records with the root key itself. Such a record loses against
every valid record under a delegation (§3.2), so a zone can start with one key and move to a
signing key any time. No way back: a zone that used a delegation signs under one until the last
it signed has expired, up to 90 days.

`scope` is a set of bits so other records signed for an offline key can reuse the message later: heads of
sets, live channels and rooms each list their key's successor as open point
([`file-set.md`](file-set.md) §10, [`live.md`](live.md), [`room.md`](room.md)).

---

## 3. Names under a key

### 3.1 Zones

**Zone**: the names under one root key. Owner gives each name one record. Name `""`, the
**apex**, is the zone itself: a website at the apex is what `ed3k:name?key=…` opens. A zone
signing under a delegation MUST keep an apex record under its newest delegation, with neither
links nor `sub` if the apex is unused: it is the floor of §2.

Names may have dots (`shop.blog`); a record may hand every name below it to another zone (`sub`):
`x.shop.blog` is then name `x` there. So a zone gives a name away without giving away its key. Before storing
a record with `sub`, the zone takes back every name below it that it gave a record (record with
neither links nor `sub`, §3.2), and keeps those until the records they replace have expired.

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

A `Signed` of kind `NAME` (`Protocol.md` §10.7), signed by the root key or by `delegation`'s key.

- **Size.** `Signed` ≤ 1 KiB (`Protocol.md` §10.7), as every record. Links ≤ 512 B *(initial)*
  together, so a 253 B label plus delegation still fits.
- **Links.** Each an ed3k link of `Protocol.md` §13.2 or an extension, starting `ed3k:`. ≤ one
  per link kind (`ed3k:set`, `ed3k:friend`, …): a name can be website and person at once; client
  takes the link fitting the user's action. Two of one kind: reader takes the first. Unknown kind
  ignored, as is a link carrying a secret (see intro). A publisher MUST NOT put such a link into
  a record.
- **`sub`.** Names below this one are in `sub`'s zone. Links and `sub` can coexist: `blog` is a
  website, `x.blog` is asked of another zone.
- **Neither** links nor `sub`: name not in use. How a name is taken back before the record that
  gave it expires.
- Storing node does not check `Name.links` against link kinds: sizes only; stores unknown kinds.

**Storing node**

| Check | Rule |
|---|---|
| Signature, `expires` | as keyword record (`Protocol.md` §10.7); `expires` ≤ 7 days *(initial)* + 1 h ahead |
| Signer | no `delegation`: signed by `zone`, `Store.signer` (or connection key) is `zone`. With `delegation`: valid under `zone` (§2) with bit 0 of `scope`, record signed by `delegation.key`, `Store` signer is that key, `expires` not after `delegation.expires` |
| Content | `name` only. `zone` 32 B, `label` a §1.1 name or empty, `links` ≤ 4, each ≥ 6 B starting `ed3k:`, ≤ 512 B together, `sub` absent or 32 B; else `REJECTED` |
| Target | `H("ed3k v1 name", zone ‖ label)[0..16)` = request target |
| Count | one record per target, whichever key signed. Replaced by a record newer by the order below; any other valid record → `OLDER` |

**Order.** Of two records for one target, newer is the one with:

1. higher **rank**: `delegation.issued` under a delegation, 0 if root-key signed;
2. then higher `seq`;
3. then, when `links`, `sub` and delegation are equal, later `expires` (renewal);
4. then lower `Signed.body`, compared as bytes as received and signed, not re-encoded.

Owner raises `seq` on every content change; renewal keeps it.

- Storing node deletes a record at its `expires`.
- Storing node counts held `NAME` records over all targets per host and site, as for `HEAD`
  ([`ip-diversity.md`](ip-diversity.md) §2.4).

**Searcher.** Checks every record as storing node does, and against the searched target. Takes
the newest valid record across answers by the order above. A record older than one already held
for the name, that one still valid (not expired, delegation not expired), is dropped, as a set
follower does ([`file-set.md`](file-set.md) §5.3).

**Who stores it.** Owner stores on signing, renews every 24 h *(initial)*, `expires` ≤ 7 days. A
node whose user opened a name SHOULD store each record on its way (§3.3: apexes and resolving
records) once per 24 h while it keeps the name, `signer` = key the record verifies under, as a
follower stores a head ([`file-set.md`](file-set.md) §5.2). A zone with offline owner thus keeps
its names until their `expires`.

### 3.3 Resolving a name in a zone

Labels from the right, as DNS:

1. Search zone apex, `H("ed3k v1 name", zone)[0..16)`. Rank of newest valid apex record = zone
   **floor**; no apex found: floor 0. A zone record with rank below floor is refused (§2).
2. For each suffix, shortest (`blog` of `x.shop.blog`) to whole name, search `NAME` at
   `H("ed3k v1 name", zone ‖ suffix)[0..16)`. Apex is no suffix: `sub` in the apex record stands
   for the apex alone.
3. First suffix whose record has `sub`, and that is shorter than the name, hands the rest on: continue
   with the labels in front of it in `sub`'s zone, from step 1.
4. Else the whole-name record is the answer. Whole-name record with `sub` and no links = apex of
   that zone: continue there with name `""` (step 1), counting one more zone and one more lookup.
5. No whole-name record: name does not exist. Name `""` is answered by step 1 alone.

- Steps 1 and 2 are independent targets; a client MAY send them at once. Most names have no `sub`
  on their way → whole name known after one lookup round.
- ≤ 8 *(initial)* zones followed, first counted, and ≤ 24 *(initial)* lookups per name, counted
  as a one-at-a-time search would: per zone the apex and each suffix up to the one that hands on
  or answers. Parallel searches beyond these are not counted, so every client reaches the same
  answer. A zone appearing twice ends resolution with no answer.
- A record for a name below a `sub` (in the handing zone) is ignored: the other zone decides, as
  DNS. Zone takes such names back before handing on (§3.1), so a lookup whose `sub` record is
  lost or withheld finds no stale answer below.

---

## 4. Global names

`alice.ed3k` has no key; nobody owns it. Client finds the zone in places its user trusts, nearest
first, only then the network. Network holds a count of claiming sites, not a proof.

### 4.1 Where a client looks

| # | Source | What it is | Shown as |
|---|---|---|---|
| 1 | **Petnames** | user's own zones under names it chose, and names the user gave keys; any text, other scripts too; never sent anywhere, unless user marks one that is a global label shareable (§4.3) | yours |
| 2 | **Pins** | zone a name led to at user's first open (§4.4) | pinned, with date |
| 3 | **Name lists** the user follows | files of names and zones, published by authors as signed sets (§4.5) | list title |
| 4 | **Friends** | friends' own zones, names they vouched for, shareable petnames, followed lists, asked by `NameQuery` (§4.3), and lists they publish | *k* of *n* friends |
| 5 | **The network** | `CLAIM` records in Kad (§4.2) | network, *s* sites |

- First source naming a zone decides. Client with no pin for the name also asks later sources,
  and warns when one names another zone.
- Two followed lists disagree: user's first-placed wins; client warns.
- Friends disagree: zone most name. In *k of n*, *n* = friends that answered the `NameQuery` for
  the name, hit or not; error or no answer not counted. Tie: no answer from this source; client
  shows the zones, user chooses.
- **Before first opening a name only the network knows**, client shows that no friend and no list
  knows it, and the site count. User confirms.
- Pinned name re-asked of friends ≤ once per 24 h *(initial)*. Most answering friends name
  another zone → client warns. Never changes the pin itself.

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

Claim = *publisher calls this zone by this name.* Signed by publisher, any key, like a keyword
record (`Protocol.md` §10.7). Lifetime 24 h *(initial)*, re-stored every 20 h *(initial)*. Travels in
`Store.names` and `KadAnswer.names`.

**Publisher**

- Publishes a claim for its own zone, under the name its user chose, and for names its user
  **vouched** for: pinned or listed, then chosen to vouch for one by one. A client MUST NOT
  publish claims for every name it pinned or saw.
- ≤ 2 *(initial)* zones per name and publisher key.
- A node hiding behind other nodes ([`hops.md`](hops.md)) publishes claims under its anonymous
  key only, so a claim does not tie its zone to its node. These arrive as stores with `signer`
  (hops.md §4.3) and count no site, as for `ALIAS`.

**Storing node.** As `ALIAS` ([`alias.md`](alias.md) §3), zone in place of root:

| Check | Rule |
|---|---|
| Signature, `expires` | as keyword record (`Protocol.md` §10.7) |
| Content | `claim` only. `name` a global label, `zone` 32 B; else `REJECTED` |
| Target | `H("ed3k v1 name claim", name)[0..16)` = request target |
| Count | one record per publisher, target and zone. ≤ 2 *(initial)* zones per publisher key and 8 *(initial)* per publisher site under a target ([`ip-diversity.md`](ip-diversity.md) §2.4); more → `REJECTED`, checked before the 64-zones-per-target limit. ≤ 64 *(initial)* zones per target (full target, below). ≤ 3 *(initial)* records kept per zone, by the `ALIAS` rule |

- **Full target.** Unlike `ALIAS`, a record for a new zone at a 64-zone target replaces the zone
  with fewest sites, among those the one first stored earliest (earliest first-store time of a record
  still held for it, renewals not counted; same second: lower zone as bytes), if that zone has ≤ 1
  site; else `FULL`. A `Store` with `signer` never replaces a zone: where it would → `REJECTED`,
  as `Protocol.md` §10.8; where no zone gives way → `FULL`, as any store.
- **Refused zones.** Per target, node remembers ≤ 64 *(initial)* zones it answered `FULL`, each
  with its site count: each site whose store for the zone was refused, until that record would
  have expired. `Store` with `signer` adds no site and no zone to the list. On a 65th refused
  zone, the remembered zone with fewest sites is forgotten, among those the earliest remembered.
  A store lifting a refused zone's count above the weakest held zone's → `STORED`: zone takes
  that place, keeps its count; displaced zone remembered as refused.
- So a zone named by more sites than the weakest held one gets in, whoever filled the target
  first. An early squatter holds 1-site zones, which yield to any new zone. To keep out a zone
  named by *n* sites, it must hold 64 zones of ≥ *n* sites, from ≥ 8 × *n* sites.

- Sites per zone counted as `ALIAS` per root: each site that stored a valid record for the zone,
  until that record would have expired, `Store` with `signer` not counted, saturating at 255
  *(initial)*.
- Search returns ≤ 16 *(initial)* records in `names`, one per zone, most publisher sites first,
  then by `H("ed3k v1 page", secret ‖ zone)`; of a zone's kept records the one with lowest
  `H("ed3k v1 page", secret ‖ zone ‖ publisher key)`. `KadAnswer.sites`: one entry per `names`
  entry. Paging by `offset` and `more`, in that order.

**Searcher.** Checks each record as storing node, merges answers by zone, site count of a zone =
lower median of non-zero `sites` reported for it, as `ALIAS`. Highest count = source 5's answer.
Tie at highest: no answer from this source; client shows them, as for friends (§4.1).

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

`NameQuery`/`NameAnswer`: request and answer (`Protocol.md` §5.2).

- **No hearsay.** Answers only from own zones, names user vouched for (§4.4), petnames user marked
  shareable, and lists user follows. A pin alone is not answered: may come from friends or
  network. Trust not passed on from friend to friend; network's answer never returned dressed as a
  friend's.
- Answers its friends ([`friend.md`](friend.md) §1); `Error{DENIED}` to any other node unless
  user chose otherwise. Node without `NAMES` answers `Error{UNSUPPORTED}`.
- Name not a global label → `Error{MALFORMED}`; so are > 16 names. One `NameQuery` answered per peer and
  10 s *(initial)*; sooner → `Error{BUSY}` with `retry_after`.
- Name not held: no hit. `zone` 32 bytes; reader drops a hit whose `zone` is not, or whose `name`
  was not asked.
- Client asks online friends for names its user opens, not ones it merely shows.

### 4.4 Pins and what the user sees

- At a name's first open, client **pins** it: name, zone, how resolved, when. Then the pin
  decides (§4.1). Pins: global names and web names (§5), not names under a key (key is in the
  link). One pin per global label, domain or handle; `x.alice.ed3k` uses the pin of `alice.ed3k`.
- Answer naming a zone other than the pin → **warning**, with both zones, both sources, pin date.
  Only the user changes a pin.
- Wherever a name is shown, so is how it resolved: *yours*, *pinned*, the list, *k of n friends*,
  *network*, *domain-verified*, *DNS, unverified* (§5.3). Name under a key shows the first 8
  chars of the key's base32.
- User may vouch for a pinned name; client then publishes a claim (§4.2) and answers it to
  friends as `NAME_PINNED`.

### 4.5 Name lists

A name list = file of names and zones its author vouches for. Published as a member of a changing signed set,
followed by its `ed3k:set` link, as filter lists ([`filter-list.md`](filter-list.md) §2).

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

- A followed set's member is a name list when top-level, not an inner set, name ending in
  `.ed3knam`, starting with the 8 bytes above, ≤ 4 MiB *(initial)*. Per set the first 64
  *(initial)* such members in `files` order are read, ≤ 64 MiB *(initial)* together, as filter
  lists.
- Reader refuses a list with > 100,000 *(initial)* entries, a `name` not a global label, or a
  `zone` not 32 bytes. Two entries for one name: first counts. `note` > 256 B does not refuse the
  list: reader cuts it at a character boundary.
- Client follows ≤ 32 *(initial)* sets of name lists, separate from filter-list sets, only those
  user chose, in a user-set order kept across restarts (§4.1: first wins). A list a friend names
  in `NameAnswer.list` is that friend's: counts for source 4; followed as source 3 only if user
  chooses.
- User's shareable petnames and vouched names can be published as its own list, which
  `NameAnswer.list` points to.

---

## 5. Names from the web

A web domain can say which ed3k link it stands for: joins a name people already know, paid for
by its holder, to a key.

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

- Strings of one TXT record joined without separator (as SPF, DKIM), result read: `v=ed3k1`, one space, one link; else not a record of this document, ignored. Joined record ≤
  1,024 B *(initial)*.
- `<local>`: 1 to 63 bytes of `a`–`z`, `0`–`9`, `-`, `_`. Not `_` alone, which is the domain
  itself in `ed3k.json` (§5.2).
- Link: one naming a key or node ID: `ed3k:name` with `key`, `ed3k:set`, `ed3k:friend`,
  `ed3k:room`, `ed3k:live`, `ed3k:node`, or an `ed3k:file`. `ed3k:name` without `key`, or a link
  carrying a secret, is ignored: a domain does not point at another name it cannot vouch for, and
  DNS is public. `r` of an `ed3k:friend` link is not refused: publishing it lets any reader send
  friend requests ([`friend.md`](friend.md)), which is what a public handle is for.
- Several records allowed, one per link kind. Client uses ≤ 4 *(initial)*. Two records of one
  kind whose links differ in more than `dn` make that kind unusable; client says so.
- DNS asked via user-configured resolver, DNS over HTTPS included; answer kept for its TTL, ≤ 24
  h *(initial)*. None within 10 s *(initial)* = no answer.

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

- `_` = domain itself; other members = `<local>` of a handle, as §5.1. Each value: list of ≤ 4
  links, §5.1 rules. Unknown members ignored.
- `https` only, certificate valid for `<domain>`. No redirects followed. Answer ≤ 64 KiB
  *(initial)*, within 10 s *(initial)*. Address rules of [`http-source.md`](http-source.md) §5
  apply to the domain's resolved address.
- `Cache-Control` honoured up to 24 h *(initial)*.

### 5.3 What a client believes

DNS can be forged on the path to the resolver unless DNSSEC protects it; hence
[`bootstrap.md`](bootstrap.md) takes no node lists from DNS. The web-server file is TLS-protected.
Client compares the **key** of links (key of zone, set, friend, …; root of an `ed3k:file`; node ID
of an `ed3k:node`), not link text. Another `id` under the same key = another set of the same
author; no warning.

Before first opening a web name, client waits for both TXT answer and `ed3k.json`, each ≤ 10 s
*(initial)*, then judges:

| Evidence | Shown as |
|---|---|
| `ed3k.json` names the key, with or without agreeing TXT | domain-verified |
| TXT returned as secure by a validating resolver (DNSSEC) | domain-verified |
| TXT only, not validated | DNS, unverified: client asks user before first open |
| `ed3k.json` and TXT name different keys | `ed3k.json` decides; client warns |

- Web names are pinned like global names (§4.4). A later key change of a domain is a warning, also
  when the new key is domain-verified: domains change hands, get taken, lapse.
- DNS and web server asked only for names the user opens, never for one found in a record or
  page, so a page cannot make the client reveal the user's address to a server of its choice.

### 5.4 From a web page to ed3k *(informative)*

A web server also offering its site in ed3k MAY tell browsers:

```
Link: <ed3k:name?key=25njqamcweflpvkl73j4szahhihoc4xt3ktcgjnpaingr5yhkena>; rel="alternate"
<link rel="alternate" href="ed3k:name?key=25njqamcweflpvkl73j4szahhihoc4xt3ktcgjnpaingr5yhkena">
```

A browser extension or the client can then offer the site from ed3k. The set can also be laid out
so the web server serves it as is ([`chaintube.md`](chaintube.md) §2).

---

## 6. Opening a website

### 6.1 What is a website

A name or `ed3k:set` link leads to a website when its set's manifest has top-level member
`index.html`, else `index.htm`. Client follows its head ([`file-set.md`](file-set.md) §5.3) while
the site is shown or held (§6.4); that is not a set the user follows. Any
other set is shown as a set, for download ([`file-set.md`](file-set.md) §2.3).

### 6.2 The local server

Client serves the website to the user's browser over HTTP on a local port and opens the browser
there.

- Listens on `127.0.0.1` and `::1` only, never another address.
- **One origin per set.** A set is served at

  ```
  http://<id>.<key>.localhost:<port>/
  ```

  set's `key` and `id` in base32 as in links (52 chars for a key, ≤ 52 for an `id`). Browsers keep
  cookies, storage, permissions per origin, so sites cannot read each other's. Origin follows the
  key, not the name: a name that comes to lead to another key does not hand the new site what the
  old one stored.
- Port is part of the origin: picked once, kept across restarts, else every site loses its
  storage.
- A client MAY also answer a readable host, `<name>.ed3k.localhost` or `<domain>.web.localhost`,
  only with a redirect (302) to the origin of the set the name leads to. No content under such a
  host.
- Where `*.localhost` does not reach loopback in a browser: each set gets its own loopback address
  (`127.x.y.z`) where the system routes them (Linux, Windows). Elsewhere each set gets its own
  port, and client tells the user sites then share cookies: browsers keep cookies per host, not
  port.
- **Only GET and HEAD**, else 405. Check order: `Host` (421), method (405), `Sec-Fetch-*` (403).
  `Host` not served by the client on that port → 421, so a web page making a browser resolve its
  own name to loopback (DNS rebinding) reaches nothing. Port serves websites only: none of the
  client's own interface is reachable on it; no answer carries a CORS header.
- **Who may make it work.** Resolving names and downloading sets happen only for the user. Client
  reads `Sec-Fetch-Site`, `Sec-Fetch-Mode`, `Sec-Fetch-User`:

  | Request | Answer |
  |---|---|
  | `Sec-Fetch-Site: none` (user typed it, or client opened it), or navigation with `Sec-Fetch-User: ?1` (user followed a link) | served; readable host resolved (§4, §5), set not held fetched |
  | `same-origin`, or `same-site` on a host `<id>.<key>.localhost` (another set of the same key: browser's site is `<key>.localhost`; under fallback address or port per set, `same-site` is treated as anything else) | served; members not held fetched |
  | anything else, or no `Sec-Fetch-Site` | 403, also for what the client holds |

  So a web page, or an ed3k site of another key, cannot make the client ask DNS, a web server or
  friends for a name (§5.3), fetch a set, or learn from an answer which sets the client holds. A
  browser sending no `Sec-Fetch-Site` gets nothing; client tells the user websites need a browser
  that sends it.

### 6.3 Paths and bytes

- Request path up to first `?` (query ignored) is percent-decoded and split at `/`; components
  follow [`file-set.md`](file-set.md) §2.2, else 400. A path naming a directory, or ending in `/`,
  = `index.html` in it, then `index.htm`. Path not in the set → top-level member `404.html` with
  status 404, or a plain 404.
- **Only verified bytes are sent.** A member not yet held is downloaded at once, before other
  downloads, and sent as blocks pass under the root, in file order, as the `Protocol.md` §12.6
  window does for media. Range requests answered from verified blocks. Member not there within
  60 s *(initial)* → 504. Same for a fetched set (§6.2) whose manifest is not there within 60 s.
  Set neither held nor fetched, under §6.2, → 503.
- Headers: `Content-Type` by member-name extension from a fixed table, else
  `application/octet-stream`; `X-Content-Type-Options: nosniff`; `Referrer-Policy: no-referrer`;
  `ETag` = member's root in base32; `Cache-Control:
  no-cache`.
- **Content-Security-Policy** `default-src 'self' data: blob:; form-action 'self';
  frame-ancestors 'self'; webrtc 'block'` by default. A page then loads no images, scripts,
  styles or frames from the internet, which would tell a server of the page's choice that this
  user reads it, and from which address. User MAY relax this per site, and MAY turn scripts off
  per site (`script-src 'none'`).
- Not stopped while scripts run: a script navigating the page or opening a window to a web
  address, and WebRTC in browsers not knowing `webrtc 'block'`. Both reveal the reader's address
  to that server (§8).
- A new set version is served from the first request after its manifest is complete.

**Code.** Client runs nothing it received ([`alias.md`](alias.md) §5, rule 10). Pages may hold
scripts; the user's browser runs them in its sandbox, under the set's origin, like any web page.
Client's part: bytes and headers above.

### 6.4 Visitors serve the site *(informative)*

- Client keeps and shares members it fetched for a website, and the manifest, as any download
  ([`file-set.md`](file-set.md) §3). Publishing source records per member is its normal policy;
  manifest sources asked first.
- Space: user's choice, 2 GiB default. Longest-ago viewed sites go first; a set the user chose to
  follow only when the user removes it.
- **Short slots.** A website is many small files; an upload queue makes each wait. Queue order
  and slot count are local policy (`Protocol.md` §12.1, §12.5), so an uploader SHOULD keep ≤ 4
  *(initial)* short slots beside regular ones: a `FileRequest` for a member of ≤ one block (256
  KiB) of a set it shares gets a `FileStatus` with `rank` = 0 at once when a short slot is free.
- Short slot = exception to the `Protocol.md` §12.5 rule that a slot belongs to the downloader:
  it belongs to that one file, ends when its stream closes, without the hold of §12.5, and is not
  kept for a `FileRequest` of another file (that one queues, or takes another short slot). ≤ 32
  *(initial)* short slots per minute per downloader from one uploader.

---

## 7. What it shows

| To whom | What |
|---|---|
| Node storing a `NAME` record | zone, name and links, in clear |
| Node storing a `CLAIM` | that the publisher calls that zone by that name |
| Lookup nodes | that someone at the searcher's address asks for that zone and name, or that global name |
| Friends | names the user opens (§4.3); with `NameAnswer.list`, names the user vouches for |
| DNS resolver and domain's web server | that this address asks for an ed3k name at that domain. DNS in clear unless user configured DNS over HTTPS |
| Browser | origins of sites visited, in its history |

A name in a zone is as public as a set's `id`. Zones with names only some can find: open point
(§11).

---

## 8. What an attacker can do

- **Claim a global name** from many sites. Network then shows attacker's zone to users with no
  pin, list or friend knowing the name. They see *network only* and confirm before first open
  (§4.1). Users with a pin get a warning; pin decides.
- **Fill a `CLAIM` target** with 64 zones from ≥ 8 sites. Each has one site and yields to a new
  zone (§4.2). After eviction the squatter can re-store and push out the next 1-site zone, so
  1-site zones rotate. Zones of ≥ 2 sites are safe from this. To keep out a zone named by *n*
  sites, squatter needs 64 zones of ≥ *n* sites, from ≥ 8 × *n* sites. Ordering by publisher sites
  puts zones named by honest nodes of many sites first.
- **Hide a newer `NAME` record.** A storing node can withhold but not change a record; searcher
  asks R nodes (`Protocol.md` §10.4), takes the newest. Old versions live until their `expires`.
- **Take a signing key.** Thief signs records for any zone name, also never-used ones. Owner signs
  a new delegation, stores apex under it; searchers then refuse every record under the old one
  (§2, §3.3). Until then the thief's records count. Withholding the new apex from a searcher needs
  every node it asks.
- **Take a root key.** Zone is the thief's. Protocol cannot tell users; an offline root key makes
  this hard.
- **Lie as a friend.** One friend = one voice of *k of n*. Friends' answers never passed on (§4.3).
- **Take a domain**, or forge DNS. A pin warns users who opened the name before. A first visit via
  an unvalidated TXT record alone is marked and confirmed (§5.3).
- **Imitate a site**: lookalike site at lookalike name. Names are ASCII (§1.1) and client shows
  how each name resolved, but the browser shows only the origin.
- **Abuse the local server.** A web page cannot read it (no CORS, `Host` check), nor make it
  resolve names or fetch sets (`Sec-Fetch-*`); an ed3k site cannot read another (one origin per
  set) or reach the client's interface (§6.2).
- **Learn who reads a site**, as its author: a script navigating to the web, or WebRTC in a
  browser without `webrtc 'block'`, reveals the reader's address (§6.3). Turning scripts off for
  the site closes both.

---

## 9. What it gives and what it costs

| | |
|---|---|
| A name under a key | one round of Kad lookups (apex and each suffix, at once); one record ≈ 200 B per name, 320 B with delegation, ≤ 1 KiB |
| A global name | nothing when pinned; else a `NameQuery` to online friends and one claims lookup |
| A name from the web | one DNS query and one HTTPS fetch, outside ed3k |
| Records per zone | one per name plus apex, stored by owner and by those who opened it, once per 24 h |
| Claims | one per vouched name and publisher, ≈ 120 B, once per 20 h |
| Trust added | none the user did not choose: own names, lists, friends, a domain. Network's count shown as such |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Global names first come, first served, priced by work** (an Argon2id proof per claim) | one owner per name, no chain | work favours most machines; name goes to first claimer who renews; disputes have no remedy |
| **Registrars**, k of n keys shipped with the client that sign names | simple to check | an authority the network would have to agree on and every client trusts |
| **A contract on a chain**, as for credits ([`credit.md`](credit.md)) | one owner per name, transferable | every lookup depends on chain and operators; a second network every client needs |
| **The zone as one file** in a set, not a record per name | no new record kind; zone = name list | lookup = head, manifest and file, each maybe behind an upload queue |
| **Labels in any script** (IDNA with confusables check) | names in every language | every node needs same confusables tables; mixed scripts still pass; petnames serve other scripts |
| **A key-blinded zone**, as in GNS | storing nodes do not learn names | daily signing with derived keys, which [`hidden-set.md`](hidden-set.md) §6 left out for the same reason |
| **Friends that ask their friends** | more answers | trust nobody chose; a lie spreads every hop |
| **DNS alone**, without the web-server file | no web server needed | forged by anyone on the path without DNSSEC |
| **An authority form of link**, `ed3k://shop.blog.<key>/a.html` | looks like a web address | breaks form `ed3k:<kind>?…` of `Protocol.md` §13.2 |
| **The readable name as the origin** in the browser | readable address bar | a name that comes to lead to another key would get the old site's storage |
| **No scripts**, enforced by client-set policy | nothing runs | many sites break; browser sandbox is made for this |
| **A viewer inside the client** | no browser, no port | a browser engine in the client, plus its updates |

---

## 11. Open points

- Zones whose names only secret holders can find, with daily targets as for hidden sets.
- Delegations for keys of sets, live channels, rooms (bits of `Delegation.scope`), closing the
  successor-key open point there ([`file-set.md`](file-set.md) §10).
- Lost or taken root key: successor statement signed in advance, kept offline.
- Hidden and paid sets as websites: their origin, how the local server gets keys.
- Whether `NameQuery` should also carry names under a key, for a friend's petnames of zones.
- All numbers are first values.
