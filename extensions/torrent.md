# Extension: torrents as file sets (`torrent`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §3 and §6, and [`Protocol.md`](../Protocol.md) §7.4, §10.3, §13.2 and §14.4. Section numbers (§) refer to this document unless a document is named.
**Capability:** none of its own; a node that uses it sets `SETS`. Record kind 13. One field in a manifest, one parameter in `ed3k:file` links.
**Required of every `ROUTER`:** storing and returning the record kind `TORRENT` (§5). The rest is optional.

A BitTorrent torrent is a list of files with SHA-1 or SHA-256 piece hashes. This extension
gives a torrent **one BLAKE3 root**, so that a client can download it from BitTorrent peers and
from ed3k nodes at once and verify every byte under the ed3k hash.

- A torrent is described by a file set ([`file-set.md`](file-set.md)): a manifest that lists
  the torrent's files with their BLAKE3 roots and names the torrent.
- The manifest has a **canonical form** (§2). Everyone who has the complete torrent computes
  the same manifest, byte for byte. Its root is the **torrent root**.
- A record in Kad leads from the infohash to the torrent root (§5).
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
  bytes  btih2 = 2;           // 32 B: SHA-256 of the info dictionary; a version 2 or hybrid torrent
  bytes  info = 3;            // 32 B BLAKE3 root of the info dictionary
  uint32 info_size = 4;       // its size in bytes
}
```

- A manifest with `torrent` is a **torrent set**. At least one of `btih` and `btih2` is set.
- **The info dictionary is an ordinary ed3k file.** It is the bencoded `info` value exactly as
  it stands in the torrent. A node that shares a torrent set shares it under `info`, and
  publishes a `SOURCE` record for it. It MUST NOT publish `KEYWORD` records for it.
- A client that has the manifest fetches the info dictionary from ed3k nodes by its root, or
  from BitTorrent peers by the metadata exchange of BitTorrent. Either way it checks that the
  SHA-1, or the SHA-256, of the bytes is the infohash in the manifest.
- `info_size` is at most 32 MiB *(initial)*.
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
| `dirs` | each directory once, in the order in which `files` first uses it |
| `torrent` | every infohash the torrent has, `info` and `info_size` |
| Everything else | absent: no `comment`, `created`, `id`, `seq`, `ed2k`, inner sets, key or signature |

- **Padding files** (attribute `p`) are not members. Their bytes are zeros and exist only in
  the piece space of the torrent.
- **Names.** Where the dictionary has a key with the suffix `.utf-8`, that one is used. A name
  that is not valid UTF-8 is not repaired.
- **Serialisation.** Fields are written in ascending field number, each once, repeated fields
  in order, varints in their shortest form, and a field at its default value is left out. The
  file is the 8 bytes of a manifest followed by a `SetFile` that has `body` only.
- **No canonical manifest exists** for a torrent
  - that is marked private (§6.3),
  - whose paths or counts break a rule of [`file-set.md`](file-set.md) §2.1 or §2.2,
  - that has a symbolic link (attribute `l`),
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
| Infohash | the hash of the info dictionary is `btih`, or `btih2` |
| Count | the manifest has one member per file of the dictionary that is not a padding file |
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
BLAKE3 without a single ed3k source.

### 4.2 Started from a torrent

The user gave a `.torrent` file or a magnet link without an ed3k link in it. **The hashes of
the torrent are the judge**, and a manifest found in Kad is a claim.

1. The client searches `TORRENT` records for the infohash (§5), in parallel with the
   BitTorrent download.
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

```protobuf
// in enum Kind (§10.3 of Protocol.md):
TORRENT = 13;
```

```
target = H("ed3k v1 torrent", infohash)[0..16)       infohash: btih (20 B) or btih2 (32 B)
```

- The content is a `SetRef` ([`file-set.md`](file-set.md) §6) that names the manifest. The
  record travels in the fields of a `MEMBER` record: `Record.member`, `Store.members` and
  `KadAnswer.sets`.
- It is signed by its publisher like a keyword record (`Protocol.md` §10.7). Lifetime 24 h
  *(initial)*.
- A hybrid torrent has two infohashes and so two targets. The publisher stores under both.

**Publisher**

- A node publishes the record only for a torrent set it shares, and only when it computed the
  manifest from complete data (§4.2 step 5) or verified every member of it under BLAKE3
  (§4.1).
- It MUST NOT publish one for a private torrent.

**Storing node**

- The rules of a `MEMBER` record: one record per publisher and set, at most 64 *(initial)*
  sets per target, and up to 16 *(initial)* in an answer, the sets named by the most publisher
  sites first.
- It cannot check the claim.

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

- A client MUST NOT search or publish a `TORRENT` record for it, and MUST NOT take ed3k
  sources for it.
- It MUST NOT share the info dictionary on ed3k.
- Whether a user shares the downloaded files on ed3k as ordinary files is the user's choice, as
  for any file.

---

## 7. Links

```
ed3k:file?b3=<root of the manifest>&sz=<size>&dn=<name>[&bt=<infohash, 40 or 64 hex>]
magnet:?xt=urn:btih:<infohash>[&xs=<url-encoded ed3k:file link of the manifest>]
```

- The `ed3k:file` link of the manifest is the link of the torrent, like that of any set.
- **`bt`** names the infohash of the torrent the manifest describes. It lets a client start
  from BitTorrent before it has the manifest (§4.1). The client MUST check it against the
  manifest once it has that; if they differ, `bt` is ignored.
- **`xs`** in a magnet link carries the `ed3k:file` link. A client that finds it starts as in
  §4.1: BLAKE3 is the judge. A BitTorrent client without ed3k ignores the parameter.
- A client without this extension ignores `bt`.

---

## 8. Who is blamed

| Case | Blamed |
|---|---|
| A block from an ed3k source fails under the member's root | that source (`Protocol.md` §12.3) |
| §4.1: a block from a BitTorrent peer fails under the member's root | that peer, by the rules of the client for BitTorrent |
| §4.1: a complete member passes under the torrent and fails under the manifest | nobody. The link was wrong, and the client tells the user |
| §4.2: a piece fails under the torrent while its blocks pass under the manifest | the publisher of the `TORRENT` record. The ed3k sources are not blamed: they sent the file they were asked for |
| A `TORRENT` record whose manifest fails §3 | the publisher of the record |

---

## 9. What it gives and what it costs

| | |
|---|---|
| Verification | the torrent root commits to every member root and to the infohash. SHA-1 pieces are an early check, never the last word in §4.1 |
| Two versions under one infohash | SHA-1 collisions can be made. Two data sets that pass the same pieces have two torrent roots; a link with a torrent root names one of them |
| Hashing | every member is hashed with BLAKE3 once, which a node does for any file it shares |
| Storage | the manifest, about 80 B per file, and the info dictionary, 20 B per piece |
| Kad | one `TORRENT` record per infohash, next to the `SOURCE` records of the manifest, the info dictionary and the members |
| What a storing node learns | that a node has a torrent. The target is a hash of the infohash, so the node can test infohashes it knows |
| What BitTorrent peers learn | nothing about ed3k |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **One hash tree over all files in a row**, with a hash of the list next to it (NeoLoader) | one hash for the torrent without a manifest file; the tree lines up with the piece space of a version 1 torrent | every byte is hashed twice, once for the file and once for the whole; the hash exists only when the torrent is complete; names are not covered. [`file-set.md`](file-set.md) §8 left this form out for sets |
| **An alias per file**, like `ed2k` in `FileInfo`: infohash and offset | no manifest; works for a single file | a file of a torrent has no hash of its own in version 1, so the alias cannot be checked without the neighbours. No single root for the torrent |
| **A manifest signed by whoever made it**, no canonical form | simpler to write; the maker is known | every maker gets another root. Sources of the manifest are split, and no root collects the publishers in Kad |
| **The info dictionary inside the manifest** | one file to fetch | the piece hashes of a large torrent are tens of MiB; a manifest is at most 4 MiB |
| **The mapping stored whole in the Kad record** | no manifest fetch | a record is a few hundred bytes; a torrent has thousands of files |
| **File requests by infohash and piece on ed3k** | an ed3k node could serve a torrent it has not hashed with BLAKE3 | a second identity on the wire, and data that cannot be verified under the root (`Protocol.md` §7.4) |
| **The SHA-256 file roots of a version 2 torrent as the ed3k identity of a member** | no second hashing for version 2 torrents | another tree, with 16 KiB leaves and another padding rule; version 1 torrents have none |

---

## 11. Open points

- A sign by which two clients that met over BitTorrent learn that both speak ed3k, and
  BitTorrent peers passed on over ed3k. Both are weighed in
  [`docs/torrent-signal.md`](../docs/torrent-signal.md), which also says why this extension
  has no capability bit.
- Test vectors for the canonical manifest: a version 1, a version 2 and a hybrid torrent, with
  padding files and a file of length 0.
- A torrent with more than 65,536 files, or a manifest over 4 MiB. Inner sets would need a
  canonical way of splitting.
- Block hashes for a member that no ed3k node has yet. A URL of the hash tree
  ([`http-source.md`](http-source.md) §3.1) is one way.
- Whether the order of the file tree of a version 2 torrent needs more words than "as it is
  bencoded".
- All numbers are first values.
