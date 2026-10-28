# Extension: Usenet posts as file sets (`usenet`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §3; [`alias.md`](alias.md) §2, §4, and [`Protocol.md`](../Protocol.md) §7.4, §14.4. § = this document unless a document is named.
**Capability:** none of its own; a node using it sets `SETS`. Two `ALIAS` schemes, one manifest field, one file format.
**Required of every `ROUTER`:** nothing beyond the `ALIAS` record of [`alias.md`](alias.md).

Usenet binary post = several files, each cut into articles of a few hundred KiB; an NZB lists
articles by message ID. Here a post gets **one BLAKE3 root**: client downloads from news
servers and ed3k nodes at once, verifies every byte under the ed3k hash, keeps the files after
the articles are gone.

- Post = file set ([`file-set.md`](file-set.md)): manifest of its files with BLAKE3 roots.
- Manifest has a **canonical form** (§3): everyone with the complete post gets it byte for
  byte; its root = **post root**.
- Articles are named in an **article list** (§2), an ordinary ed3k file.
- `ALIAS` records lead NZB → post root, and one file of an NZB → its root (§1, §6).

Usenet not carried in ed3k: client speaks NNTP with its user's news servers, ed3k with nodes. Every ed3k request still names a file by root (`Protocol.md` §7.4).
[`alias.md`](alias.md) §5 rules apply.

---

## 1. Names

NZB is XML; two indexers' NZBs for one post differ in text, entry order, often the files listed.
Names here come from message IDs alone.

```
file id = H("ed3k v1 nzb file", m₁ ‖ 0x00 ‖ m₂ ‖ 0x00 ‖ … ‖ mₙ ‖ 0x00)      32 B
post id = H("ed3k v1 nzb", f₁ ‖ f₂ ‖ … ‖ fₖ)                                32 B
```

| | |
|---|---|
| mᵢ | message ID of part *i*: character data of the `segment` element after XML decoding (entities, character references resolved; CDATA = character data; comments, processing instructions skipped; a `segment` with a child element gives no mᵢ, like an empty one), leading/trailing whitespace stripped, then one enclosing `<`/`>` pair if both present, as UTF-8 bytes. Whitespace here = space (0x20), tab (0x09), CR (0x0D), LF (0x0A), as XML elements may hold line breaks; in §2 = space and tab |
| Order of parts | by `number` attribute, 1 to n, n = count of the file's distinct `number` values. Duplicate number: first NZB entry counts |
| fⱼ | distinct file ids of the NZB's files, ascending by bytes; a file whose file id equals another's counts once: same articles → one file, one `name` and `size` (§2) |

- **File with a gap** in part numbers: no file id; its NZB no post id. Part 0 is a gap, as is a
  file without parts: parts are 1 to n, each present. A `number` outside 1..n or
  not decimal → no file id; same for an empty or missing mᵢ. Such an NZB is not joined with
  ed3k; its whole files still are, one by one.
- Subject, poster, date, groups, NZB-stated sizes: not part of a name; differ per indexer.

| Scheme of `ALIAS` | `id` | Relation | `root` |
|---|---|---|---|
| `nzb` | the post id | `SET` | the manifest of the post (§3) |
| `nzb-file` | the file id | `SAME` | the file |

NZBs listing different subsets of a post → different post ids; a file's file id is equal in
both if both list the same parts for it, so it is still found.

---

## 2. The article list

```
"ed3knzb" ‖ 0x01 ‖ ArticleList
```

File starts with the 8 bytes `65 64 33 6B 6E 7A 62 01`; rest is one protobuf message:

```protobuf
message ArticleList {
  repeated Post files = 1;    // in ascending order of their file ids (§1); equal file ids once
}

message Post {
  string name = 1;            // file name, from the articles (below)
  uint64 size = 2;            // size of the decoded file, from the articles
  repeated string ids = 3;    // message IDs of parts 1 to n, without "<" and ">"
}
```

- **`name`, `size` from the articles,** not the NZB: those of the yEnc header line `=ybegin`, trailing
  whitespace (space, tab) removed. `name` = text after `name=` to end of line or first `=ypart `
  on it, as is (quotes are part of the name); text from such a `=ypart ` on is that article's
  `=ypart` line. All parts carry the same values; client takes them from its lowest-numbered
  part that passed CRC-32 (§4). If a file's passed parts
  disagree (also when the NZB lists that file twice, §1): no manifest and no `ALIAS` record of
  `nzb` for the post. `nzb-file` records do not depend on names; published as in §6.
- Name not valid UTF-8 → read as ISO 8859-1, written as UTF-8.
- **Message ID not valid UTF-8** can't go into `ids`; post has no article list, no manifest
  (§3).
- **List doesn't say which bytes an article holds;** its `=ypart` line does (`begin`, `end`).
  For a range, client asks the part it falls in assuming all parts are the first's size, and
  corrects from the answer.
- List: canonical serialisation of `Protocol.md` §0.
- At most 32 MiB *(initial)*. Shared as an ordinary file with a `SOURCE` record; MUST NOT get
  `KEYWORD` records.
- No groups in the list; articles asked by message ID.

Only yEnc covered; other encodings: no article list, not joined.

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

Manifest with `usenet` = **post set**. Canonical form:

| Field | Value |
|---|---|
| `files` | files of the article list, in its order |
| `Member.name`, `Member.size` | `name`, `size` of the list entry |
| `Member.root` | BLAKE3 root of the decoded file |
| `Member.dir` | 0: all at top level |
| `usenet` | post id, root and size of the article list |
| Everything else | absent: every other field of `FileSet` and `Member`, incl. `title`, `dirs`, `comment`, `created`, `id`, `seq`, `ed2k`, `links`, inner sets and mounts, `sealed`, `open`, `torrent`, `tube`, key, signature |

- Serialisation as [`torrent.md`](torrent.md) §2: the 8 bytes of a manifest + a `SetFile` with
  `body` only.
- Every NZB file is a member: archive volumes, repair, text files.
- **No canonical manifest** for a post
  - with a file lacking any article that passed CRC-32, so name, size unknown; a file rebuilt
    from repair files alone has none (§10),
  - with a message ID not valid UTF-8 (§2),
  - whose article list would exceed 32 MiB *(initial)* (`list_size` limit),
  - whose names break a rule of [`file-set.md`](file-set.md) §2.1 or §2.2, two equal names
    included,
  - or whose manifest would exceed 4 MiB.

Non-canonical `usenet` manifest (e.g. author added a title): still a valid post set, other
root. A node making a manifest for a post it downloaded MUST make the canonical one.

**Checking a post set,** before using manifest and article list together:

| Check | Rule |
|---|---|
| List | article list has root `usenet.list`. `list_size` above 32 MiB *(initial)* fails: list not fetched |
| Post id | list's file ids, hashed as §1, give `usenet.id` |
| Count, order | one member per list entry, same order |
| Name, size | member *i* has name and size of entry *i* |

Any failure: plain set for this client, Usenet not used. Checks show list fits manifest,
nothing about roots; §4 does.

---

## 4. Downloading

Member held as byte ranges (`Protocol.md` §14.4); decoded article = range at the place its
`=ypart` line says.

**Article is checked, not verified.** yEnc CRC-32 per part finds damage, proves nothing. Article passes
only if its decoded length also = `=yend size` and, with a `=ypart` line, `end` − `begin` +
1; else fails like a failed CRC-32. Also failed: `=yend` without `size`; a `=ypart` line
(glued, §2, or own) without decimal `begin`, `end` with 1 ≤ `begin` ≤ `end`. Passing decoded
article is *checked*: client MUST NOT show its bytes in a `have` bitmap or send them to an ed3k
node until they pass under the member's root ([`alias.md`](alias.md) §5 rule 5).

### 4.1 Started from a post root

User gave a post set's `ed3k:file` link, or chose it in a search. **BLAKE3 is the judge.**

1. Download manifest and article list from ed3k nodes; checks of §3.
2. Download each member from ed3k sources and news servers at once.
3. Every block verified against member's root, whoever sent it. Failing block made of articles
   dropped; articles re-asked from another server, or block taken from an ed3k source.
4. **Gone article = range taken elsewhere.** Nothing fails because of it.

**No ed3k node has a member:** no block hashes (`Protocol.md` §12.4). Download from Usenet,
repair with the post's repair files where needed, compute root when complete. Root = manifest's root
→ verified, client is first ed3k source. Else manifest and articles don't belong together;
client says so, keeps nothing of the member as an ed3k file.

### 4.2 Started from an NZB

User gave an NZB. **Articles are what was asked for;** a Kad root is a claim.

1. Compute post id, file ids (§1); search `ALIAS` `nzb` in parallel with the Usenet download. Search `nzb-file` only if `nzb` gave no post set passing §3, then for at most 256
   *(initial)* NZB files.
2. Per post set found: fetch manifest, article list; §3 checks. `usenet.id` ≠ searched post id
   → refused, publisher blamed (§8). At most 3 *(initial)* sets tried, most publisher sites
   first. A root via `nzb-file` is a claim for that one file.
3. Add each member's ed3k sources; blocks verified under member's root as always.
4. **Claim tested against articles.** Before keeping anything from a member's ed3k sources,
   client fetches ≥ 1 of its articles, or, if none fetchable, the complete member passes under
   the repair files' hashes; keeps comparing as articles arrive. If an article passes CRC-32 but its
   block fails under the claimed root, or its `=ybegin` `size` ≠ member's → claim false for that
   member: drop it and what came from ed3k sources for it; ignore publishers of records naming
   that root.
5. Post complete → client computes canonical manifest itself (§3); its root = post root,
   whatever records said. Differs from fetched set → nobody blamed: names are
   not part of the claim, and clients comparing other parts of a post whose parts disagree can
   see other names (§2). Client publishes for its own manifest per §6; MAY keep sharing the
   fetched manifest and article list as files, `SOURCE` only, no `ALIAS` or `MEMBER` record.

**What a true claim proves.** Bytes = file with that root, per block. That it is the
articles' file: proven for compared articles, and for the whole file where the post has repair
files and it passes under their hashes. Client SHOULD check this when repair
files exist. Without them a publisher could name a root agreeing with the post only
where clients are likely to compare; §9 has the cost.

### 4.3 What each side adds

| | From news servers | From ed3k sources |
|---|---|---|
| Speed | as fast as the user's account allows | as peers give |
| Old posts | gone after server retention, or removed | there while one node shares the file |
| A damaged range | found by CRC-32 per article; repaired from repair files, if enough | found per 256 KiB block; fetched again |
| Which range is bad | known after the repair files were read | known at once, from block hashes |
| Cost | an account | none |

---

## 5. Archives

Most posts hold an archive in volumes, often meaninglessly named, sometimes passworded.

- Volumes = post set members, files like any other.
- **What the archive unpacks to** = `PACKED` record of [`hoster.md`](hoster.md) §6, unchanged.
  Its `root` = post set's manifest; non-volume members left aside when unpacking. `password`
  carries the password, where the NZB or its source told one.
- So a file search finds a post holding it; NZB holder learns the output before download ends.
- Repair files are members too: useful when taking bytes from Usenet, useless with every block
  verified under roots.

---

## 6. Publishing

Node publishes once it has the complete post and shares the files. Complete = every member incl.
repair files: client that fetched repair volumes only where a repair needed them makes no
manifest, article list or `nzb` record until it has the rest. It MAY fetch them afterwards to
publish, ed3k sources first; whether it does is local policy. Members and their `nzb-file` records published
meanwhile. Further condition, as [`torrent.md`](torrent.md) §5: `ALIAS` rows and `PACKED`
published only by a node that decoded the articles itself and computed every member's root from them,
no byte from ed3k sources (bytes
repaired by the post's repair files count as from the articles). Members only verified under
given roots prove the manifest, not the post; such a node (post via ed3k) still publishes the
other rows.

| What | How | Who |
|---|---|---|
| The members | as files: `SOURCE`, and `KEYWORD` by their names | every node sharing them |
| The manifest and the article list | as files: `SOURCE` only | every node sharing them |
| `ALIAS` of `nzb` | post id → manifest. Canonical manifest only | only a node that decoded the articles itself |
| `ALIAS` of `nzb-file` | file id → member, per member. At most 256 *(initial)* per post; a client needing more finds them through the set | only a node that decoded that member's articles itself, ≥ 1 passing CRC-32: a member rebuilt from repair files alone has nothing tying file id to bytes |
| `MEMBER` | each member → post set ([`file-set.md`](file-set.md) §6): a download started from one file finds the article list | every node sharing both |
| `PACKED` | where the node unpacked the archive itself | only a node that decoded the articles itself |

- **Downloaded post is published:** sharing a post's files = sharing manifest and article list
  too, and publishing the records above.
- **Holding posts back:** setting, default off. On → no article list shared, no `ALIAS` of
  `nzb` or `nzb-file` published, so nothing sent leads to the articles.
  **Files shared all the same,** each under its root with `SOURCE` and `KEYWORD`, like any file.
  Post set not shared either: a `usenet` manifest names the article list; a set others can't
  check is useless. It MAY share a plain set of the same files.
- Complete, verified member = ordinary shared file at once, before the rest; its `nzb-file`
  record can be published then.

**Link in an NZB.** An NZB MAY carry the post set's `ed3k:file` link in its head:

```xml
<head><meta type="ed3k">ed3k:file?b3=…&amp;sz=…&amp;dn=…</meta></head>
```

Client finding it starts as §4.1 (BLAKE3 judges). Other programs ignore unknown `meta` types.

---

## 7. News servers

- Server addresses, account names, passwords are the user's: never sent to a peer or put in a
  record; a client MUST NOT write a password to a log or show it in an error message.
- Articles asked by message ID; servers, order, connection count: local policy.
- An article a server lacks is no failure of any ed3k node, reported to none.

---

## 8. Who is blamed

| Case | Blamed |
|---|---|
| A block from an ed3k source fails under the member's root | that source (`Protocol.md` §12.3) |
| §4.1: a block made of articles fails under the member's root | nobody in ed3k. Articles damaged, or not this post's |
| §4.1: a complete member from Usenet does not give the manifest's root | nobody. Link wrong; client tells the user |
| §4.2: an article passes its CRC-32 and its block fails under a claimed root, or its `=ybegin` gives another `size` | publishers of the `ALIAS` records naming that root. Not ed3k sources: they sent the file asked for |
| An `ALIAS` record of `nzb` whose manifest fails §3 or names another post id | the record's publisher |
| A `PACKED` record whose archive does not give the file | its publisher ([`hoster.md`](hoster.md) §7) |

---

## 9. What it gives and what it costs

| | |
|---|---|
| Verification | post root commits to every member root and the article list. CRC-32: early check, never the last word in §4.1 |
| Life of a post | files outlive articles |
| A false claim in §4.2 | blocks from ed3k sources until the first article disagrees. Root made to agree where clients compare, no repair files: a file that is not the post. Publisher signed it, ignored afterwards |
| Hashing | each member BLAKE3-hashed once, as any shared file |
| Storage | manifest ~80 B per file, article list ~45 B per article: 3 MiB for a 50 GiB post |
| Kad | one `ALIAS` record per post and per file, beside `SOURCE` records |
| What a storing node learns | that a node has a post, and its post id in clear, from the `ALIAS` record ([`alias.md`](alias.md) §1). Id = hash of message IDs → node can test posts it knows |
| What others learn | the post's message IDs, from the article list; for meaninglessly named posts all an index sells. §6 shares them on purpose, unless the user holds posts back |
| What a news server learns | nothing about ed3k |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **The hash of the NZB file as the name** | one hash call; no parsing rules | each indexer writes its own NZB per post → own name each, no root collects publishers |
| **The NZB file itself as the article list** | no second format | not canonical: two holders, two files, two roots. Carries subject, poster, group names files don't need |
| **Message IDs inside the manifest** | one file to fetch | 45 B per article: large post exceeds the manifest's 4 MiB |
| **Byte ranges per article in the list** | range leads to its article without a guess | node unable to fetch an article doesn't know its range, can't write the list. Article says it itself |
| **A record per article**, message ID → root and offset | finds a file from a single article | hundreds of thousands of records per post |
| **CRC-32 or the repair files' hashes as a name** (`ALIAS` of `md5`) | reuses [`alias.md`](alias.md) §4.1 | not every post has repair files; where present, a client can publish that record too |
| **Names from the repair files** in the canonical manifest | real names for meaninglessly named posts | two clients, one with repair files, one without, would write two manifests |
| **Articles served over ed3k**, by message ID | node could pass on what it has before the file is whole | second wire identity; data not verifiable under the root (`Protocol.md` §7.4) |

---

## 11. Open points

- Servers wanting a group selected before giving an article. Group not in list; client has it
  from its starting NZB, not in §4.1.
- Post repaired with fewer articles than it has: file whole, list names unfetchable articles;
  correct, useless; marking dead ones would be local.
- Test vectors (file id, post id, article list, canonical manifest) of a two-file post, one name
  not UTF-8, in [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- Are differing part sizes within one file common enough to need more than "ask and correct"?
- Posting to Usenet from ed3k not specified.
- All numbers are first values.
