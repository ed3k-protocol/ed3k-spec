# Extension: Usenet posts as file sets (`usenet`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1 and §3, [`alias.md`](alias.md) §2 and §4, and [`Protocol.md`](../Protocol.md) §7.4 and §14.4. Section numbers (§) refer to this document unless a document is named.
**Capability:** none of its own; a node that uses it sets `SETS`. Two schemes of the `ALIAS` record. One field in a manifest, one file format.
**Required of every `ROUTER`:** nothing beyond the `ALIAS` record of [`alias.md`](alias.md).

A binary post on Usenet is a number of files, each cut into articles of a few hundred
kilobytes. An NZB file lists the articles by their message IDs. This extension gives a post
**one BLAKE3 root**, so that a client can download it from news servers and from ed3k nodes at
once, verify every byte under the ed3k hash, and still have the files when the articles are
gone.

- A post is described by a file set ([`file-set.md`](file-set.md)): a manifest that lists the
  files of the post with their BLAKE3 roots.
- The manifest has a **canonical form** (§3). Everyone who has the complete post computes the
  same manifest, byte for byte. Its root is the **post root**.
- The articles are named in an **article list** (§2), which is an ordinary ed3k file.
- `ALIAS` records lead from an NZB to the post root, and from one file of an NZB to its root
  (§1, §6).

Usenet itself is not carried in ed3k. A client that uses this extension speaks NNTP with the
news servers of its user and ed3k with ed3k nodes. Every ed3k request still names a file by
its root (`Protocol.md` §7.4). The rules of [`alias.md`](alias.md) §5 apply.

---

## 1. Names

An NZB file is XML. Two NZB files for one post, made by two indexers, differ in their text,
in the order of their entries and often in which files they list. The names used here are
computed from the message IDs alone.

```
file id = H("ed3k v1 nzb file", m₁ ‖ 0x00 ‖ m₂ ‖ 0x00 ‖ … ‖ mₙ ‖ 0x00)      32 B
post id = H("ed3k v1 nzb", f₁ ‖ f₂ ‖ … ‖ fₖ)                                32 B
```

| | |
|---|---|
| mᵢ | the message ID of part *i* of the file: the text of the `segment` element, without `<` and `>`, as bytes |
| Order of parts | by the `number` attribute, 1 to n. Where a number occurs twice, the first entry in the NZB counts |
| fⱼ | the file ids of all files of the NZB, in ascending order of their bytes |

- **A file with a gap** in its part numbers has no file id, and an NZB with such a file has no
  post id. Such an NZB is not joined with ed3k; the files of it that are whole still are, one
  by one.
- Subject, poster, date, groups and the sizes an NZB tells are not part of a name. They differ
  from indexer to indexer.

| Scheme of `ALIAS` | `id` | Relation | `root` |
|---|---|---|---|
| `nzb` | the post id | `SET` | the manifest of the post (§3) |
| `nzb-file` | the file id | `SAME` | the file |

Two NZB files that list other subsets of a post have two post ids. The file id of a file is the
same in both when both list the same parts for it, so such a file is still found.

---

## 2. The article list

```
"ed3knzb" ‖ 0x01 ‖ ArticleList
```

The file starts with the 8 bytes `65 64 33 6B 6E 7A 62 01`. The rest is one protobuf message:

```protobuf
message ArticleList {
  repeated Post files = 1;    // in ascending order of their file ids (§1)
}

message Post {
  string name = 1;            // file name, from the articles (below)
  uint64 size = 2;            // size of the decoded file, from the articles
  repeated string ids = 3;    // message IDs of parts 1 to n, without "<" and ">"
}
```

- **`name` and `size` come from the articles,** not from the NZB: the `name` and the `size` of
  the yEnc header line `=ybegin`. All parts of a file carry the same values; a client takes
  them from any part it has.
- A name that is not valid UTF-8 is read as ISO 8859-1 and written as UTF-8.
- **The list does not say which bytes an article holds.** Each article says so itself, in its
  `=ypart` line (`begin`, `end`). A client that wants a range asks for the part the range
  falls in when all parts have the size of the first, and corrects itself from the answer.
- The list is written in the canonical serialisation of `Protocol.md` §0.
- The list is at most 32 MiB *(initial)*. It is shared as an ordinary file with a `SOURCE`
  record, and MUST NOT get `KEYWORD` records.
- Groups are not in the list. An article is asked for by its message ID.

Only yEnc is covered. A post in another encoding has no article list and is not joined.

---

## 3. The post set

```protobuf
// in FileSet (file-set.md §1):
Usenet usenet = 10;

message Usenet {
  bytes  id = 1;              // 32 B: the post id (§1)
  bytes  list = 2;            // 32 B BLAKE3 root of the article list (§2)
  uint32 list_size = 3;       // its size in bytes
}
```

A manifest with `usenet` is a **post set**. Its canonical form:

| Field | Value |
|---|---|
| `files` | the files of the article list, in its order |
| `Member.name`, `Member.size` | `name` and `size` of the entry in the list |
| `Member.root` | BLAKE3 root of the decoded file |
| `Member.dir` | 0: all files at the top level |
| `usenet` | post id, root and size of the article list |
| Everything else | absent: every other field of `FileSet` and of a `Member`, among them `title`, `dirs`, `comment`, `created`, `id`, `seq`, `ed2k`, `links`, inner sets and mounts, `sealed`, `open`, `torrent`, `tube`, key and signature |

- Serialisation as in [`torrent.md`](torrent.md) §2. The file is the 8 bytes of a manifest
  followed by a `SetFile` that has `body` only.
- Every file of the NZB is a member: archive volumes, repair files, text files.
- **No canonical manifest exists** for a post
  - with a file of which the client has no article, so that name and size are unknown,
  - whose names break a rule of [`file-set.md`](file-set.md) §2.1 or §2.2, two equal names
    included,
  - or whose manifest would be larger than 4 MiB.

A manifest with `usenet` that is not canonical is still a valid post set, for example one that
its author gave a title. It has another root. A node that makes a manifest for a post it
downloaded MUST make the canonical one.

**Checking a post set.** A client that holds the manifest and the article list checks, before
it uses both together:

| Check | Rule |
|---|---|
| List | the article list has the root `usenet.list` |
| Post id | the file ids of the list, hashed as in §1, give `usenet.id` |
| Count, order | the manifest has one member per entry of the list, in its order |
| Name, size | member *i* has the name and the size of entry *i* |

If a check fails, the manifest is a plain set for this client and Usenet is not used for it.
The checks say that the list fits the manifest. They say nothing about the roots. §4 does.

---

## 4. Downloading

A client keeps what it has of a member as byte ranges (`Protocol.md` §14.4). A decoded article
is a range of a member, at the place its `=ypart` line says.

**An article is checked, not verified.** yEnc carries a CRC-32 per part. It finds damage and
proves nothing. A decoded article that passes it is *checked*: the client MUST NOT show its
bytes in a `have` bitmap or send them to an ed3k node until they pass under the member's root
([`alias.md`](alias.md) §5 rule 5).

### 4.1 Started from a post root

The user gave an `ed3k:file` link of a post set, or chose it in a search. **BLAKE3 is the
judge.**

1. The client downloads the manifest and the article list from ed3k nodes and makes the
   checks of §3.
2. It downloads each member from ed3k sources and from its news servers at once.
3. Every block is verified against the member's root, whoever sent it. A block made of
   articles that fails is dropped; the articles in it are asked for again from another server,
   or the block is taken from an ed3k source.
4. **An article that is gone is a range to take from somewhere else.** Nothing fails because
   of it.

**When no ed3k node has a member,** block hashes are missing (`Protocol.md` §12.4). The client
then downloads the member from Usenet, repairs it by the repair files of the post where it
has to, and computes the root when the member is complete. If it is the root of the manifest,
the member is verified and the client is the first ed3k source for it. If not, the manifest
and the articles do not belong together; the client says so and keeps nothing of the member
as an ed3k file.

### 4.2 Started from an NZB

The user gave an NZB file. **The articles are what was asked for,** and a root found in Kad is
a claim.

1. The client computes the post id and the file ids (§1) and searches `ALIAS` records of
   `nzb` and of `nzb-file`, in parallel with the download from Usenet.
2. For a post set it finds, it downloads the manifest and the article list and makes the
   checks of §3. It tries at most 3 *(initial)* sets, those named by the most publisher sites
   first. A root found through `nzb-file` is a claim for that one file.
3. It adds the ed3k sources of each member. A block from them is verified under the member's
   root as always.
4. **The claim is tested against articles.** Before the client keeps anything from ed3k
   sources of a member, it fetches at least one article of that member, and it goes on
   comparing as articles arrive. If an article passes its CRC-32 and its block fails under
   the claimed root, the claim is false for that member: the client drops the claim and what
   it took from ed3k sources for it, and ignores the publisher of the record.
5. When the post is complete, the client computes the canonical manifest itself (§3). That
   root is the post root, whatever the records said.

**What a true claim proves.** The bytes are the file with that root, block by block. That the
file is the one the articles hold is proven for the articles that were compared, and for the
whole file where the post has repair files and the file passes under their hashes. A client
SHOULD make that check when repair files exist. Without them, a publisher could name a root
that agrees with the post only in the ranges a client is likely to compare; §9 says what that
costs.

### 4.3 What each side adds

| | From news servers | From ed3k sources |
|---|---|---|
| Speed | as fast as the user's account allows | as peers give |
| Old posts | gone after the retention time of the server, or removed | there as long as one node shares the file |
| A damaged range | found by CRC-32 per article; repaired from repair files, if the post has enough | found per 256 KiB block; fetched again |
| Which range is bad | known after the repair files were read | known at once, from the block hashes |
| Cost | an account | none |

---

## 5. Archives

Most posts hold an archive in volumes, often under names that say nothing, sometimes with a
password.

- The volumes are members of the post set, and files like any other.
- **What the archive unpacks to** is the `PACKED` record of [`hoster.md`](hoster.md) §6,
  unchanged. Its `root` is the manifest of the post set; members that are not volumes of the
  archive are left aside when unpacking. `password` carries the password, where the NZB or
  its source told one.
- A user who looks for a file can so find a post that holds it, and a user who has an NZB
  learns what comes out of it before the download ends.
- Repair files are members too. They are of use to a client that takes bytes from Usenet, and
  none to one that has every block verified under the roots.

---

## 6. Publishing

A node publishes when it has the complete post, every member verified under the root it
computed, and shares the files.

| What | How |
|---|---|
| The members | as files: `SOURCE`, and `KEYWORD` by their names |
| The manifest and the article list | as files: `SOURCE` only |
| `ALIAS` of `nzb` | post id → manifest. Only for the canonical manifest |
| `ALIAS` of `nzb-file` | file id → member, for each member. At most 256 *(initial)* per post; a client that needs more members finds them through the set |
| `MEMBER` | from each member to the post set ([`file-set.md`](file-set.md) §6), so that a download that started from one file finds the article list |
| `PACKED` | where the node unpacked the archive itself |

- **A post that was downloaded is published.** A client that shares the files of a post
  shares the manifest and the article list with them, and publishes the records above.
- **Holding posts back** is a setting, off by default. With it on, the client does not share
  the article list and publishes no `ALIAS` record of `nzb` or `nzb-file`, so that nothing it
  sends leads to the articles. **The files are shared all the same,** each under its root
  with `SOURCE` and `KEYWORD` records, like any file. The client then does not share the post
  set either: a manifest with `usenet` names the article list, and a set that others cannot
  check is of no use. It MAY share a plain set of the same files.
- A member that is complete and verified is an ordinary shared file from that moment, before
  the rest of the post is done. Its `nzb-file` record can be published then.

**A link in an NZB.** An NZB file MAY carry the `ed3k:file` link of the post set in its head:

```xml
<head><meta type="ed3k">ed3k:file?b3=…&amp;sz=…&amp;dn=…</meta></head>
```

A client that finds it starts as in §4.1: BLAKE3 is the judge. Other programs ignore a `meta`
of a type they do not know.

---

## 7. News servers

- Server addresses, account names and passwords belong to the user. They are never sent to a
  peer and never put into a record, and a client MUST NOT write a password to a log or show
  it in an error message.
- A client asks for articles by message ID. Which servers it asks, in which order and with
  how many connections is local policy.
- An article that a server does not have is not a failure of any ed3k node and is reported to
  none.

---

## 8. Who is blamed

| Case | Blamed |
|---|---|
| A block from an ed3k source fails under the member's root | that source (`Protocol.md` §12.3) |
| §4.1: a block made of articles fails under the member's root | nobody in ed3k. The articles are damaged, or they are not the ones of this post |
| §4.1: a complete member from Usenet does not give the root of the manifest | nobody. The link was wrong, and the client tells the user |
| §4.2: an article passes its CRC-32 and its block fails under a claimed root | the publisher of the `ALIAS` record. The ed3k sources are not blamed: they sent the file they were asked for |
| An `ALIAS` record of `nzb` whose manifest fails §3 | the publisher of the record |
| A `PACKED` record whose archive does not give the file | its publisher ([`hoster.md`](hoster.md) §7) |

---

## 9. What it gives and what it costs

| | |
|---|---|
| Verification | the post root commits to every member root and to the article list. CRC-32 is an early check, never the last word in §4.1 |
| Life of a post | the files outlive the articles |
| A false claim in §4.2 | costs the blocks taken from ed3k sources until the first article disagrees. With a root made to agree where a client compares, and no repair files, it costs a file that is not the post. The publisher signed it and is ignored afterwards |
| Hashing | every member is hashed with BLAKE3 once, which a node does for any file it shares |
| Storage | the manifest, about 80 B per file, and the article list, about 45 B per article: 3 MiB for a post of 50 GiB |
| Kad | one `ALIAS` record per post and one per file, next to the `SOURCE` records |
| What a storing node learns | that a node has a post. The target is a hash, so it can test posts it knows |
| What others learn | the message IDs of the post, from the article list. For a post under meaningless names they are all that an index sells. §6 shares them on purpose, unless the user holds posts back |
| What a news server learns | nothing about ed3k |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **The hash of the NZB file as the name** | one hash call; no parsing rules | every indexer writes another NZB for one post, so each would be a name of its own and no root would collect the publishers |
| **The NZB file itself as the article list** | no second format | not canonical: two holders share two files with two roots. It also carries subject, poster and group names that the files do not need |
| **Message IDs inside the manifest** | one file to fetch | 45 B per article: a large post passes the 4 MiB of a manifest |
| **Byte ranges per article in the list** | a range leads to its article without a guess | a node that could not fetch an article does not know its range, and could not write the list. The article says it itself |
| **A record per article**, from a message ID to root and offset | finds a file from a single article | hundreds of thousands of records per post |
| **CRC-32 or the repair files' hashes as a name** (`ALIAS` of `md5`) | reuses [`alias.md`](alias.md) §4.1 | not every post has repair files; where it has, a client can publish that record too |
| **Names from the repair files** in the canonical manifest | real names for posts with meaningless ones | two clients, one with the repair files and one without, would write two manifests |
| **Articles served over ed3k**, by message ID | a node could pass on what it has before the file is whole | a second identity on the wire, and data that cannot be verified under the root (`Protocol.md` §7.4) |

---

## 11. Open points

- Servers that want a group selected before they give an article. The group is not in the
  list; a client has it from the NZB it started from, and not in §4.1.
- A post that is repaired with fewer articles than it has: the file is whole, the list names
  articles nobody can fetch. That is correct and of no use; a mark for dead articles would be
  a local matter.
- Test vectors: file id, post id, article list and canonical manifest for a post of two
  files, one with a name that is not UTF-8.
- Whether part sizes that differ inside one file occur often enough to need more than "ask
  and correct".
- Posting to Usenet from ed3k is not specified.
- All numbers are first values.
