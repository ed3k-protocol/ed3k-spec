# Extension: sets with several authors (`multi-author`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §2.4, §2.5 and §5, and [`Protocol.md`](../Protocol.md) §10.1, §10.3 and §10.7. § = this document unless a document is named.
**Capability:** none of its own; a node using it sets `SETS`. Record kind 9. Two fields in `Record`, three in a manifest file, one in `FileSet`, one in `Member`.
**Required of every `ROUTER`:** the checks of a joint head (§1.3) and storing and returning record kind `PART` (§3.4). The rest is optional.

A changing set has one author, one key signing every version ([`file-set.md`](file-set.md) §5).
Here several keys may write to a set, in three combinable forms.

| Form | Who may write | Approved by | Section |
|---|---|---|---|
| **Co-owners** | a fixed group of keys, to the whole set | the owner, key by key | 1 |
| **A directory handed over** | one other author, to one directory | the owner, key by key | 2 |
| **Open parts** | everyone meeting the owner's rules, each to an own directory | nobody in advance; an issuer if the owner names one | 3 |

Every form keeps the set's name, the pair (**owner key**, `id`), and its `ed3k:set` link.

The three forms are those of a ZeroNet site
([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.4).

---

## 1. Co-owners

### 1.1 The grant

Owner key signs a **grant**: keys that may sign set versions, and how many are needed.

```protobuf
message Grant {
  bytes  id = 1;              // FileSet.id of the set
  uint64 number = 2;          // 1 or higher; a higher number replaces every lower one
  repeated bytes keys = 3;    // 32 B Ed25519 keys, at most 8 (initial)
  uint32 required = 4;        // 1 .. min(4, number of keys) (initial)
}

message SignedGrant {
  bytes  body = 1;            // a serialised Grant
  bytes  key = 2;             // 32 B owner key
  repeated Signature sigs = 3;  // by the owner key, over "ed3k v1 set grant" ‖ body
}

message CoSig {
  uint32 signer = 1;          // index into Grant.keys
  repeated Signature sigs = 2;
}
```

**A valid grant.** A `SignedGrant` is valid when all hold *(numbers initial)*:

- `key` is 32 bytes, `sigs` valid under it per `Protocol.md` §10.7, over
  `"ed3k v1 set grant" ‖ body`;
- `number` ≥ 1, `id` 1 to 32 bytes (`file-set.md` §5.1);
- `keys` holds 1 to 8 keys of 32 bytes each, no two equal;
- `required` from 1 to min(4, number of keys).

§1.2, §1.3 and [`pubsub.md`](pubsub.md) §4.1 use this definition only. Invalid grant → manifest
refused, head `REJECTED`.

- Owner key is a signer only if in `keys`; listing only other keys keeps it off the publishing machine.
- **A set that may get co-owners is created with an `id` of at most 16 bytes.** A joint head
  can't hold longer (§1.3); a set's `id` can't change. Joint head's `Head.name` is empty, so all
  co-owners sign the same bytes; storing node answers a joint head with a `name` `REJECTED`.
  Reader takes the name from manifest or link.
- A grant has no end date; the owner ends it with a higher-`number` grant.
- **The owner stays above the group:** may replace the grant any time, with any keys.
  `required` guards against one bad co-owner, not the owner.

### 1.2 The manifest

```protobuf
// in SetFile (file-set.md §1):
SignedGrant grant = 4;
repeated CoSig cosigs = 5;    // each over "ed3k v1 set cosig" ‖ body: not the sole-author string "ed3k v1 set manifest" of file-set.md §2.4, so a cosig is no signature of a set of one's own
```

- `SetFile.key` = owner key, as before. `SetFile.sigs` empty: a signed manifest has exactly one
  of `sigs` and `grant`, `cosigs` only with `grant`
  ([`file-set.md`](file-set.md) §2.4).
- **A valid `CoSig`.** `signer` indexes `Grant.keys` from 0; `sigs` valid under that key per
  `Protocol.md` §10.7, unknown-algorithm signatures ignored as there. Out-of-range
  `signer`, a second `CoSig` of one signer, or a non-verifying `CoSig` invalidates the whole
  manifest or head (§1.3); not skipped.
- A manifest with a grant is valid iff: grant valid (§1.1), `grant.key` = `SetFile.key`,
  `Grant.id` = manifest's `id`, every `CoSig` valid, ≥ `required` of them. Else refused.
- Reader remembers the highest grant `number` accepted per set; refuses manifests with a lower
  number, and grantless ones once it has seen a grant.

### 1.3 The head

```protobuf
// in Record (§10.7 of Protocol.md):
Joint joint = 8;

message Joint {
  bytes  head = 1;            // a serialised Head (file-set.md §5.2)
  SignedGrant grant = 2;
  repeated CoSig cosigs = 3;  // each over "ed3k v1 joint head" ‖ target(16) ‖ Grant.number(8, big-endian) ‖ head
}
```

A `HEAD` record at the set's target, with `joint` in place of `head`, record-signed by any one
grant key. That signature only allows re-storing with a new `expires`; the `cosigs` count. They
bind the grant's `number`, so a head cosigned under an earlier grant can't be carried under a
later one to outrank that grant's heads.

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| Grant | valid grant (§1.1); else `REJECTED` | same, and `grant.key` = key it searched for |
| Target | `H("ed3k v1 set head", grant.key ‖ Grant.id)[0..16)` = request's target, and `Head.id` = `Grant.id`; else `REJECTED` | same |
| Record signature | `Store.signer` (or connection's key if `signer` absent) is one of `Grant.keys`, record signature valid under it; else `REJECTED` | record signature valid under one of `Grant.keys` |
| `cosigs` | every `CoSig` valid (§1.2) over the bytes above: `signer` in range, no signer twice, signatures verify; and exactly `required` of them; else `REJECTED`. Exactly, not at least: record must fit 1 KiB, and storing node makes a fixed number of checks. Publisher with more valid cosigs sends `required`; a manifest is a file, takes `required` or more (§1.2) | same; failing head dropped |
| Order | by `Grant.number`, then `Head.seq`, then `Head.root`, lower root = higher in order ([`file-set.md`](file-set.md) §5.2). A head without `joint` has number 0. Replaced only by a higher record, or an equal one with later `expires`; a valid non-replacing record → `OLDER` | highest in this order wins |
| `expires` | as for any head | as for any head |

- Once a storing node holds a joint head, an owner-key-only head no longer replaces it. To
  write alone again, owner signs a grant naming its own key.
- A removed co-owner can still show an old head with the old grant to a storing node that never saw
  the new one, as with any old head (`file-set.md` §5.2); the order above makes searchers
  prefer the newer grant.
- With 8 keys and 4 signatures the record is about 900 bytes, inside the 1 KiB of `Protocol.md` §10.7.
  Hence in a joint head `Head.name` is empty (§1.1) and `id` at most 16 bytes; else storing node →
  `REJECTED`.

Reader follows the set per `file-set.md` §5.3, with §1.2 of this document replacing "signed by the same key", plus:

- **Step 2 compares in the table's order:** grant `number`, then `seq`, then lower `root`.
  Grantless head or manifest = number 0. So a head under a newer grant is followed even if its
  `seq` isn't higher; one under an older grant never, whatever its `seq`.
- **The manifest carries the grant of its head.** The manifest a joint head names MUST have a
  grant with the same `number` as the head's; else reader ignores the head, keeps the old
  version.

---

## 2. A directory handed over

A member can be a **mount**: a directory holding the newest version of another author's set.

```protobuf
// in Member (file-set.md §1), together with `set`:
Mount mount = 10;

message Mount {
  bytes  key = 1;             // 32 B owner key of the set that fills this directory
  bytes  id = 2;              // its id
  uint64 max_size = 3;        // sum of file sizes it may hold; 0 = no limit (file-set.md §2.1 has none on sizes)
  uint32 max_files = 4;       // number of files it may hold; 0 = likewise
}
```

- Member is an inner set (`file-set.md` §2.5); `root` and `size` name the version the outer
  author knew when writing the manifest. `root` MUST name a version (32 bytes, as
  `file-set.md` §2.1 demands of every member): a set with no version yet can't be mounted.
- **The version named MUST be a version of the set (`key`, `id`):** manifest signed by `key`
  (`file-set.md` §2.4) or with a valid grant of `key` (§1.2), carrying `id`. Else member fails,
  like a refused inner manifest (`file-set.md` §2.5).
- Reader follows set (`key`, `id`) like any changing set, by its own head. A newer version
  replaces the outer manifest's one when valid and within limits; otherwise the previous stays.
- **Limits measured on manifests the reader opened,** not claims: files listed by the mounted
  version and inner sets below, and their `size` sum, count against `max_files` and `max_size`. Every `files` entry counts, `set` members included,
  with its `size`, as for the total of `file-set.md` §2.5. `set_files` and `set_size` are for display
  (`file-set.md` §2.5), unused here. A mount below counts with the version the reader
  shows: the one its outer manifest names until a newer is taken. Newer mount version taken
  only once every manifest below it is fetched or refused (a refused one counts nothing), and
  only if it and every mount above stay within limits; else previous kept. A manifest below
  not fetched yet holds the newer version back; one the reader holds, verified, counts as fetched; `file-set.md` §2.5 "nothing counted until it
  is" is for display only here.
- **The outer signature covers the mount, not what will be in it:** this key may fill this
  directory, up to these limits.
- Mounted set may have co-owners (§1) and own mounts; `file-set.md` §2.5 depth and total
  limits hold over all. It MUST NOT have open parts (§3); a reader ignores them
  there.
- ≤ 64 *(initial)* mounts per manifest; each costs a follower one more head to watch.
- Reader refuses a manifest with more mounts, a `mount` on a member not marked `set`, or a
  malformed mount (`key` not 32 bytes, `id` empty or above 32 bytes).
- **One set has at most 256 *(initial)* mounts over all its levels,** mounts inside mounted sets
  included, counted in the order the reader opens manifests. A mount in any inner set, mounted
  or not, counts and is followed while within them. Beyond that: not followed; reader shows the
  outer manifest's version, watches no head.
- Outer author takes the directory back with a new version leaving the mount out.
- Client without this extension sees an inner set with fixed root: the outer author's version.

---

## 3. Open parts

Owner sets rules once; any key meeting them can add a **part**: a small own set, shown in a
directory named after that key.

### 3.1 The rules

```protobuf
// in FileSet (file-set.md §1):
Open open = 8;

message Open {
  string dir = 1;             // directory of the set under which the parts appear; not empty
  repeated Issuer issuers = 2;  // empty = every key may add a part
  uint64 max_size = 3;        // per part, when there is no issuer; 0 = the caps of §3.1 only
  uint32 max_files = 4;       // 0 = likewise
  repeated Change changes = 5;  // decisions about single authors
}

message Issuer {
  bytes  key = 1;             // 32 B Ed25519 key
  string name = 2;            // shown with the names it certifies
  uint64 max_size = 3;        // per part of an author with a certificate of this issuer; 0 = the caps of §3.1 only
  uint32 max_files = 4;       // 0 = likewise
}

message Change {
  bytes  author = 1;          // 16 B node ID of the author's key
  bool   banned = 2;          // no part of this author is shown
  uint64 below = 3;           // versions of the part with seq below this are not shown
  uint64 max_size = 4;        // other limits for this author; 0 = no change
  uint32 max_files = 5;
}
```

- Rules are part of the manifest: signed by the owner or co-owners, changed by a new version.
- At most 8 *(initial)* issuers and 4,096 *(initial)* changes. Reader refuses a manifest with
  more, or an `Issuer.key` not 32 bytes, or a `Change.author` not 16 bytes.
- **Caps**: a part ≤ 64 MiB and 256 files *(initial)* whatever the rules say, its manifest
  ≤ 64 KiB.
- `max_size`, `max_files` of `Open` and `Issuer` lower the caps; 0 = caps alone.
- `dir` MUST NOT be empty; follows path rules of `file-set.md` §2.2 and MUST NOT hold own
  members or directories: no member's directory and no `dirs` entry is `dir` or below it, no
  member's path is `dir` or a directory above it. Reader refuses a manifest breaking this.
- **Several changes for one author:** banned if any says `banned`; every `below` applies (highest
  counts); `max_size`, `max_files` each from the last change in `changes` setting it non-0.

### 3.2 The certificate

Issuer vouches a key belongs to a name. How an author gets one is outside ed3k (web page, chat,
meeting).

```protobuf
message Cert {
  bytes  issuer = 1;          // 32 B key of the issuer
  string name = 2;            // at most 64 B
  uint64 expires = 3;         // Unix time in seconds; 0 = never
  repeated Signature sigs = 4;  // by the issuer, over "ed3k v1 part cert" ‖ author key(32) ‖ expires(8, big-endian) ‖ name
}
```

- A certificate is not tied to a set; good wherever the issuer is accepted.
- Set without issuers: no certificate needed, open to all, limits the only brake.

### 3.3 The part

A part is a signed manifest (`file-set.md` §2.4) by the author's key, with

```
part id = H("ed3k v1 part", owner key ‖ id of the set)[0..16)        the `id` of the part's manifest
seq     = its version number
```

```protobuf
// in SetFile (file-set.md §1):
Cert cert = 6;                // for display only; the certificate that counts is in the PART record (§3.4)
```

- One key, one part per set.
- `SetFile.cert` MAY carry the author's certificate so a client with only the manifest can show
  the certified name. Not checked against set rules; never a reason to accept or refuse a part: the certificate that counts is the one in the `PART` record (§3.4, §3.5).
- A part holds files only: no inner sets, mounts, rules or grant.
- Readers show its files under `<dir>/<node ID of the author, 32 hex digits, lower case (Protocol.md §6.1)>/`.
  A client MAY show the certified name next to it.

### 3.4 Finding the parts

```protobuf
// in enum Kind (§10.3 of Protocol.md):
PART = 9;

// in Record (§10.7 there):
Part part = 9;

message Part {
  bytes  set_key = 1;         // 32 B owner key of the set
  bytes  id = 2;              // id of the set
  Cert   cert = 3;            // absent for a set without issuers
  uint64 seq = 4;             // version of the part
  bytes  root = 5;            // 32 B root of the part's manifest
  uint64 size = 6;
}
```

```
target = H("ed3k v1 parts", set_key ‖ id ‖ issuer key)[0..16)        issuer key empty when there is no certificate
```

Record: a `Signed` of kind `PART`, author-signed; the part's head. In `Store.head` (with
`Store.signer` when the author's key isn't the connection's) and `KadAnswer.sets`. Search paged with `offset` and `more`; of a `Query` it uses `offset` only.
Answer holds at most 16 *(initial)* records, in the node's own order by
`H("ed3k v1 page", secret ‖ author key)`, as in `Protocol.md` §10.3.

| | Storing node, on `Store` | Reader, on `KadAnswer` |
|---|---|---|
| Signature | valid under `signer` (connection's key if `signer` absent) | valid under `keys[key]` |
| Target | the one above, from record fields | the one searched |
| Content | `part` only; else `REJECTED` | same |
| Size | `set_key` 32 B, `id` 1 to 32 B, `root` 32 B, `cert.issuer` 32 B, `cert.name` at most 64 B; else `REJECTED` | same |
| Certificate | if present: valid under `cert.issuer` for the signing key, not expired; if `cert.expires` is not 0, `Record.expires` not later than `cert.expires`; else `REJECTED` | same, and issuer is one of the rules |
| `expires` | later than now, at most 7 days *(initial)* plus 1 h ahead | later than now minus 1 h |
| Count | one record per author per target; replaced by higher `seq`, same `seq` with lower `root`, or same `seq` and `root` with later `expires`; valid non-replacing record → `OLDER`; at most 8 per host and 32 per site ([`ip-diversity.md`](ip-diversity.md) §2.4), `REJECTED` beyond. Kept record's host and site = address of the `Store` that last stored it, renewal included, also a `Store` with `signer`: follower's re-store counts at follower's address; over the limit there → `REJECTED`, held record keeps its host and site. At most 1,024 *(initial)* records per target, then `FULL` | highest `seq` per author; among equal ones the lower `root` |

- **With an issuer, the storing node keeps the door:** target contains the issuer's key, node
  checks the certificate against it; a key lacking that issuer's certificate can't store.
- **Without an issuer, anyone can store**, bounded by per-address flood limits, per-host and
  per-site limits, per-target count, reader's checks.
- Reader searches one target per rule issuer, or an open set's one target.
- A follower holding a part SHOULD re-store its record when its own search stops returning it;
  author need not be online.

### 3.5 What a reader accepts

Part shown when all hold:

1. Its record passes the checks above.
2. Author not `banned`, and `seq` not `below` a change's value for it.
3. Manifest has the record's root, is author-signed, carries the §3.3 part id as `id` and the
   record's `seq`, holds files only.
4. Inside the limits: those of a change for this author, else of its issuer, else of `open`.

A failing part is not shown; an older version that passed is kept.

- **One author, several issuers.** Certificates of two rule issuers → a record under each
  target, still one part: reader takes highest `seq`, then lower `root`, then the issuer earlier
  in `Open.issuers`. Step 4 limits are that issuer's.

- **Listing is automatic, downloading is not.** A client MAY fetch part manifests to show them;
  it MUST NOT download a part's files unless the user chose so for this set, author, or file.
- Client reads ≤ 1,024 *(initial)* parts per set; pages a part target until no node sets `more`
  or it holds that many.
- Followers stop re-storing a part no longer shown. As everywhere, a client doesn't delete its
  files without the user (`file-set.md` §5.3).
- An author named in a followed filter list ([`filter-list.md`](filter-list.md)) is marked like
  any other author.

---

## 4. What an attacker can do

| Attack | What limits it |
|---|---|
| **A co-owner publishes alone** | `required`, checked by storing nodes and readers |
| **A removed co-owner publishes with the old grant** | order by grant number; owner's next head carries the new grant |
| **The owner key is taken** | nothing. Owner key can replace every grant ([`file-set.md`](file-set.md) §10) |
| **A mounted author fills the directory** | `max_size` and `max_files`; outer author removes the mount |
| **Junk parts in a set with an issuer** | storing node refuses keys without certificate; issuer decides who gets one |
| **Junk parts in an open set** | per-part limits, 1,024 records per target, no automatic download. An open set can be filled; that's what open means |
| **Filling the target so that real parts find no room** | with issuer: impossible w/o certificates. Without: possible from 32 sites (keys free); only price: per-site limit of [`ip-diversity.md`](ip-diversity.md) §2.4, per-address rate of `Protocol.md` §10.3 |
| **An issuer gives certificates to everyone** | owner removes the issuer from the rules |
| **Unwanted content in a part** | `banned` and `below` in the next version; user's own filter lists |

**What it shows.** Part record names set and author key; storing nodes see who stores it. A
certificate ties key to name wherever shown.

---

## 5. What it costs

| | |
|---|---|
| Joint head | ~400 bytes grant + 72 per signature, record ≤ 1 KiB |
| Mount | one more head to watch per mount: a search every 6 h, or a [`pubsub.md`](pubsub.md) topic |
| Open parts | one paged search per issuer and check; ~250 bytes per part record, so 1,024 parts ≈ 256 KiB per check |

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Co-owners share one key** | nothing to specify | no way to remove one; no `required` |
| **A threshold signature**: the group makes one signature together | head looks like any; storing nodes need no new check | co-owner signing protocol; group can't change w/o new key, i.e. new set |
| **A grant with an end date** | a lost co-owner key ends by itself | owner must come back before the date, or the set stops |
| **Mount without limits** | simpler | outer author signs a blank cheque for its own set's size |
| **Each part under the author's own target**, and a list of authors kept by the owner | no new record kind | owner approves every author by hand, i.e. form 2 |
| **Parts passed between followers**, in the manner of [`pubsub.md`](pubsub.md) | no per-target limit; every hop checks full rules | followers must hold, compare part lists; topic exchange can't today |
| **Rules by pattern on the name**, as ZeroNet has | finer rules per user group | a pattern language in a signed format; per-issuer limits do most of it |
| **Removal by date** | one value removes everything old | a part's date is the author's claim |
| **Certificates per set** | issuer can admit an author to one set only | author needs one per set; issuer must know the sets |

---

## 7. Open points

- Push: new part reaches followers at next check; a parts topic needs the exchange named in
  section 6.
- More than 1,024 parts per target.
- Hidden sets ([`hidden-set.md`](hidden-set.md)) with co-owners, mounts or parts: not defined.
- Whether an issuer should be able to revoke a certificate before it expires.
- Whether an author should be able to sign with a second key on behalf of its part, as a
  co-owner does for a set.
- All numbers are first values.
