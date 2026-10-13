# Extension: other names of a file (`alias`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §7.4, §10.3 and §10.7. Section numbers (§) refer to this document unless a document is named.
**Capability:** none. Record kind 13, `ALIAS`. One field in `Record`, in `Store` and in `KadAnswer`.
**Required of every `ROUTER`:** storing and returning the record kind `ALIAS` (§3), for every scheme, known or not. The rest is optional.

A file has one identity in ed3k: its BLAKE3 root (`Protocol.md` §7.1). Outside ed3k the same
bytes have other names: an eD2K hash, the infohash of a torrent, a link at a file hoster, a
SHA-1 from a magnet link. This extension is the one way from such a name to the root.

- An **`ALIAS` record** says: *the thing with this name is the file, or the set, with this
  root.* It is signed by its publisher and stored in Kad under a hash of the name.
- The kind of name is a **scheme**, a short string. A storing node needs to know nothing about
  a scheme to store its records. **A network or data source that is joined later needs a
  scheme name and a document. It needs no new record kind and no upgrade of the network.**
- §5 states the rules that every such document follows, so that whatever is joined ends under
  the root.

The extensions for torrents ([`torrent.md`](torrent.md)), for links at file hosters
([`hoster.md`](hoster.md)) and for Usenet posts ([`usenet.md`](usenet.md)) are its users.

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
  string entry = 9;           // PACKED: path of the file inside the archive; absent = the only file
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

- The record is signed by its publisher like a keyword record (`Protocol.md` §10.7). Lifetime
  24 h *(initial)*.
- `scheme` is 1 to 16 bytes of `a`–`z`, `0`–`9` and `-`.
- `entry` is at most 255 bytes, `password` at most 128 bytes.
- The target is a hash of the name, but `scheme` and `id` are fields of the record, so a
  storing node holds every name it stores in clear. Only a scheme whose `id` is itself a hash
  (`url`, [`hoster.md`](hoster.md) §4) keeps the name from it.

---

## 2. Schemes

| Scheme | `id` | Relation | Defined in |
|---|---|---|---|
| `ed2k` | 16 B eD2K hash ‖ file size as 8 B, big endian | `SAME` | `Protocol.md` §7.4, §14.4 |
| `md5` | 16 B MD5 of the file | `SAME` | §4.1 |
| `sha1` | 20 B SHA-1 of the file | `SAME` | §4.1 |
| `sha256` | 32 B SHA-256 of the file | `SAME` | §4.1 |
| `btih` | 20 B infohash of a version 1 or hybrid torrent | `SET` | [`torrent.md`](torrent.md) §5 |
| `btmh` | 32 B infohash of a version 2 or hybrid torrent | `SET` | [`torrent.md`](torrent.md) §5 |
| `url` | 32 B hash of a link in its canonical form | `SAME`, `SET` | [`hoster.md`](hoster.md) §4 |
| `b3` | 32 B BLAKE3 root of a file | `PACKED` | [`hoster.md`](hoster.md) §6 |
| `nzb` | 32 B hash of the message IDs of a Usenet post | `SET` | [`usenet.md`](usenet.md) §1 |
| `nzb-file` | 32 B hash of the message IDs of one file of a post | `SAME` | [`usenet.md`](usenet.md) §1 |

- **A new scheme is registered by a document** that fixes the form of `id`, the relations it
  allows, and how a claim is checked (§5). Until this table lists a name, an implementer who
  needs one SHOULD use a name that starts with `x-`.
- A scheme names **content**, never a place where a node is. A name whose bytes can change
  (a web address that is edited, a feed) is handled by the document of its scheme.

---

## 3. Publishing, storing, searching

**Publisher**

- A node publishes an `ALIAS` record only for a root it shares, and only when it has seen
  itself that the name and the root belong together: it has the complete file under the root
  and computed the name from those bytes, or it verified the same bytes under both
  (`Protocol.md` §14.4). The documents of the schemes say what that means for a set and for an
  archive.
- It publishes at most 4 *(initial)* records per root and scheme.

**Storing node**

| Check | Rule |
|---|---|
| Signature, `expires` | as for a keyword record (`Protocol.md` §10.7) |
| Content | `alias`, nothing else. `root` is 32 B, `id` 1 to 64 B, `scheme` as in §1, `name` at most 255 B of UTF-8 |
| Target | the target of the request is the one computed from `scheme` and `id` |
| Count | one record per publisher, target and root. At most 64 *(initial)* different roots per target; further ones get `FULL` |

- It checks nothing that depends on the scheme, and stores records of schemes it does not
  know.
- It cannot check the claim.
- A search returns up to 16 *(initial)* records, the roots named by the most publisher sites
  first, one record per root. A site is the group of [`ip-diversity.md`](ip-diversity.md) §1.

**Searcher**

- It checks the signature and that `scheme` and `id` give the target it searched.
- **Every record is a claim.** §4 says how each relation is checked.
- It tries at most 3 *(initial)* roots per name, those named by the most publisher sites first.
- A publisher whose claim proved false is ignored from then on.

**What an attacker can do**

- **Fill a target** with 64 false roots. The order by publisher sites puts the root that
  several honest nodes name first, and each false record costs a signature under a node ID
  that clients then ignore.
- **Make a client download a file it did not ask for.** §4 bounds what that costs for each
  relation. Nothing false is kept or passed on under the name.

---

## 4. Checking a claim

**The name the user started from is the judge.** A download that started from an `ed3k:file`
link is judged by its root, and an alias found for it is only a way to more sources. A
download that started from a foreign name is judged by that name, and the root is the claim.

| Relation | The claim is true when | Until then |
|---|---|---|
| `SAME`, name can check parts (`ed2k`) | the same bytes pass under both hashes (`Protocol.md` §14.4) | false as soon as a part fails under the name while its blocks pass under the root |
| `SAME`, name is a hash of the whole file | the complete file gives the name (§4.1) | the file is not shown as complete |
| `SAME` and `SET`, name is a place (`url`) | the bytes fetched from the place pass under the root | the place is a source like any other ([`hoster.md`](hoster.md) §4) |
| `SET` (`btih`, `btmh`) | the manifest fits the torrent, and its data passes under the hashes of the torrent ([`torrent.md`](torrent.md) §3, §4.2) | false as soon as a piece fails under the torrent while its blocks pass under the roots |
| `SET` and `SAME` (`nzb`, `nzb-file`) | the manifest fits the article list, and articles agree with the roots ([`usenet.md`](usenet.md) §3, §4.2) | false as soon as an article passes its own checksum while its block fails under the root |
| `PACKED` | the archive, unpacked, gives the file `id` names ([`hoster.md`](hoster.md) §6) | the archive is a download of its own |

### 4.1 A hash of the whole file

`md5`, `sha1` and `sha256` name a file by a hash that cannot check a part of it. Such names
come from magnet links (`urn:sha1`), from download pages and from file hosters.

- A client that starts from such a name downloads the file under the claimed root. Every block
  is verified under that root as always, so every sender is known.
- When the file is complete, the client computes the hash. If it is the name, the claim is
  true. If not, the client tells the user that it found another file, does not show it as the
  file that was asked for, and ignores the publisher.
- A false claim therefore costs one download of a file that is what its root says. A client
  SHOULD compare `size` with the size it knows for the name before it starts.
- MD5 and SHA-1 are broken for collisions. Two files with one such name can exist; the root
  names one of them.

---

## 5. Rules for joining another network or data source

Every document that joins a network or a source of bytes to ed3k follows these rules. They are
what keeps one identity.

| # | Rule |
|---|---|
| 1 | **One identity on the wire.** Every ed3k message names a file by its root. No request carries a foreign name (`Protocol.md` §7.4) |
| 2 | **No foreign protocol inside ed3k.** A client speaks the other network with that network's peers. ed3k carries at most a description of where bytes are |
| 3 | **A name is found through `ALIAS`.** A scheme, no new record kind |
| 4 | **A pairing is a claim** until the client has checked it itself (§4). Who is blamed for a false one is stated per scheme |
| 5 | **Checked is not verified.** Bytes that passed only a foreign check are not shown in a `have` bitmap and not sent to an ed3k node, until they pass under the root |
| 6 | **One store of bytes.** What a client has of a file is a set of byte ranges. The bitmaps of each network are views of it (`Protocol.md` §14.4) |
| 7 | **Several files are a set.** A thing that holds several files is a manifest ([`file-set.md`](file-set.md)), and each file keeps a root and sources of its own. Where every holder can compute the same manifest, the document fixes a canonical form ([`torrent.md`](torrent.md) §2) |
| 8 | **Another form of the bytes is another file.** An archive, an encrypted or an encoded copy has a root of its own. What it turns into is a relation in an `ALIAS` record (`PACKED`), never a second identity of the file |
| 9 | **A source of bytes is a field.** Where bytes can be fetched is told in the file stream and next to the `SOURCE` record, as HTTP sources are ([`http-source.md`](http-source.md) §2). A new kind of source gets a field of its own and a capability bit, so that a node without it is never handed something it would fetch wrongly |
| 10 | **Nothing is run that a peer sent.** Code that speaks to another network or site comes with the client, never over ed3k |

How networks fit under these rules. The last column says where, or that nothing is specified.

| Network or source | Name | Relation | Bytes come from | Specified |
|---|---|---|---|---|
| eD2K | `ed2k` | `SAME` | eD2K peers; part hashes check 9.28 MB parts | `Protocol.md` §14 |
| BitTorrent | `btih`, `btmh` | `SET` | BitTorrent peers; piece hashes | [`torrent.md`](torrent.md) |
| Web servers, cache servers | none needed: the URL travels with the file | — | HTTP | [`http-source.md`](http-source.md) |
| File hosters | `url`; `b3` for archives | `SAME`, `SET`, `PACKED` | a resolver that turns a link into a byte stream | [`hoster.md`](hoster.md) |
| Magnet links with `urn:sha1`, download pages with a checksum | `sha1`, `md5`, `sha256` | `SAME` | ed3k sources | §4.1 |
| Usenet | `nzb`, `nzb-file`; `b3` for archives | `SET`, `SAME`, `PACKED` | news servers; the checksum of an article is a foreign check (rule 5) | [`usenet.md`](usenet.md) |
| A catalogue on a chain (a video site) | none: the chain names the set, and the set names the chain | — | ed3k sources; the site's server or an IPFS gateway as an HTTP source. A content ID in the link is a place, not a name | [`chaintube.md`](chaintube.md) |
| A network with a tree hash of its own (a content ID over a block graph, a tiger tree) | a scheme for that ID | `SAME` for one file, `SET` for a directory | its peers or gateways; its own hashes check parts, as eD2K part hashes do | no |

---

## 6. What it gives and what it costs

| | |
|---|---|
| A later network | a scheme name. Storing nodes of today store its records |
| Kad | one record kind for all names; one record per publisher, name and root, about 150 B |
| What a storing node can check | that the record sits under the hash of its own name. Nothing about the name itself |
| What a storing node learns | the name, in clear, and that a publisher has a file with it; for `url` only the hash of the link |
| Trust | none added. A record is a signed claim, ordered by how many sites make it |

---

## 7. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A record kind per network** (`TORRENT`, then one for links, …) | each kind has exactly the fields it needs; a storing node can apply checks of that network | every new network is a kind that every `ROUTER` must learn, so joining one is a network upgrade. This was the first form of [`torrent.md`](torrent.md) |
| **A list of typed hashes in `FileInfo`**, in place of the one `ed2k` field | no lookup: the names travel with every file record | goes from the root to the names, not from a name to the root; grows every record; names of a whole-file hash cannot be checked by the receiver |
| **Requests by any hash type** | a node could serve a file it knows only by a foreign name | a second identity on the wire, and data that cannot be verified under the root (`Protocol.md` §7.4) |
| **A scheme number** from a registry | 1 byte | two implementers who add a network at the same time collide; a string does not |
| **The name in clear** as the Kad target | a storing node could check the form of a name | storing nodes read every name; a link at a hoster would be public by being looked up |

---

## 8. Open points

- `FileInfo.ed2k` (`Protocol.md` §7.4) and the scheme `ed2k` say the same thing in two places.
  When the field is dropped, the record remains.
- Whether a search should be able to ask for one scheme's relation only. Today a target holds
  one scheme by construction.
- A record from a root to its names (the reverse direction). `MEMBER` does this for sets
  ([`file-set.md`](file-set.md) §6); for plain names nothing does.
- All numbers are first values.
