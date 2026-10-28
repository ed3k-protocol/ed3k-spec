# Extension: torrents as file sets (`torrent`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §3, [`alias.md`](alias.md) §2, [`Protocol.md`](../Protocol.md) §7.4, §13.2, §14.4. § = this document unless a document is named.
**Capability:** none of its own; a node using it sets `SETS`. Two `ALIAS` schemes, one manifest field, one `ed3k:file` link parameter.
**Required of every `ROUTER`:** nothing beyond the `ALIAS` record of [`alias.md`](alias.md).

Torrent = file list with SHA-1 or SHA-256 piece hashes. Here it gets **one BLAKE3 root**, so a
client can download from BitTorrent peers and ed3k nodes at once and verify every byte under the
ed3k hash.

- Torrent = file set ([`file-set.md`](file-set.md)): manifest listing its files with BLAKE3
  roots, naming the torrent.
- Manifest has a **canonical form** (§2): all holders of the complete torrent compute the same manifest byte
  for byte. Its root = **torrent root**.
- `ALIAS` record in Kad: infohash → torrent root (§5).
- Each torrent file stays an ordinary ed3k file: own root, sources, also outside the torrent.

BitTorrent not carried in ed3k: client speaks BitTorrent with BitTorrent peers, ed3k with ed3k nodes.
Every ed3k request still names a file by root (`Protocol.md` §7.4).

NeoLoader joins BitTorrent and its own network in one download, one hash over the whole torrent.
How, and why ed3k differs: [`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md) §2.1.

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

- Manifest with `torrent` = **torrent set**; at least one of `btih`, `btmh` set.
- **Info dictionary = ordinary ed3k file:** bencoded `info` value exactly as in the torrent.
  Node sharing a torrent set shares it under `info`, publishes a `SOURCE` record; MUST NOT
  publish `KEYWORD` records for it.
- Client with manifest fetches info dictionary from ed3k nodes by root, or BitTorrent peers by
  metadata exchange; either way checks bytes against every manifest infohash: SHA-1 vs `btih`,
  SHA-256 vs `btmh` (§3).
- `info_size` at most 32 MiB *(initial)*. Larger: not fetched, manifest shown as plain set, as after a
  failed §3 check.
- Client w/o this extension ignores `torrent`, sees a plain set.

---

## 2. The canonical manifest

Two nodes completing one torrent must reach one manifest; canonical form leaves no choice.

| Field | Value |
|---|---|
| `title` | `name` of the info dictionary |
| `files` | info dictionary's files, its order. Version 1 and hybrid: `files` list, or the one file. Version 2 only: file tree, in bencoding order |
| `Member.name`, `Member.dir` | file's last path component, and its directory. multi-file torrent's `name` not in the path |
| `Member.root`, `Member.size` | file's BLAKE3 root and length. Length 0 → root of empty input |
| `dirs` | every directory of every path, ancestors included, each once; by first use in `files`, parent before children |
| `torrent` | all the torrent's infohashes, `info`, `info_size` |
| Everything else | absent: all other `FileSet` and `Member` fields, incl. `comment`, `created`, `id`, `seq`, `ed2k`, `links`, inner sets and mounts, `sealed`, `open`, `usenet`, `tube`, key, signature |

- **Padding files** (attribute `p`) are no members: zeros existing only in the piece space.
- **Names.** Where a key with suffix `.utf-8` exists, that one is used. Non-UTF-8 name → read as
  ISO 8859-1, written as UTF-8, as [`usenet.md`](usenet.md) §2; `Member.name` is a protobuf
  string, must be UTF-8.
- **Serialisation.** Canonical, `Protocol.md` §0: fields ascending by number, each once,
  repeated fields in order, shortest varints, default values omitted. File = the 8 bytes of a
  manifest + a `SetFile` with `body` only.
- **No canonical manifest** for a torrent
  - whose info dictionary is not strict bencoding: each dictionary's keys ascending by bytes,
    each once; each integer and string length in its one decimal form, no leading zero, `+` or
    `-0`; nothing after the end. Reader doesn't repair it,
  - marked private (§6.3),
  - whose paths or counts break a rule of [`file-set.md`](file-set.md) §2.1 or §2.2,
  - with a symbolic link (attribute `l`),
  - hybrid, whose version 1 list without padding files doesn't name the file tree's files with
    same paths and lengths in same order,
  - whose info dictionary exceeds 32 MiB *(initial)*, the `info_size` limit (§1),
  - or whose manifest would exceed 4 MiB.

  Such a torrent: no torrent root, not joined with ed3k.

Non-canonical manifest with `torrent` (e.g. author signed it or added a comment): valid torrent
set, other root; §3, §4 apply unchanged. Node making a manifest for a torrent it downloaded
MUST make the canonical one.

---

## 3. Checking a torrent set

Client holding manifest with `torrent` and info dictionary checks, before using both together:

| Check | Rule |
|---|---|
| Infohash | every set infohash MUST match: info dictionary's SHA-1 = `btih` if `btih` set, SHA-256 = `btmh` if `btmh` set. Neither set → fails |
| Size | `info_size` at most 32 MiB *(initial)* |
| Count | one member per non-padding dictionary file |
| Hybrid | hybrid torrent: version 1 list, without padding files, names the file tree's files with same paths and lengths in same order |
| Order, size, path | member *i* has the length and path of file *i* |
| Not private | dictionary not marked private |

- Any failure: plain set for this client; BitTorrent not used for it.
- All hold → each member has a place in the torrent's **piece space**: version 1, files in a
  row, padding files included; version 2, each file starts its own piece.
- Checks show list fits torrent, nothing about roots; §4 does.

---

## 4. Downloading

Client keeps a member's data as byte ranges; BitTorrent and ed3k piece bitmaps both view them
(`Protocol.md` §14.4).

### 4.1 Started from a torrent root

User gave a torrent set's `ed3k:file` link or chose it in a search. **BLAKE3 judges.**

1. Download manifest, verify against link's root.
2. Fetch info dictionary, §3 checks.
3. Download each member from ed3k sources and BitTorrent peers together.
4. Every block verified against member's root (`Protocol.md` §7.2), whoever sent it.

**No ed3k node has a member.** Block hashes come from a node with the file (`Protocol.md`
§12.4); without one, no block verifiable under BLAKE3.

- Client then checks pieces under torrent hashes. Such data is **checked, not verified**: client
  MAY pass it to BitTorrent peers, MUST NOT show it in a `have` bitmap or send it to an ed3k
  node.
- Member complete → compute BLAKE3 root. Equals manifest's → whole member verified, client is
  first ed3k source for it.
- Else manifest and torrent don't match; client says so, keeps nothing of the member as an ed3k
  file.

**No ed3k node has the manifest.** Link with `bt` (§7) names the infohash. Client downloads
from BitTorrent alone, computes the canonical manifest (§2), compares root with the link's.
Equal → whole torrent verified under BLAKE3, no ed3k source. Unequal → link and torrent
don't match: client says so, keeps data as checked, not verified (above).

### 4.2 Started from a torrent

User gave a `.torrent` or a magnet link without ed3k link. **Torrent hashes are the judge**; a
manifest found in Kad is a claim.

1. Search `ALIAS` records for the infohash (§5) once holding the info dictionary, if torrent not
   private (§6.3): for a magnet link after metadata exchange, not before.
2. Per set found: download manifest, §3 checks. At most 3 *(initial)* sets per torrent, most
   publisher sites first.
3. Add each member's ed3k sources; blocks verified under member's root as always, verified for
   the torrent only once their piece passes under the torrent hash.
4. **A false claim shows itself.** Piece fails under torrent while all its blocks passed under
   manifest roots → manifest false for that member. Client drops manifest and data from ed3k
   sources for it, ignores the record's publisher from then on.
5. Torrent complete → client computes canonical manifest itself; that root is the torrent root,
   whatever records said. Shares the set, publishes the record.

Complete member passing both hashes = ordinary shared ed3k file at once, before the rest.

### 4.3 What ed3k sources add

| | From BitTorrent peers | From ed3k sources |
|---|---|---|
| A single file of a version 1 torrent | end pieces reach into neighbouring files | the file alone |
| A file that is also in another torrent, or shared on its own | another swarm | same file, same sources |
| A corrupt range | found after a piece, up to several MiB; sender one of those who sent part | found after 256 KiB; sender known |
| Peers behind NAT | as BitTorrent allows | relays, hole punching (`Protocol.md` §9) |

---

## 5. From an infohash to the torrent root

`ALIAS` record ([`alias.md`](alias.md)): infohash → manifest.

| Field | Value |
|---|---|
| `scheme` | `btih` for the 20 B infohash, `btmh` for the 32 B infohash |
| `id` | the infohash |
| `rel` | `SET` |
| `root`, `size`, `name`, `set_files`, `set_size` | the manifest |

- Target, signature, lifetime, storing-node rules: [`alias.md`](alias.md) §1 and §3; storing
  node needs no torrent knowledge.
- Hybrid: two infohashes → two targets; publisher stores under both.

**Publisher**

- Only for the canonical manifest (§2), never another manifest with `torrent`, and only for a
  torrent set it shares.
- Only after computing the manifest itself from complete data checked against the info
  dictionary (§4.2 step 5). Members verified under BLAKE3 alone don't suffice: proves manifest,
  not infohash; publishing on that ground repeats a false claim as one's own (publisher rule of
  [`alias.md`](alias.md)).
- MUST NOT publish for a private torrent.
- **Downloaded torrent is published,** whether started from `.torrent`, magnet link or torrent
  root: node sharing the set and meeting the rules above publishes the record. Non-private =
  public by nature; no setting holds it back. Sole exception: private flag (§6.3).

Canonical manifest → honest publishers all name one root, collecting their sites.

**What an attacker can do**

- **Fill the target** with false sets. Publisher-site order puts the canonical root first once a
  few honest nodes publish it.
- **Make clients fetch manifests.** Fetch ≤ 4 MiB, client tries 3 sets; manifest failing §3
  costs nothing more.
- **Manifest fitting the torrent, false roots.** Costs blocks taken from those roots' sources
  until the first piece fails (§4.2 step 4). Nothing false kept: torrent hashes decide.

---

## 6. The BitTorrent side

### 6.1 Peers

- BitTorrent peers found by BitTorrent means (trackers, its DHT, peer exchange), none in ed3k.
- Node reachable over both protocols SHOULD be used over ed3k only.
- Client MAY upload to BitTorrent peers what it checked under torrent hashes.

### 6.2 Trackers

Tracker addresses aren't in the info dictionary, so not in a torrent set; client keeps those of
its starting `.torrent` or magnet link. Download from a torrent root finds BitTorrent peers via
BitTorrent's DHT.

### 6.3 Private torrents

Info dictionary marked private → torrent asks that its data stay with the peers its tracker names.

- Client MUST NOT search or publish an `ALIAS` record for it, MUST NOT take ed3k sources for
  it.
- MUST NOT share the info dictionary on ed3k.
- Sharing downloaded files on ed3k as ordinary files: user's choice, as for any file.

---

## 7. Links

```
ed3k:file?b3=<root of the manifest>&sz=<size>&dn=<name>[&bt=<infohash, 40 or 64 hex>]
magnet:?xt=urn:btih:<infohash, 40 hex>[&xs=<url-encoded ed3k:file link of the manifest>]
```

- Manifest's `ed3k:file` link = torrent's link, as for any set.
- `xt=urn:btih:` carries the 40-hex SHA-1 infohash of BitTorrent version 1; its base32 form and
  `urn:btmh:` not part of version 1. `bt` is hex per `Protocol.md` §13.2: written lower case,
  read in either.
- **`bt`**: infohash of the torrent the manifest describes; lets client start from BitTorrent
  before having the manifest (§4.1). Client MUST check it against the manifest once it has
  that; differing → `bt` ignored.
- `bt` MAY occur twice, once with 40 and once with 64 hex digits: hybrid torrent, two
  infohashes, each checked on its own. Other length, non-hex character, or two values of one
  length → link invalid (`Protocol.md` §13.2).
- **`xs`** in a magnet link carries the `ed3k:file` link; client finding it starts as §4.1
  (BLAKE3 judges). `xs` with invalid `ed3k:file` link ignored; magnet then treated as one without ed3k
  link (§4.2). BitTorrent clients without ed3k ignore the parameter.
- Client without this extension ignores `bt`.

---

## 8. Who is blamed

| Case | Blamed |
|---|---|
| A block from an ed3k source fails under the member's root | that source (`Protocol.md` §12.3) |
| §4.1: a block from a BitTorrent peer fails under the member's root | that peer, per client's BitTorrent rules |
| §4.1: a complete member passes under the torrent and fails under the manifest | nobody. Wrong link; client tells user |
| §4.2: a piece fails under the torrent while its blocks pass under the manifest | `ALIAS` record's publisher. Not ed3k sources: they sent the file asked for |
| An `ALIAS` record whose manifest fails §3 | the record's publisher |

---

## 9. What it gives and what it costs

| | |
|---|---|
| Verification | torrent root commits to every member root and the infohash. SHA-1 pieces: early check, never the last word in §4.1 |
| Two versions under one infohash | SHA-1 collisions can be made. Two data sets passing same pieces → two torrent roots; a torrent-root link names one |
| Hashing | each member BLAKE3-hashed once, as any shared file |
| Storage | manifest ~80 B/file, info dictionary 20 B/piece |
| Kad | one `ALIAS` record per infohash, beside `SOURCE` records of manifest, info dictionary, members |
| What a storing node learns | a node has a torrent, and its infohash: `ALIAS` carries `id` in clear ([`alias.md`](alias.md) §1) |
| What BitTorrent peers learn | nothing about ed3k |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **One hash tree over all files in a row**, with a hash of the list next to it (NeoLoader) | one torrent hash, no manifest file; tree lines up with a version 1 piece space | every byte hashed twice (per file, whole); hash exists only once the torrent is complete; names not covered. [`file-set.md`](file-set.md) §8 left this form out for sets |
| **An alias per file**, like `ed2k` in `FileInfo`: infohash and offset | no manifest; works for a single file | version 1 torrent file has no own hash: alias uncheckable without neighbours. No single torrent root |
| **A manifest signed by whoever made it**, no canonical form | simpler; maker known | each maker → another root; manifest sources split, no root collects the publishers in Kad |
| **The info dictionary inside the manifest** | one file to fetch | large torrent's piece hashes: tens of MiB; manifest ≤ 4 MiB |
| **A record kind of its own for torrents** | storing node could apply torrent rules | each further network needs a kind too, every `ROUTER` an upgrade ([`alias.md`](alias.md) §7) |
| **The mapping stored whole in the Kad record** | no manifest fetch | record: a few hundred bytes; torrent: thousands of files |
| **File requests by infohash and piece on ed3k** | ed3k node could serve a torrent not hashed with BLAKE3 | second identity on the wire; data not verifiable under the root (`Protocol.md` §7.4) |
| **The SHA-256 file roots of a version 2 torrent as the ed3k identity of a member** | no second hashing for version 2 | other tree, 16 KiB leaves, other padding rule; none in version 1 torrents |

---

## 11. Open points

- A sign by which two clients that met over BitTorrent learn that both speak ed3k, and BitTorrent peers
  passed on over ed3k. Both weighed in [`docs/torrent-signal.md`](../docs/torrent-signal.md),
  also why this extension has no capability bit.
- Test vectors: canonical manifest of a version 1, a version 2 and a hybrid torrent, with a
  padding file and a file of length 0, in [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- Torrent with more than 65,536 files, or manifest over 4 MiB: inner sets would need canonical
  splitting.
- Block hashes for a member no ed3k node has yet: hash-tree URL
  ([`http-source.md`](http-source.md) §3.1) one way.
- Whether version 2 file-tree order needs more than "as it is bencoded".
- All numbers are first values.
