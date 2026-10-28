# Extension: file sets (`file-set`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §5.4, §10.3, §10.7, §12.1, §13.1 and §13.2. § = this document unless a document is named.
**Capability:** bit 10, `SETS`. No message types; one file format, two Kad record kinds and fields in existing messages.
**Required of every `ROUTER`:** storing and returning the record kinds `HEAD` and `MEMBER` (§5.2, §6). The rest is optional.

ed3k shares single files; a folder, album or series is many unrelated files, each found
separately. This extension adds a **file set**: list of files with paths, published, found and
downloaded as one item. Described by a **manifest**, an ordinary file (shared, searched,
verified, transferred like any other); its BLAKE3 root (`Protocol.md` §7.1) is the set's
identity. Each listed file stays an ordinary ed3k file with its own root. Gives:

- **One link for many files.** The manifest's `ed3k:file` link is the set's link.
- **Any part of a set.** Downloader takes the members it wants. Nothing spans two files.
- **Shared sources.** File in two sets, or also shared alone = one file, one source set.
- **Sets that change.** Author can publish a new version; followers find it (§5).
- **From a file to its sets.** A record under a file's root names the sets containing it (§6).
- **Works without the capability.** Nodes w/o this extension can still store, upload, download
  the manifest and every member. Capability adds member bitmap (§4.1) and use of the two record
  kinds (§5, §6). Storing those records isn't optional: every `ROUTER` of wire version 1 does
  it, so they're held by as many nodes as any record.

## 1. The manifest

```
"ed3kset" ‖ 0x01 ‖ SetFile
```

File starts with the 8 bytes `65 64 33 6B 73 65 74 01`; the rest is one protobuf message:

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

- Manifest SHOULD be named `<title>.ed3kset`; recognised by its first 8 bytes, not its name.
- `dirs` holds each directory once (no per-file path repeats). Empty directories listed too,
  created by downloader.
- Order of `files` fixed by the manifest; §4's entry numbers refer to it.
- A member's `size` is a claim, as everywhere in ed3k: root doesn't commit to it (`Protocol.md` §7.1).
- Manifest immutable; changed set = new manifest, new root. §5: finding new from old.

## 2. Rules for a reader

### 2.1 Limits

No canonical serialisation needed (`Protocol.md` §0); reader accepts any valid encoding.

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

Limits over several manifests (depth, total files): §2.5. They leave an inner set unopened; no
manifest refused for them.

### 2.2 Paths

A manifest comes from a stranger and names places in the reader's file system.

- A component MUST NOT be `.` or `..`, and MUST NOT contain `/`, `\`, NUL or a code point below
  U+0020. A directory path MUST NOT begin or end with `/` or contain `//`.
- Two members MUST NOT have the same `dir` and `name`. Two entries of `dirs` MUST NOT be equal.
  A member's path (directory and `name`) MUST NOT equal an entry of `dirs`, or a prefix of one
  ending at a `/` (file `a` with `dirs = ["a/b"]`).
- A reader MUST refuse a manifest breaking these rules. It MUST NOT repair it.
- Reader writes every member below one user-chosen directory for the set, and MUST NOT follow a
  symbolic link leaving it.
- Names the reader's file system can't hold or would treat as one (reserved names, `:`, trailing dot or
  space, names differing only in case or Unicode normalisation, too long) are changed by the
  reader, not in the manifest.

### 2.3 Starting a download

- Client SHOULD show title, file count, total size, and let the user choose members, before
  downloading any.
- A client MUST NOT treat a downloaded file as a manifest by its name alone, and MUST NOT start
  member downloads of a manifest the user didn't ask for.
- Member of size 0 is created w/o download.
- A member that is a manifest is treated as inner set only when marked `set` (§2.5); unmarked
  it's downloaded as a plain file.

### 2.4 Author signature

A manifest MAY be signed by its author. Each signature is over

```
"ed3k v1 set manifest" ‖ body
```

with the key in `key`. `sigs` rules: `Protocol.md` §10.7. Author's node ID =
`H("ed3k v1 node id", key)[0..16)` (`Protocol.md` §6.1).

- A manifest is **signed** when `key` is present. It then has exactly one of `sigs` and
  `grant` ([`multi-author-set.md`](multi-author-set.md) §1.2); with both or neither it MUST be
  refused, as must one with `cosigs` but no `grant`. A manifest with `sigs`, `grant` or `cosigs`
  and no `key` MUST be refused.
- A signed manifest with a signature that doesn't verify MUST be refused.
- Signature says who made the list, nothing about the files. Unsigned manifest is valid; most
  sets need no author.
- A set has one author. [`multi-author-set.md`](multi-author-set.md) adds co-owners, a directory
  handed to another author, and parts anyone may add under the owner's rules.

### 2.5 Sets inside sets

Member marked `set` = manifest of an **inner set**: splits a collection too large for one
manifest; lets an existing set join a larger one w/o copying its list.

- Member's `root` and `size` are those of the inner manifest. Its `name` = name of the directory
  receiving the inner set's files, at the place `dir` gives. §2.2 applies to it as to a
  directory: it MUST NOT equal another member's path, and no `dirs` entry may equal it or lie
  below it.
- `set_files`, `set_size`: what the inner set holds, so a client can show whole tree and total
  size from the outer manifest alone. Count files of all levels below, as in `FileInfo` (§3), 0
  when author didn't tell. Display claims: inner manifest's list decides. A reader MAY leave an inner set unopened when its `set_files` would pass the total
  limit below.
- Reader downloads inner manifest like any member, verifies against `root`, applies §2.1 to
  §2.4. Refused inner manifest, or non-manifest file, fails that member only; rest of outer set
  unaffected. Not yet fetched = member still downloading: nothing below opened or counted until
  it is.
- Client keeps the inner manifest next to the outer one, not in the set's directory tree.
- **Depth.** At most 8 *(initial)* manifests below each other, counting the outer one. A `set`
  member at the eighth level isn't opened.
- **Total.** Reader stops opening inner sets when files of outer set and all inner sets opened
  so far exceed 1,000,000 *(initial)*. Every `files` entry counts, `set` members included; an
  inner manifest that is a member in several places counts each time. Otherwise a few small
  manifests naming each other many times would describe more files than any disk holds.
- A path, with directories of all levels, has at most 32 components. Only directories count,
  from the top of the outer set: each `set` member's `dir` and `name`, and the inner set's
  `dirs`; a file name, and the user-chosen directory for the set, don't. §2.1's limit is per
  manifest, this one over levels. Inner set exceeding it shown as plain file, not opened; naming
  manifest not refused for that.
- A manifest can't contain itself (root would have to be known before writing); depth limit
  bounds a reader anyway.
- **Choice.** User's choice of an outer set covers the inner sets selected in it (§2.3). A client
  MAY fetch inner manifests before the user chooses, to show their content; they're small. It
  MUST NOT fetch their members before.
- **Signature.** Outer manifest's signature covers inner manifests' roots, and through them their lists.
  Inner manifest may carry its own signature, by another author.
- **Bitmap.** In `members` (§4.1) a `set` member's bit = uploader holds the inner manifest. For
  the inner set's members, downloader asks on a file stream of the inner manifest.

**How to split.** Author chooses.

| Layout | Manifests | An update re-sends |
|---|---|---|
| One flat manifest with `dirs` | 1 | the whole manifest, up to 4 MiB |
| Inner sets for the subtrees that change on their own | a few | the outer manifest and the changed inner ones |
| One manifest per directory, no `dirs`, every subdirectory a `set` member | one per directory | the manifests on the path from the top to the change |

Third layout = git / IPFS UnixFS tree: directory = list of names and hashes; unchanged subtree
keeps its root across versions. Costs one fetch per directory before the whole list is known;
limited to 8 levels. First layout cheapest for a small or never-changing set.

## 3. Publishing and finding

- **The manifest** published like any shared file: a `SOURCE` record under its root and
  `KEYWORD` records under its name's keywords (`Protocol.md` §10.1). `ed3kset` is the name's
  extension, so `Query.ext` finds sets.
- **Members**: ordinary shared files; publishing each under its own root and name is normal
  node policy.
- **`FileInfo`** (`Protocol.md` §5.4) gains two fields for the manifest's record, so a search
  result can show what the set holds:

  ```protobuf
  // in FileInfo:
  uint32 set_files = 8;       // number of files, inner sets included; 0 = not a set
  uint64 set_size = 9;        // sum of their sizes
  ```

  Publisher claims. Storing node w/o this extension keeps them (record stored as signed bytes,
  `Protocol.md` §10.7). `size` stays the manifest's size, so `Query.min_size` and `max_size`
  filter on that, not the set.
- **Sources of members.** A node sharing a set usually has its members. Downloader SHOULD ask
  manifest's sources for members first, look up a member's `SOURCE` records only when none has
  it.

## 4. Transfer

Without additions, a set of a thousand small files would cost a thousand source lookups, file
requests and queue entries. Two rules avoid that; the first needs `SETS` on the uploader, the
second is part of `Protocol.md` §12.5.

### 4.1 Member bitmap

```protobuf
// in FileRequest:
bool  want_members = 8;       // on the stream of a manifest: tell me which members you have

// in FileStatus:
optional bytes members = 10;  // bitmap over FileSet.files, LSB first; bit set = member complete; missing bits are 0; absent = not told
```

- Downloader sets `want_members` in a `FileRequest` for a manifest.
- Uploader sharing the file as a set answers with `members`. Absent = "not told"; downloader
  asks per member.
- Bitmap MAY be shorter than the list: missing trailing bytes = 0, bits past last member
  ignored. Uploader with no complete member sends one byte `00`, not to be read as "not told".
- Bit set only for a member held completely; partial members found by asking. A member whose root is that of the empty input is complete w/o data (§2.3,
  `Protocol.md` §12.4): its bit is always set.
- `members` re-sent only when the downloader asks again: a later `FileRequest` on the stream
  with `want_members` is answered by a `FileStatus` carrying `members` alone, without
  `Protocol.md` §12.1's limit for sources. One also asking for sources gets them in the same
  `FileStatus` within that limit.

For 10,000 members the bitmap is 1.25 KiB.

### 4.2 One turn, several files

`Protocol.md` §12.5 gives a downloader one queue entry per uploader, one file stream at a time,
and lets a slot holder open a file stream for another file and keep the slot: a set's members
are fetched one after another within one turn. Holds for every node; doesn't need `SETS`.

- Downloader sets `brief`: it has size and name from the manifest.
- A turn stays 16 MiB or 1 h, so a set gets no more than a single file does.

Member of at most one block (256 KiB) needs no hash exchange (`Protocol.md` §7.2): one
`FileRequest`, one `BlockRequest`, the data.

## 5. Sets that change

Manifests never change, so a growing set is a row of manifests. A **head** = signed Kad record
naming the newest; it is for a set what IPNS is for an IPFS directory.

### 5.1 Naming

A changing set has an author and an `id`:

- Manifest is signed (§2.4); `key` is the author's key.
- `FileSet.id`: 1 to 32 bytes, author-chosen at creation, random or readable; same in every
  version.
- A set that may get co-owners later is created with an `id` of at most 16 bytes: a joint head
  can't hold a longer one, and the `id` can't change
  ([`multi-author-set.md`](multi-author-set.md) §1.1, §1.3). In a joint head `Head.name` (§5.2)
  is empty.
- `FileSet.seq` is the version number; every new version has a higher one.

The pair (`key`, `id`) names the set. Its head is stored at

```
target = H("ed3k v1 set head", key ‖ id)[0..16)
```

Anyone holding one manifest version can compute the target, find the newest.

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

Record is a `Signed` (`Protocol.md` §10.7) of kind `HEAD`, signed by the author's key.

| | Storing node, on `Store` | Searcher, on `KadAnswer` |
|---|---|---|
| Signature | valid under `signer`, or the connection's key when `signer` absent | valid under `keys[key]` |
| Content | `head`, or `joint` in a joint head ([`multi-author-set.md`](multi-author-set.md) §1.3), nothing else; else `REJECTED` | the same |
| Target | `H("ed3k v1 set head", signing key ‖ head.id)[0..16)` is the request's target; for a joint head see [`multi-author-set.md`](multi-author-set.md) §1.3 | the same, with the key and `id` searched for |
| `expires` | later than now, at most 7 days *(initial)* plus 1 h ahead | later than now minus 1 h |
| Size | `id` 1 to 32 B, `root` 32 B, `name` at most 255 B of UTF-8; else `REJECTED` | the same |
| `seq` | any; record not replacing the held one → `OLDER`. Joint head orders by `Grant.number` first ([`multi-author-set.md`](multi-author-set.md) §1.3) | highest of all answers wins; tie → lower `root` |
| Count | one record per target. Replaced by: higher `seq`; same `seq`, lower `root`; same `seq` and `root`, later `expires`. Valid record not replacing it → `OLDER` (`Protocol.md` §10.3) | |

- **Two heads with one `seq`.** Author on two machines, or two co-owners, can sign two versions
  with the same `seq`. Lower `root` (byte-string compare) wins on every storing node and for
  every follower: network agrees on one, not whichever arrived first. Client that has seen both
  MAY tell its user. Reasoning in
  [`docs/hypercore-syncthing-comparison.md`](../docs/hypercore-syncthing-comparison.md) §2.1.
- **Only the author can make a head**, since the target contains the key.
- **Anyone can store it again.** A follower SHOULD store the head it holds once per 24 h
  *(initial)* on the closest nodes, with `signer` set. Record then outlives leaving storing nodes;
  author needn't be online until the head's `expires` (7 days at most), when only the author can
  sign a new one. `signer` is how a `Store` carries a record the
  connection's key didn't sign (`Protocol.md` §10.7).
- Node signing a new head (author, or a key of the grant,
  [`multi-author-set.md`](multi-author-set.md)) stores it on publishing; counts as its 24 h
  store, next one 24 h later. `Store.signer` = key the record signature verifies
  under, the author's key or, in a joint head, a key of the grant; MAY be left out when that is
  the connection's key.
- **An old head can be shown again** until its `expires`, to a storing node that never saw the
  newer. Searcher asks R nodes (`Protocol.md` §10.4), takes highest `seq`. Author wanting old
  versions gone fast: short `expires`, store more often.
- Set whose head expired everywhere isn't lost: every version still a file with sources; only
  the pointer to the newest is gone until someone stores it again.

### 5.3 Following a set

1. Search `HEAD` at the target. No answer = version held is newest known.
2. For a head with higher `seq` than the held manifest, or same `seq` and lower `root`, download
   manifest `root` as any file, from `root`'s sources (§3). How long a client tries and when it
   retries is local policy; until it has the manifest it keeps the old version. Where a head or
   the held manifest has a grant, comparison follows
   [`multi-author-set.md`](multi-author-set.md) §1.3: grant's `number` first, then `seq`, then
   lower `root`.
3. Accept it only when signed by the same key with the same `id` and the head's `seq`. Else keep
   the old version. The head itself isn't dropped: it was verified, and a node with
   [`pubsub.md`](pubsub.md) still holds and passes it on (§4.1 there).
4. Compare the two lists by path. Path only in the new list → added; only in the old → removed;
   root changed → changed. A root the old list holds under another path → moved, not removed
   and added.

- Client checks a followed set at most once per hour and SHOULD check once per 6 h
  *(initial)*. With the publish and subscribe extension it also gets new heads from other
  followers within seconds ([`pubsub.md`](pubsub.md)); still checks as above.
- New members: user's choice per set, ask or download.
- A client MUST NOT delete or overwrite a file because a newer manifest leaves it out or names
  another root for its path. It keeps the old file until the user decides.
- Member with unchanged root isn't downloaded again, wherever it moved.
- Inner sets followed via the outer one (new outer manifest names new inner roots); an inner set
  with its own `id` may also be followed alone.

### 5.4 Links

```
ed3k:set?key=<author key, base32 lower case, no padding>&id=<id, base32>[&dn=<url-encoded name>][&path=<url-encoded path>]
ed3k:file?b3=<manifest root>&sz=<size>&dn=<name>[&path=<url-encoded path>]
```

- `ed3k:set` names a changing set: client searches the head, takes newest version. Link to hand
  out for a growing set.
- `key` and `id` are base32 as `Protocol.md` §13.2 fixes: written lower case w/o padding, read
  either way.
- An `ed3k:set` link is invalid when `key` is missing or doesn't decode to 32 bytes, when it has
  neither `id`, `s` nor `paid`, when `id` decodes to 0 or more than 32 bytes, or when `key`,
  `id`, `dn` or `path` is repeated. Here `id` is base32; `Protocol.md` §13.2's hex rule is for
  `ed3k:node`.
- `ed3k:file` with a manifest's root names one version, as before.
- **`path`** selects one member or directory, `/` between components, as
  `ipfs://<root>/dir/file` does; passes through inner sets by directory names. Directory selects
  everything below. `path` not in the manifest selects nothing; client says so.
- Client w/o this extension ignores `path`, downloads the manifest as a file; doesn't know
  `ed3k:set`.
- `ed3k:set` link with `s` in place of `id` names a hidden set ([`hidden-set.md`](hidden-set.md)):
  only link holders can find and read it. Link with both `id` and `s` is refused.
- `ed3k:set` link with `paid=1` in place of `id` names a hidden set only pass holders can read
  ([`subscription.md`](subscription.md) §8.3). A set with an open manifest is paid when its
  manifest carries an offer (§8.2 there), and keeps its ordinary link.
- Unknown `ed3k:set` parameters are ignored. [`chaintube.md`](chaintube.md) §3.1 adds `b3` and
  `sz`, pinning one manifest version, and `ipfs`, naming a copy in IPFS.

## 6. From a member to its sets

User with one episode wants the season: a `MEMBER` record under a file's root names a set
containing it.

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

Target = member's `root[0..16)`, as for `SOURCE` and `NOTE`. Signed by its publisher like a
keyword record (`Protocol.md` §10.7). Lifetime 24 h *(initial)*.

**Publisher**

- Publishes a `MEMBER` record only for a file and a set it shares, both under the same key
  ([`hops.md`](hops.md) §1), only for the manifest listing the file directly, not outer sets.
  Sharing = serving file and manifest file; publishing their `SOURCE` and `KEYWORD` records is
  normal policy (§3).
- Sends the record to nodes its `SOURCE` lookup for the file found; same target, no second
  lookup. Each `MEMBER` record goes in a `Store` of kind `MEMBER` at its own target, never
  in `also`, which carries `SOURCE` only. For a file whose source record a batch stored
  (`Protocol.md` §10.4): the batch's nodes that didn't answer `FAR` for its target.
- At most 4 *(initial)* sets per file; SHOULD prefer the newest version of a changing set.

**Storing node**

- Signature and `expires` checked as for a keyword record (`Protocol.md` §10.7). Record holds
  `member` and nothing else; else `REJECTED`.
- One record per publisher, file and set. ≤ 4 *(initial)* sets per publisher key and 16
  *(initial)* per publisher site under a target ([`ip-diversity.md`](ip-diversity.md) §2.4);
  further one `REJECTED`, checked before the target. A set counts as a file does in
  `Protocol.md` §10.8. ≤ 64 *(initial)* different sets per target; further ones `FULL`.
- Keeps at most 3 *(initial)* records per set under a target, of different publishers, by the
  rule of `Protocol.md` §10.8. Kept record's site = site ([`ip-diversity.md`](ip-diversity.md)
  §1) of the address of the `Store` that last stored it, renewal included.
  - Record of an already-kept publisher takes that publisher's place, whatever its site; kept
    record then has this `Store`'s site.
  - Else record from a site holding a place takes that site's record's place (least fresh one,
    should a renewal have left two of one site).
  - Record from another site: added while fewer than 3 kept, else replaces least fresh kept.
  - A `Store` with `signer` never takes the place of another publisher's record of another
    site; answered `REJECTED`. Otherwise answer is `STORED`.
- Node remembers, per set under a target, sites (as above: of each `Store`'s address) that
  stored a valid record for it, each until that record would have expired, even if displaced. `Store` with `signer` adds no site; only stores answered `STORED` count. One
  site counts once; count saturates at 255 *(initial)*. `sites` and the order below use this
  count.
- `set` is 32 bytes, `name` at most 255 bytes of UTF-8; else `REJECTED`.
- Can't check the claim (no manifest).
- Search returns up to 16 *(initial)* records in `sets`, one per set: sets named by most
  publisher sites first; equal counts by `H("ed3k v1 page", secret ‖ set)` compared as bytes,
  where `set` = the 32 bytes of `SetRef.set` (manifest root) and `secret` that of
  `Protocol.md` §10.3. Of the records kept for a set, answer carries the one with lowest
  `H("ed3k v1 page", secret ‖ set ‖ publisher key)`, so no publisher chooses which record is
  shown, also not by renewing often. A site is the group of
  [`ip-diversity.md`](ip-diversity.md) §1.
- `KadAnswer.sites` (`Protocol.md` §10.3): one entry per `sets` entry, publisher sites counted
  for that set, 0 = not counted.
- Paged with `offset` and `more` as in `Protocol.md` §10.3, along the order above; `offset`
  counts sets.

**Searcher**

- Checks signature and `expires` as for a keyword record (`Protocol.md` §10.7), and that the
  record holds `member` and nothing else. Failing record dropped.
- Merges answers by `set`. A set's site count = lower median of non-zero `sites` values
  reported for it, 0 when all are 0; among equal counts the lower `set` first.

- Claim checked by fetching the manifest, looking for the root. Record whose set root is no valid
  manifest (§2.1 to §2.4), or whose manifest doesn't list the file, is false; client SHOULD
  ignore that publisher from then on.
- Client SHOULD fetch a manifest for this only when the user asks for a file's sets, and SHOULD
  start with the set most sites name.
- From a manifest with an `id`, client can go on to the newest version (§5).

**What an attacker can do**

- **Fill the target** with 64 false sets so real ones get `FULL`. Ordering by publisher sites
  puts sets with honest publishers of several sites first, displaced records included; each
  false record costs a signature under a node ID clients then ignore.
- **Make clients fetch manifests** not listing the file. Fetch ≤ 4 MiB, only on user request.
- **Advertise.** Real manifest listing the file plus a thousand unwanted files is a true record.
  Client shows set content before anything is downloaded (§2.3).

## 7. What it costs

| | Value |
|---|---|
| Manifest size per member | ~50 B plus the name |
| Added per inner set | up to 14 B for count and size |
| Manifest for 1,000 files with 30-byte names | ~80 KiB |
| Largest set in one manifest | 65,536 files, or what fits in 4 MiB |
| Largest set with inner sets | 1,000,000 files |
| Extra messages | none |
| Extra Kad records | those of one more file, the manifest |
| Head record | ~180 B; stored once per 24 h by author and each follower |
| Checking a followed set | 1 lookup per 6 h |
| Member record | ~150 B per file and set, once per 24 h, on the source lookup's connections |
| Per member in a turn | one `FileRequest` / `FileStatus` pair |

- Manifest travels as file data. Sender may compress blocks (`Protocol.md` §12.3), but a stored
  manifest is as large as written; hence paths stored once.
- Members hashed and verified one by one. Only whole-set hash is the manifest's root, which
  commits to every member root.

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| **One hash tree over all files in a row** (BitTorrent v1) | Pieces span file borders: a file can't be verified or shared alone; same file in two sets = two things. BitTorrent needed padding files around it |
| **A new record kind in Kad for the list** | Would live one record lifetime, limited to record size. As a file it has sources while shared, needs no new storage rule |
| **A message that transfers the list between peers** | A second way to move bytes the file stream already moves, with its own verification |
| **A link that lists all members** | Works for a handful of files. A 100 KB link isn't something users pass around |
| **Compressing the manifest** | Halves an already small file, costs a decompression limit in the reader. Left out; `dirs` removes the largest repetition, block compression (`Protocol.md` §12.3) covers transfer |
| **Paths as one string per file** | Simpler to read, but repeats the directory per file |
| **Updates by a note under the old manifest** | No new record kind, but a follower missing versions walks manifest to manifest; only the author could store it |
| **A pointer to the previous version in the manifest** | Gives a history, not a way forward. May be added later; nothing depends on it |
| **One manifest per directory as the only form** | A set with 500 directories = 500 files to fetch before its list is known. One of the layouts an author may choose (§2.5) |
| **Member-to-set claims in `FileStatus` only** | No Kad storage, nothing to fill with false claims; but known only after contacting a source |
| **Sharing equal data between files, per block** (IPFS) | ed3k verifies a file against one root; a block matches in two files only at the same aligned offset. An IPFS measurement: fixed 256 KB chunks (its default, the ed3k block size) remove ~4 % of the data in an 18 TB sample; content-defined chunks nearly 90 % there, at a speed cost (§9) |
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
| How the list is fetched | as a file | peers' metadata exchange, or a `.torrent` file | the same | as a file |
| Availability of members in one message | no | piece bitmap | piece bitmap | member bitmap (§4.1) |

**With IPFS**

| | IPFS (UnixFS, IPNS) | ed3k file set |
|---|---|---|
| A directory is | a node linking its children by hash | a manifest; flat, or one per directory (§2.5) |
| A change makes | new nodes on the path to the change; new root | new manifest, new outer manifests above; new root |
| Unchanged subtrees | shared | shared when they are inner sets |
| Very large directory | split into hash tree (HAMT) by software | split into inner sets by author |
| Same data in two files | shared per chunk | not shared; files are whole |
| Same file in two directories | shared | shared |
| Name that follows changes | IPNS: record signed by a key, with a sequence number | head: the same, one key for many sets via `id` |
| Who keeps the name alive | author's node, or anyone storing the record again | the same (§5.2) |
| Validity of the name record | set by the author, 48 h by default | set by the author, at most 7 days |
| Stored again | every 4 h by default | every 24 h |
| Link to a file inside | `ipfs://<root>/path` | `path` in the link (§5.4) |
| From a file to its directories | none | `MEMBER` record (§6) |
| History | none | none |

- IPNS defaults are the reference implementation's: `DefaultRecordLifetime` = 48 h in
  `boxo/ipns/defaults.go`, `Ipns.RepublishPeriod` = 4 h in the Kubo configuration, both read on
  2026-10-11.
- Per-chunk sharing figures from Shi et al., *Centralization in the Decentralized Web:
  Challenges and Opportunities in IPFS Data Management* (WWW 2025), section 6.1. Sample: 18.2 TB
  in 9.1 million files, 1 % of what the 180 largest providers stored. The paper concludes fixed
  chunks reach "about zero" at the default size in all four of its data sets.

Comparison with NeoShare's multi-file hash:
[`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md).

## 10. Open points

- Should a head be able to name a successor key, for an author whose key is lost or taken? Two
  variants weighed in [`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §4.1.
- Do the `MEMBER` record limits (4 sets per file and publisher, 64 per target) hold for files in
  very many sets?
- Should a search be able to filter on `set_size`?
- §2.1's limits are set without measurements.
