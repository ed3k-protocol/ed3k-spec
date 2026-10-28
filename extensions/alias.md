# Extension: other names of a file (`alias`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §7.4, §10.3, §10.7. § = this document unless a document is named.
**Capability:** none. Record kind 13, `ALIAS`. One field each in `Record`, `Store`, `KadAnswer`.
**Required of every `ROUTER`:** storing and returning the record kind `ALIAS` (§3), for every scheme, known or not. The rest is optional.

A file's one ed3k identity: its BLAKE3 root (`Protocol.md` §7.1). Outside ed3k the same bytes
have other names (eD2K hash, torrent infohash, file-hoster link, magnet-link SHA-1). This is the
one way from such a name to the root.

- **`ALIAS` record**: *the thing with this name is the file, or the set, with this root.*
  Publisher-signed, stored in Kad under a hash of the name.
- Kind of name = **scheme**, a short string; storing nodes need no scheme knowledge to store
  its records. **A later-joined network or data source needs a scheme name and a document; no new
  record kind, no network upgrade.**
- §5: rules for every such document, so whatever is joined ends under the root.

Users: [`torrent.md`](torrent.md), [`hoster.md`](hoster.md) (hoster links), [`usenet.md`](usenet.md).

---

## 1. The record

```protobuf
// in enum Kind (Protocol.md §10.3):
ALIAS = 13;

// in Record (Protocol.md §10.7):
Alias alias = 11;

message Alias {
  string scheme = 1;          // the kind of name (§2)
  bytes  id = 2;              // the name, in the form the scheme fixes; 1 to 64 B
  Relation rel = 3;
  bytes  root = 4;            // 32 B BLAKE3 root
  uint64 size = 5;            // size of the file with that root
  string name = 6;            // its file name
  uint32 set_files = 7;       // SET and PACKED: as in FileInfo (file-set.md §3)
  uint64 set_size = 8;
  string entry = 9;           // PACKED: path of the file inside the archive, "/" between components; absent = the only file
  string password = 10;       // PACKED: password of the archive, if it has one

  enum Relation {
    SAME = 0;                 // the bytes that `id` names are the file `root`
    SET = 1;                  // `id` names several files; `root` is the manifest that lists them
    PACKED = 2;               // `root` is the manifest of an archive; unpacked, it gives the file `id` names
  }
}

// in Store (Protocol.md §10.3):
repeated Signed aliases = 11; // ALIAS

// in KadAnswer:
repeated Signed aliases = 18; // answer to a search of kind ALIAS
```

```
target = H("ed3k v1 alias", len(scheme) as 1 byte ‖ scheme ‖ id)[0..16)
```

- Signed by publisher like a keyword record (`Protocol.md` §10.7). Lifetime 24 h *(initial)*.
- `scheme`: 1 to 16 bytes of `a`–`z`, `0`–`9`, `-`.
- `entry` ≤ 255 bytes, `password` ≤ 128 bytes.
- Target hashes the name, but `scheme`, `id` are record fields: storing node holds every stored
  name in clear, unless the scheme's `id` is itself a hash (`url`, [`hoster.md`](hoster.md) §4).

---

## 2. Schemes

| Scheme | `id` | Relation | Defined in |
|---|---|---|---|
| `ed2k` | 16 B eD2K hash ‖ file size as 8 B, big endian | `SAME` | `Protocol.md` §7.4, §14.4 |
| `md5` | 16 B MD5 of the file | `SAME` | §4.1 |
| `sha1` | 20 B SHA-1 of the file | `SAME` | §4.1 |
| `sha256` | 32 B SHA-256 of the file | `SAME` | §4.1 |
| `btih` | 20 B infohash, version 1 or hybrid torrent | `SET` | [`torrent.md`](torrent.md) §5 |
| `btmh` | 32 B infohash, version 2 or hybrid torrent | `SET` | [`torrent.md`](torrent.md) §5 |
| `url` | 32 B hash of a link in canonical form | `SAME`, `SET` | [`hoster.md`](hoster.md) §4 |
| `b3` | 32 B BLAKE3 root of a file | `PACKED` | [`hoster.md`](hoster.md) §6 |
| `nzb` | 32 B hash of a Usenet post's message IDs | `SET` | [`usenet.md`](usenet.md) §1 |
| `nzb-file` | 32 B hash of the message IDs of one file of a post | `SAME` | [`usenet.md`](usenet.md) §1 |

- **New scheme registered by a document** fixing `id` form, allowed relations, claim check (§5).
  Until listed here, an implementer needing one SHOULD use a name starting `x-`.
- Scheme names **content**, never where a node is. Names with changing bytes (edited web
  address, feed): handled by the scheme's document.

---

## 3. Publishing, storing, searching

**Publisher**

- A node publishes `ALIAS` only for a root it shares, and only when it has itself seen that name and root
  belong together: it holds the complete file under the root and computed the name from those bytes (an
  earlier hash of the file counts while its size and modification time are unchanged), or verified the same
  bytes under both (`Protocol.md` §14.4). Scheme documents say what this means for set, archive.
- It publishes ≤ 4 *(initial)* records per root and scheme.

**Storing node**

| Check | Rule |
|---|---|
| Signature, `expires` | as keyword record (`Protocol.md` §10.7) |
| Content | `alias` only. `root` 32 B, `id` 1 to 64 B, `scheme` per §1, `name` ≤ 255 B UTF-8, `entry` ≤ 255 B, `password` ≤ 128 B; else `REJECTED` |
| Target | request target = computed from `scheme` and `id` |
| Count | one record per publisher, target, root. Per target ≤ 2 *(initial)* roots per publisher key, 8 *(initial)* per publisher site ([`ip-diversity.md`](ip-diversity.md) §2.4); more → `REJECTED`, checked before target; a root counts as a file in `Protocol.md` §10.8. ≤ 64 *(initial)* distinct roots per target; more → `FULL`. ≤ 3 *(initial)* records kept per root per target (below) |

- No scheme-dependent checks; stores records of unknown schemes. Cannot check the claim.
- **Records per root.** ≤ 3 *(initial)* per root per target, distinct publishers, by
  `Protocol.md` §10.8. Kept record's site = site ([`ip-diversity.md`](ip-diversity.md) §1) of
  the address of the `Store` that last stored it, renewal included.
  - A record of an already-kept publisher replaces its old one, from any site; kept record takes this
    `Store`'s site.
  - Else a record from a site holding a place replaces that site's (least fresh one, if a renewal
    left two of one site).
  - Record from another site: added while fewer than 3 are kept, else replaces least fresh kept record.
  - `Store` with `signer` never replaces another publisher's record of another site →
    `REJECTED`. Else `STORED`.

  This is the storing node's limit; the 4 per root and scheme above limit one publisher over its names.
- **Sites per root.** Per root per target, remembers sites (as above: of each `Store`'s address)
  that stored a valid record for it, each until that record would have expired, even if
  displaced. `Store` with `signer` adds no site; only stores answered `STORED` count. Site counts
  once; count saturates at 255 *(initial)*. `sites` and the order below use this count.
- Search: ≤ 16 *(initial)* records in `aliases`, one per root, most publisher sites first; ties by `H("ed3k v1 page", secret ‖ root)` as bytes, `root` = 32 bytes of `Alias.root`,
  `secret` that of `Protocol.md` §10.3. Of a root's kept records the answer carries the one with the lowest
  `H("ed3k v1 page", secret ‖ root ‖ publisher key)`: no publisher picks which is shown, not
  even by frequent renewal. Site = group of [`ip-diversity.md`](ip-diversity.md) §1.
- `KadAnswer.sites` (`Protocol.md` §10.3): one entry per `aliases` entry, publisher sites counted for that
  root, 0 = not counted.
- Paged with `offset`, `more` as in `Protocol.md` §10.3, along this order; `offset` counts roots.

**Searcher**

- Checks signature, `expires`, content as keyword record (`Protocol.md` §10.7) and Content row
  above, and that `scheme`, `id` give the searched target; else drops it.
- **Every record is a claim;** §4 says how each relation is checked.
- Merges answers by `root`; site count = lower median of non-zero reported `sites` (0 if all
  0); ties → lower `root` first.
- Tries ≤ 3 *(initial)* roots per name, most publisher sites first.
- Publisher whose claim proved false: ignored from then on.

**What an attacker can do**

- **Fill a target** with 64 false roots, from ≥ 8 sites. Site ordering puts first the root named
  by honest nodes of several sites, displaced records included; each false record costs a
  signature under a node ID clients then ignore.
- **Make a client download a file it did not ask for.** §4 bounds cost per relation; nothing
  false is kept or passed on under the name.

---

## 4. Checking a claim

**The name the user started from is the judge.** A download started from an `ed3k:file` link is judged by its
root; an alias only adds sources. One started from a foreign name is judged by that name; the root is the claim.

| Relation | The claim is true when | Until then |
|---|---|---|
| `SAME`, name can check parts (`ed2k`) | same bytes pass under both hashes (`Protocol.md` §14.4) | false once a part fails under the name while its blocks pass under the root |
| `SAME`, name is a hash of the whole file | complete file gives the name (§4.1) | file not shown complete |
| `SAME` and `SET`, name is a place (`url`) | bytes fetched there pass under the root | place = source like any other ([`hoster.md`](hoster.md) §4) |
| `SET` (`btih`, `btmh`) | manifest fits the torrent; data passes under torrent hashes ([`torrent.md`](torrent.md) §3, §4.2) | false once a piece fails under the torrent while its blocks pass under the roots |
| `SET` and `SAME` (`nzb`, `nzb-file`) | manifest fits the article list; articles agree with roots ([`usenet.md`](usenet.md) §3, §4.2) | false once an article passes its own checksum while its block fails under the root |
| `PACKED` | archive, unpacked, gives the file `id` names ([`hoster.md`](hoster.md) §6) | archive is a download of its own |

### 4.1 A hash of the whole file

`md5`, `sha1`, `sha256`: whole-file hash, can't check parts. From magnet links (`urn:sha1`),
download pages, file hosters.

- Client downloads under the claimed root; every block verified under it, so every sender known.
- When complete, the client computes the hash; if it equals the name, the claim is true. Else it tells the user another file was found, doesn't show it
  as the requested file, and ignores the publisher.
- False claim costs one download of a file that is what its root says. Client SHOULD compare
  `size` with the size it knows for the name before starting.
- MD5, SHA-1 broken for collisions: two files may share one such name; the root names one.

---

## 5. Rules for joining another network or data source

Every document joining a network or byte source to ed3k follows these; they keep one identity.

| # | Rule |
|---|---|
| 1 | **One identity on the wire.** Every ed3k message names a file by root. No request carries a foreign name (`Protocol.md` §7.4) |
| 2 | **No foreign protocol inside ed3k.** Client speaks the other network with its peers; ed3k carries at most a description of where bytes are |
| 3 | **A name is found through `ALIAS`.** A scheme, no new record kind |
| 4 | **A pairing is a claim** until the client checked it itself (§4). Blame for a false one stated per scheme |
| 5 | **Checked is not verified.** Bytes passing only a foreign check: not in a `have` bitmap, not sent to an ed3k node, until they pass under the root |
| 6 | **One store of bytes.** A client's data of a file = byte ranges; each network's bitmaps are views (`Protocol.md` §14.4) |
| 7 | **Several files are a set.** Multi-file thing = manifest ([`file-set.md`](file-set.md)); each file keeps own root, sources. Where every holder can compute the same manifest, the document fixes a canonical form ([`torrent.md`](torrent.md) §2) |
| 8 | **Another form of the bytes is another file.** Archive, encrypted or encoded copy: own root. What it turns into = an `ALIAS` relation (`PACKED`), never a second identity of the file |
| 9 | **A source of bytes is a field.** Fetch locations told in the file stream and beside the `SOURCE` record, as for HTTP sources ([`http-source.md`](http-source.md) §2). A new source kind gets its own field and a capability bit, so a node lacking it is never handed something it would fetch wrongly |
| 10 | **Nothing is run that a peer sent.** Code talking to another network or site ships with the client, never over ed3k |

How networks fit; last column: where specified, or not.

| Network or source | Name | Relation | Bytes come from | Specified |
|---|---|---|---|---|
| eD2K | `ed2k` | `SAME` | eD2K peers; part hashes check parts of 9,728,000 bytes | `Protocol.md` §14 |
| BitTorrent | `btih`, `btmh` | `SET` | BitTorrent peers; piece hashes | [`torrent.md`](torrent.md) |
| Web servers, cache servers | none needed: URL travels with the file | — | HTTP | [`http-source.md`](http-source.md) |
| File hosters | `url`; `b3` for archives | `SAME`, `SET`, `PACKED` | a resolver turning a link into a byte stream | [`hoster.md`](hoster.md) |
| Magnet links with `urn:sha1`, download pages with a checksum | `sha1`, `md5`, `sha256` | `SAME` | ed3k sources | §4.1 |
| Usenet | `nzb`, `nzb-file`; `b3` for archives | `SET`, `SAME`, `PACKED` | news servers; article checksum = foreign check (rule 5) | [`usenet.md`](usenet.md) |
| A catalogue on a chain (a video site) | none: chain names the set, set names the chain | — | ed3k sources; site's server or IPFS gateway as HTTP source. Content ID in the link = a place, not a name | [`chaintube.md`](chaintube.md) |
| A network with a tree hash of its own (a content ID over a block graph, a tiger tree) | a scheme for that ID | `SAME` for a file, `SET` for a directory | its peers or gateways; own hashes check parts, like eD2K's | no |

---

## 6. What it gives and what it costs

| | |
|---|---|
| A later network | a scheme name; today's storing nodes store its records |
| Kad | one record kind for all names; one ~150 B record per publisher, name, root |
| What a storing node can check | record sits under its own name's hash; nothing about the name |
| What a storing node learns | name in clear, and that a publisher has a file with it; for `url` only the link hash |
| Trust | none added. Record = signed claim, ranked by how many sites make it |

---

## 7. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A record kind per network** (`TORRENT`, then one for links, …) | exact fields per kind; storing node can apply that network's checks | each new network = a kind every `ROUTER` must learn → joining = network upgrade. [`torrent.md`](torrent.md)'s first form |
| **A list of typed hashes in `FileInfo`**, in place of the one `ed2k` field | no lookup: names ride every file record | root → names, not name → root; grows every record; receiver can't check whole-file-hash names |
| **Requests by any hash type** | node could serve a file known only by a foreign name | second identity on the wire; data not verifiable under the root (`Protocol.md` §7.4) |
| **A scheme number** from a registry | 1 byte | two implementers adding a network at once collide; strings don't |
| **The name in clear** as the Kad target | storing node could check a name's form | storing nodes read every name; looking up a hoster link makes it public |

---

## 8. Open points

- `FileInfo.ed2k` (`Protocol.md` §7.4) and scheme `ed2k` say the same twice; when the field
  goes, the record remains.
- Search for one scheme's relation only? Today a target holds one scheme by construction.
- Reverse record, root → names. `MEMBER` does it for sets ([`file-set.md`](file-set.md) §6);
  nothing for plain names.
- All numbers are first values.
