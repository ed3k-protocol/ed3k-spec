# Extension: torrents as file sets (`torrent`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1 and §3, [`alias.md`](alias.md) §2, and [`Protocol.md`](../Protocol.md) §7.4, §13.2 and §14.4. Section numbers (§) refer to this document unless a document is named.
**Capability:** none of its own; a node that uses it sets `SETS`. Two schemes of the `ALIAS` record. One field in a manifest, one parameter in `ed3k:file` links.
**Required of every `ROUTER`:** nothing beyond the `ALIAS` record of [`alias.md`](alias.md).

A BitTorrent torrent is a list of files with SHA-1 or SHA-256 piece hashes. This extension
gives a torrent **one BLAKE3 root**, so that a client can download it from BitTorrent peers and
from ed3k nodes at once and verify every byte under the ed3k hash.

- A torrent is described by a file set ([`file-set.md`](file-set.md)): a manifest that lists
  the torrent's files with their BLAKE3 roots and names the torrent.
- The manifest has a **canonical form** (§2). Everyone who has the complete torrent computes
  the same manifest, byte for byte. Its root is the **torrent root**.
- An `ALIAS` record in Kad leads from the infohash to the torrent root (§5).
- Every file of the torrent stays an ordinary ed3k file. It has its own root and its own
  sources, also outside the torrent.

BitTorrent itself is not carried in ed3k. A client that uses this extension speaks BitTorrent
with BitTorrent peers and ed3k with ed3k nodes. Every ed3k request still names a file by its
root (`Protocol.md` §7.4).

NeoLoader joins BitTorrent and its own network in one download, with one hash over the whole
torrent. How it does that, and why ed3k does it in another way, is in
[`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md) §2.1.

---

## 1. The torrent set

```protobuf
// in FileSet (file-set.md §1):
Torrent torrent = 9;

message Torrent {
  bytes  btih = 1;            // 20 B: SHA-1 of the info dictionary; a version 1 or hybrid torrent
  bytes  btmh = 2;            // 32 B: SHA-256 of the info dictionary; a version 2 or hybrid torrent
  bytes  info = 3;            // 32 B BLAKE3 root of the info dictionary
  uint32 info_size = 4;       // its size in bytes
}
```

- A manifest with `torrent` is a **torrent set**. At least one of `btih` and `btmh` is set.
- **The info dictionary is an ordinary ed3k file.** It is the bencoded `info` value exactly as
  it stands in the torrent. A node that shares a torrent set shares it under `info`, and
  publishes a `SOURCE` record for it. It MUST NOT publish `KEYWORD` records for it.
- A client that has the manifest fetches the info dictionary from ed3k nodes by its root, or
  from BitTorrent peers by the metadata exchange of BitTorrent. Either way it checks the bytes
  against every infohash the manifest has: the SHA-1 against `btih`, the SHA-256 against
  `btmh` (§3).
- `info_size` is at most 32 MiB *(initial)*. A reader does not fetch a larger info dictionary:
  it shows the manifest as a plain set, as after a failed check of §3.
- A client without this extension ignores `torrent` and sees a plain set.

---

## 2. The canonical manifest

Two nodes that complete the same torrent must arrive at the same manifest. The canonical form
leaves nothing to choose.

| Field | Value |
|---|---|
| `title` | the `name` of the info dictionary |
| `files` | the files of the info dictionary, in its order. Version 1 and hybrid: the `files` list, or the one file. Version 2 only: the file tree, in the order of its bencoding |
| `Member.name`, `Member.dir` | the last component of the file's path, and its directory. The `name` of a torrent with several files is not part of the path |
| `Member.root`, `Member.size` | BLAKE3 root and length of the file. A file of length 0 has the root of the empty input |
| `dirs` | every directory of every path, ancestors included, each once; in the order in which `files` first uses them, a parent before its children |
| `torrent` | every infohash the torrent has, `info` and `info_size` |
| Everything else | absent: every other field of `FileSet` and of a `Member`, among them `comment`, `created`, `id`, `seq`, `ed2k`, `links`, inner sets and mounts, `sealed`, `open`, `usenet`, `tube`, key and signature |

- **Padding files** (attribute `p`) are not members. Their bytes are zeros and exist only in
  the piece space of the torrent.
- **Names.** Where the dictionary has a key with the suffix `.utf-8`, that one is used. A name
  that is not valid UTF-8 is read as ISO 8859-1 and written as UTF-8, as [`usenet.md`](usenet.md)
  §2 does; `Member.name` is a protobuf string and has to be UTF-8.
- **Serialisation.** The canonical serialisation of `Protocol.md` §0: fields in ascending field
  number, each once, repeated fields in order, varints in their shortest form, a field at its
  default value left out. The file is the 8 bytes of a manifest followed by a `SetFile` that
  has `body` only.
- **No canonical manifest exists** for a torrent
  - whose info dictionary is not in strict bencoding: every dictionary with its keys in
    ascending order of their bytes, each key once; every integer and every string length in
    its one decimal form, without a leading zero, a `+` or `-0`; nothing after the end of the
    dictionary. A reader does not repair it,
  - that is marked private (§6.3),
  - whose paths or counts break a rule of [`file-set.md`](file-set.md) §2.1 or §2.2,
  - that has a symbolic link (attribute `l`),
  - that is hybrid and whose version 1 list, without padding files, does not name the files
    of the file tree with the same paths and lengths in the same order,
  - whose info dictionary is larger than 32 MiB *(initial)*, the limit of `info_size` (§1),
  - or whose manifest would be larger than 4 MiB.

  Such a torrent has no torrent root and is not joined with ed3k.

A manifest with `torrent` that is not canonical is still a valid torrent set, for example one
that its author signed or gave a comment. It has another root, and the rules of §3 and §4 apply
to it unchanged. A node that makes a manifest for a torrent it downloaded MUST make the
canonical one.

---

## 3. Checking a torrent set

A client that holds a manifest with `torrent` and the info dictionary checks, before it uses
both together:

| Check | Rule |
|---|---|
| Infohash | every infohash that is set MUST match: the SHA-1 of the info dictionary is `btih` when `btih` is set, and its SHA-256 is `btmh` when `btmh` is set. A manifest with neither fails |
| Size | `info_size` is at most 32 MiB *(initial)* |
| Count | the manifest has one member per file of the dictionary that is not a padding file |
| Hybrid | in a hybrid torrent the version 1 list, without padding files, names the files of the file tree with the same paths and lengths in the same order |
| Order, size, path | member *i* has the length and the path of file *i* |
| Not private | the dictionary is not marked private |

- If a check fails, the manifest is a plain set for this client. BitTorrent is not used for it.
- If all hold, every member has a place in the **piece space** of the torrent: in version 1
  the files stand one after the other, padding files included; in version 2 every file starts
  a piece of its own.
- The checks say that the list fits the torrent. They say nothing about the roots. §4 does.

---

## 4. Downloading

A client keeps what it has of a member as byte ranges. The piece bitmap of BitTorrent and the
piece bitmap of ed3k are both views of them (`Protocol.md` §14.4).

### 4.1 Started from a torrent root

The user gave an `ed3k:file` link of a torrent set, or chose it in a search. **BLAKE3 is the
judge.**

1. The client downloads the manifest and verifies it against the root of the link.
2. It fetches the info dictionary and makes the checks of §3.
3. It downloads each member from ed3k sources and from BitTorrent peers at once.
4. Every block is verified against the member's root (`Protocol.md` §7.2), whoever sent it.

**When no ed3k node has a member.** Block hashes come from a node that has the file
(`Protocol.md` §12.4). Without one, the client cannot verify a single block under BLAKE3.

- It then checks pieces under the hashes of the torrent. Such data is **checked, not
  verified**: the client MAY pass it to BitTorrent peers, and MUST NOT show it in a `have`
  bitmap or send it to an ed3k node.
- When the member is complete, the client computes its BLAKE3 root. If it is the root of the
  manifest, the whole member is verified, and the client is the first ed3k source for it.
- If it is not, the manifest and the torrent do not belong together. The client says so, and
  keeps nothing of the member as an ed3k file.

**When no ed3k node has the manifest.** A link with `bt` (§7) names the infohash. The client
downloads the torrent from BitTorrent alone, computes the canonical manifest (§2), and
compares its root with the root of the link. Equal roots verify the whole torrent under
BLAKE3 without a single ed3k source. With unequal roots the link and the torrent do not
belong together: the client says so and keeps the data as checked, not verified (above).

### 4.2 Started from a torrent

The user gave a `.torrent` file or a magnet link without an ed3k link in it. **The hashes of
the torrent are the judge**, and a manifest found in Kad is a claim.

1. The client searches `ALIAS` records for the infohash (§5) once it holds the info
   dictionary and the torrent is not private (§6.3): from a magnet link that is after the
   metadata exchange, not before.
2. For a set it finds, it downloads the manifest and makes the checks of §3. It tries at most
   3 *(initial)* sets per torrent, those named by the most publisher sites first.
3. It adds the ed3k sources of each member. A block from them is verified under the member's
   root as always. It counts as verified for the torrent only when its piece passes under the
   hash of the torrent.
4. **A false claim shows itself.** If a piece fails under the torrent while all its blocks
   passed under the roots of the manifest, the manifest is false for that member. The client
   drops the manifest and the data it took from ed3k sources for it, and ignores the publisher
   of the record from then on.
5. When the torrent is complete, the client computes the canonical manifest itself. That root
   is the torrent root, whatever the records said. It shares the set and publishes the record.

A member that is complete and passes under both hashes is an ordinary shared ed3k file from
that moment, before the rest of the torrent is done.

### 4.3 What ed3k sources add

| | From BitTorrent peers | From ed3k sources |
|---|---|---|
| A single file of a version 1 torrent | the pieces at its two ends reach into the files next to it | the file alone |
| A file that is also in another torrent, or shared on its own | another swarm | the same file, the same sources |
| A corrupt range | found after a piece, up to several MiB; the sender is one of those who sent a part of it | found after 256 KiB; the sender is known |
| Peers behind NAT | as BitTorrent allows | through relays and hole punching (`Protocol.md` §9) |

---

## 5. From an infohash to the torrent root

An `ALIAS` record ([`alias.md`](alias.md)) leads from the infohash to the manifest.

| Field | Value |
|---|---|
| `scheme` | `btih` for the 20 B infohash, `btmh` for the 32 B infohash |
| `id` | the infohash |
| `rel` | `SET` |
| `root`, `size`, `name`, `set_files`, `set_size` | the manifest |

- Target, signature, lifetime and the rules of a storing node are those of
  [`alias.md`](alias.md) §1 and §3. A storing node needs to know nothing about torrents.
- A hybrid torrent has two infohashes and so two targets. The publisher stores under both.

**Publisher**

- A node publishes the record only for the canonical manifest (§2), never for another
  manifest with `torrent`, and only for a torrent set it shares.
- It publishes only when it computed the
  manifest itself from complete data that it checked against the info dictionary (§4.2
  step 5). Verifying the members under BLAKE3 alone is not enough: that proves the manifest,
  not the infohash, and a node that published on that ground would repeat a false claim as
  its own (the publisher rule of [`alias.md`](alias.md)).
- It MUST NOT publish one for a private torrent.

Because the manifest is canonical, all honest publishers of a torrent name the same root, and
that root collects their sites.

**What an attacker can do**

- **Fill the target** with false sets. The order by publisher sites puts the canonical root
  first as soon as a few honest nodes publish it.
- **Make clients fetch manifests.** A fetch is at most 4 MiB, and a client tries 3 sets. A
  manifest that fails §3 costs nothing more.
- **A manifest that fits the torrent but has false roots.** It costs the client the blocks it
  takes from the sources of those roots until the first piece fails (§4.2 step 4). Nothing
  false is kept: the hashes of the torrent decide.

---

## 6. The BitTorrent side

### 6.1 Peers

- A client finds BitTorrent peers by the means of BitTorrent: trackers, its DHT, peer
  exchange. None of that is part of ed3k.
- A node that is reachable over both protocols SHOULD be used over ed3k only.
- A client MAY upload to BitTorrent peers what it has checked under the hashes of the torrent.

### 6.2 Trackers

Tracker addresses are not in the info dictionary, and so not in a torrent set. A client keeps
those of the `.torrent` file or magnet link it started from. A download that started from a
torrent root finds BitTorrent peers through the DHT of BitTorrent.

### 6.3 Private torrents

A torrent whose info dictionary is marked private asks that its data stay with the peers its
tracker names.

- A client MUST NOT search or publish an `ALIAS` record for it, and MUST NOT take ed3k
  sources for it.
- It MUST NOT share the info dictionary on ed3k.
- Whether a user shares the downloaded files on ed3k as ordinary files is the user's choice, as
  for any file.

---

## 7. Links

```
ed3k:file?b3=<root of the manifest>&sz=<size>&dn=<name>[&bt=<infohash, 40 or 64 hex>]
magnet:?xt=urn:btih:<infohash, 40 hex>[&xs=<url-encoded ed3k:file link of the manifest>]
```

- The `ed3k:file` link of the manifest is the link of the torrent, like that of any set.
- `xt=urn:btih:` carries the 40-hex SHA-1 infohash of BitTorrent version 1; the base32 form
  of that hash and `urn:btmh:` are not part of version 1. `bt` is hex as `Protocol.md` §13.2
  fixes: written lower case, read in either case.
- **`bt`** names the infohash of the torrent the manifest describes. It lets a client start
  from BitTorrent before it has the manifest (§4.1). The client MUST check it against the
  manifest once it has that; if they differ, `bt` is ignored.
- `bt` MAY occur twice, once with 40 and once with 64 hex digits: a hybrid torrent has two
  infohashes. Each is checked on its own. Another length, a character outside hex, or two
  values of one length make the link invalid (`Protocol.md` §13.2).
- **`xs`** in a magnet link carries the `ed3k:file` link. A client that finds it starts as in
  §4.1: BLAKE3 is the judge. An `xs` whose `ed3k:file` link is invalid is ignored; the magnet
  is then one without an ed3k link (§4.2). A BitTorrent client without ed3k ignores the
  parameter.
- A client without this extension ignores `bt`.

---

## 8. Who is blamed

| Case | Blamed |
|---|---|
| A block from an ed3k source fails under the member's root | that source (`Protocol.md` §12.3) |
| §4.1: a block from a BitTorrent peer fails under the member's root | that peer, by the rules of the client for BitTorrent |
| §4.1: a complete member passes under the torrent and fails under the manifest | nobody. The link was wrong, and the client tells the user |
| §4.2: a piece fails under the torrent while its blocks pass under the manifest | the publisher of the `ALIAS` record. The ed3k sources are not blamed: they sent the file they were asked for |
| An `ALIAS` record whose manifest fails §3 | the publisher of the record |

---

## 9. What it gives and what it costs

| | |
|---|---|
| Verification | the torrent root commits to every member root and to the infohash. SHA-1 pieces are an early check, never the last word in §4.1 |
| Two versions under one infohash | SHA-1 collisions can be made. Two data sets that pass the same pieces have two torrent roots; a link with a torrent root names one of them |
| Hashing | every member is hashed with BLAKE3 once, which a node does for any file it shares |
| Storage | the manifest, about 80 B per file, and the info dictionary, 20 B per piece |
| Kad | one `ALIAS` record per infohash, next to the `SOURCE` records of the manifest, the info dictionary and the members |
| What a storing node learns | that a node has a torrent, and its infohash: the `ALIAS` record carries `id` in clear ([`alias.md`](alias.md) §1) |
| What BitTorrent peers learn | nothing about ed3k |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **One hash tree over all files in a row**, with a hash of the list next to it (NeoLoader) | one hash for the torrent without a manifest file; the tree lines up with the piece space of a version 1 torrent | every byte is hashed twice, once for the file and once for the whole; the hash exists only when the torrent is complete; names are not covered. [`file-set.md`](file-set.md) §8 left this form out for sets |
| **An alias per file**, like `ed2k` in `FileInfo`: infohash and offset | no manifest; works for a single file | a file of a torrent has no hash of its own in version 1, so the alias cannot be checked without the neighbours. No single root for the torrent |
| **A manifest signed by whoever made it**, no canonical form | simpler to write; the maker is known | every maker gets another root. Sources of the manifest are split, and no root collects the publishers in Kad |
| **The info dictionary inside the manifest** | one file to fetch | the piece hashes of a large torrent are tens of MiB; a manifest is at most 4 MiB |
| **A record kind of its own for torrents** | a storing node could apply rules that fit torrents | every further network would need a kind too, and every `ROUTER` an upgrade ([`alias.md`](alias.md) §7) |
| **The mapping stored whole in the Kad record** | no manifest fetch | a record is a few hundred bytes; a torrent has thousands of files |
| **File requests by infohash and piece on ed3k** | an ed3k node could serve a torrent it has not hashed with BLAKE3 | a second identity on the wire, and data that cannot be verified under the root (`Protocol.md` §7.4) |
| **The SHA-256 file roots of a version 2 torrent as the ed3k identity of a member** | no second hashing for version 2 torrents | another tree, with 16 KiB leaves and another padding rule; version 1 torrents have none |

---

## 11. Open points

- A sign by which two clients that met over BitTorrent learn that both speak ed3k, and
  BitTorrent peers passed on over ed3k. Both are weighed in
  [`docs/torrent-signal.md`](../docs/torrent-signal.md), which also says why this extension
  has no capability bit.
- Test vectors for the canonical manifest of a version 1, a version 2 and a hybrid torrent,
  with a padding file and a file of length 0, are in
  [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- A torrent with more than 65,536 files, or a manifest over 4 MiB. Inner sets would need a
  canonical way of splitting.
- Block hashes for a member that no ed3k node has yet. A URL of the hash tree
  ([`http-source.md`](http-source.md) §3.1) is one way.
- Whether the order of the file tree of a version 2 torrent needs more words than "as it is
  bencoded".
- All numbers are first values.
