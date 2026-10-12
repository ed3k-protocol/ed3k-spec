# Extension: links at file hosters (`hoster`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`http-source.md`](http-source.md) §1, §2 and §5, [`alias.md`](alias.md) §2 and §4, [`file-set.md`](file-set.md) §1, and [`Protocol.md`](../Protocol.md) §12.1 and §13.2. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 17, `LINKS`. No message types; fields in existing messages, one field in a manifest, one parameter in `ed3k:file` links, two schemes of the `ALIAS` record.

A file hoster gives a link to a page, not to bytes. Between the link and the bytes stand a
form, a timer, sometimes a puzzle for a human or a login, and an address that is valid for
minutes. This extension lets nodes tell each other such links, and puts what is fetched from
them under the BLAKE3 root like bytes from any other source.

- A **link** is described like an HTTP source ([`http-source.md`](http-source.md) §1). It
  travels in fields of its own, so that a node without this extension never fetches a page and
  takes it for the file.
- A **resolver** turns a link into a stream of bytes. It is a part of the client and outside
  the protocol (§3).
- A link leads to a root through an `ALIAS` record (§4).
- What hosters hold is often **an archive in several volumes**. Each volume is an ordinary
  ed3k file, a file set groups them, and a signed record says which file the archive unpacks
  to (§6).

The rules of [`alias.md`](alias.md) §5 apply. NeoLoader joins hosters with its own network in
one download; what was taken from it and what was not is in
[`docs/neoloader-comparison.md`](../docs/neoloader-comparison.md) §2.2.

---

## 1. A link as a source

A link is an `HttpSource` ([`http-source.md`](http-source.md) §1) whose `url` names a page of a
hoster. All its fields keep their meaning:

| Field | For a link |
|---|---|
| `url` | the link, as a user would paste it. At most 1,024 bytes, at most 256 bytes in a Kad record |
| `offset`, `length` | the range of the file that the hoster's file holds. 0 and 0: the whole file, which is the usual case |
| `expires` | when the link may be gone; 0 = unknown |
| `sealed` | set when the hoster's file is a sealed chunk ([`http-source.md`](http-source.md) §1). The rules of a sealed chunk apply, its size limit included |
| `hashes` | a plain URL of the file's hash body ([`http-source.md`](http-source.md) §3.1), not a link |

- **The bytes the hoster delivers are the bytes of the file,** or of the range, or of the
  sealed chunk. A link to anything else, such as an archive that contains the file, is not a
  source of the file. It is a source of the archive (§6).
- A link is never placed in an `http` field, and a URL of bytes never in a `links` field.

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

- A node sends `links` only to a peer that set `LINKS`. A storing node keeps and returns
  `Store.links` whether or not it sets the bit; it checks the limits and nothing else.
- The rules of [`http-source.md`](http-source.md) §2.1 to §2.3 apply to links as they do to
  HTTP sources: when an uploader may send them, that a downloader reports the outcome, that a
  publisher attaches only what it vouches for, and that a link may be passed on.
- A node vouches for a link when it has fetched at least one block from it and verified that
  block, or when it uploaded the file there itself.

**Results.** `HttpResult.url_id` is computed from the link as it was received. This extension
adds three codes:

```protobuf
// in HttpResult.Code (http-source.md §2.1):
NO_RESOLVER = 8;              // this client has no resolver for the hoster
NEEDS_USER = 9;               // a wait, a puzzle, a login or a quota stands in the way; not tried further for now
GONE = 10;                    // the hoster says the file was removed
```

An uploader stops handing out a link after 3 *(initial)* results of `GONE`, `SIZE_MISMATCH` or
`CORRUPT` from different downloaders. `NO_RESOLVER`, `NEEDS_USER` and `HTTP_FAILED` say
something about the downloader or the moment, and do not count.

---

## 3. The resolver

The resolver is the part of a client that knows hosters. The protocol fixes what it is asked
and what it answers, not how it is built.

| Asked | Answered |
|---|---|
| **Check** a link | whether the file is there, its name and size, and a checksum where the hoster tells one |
| **Open** a link at a byte offset | a stream of the hoster's file from that offset, or from 0 when the hoster cannot resume |
| **Expand** a link that names a folder or a list | the links it stands for, each with name and size where known |

A resolver may also answer: no resolver for this hoster, wait this long, a human or a login is
needed, the file is gone.

**Rules for a client**

- A client without a resolver still stores links, passes them on and looks them up.
- **A resolver is code about other people's sites. It comes with the client.** A client MUST
  NOT take resolver code, or rules that steer one, from a peer or from a Kad record. Updates
  come the way updates of the client come.
- A resolver SHOULD run apart from the node: in a process of its own, without access to the
  node's keys and files. It handles pages that the hoster and its advertisers control.
- The checks of [`http-source.md`](http-source.md) §5 apply to the link before the resolver is
  asked: scheme, public target address, range, origin and load. The resolver applies the
  address check to every request it makes itself.
- **Accounts.** A user may give the resolver a login for a hoster. Login data, session cookies
  and the addresses a hoster hands out after a login are never sent to a peer, never put into
  a record or a link, and MUST NOT be written to a log or shown in an error message.
- A client MUST NOT solve a puzzle, wait out a timer or use an account for a link without the
  user having allowed downloads from hosters. It is off by default.
- **A link is used sparingly.** Many hosters allow one download at a time and a quota per day
  for each address. A client opens at most one stream per hoster and account at a time
  *(initial)*, and counts failed attempts per hoster with the limits of
  [`http-source.md`](http-source.md) §5.

---

## 4. From a link to the root

A user who pastes a link wants to know whether ed3k has the file, before the hoster's timer
starts. An `ALIAS` record ([`alias.md`](alias.md)) of the scheme `url` says so.

```
canonical = host ‖ path [ ‖ "?" ‖ query ] [ ‖ "#" ‖ fragment ]
id        = H("ed3k v1 url", canonical)                          32 B
```

| Part | Canonical form |
|---|---|
| Scheme | `http` or `https`. Not part of `canonical` |
| Host | lower case, in its ASCII form; a leading `www.` is removed; no port, no user name |
| Path | as in the link. Percent-escapes of letters, digits, `-`, `.`, `_` and `~` are decoded, other escapes are written with upper case digits. An empty path is `/` |
| Query, fragment | as in the link, each left out when empty. The fragment stays: some hosters keep the key of the file there |

| Relation | Meaning | `root` |
|---|---|---|
| `SAME` | the link delivers exactly this file | the file |
| `SET` | the link names a folder or a list | a manifest that lists its files, with their links (§5) |

**Publisher.** A node publishes the record when it fetched the whole file from the link and
the bytes gave the root, or when it uploaded the file there itself. A node that shares the
file does so unless it holds links back (§4.1). For `SET`: when it
expanded the link itself and holds every member.

**Searcher**

- It looks the link up before it asks the resolver to open it. For a root it finds, it
  searches sources as for any file, and adds the link as a source of that root.
- **The root is a claim until bytes from the link pass under it.** A client that wants to be
  sure fetches the first block from the link and verifies it; block hashes come from an ed3k
  source (`Protocol.md` §12.4).
- If a block from the link fails under the root while the link was the name the user started
  from, the claim is false: the client ignores the publisher, drops what it took for that root,
  and downloads from the link alone (§4.1).
- A hoster can replace the file behind a link. A true record then becomes a false one; this
  looks the same to the client and costs the same.

Two links to one file at one hoster often differ in their text. Each is a name of its own, and
this extension does not try to see that they are the same (§9).

### 4.1 A link without a root

Nobody has published the file, or the client has no ed3k source for it.

- The client downloads from the link. Until the file is complete there are no hashes to verify
  a block with. Where the hoster tells a checksum, the complete file is checked against it;
  such data is **checked, not verified** ([`alias.md`](alias.md) §5 rule 5).
- When the file is complete the client computes its root. From then on it is an ordinary
  shared file and the client is its first source.
- **A link that was downloaded is shared.** Once the file is complete and verified, a client
  that shares the file publishes the `ALIAS` record of the link, attaches the link to its
  `SOURCE` record, and hands it to its downloaders (§2). This holds for a link the user
  pasted as for any other, and for a link that came with a known root as well.
- **Holding links back** is a setting, off by default. With it on, the client does none of
  the three: no `ALIAS` record of the link, no link at its `SOURCE` record, none handed to
  downloaders, and no link in a manifest it makes. **The file is shared all the same,** under
  its root, like any file. The setting covers every link the client knows, also those it got
  from peers.
- Whether a client shares what it downloaded from a hoster at all is the user's choice, as
  for any file.

---

## 5. Links in a manifest

```protobuf
// in Member (file-set.md §1):
repeated string links = 11;   // links that deliver this member whole; at most 4, each at most 1,024 B
```

- A set whose members carry links is a download that needs no node to be online for the
  bytes: the manifest names the roots, and the links name where the bytes are. It is what a
  folder link, or a list of links in a text, becomes in ed3k.
- A link in a manifest is a claim of the manifest's author. Each block from it is verified
  under the member's root, so a wrong link costs a request and nothing else.
- A client without this extension ignores the field and sees a plain set.
- The manifest is immutable ([`file-set.md`](file-set.md) §1). Links that die are replaced in
  a new version of a set that changes ([`file-set.md`](file-set.md) §5), or found through
  `links` in the file stream and in Kad (§2).

---

## 6. Archives

Much of what lies at hosters is an archive cut into volumes, sometimes with a password. The
archive is not the file a user looks for, but it contains it.

**Volumes are files.** Each volume has a root of its own and is downloaded, verified and
shared like any file. A link to a volume is a source of that volume. A manifest lists the
volumes of one archive, in their order, with their names as the archive format needs them.
It may list other files too, which are left aside when unpacking.
Nothing in the manifest marks it as an archive.

**What it unpacks to** is a signed claim, an `ALIAS` record of the scheme `b3`
([`alias.md`](alias.md) §1):

| Field | Value |
|---|---|
| `id` | the root of the file that comes out |
| `rel` | `PACKED` |
| `root`, `size`, `name` | the manifest of the volumes. For an archive of one volume without a manifest: that volume |
| `entry` | the path of the file inside the archive; absent when the archive holds one file |
| `password` | the password of the archive, if it has one |

The target is computed from the root of the file that comes out. A client that looks for
sources of a file can so find an archive that holds it.

**Publisher.** A node publishes the record only when it unpacked the archive itself, holds
every volume, and the file that came out has the root in `id`.

**Searcher**

- It searches `ALIAS` records of the scheme `b3` for a file only when the user asks for it, or
  when the file has no source that delivers.
- It downloads the volumes as a set of their own. Every block of a volume is verified under
  the volume's root, whoever sent it.
- **Unpacking is the check.** The client unpacks only `entry`. It stops and drops the output
  when the output grows beyond the size of the file it looks for. Where it has block hashes of
  that file, it verifies the output block by block as it comes; otherwise it computes the root
  at the end.
- If the output is not the file, the claim is false. The client ignores the publisher. The
  volumes stay what they are: verified files of their own roots.
- Bytes of the file that the client already has do not make the archive smaller. A client
  SHOULD compare what is still missing with `set_size` before it starts.

**Rules for unpacking**

- A client unpacks by formats it chooses to support, and MAY refuse any. An archive it cannot
  read is not a false claim.
- The output is written as a stream to the place of the download. Paths, links and attributes
  from inside the archive are not used.
- Unpacking reads data that its author controls. It SHOULD run apart from the node, like a
  resolver (§3).
- `password` is public: it stands in a record anyone can read. It is the password of an
  archive that is being shared anyway, not a secret of the user.

---

## 7. Who is blamed

| Case | Blamed |
|---|---|
| A block from a link fails under the root, link from a peer or from Kad | the link: it is dropped, and the node that named it is told `CORRUPT`. It is not banned; hosters replace files |
| The same, link from a manifest | nobody; the link is dropped |
| An `ALIAS` record of the scheme `url` whose root fails for bytes from that link | the publisher of the record |
| A sealed chunk at a hoster | as in [`http-source.md`](http-source.md) §6 |
| An archive that does not give the file | the publisher of the `PACKED` record |
| A volume that fails under its root | the sender of the block, as for any file |
| The hoster | never by the protocol. A client keeps its own count of failures per hoster |

---

## 8. What it reveals

- **To the hoster:** the address of the node, and which file it fetches. With a login, also
  who the user is to that hoster.
- **To a peer or a storing node that is given a link:** the link. A link is often all that
  protects a file at a hoster. This extension shares it on purpose: a link the user pasted is
  passed on and published once the file is downloaded (§4.1). A client MUST say so where
  the user switches downloads from hosters on, and offer the setting that holds links back
  there. What stays with the node is everything of an
  account (§3).
- **In Kad:** an `ALIAS` record of the scheme `url` holds a hash of the link, not the link.
  Whoever knows the link learns the root and the publisher.
- **To the pages of a hoster:** whatever the resolver lets them see. This is the reason for
  running it apart (§3).
- **Through an anonymous path** ([`hops.md`](hops.md)): a link fetched directly shows the
  node's address to the hoster. A node that hides its address MUST NOT fetch links, except
  through the same means it uses for HTTP there.

---

## 9. What it gives and what it costs

| | |
|---|---|
| Sources | files that no node shares can still be fetched, and a file at a hoster gains ed3k sources |
| Verification | every block under the root where an ed3k node has the file; otherwise the whole file at the end |
| A dead link | costs one request. A file in ed3k outlives the link it came from |
| Dependence | on code that follows the pages of each hoster. It breaks when a page changes, and no rule of the protocol can fix that |
| Speed | one stream per hoster, a wait before it, a quota per day; these are the hoster's terms |
| Law and terms | fetching by program is against the terms of many hosters. That is between the user and the hoster; the protocol only names links |

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A flag in `HttpSource`** that says "this URL is a page" | no new fields | a node that knows HTTP sources and not this flag fetches the page, takes it for the file, and reports the source as corrupt |
| **Describing the steps to the bytes** in the record (a form to post, a pattern to find) | no code per hoster in the client | hosters change pages weekly and work against it; a record that tells a client what to request is a peer steering a client's requests (§3) |
| **The address of the bytes** that a resolver found, passed on as a plain HTTP source | no resolver at the receiver | such addresses are bound to an IP address or a session and live for minutes; with an account they are a credential |
| **The archive as a second form of the file itself**, with ranges of the file mapped into volumes | bytes from volumes and from peers in one download | needs the layout of every archive format in the protocol, and works for stored archives only; compressed or encrypted ones have no such map |
| **No `PACKED` record:** archives and their content stay unrelated | no unpacking on a protocol path | peers that hold the file cannot help a user who has the links, and the other way round |
| **The `PACKED` relation inside the manifest** of the volumes | found with the manifest, no lookup | the manifest's root would depend on who looked inside; and the search starts from the file, not from the archive |
| **One identity for all links to one file of a hoster**, given by the resolver | two spellings of a link meet in Kad | the identity would depend on the resolver and its version; two clients would compute two targets |

---

## 11. Open points

- **Uploading to hosters** as a cache, the way sealed chunks go to cache servers
  ([`http-source.md`](http-source.md) §4). A sealed chunk at a hoster can be described (§1);
  when and where a node uploads is not specified.
- **The same file under several links.** A table of link patterns per hoster would let
  clients agree on one form, at the price of a registry that ages.
- **Block hashes for a link without an ed3k source.** A `hashes` URL is one way; nothing says
  who would host it.
- **Passing on a checksum** that a hoster tells, as an `ALIAS` record of `md5` or `sha1`
  ([`alias.md`](alias.md) §4.1).
- **A record from an archive to what is inside,** for a user who has the volumes and wants to
  know before unpacking.
- Whether `NEEDS_USER` should say which of its four causes it was. It would tell an uploader
  more, and a peer something about the user.
- All numbers are first values.
