# Extension: sets with several authors (`multi-author`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §2.4, §2.5 and §5, and [`Protocol.md`](../Protocol.md) §10.1, §10.3 and §10.7. Section numbers (§) refer to this document unless a document is named.
**Capability:** none of its own; a node that uses it sets `SETS`. Record kind 9. Two fields in `Record`, three in a manifest file, one in `FileSet`, one in `Member`.
**Required of every `ROUTER`:** the checks of a joint head (§1.3) and storing and returning the record kind `PART` (§3.4). The rest is optional.

A set that changes has one author: one key signs every version
([`file-set.md`](file-set.md) §5). This extension lets more than one key write to a set, in
three forms. They can be used together.

| Form | Who may write | Approved by | Section |
|---|---|---|---|
| **Co-owners** | a fixed group of keys, to the whole set | the owner, key by key | 1 |
| **A directory handed over** | one other author, to one directory | the owner, key by key | 2 |
| **Open parts** | everyone who meets the owner's rules, each to a directory of their own | nobody in advance; an issuer if the owner names one | 3 |

In every form the set keeps its name, the pair of the **owner key** and `id`, and its
`ed3k:set` link.

The three forms are those of a ZeroNet site
([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.4).

---

## 1. Co-owners

### 1.1 The grant

The owner key signs a **grant**: the list of keys that may sign versions of the set, and how
many of them are needed.

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

**A valid grant.** A `SignedGrant` is valid when all of this holds *(numbers initial)*:

- `key` is 32 bytes, and `sigs` is valid under it by the rules of `Protocol.md` §10.7, over
  `"ed3k v1 set grant" ‖ body`;
- `number` is 1 or higher, and `id` is at most 32 bytes (`file-set.md` §5.1);
- `keys` holds 1 to 8 keys of 32 bytes each, no two equal;
- `required` is from 1 to min(4, number of keys).

§1.2, §1.3 and [`pubsub.md`](pubsub.md) §4.1 use this definition and no other. A manifest
whose grant is not valid is refused, and a head whose grant is not valid is `REJECTED`.

- The owner key is a signer only when it is in `keys`. An owner who lists other keys alone can
  keep its own key off the machine that publishes.
- **A set that may get co-owners is created with an `id` of at most 16 bytes.** A joint head
  holds no longer one (§1.3), and the `id` of a set cannot change. `Head.name` of a joint head
  is at most 64 bytes: it MAY be empty, or the manifest's name cut at a code point boundary.
- A grant has no end date. The owner ends it with a grant of a higher `number`.
- **The owner stays above the group.** It can replace the grant at any time, with any keys.
  `required` protects against one bad co-owner. It does not protect against the owner.

### 1.2 The manifest

```protobuf
// in SetFile (file-set.md §1):
SignedGrant grant = 4;
repeated CoSig cosigs = 5;    // each over "ed3k v1 set cosig" ‖ body: not the sole-author string "ed3k v1 set manifest" of file-set.md §2.4, so a cosig is no signature of a set of one's own
```

- `SetFile.key` is the owner key, as before. `SetFile.sigs` is left empty: a signed manifest
  has exactly one of `sigs` and `grant`, and `cosigs` only with `grant`
  ([`file-set.md`](file-set.md) §2.4).
- **A valid `CoSig`.** `signer` is an index into `Grant.keys`, counted from 0, and `sigs` is
  valid under that key by the rules of `Protocol.md` §10.7, signatures of unknown algorithms
  being ignored as there. A `CoSig` whose `signer` is out of range, a second `CoSig` of one
  signer, or a `CoSig` that does not verify makes the whole manifest, or the whole head
  (§1.3), invalid. It is not skipped.
- A manifest with a grant is valid when the grant is valid (§1.1), `grant.key` is
  `SetFile.key`, `Grant.id` is the `id` of the manifest, every `CoSig` is valid, and there
  are at least `required` of them. Else it is refused.
- A reader remembers the highest grant `number` it has accepted for a set. It refuses a
  manifest with a lower number, and one without a grant once it has seen a grant.

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

The record is of kind `HEAD`, at the target of the set, and carries `joint` in place of `head`.
It is signed, as a record, by any one key of the grant. That signature only lets the record be
stored again with a new `expires`; what counts are the `cosigs`. They bind the grant's
`number`, so a head cosigned under an earlier grant cannot be carried under a later one and
outrank the heads of that grant.

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| Grant | a valid grant (§1.1); else `REJECTED` | the same, and `grant.key` is the key it searched for |
| Target | `H("ed3k v1 set head", grant.key ‖ Grant.id)[0..16)` is the target of the request, and `Head.id` equals `Grant.id`; else `REJECTED` | the same |
| Record signature | by a key of the grant | the same |
| `cosigs` | every `CoSig` is valid (§1.2) over the bytes above: `signer` in range, no signer twice, signatures that verify; and there are exactly `required` of them; else `REJECTED` | the same; a head that fails is dropped |
| Order | by `Grant.number`, then by `Head.seq`, then by `Head.root`, where the lower root is the higher in order ([`file-set.md`](file-set.md) §5.2). A head without `joint` has number 0. A record is replaced only by one that is higher in this order, or equal with a later `expires`; a valid record that does not replace it is answered `OLDER` | the highest in this order wins |
| `expires` | as for any head | as for any head |

- Once a storing node holds a joint head, a head signed by the owner key alone no longer
  replaces it. An owner who wants to write alone again signs a grant that names its own key.
- A removed co-owner can still show an old head with the old grant to a storing node that
  never saw the new one, as with any old head (`file-set.md` §5.2). The order above makes a
  searcher prefer the newer grant.
- With 8 keys and 4 signatures the record is about 900 bytes, inside the 1 KiB of `Protocol.md` §10.7.
  In a joint head `Head.name` is therefore at most 64 bytes and `id` at most 16 bytes; a
  storing node answers `REJECTED` above that.

A reader follows the set as in `file-set.md` §5.3, with §1.2 in place of "signed by the same
key", and with two additions:

- **Step 2 compares in the order of the table above:** by the grant's `number` first, then by
  `seq`, then by the lower `root`. A head or a manifest without a grant has number 0. A head
  under a newer grant is therefore followed even when its `seq` is not higher, and one under
  an older grant is not, whatever its `seq`.
- **The manifest carries the grant of its head.** The manifest that a joint head names MUST
  have a grant with the same `number` as the grant of the head. Else the reader ignores the
  head and keeps the old version.

---

## 2. A directory handed over

A member can be a **mount**: a directory whose content is the newest version of another
author's set.

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

- The member is an inner set (`file-set.md` §2.5). Its `root` and `size` name the version the
  outer author knew when it wrote the manifest. `root` MUST name a version (32 bytes, as
  `file-set.md` §2.1 demands of every member): a set that has no version yet cannot be mounted.
- **The version named MUST be a version of the set (`key`, `id`):** a manifest signed by `key`
  (`file-set.md` §2.4), or with a valid grant of `key` (§1.2), that carries `id`. Else the
  member fails, as an inner manifest that is refused does (`file-set.md` §2.5).
- A reader follows the set (`key`, `id`) as any set that changes, by its own head. A newer
  version replaces the one named in the outer manifest when it is valid and inside the limits.
  A version that is not keeps the one before.
- **The limits are measured on the manifests the reader opened,** not on claims: the files
  that the mounted version and the inner sets below it list, and the sum of their `size`
  values, count against `max_files` and `max_size`. `set_files` and `set_size` are for display
  (`file-set.md` §2.5) and are not used for this.
- **The outer signature covers the mount, not what will be in it.** It says: this key may fill
  this directory, up to these limits.
- The mounted set may have co-owners (§1) and mounts of its own. The depth and total limits of
  `file-set.md` §2.5 hold over all of them. It MUST NOT have open parts (§3); a reader ignores
  them there.
- A manifest has at most 64 *(initial)* mounts. Each costs a follower one more head to watch.
- A reader refuses a manifest with more mounts, with a `mount` on a member that is not marked
  `set`, or with a malformed mount: a `key` that is not 32 bytes, or an `id` that is empty or
  above 32 bytes.
- **One set has at most 256 *(initial)* mounts over all its levels,** mounts inside mounted
  sets included, counted in the order in which the reader opens the manifests. A reader does
  not follow a mount beyond that: it shows the version the outer manifest names and watches
  no head for it.
- The outer author takes the directory back with a new version that leaves the mount out.
- A client without this extension sees an inner set with a fixed root, and gets the version
  the outer author named.

---

## 3. Open parts

The owner sets rules once. Any key that meets them can add a **part**: a small set of its own
that readers show in a directory named after that key.

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

- The rules are part of the manifest, so they are signed by the owner or its co-owners and
  change with a new version.
- At most 8 *(initial)* issuers and 4,096 *(initial)* changes. A reader refuses a manifest
  with more, or with an `Issuer.key` that is not 32 bytes or a `Change.author` that is not
  16 bytes.
- A part is at most 64 MiB and 256 files *(initial)* whatever the rules say, and its manifest
  at most 64 KiB. These are the **caps**.
- `max_size` and `max_files` of `Open` and of an `Issuer` lower the caps. A value of 0 sets no
  limit of its own: the caps alone hold.
- `dir` MUST NOT be empty. It follows the path rules of `file-set.md` §2.2 and MUST NOT hold
  members or directories of its own: no member's directory is `dir` or lies below it, no
  entry of `dirs` is `dir` or lies below it, and no member's path is `dir` or a directory
  above it. A reader refuses a manifest that breaks this.
- **Several changes for one author.** The author is banned when any of them says `banned`.
  Every `below` applies, so the highest one counts. Of `max_size` and `max_files`, each is
  taken from the last change in `changes` that sets it to a value other than 0.

### 3.2 The certificate

An issuer vouches that a key belongs to a name. How an author gets a certificate is not part
of ed3k: a web page, a chat, a meeting.

```protobuf
message Cert {
  bytes  issuer = 1;          // 32 B key of the issuer
  string name = 2;            // at most 64 B
  uint64 expires = 3;         // Unix time in seconds; 0 = never
  repeated Signature sigs = 4;  // by the issuer, over "ed3k v1 part cert" ‖ author key(32) ‖ expires(8, big-endian) ‖ name
}
```

- A certificate is not tied to a set. It is good wherever the issuer is accepted.
- A set without issuers needs no certificate. It is open to everyone, and its limits are the
  only brake.

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

- One key has one part per set.
- `SetFile.cert` MAY carry the author's certificate so that a client that has the manifest
  alone can show the certified name. It is not checked against the rules of the set, and a
  part is never accepted or refused because of it: the certificate that counts is the one in
  the `PART` record (§3.4, §3.5).
- A part holds files only: no inner sets, no mounts, no rules, no grant.
- Readers show its files under `<dir>/<node ID of the author, 32 hex digits, lower case (Protocol.md §6.1)>/`. A client MAY show the
  certified name next to it.

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

The record is a `Signed` of kind `PART`, signed by the author. It is the head of the part. It
travels in `Store.head`, with `Store.signer` when the author's key is not the connection's,
and in `KadAnswer.sets`. A search is paged with
`offset` and `more` like a keyword search; an answer holds at most 16 *(initial)* records, in
an order of the node's own by `H("ed3k v1 page", secret ‖ author key)`, as in `Protocol.md`
§10.3.

| | Storing node, on `Store` | Reader, on `KadAnswer` |
|---|---|---|
| Signature | valid under `signer`, or under the connection's key when `signer` is absent | valid under `keys[key]` |
| Target | the one above, from the fields of the record | the one it searched |
| Content | `part` and nothing else; else `REJECTED` | the same |
| Size | `set_key` 32 B, `id` 1 to 32 B, `root` 32 B, `cert.issuer` 32 B, `cert.name` at most 64 B; else `REJECTED` | the same |
| Certificate | when present: valid under `cert.issuer` for the signing key, and not expired | the same, and the issuer is one of the rules |
| `expires` | later than now, at most 7 days *(initial)* plus 1 h ahead | later than now minus 1 h |
| Count | one record per author under a target; a higher `seq` replaces it, and so do the same `seq` with a lower `root`, and the same `seq` and `root` with a later `expires`; a valid record that does not replace is answered `OLDER`; at most 8 per host and 32 per site ([`ip-diversity.md`](ip-diversity.md) §2.4), `REJECTED` beyond; at most 1,024 *(initial)* records per target, then `FULL` | the highest `seq` per author; among equal ones the lower `root` |

- **With an issuer, the storing node keeps the door.** The target contains the issuer's key,
  and the node checks the certificate against it. A key without a certificate of that issuer
  cannot store there.
- **Without an issuer, anyone can store.** What limits it are the flood limits per address,
  the limits per host and per site, the count per target, and the reader's checks.
- A reader searches one target per issuer of the rules, or the one target of an open set.
- A follower that holds a part SHOULD store its record again when its own search no longer
  returns it. The author need not be online.

### 3.5 What a reader accepts

A part is shown when all of this holds:

1. Its record passes the checks above.
2. The author is not `banned`, and `seq` is not `below` the value of a change for it.
3. Its manifest has the root of the record, is signed by the author, carries the part id of
   §3.3 as its `id` and the `seq` of the record, and holds files only.
4. It is inside the limits: those of a change for this author, else those of its issuer, else
   those of `open`.

A part that fails is not shown, and an older version that passed is kept.

- **One author, several issuers.** An author with certificates of two issuers of the rules
  has a record under the target of each. It still has one part: the reader takes the record
  with the highest `seq`, among equal ones the lower `root`, and among those the record of the
  issuer that stands earlier in `Open.issuers`. The limits of step 4 are those of that issuer.

- **Listing is automatic, downloading is not.** A client MAY fetch part manifests to show
  them. It MUST NOT download the files of a part unless the user chose that for this set, for
  this author, or for the file.
- A client reads at most 1,024 *(initial)* parts per set.
- A part that is no longer shown is no longer stored again by followers. As everywhere, a
  client does not delete its files without the user (`file-set.md` §5.3).
- An author named in a filter list the user follows
  ([`filter-list.md`](filter-list.md)) is marked like any other author.

---

## 4. What an attacker can do

| Attack | What limits it |
|---|---|
| **A co-owner publishes alone** | `required`, checked by storing nodes and readers |
| **A removed co-owner publishes with the old grant** | the order by grant number; the owner's next head carries the new grant |
| **The owner key is taken** | nothing. The owner key can replace every grant ([`file-set.md`](file-set.md) §10) |
| **A mounted author fills the directory** | `max_size` and `max_files`; the outer author removes the mount |
| **Junk parts in a set with an issuer** | the storing node refuses keys without a certificate; the issuer decides who gets one |
| **Junk parts in an open set** | per-part limits, 1,024 records per target, no automatic download. An open set can be filled; that is what open means |
| **Filling the target so that real parts find no room** | with an issuer: not possible without certificates. Without: possible from 32 sites, since keys are free; the per-site limit of [`ip-diversity.md`](ip-diversity.md) §2.4 and the per-address rate of `Protocol.md` §10.3 are the only price |
| **An issuer gives certificates to everyone** | the owner removes the issuer from the rules |
| **Unwanted content in a part** | `banned` and `below` in the next version; the user's own filter lists |

**What it shows.** A part record names the set and the author's key, and its storing nodes
see who stores it. A certificate ties a key to a name wherever it is shown.

---

## 5. What it costs

| | |
|---|---|
| Joint head | about 400 bytes for the grant and 72 per signature, in a record of at most 1 KiB |
| Mount | one more head to watch per mount: a search every 6 h, or a topic of [`pubsub.md`](pubsub.md) |
| Open parts | one paged search per issuer and check; about 250 bytes per part record, so 1,024 parts are about 256 KiB per check |

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Co-owners share one key** | nothing to specify | no way to remove one; no `required` |
| **A threshold signature**: the group makes one signature together | the head looks like any head, and storing nodes need no new check | a signing protocol between the co-owners; the group cannot change without a new key, which is a new set |
| **A grant with an end date** | a lost co-owner key ends by itself | the owner has to come back before the date, or the set stops |
| **Mount without limits** | simpler | the outer author signs a blank cheque for the size of its own set |
| **Each part under the author's own target**, and a list of authors kept by the owner | no new record kind | the owner approves every author by hand, which is form 2 |
| **Parts passed between followers**, in the manner of [`pubsub.md`](pubsub.md) | no limit per target; every hop checks the full rules | followers must hold and compare lists of parts, which the topic exchange cannot do today |
| **Rules by pattern on the name**, as ZeroNet has | finer rules per group of users | a pattern language in a signed format; limits per issuer do most of it |
| **Removal by date** | one value removes everything old | the date of a part is the author's claim |
| **Certificates per set** | an issuer can admit an author to one set only | an author needs one per set, and the issuer must know the sets |

---

## 7. Open points

- Push: a new part reaches followers at the next check. A topic for parts needs the exchange
  named in section 6.
- More than 1,024 parts per target.
- Hidden sets ([`hidden-set.md`](hidden-set.md)) with co-owners, mounts or parts are not
  defined.
- Whether a certificate should be able to be taken back by its issuer before it expires.
- Whether an author should be able to sign with a second key on behalf of its part, as a
  co-owner does for a set.
- All numbers are first values.
