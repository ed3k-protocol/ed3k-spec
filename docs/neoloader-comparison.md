# NeoLoader (NeoKad and NeoShare) compared with ed3k

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §7, §9, §10, §12 and §14, and [`extensions/file-set.md`](../extensions/file-set.md). Section numbers (§) refer to `Protocol.md`.

NeoLoader is a download client with a network of its own: **NeoKad**, a Kademlia DHT whose
storage logic runs as scripts and that can relay traffic, and **NeoShare**, a file transfer that
runs over it. It also speaks eD2K, BitTorrent and web hosters, and joins all of them in one
download. This note records what ed3k can take from it and where the two differ.

**Result:**

- Two further extensions followed from it on 2026-10-12: other names of a file, and links at
  file hosters (§2.2, §9).

- One feature was adopted: multi-file sets, as [`extensions/file-set.md`](../extensions/file-set.md),
  built differently from NeoShare's (§2).
- Three small gaps in ed3k showed up in the comparison. They are closed as listed in §9.
- Most of NeoShare's transfer rules match what ed3k already has. The values it uses for a turn
  and a slot are close to ed3k's, arrived at independently.
- The two central ideas of NeoKad, scripts on storing nodes and relayed transfers, are not
  adopted. §7 and §8 give the reasons.

**Method.** The NeoLoader source tree was read on 2026-10-11; nothing was run. File references
are relative to its `NeoLoader/` directory. The statements marked *by reading* are a trace of
the logic that no test confirmed. The tree's own summary (`Docs/NeoShare.md`) is wrong in a few
places; where this note differs from it, the code was followed.

## 1. Overview

| | NeoKad / NeoShare | ed3k |
|---|---|---|
| Transport | uTP over UDP, own obfuscation, own channel encryption | QUIC with TLS 1.3, obfuscation layer (§4) |
| Node ID | SHA-256 of an EC key, folded to 128 bits; not bound to the address | BLAKE3 of an Ed25519 key; routing ID bound to the address (§6.3) |
| Kad messages | a stream and a handshake per queried node | one request/answer exchange per node (§10.3) |
| Record kinds | free: a path and a payload, handled by scripts | fixed: 13 kinds in version 1 (§10.3), each with the checks of the document that defines it |
| Peers find each other | by identity and a rendezvous area in the DHT; data runs through relay nodes | by address; relays carry signalling only (§9.4) |
| File hash | SHA-256 tree, 256 KB leaves, top 12 levels kept | BLAKE3 tree, 256 KiB blocks, all levels (§7) |
| Multi-file | cross hash over a set, list fetched from peers or the DHT | manifest file (extension) |
| Other networks | eD2K, BitTorrent, hosters in one download | eD2K optional (§14); HTTP sources, torrents and hoster links as extensions, all under one root ([`extensions/alias.md`](../extensions/alias.md) §5) |

## 2. Multi-file sets

This is the feature that led to [`extensions/file-set.md`](../extensions/file-set.md).

**How NeoShare does it** (`NeoLoader/FileList/File.cpp:937-973`,
`FileList/Hashing/FileHashTreeEx.cpp`, `FileTransfer/NeoShare/NeoManager.cpp`):

- The set has a hash of its own: `SHA256(MetaHash ‖ TreeRoot)`.
  - `TreeRoot` is a hash tree over all sub-files written one after the other, without padding.
  - `MetaHash` is the SHA-256 of, per sub-file, its size and its own tree hash.
- Every sub-file also has a tree hash of its own and is a normal file of the client. A file the
  client already has is reused.
- A downloader that knows only the set hash asks peers for the list, one sub-file entry per
  message, and assembles it. The same list can be stored in the DHT, limited to 64 KB.
- A record under a sub-file's hash names the sets that contain it.

| | NeoShare | ed3k file set |
|---|---|---|
| Identity of the set | hash over sizes, sub-file hashes and a second tree over the joined data | root of the manifest file |
| Names and directories | **not covered by the hash**; taken from whichever peer answered | covered: they are bytes of the manifest |
| Hashing work | every byte is hashed twice, once per file and once for the set | once per file |
| How the list travels | a message pair per entry, with voting on the number of entries | as a file, with the block transfer that exists |
| Verification of the list | sizes and hashes against `MetaHash` | every block against the manifest root |
| A member alone | a normal file | a normal file |
| Empty files | the whole set is refused | allowed |
| Limit | 100,000 entries | 65,536 entries, 4 MiB |
| From a member to its sets | a DHT record | a `MEMBER` record under the file's root ([`extensions/file-set.md`](../extensions/file-set.md) §6) |

What was taken:

- **Members stay normal files with a hash of their own.** This is the part of NeoShare's design
  that works well: partial download, reuse of files already held, shared sources.
- **Path rules.** NeoShare cleans paths it receives (no `.` or `..`, limits on depth and
  length). The extension refuses a bad manifest instead of repairing it, because a repaired
  list would no longer be the list that was hashed.

What was not taken:

- **A second tree over the joined data.** It costs a second pass over all data and commits to
  nothing that the member hashes do not already commit to.
- **An exchange protocol for the list.** NeoShare needs one because its list is not a file.
  Its implementation has to vote on the entry count and try combinations of offers (up to 8 × 8)
  until one matches the hash.
- **Names outside the hash.** Two peers can give different names for the same set, and nothing
  decides between them.

### 2.1 Torrents under the set hash

NeoLoader downloads a torrent from BitTorrent peers and NeoShare peers at once. Read on
2026-10-12, *by reading*.

**How NeoLoader does it**

- A torrent with several files becomes one file whose part map is joined from sub-files. Each
  file of the torrent is a sub-file with a part map of its own, linked to its range in the
  whole (`FileTransfer/BitTorrent/Torrent.cpp:75-135`).
- The download starts under the torrent hash as **master hash**. Further hashes of the whole
  and of each sub-file are looked up in the DHT as aliases: a map of hash types to values
  (`FileTransfer/NeoShare/NeoKad.cpp:226`). An alias is taken as it comes.
- A false alias is found by its effect. If a range is corrupt under an added hash and lies
  wholly inside a range that the master hash verified, the added hash is put on a black list
  (`FileTransfer/HashInspector.cpp:446`, `:808`).
- The set hash of this section needs the tree over the joined data and the hash of every
  sub-file. A torrent gets it when it is complete, and is published under it then, with the
  list of sub-files and their hash maps (`FileTransfer/NeoShare/NeoKad/HashPublisher.cpp:105-150`).
- A sub-file with a hash of its own is a normal file, with sources of its own in NeoShare and
  eD2K.

| | NeoLoader | ed3k ([`extensions/torrent.md`](../extensions/torrent.md)) |
|---|---|---|
| One hash for a torrent | `SHA256(MetaHash ‖ TreeRoot)` over the joined data | the root of a manifest in a fixed form, which every node computes alike |
| Commits to the infohash | no; the link is an alias record | yes, a field of the manifest |
| Commits to names and paths | no | yes |
| From the infohash to that hash | alias record, unsigned | `ALIAS` record, signed, ordered by publisher sites |
| A false link between the two | found by a hash conflict, the added hash is dropped | the same test (`torrent.md` §4.2) |
| Who decides | the hash the download started with | the same: BLAKE3 for a start from an ed3k link, the torrent for a start from a magnet link |
| Hashing | every byte twice | once per file |
| A file of the torrent alone | a normal file | a normal file |

What was taken: the master hash, the conflict test, and sub-files as normal files. What was
not: the tree over the joined data, for the reasons above, and aliases that nobody signs.

### 2.2 Links at file hosters

NeoLoader downloads from file hosters next to its peers. Read on 2026-10-12 at commit
`34d847a4`, *by reading*; paths below `FileTransfer/HosterTransfer/` are given by file name.

**How NeoLoader does it**

- **A link is turned into bytes by a script.** One JavaScript file per site names URL patterns
  and the entry points `Check`, `Decrypt` and `Download` (`Scripts/*.js`). Nearly all of them
  are three-line stubs that call into one 12.7 MB script, which is a Java download manager's
  plugin collection compiled to JavaScript (`JLoader/`, `Scripts/jLoader.js`).
- **The script does not hand over an address.** The plugin's own HTTP session performs the
  transfer, and the host writes the reply into the file (`HosterTask.cpp`,
  `JLoader/src/jd/plugins/BrowserAdapter.java:66-71`). One connection per link; resuming where
  the site allows it.
- **Two kinds of link** (`HosterLink.cpp`):
  - A *range link* holds ranges of the file itself, written behind the URL as
    `\Range: … \Crypto: <cipher>:<key>` (`HosterLink.cpp:924-934`). Its bytes are verified
    like bytes from a peer, under the master hash.
  - An *archive link* is a volume of a 7z or RAR archive. Volumes are grouped by their file
    name, not by content. Nothing verifies a volume; a failed extraction is the only check.
    The files that come out are hashed and matched to the download
    (`ArchiveDownloader.cpp:82-111`).
- **A link with no hash at all** gives a file that is completed "without hashing"
  (`HashInspector.cpp:166-175`) and hashed afterwards.
- **Links are found in the DHT** under the file hash, by a script on the storing node that
  returns every list it was given (`Resources/KadScripts/LinkSafe.js`). Links are hidden from
  the user interface by a cipher with a key that is fixed in the source
  (`HosterLink.cpp:1065-1091`).
- **Uploading to hosters** as a cache exists in the code: ranges of up to 100 MB, encrypted
  with a key per link that travels in the link, at most 3 copies of a range
  (`NeoCore.cpp:820`, `:854-858`). It is off by default, and no script in the tree can upload.
- **Scripts are signed,** but the check does not gate loading, and updating them over the
  network is switched off.

| | NeoLoader | ed3k ([`extensions/hoster.md`](../extensions/hoster.md)) |
|---|---|---|
| What turns a link into bytes | scripts, named in the protocol's own tree | a resolver outside the protocol; never code from a peer |
| A link to the file's own bytes | range link, verified under the master hash | the same: an `HttpSource` in a field of its own, every block under the root |
| A range at a hoster, encrypted | key in the link | a sealed chunk, with a digest of the ciphertext |
| A volume of an archive | a part of an archive set, named by file name, unverified | an ordinary file with a root; a manifest lists the volumes |
| From the archive to the file | by hashing what comes out | a signed `PACKED` record, checked by unpacking |
| From a link to the file | not possible; links hang under a hash | an `ALIAS` record under a hash of the link |
| A link with no known hash | accepted unverified | downloaded, then it is an ordinary file under the root it gives |
| Where links are found | DHT record under the file hash, unsigned by default | fields next to the `SOURCE` record, and in a manifest |
| Hiding links | a fixed key | none claimed: a link that is passed on is known to who receives it |

What was taken: a link as a source of exact bytes, ranges at hosters, and archives as a second
way to a file. What was not: scripts as part of the network, volumes that nothing verifies,
and completing a file without a hash.

## 3. File hash and verification

| | NeoShare | ed3k |
|---|---|---|
| Leaf / block | 256 KB | 256 KiB |
| Levels kept | 12, about 1,024 leaves *(by reading)*; a larger file is verified in larger units | all |
| Proof for a part of the tree | none; the kept tree is sent whole and checked whole | `HashRequest` for a range with a proof (§12.4) |
| Leaf and inner nodes told apart | no | yes, by BLAKE3's flags |
| Tree root commits to the length | no *(by reading)* | yes |
| Sender of a bad block | known per verified range; blame shared by byte count among all who contributed | known exactly: one block, one sender |
| Reaction to bad data | source dropped for that file above 40 % bad bytes, forgotten after 4 h | one bad block drops the source for that file for 4 h; a second drops the node for 24 h (§12.3) |

- Without a mark for leaf and inner nodes, and without the length in the root, a 64-byte file
  made of two node hashes has the same root as the two-leaf file they belong to. BLAKE3 rules
  that out by construction, which is one more reason for the choice in
  [`hash-comparison.md`](hash-comparison.md).
- NeoShare can also store the hash tree in the DHT, so that a download can start before a
  source answers. ed3k has the `wh` link parameter of the HTTP source extension for the same
  purpose. A DHT record would be limited to the record size.

## 4. Transfer and queue

| | NeoShare | ed3k |
|---|---|---|
| Request size | 64 KB, never across a 256 KB border | up to 256 KiB (§12.2) |
| Requests outstanding | starts at 16, doubles when all were served, halves when over half are open, at most 100 | 16 |
| Bytes outstanding | 1 to 6.4 MB | 4 MiB |
| Availability message | bitmap, blocks joined by powers of two until it fits in 1 KB | piece bitmap at 4 MiB (§12.1) |
| Availability updates | none; sent once | `have` in later messages |
| Who gets a slot | a draw, weighted by the ratio of bytes exchanged (0.5 to 4) and file priority | local policy (§12.1) |
| Waiting | connection closed after 2 min idle; downloader asks again every 20 min | uploader decides: stay, or come back after 30 min to 4 h (§12.5) |
| Granting a slot | the uploader dials the downloader | the same: `SlotOffer` (§12.5) |
| Queue entry expiry | none found | 30 min after the time to come back |
| Turn | 18 MB when someone waits | 16 MiB, at most 1 h |
| Speed aimed at per slot | 8 KB/s | 9 to 55 KiB/s |
| Minimum slots | 2 | 2 |
| Source exchange between peers | none | `FileStatus.sources` |
| End of a download | up to 5 sources planned for one range | not stated; local |

- **Turn, slot speed and minimum slot count agree** with what ed3k chose in §12.5.
- **The adaptive request count is not needed.** NeoShare starts at 1 MB outstanding because its
  requests are 64 KB. ed3k's 16 × 256 KiB cover about 40 MiB/s at 100 ms round trip.
- **The draw instead of a queue** is a scoring policy and stays local in ed3k. A draw gives a
  waiting downloader no position to report, which is why NeoShare always answers with rank 1.
- **Part selection** in NeoShare finishes started parts first, then prefers rare ones with a
  random share, and raises the start and the end of a file for preview and streaming. ed3k
  leaves selection to the downloader; nothing on the wire depends on it.

## 5. Kad

| | NeoKad | ed3k |
|---|---|---|
| Bucket | 10; a full bucket refuses the newcomer | 10 (§10.2) |
| Who is handed out | by default only nodes that answered a handshake | verified contacts only (§10.3) |
| Address and site limits | none found | [`extensions/ip-diversity.md`](../extensions/ip-diversity.md) |
| Responsible for a target | within the distance of the 20th closest node known after a self lookup | fewer than K verified contacts closer (§10.3) |
| Lookup parallelism | 2 | 3 |
| Cost per queried node | uTP stream and key exchange; a cached key is reused later | QUIC handshake (§10.5) |
| Record lifetime | asked by the publisher, at most 90 days; in effect 90 days for most records | 5 h and 24 h, in the signed record (§10.3) |
| Records per target | 1,000, shared by all kinds; refused when full | per kind; keyword targets as in §10.8 |
| Store answer | expiry per record, and a load figure | result per record, `load`, `spill` |
| Who may replace a record | the same key; other keys add a row | the same publisher (§10.7) |
| Reader can verify a record | no; the signature is checked by the storing node only | yes, the record carries key and signature |
| Limits per address, bans | none found | §4.6 and the flood protection extension |
| Keyword split | lower case, split on a fixed list of ASCII punctuation; no minimum length, no Unicode rules | §10.1 |

- **Lifetime.** The client sends an absolute time where the storing node expects a duration,
  so the node cuts it to its maximum of 90 days
  (`FileTransfer/NeoShare/NeoKad/SourcePublisher.cpp:52`, `NeoKad/Kad/PayloadIndex.cpp:185`).
  A source record with an address stays for that long. ed3k avoids the class of error by
  signing an absolute `expires` and by having the storing node check it against the lifetime of
  the kind (§10.7).
- **Keyword split.** Publisher and storing node lower-case with different libraries, and can
  disagree on some characters. This is the case §10.1 was written for.
- **Ratings.** The publisher ID is removed from a rating before it is stored, and the client
  keys ratings by that ID, so all ratings of a file collapse into one on the reader
  *(by reading)*. ed3k's notes are signed and counted per author (§10.7).
- **Expiry anchored to the publisher's date.** NeoKad computes the expiry from the signed
  release date of a record, not from the time it arrived, so replaying an old record gains
  nothing. ed3k has the same property through the signed `expires`.

## 6. NAT traversal

| | NeoKad | ed3k |
|---|---|---|
| Own address | 3 peers asked; 60 % must agree on the address | 3 counted reports from different sites, twice as many as for any other endpoint (§9.2) |
| Reachability test | a peer asks a third node to send to the tested address; that node refuses when it has a connection to it | the same (§9.3) |
| Address that is tested | **given by the asker** | the asker's observed address, never a value from the asker |
| Test repeated | every 60 min; after a test nobody completed, 30 s doubling up to 60 min | the same, and at once on an address change (§9.2) |
| Symmetric NAT | inferred when peers see different ports | the same |
| Port mapping (UPnP, PCP) | not found | first step (§9.1) |
| Helper for an unreachable node | 1 random node per protocol; not checked to be reachable itself | 2 relays, `ROUTER` nodes with `RELAY` set (§9.4) |
| Helper capacity | 10 | at least 8 (§9.4) |
| Forward limit | none; forwards for anyone | 30 per minute per leaf, 3 per initiator (§9.4) |
| Who dials after the punch | decided by comparing two keys | fixed: the initiator (§9.5) |

- NeoKad lets a node name the address to be tested, so a third node can be made to send a
  datagram to any address. ed3k closed that in §9.3.
- NeoKad's own code notes that a helper that talked to the tested node shortly before can
  still pass its NAT, so unreachable nodes test as open in a small network. ed3k counts a test
  only when it comes "from a node it had never sent to" (§9.2).
- The signalling for hole punching exists in NeoKad, but the step that turns a punched path
  into a usable connection has no caller *(by reading)*. Its numbers are therefore not evidence
  for or against a design.

## 7. Scripts on storing nodes

In NeoKad a storing node runs JavaScript that the publisher or searcher names by the ID of a
signing key. A node that lacks the script, or has an older version, fetches it from the
requester. Six scripts ship: sources, keywords, hash and set data, ratings, hoster links, and
an empty one for hubs.

What this allows without a new client release:

- new record kinds and new fields
- search logic on the storing node (matching, filtering by hash type, counting publishers)
- a write rule per record kind (the link script pins the first writer's key)
- other applications on the same DHT

What it costs, as the tree shows:

- **Every node runs code from strangers.** The configuration carries a block of limits added
  later, with the comment that each of them "used to be unbounded": script count and size,
  memory, storage, run time. When the memory limit is passed the whole engine is reset,
  because it is not tracked which script used it.
- **The freedom is little used.** Four of the six scripts only add a prefix to a path and call
  the built-in store. None checks the content of a record.
- **Readers cannot verify.** The built-in store checks the publisher's signature and then
  hands out the data without it.
- **Upgrades are by highest version of one key.** There is no way back from a bad version.

ed3k keeps fixed record kinds whose content every node can check (§10.7), and changes them by
network upgrade (§16). The one thing scripts give that ed3k lacks is server-side search logic
that can grow; `Query` (§10.3) has fields for what is planned and unknown fields are ignored.

## 8. Relayed transfers and anonymity

NeoShare peers do not connect to each other. A peer is an identity plus a rendezvous area in
the DHT; data is cut into 4 KB frames and passed along relay nodes, encrypted end to end.
Lookups and stores can be handed to other nodes ("hops"), with random detours ("jumps").

- There is no layered encryption. A relay sees who talks to whom by identity, and the volume.
- The setting has five levels. The default is 0, where a node places itself next to its own
  rendezvous area and nothing stands in between.
- A relay holds frames in a queue without a size limit and forwards for anyone; the only bound
  is 100 relayed operations in total.
- Every byte costs upload at each relay.

The core of ed3k does not adopt this. Its relays forward a few signalling messages (§9.4), and
transfers run directly. The optional extension [`hops.md`](../extensions/hops.md) takes the
idea of a number of hops that the user chooses and builds it another way: the owner of a path
picks its nodes, every node adds a layer of encryption, and a node limits what it carries
(§7 there has the comparison, read from the NeoShare source). A path of one hop there is also
the data relay for pairs that cannot be punched, which §16 had reserved.

Two details of NeoKad that the extension does not use, because its links are QUIC exchanges:

- NeoKad acknowledges a frame hop by hop at once and end to end later, so a slow last hop does
  not hold the first hop's window.
- It chooses among several next hops by measured delay and loss.

## 9. Changes made to ed3k

All three were applied on 2026-10-11.

| # | Gap | Rule added | Where |
|---|---|---|---|
| 1 | §9.2 does not say when a node tests its reachability again | Test again every 60 min and at once when the local address or the observed endpoint changes. After a test without prober, retry after 30 s, doubling up to 60 min. Two tests without a probe end the `open` class | §9.2, Appendix A |
| 2 | Nothing says what a downloader does with the sender of a block that fails verification | The sender is known exactly (§7.2). One bad block: not used for that file for 4 h. A bad block in a second file, or 2 in total, within 24 h: not used at all for 24 h. Credits stay local policy. NeoShare's 40 % rule exists only because it cannot tell who sent the bad bytes | §12.3, Appendix A |
| 3 | The HTTP source extension does not limit how many sealed chunks of one range exist | A node SHOULD NOT publish a chunk for a range while it knows 2 live sealed sources for it, and publishes the second only on a server with another host name or IP address. NeoLoader's hoster cache stops at 3 | [`extensions/http-source.md`](../extensions/http-source.md) §4 |

Added on 2026-10-12:

| # | Gap | What was added | Where |
|---|---|---|---|
| 4 | A name from another network led to a root in two special ways: a field for eD2K and a record kind for torrents | One record kind, `ALIAS`, signed and ordered by publisher sites, with the kind of name as a string. NeoKad has such records too, unsigned and first come | [`extensions/alias.md`](../extensions/alias.md) |
| 5 | Links at file hosters had no place | Links as sources, archives as sets, and a record from a file to an archive that holds it (section 2.2) | [`extensions/hoster.md`](../extensions/hoster.md) |

Considered and not adopted:

| Idea in NeoLoader | Why not |
|---|---|
| A keyword `topfiles<year>cw<week>` that lists popular files without a search word | One target that everybody publishes to and everybody reads. It is the most crowded keyword by design and the first place for spam |
| File requests by any hash type (eD2K, BitTorrent) | §7.4 keeps every ed3k request on `root`, so that there is one identity to verify against |
| Source records under an eD2K hash that also name the ed3k node | Covered by the Kad tag in [`ed2k-tags.md`](ed2k-tags.md) |
| Availability bitmap that halves its resolution until it fits in 1 KB | A 100 GiB file is 3.2 KiB of bitmap at 4 MiB pieces, and bodies are compressed (§5.3) |
| Lookup tracing (each hop adds its ID) | A diagnostic aid; it would add a field to `KadAnswer` for no protocol use |
| A typed list of alias hashes in every file record, instead of the single `ed2k` field | The field is expected to be dropped (§7.4). Other names are looked up, not carried along ([`extensions/alias.md`](../extensions/alias.md) §7) |

## 10. Open points

- A way from a file to the sets that contain it (NeoKad's nesting record) is the `MEMBER`
  record of the file set extension (§6 there); what is open is whether its limits hold for
  files in very many sets.
- NeoShare's part selection for streaming is local policy, but an implementation note for
  eMuleQt could reuse its numbers: the first 8 MB or a tenth of the file, and the last 2 MB or
  a twentieth.
- The statements marked *by reading* should be checked by running NeoLoader before they are
  quoted elsewhere.
