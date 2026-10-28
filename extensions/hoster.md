# Extension: links at file hosters (`hoster`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`http-source.md`](http-source.md) §1, §2 and §5, [`alias.md`](alias.md) §2 and §4, [`file-set.md`](file-set.md) §1, [`Protocol.md`](../Protocol.md) §12.1 and §13.2. § = this document unless a document is named.
**Capability:** bit 17, `LINKS`. No message types; fields in existing messages, one manifest field, one `ed3k:file` link parameter, two `ALIAS` record schemes.
**Required of every `ROUTER`:** keeping and returning `Store.links` (§2), whether or not it sets the bit. The rest is optional.

A file hoster links to a page, not bytes; between them a form, a timer, sometimes a puzzle for a
human or login, and an address valid for minutes. This extension lets nodes exchange such links
and puts fetched bytes under the BLAKE3 root like any source's.

- A **link** is described like an HTTP source ([`http-source.md`](http-source.md) §1), but in
  its own fields, so a node without this extension never fetches a page taking it for the file.
- A **resolver** turns a link into a byte stream; client part, outside the protocol (§3).
- A link leads to a root via an `ALIAS` record (§4).
- Hosters often hold **an archive in several volumes**. Each volume = ordinary ed3k file; a
  file set groups them; a signed record says which file the archive unpacks to (§6).

[`alias.md`](alias.md) §5 rules apply. NeoLoader joins hosters with its own network in one
download; what was taken from it and what not: [`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md) §2.2.

---

## 1. A link as a source

An `HttpSource` ([`http-source.md`](http-source.md) §1) whose `url` names a hoster page; all
fields keep their meaning:

| Field | For a link |
|---|---|
| `url` | the link as a user would paste it. ≤ 1,024 bytes; ≤ 256 bytes in a Kad record |
| `offset`, `length` | file range the hoster's file holds. 0 and 0: whole file (usual) |
| `expires` | when the link may be gone; 0 = unknown |
| `sealed` | set when the hoster's file is a sealed chunk ([`http-source.md`](http-source.md) §1); sealed-chunk rules apply, size limit included |
| `hashes` | plain URL of the file's hash body ([`http-source.md`](http-source.md) §3.1), not a link |

- **Bytes the hoster delivers are the bytes of the file,** range or sealed chunk. A link to
  anything else (e.g. an archive containing the file) is a source of the archive (§6), not of
  the file.
- A link never goes in an `http` field, a URL of bytes never in a `links` field.

---

## 2. Where links travel

| Carrier | Field | Limit |
|---|---|---|
| Uploader to downloader | `FileStatus.links` = 12, `repeated HttpSource` | 8 per message *(initial)* |
| Downloader to uploader | `FileRequest.http_results` = 7 ([`http-source.md`](http-source.md) §2.1) | one per link used |
| Kad source record, stored | `Store.links` = 12, `repeated HttpSource` | 3 per publisher and file |
| Kad source search, answered | `KadAnswer.links` = 19, `repeated HttpSource` | 16 per answer *(initial)* |
| Manifest | `Member.links` = 11, `repeated string` (§5) | 4 per member |
| Link | `hl=<url-encoded link>` in `ed3k:file` (`Protocol.md` §13.2) | whole file; may occur several times |

- `links` sent only to peers that set `LINKS`. Storing node keeps and returns `Store.links`
  whether or not it sets the bit; checks limits only. A `Store` with more than 3 links in
  `links`, or a `url` or `hashes` above 256 bytes → `REJECTED`, source record not kept. Returned
  with their source record, as `http` ([`http-source.md`](http-source.md) §2.2).
- [`http-source.md`](http-source.md) §2.1–§2.3 apply as to HTTP sources: when uploader may
  send, downloader reports outcome, publisher attaches only what it vouches for, links may be
  passed on.
- A node vouches for a link if it fetched and verified ≥ 1 block from it, or uploaded the file
  there itself.

**Results.** `HttpResult.url_id` computed from the link as received. Three added codes:

```protobuf
// in HttpResult.Code (http-source.md §2.1):
NO_RESOLVER = 8;              // this client has no resolver for the hoster
NEEDS_USER = 9;               // a wait, a puzzle, a login or a quota stands in the way; not tried further for now
GONE = 10;                    // the hoster says the file was removed
```

An uploader stops handing out a link after 3 *(initial)* results of `GONE`, `SIZE_MISMATCH` or
`CORRUPT` from downloaders in 3 different sites ([`ip-diversity.md`](ip-diversity.md) §1).
`NO_RESOLVER`, `NEEDS_USER`, `HTTP_FAILED` concern the downloader or the moment; don't count.

---

## 3. The resolver

Client part that knows hosters. Protocol fixes what it is asked and answers, not how it's built.

| Asked | Answered |
|---|---|
| **Check** a link | whether the file is there, name and size, checksum where the hoster tells one |
| **Open** a link at a byte offset | stream of the hoster's file from that offset, or from 0 if the hoster can't resume |
| **Expand** a folder or list link | the links it stands for, each with name and size where known |

May also answer: no resolver for this hoster, wait this long, human or login needed, file gone.

**Rules for a client**

- No resolver → client still stores, passes on, looks up links.
- **A resolver is code about other people's sites. It comes with the client.** A client MUST
  NOT take resolver code, or rules steering one, from a peer or Kad record; updates come as
  client updates.
- A resolver SHOULD run apart from the node: own process, no access to node keys and files; it
  handles pages the hoster and its advertisers control.
- [`http-source.md`](http-source.md) §5 checks apply to the link before the resolver is asked:
  scheme, target address, range, origin, load. Resolver applies the address check to every
  request it makes itself, proxy rule included. Refused address reported as `HTTP_FAILED`, per
  that section.
- **Accounts.** User may give the resolver a hoster login. Login data, session cookies and
  addresses handed out after login: never sent to a peer, never in a record or link, and
  MUST NOT be written to a log or shown in an error message.
- A client MUST NOT solve a puzzle, wait out a timer or use an account for a link unless the
  user allowed hoster downloads. Off by default.
- **Links used sparingly.** Many hosters allow one download at a time and a daily quota per
  address. ≤ 1 stream per hoster and account at a time *(initial)*; failed attempts per hoster
  counted with the limits of [`http-source.md`](http-source.md) §5: per file, over all files
  per pair (who named the link, hoster's address), and over all files and namers together.
  Namer of a manifest link = the manifest; of a Kad-answer link = target and answering node.
  User-pasted links not counted over all files. A core blind to the addresses its resolver
  connects to counts per hoster by the link's host per `canonical` (§4), with per-host values;
  a resolver knowing several names of one hoster counts them as one.

---

## 4. From a link to the root

A user pasting a link wants to know if ed3k has the file before the hoster's timer starts; an
`ALIAS` record ([`alias.md`](alias.md)) of scheme `url` tells.

```
canonical = host ‖ path [ ‖ "?" ‖ query ] [ ‖ "#" ‖ fragment ]
id        = H("ed3k v1 url", canonical)                          32 B
```

`canonical` is built by these steps, in order. A link a step calls invalid has no `id`: no
`ALIAS` record of it published or looked up. A link above 1,024 bytes is invalid before step 1.

1. **Split** per RFC 3986 §3: fragment = after first `#`; query = after first `?` before it;
   authority = between `//` and next `/`, `?` or `#`; path = rest. Scheme `http` or `https`,
   any letter case, not part of `canonical`; other schemes → invalid.
2. **Authority.** User name removed: everything up to last `@`, inclusive. Port removed: from
   first `:` after the host, or the `:` right after `]` of a bracketed host, to end of
   authority. What follows that `:` MUST be digits only; empty port allowed. Anything else
   after `]` → invalid. Rest is the host; empty host → invalid. Port dropped on purpose: same
   path on 443, 80 or other port = one `id`; fetch decides if bytes match.
3. **Host in brackets.** MUST be an IPv6 address (RFC 4291 §2.2) w/o zone. Written in RFC 5952
   form inside `[` and `]`: lower case, longest zero-group run as `::`, always hex groups, never
   dotted IPv4 part. Anything else in brackets → invalid. Skip steps 4–6.
4. **Name.** Other hosts not percent-decoded. UTS 46 `ToASCII`, non-transitional,
   `UseSTD3ASCIIRules` = true, `CheckHyphens` = false (real host names have hyphens in third and
   fourth place), `CheckBidi` = true, `CheckJoiners` = true, `VerifyDnsLength` = false, Unicode
   16.0.0 tables for code points that version assigns, as `Protocol.md` §10.1 does. Result: lower
   case ASCII. **Failing host → link invalid**; a `%` in a host fails here.
5. **Dot and length.** One trailing `.` removed. Host is then 1 to 253 bytes, labels 1 to
   63 bytes; else invalid.
6. **IPv4 or `www.`** Last label all digits, or starting `0x` → host is IPv4. It MUST then be
   dotted decimal: exactly four numbers 0 to 255, no leading zero. Any other form (fewer parts,
   hex, octal, one number) → invalid. Any other host: leading `www.` removed, repeatedly while
   the host still begins with one: `www.www.example.com` is `example.com`. A `canonical` read
   as a link again then gives itself.
7. **Path, query and fragment**: each written by one left-to-right pass over its bytes. Output
   never re-read, so the pass is idempotent on its own output.
   - `%` + two hex digits for a letter, digit, `-`, `.`, `_` or `~`: that character.
   - `%` + two hex digits for any other byte, `%` itself (`%25`) included: the escape, upper
     case digits. So literal `%` stays `%25`, `%2541` stays `%2541` (not `%41` or `A`).
   - `%` not followed by two hex digits: invalid.
   - RFC 3986 unreserved or reserved character: as is. `+` kept as is.
   - Any other byte (among them space, `"`, `<`, `>`, `{`, `}`, every non-ASCII byte): its
     percent-escape, upper case digits.
8. **Join.** Empty path = `/`. Empty query or fragment left out with its `?` or `#`. Path
   otherwise as in the link: letter case stays, `.` and `..` segments not removed. Fragment
   stays: some hosters keep the file key there.

`canonical` is hashed as its bytes; after these steps it is ASCII.

| Relation | Meaning | `root` |
|---|---|---|
| `SAME` | link delivers exactly this file | the file |
| `SET` | link names a folder or list | a manifest listing its files, with their links (§5) |

**Publisher.** Publishes `SAME` when ≥ 1 block fetched from the link verified under the root
(block hashes from ed3k, `Protocol.md` §12.4), or the whole file from the link gave the root,
or it uploaded the file there itself. A node sharing the file does so unless it holds
links back (§4.1). `SET`: when it expanded the link itself and holds every member.

**Searcher**

- Looks the link up before asking the resolver to open it. Found root: search sources as for
  any file, add the link as its source.
- **Root is a claim until bytes from the link pass under it.** For certainty, fetch and verify
  the link's first block; block hashes from an ed3k source (`Protocol.md` §12.4).
- A client SHOULD ask an ed3k source for a range's hash values before opening the link for it.
  A block whose value hasn't arrived is kept unverified, not served, verified on arrival.
- Block from the link fails under the root while the link was the user's starting name → claim
  false: ignore publisher, drop what was taken for that root, download from the link alone
  (§4.1).
- A hoster can replace the file behind a link, making a true record false; same look and cost
  to the client.

Two links to one hoster file often differ in text; each is its own name, not equated here (§11).

### 4.1 A link without a root

Nobody published the file, or the client has no ed3k source for it.

- Client downloads from the link; no hashes to verify blocks until complete. A hoster checksum,
  if given, is checked against the complete file: **checked, not verified**
  ([`alias.md`](alias.md) §5 rule 5).
- When complete, client computes its root; from then on an ordinary shared file, client its
  first source.
- **A downloaded link is shared.** Once complete and verified, a client that shares the file
  and doesn't hold links back (below) publishes the link's `ALIAS` record, attaches the link to
  its `SOURCE` record, hands it to its downloaders (§2). Holds for user-pasted links as for any
  other, and for links that came with a known root.
- **Holding links back**: setting, **off by default**; links shared like their files. A link is
  often a hoster file's only protection, so a user given a link meant to stay private switches
  it on. Then client does none of the three: no `ALIAS` record of the link, no
  link at its `SOURCE` record, none handed to downloaders; also no link in a manifest it makes.
  **The file is shared all the same,** under its root, like any file. Setting covers every link the
  client knows, also those from peers.
- Whether a client shares what it downloaded from a hoster at all is the user's choice, as for
  any file.

---

## 5. Links in a manifest

```protobuf
// in Member (file-set.md §1):
repeated string links = 11;   // links that deliver this member whole; at most 4, each at most 1,024 B
```

- A set whose members carry links needs no online node for bytes: manifest names roots, links
  where bytes are. A folder link, or a list of links in a text, becomes this in ed3k.
- A manifest link is a claim of the manifest's author. Each block is verified under the
  member's root, so a wrong link costs a request, nothing else.
- Client without this extension ignores the field, sees a plain set.
- Reader with it ignores a member's links beyond the fourth and links above 1,024 bytes;
  manifest not refused for them.
- Manifest is immutable ([`file-set.md`](file-set.md) §1). Dead links are replaced in a new
  version of a changing set ([`file-set.md`](file-set.md) §5), or found via `links` in the file
  stream and Kad (§2).

---

## 6. Archives

Much at hosters is an archive in volumes, sometimes password-protected: not the sought file,
but containing it.

**Volumes are files**: own root each, downloaded, verified, shared like any file; a volume's
link is its source. A manifest lists one archive's volumes in order,
with names as the archive format needs. May list other files too, left aside when unpacking.
Nothing in the manifest marks it as an archive.

**What it unpacks to** is a signed claim, an `ALIAS` record of scheme `b3`
([`alias.md`](alias.md) §1, §2):

| Field | Value |
|---|---|
| `id` | root of the output file |
| `rel` | `PACKED` |
| `root`, `size`, `name` | the volumes' manifest, also for a one-volume archive: `root` is always a manifest here, per the `PACKED` comment |
| `entry` | file's path inside the archive; absent when the archive holds one file. `/` between components. Compared byte for byte with the archive's path after the format's separator is replaced by `/` |
| `password` | archive password, if any |

Target computed from the output file's root, so a source search for a file can find archives
holding it.

**Publisher.** Publishes only when it unpacked the archive itself, holds every volume, and the
output file has the root in `id`.

**Searcher**

- Searches scheme-`b3` `ALIAS` records for a file only when the user asks, or the file has no
  delivering source.
- Downloads the volumes as their own set. Every volume block verified under the volume's root,
  whoever sent it.
- **Unpacking is the check.** Unpacks only `entry`. Stops and drops unverified output once it
  grows beyond the sought file's size. Blocks already passed under the root stay, as from any
  source. With block hashes of that file it verifies output block by block as it comes; else
  computes the root at the end.
- Output not the file → claim false; client ignores the publisher. Volumes stay verified files
  of their own roots.
- Already-held file bytes don't shrink the archive. A client SHOULD compare what's missing with
  `set_size` before starting. How much more it fetches: local policy; for a file with no
  delivering source there's no other way.

**Rules for unpacking**

- Client unpacks formats it chooses to support, MAY refuse any. Unreadable archive is no false
  claim: unsupported format, method or encryption, or unnameable failure (e.g. local fault).
  Retrying: local policy. An archive of supported format and method whose
  directory, compressed data or checksum is broken gives no file, and the claim is false:
  volumes are verified, so they're the bytes the publisher unpacked. Same for a supported
  encryption that the record's `password`, or its absence, doesn't open.
- Output written as a stream to the download's place. Paths, links, attributes from inside the
  archive not used.
- Unpacking reads author-controlled data. It SHOULD run apart from the node, like a resolver
  (§3).
- `password` is public (record anyone can read): password of an archive shared anyway, not a
  user secret.

---

## 7. Who is blamed

| Case | Blamed |
|---|---|
| A block from a link fails under the root, link from a peer or from Kad | the link: dropped, namer told `CORRUPT`. Not banned; hosters replace files |
| The same, link from a manifest | nobody; link dropped |
| An `ALIAS` record of scheme `url` whose root fails for bytes from that link | record publisher |
| A sealed chunk at a hoster | as in [`http-source.md`](http-source.md) §6 |
| An archive that does not give the file | publisher of the `PACKED` record |
| A volume that fails under its root | block sender, as for any file |
| The hoster | never by the protocol. Client keeps its own failure count per hoster |

---

## 8. What it reveals

- **To the hoster:** node address, file fetched; with login, also who the user is there.
- **To a peer or storing node given a link:** the link. A link is often all that protects a
  hoster file. This extension shares it: by default a user-pasted link is passed on and
  published once downloaded (§4.1), unless the user holds links back. A client MUST say what
  sharing a link means where hoster downloads are switched on, and offer holding links back
  there. Everything of an account stays with the node (§3).
- **In Kad:** a scheme-`url` `ALIAS` record holds a hash of the link, not the link. Whoever
  knows the link learns root and publisher.
- **To hoster pages:** what the resolver lets them see; hence running it apart (§3).
- **Through an anonymous path** ([`hops.md`](hops.md)): a directly fetched link shows the
  node's address to the hoster. A node hiding its address MUST NOT fetch links there: under
  the anonymous key it uses no HTTP source either ([`hops.md`](hops.md) §1).

---

## 9. What it gives and what it costs

| | |
|---|---|
| Sources | files no node shares can still be fetched; a hoster file gains ed3k sources |
| Verification | every block under the root where an ed3k node has the file; else whole file at the end |
| A dead link | costs one request. A file in ed3k outlives the link it came from |
| Dependence | on code tracking each hoster's pages; breaks on page change, no protocol rule fixes that |
| Speed | one stream per hoster, a wait before it, a daily quota: hoster's terms |
| Law and terms | programmatic fetching is against many hosters' terms; between user and hoster. Protocol only names links |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A flag in `HttpSource`** that says "this URL is a page" | no new fields | a node knowing HTTP sources but not this flag fetches the page, takes it for the file, reports the source corrupt |
| **Describing the steps to the bytes** in the record (a form to post, a pattern to find) | no per-hoster client code | hosters change pages weekly, fight it; record telling client what to request is peer steering its requests (§3) |
| **The address of the bytes** a resolver found, passed on as a plain HTTP source | no resolver at the receiver | bound to IP or session, live minutes; with an account a credential |
| **The archive as a second form of the file itself**, with file ranges mapped into volumes | bytes from volumes and peers in one download | every archive format's layout in the protocol; stored archives only, compressed/encrypted have no such map |
| **No `PACKED` record:** archives and their content stay unrelated | no unpacking on a protocol path | peers holding the file can't help a user with the links, and vice versa |
| **The `PACKED` relation inside the manifest** of the volumes | found with the manifest, no lookup | manifest root would depend on who looked inside; and search starts from the file, not the archive |
| **One identity for all links to one file of a hoster**, given by the resolver | two spellings of a link meet in Kad | identity would depend on resolver and version; two clients, two targets |

---

## 11. Open points

- **Uploading to hosters** as a cache, as sealed chunks go to cache servers
  ([`http-source.md`](http-source.md) §4). A sealed chunk at a hoster can be described (§1);
  when and where a node uploads: unspecified.
- **Same file under several links.** A per-hoster link-pattern table would let clients agree on one form;
  cost: an aging registry.
- **Block hashes for a link w/o ed3k source.** A `hashes` URL is one way; nobody named to host
  it.
- **Passing on a hoster's checksum** as an `ALIAS` record of `md5` or `sha1`
  ([`alias.md`](alias.md) §4.1).
- **A record from an archive to its contents,** for a user holding the volumes who wants to know
  before unpacking.
- Test vectors for `canonical` (§4): [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- Whether `NEEDS_USER` should say which of its four causes applied: it would tell an uploader more, and
  a peer something about the user.
- All numbers are first values.
