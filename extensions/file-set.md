# Extension: file sets (`file-set`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §5.4, §10.3, §10.7, §12.1, §13.1 and §13.2. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 10, `SETS`. No message types; one file format, two Kad record kinds and fields in existing messages.
**Required of every `ROUTER`:** storing and returning the record kinds `HEAD` and `MEMBER` (§5.2, §6). The rest is optional.

ed3k shares single files. A folder, an album or a series is a number of unrelated files, and a
downloader has to find each one. This extension adds a **file set**: a list of files with their
paths that is published, found and downloaded as one item.

A set is described by a **manifest**. The manifest is an ordinary file. It is shared, searched,
verified and transferred like any other, and its BLAKE3 root (`Protocol.md` §7.1) is the identity of the set.
Every file it lists stays an ordinary ed3k file with a root of its own.

What this gives:

- **One link for many files.** The `ed3k:file` link of the manifest is the link of the set.
- **Any part of a set.** A downloader takes the members it wants. Nothing spans two files.
- **Shared sources.** A file that is a member of two sets, or is also shared on its own, is one
  file with one set of sources.
- **Sets that change.** An author can publish a new version of a set, and whoever follows the
  set finds it (§5).
- **From a file to its sets.** A record under a file's root names the sets that contain it
  (§6).
- **Works without the capability.** A node that does not know this extension can still store,
  upload and download the manifest and every member. The capability adds the member bitmap
  (§4.1) and the use of the two record kinds (§5, §6). Storing those records is not optional:
  every `ROUTER` of wire version 1 does it, so they are held by as many nodes as any record.

## 1. The manifest

```
"ed3kset" ‖ 0x01 ‖ SetFile
```

The file starts with the 8 bytes `65 64 33 6B 73 65 74 01`. The rest is one protobuf message:

```protobuf
message SetFile {
  bytes  body = 1;            // a serialised FileSet
  bytes  key = 2;             // optional: 32 B Ed25519 key of the author
  repeated Signature sigs = 3;  // optional: over body (§2.4)
  // fields 4 to 6 belong to the optional extension for several authors (multi-author-set.md)
}

message FileSet {
  string title = 1;
  repeated string dirs = 2;   // directory paths, "/" between components
  repeated Member files = 3;
  string comment = 4;
  uint64 created = 5;         // Unix time in seconds; 0 = not told
  bytes  id = 6;              // optional: 1 to 32 B, names a set that changes (§5)
  uint64 seq = 7;             // with `id`: version number, higher = newer
  // field 8 belongs to the optional extension for several authors (multi-author-set.md §3.1)
  // field 9 belongs to the optional torrent extension (torrent.md §1)
  // field 10 belongs to the optional Usenet extension (usenet.md §3)
  // field 11 belongs to the optional extension for videos named on a chain (chaintube.md §2.3)
  // fields 12 to 14 belong to the optional paid subscription extension (subscription.md §8)
}

message Member {
  bytes  root = 1;            // 32 B BLAKE3 root
  uint64 size = 2;
  string name = 3;            // file name, one path component
  uint32 dir = 4;             // 0 = top level of the set; else 1 + index into `dirs`
  bytes  ed2k = 5;            // optional: 16 B eD2K hash (`Protocol.md` §7.4)
  bool   set = 6;             // this member is the manifest of an inner set (§2.5)
  uint32 set_files = 7;       // with `set`: number of files in the inner set, its inner sets included
  uint64 set_size = 8;        // with `set`: sum of their sizes
  // field 9 belongs to the optional hidden set extension (hidden-set.md §4.2)
  // field 10 belongs to the optional extension for several authors (multi-author-set.md §2)
  // field 11 belongs to the optional extension for links at file hosters (hoster.md §5)
  // field 12 belongs to the optional paid subscription extension (subscription.md §8.1)
}
```

`Signature` is the type of `Protocol.md` §10.7.

- The manifest SHOULD be named `<title>.ed3kset`. A client recognises a manifest by its first 8
  bytes, not by its name.
- `dirs` holds each directory once, so a path is not repeated per file. A directory without
  files is listed too and is created by the downloader.
- The order of `files` is fixed by the manifest. Entry numbers in §4 refer to it.
- A member's `size` is a claim, as everywhere in ed3k: the root does not commit to it (`Protocol.md` §7.1).
- The manifest is immutable. A changed set is a new manifest with a new root. §5 says how the
  new one is found from the old one.

## 2. Rules for a reader

### 2.1 Limits

A manifest needs no canonical serialisation (`Protocol.md` §0). A reader accepts any valid
encoding of it.

A reader MUST refuse a manifest that breaks one of these *(initial)*:

| | Limit |
|---|---|
| Size of the manifest | 4 MiB (one piece) |
| Entries in `files` | 65,536 |
| Entries in `dirs` | 65,536 |
| One path component | 1 to 255 bytes in UTF-8 |
| Components in a directory path | 32 |
| `root` | 32 bytes |
| `dir` | 0 to the number of `dirs` |

The limits over several manifests, the depth and the total of files, are in §2.5. They leave
an inner set unopened; no manifest is refused for them.

### 2.2 Paths

A manifest comes from a stranger and names places in the reader's file system.

- A component MUST NOT be `.` or `..`, and MUST NOT contain `/`, `\`, NUL or a code point below
  U+0020. A directory path MUST NOT begin or end with `/` or contain `//`.
- Two members MUST NOT have the same `dir` and `name`. Two entries of `dirs` MUST NOT be equal.
  A member's path, its directory and `name`, MUST NOT be equal to an entry of `dirs`, or a
  prefix of one that ends at a `/` (file `a` with `dirs = ["a/b"]`).
- A reader MUST refuse a manifest that breaks these rules. It MUST NOT repair it.
- A reader writes every member below one directory that the user chose for the set, and MUST
  NOT follow a symbolic link that leaves it.
- Names that the reader's file system cannot hold or would treat as one (reserved names, a
  `:`, a trailing dot or space, names that differ only in case or in Unicode normalisation,
  names that are too long) are changed by the reader. The manifest is not changed.

### 2.3 Starting a download

- A client SHOULD show the title, the number of files and the total size, and let the user
  choose members, before it downloads any of them.
- A client MUST NOT treat a downloaded file as a manifest because of its name alone, and MUST
  NOT start member downloads of a manifest the user did not ask for.
- A member of size 0 is created without a download.
- A member that is a manifest is treated as an inner set only when it is marked `set` (§2.5).
  Without the mark it is downloaded as a plain file.

### 2.4 Author signature

A manifest MAY be signed by its author. Each signature is computed over

```
"ed3k v1 set manifest" ‖ body
```

with the key in `key`. The rules for `sigs` are those of `Protocol.md` §10.7. The author's
node ID is `H("ed3k v1 node id", key)[0..16)` (`Protocol.md` §6.1).

- A manifest is **signed** when `key` is present. It then has exactly one of `sigs` and
  `grant` ([`multi-author-set.md`](multi-author-set.md) §1.2); with both or with neither it
  MUST be refused, and so must one with `cosigs` but no `grant`. A manifest with `sigs`, `grant` or `cosigs` and no `key` MUST be refused.
- A signed manifest with a signature that does not verify MUST be refused.
- The signature says who made the list. It says nothing about the files.
- An unsigned manifest is valid. Most sets need no author.
- A set has one author. [`multi-author-set.md`](multi-author-set.md) adds co-owners, a
  directory handed to another author, and parts that anyone may add under the owner's rules.

### 2.5 Sets inside sets

A member marked `set` is the manifest of an **inner set**. A collection too large for one
manifest is split this way, and an existing set can be made part of a larger one without
copying its list.

- `root` and `size` of the member are those of the inner manifest. Its `name` is the name of
  the directory that receives the inner set's files, at the place `dir` gives. The rules of
  §2.2 apply to it as to a directory: it MUST NOT equal another member's path, and no entry of
  `dirs` may be equal to it or lie below it.
- `set_files` and `set_size` say what the inner set holds, so that a client can show the
  whole tree and its total size from the outer manifest alone. They count the files of all
  levels below, as in `FileInfo` (§3), and are 0 when the author did not tell. They are
  claims for display: what the inner manifest lists decides. A reader MAY leave an inner set
  unopened when its `set_files` would pass the total limit below.
- The reader downloads the inner manifest like any member, verifies it against `root` and
  applies §2.1 to §2.4 to it. An inner manifest that is refused, or a file that is not a
  manifest, fails that member only. The rest of the outer set is not affected.
- The inner manifest is kept by the client next to the outer one. It is not written into the
  directory tree of the set.
- **Depth.** At most 8 *(initial)* manifests lie below each other, counting the outer one. A
  `set` member at the eighth level is not opened.
- **Total.** A reader stops opening inner sets when the files of the outer set and all inner
  sets opened so far exceed 1,000,000 *(initial)*. Every entry of `files` counts, `set`
  members included. One inner manifest may be a member in
  several places; it is counted each time. Without this limit a few small manifests that name
  each other many times would describe more files than any disk holds.
- A path, with the directories of all levels, has at most 32 components. Only directories
  count, counted from the top of the outer set: the `dir` of each `set` member and its
  `name`, and the `dirs` of the inner set; a file name, and the directory the user chose for
  the set, do not. §2.1's limit is per
  manifest; this one is over the levels. An inner set that would exceed it is shown as a plain
  file and not opened; the manifest that names it is not refused for that.
- A manifest cannot contain itself: its root would have to be known before it is written. The
  depth limit bounds a reader in any case.
- **Choice.** The user's choice of an outer set covers the inner sets the user selected in it
  (§2.3). A client MAY fetch inner manifests before the user chooses, to show their content;
  they are small. It MUST NOT fetch their members before.
- **Signature.** A signature on the outer manifest covers the roots of the inner manifests, and
  through them their lists. An inner manifest may carry a signature of its own, by another
  author.
- **Bitmap.** In `members` (§4.1) the bit of a `set` member says that the uploader holds the
  inner manifest. For the members of the inner set the downloader asks on a file stream of the
  inner manifest.

**How to split.** The author chooses.

| Layout | Manifests | An update re-sends |
|---|---|---|
| One flat manifest with `dirs` | 1 | the whole manifest, up to 4 MiB |
| Inner sets for the subtrees that change on their own | a few | the outer manifest and the changed inner ones |
| One manifest per directory, no `dirs`, every subdirectory a `set` member | one per directory | the manifests on the path from the top to the change |

The third layout is the tree of git and of IPFS UnixFS: a directory is a list of names and
hashes, and an unchanged subtree keeps its root from version to version. It costs one fetch
per directory before the whole list is known, and it is limited to 8 levels. The first is the
cheapest for a set that is small or never changes.

## 3. Publishing and finding

- **The manifest** is published like any shared file: a `SOURCE` record under its root and
  `KEYWORD` records under the keywords of its name (`Protocol.md` §10.1). The extension `ed3kset` is the
  extension of the name, so `Query.ext` finds sets.
- **Members** are ordinary shared files of the node. Whether it publishes each one under its
  own root and name is the node's normal policy.
- **`FileInfo`** (`Protocol.md` §5.4) gains two fields for the manifest's record, so that a search result can
  show what the set holds:

  ```protobuf
  // in FileInfo:
  uint32 set_files = 8;       // number of files, inner sets included; 0 = not a set
  uint64 set_size = 9;        // sum of their sizes
  ```

  Both are claims of the publisher. A storing node without this extension keeps them, because a
  record is stored as signed bytes (`Protocol.md` §10.7). `size` stays the size of the manifest, so
  `Query.min_size` and `max_size` filter on that and not on the set.
- **Sources of members.** A node that shares a set usually has its members. A downloader
  SHOULD ask the sources of the manifest for the members first, and look up `SOURCE` records
  for a member only when none of them has it.

## 4. Transfer

A set of a thousand small files would cost a thousand source lookups, file requests and queue
entries if nothing were added. Two rules avoid that. The first needs `SETS` on the uploader; the
second is part of `Protocol.md` §12.5.

### 4.1 Member bitmap

```protobuf
// in FileRequest:
bool  want_members = 8;       // on the stream of a manifest: tell me which members you have

// in FileStatus:
optional bytes members = 10;  // bitmap over FileSet.files, LSB first; bit set = member complete; missing bits are 0; absent = not told
```

- A downloader sets `want_members` in a `FileRequest` for a manifest.
- An uploader that shares the file as a set answers with `members`. An absent field means "not
  told"; the downloader then asks per member.
- The bitmap MAY be shorter than the list: bytes that are missing at its end read as 0, and
  bits after the last member are ignored. An uploader that has no member complete sends the
  one byte `00`, so that its answer is not read as "not told".
- A bit is set only for a member the uploader holds completely. A partial member is found by
  asking for it. A member whose root is that of the empty input is complete without data
  (§2.3, `Protocol.md` §12.4): its bit is always set.
- `members` is sent again only when the downloader asks again: a later `FileRequest` on the
  stream with `want_members` is answered with a `FileStatus` that carries `members` alone,
  without the limit of `Protocol.md` §12.1 for sources. One that also asks for sources gets
  them in the same `FileStatus` within that limit.

For 10,000 members the bitmap is 1.25 KiB.

### 4.2 One turn, several files

`Protocol.md` §12.5 gives a downloader one queue entry per uploader and one file stream at a time. It also
lets a downloader that holds a slot open a file stream for another file and keep the slot, so
the members of a set are fetched one after the other within one turn. That rule holds for
every node and does not need `SETS`.

- The downloader sets `brief`: it has size and name from the manifest.
- A turn stays 16 MiB or 1 h, so a set gives no more than a single file does.

A member of at most one block (256 KiB) needs no hash exchange (`Protocol.md` §7.2): one `FileRequest`, one
`BlockRequest` and the data.

## 5. Sets that change

A manifest never changes, so a set that grows is a row of manifests. A **head** is a signed Kad
record that names the newest one. It does for a set what IPNS does for an IPFS directory.

### 5.1 Naming

A set that changes has an author and an `id`:

- The manifest is signed (§2.4). `key` is the author's key.
- `FileSet.id` is 1 to 32 bytes chosen by the author when the set is created, random or
  readable. It is the same in every version.
- A set that may get co-owners later is created with an `id` of at most 16 bytes: a joint
  head cannot hold a longer one, and the `id` cannot change
  ([`multi-author-set.md`](multi-author-set.md) §1.1, §1.3). In a joint head `Head.name`
  (§5.2) is empty.
- `FileSet.seq` is the version number. Every new version has a higher one.

The pair (`key`, `id`) is the name of the set. Its head is stored at

```
target = H("ed3k v1 set head", key ‖ id)[0..16)
```

Anyone who holds one version of the manifest can compute the target and find the newest.

### 5.2 The record

```protobuf
// in enum Kind (Protocol.md §10.3):
HEAD = 4;

// in Record (Protocol.md §10.7):
Head head = 4;

message Head {
  bytes  id = 1;              // FileSet.id
  uint64 seq = 2;             // FileSet.seq of the manifest named here
  bytes  root = 3;            // 32 B root of that manifest
  uint64 size = 4;            // its size
  string name = 5;            // its file name
}

// in Store (Protocol.md §10.3):
Signed head = 5;              // HEAD; also HIDDEN (hidden-set.md §3) and PART (multi-author-set.md §3.4)
// Store.signer = 6 (Protocol.md §10.3) carries the signing key when it is not the key of the connection

// in KadAnswer (Protocol.md §10.3):
repeated Signed sets = 14;    // answer to a search of kind HEAD, MEMBER, HIDDEN or PART
```

The record is a `Signed` (`Protocol.md` §10.7) of kind `HEAD`, signed by the author's key.

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| Signature | valid under `signer`, or under the connection's key when `signer` is absent | valid under `keys[key]` |
| Content | `head`, or `joint` in a joint head ([`multi-author-set.md`](multi-author-set.md) §1.3), and nothing else; else `REJECTED` | the same |
| Target | `H("ed3k v1 set head", signing key ‖ head.id)[0..16)` is the target of the request; for a joint head see [`multi-author-set.md`](multi-author-set.md) §1.3 | the same, with the key and `id` it searched for |
| `expires` | later than now, at most 7 days *(initial)* plus 1 h ahead | later than now minus 1 h |
| Size | `id` 1 to 32 B, `root` 32 B, `name` at most 255 B of UTF-8; else `REJECTED` | the same |
| `seq` | any; a record that does not replace the one held is answered `OLDER`. A joint head orders by `Grant.number` first ([`multi-author-set.md`](multi-author-set.md) §1.3) | the highest of all answers wins; among equal ones the lower `root` |
| Count | one record per target. It is replaced by a record with a higher `seq`; by one with the same `seq` and a lower `root`; and by one with the same `seq` and `root` and a later `expires`. A valid record that does not replace it is answered `OLDER` (`Protocol.md` §10.3) | |

- **Two heads with one `seq`.** An author who publishes from two machines, or two co-owners,
  can sign two versions with the same `seq`. The one with the lower `root`, compared as a byte
  string, wins on every storing node and for every follower, so the network agrees on one
  instead of keeping whichever arrived first. A client that has seen both MAY tell its user.
  The reasoning is in [`docs/hypercore-syncthing-comparison.md`](../docs/hypercore-syncthing-comparison.md) §2.1.
- **Only the author can make a head**, because the target contains the key.
- **Anyone can store it again.** A node that follows a set SHOULD store the head it holds once
  per 24 h *(initial)* on the closest nodes, with `signer` set. The record then outlives the
  storing nodes that leave, and the author need not be online until the head's `expires`
  (7 days at most), when only the author can sign a new one. `signer` is how a `Store`
  carries a record that the connection's key did not sign (`Protocol.md` §10.7).
- The node that signs a new head, the author or a key of the grant
  ([`multi-author-set.md`](multi-author-set.md)), stores it when it publishes it. That store counts as its store of the
  24 h: the next one follows 24 h later. `Store.signer` is the key the record signature
  verifies under, the author's key or, in a joint head, a key of the grant; it MAY be left out
  when that is the key of the connection.
- **An old head can be shown again** until its `expires`, to a storing node that never saw the
  newer one. A searcher asks R nodes (`Protocol.md` §10.4) and takes the highest `seq`. An author who wants
  old versions to go quickly sets a short `expires` and stores more often.
- A set whose head has expired everywhere is not lost. Every version is still a file with
  sources; only the pointer to the newest is gone until someone stores it again.

### 5.3 Following a set

1. Search `HEAD` at the target. No answer means that the version held is the newest known.
2. For a head with a higher `seq` than the manifest held, or with the same `seq` and a lower
   `root`, download the manifest `root` as any file, from the sources of `root` (§3). How long
   a client tries, and when it tries again, is local policy; until it has the manifest it keeps
   the old version. Where a head or the manifest held has a grant, the
   comparison is in the order of [`multi-author-set.md`](multi-author-set.md) §1.3: by the
   grant's `number` first, then by `seq`, then by the lower `root`.
3. Accept it only when it is signed by the same key and carries the same `id` and the `seq` of
   the head. Otherwise keep the old version. The head itself is not dropped: it was verified,
   and a node with [`pubsub.md`](pubsub.md) still holds it and passes it on (§4.1 there).
4. Compare the two lists by path. A path only in the new list is added, one only in the old
   list is removed, and one whose root changed is changed. A root that the old list holds under
   another path is moved, not removed and added.

- A client checks a set it follows at most once per hour and SHOULD check once per 6 h
  *(initial)*.
- A client with the publish and subscribe extension also receives a new head from other
  followers within seconds ([`pubsub.md`](pubsub.md)). It still checks as above.
- What it does with new members is the user's choice, made per set: ask, or download.
- A client MUST NOT delete or overwrite a file because a newer manifest leaves it out or names
  another root for its path. It keeps the old file until the user decides.
- A member whose root did not change is not downloaded again, wherever it moved.
- Inner sets are followed through the outer one: the new outer manifest names the new inner
  roots. An inner set with an `id` of its own may also be followed alone.

### 5.4 Links

```
ed3k:set?key=<author key, base32 lower case, no padding>&id=<id, base32>[&dn=<url-encoded name>][&path=<url-encoded path>]
ed3k:file?b3=<manifest root>&sz=<size>&dn=<name>[&path=<url-encoded path>]
```

- `ed3k:set` names a set that changes: the client searches the head and takes the newest
  version. It is the link to hand out for a set that will grow.
- `key` and `id` are base32 as `Protocol.md` §13.2 fixes: written lower case without padding,
  read either way.
- An `ed3k:set` link is invalid when `key` is missing or does not decode to 32 bytes, when it
  has neither `id`, `s` nor `paid`, when `id` decodes to 0 or more than 32 bytes, or when
  `key`, `id`, `dn` or `path` is repeated. Here `id` is base32; the hex rule of
  `Protocol.md` §13.2 is for `ed3k:node`.
- `ed3k:file` with the root of a manifest names one version, as before.
- **`path`** selects one member or one directory of the set, with `/` between components, as
  `ipfs://<root>/dir/file` does. It passes through inner sets by their directory names. A
  directory selects everything below it. A `path` that the manifest does not hold selects
  nothing, and the client says so.
- A client without this extension ignores `path` and downloads the manifest as a file. It does
  not know `ed3k:set`.
- An `ed3k:set` link with `s` in place of `id` names a hidden set
  ([`hidden-set.md`](hidden-set.md)): one that only holders of the link can find and read.
  A link with both `id` and `s` is refused.
- An `ed3k:set` link with `paid=1` in place of `id` names a hidden set that only pass holders
  can read ([`subscription.md`](subscription.md) §8.3). A set with an open manifest is paid
  when its manifest carries an offer (§8.2 there), and keeps its ordinary link.
- Unknown parameters of an `ed3k:set` link are ignored. [`chaintube.md`](chaintube.md) §3.1
  adds `b3` and `sz`, which pin one version of the manifest, and `ipfs`, which names a copy in
  IPFS.

## 6. From a member to its sets

A user who has one episode wants the season. A `MEMBER` record under a file's root names a set
that contains the file.

```protobuf
// in enum Kind (Protocol.md §10.3):
MEMBER = 5;

// in Record (Protocol.md §10.7):
SetRef member = 5;

message SetRef {
  bytes  set = 1;             // 32 B root of a manifest that lists the file
  uint64 size = 2;            // size of that manifest
  string name = 3;            // its file name
  uint32 set_files = 4;       // as in FileInfo (§3)
  uint64 set_size = 5;
}

// in Store (Protocol.md §10.3):
repeated Signed members = 7;  // MEMBER
```

The target is `root[0..16)` of the member, as for `SOURCE` and `NOTE`. The record is signed by
its publisher like a keyword record (`Protocol.md` §10.7). Lifetime 24 h *(initial)*.

**Publisher**

- A node publishes a `MEMBER` record only for a file it shares and a set it shares, both
  under the same key ([`hops.md`](hops.md) §1), and only for the manifest that lists the file
  directly, not for outer sets. Sharing is serving: the node serves the file and the manifest
  file; whether it publishes their `SOURCE` and `KEYWORD` records is its normal policy (§3).
- It sends the record to the nodes its `SOURCE` lookup for the file found. No second lookup is
  needed; the target is the same. Each `MEMBER` record goes in a `Store` of kind `MEMBER` at
  its own target, never in `also`, which carries `SOURCE` only. For a file whose source record
  a batch stored (`Protocol.md` §10.4), these are the nodes of that batch that did not answer
  `FAR` for its target.
- It publishes at most 4 *(initial)* sets per file, and SHOULD prefer the newest version of a
  set that changes.

**Storing node**

- Signature and `expires` are checked as for a keyword record (`Protocol.md` §10.7). The
  record holds `member` and nothing else; else `REJECTED`.
- One record per publisher, file and set. At most 4 *(initial)* sets per publisher key and
  16 *(initial)* per publisher site under a target ([`ip-diversity.md`](ip-diversity.md)
  §2.4); a further one is `REJECTED`, checked before the target. A set counts as a file does
  in `Protocol.md` §10.8. At most 64 *(initial)* different sets per target; further ones get
  `FULL`.
- It keeps at most 3 *(initial)* records per set under a target, of different publishers, by
  the rule of `Protocol.md` §10.8. The site of a kept record is the site
  ([`ip-diversity.md`](ip-diversity.md) §1) of the address of the `Store` that last stored it,
  a renewal included.
  - A record of a publisher already kept takes the place of that publisher's record, whatever
    site it comes from; the kept record then has the site of this `Store`.
  - Otherwise a record from a site that holds one of the places takes the place of that
    site's record (of the least fresh one, should a renewal have left two of one site).
  - A record from another site is added while fewer than 3 are kept, else it takes the place
    of the least fresh record kept.
  - A `Store` with `signer` never takes the place of another publisher's record of another
    site; it is answered `REJECTED`. Otherwise the answer is `STORED`.
- The node remembers, per set under a target, the sites (as above: of the address of each
  `Store`) that stored a valid record for it, each until that record would have expired, also
  after the record was displaced. A `Store` with `signer` adds no site, and only stores
  answered `STORED` count. One site counts once; the count saturates at 255 *(initial)*.
  `sites` and the order below use this count.
- `set` is 32 bytes and `name` at most 255 bytes of UTF-8; else `REJECTED`.
- It cannot check the claim: it does not have the manifest.
- A search returns up to 16 *(initial)* records in `sets`, one record per set: the sets named
  by the most publisher sites first, and among equal counts by
  `H("ed3k v1 page", secret ‖ set)` compared as bytes, where `set` is the 32 bytes of
  `SetRef.set` (the root of the manifest) and `secret` that of `Protocol.md` §10.3. Of the
  records kept for a set, the answer carries the one with the lowest
  `H("ed3k v1 page", secret ‖ set ‖ publisher key)`, so that no publisher chooses which record
  is shown, also not by renewing often. A site is the group of
  [`ip-diversity.md`](ip-diversity.md) §1.
- `KadAnswer.sites` (`Protocol.md` §10.3) has one entry per entry of `sets`: the publisher
  sites the node counted for that set, 0 = not counted.
- A search is paged with `offset` and `more` as in `Protocol.md` §10.3, along the order
  above: `offset` counts sets in it.

**Searcher**

- It checks signature and `expires` as for a keyword record (`Protocol.md` §10.7), and that
  the record holds `member` and nothing else. A record that fails is dropped.
- It merges the answers by `set`. The site count of a set is the lower median of the non-zero
  `sites` values reported for it, and 0 when all are 0; among equal counts the lower `set`
  comes first.

- The claim is checked by fetching the manifest and looking for the root. A record whose
  manifest does not list the file is false; the client SHOULD ignore that publisher from then
  on.
- A client SHOULD fetch a manifest for this purpose only when the user asks for the sets of a
  file, and SHOULD start with the set that most sites name.
- From a manifest that has an `id`, the client can go on to the newest version (§5).

**What an attacker can do**

- **Fill the target** with 64 false sets, so that real ones get `FULL`. The order by publisher
  sites puts sets with honest publishers of several sites first, displaced records included, and each false record costs the
  attacker a signature under a node ID that clients then ignore.
- **Make clients fetch manifests** that do not list the file. A fetch is at most 4 MiB and is
  made on the user's request only.
- **Advertise.** A real manifest that does list the file, with a thousand unwanted files next
  to it, is a true record. The client shows what the set holds before anything is downloaded
  (§2.3).

## 7. What it costs

| | Value |
|---|---|
| Manifest size per member | about 50 B plus the name |
| Added per inner set | up to 14 B for count and size |
| Manifest for 1,000 files with 30-byte names | about 80 KiB |
| Largest set in one manifest | 65,536 files, or what fits in 4 MiB |
| Largest set with inner sets | 1,000,000 files |
| Extra messages | none |
| Extra Kad records | those of one more file, the manifest |
| Head record | about 180 B; stored once per 24 h by the author and by each follower |
| Checking a followed set | 1 lookup per 6 h |
| Member record | about 150 B per file and set, once per 24 h, on the connections of the source lookup |
| Per member in a turn | one `FileRequest` / `FileStatus` pair |

- The manifest travels as file data. A sender may compress its blocks (`Protocol.md` §12.3), but a stored
  manifest is as large as it is written. Directory paths are stored once for that reason.
- Members are hashed and verified one by one. There is no hash over the whole set other than
  the manifest's root, which commits to every member root.

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| **One hash tree over all files in a row** (BitTorrent v1) | Pieces span file borders, so one file cannot be verified or shared alone, and the same file in two sets is two different things. BitTorrent needed padding files to work around it |
| **A new record kind in Kad for the list** | The list would live for a record lifetime and be limited to the record size. As a file it has sources as long as someone shares it, and needs no new storage rule |
| **A message that transfers the list between peers** | It would be a second way to move bytes that the file stream already moves, with its own verification |
| **A link that lists all members** | Works for a handful of files. A link of 100 KB is not something users pass around |
| **Compressing the manifest** | Halves a file that is small already, at the cost of a decompression limit in the reader. Left out of the format; `dirs` removes the largest repetition, and block compression (`Protocol.md` §12.3) covers the transfer |
| **Paths as one string per file** | Simpler to read, but repeats the directory for every file |
| **Updates by a note under the old manifest** | Needs no new record kind, but a follower that missed versions walks from manifest to manifest, and only the author could store it |
| **A pointer to the previous version in the manifest** | Gives a history, not a way forward. May be added later; nothing depends on it |
| **One manifest per directory as the only form** | A set with 500 directories would be 500 files to fetch before its list is known. It is one of the layouts an author may choose (§2.5) |
| **Member-to-set claims in `FileStatus` only** | No Kad storage and nothing to fill with false claims, but known only after a source was contacted |
| **Sharing equal data between files, per block** (IPFS) | ed3k verifies a file against one root, and a block is the same in two files only at the same aligned offset. A measurement of IPFS found that fixed chunks of 256 KB, its default and the size of an ed3k block, remove about 4 % of the data in an 18 TB sample; chunks cut by content removed nearly 90 % there, at a cost in speed (§9) |
| **Size as part of the member identity** | Would differ from every other place in ed3k, where the root alone names a file |

## 9. Comparison

| | eMule collection | BitTorrent v1 | BitTorrent v2 | ed3k file set |
|---|---|---|---|---|
| The list is | a file of eD2K links | the torrent's info dictionary | the same, with a file tree | a file of roots and paths |
| Identity of the set | none; the file's own hash | hash of the info dictionary | the same | root of the manifest |
| Hash per member | yes | no; pieces span files | yes, one tree per file | yes, one tree per file |
| A member alone is a shareable file | yes | no | yes, same root in any torrent | yes |
| Directories | no | yes | yes | yes |
| Author signature | optional | no | no | optional |
| How the list is fetched | as a file | from peers by metadata exchange, or a `.torrent` file | the same | as a file |
| Availability of members in one message | no | piece bitmap | piece bitmap | member bitmap (§4.1) |

**With IPFS**

| | IPFS (UnixFS, IPNS) | ed3k file set |
|---|---|---|
| A directory is | a node that links its children by hash | a manifest; flat, or one per directory (§2.5) |
| A change makes | new nodes on the path to the change; a new root | a new manifest, and new outer manifests above it; a new root |
| Unchanged subtrees | shared | shared when they are inner sets |
| Very large directory | split into a hash tree (HAMT) by the software | split into inner sets by the author |
| Same data in two files | shared per chunk | not shared; files are whole |
| Same file in two directories | shared | shared |
| Name that follows changes | IPNS: record signed by a key, with a sequence number | head: the same, with one key for many sets through `id` |
| Who keeps the name alive | the author's node, or anyone who stores the record again | the same (§5.2) |
| Validity of the name record | set by the author, 48 h by default | set by the author, at most 7 days |
| Stored again | every 4 h by default | every 24 h |
| Link to a file inside | `ipfs://<root>/path` | `path` in the link (§5.4) |
| From a file to its directories | none | `MEMBER` record (§6) |
| History | none | none |

- The IPNS defaults are those of the reference implementation: `DefaultRecordLifetime` = 48 h in
  `boxo/ipns/defaults.go`, and `Ipns.RepublishPeriod` = 4 h in the Kubo configuration, both read
  on 2026-10-11.
- The figures on sharing per chunk are from Shi et al., *Centralization in the Decentralized
  Web: Challenges and Opportunities in IPFS Data Management* (WWW 2025), section 6.1. The
  sample is 18.2 TB in 9.1 million files, 1 % of what the 180 largest providers stored. The
  paper's own conclusion is that fixed chunks reach "about zero" at the default size in all
  four of its data sets.

The comparison with NeoShare's multi-file hash is in
[`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md).

## 10. Open points

- Whether a head should be able to name a successor key, for an author whose key is lost or
  taken. Two variants are weighed in
  [`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §4.1.
- Whether the limits on `MEMBER` records (4 sets per file and publisher, 64 per target) hold
  for files that are in very many sets.
- Whether a search should be able to filter on `set_size`.
- The limits of §2.1 are set without measurements.
