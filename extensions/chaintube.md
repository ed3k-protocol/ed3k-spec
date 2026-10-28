# Extension: videos named on a chain (`chaintube`)

**Status:** draft 0.4 (2026-10-13). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §2.4 and §5, [`http-source.md`](http-source.md) §2, and [`Protocol.md`](../Protocol.md) §11 and §13.2. § = this document unless a document is named.
**Capability:** none (`SETS`). No message types or record kinds; one manifest field, three parameters in `ed3k:set` links and one in `ed3k:file` links, a set layout, a rule for links of other entries, and chain reads/transactions outside ed3k.

ChainTube: a video site whose catalogue is a set of contracts on an EVM chain. Upload, comment,
rating = transactions signed by the visitor's own account. Chain holds text and, per video, a
string saying where the media is; no media.

This extension makes ed3k the place where the media is.

- Upload = **signed file set** ([`file-set.md`](file-set.md)): renditions, further sound tracks,
  subtitles, a picture (§2).
- Chain holds one **`ed3k:set` link** per video (§3). Writing account = author; link names the
  set's key. That is the whole tie between the two identities.
- Other chain entries (thread, message, comment) name **attachments** the same way: `ed3k:set`
  link, or `ed3k:file` link to one file (§3.4).
- **Comments, ratings, profiles stay on chain.** Client reads them from a chain node; none go into
  an ed3k record (§4).
- A **site** = separate program beside an ordinary ed3k node; no new ed3k role (§5).
- A site MAY keep media in **IPFS**, served from its IPFS node's HTTP gateway. Same files are then
  ed3k and IPFS files at once, one layout (§5.5).

File transfer unchanged. A node without this extension stores, uploads, downloads every file of
such a set.

---

## 1. Parts

| Part | What it is | Speaks |
|---|---|---|
| **Chain** | §4.1 contracts on an EVM chain | Ethereum JSON-RPC |
| **Client** | ed3k node whose user watches or uploads | ed3k; JSON-RPC to a chain node |
| **Site** | catalogue for web browsers: chain indexer, web front end, gateway | HTTP to browsers; JSON-RPC to chain |
| **Site node** | ordinary ed3k node a site runs | ed3k |

Any number of sites read the same chain, show the same videos. A site owns nothing: an upload via
one is found by all.

---

## 2. The upload set

Upload = file set with author signature (`file-set.md` §2.4), `id` and `seq` (`file-set.md`
§5.1). One set = one video. One key may sign many sets: the uploader's **channel key** in ed3k.

### 2.1 Layout

| Path | Content | Rule |
|---|---|---|
| `video/<label>.mp4` | one rendition with main sound track | at least one |
| `audio/<language>.m4a` | further sound track | optional |
| `subs/<language>.vtt` | subtitles, WebVTT, UTF-8 | optional, ≤ 1 MiB each *(initial)* |
| `thumb.jpg` | still picture, JPEG | optional, ≤ 1 MiB *(initial)* |
| `video/<label>.m3u8` | HLS media playlist, byte ranges into `<label>.mp4` beside it | optional, ≤ 1 MiB *(initial)* |
| `audio/<language>.m3u8` | HLS media playlist, byte ranges into `<language>.m4a` beside it | optional, ≤ 1 MiB *(initial)* |
| `subs/<language>.m3u8` | HLS subtitle playlist naming `<language>.vtt` beside it | optional, ≤ 1 MiB *(initial)* |
| `master.m3u8` | HLS multivariant playlist naming the above playlists by relative path: renditions, sound tracks, subtitles | optional, ≤ 1 MiB *(initial)* |

- `<label>` = height in lines + `p`: `2160p`, `1080p`, `720p`, `480p`, `360p`. Reader takes the
  label as what the file is; need not open it to choose.
- `<language>`: BCP 47 tag, lower case: `en`, `de`, `pt-br`.
- Members at other paths: ignored, not downloaded unless user asks.
- A set MUST NOT have a member with a path under `ed3k/`: name taken in a directory holding the
  set (§5.5).
- **All paths relative, nothing made on request** → a plain file server can serve the set as is (web
  server, IPFS gateway, §5.5). Playlists are for web players there; ed3k clients do not need them.
- Playlists name files by relative paths inside the set only. A reader MUST NOT follow an
  absolute URL in one.
- Playlists name set members only: a media playlist the file beside it by bare name,
  `master.m3u8` set playlists by path (`video/<label>.m3u8`, …), as in the manifest: no query,
  fragment, scheme, leading `/`, `.` or `..` segment, nor percent-escape. Reader ignores a
  playlist naming a file not in the set, or one it ignores.
- Upload set: ≤ 64 files *(initial)*, no inner sets.
- Reader ignores, like one at another path: a member over a size limit of this table, one marked
  `set`, one under `ed3k/`. A set of > 64 files is no upload set: video has no media (§3.2).
- Members are ordinary files. A rendition shared by two uploads = one file, one source set.

### 2.2 Formats

Any format may be carried. Baseline = live streams' ([`live.md`](live.md) §2.3), so one player
serves both.

| | Baseline |
|---|---|
| Container | fragmented MP4, `ftyp` and `moov` at file start |
| Video | H.264 (AVC), High profile, up to level 4.1 (1080p at 30 frames per second) |
| Audio | AAC-LC or Opus |
| Fragments | each starts with a key frame; 2 to 6 s long |
| Index | a segment index (`sidx`) after `moov` SHOULD be present |

- **Rendition = one file, not a row of segments.** ed3k verifies any range block by block
  (`Protocol.md` §7.2), so a seeking player asks for the range it needs. Segment files would mean
  thousands of manifest members for nothing.
- Rendition starting with `moov` plays while downloading. Playing client picks blocks per
  `Protocol.md` §12.6: file order, around seek position first.
- §2.1 playlists are written from the segment index: one byte range per fragment. A web player
  without playlist support plays the file itself via range requests.
- Sound track (`audio/<language>.m4a`): same container, fragments, index.
- Other codecs allowed; reader that cannot play a member says so.
- Above level 4.1 = outside baseline: `1080p` at 60 frames per second needs 4.2, `2160p` 5.1, and
  5.2 at 60. A set with one SHOULD also have a baseline rendition. A reader not opening the file
  takes a label ≤ `1080p` for one.
- Who encodes is outside the protocol: uploader's client, or a site on its behalf.

### 2.3 The field in the manifest

```protobuf
// in FileSet (file-set.md §1):
Tube tube = 11;

message Tube {
  uint64 chain = 1;           // chain ID
  bytes  contract = 2;        // 20 B address of the contract that holds the entry: the content contract for a video
  bytes  author = 3;          // 20 B address of the account that publishes the set on the chain
  uint64 content_id = 4;      // ID of the entry at `contract`; 0 = not yet known (the first upload)
}
```

- Points **set → chain**. A client that found the set by search can find its video on chain, and
  so comments and ratings (§4.3).
- **A claim.** True when chain holds a video of `author` at `contract` whose link names this
  set's `key` and `id` (§3.2). `author` = author the contract holds for the video (§4.1); another
  account's video does not match. Client shows nothing from chain until then.
- **Claim covers the set, link one version.** A manifest whose root ≠ that link's `b3` is another
  version, not the video; client says so, offers the version the chain names.
- `tube` naming a chain or contract other than §4.2's: ignored.
- `content_id` = ID the contract assigned. First manifest is written before the transaction,
  carries 0; next version carries the ID. With ID: checked by one `getContentById` (or
  `getCommentById`) call; with 0: by `eth_getLogs` as §4.3, unbounded in chain history; client MAY
  give up on the latter after a bounded number of windows and show the set unverified.
- **After a one-transaction upload, the chain-pinned version carries `content_id` = 0**: its root
  is the link's `b3` (§3.1), and the ID exists only once the link is on chain, so it can't carry it. Verified by log
  search. Author SHOULD follow with an update: new set version carrying the ID, plus `updateMedia`
  with its `b3` (§3.2). Readers then check with one call.
- Set without the field: still a valid upload set, found from chain only.

---

## 3. The link on the chain

### 3.1 Form

Content contract stores per video a list of key/value pairs, `media` (§4.1). Upload = pair with
key `ed3k`:

```
key   = "ed3k"
value = ed3k:set?key=<channel key, base32>&id=<id, base32>&b3=<root, base32>&sz=<size>[&ipfs=<CID>][&dn=<url-encoded name>]
```

- `key`, `id`, `dn` as `file-set.md` §5.4. Base32 written lower case, no padding, read either way
  (`Protocol.md` §13.2); `sz` decimal digits.
- **`b3`, `sz` added by this extension**: root and size of one set version's manifest. **That
  version is the video** (§3.2). Clients without this extension ignore them.
- **`ipfs` also added here**: content ID of an IPFS directory holding the set's files at their
  paths (§5.5). Optional, ≤ 128 bytes, SHOULD be a version 1 CID in base32. It MUST consist only
  of base32 chars (`a`–`z`, `2`–`7`) or only of base58 (`1`–`9`, `A`–`H`, `J`–`N`, `P`–`Z`,
  `a`–`k`, `m`–`z`); a client ignores any other value rather than put it into a gateway URL.
- Value ≤ 512 bytes; longer not read.
- One such pair per video, written at creation → upload is one transaction.
- A reader of this extension ignores every other pair of a video. It MUST NOT fetch a URL from one
  on its own.

### 3.2 Reading it

1. Parse. A value that is no `ed3k:set` link with `key`, `id`, `b3` and `sz` is no upload of this
   extension.
2. Fetch manifest with root `b3`, size `sz`, as a file.
3. Accept only if signed by `key` and carrying `id`; else video has no media, client says so.

- **Link pins the version.** A reader of this extension MUST NOT follow the set's head for a
  video. A version the chain does not name is not the video, whatever its `seq`.
- **New version = transaction.** Author rewrites the pair (`updateMedia`, §4.1) with new `b3`,
  `sz`, `ipfs`; for every upload, site or not. Chain thus shows every version with its time.
- **Transaction is the tie.** Its signer is the chain author (§4.1) and wrote the channel key into
  the link. No signature joins the two keys, none needed: nobody else can write that pair.
- **Channel key alone changes nothing.** Holder can sign a newer version; only the account can
  make it the video. A site holding the key, or a thief, cannot change what a video shows.
- **No head needed.** A head lasts 7 days unless re-stored (`file-set.md` §5.2). A video is found
  by `b3` while its files have sources.
- A client without this extension reads the link as a plain set link and follows the head. So it
  sees the same video, the author and every node of this extension storing a head for the set MUST
  store the chain-named version's, and a newer one only after its transaction is in a block the
  chain treats as final (§4.3). Also on a `SET_HEAD` topic ([`pubsub.md`](pubsub.md) §4.1): a node
  of this extension holds, passes on and gives joining nodes only such a head. A newer head whose
  version no final block names yet is dropped as not newer; no fault of the neighbour.
- One channel key in links of two accounts = both chose it. Nothing more.

### 3.3 Why a set link and not file links

| | One `ed3k:set` link (chosen) | One `ed3k:file` link per rendition |
|---|---|---|
| On chain per upload | one link, ≈ 200 B | one ≈ 100 B link per rendition, sound track, subtitle |
| Committed | everything, by one root | each file alone; nothing joins them |
| Rendition added later | new set version and one transaction | one transaction |
| Paths, so web server / IPFS gateway serves upload as is | yes (§2.1) | none |
| First byte | after manifest, a few hundred bytes | at once |
| Plain web player | needs a gateway to read manifest | could read a `ws` URL in the link |

### 3.4 Attachments of other entries

Contracts carry more than videos (§4.1). A content entry of kind ≠ 0, and a comment on any entry,
MAY carry files: post pictures, thread document, message sound clip. Named like a video, by one
pair with key `ed3k` in the entry's `media`:

```
key   = "ed3k"
value = ed3k:file?b3=<root, base32>&sz=<size>&dn=<url-encoded name>[&ipfs=<CID>]
      | ed3k:set?key=<channel key, base32>&id=<id, base32>&b3=<root, base32>&sz=<size>[&ipfs=<CID>][&dn=<url-encoded name>]
```

| | `ed3k:file` link | `ed3k:set` link |
|---|---|---|
| Names | one file | ≤ 64 files with paths |
| Who made it | not told; account vouches by writing it | channel key, as for a video |
| Fetched before first byte | nothing | the manifest |
| Copy in IPFS | `ipfs`: the file alone | `ipfs`: directory with files and block hashes (§5.5) |
| Verified from gateway | at the end, or block by block with peer hashes | block by block |
| On chain | ≈ 100 B with name, 160 B with CID | ≈ 200 B, 260 B with CID |

- Value ≤ 512 bytes. Longer or other form: not read. Empty value = no attachments.
- One such pair per entry. Both contracts keep a key once: rewriting replaces the old value.
- **On a kind 0 entry the pair is the §3.1 upload**, never an attachment. Further video files are
  set members.
- **`ed3k:file` link**: that of `Protocol.md` §13.2 with `b3`, `sz`, `dn`; MAY have `ipfs`
  (below). Reader ignores every other parameter, and MUST NOT fetch from a `ws`, `wh` or `hl` in
  it: a stranger wrote the link, the user did not choose it. A manifest named this way is a file
  like any other (`file-set.md` §2.3).
- **`ed3k:set` link**: §3.1 form, read as §3.2: without `key`, `id`, `b3`, `sz` not read; manifest
  accepted only if signed by `key` and carrying `id`; link pins the version. No head followed.

**`ipfs` in a file link**

**This extension adds `ipfs` to `ed3k:file` links.** Content ID of the file itself in IPFS, not a
directory. Rules as §3.1: optional, ≤ 128 bytes, a version 1 CID in base32 SHOULD be used; §3.1
character rule applies.

```
<gateway>/ipfs/<CID>
```

- Bytes behind the CID = byte for byte the file with root `b3`. Neither name derives from the
  other; CID is a place, not an identity (§5.5).
- Client builds the URL for user-configured gateways, ≤ 3 per file *(initial)*, as plain HTTP
  source of the whole file. §5.5 gateway rules hold: none built in, `http-source.md` §5 checks
  apply, `Range` SHOULD be answered.
- **No hash body.** One-block file needs none. Larger: verified block by block with peer hashes,
  or whole when complete (`http-source.md` §3). A file then failing cannot be repaired from the
  gateway; source dropped.
- So file-link `ipfs` is for small files (picture, document). An author wanting a large file
  verified while arriving from a gateway alone writes a set link, whose directory has block
  hashes (§5.5).
- CID naming a directory, or other bytes: fails under the root, dropped.
- Clients without this extension ignore the parameter. Same meaning in an `ed3k:file` link off
  chain.
- A web player without ed3k shows `<gateway>/ipfs/<CID>` and verifies nothing, as §5.5.
- A site SHOULD check the CID is fetchable before writing it to chain.
- A picture fetched on its own (below) MAY come from a gateway. Limits stay.

**The attachment set**

- ≤ 64 files *(initial)*, no inner sets. Member marked `set` ignored. No member path under `ed3k/`
  (§2.1); reader ignores one.
- **Paths are free.** Each member = one attachment, shown under its path.
- **Set with §2.1 layout** (≥ one `video/<label>.mp4`): played as a video, other members read per
  §2.1. Still an attachment: entry not kind 0, in no video list (§4.3).
- MAY carry `tube` (§2.3). `contract` = contract holding the entry: content contract, or comment
  contract for a comment. Claim checked as §2.3, entry in place of video.
- Any format allowed. What a client shows of a member: below.

**Changing**

- Content-entry author rewrites the pair with `updateMedia` (§4.1). Pairs cannot be removed; empty
  value stands for that.
- Comment author calls `updateComment`, replacing text and whole list together; client resends the
  text with it.
- New attachment-set version = transaction, as §3.2.

**Shown in place**

**Shown in place** = client shows/plays it inside its entry with its own decoder. Any other
attachment is a line with name and size, and a file once the user downloaded it.

| Type | Known by | In place |
|---|---|---|
| Picture | first bytes of a JPEG, PNG, GIF or WebP file | shown |
| Sound | MP4 (`ftyp` at start), one sound track, no video track, AAC-LC or Opus | player; starts when user asks |
| Video | MP4 with video track in §2.2 baseline | player; starts when user asks |
| Set with §2.1 layout | its paths (above) | one player for whole set, as for a video |
| Anything else | | never |

- **Bytes say what a file is**; name, path, ending say nothing. First bytes of no table type →
  "anything else", whatever it is called.
- A client SHOULD show pictures and SHOULD play the two baselines. It MAY play more sound/video
  formats. It MUST NOT show in place a format able to carry scripts, followed links or references
  to other files: SVG, HTML, PDF among them.
- **Only verified bytes decoded.** A block is shown/played after passing under the root. A file
  from a gateway without block hashes (`ipfs` in a file link) is shown when complete and verified.
- Player asks for blocks per §2.2 and `Protocol.md` §12.6: file order, around seek position first.
  Sound/video never fetched before the user asks; the user's leave to fetch pictures unasked
  (below) does not cover them.
- Client refuses a picture > 16,384 pixels a side or 64 million pixels total *(initial)*.
  Animation plays when user asks, or under the picture leave.
- Text inside a file (title, comment, place) is stranger-written (§4.3). Client follows no link in
  it, fetches nothing a file names.
- A set's attachments shown in manifest order.
- A site's web front end decides the same for its browser; outside this protocol.

**What a client does**

- Shows name and size from a file link, name from a set link, without fetching. Both
  stranger-written (§4.3).
- **No attachment, nor its manifest, fetched before user asks.** If the user allowed it, client
  MAY fetch on its own for entries it shows: manifests ≤ 64 KiB, pictures ≤ 1 MiB, 4 pictures per
  entry *(all initial)*. Larger picture = name line until user asks.
- Never hands an attachment to another program on its own.
- Attachments of an entry with `blocked` set: not shown or fetched unless user asks.
- Before downloading an attachment known from a site only (§4.5), reads the entry from chain:
  `getContentById`, or `getCommentById`.

---

## 4. The chain

### 4.1 Contracts

Four contracts, plus a forwarder (`TubeForwarder`) for site-sent requests (§10). Sources are part
of the ChainTube backend.

| Contract | Holds | Events |
|---|---|---|
| Content | per entry: author, kind, parent, reference, title, one list of pairs (`media`), tags, a text, `closed`, `blocked` | `ContentSet(uint256 contentId, address indexed author, uint256 indexed parentId, uint32 indexed kind, uint256 refId, string title, (string,string)[] media, string[] tags, string content, bool closed, bool blocked)`, `ModeratorSet(uint256 indexed contentId, address indexed moderator, bool enabled)` |
| Comment | per comment: author, content contract, content ID, comment it answers, text, a list of pairs (`media`), `blocked` | `CommentSet(uint256 commentId, address indexed author, address indexed contractHash, uint256 indexed contentId, uint256 replyId, string content, (string,string)[] media, bool blocked)` |
| Vote | per vote: author, contract and ID of what is rated, type, value | `VoteSet(uint256 voteId, address indexed author, address indexed contractHash, uint256 indexed contentId, int256 voteType, uint256 voteValue)` |
| Profile | per account: unique name, alias, avatar, text. Follows, as events only | `UserSet(address indexed userAddress, uint256 userId, string userName, string userAlias, string userAvatar, string userBio)`, `FollowSet(address indexed follower, address indexed target, uint256 indexed targetId, bool following)` |

- **More than videos.** Content entry `kind`: 0 video, 1 article, 2 board, 3 forum thread, 4 short
  message. `parentId` = containing entry, `refId` = quoted entry. **A video of this extension is an
  entry of kind 0.** For other kinds this extension only says how attachments are named (§3.4);
  meaning in backend's `docs/protocol.md`.
- **Author** of an entry = account the contract holds: call's sender, or signer of a site-sent
  request (§10).
- **Every event carries the item's whole state**, new or changed. Newest event of an ID = the
  item; within a block the later log is newer.
- IDs count from 1 per contract. A video = chain ID, content contract and content ID.
- A vote names its target by contract and ID: content contract for a video, comment contract for a
  comment.
- `voteType` > 0 up, < 0 down, 0 plain like: neither, value still counted. `voteValue` = total
  chain coin the voter paid, after any `updateVote`: `updateVote` adds its value, replaces
  `voteType`. Contract refuses it once `voteUpdatePeriod` seconds passed since `castVote`
  (0 = never); `getVoteById` returns `castVote` time as `created`. Contract splits the value among
  author, video creator, contract operator, and a wallet the transaction sender names.
- A video cannot be deleted. Author can change title, pairs, tags, text. Kind, parent, reference
  fixed at creation.
- **Moderators** of an entry: its author, accounts the author named, moderators of its parents. A
  parent's moderator can set `blocked` on an entry. An entry's moderator can set `closed` on it and
  `blocked` on its comments. A parentless video has nobody above: only its comments can be
  blocked, by its moderators.

| Use | Function |
|---|---|
| Upload | `createContent(uint32 kind, uint256 parentId, uint256 refId, string title, (string,string)[] media, string[] tags, string content)` with `kind` = 0, `parentId` and `refId` = 0 unless video is in a board or quotes an entry, `media` = one pair of `ed3k` and the link of §3.1. Other kinds: `media` empty, or one pair of `ed3k` and a §3.4 link |
| Rewrite the link | `updateMedia(uint256 contentId, string key, string value)` with `key` = `ed3k` |
| Comment | `createComment(address contractHash, uint256 contentId, uint256 replyId, string content, (string,string)[] media)`; `replyId` 0 = not an answer; `media` empty, or one pair of `ed3k` and a §3.4 link |
| Change a comment | `updateComment(uint256 commentId, string content, (string,string)[] media)`; both replaced, `blocked` kept |
| Rate | `castVote(address contractHash, uint256 contentId, int256 voteType, address platformWallet)`, value = transaction amount |
| Change a vote | `updateVote(uint256 voteId, int256 voteType, address platformWallet)`, added value = transaction amount |
| Read one video or other entry | `getContentById(uint256 contentId)` |
| Read one comment | `getCommentById(uint256 commentId)` |
| Read one vote | `getVoteById(uint256 voteId)` |
| Item counts | `totalCount()`, `totalComments()`, `totalVotes()` |
| Name a moderator | `setModerator(uint256 contentId, address moderator, bool enabled)`, by entry author; counts for entry and all below |
| May this account comment here | content contract `canPost(uint256 contentId, address account)`: entry exists, not `blocked`, no `blocked` entry above, not `closed` unless account is moderator |
| Is a profile needed | comment contract `requireProfile()`, `profileContract()`; then that profile contract's `getUserByAddress(address)`, `userId` 0 = no profile |
| How long a vote can be changed | vote contract `voteUpdatePeriod()`, seconds; 0 = always |
| Forward a site's request | forwarder `execute`; `getNonce(address from)` reads an account's next nonce |

ABI published with the ChainTube backend (`github.com/ChainTube/stream-daemon`,
`chain/contracts/abi/`); signatures above taken from it. Getters return values flat, not as a
struct, in this order:

| Getter | Returns |
|---|---|
| `getContentById` | `address author, uint32 kind, uint256 parentId, uint256 refId, string title, (string,string)[] media, string[] tags, string content, bool closed, bool blocked` |
| `getCommentById` | `address author, address contractHash, uint256 contentId, uint256 replyId, string content, (string,string)[] media, bool blocked` |
| `getVoteById` | `address author, address contractHash, uint256 contentId, int256 voteType, uint256 voteValue, uint64 created` (block time of the `castVote`) |
| `getUserByAddress` | `uint256 userId, string userName, string userAlias, string userAvatar, string userBio` |

Contract addresses are assigned with the parameters of §4.2.

### 4.2 Parameters of a network

| Parameter | Public network |
|---|---|
| Chain ID | *to be assigned at deployment* |
| Addresses of the four contracts and the forwarder | *to be assigned at deployment* |
| RPC endpoints | list in client config; none built into the protocol |

A named network (`Protocol.md` §4.3) has its own contracts or none.

Test deployment on Avalanche Fuji (chain ID 43113): content
`0x8cF7928c9d3aDeC175ea7f3e919A681b06Ba3ED8`, comment
`0x6959AD50f2Ed0601227Bd2a966F9e9b2B313D2Bb`, vote
`0x39f2e54075073b5Ad934CdB39E3410AbC3e540cc`, no profile contract. Addresses per backend config of
2026-10-12. **Code there is older than §4.1** (three pair lists, no kind): contracts changed
2026-10-13, not redeployed yet.

### 4.3 Reading

**The chain is the rule.** Client reads via a chain node's JSON-RPC, as the credit extension
([`credit.md`](credit.md) §7.3).

| Wanted | Read |
|---|---|
| Right chain? | `eth_chainId` |
| One video | `eth_call` of `getContentById` |
| An account's videos | `eth_getLogs` at content contract, topics `author` and `kind` = 0 |
| A set's video | `eth_call` of `getContentById(tube.content_id)` when id set, checking `author` and link; else `eth_getLogs` at content contract, topics `author` = `tube.author` and `kind` = 0; then the video whose link names the set's `key` and `id` |
| An attachment set's entry | `eth_call` of `getContentById(tube.content_id)` or `getCommentById(tube.content_id)` when id set; else `eth_getLogs` at `tube.contract`, topic `author` = `tube.author` (both contracts index it, §4.1); then the entry whose link names the set's `key` and `id` |
| A video's comments | `eth_getLogs` at comment contract, topics `contractHash` and `contentId` |
| Votes on a video or comment | `eth_getLogs` at vote contract, same two topics |
| New videos of all accounts | `eth_getLogs` at content contract, topic `kind` = 0, backwards from newest block |

- ≤ 2,048 blocks *(initial)* per `eth_getLogs`; read at a block the chain treats as final.
- Client using a remote node SHOULD ask two independent providers for a read it acts on, and
  SHOULD send RPC via the user-configured proxy, if any.
- Local chain node: nothing more needed.

**What a client makes of it**

- **All chain data is stranger-written text.** Title, tag, text, comment, alias, name, file name in
  a link: shown as plain text, never markup; no link in them followed on the client's own.
- Shown at most *(all initial)*: 256 bytes of a title, 64 bytes per tag, 16 tags, 8,192 bytes of a
  video's or comment's text. Contracts set no limits.
- Comment with `blocked` set: hidden unless user asks. Only the video's moderators can set it
  (§4.1); a later change by its author keeps it.
- Video with `blocked` set: hidden unless user asks. Video with `closed` set takes comments from
  its moderators only; comment contract refuses others.
- Contracts refuse a comment or vote on a nonexistent video, and an answer to another video's
  comment. Comment contract also refuses a comment on an entry that is `blocked` or has a
  `blocked` entry above (≤ 8 levels, content contract's max depth), and, while `requireProfile` is
  set, from an account without profile in the profile contract. Client reads `canPost` and the
  profile (§4.1) before sending a comment. A comment names the content contract; a vote names the
  content contract for an entry, comment contract for a comment. Client ignores one naming another
  contract.
- An account can vote on one thing many times. A client SHOULD count one vote per account and
  thing, the newest, and SHOULD show the sum of values (newest `voteValue` per vote ID) apart from
  the count. A plain like (`voteType` 0) counts in the sum, as neither up nor down.
- Avatar used if it is an `ed3k:file` link to a picture ≤ 1 MiB *(initial)*.

### 4.4 Writing

- Transactions signed by a user account: in a wallet, or a client-held key. Account pays gas,
  unless a site pays for it (§10).
- **The key SHOULD NOT be the credit key** ([`credit.md`](credit.md) §2). An uploading, commenting
  account is a public person; a node's credit address is meant not to be.
- Client-held key is as private as the node key. A client MUST NOT write it to a log or show it
  in an error message.
- **Write done** when a read at a final block shows it: `getContentById`, `getCommentById` or
  `getVoteById` returns the new state, written values compared in full (`closed`, `blocked` not
  compared). `updateVote` adds to a sum: client reads `getVoteById` at the newest block before
  sending; vote done when `voteType` = the one written and `voteValue` ≥ that total + added value.
  No `updateVote` sent when `created` + `voteUpdatePeriod` (unless 0) has passed by the newest
  block's `timestamp`. A client SHOULD also run a transaction through `eth_estimateGas` at the
  newest block before showing the amount (below), and sends none that reverts there.
- **`setModerator`** has no own getter: `isModerator` is true via parents too. Done when its
  `ModeratorSet` log, `contentId`, `moderator`, `enabled` as written, is in the receipt at a final
  block; for a site-sent request (§10), when the newest `ModeratorSet` log of that `contentId` and
  `moderator` from the hand-over block has `enabled` as written. Same for the client's own
  transaction and one a site sent for it (§10).
- **Failed write.** Receipt at a final block with status 0 = reverted: not done, reported once. A
  site request via the forwarder (§10) leaves no trace on revert; forwarder keeps the nonce, request
  stays valid until its `deadline`. When a final block with `timestamp` later than `deadline` does
  not show the write, it is not done; `getNonce(from)` still at the request's `nonce` at that block
  shows it never executed. `from` = author.
- New item's ID is in its event (`ContentSet`, `CommentSet`, `VoteSet`): in the client's
  transaction receipt, or, if site-sent, via `eth_getLogs` at the contract with topic `author`
  (§4.3) from the hand-over block, as the newest item showing what was written. A site-given ID is
  a shortcut: counts when the getter shows what was written and, for a new item, the ID exceeds the
  contract's count (`totalCount`, `totalComments`, `totalVotes`) at the hand-over block. Else
  dropped, logs searched.
- Client sends a transaction only when the user asked for this one; shows amount and receiver
  contract first.
- `platformWallet` = zero address unless user chose a site to support.
- A client that only makes the set and shows the §3.1 link needs no key: user pastes the link into
  a site's upload form, signs there.

### 4.5 A site's list as a shortcut

Reading a year of logs from a remote node is slow; a site has done it and answers lists and
searches at once. Its interface is its own, not part of this protocol.

- A client MAY ask user-configured sites for lists of videos, comments, votes.
- **Before downloading a video's media a client MUST read that video from the chain**
  (`getContentById`) and take the link from there.
- A client SHOULD mark comments and ratings it has from a site only, and reads them from chain when
  the user is about to pay or answer.
- What a site can do to such a client: omit things, show comments or counts not on chain until the
  client reads them itself. Cannot make it download media other than the author named.
- A site sees the client's address and requests.

---

## 5. A site

### 5.1 Not a role in ed3k

A site is its own program, invisible to ed3k. In ed3k only the **site node** takes part: an
ordinary node with any node's duties and limits.

| | Own program, node beside it (chosen) | Site as index node with own messages |
|---|---|---|
| ed3k | nothing added | record kinds or messages for videos, comments, votes |
| Truth | one: the chain | two that can differ: chain and node's answers |
| Web browsers | need HTTP either way; site has it | same, so HTTP part exists anyway |
| Site's program | reads a chain, serves a web site; no ed3k code | must carry a whole ed3k node |
| Found by | a link, a list, user's choice | Kad, like any index node |
| ed3k client without chain access | finds videos by keyword only if site node publishes them (§5.4) | could be served everything, on the site's word |

An index node (`Protocol.md` §11) stores and answers Kad records for every target: records, not a
catalogue. A site node MAY be one too (§5.4); then it does what every index node does, nothing
else.

### 5.2 What a site node does

- **Seeds.** Reads from chain the videos its site shows, downloads members of the version each link
  names, shares them, by its own policy on which and how long. It MAY do the same for attachments
  (§3.4).
- **It MAY keep heads alive** for clients reading the link as plain set link: the chain-named
  version's head, no other (§3.2).
- **Names its gateway as HTTP source.** A site serving files over HTTP for web players is a plain
  source ([`http-source.md`](http-source.md) §1). Site node hands that URL out in the file stream
  and beside its `SOURCE` record, per that document. ed3k clients then fetch from gateway and peers
  at once, verifying every byte under the root.
- A site node SHOULD be a `ROUTER`, reachable without signalling.
- A site keeping media in IPFS does all this with its IPFS node's gateway (§5.5).

None of this is required for a video to exist; an upload seeded by its author alone is valid.

### 5.3 Uploads through a site

Browsers cannot speak ed3k. A visitor without ed3k client hands the file to a site, which encodes
it, makes the set, has its node share it. How the file gets there is the site's business.

The two ways differ in who holds the channel key:

| | Visitor holds channel key | Site holds it |
|---|---|---|
| Who signs the manifest | visitor's browser or client, over the body the site prepared | the site |
| Site can sign newer set version | no | yes; it is the video only after the visitor's transaction (§3.2) |
| Visitor can move to another site | yes, with key and files | only by new upload under new key |
| Needs | key kept by visitor beside the wallet | nothing from visitor |

- Either way the visitor's account signs the transaction and the link names the key. Visitor's
  wallet sends it, or visitor signs and site sends. A sending site cannot change it, only hold it
  back.
- **New version = transaction** (§3.2), signed and sent like the first. A site cannot change a
  video without the account's signature, even holding the channel key.
- A site holding channel keys MUST keep one per account and MUST NOT sign one account's set with
  another's key.
- A site SHOULD say which way it does.

### 5.4 The site node as an index node

A site node MAY set `INDEX`, and SHOULD publish a keyword record (`Protocol.md` §10.3) for the
manifest of each video it seeds, so videos are found by ed3k search, without chain.

- Manifest shared as `<title>.ed3kset`, chain title cut to what a name holds (`file-set.md` §2.1).
  Keywords = those of that name, as any file. Tags are not keywords.
- Record is the site node's, signed by its key like any keyword record. **That node's claim**: a
  file with this name and root exists. Says nothing of the chain.
- A client getting the manifest via such a search reads `tube` (§2.3) and checks it on chain before
  showing author, comment or rating.
- As index node it accepts records for every target from everyone, under its own admission policy
  (`Protocol.md` §11). A site not wanting that load leaves `INDEX` unset and publishes keyword
  records in Kad.
- Clients find a site node like any index node: from a list, or an `ed3k:node` link (`Protocol.md`
  §13.2) the site shows.

### 5.5 Media kept in IPFS

A site MAY store uploads in IPFS and serve them over HTTP from its IPFS node's gateway. ed3k
unchanged: **a file behind an IPFS gateway is a plain HTTP source**
([`http-source.md`](http-source.md) §1), and a set laid out per §2.1 is an IPFS directory as is.

**The directory**

```
<CID>/video/720p.mp4            every member of the set at its path
<CID>/video/720p.m3u8
<CID>/master.m3u8
<CID>/thumb.jpg
<CID>/ed3k/set                  the manifest of the set
<CID>/ed3k/<root, base32>       the hash body of the member with that root (http-source.md §3.1)
```

`<root, base32>` = root in base32, lower case, no padding, as `Protocol.md` §13.2 writes it in
links; gateway paths are case-sensitive, so no other spelling is a name.

- `ed3k/` is not part of the set: it is what an ed3k client needs to use the directory without a
  peer: manifest, plus block hashes to verify a member while it arrives. One-block member: no hash
  body.
- Directory files = set members, byte for byte. Site hashes each once more with BLAKE3; IPFS
  neither needs nor sees that root.
- Newer set version = new directory, new CID. Unchanged files keep their CIDs inside, nothing stored
  twice. New CID goes on chain with new `b3` (§3.2).
- Name `ed3k/` is taken: no upload-set member path under it (§2.1).
- No leading dot on purpose: tools adding a directory to IPFS silently skip dot-names unless told
  otherwise.
- Directory without `ed3k/` plays in a web player, useless to an ed3k client. A site SHOULD check
  `<CID>/ed3k/set` is fetchable before writing the CID to chain.

**How the two names meet**

| Name | Where | Commits to |
|---|---|---|
| root of the manifest (`b3`) | the link on the chain | every member, by its root |
| CID of the directory (`ipfs`) | the link on the chain | every file, by IPFS's own tree |

- Both written by author in one transaction. Neither derives from the other; ed3k never asks for a
  file by CID (`Protocol.md` §7.4).
- **To an ed3k client the CID is a place, not an identity**: where bytes may be fetched. Bytes count
  when they pass under the root, as from any source.
- CID holding other bytes, or another set version: costs failed fetches, nothing else; source
  dropped per `http-source.md` §6.

**Fetching**

A member's URL is

```
<gateway>/ipfs/<CID>/<path of the member, each component percent-encoded>
```

and its `hashes` URL is `<gateway>/ipfs/<CID>/ed3k/<root, base32 lower case, no padding>`.

- **Site node hands these URLs out** as HTTP sources per member (§5.2), with its own gateway. Works
  for every client with `HTTP` capability; needs no `ipfs` in the link.
- URL of an attachment a file link names: `<gateway>/ipfs/<CID>` alone, no hash body (§3.4).
- **With `ipfs` in the link, client can build them itself**, for user-configured gateways: own IPFS
  node, or a gateway of its choice. No site or peer needed. ≤ 3 gateways per video *(initial)*.
- No gateway built into the protocol. A client MUST NOT send a request to a gateway named neither
  by the user nor by a source of the file.
- `http-source.md` §3 and §5 checks, limits, redirects apply. A gateway redirecting to its own host
  name for the CID is followed like any redirect. A gateway on the user's own machine is allowed
  because the user configured it.
- **That exception is narrow.** Lifts the `http-source.md` §5 address check only for the gateway's
  address and port as user-configured, and for a redirect whose target resolves to that same
  address and port, e.g. `<CID>.ipfs.localhost` on a local gateway. Nothing else: a gateway redirect
  to any other non-global-unicast address is refused, as is such an address in a gateway a source
  named.
- A site's gateway SHOULD answer `Range` requests on files; player seeking and multi-source fetching
  need it. One that does not is read from the start or not used (`http-source.md` §3).
- A client with its own IPFS node MAY fetch the files through it by any means that node offers;
  between client and IPFS, outside this protocol.

**A web player without ed3k** reads the link from chain, plays `<gateway>/ipfs/<CID>/master.m3u8`
or a rendition directly. Verifies nothing under the root; as true as its gateway: site's own IPFS
node checks blocks against the CID, a foreign gateway is believed.

**What the site node shares.** An ed3k source for the same files. It SHOULD read them from the IPFS
node's store, not a second copy, and verifies them under their roots before serving a block, as
always.

---

## 6. What an attacker can do

| Attack | What limits it |
|---|---|
| **A false `tube` field**: set claiming an author it lacks | claim checked on chain (§2.3); nothing shown before |
| **A link to someone else's set** in an own video | set keeps its key; client shows the chain account as video author. Copies as possible as on any video site |
| **A site lists what is not on the chain** | client reads the video from chain before downloading (§4.5) |
| **A site holding a channel key, or a thief of one, changes a video** | cannot: reader takes the chain-named version, follows no head (§3.2) |
| **A remote RPC node lies** | two independent providers (§4.3). Cannot sign for anyone |
| **A false `ipfs` in a link**, or a gateway sending other bytes | bytes fail under root; source dropped. Web player without ed3k has no such check (§5.5) |
| **A link names a gateway inside the reader's network** | links name a CID, never a host; gateways are the user's or come with a source and pass `http-source.md` §5 address checks |
| **Long strings on the chain** | display limits (§4.3), 512 bytes per link (§3.1) |
| **Markup or links in a title or comment** | shown as plain text |
| **An attachment seeded only by its writer**, to learn who reads a thread | nothing fetched before user asks; pictures on their own only if allowed (§3.4). Rest in §7 |
| **A URL in an attachment link** | `ws`, `wh`, `hl` in a chain link unused (§3.4) |
| **A harmful file as attachment** | nothing handed to another program on its own. Decodes only pictures, sound, video, known by bytes, verified first, no script-carrying format (§3.4) |
| **A picture small as file, huge decoded** | pixel limit (§3.4) |
| **Many accounts vote or comment** | each costs gas; counting is the client's (§4.3). Nothing in ed3k depends on a count |
| **A huge set** | limits of `file-set.md` §2.1 and 64 files (§2.1); client downloads the user-chosen rendition, not the set |

## 7. What it reveals

- An upload joins a chain account to a channel key, publicly, for good.
- A node seeding its own upload is a source. Whoever asks for sources learns a holding node's
  address, as for any file. An uploader not wanting its address beside its account lets a site node
  seed and stops, or uses an anonymous path ([`hops.md`](hops.md)).
- A gateway sees fetcher address and file. A client SHOULD use the user-configured proxy for it, as
  for any HTTP source.
- An attachment's sources see who fetches it, as for any file. A commenter who is its attachment's only
  source learns fetching readers' addresses.
- A chain node sees which videos a client reads about; a site, the same for what it is asked.
- Comments and votes are public and cannot be taken back.

## 8. What it costs

| | |
|---|---|
| ed3k wire | nothing: no message, record kind, capability bit |
| A manifest | ≈ 50 B for `tube` |
| An upload | one transaction, link ≈ 200 B, 260 B with CID |
| Attachments | one link in their entry's transaction: ≈ 100 B per file, 160 B with CID; set as above |
| Kad | any set's records: sources per member, a head where kept (§3.2), keyword records where a site node publishes them |
| A viewer | one chain read, one manifest, then the played rendition |

---

## 9. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **The site as an index node with messages for videos, comments and votes** | one program; ed3k client needs no chain | second catalogue beside chain (§5.1) |
| **Comments and votes as signed Kad records too** | readable without chain | two stores that can differ; votes carry payment only the chain enforces |
| **An `ALIAS` scheme from a video ID to the root** ([`alias.md`](alias.md)) | video found in Kad by chain name | ID holder can ask the chain, the judge anyway; a record only the chain can check adds a lookup, no certainty |
| **A signature between the account and the channel key** | set proves author without chain read | new signed object and wallet prompt; a copy proves nothing unless also on chain |
| **The credit key as the account** | one key for both | joins node credit address to a public person (§4.4) |
| **A link without `b3`** | shorter; head alone decides | video lost when head expires everywhere; channel key alone can change it |
| **`b3` as a fallback, the head followed when it is newer** | author seeding alone adds a rendition fee-free | channel key holder changes a video with no chain trace; a site rule no reader can check |
| **A `pin` flag in the link**, the head followed without it | both above, author's choice | two link readings; viewer has to be shown which a video uses |
| **The CID as the identity of the video**, with no root | one name; what an IPFS site has anyway | second identity ed3k cannot verify block by block from a peer; rule 1 of [`alias.md`](alias.md) §5 |
| **An `ALIAS` scheme from a CID to the root** | video found in ed3k by CID | claim checkable only by rebuilding IPFS's tree with publisher's chunk/layout settings; chain link names both already |
| **The CID in the manifest** | one link parameter less | directory holds the manifest, so manifest cannot hold the directory CID |
| **A gateway URL in the link** | client needs no setting | host name on chain for good; a link could point readers at any host |
| **Segments as members** (the layout of HLS) | what web players expect | thousands of members per video; ranges of one file do the same (§2.2) |
| **Renditions in the pairs of the chain** | readable without manifest | a link per file on chain, nothing joins them (§3.3) |
| **An attachment layout with fixed directories**, as §2.1 has | reader knows member from path | attachments are any kind; bytes say what a file is; videos among them have §2.1 already |
| **The CID in a file link names a directory** with the file and `ed3k/<root>` | block hashes for a single file from a gateway | link has to give the file's path in the directory; directory with files and hashes = what a set link names already |
| **A second CID in a file link for the hash body** | same, no directory | 65 B more on chain per attachment; small files need no hashes, large have the set link |
| **A type next to the link on the chain** (a pair `type`, or a parameter) | reader knows attachment before fetch | a claim the bytes decide anyway; a believing reader would decode what a stranger chose |
| **Attachments by `ed3k:file` links only**, a manifest among them | one link form | no author key, no pinned one-name set, no place for block hashes beside a CID |

## 10. Open points

- **A site registry.** Sites are configured or found by links. A chain list would let clients find
  them, and need moderating.
- **Live streams of a channel.** [`live.md`](live.md) channels have their own key. A pair naming an
  `ed3k:live` link would join one to an account.
- **Following an account.** Profile contract logs `FollowSet` for an account, board or thread.
  Public statement, no delivery: client still follows an account's logs, or a channel key's sets via
  [`pubsub.md`](pubsub.md).
- **Tips and credits.** Votes carry chain-coin payment; credits ([`credit.md`](credit.md)) live on
  another chain. Nothing joins them.
- **The contracts are a draft.** No profile contract deployed. Contract changes change §4.1.
- A site not using IPFS serves files under its own paths. §2.1 layout under one base URL would let
  a web front end use any site's server; no link parameter names such a base today.
- How fast common IPFS nodes' gateways answer `Range` on large files: not measured.
- **Who pays the fee** is outside this extension. Contracts take a call from the account itself, or
  an account-signed request (EIP-712) a site sends through a forwarder contract (ERC-2771) and pays
  for. Chain author = the account either way, so §3.2 holds unchanged. Request form in backend's
  `docs/protocol.md`.
- All numbers are first values.
