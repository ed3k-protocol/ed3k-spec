# Extension: videos named on a chain (`chaintube`)

**Status:** draft 0.3 (2026-10-13). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §2.4 and §5, [`http-source.md`](http-source.md) §2, and [`Protocol.md`](../Protocol.md) §11 and §13.2. Section numbers (§) refer to this document unless a document is named.
**Capability:** none (`SETS`). No message types and no record kinds; one field in a manifest, three parameters in `ed3k:set` links, a layout for a set, and reads and transactions on a chain outside ed3k.

ChainTube is a video site whose catalogue is a set of contracts on an EVM chain. An upload, a
comment and a rating are transactions, signed by the visitor's own account. The chain holds the
text and, for each video, a string that says where the media is. It holds no media.

This extension makes ed3k the place where the media is.

- An upload is a **signed file set** ([`file-set.md`](file-set.md)): the renditions of the
  video, further sound tracks, subtitles and a picture (§2).
- The chain holds one **`ed3k:set` link** per video (§3). The account that sent the transaction
  is the author, and the link it wrote names the key of the set. That is the whole tie between
  the two identities.
- **Comments, ratings and profiles stay on the chain.** A client reads them from a node of the
  chain. Nothing of them is put into an ed3k record (§4).
- A **site** is a separate program next to an ordinary ed3k node. It is no new role in ed3k
  (§5).
- A site MAY keep the media in **IPFS** and serve it from the HTTP gateway of its IPFS node.
  The same files are then ed3k files and IPFS files at once, with one layout (§5.5).

The file transfer is unchanged. A node without this extension stores, uploads and downloads
every file of such a set.

---

## 1. Parts

| Part | What it is | Speaks |
|---|---|---|
| **Chain** | the contracts of §4.1 on an EVM chain | Ethereum JSON-RPC |
| **Client** | an ed3k node whose user watches or uploads | ed3k; JSON-RPC to a node of the chain |
| **Site** | a catalogue for web browsers: an indexer of the chain, a web front end, a gateway | HTTP to browsers; JSON-RPC to the chain |
| **Site node** | an ordinary ed3k node that a site runs | ed3k |

Any number of sites read the same chain and show the same videos. A site owns nothing: an
upload made through one is found by all.

---

## 2. The upload set

An upload is a file set with an author signature (`file-set.md` §2.4), an `id` and a `seq`
(`file-set.md` §5.1). One set is one video. One key may sign many sets; it is the uploader's
**channel key** in ed3k.

### 2.1 Layout

| Path | Content | Rule |
|---|---|---|
| `video/<label>.mp4` | one rendition of the video with its main sound track | at least one |
| `audio/<language>.m4a` | a further sound track | optional |
| `subs/<language>.vtt` | subtitles, WebVTT in UTF-8 | optional, at most 1 MiB each *(initial)* |
| `thumb.jpg` | a still picture, JPEG | optional, at most 1 MiB *(initial)* |
| `video/<label>.m3u8` | an HLS media playlist with byte ranges into `<label>.mp4` beside it | optional, at most 1 MiB *(initial)* |
| `master.m3u8` | an HLS multivariant playlist that names the playlists above, the sound tracks and the subtitles by relative paths | optional |

- `<label>` is the height in lines followed by `p`: `2160p`, `1080p`, `720p`, `480p`, `360p`.
  A reader takes the label for what the file is and need not open the file to choose.
- `<language>` is a language tag of BCP 47 in lower case: `en`, `de`, `pt-br`.
- A reader ignores members at other paths. It does not download them unless the user asks.
- A set MUST NOT have a member with a path under `ed3k/`. That name is taken in a directory
  that holds the set (§5.5).
- **Every path is relative and nothing is made on request.** The set can therefore be served
  as it is by a server that only hands out files: a plain web server, or the gateway of an
  IPFS node (§5.5). The playlists are for web players there; an ed3k client does not need
  them.
- A playlist names files by relative paths inside the set only. A reader MUST NOT follow an
  absolute URL in one.
- An upload set has at most 64 files *(initial)* and no inner sets.
- A member is an ordinary file. A rendition that two uploads share is one file with one set of
  sources.

### 2.2 Formats

The protocol carries any format. The baseline is the one of live streams
([`live.md`](live.md) §2.3), so that one player serves both.

| | Baseline |
|---|---|
| Container | fragmented MP4 with `ftyp` and `moov` at the start of the file |
| Video | H.264 (AVC), High profile, up to level 4.1 |
| Audio | AAC-LC or Opus |
| Fragments | each starts with a key frame; 2 to 6 s long |
| Index | a segment index (`sidx`) after `moov` SHOULD be present |

- **A rendition is one file, not a row of segments.** ed3k verifies any range of a file block
  by block (`Protocol.md` §7.2), so a player that seeks asks for the range it needs. Segments
  as separate files would give a manifest of thousands of members and nothing in return.
- A rendition that starts with `moov` plays while it downloads. A client that plays SHOULD
  request the blocks in file order, and the blocks around a seek position first.
- The segment index is what the playlist of §2.1 is written from: one byte range per
  fragment. A web player that knows no playlists plays the file itself with range requests.
- Other codecs are allowed. A reader that cannot play a member says so.
- Who encodes is not part of the protocol: the uploader's client, or a site on its behalf.

### 2.3 The field in the manifest

```protobuf
// in FileSet (file-set.md §1):
Tube tube = 11;

message Tube {
  uint64 chain = 1;           // chain ID
  bytes  contract = 2;        // 20 B address of the content contract
  bytes  author = 3;          // 20 B address of the account that publishes the set on the chain
}
```

- The field goes **from the set to the chain**. A client that found the set by a search can
  look for its video on the chain, and so for its comments and ratings (§4.3).
- **It is a claim.** It is true when the chain holds a video of `author` at `contract` whose
  link names this set (§3.2). `author` is the account that sent the transaction that created
  the video; a video created by another account does not match. A client shows nothing from
  the chain before that holds.
- The ID of the video is not in the field. The contract assigns it in the transaction, and the
  manifest exists before.
- A set without the field is still a valid upload set. It is found from the chain only.

---

## 3. The link on the chain

### 3.1 Form

The content contract stores, per video, a list of pairs of a key and a value, `media` (§4.1).
The upload is the pair with the key `ed3k`:

```
key   = "ed3k"
value = ed3k:set?key=<channel key, base32>&id=<id, base32>&b3=<root, base32>&sz=<size>[&ipfs=<CID>][&dn=<url-encoded name>]
```

- `key`, `id` and `dn` are those of `file-set.md` §5.4.
- **`b3` and `sz` are added by this extension.** They are the root and the size of the
  manifest of one version of the set. **That version is the video** (§3.2). A client without
  this extension ignores them.
- **`ipfs` is added by this extension too.** It is the content ID of an IPFS directory that
  holds the files of the set at their paths (§5.5). It is optional, at most 128 bytes, and
  SHOULD be a version 1 CID in base32.
- The value is at most 512 bytes. A longer one is not read.
- A video has one such pair. It is written when the video is created, so an upload is one
  transaction.
- A reader of this extension ignores every other pair of a video. It MUST NOT fetch a URL from
  one on its own.

### 3.2 Reading it

1. Parse the link. A value that is no `ed3k:set` link with `key`, `id`, `b3` and `sz` is not an
   upload of this extension.
2. Fetch the manifest with the root `b3` and the size `sz`, as a file.
3. Accept it only when it is signed by `key` and carries `id`. Otherwise the video has no
   media, and the client says so.

- **The link pins the version.** A reader of this extension MUST NOT follow the head of the
  set for a video. A version of the set that the chain does not name is not the video,
  whatever its `seq`.
- **A new version is a transaction.** The author writes the pair again (`updateMedia`, §4.1)
  with the `b3`, `sz` and `ipfs` of the new version. This holds for every upload, with a site
  or without one. The chain so shows every version of a video with its time.

- **The transaction is the tie.** The account that sent it is the author on the chain, and it
  wrote the channel key into the link. No signature joins the two keys, and none is needed:
  nobody else can write that pair.
- **The channel key alone changes nothing.** Whoever holds it can sign a newer version, but
  only the account can make it the video. A site that holds the key, or a thief of it, cannot
  change what a video shows.
- **No head is needed.** A head lasts 7 days unless someone stores it again (`file-set.md`
  §5.2). A video is found by `b3` for as long as its files have sources.
- A client without this extension reads the link as a plain set link and follows the head.
  So that it sees the same video, whoever stores a head for the set MUST store the one of the
  version the chain names, and a newer one only after its transaction is in a block.
- One channel key in the links of two accounts means that both accounts chose it. It says
  nothing more.

### 3.3 Why a set link and not file links

| | One `ed3k:set` link (chosen) | One `ed3k:file` link per rendition |
|---|---|---|
| Transactions per upload | 1 | 1 per rendition, sound track and subtitle |
| What is committed | everything, by one root | each file alone; nothing joins them |
| A rendition added later | a new version of the set and one transaction | a transaction |
| Picture | has a place | none in the contract |
| First byte | after the manifest, a few hundred bytes | at once |
| A plain web player | needs a gateway to read the manifest | could read a `ws` URL in the link |

---

## 4. The chain

### 4.1 Contracts

Four contracts. The sources are part of the ChainTube backend.

| Contract | Holds | Events |
|---|---|---|
| Content | per entry: author, kind, parent, reference, title, one list of pairs (`media`), tags, a text, `closed`, `blocked` | `ContentSet(uint256 contentId, address indexed author, uint256 indexed parentId, uint32 indexed kind, uint256 refId, string title, (string,string)[] media, string[] tags, string content, bool closed, bool blocked)` |
| Comment | per comment: author, content contract, content ID, the comment it answers, text, a list of pairs (`media`), `blocked` | `CommentSet(uint256 commentId, address indexed author, address indexed contractHash, uint256 indexed contentId, uint256 replyId, string content, (string,string)[] media, bool blocked)` |
| Vote | per vote: author, contract and ID of what is rated, type, value | `VoteSet(uint256 voteId, address indexed author, address indexed contractHash, uint256 indexed contentId, int256 voteType, uint256 voteValue)` |
| Profile | per account: a unique name, an alias, an avatar, a text. Follows, as events only | `UserSet(address indexed userAddress, uint256 userId, string userName, string userAlias, string userAvatar, string userBio)`, `FollowSet(address indexed follower, address indexed target, uint256 indexed targetId, bool following)` |

- **The contracts carry more than videos.** An entry of the content contract has a `kind`:
  0 a video, 1 an article, 2 a board, 3 a thread of a forum, 4 a short message. `parentId`
  names the entry it is in, `refId` the entry it quotes. **A video of this extension is an
  entry of kind 0.** A reader of this extension ignores the other kinds; what they mean is in
  the backend's `docs/protocol.md`.

- **Every event carries the whole state** of its item, for a new item and for a change. The
  newest event of an ID is the item. Within one block the later log is the newer.
- IDs count from 1 per contract. A video is named by chain ID, content contract and content ID.
- A vote names what it rates by a contract and an ID: the content contract for a video, the
  comment contract for a comment.
- `voteType` above 0 is a vote up, below 0 a vote down. `voteValue` is an amount of the chain's
  coin that the voter paid. The contract divides it between the author, the creator of the
  video, the operator of the contract and a wallet that the sender of the transaction names.
- A video cannot be deleted. Its author can change the title, the pairs, the tags and the
  text. Kind, parent and reference are fixed when it is created.
- The **moderators** of an entry are its author, the accounts its author named, and the
  moderators of its parents. A moderator of a parent can set `blocked` on an entry. A
  moderator of an entry can set `closed` on it and `blocked` on its comments. A video without
  a parent has nobody above it: only its comments can be blocked, by its author.

| Use | Function |
|---|---|
| Upload | `createContent(uint32 kind, uint256 parentId, uint256 refId, string title, (string,string)[] media, string[] tags, string content)` with `kind` = 0, `parentId` and `refId` = 0 unless the video is in a board or quotes an entry, and `media` = one pair of `ed3k` and the link of §3.1 |
| Write the link again | `updateMedia(uint256 contentId, string key, string value)` with `key` = `ed3k` |
| Comment | `createComment(address contractHash, uint256 contentId, uint256 replyId, string content, (string,string)[] media)`; `replyId` 0 = not an answer; `media` empty, or attachments under the rule for other kinds (§10) |
| Rate | `castVote(address contractHash, uint256 contentId, int256 voteType, address platformWallet)`, with the value as the amount of the transaction |
| Read one video | `getContentById(uint256 contentId)` |

The ABI of the contracts is not in this repository. The signatures above are what a reader
needs; they are informative until the contracts are published with the parameters of §4.2.

### 4.2 Parameters of a network

| Parameter | Public network |
|---|---|
| Chain ID | *to be assigned at deployment* |
| Addresses of the four contracts | *to be assigned at deployment* |
| RPC endpoints | a list in the client's configuration; none is built into the protocol |

A network with a name (`Protocol.md` §4.3) has its own contracts or none.

A test deployment exists on Avalanche Fuji (chain ID 43113): content
`0x8cF7928c9d3aDeC175ea7f3e919A681b06Ba3ED8`, comment
`0x6959AD50f2Ed0601227Bd2a966F9e9b2B313D2Bb`, vote
`0x39f2e54075073b5Ad934CdB39E3410AbC3e540cc`, no profile contract. The addresses are those of
the backend's configuration on 2026-10-12. **The code at them is older than §4.1** (three lists
of pairs, no kind): the contracts were changed on 2026-10-13 and are not deployed again yet.

### 4.3 Reading

**The chain is the rule.** A client reads over the JSON-RPC of a node of the chain, as the
credit extension does ([`credit.md`](credit.md) §7.3).

| Wanted | Read |
|---|---|
| Is this the right chain | `eth_chainId` |
| One video | `eth_call` of `getContentById` |
| The videos of an account | `eth_getLogs` at the content contract, topics `author` and `kind` = 0 |
| The video of a set | the same with `tube.author`; then the video whose link names the set's `key` and `id` |
| The comments of a video | `eth_getLogs` at the comment contract, topics `contractHash` and `contentId` |
| The votes on a video or a comment | `eth_getLogs` at the vote contract, the same two topics |
| New videos of all accounts | `eth_getLogs` at the content contract, topic `kind` = 0, backwards from the newest block |

- A client asks for at most 2,048 blocks *(initial)* in one `eth_getLogs` and reads at a block
  that the chain treats as final.
- A client that uses a remote node SHOULD ask two of different operators for a read it acts
  on, and SHOULD send RPC through the proxy the user configured, if any.
- A local node of the chain needs nothing more.

**What a client makes of it**

- **Everything from the chain is text that a stranger wrote.** A client shows a title, a tag, a
  comment, an alias and a name as plain text, never as markup, and follows no link in them on
  its own.
- A client shows at most 256 bytes of a title, 64 bytes of a tag and 16 tags, and 8,192 bytes
  of a comment *(all initial)*. The contracts set no limits.
- A comment with `blocked` set is not shown unless the user asks. Only the moderators of the
  video can set it (§4.1), and a later change of the comment by its author keeps it.
- A video with `blocked` set is not shown unless the user asks. A video with `closed` set takes
  comments from its moderators only; the comment contract refuses the others.
- The contracts refuse a comment or a vote on a video that does not exist, and an answer to a
  comment of another video. A client still ignores one that names a contract other than those
  of §4.2.
- An account can vote on one thing many times. A client SHOULD count one vote per account and
  thing, the newest, and SHOULD show the sum of the values apart from the count.
- An avatar is used when it is an `ed3k:file` link to a picture of at most 1 MiB *(initial)*.

### 4.4 Writing

- A transaction is signed by an account of the user: in a wallet, or with a key the client
  holds. The account pays the gas.
- **The key SHOULD NOT be the credit key** ([`credit.md`](credit.md) §2). An account that
  uploads and comments is a public person, and the credit address of a node is meant not to be
  one.
- A key that the client holds is as private as the node key. A client MUST NOT write it to a
  log or show it in an error message.
- A client sends a transaction only when the user asked for this one. It shows the amount and
  the receiver contract before.
- `platformWallet` is the zero address unless the user chose a site to support.
- A client that only makes the set and shows the link of §3.1 needs no key at all. The user
  then pastes the link into the upload form of a site, and signs there.

### 4.5 A site's list as a shortcut

Reading a year of logs from a remote node is slow. A site has done it already and answers
lists and searches at once. Its interface is its own and not part of this protocol.

- A client MAY ask sites the user configured for lists of videos, comments and votes.
- **Before a client downloads the media of a video it MUST read that video from the chain**
  (`getContentById`) and take the link from there.
- A client SHOULD mark comments and ratings that it has from a site only, and reads them from
  the chain when the user is about to pay or to answer.
- What a site can do to a client that follows this: leave things out, and show comments or
  counts that are not on the chain until the client reads them itself. It cannot make a client
  download other media than the author named.
- A site sees the address of the client and what it asks for.

---

## 5. A site

### 5.1 Not a role in ed3k

A site is a program of its own. To ed3k it is invisible. What takes part in ed3k is the
**site node**: an ordinary node, with the duties and limits of any node.

| | A program of its own, with a node next to it (chosen) | The site as an index node with messages of its own |
|---|---|---|
| ed3k | nothing added | record kinds or messages for videos, comments and votes |
| Truth | one: the chain | two that can differ: the chain and what the node answers |
| Web browsers | need HTTP in either case; the site has it | the same, so the HTTP part exists anyway |
| The site's program | reads a chain and serves a web site; needs no ed3k code | must carry a whole ed3k node |
| Found by | a link, a list, the user's choice | Kad, like any index node |
| An ed3k client without access to the chain | finds videos by keyword only when the site node publishes them (§5.4) | could be served everything, on the site's word |

An index node (`Protocol.md` §11) is a node that stores and answers Kad records for every
target. That is about records, not about a catalogue. A site node MAY be one as well (§5.4);
it then does what every index node does and nothing else.

### 5.2 What a site node does

- **It seeds.** It reads the videos its site shows from the chain, downloads the members of
  the version each link names, and
  shares them, by its own policy of which and for how long.
- **It MAY keep heads alive** for clients that read the link as a plain set link: the head
  of the version the chain names, and no other (§3.2).
- **It names its gateway as an HTTP source.** A site that serves the files over HTTP for web
  players is a plain source ([`http-source.md`](http-source.md) §1). The site node hands that
  URL out in the file stream and next to its `SOURCE` record, as that document says. An ed3k
  client then fetches from the gateway and from peers at once, and verifies every byte under
  the root.
- A site node SHOULD be a `ROUTER`, so that it is reached without signalling.

- A site that keeps its media in IPFS does all of this with the gateway of its IPFS node
  (§5.5).

Nothing of this is required for a video to exist. An upload that its author seeds alone is a
valid upload.

### 5.3 Uploads through a site

A web browser cannot speak ed3k. A visitor without an ed3k client hands the file to a site,
and the site encodes it, makes the set, and has its node share it. How the file reaches the
site is the site's business.

The channel key is where the two ways differ:

| | The visitor holds the channel key | The site holds it |
|---|---|---|
| Who signs the manifest | the visitor's browser or client, over the body the site prepared | the site |
| The site can sign a newer version of the set | no | yes; it is the video only after the visitor's transaction (§3.2) |
| The visitor can move to another site | yes, with key and files | only by a new upload under a new key |
| Needs | a key kept by the visitor next to the wallet | nothing from the visitor |

- In both ways the transaction is signed by the visitor's account, and the link names the key.
  The visitor's wallet sends it, or the visitor signs it and the site sends it. A site that
  sends it cannot change it; it can only hold it back.
- **A new version is a transaction** (§3.2), signed and sent as the first one was. A site
  cannot change a video without the account's signature, also when it holds the channel key.
- A site that holds channel keys MUST keep one per account and MUST NOT sign a set of one
  account with the key of another.
- A site SHOULD say which of the two it does.

### 5.4 The site node as an index node

A site node SHOULD set `INDEX` and SHOULD publish a keyword record (`Protocol.md` §10.3) for
the manifest of each video it seeds, so that the videos are found by a search in ed3k, without
the chain.

- The manifest is shared under the name `<title>.ed3kset`, with the title of the chain. The
  keywords are those of that name, as for any file. Tags are not keywords.
- The record is the site node's, signed by its key like any keyword record. **It is a claim of
  that node**: that a file with this name and root exists. It says nothing of the chain.
- A client that gets the manifest from such a search reads `tube` (§2.3) and checks it on the
  chain before it shows an author, a comment or a rating.
- As an index node it accepts records for every target from everyone, under its own admission
  policy (`Protocol.md` §11). A site that does not want that load leaves `INDEX` unset and
  publishes its keyword records in Kad.
- Clients find a site node like any index node: from a list, or from an `ed3k:node` link
  (`Protocol.md` §13.2) that the site shows.

### 5.5 Media kept in IPFS

A site MAY store uploads in IPFS and serve them over HTTP from the gateway of its IPFS node.
Nothing in ed3k changes for that: **a file behind an IPFS gateway is a plain HTTP source**
([`http-source.md`](http-source.md) §1), and a set laid out as in §2.1 is an IPFS directory as
it is.

**The directory**

```
<CID>/video/720p.mp4            every member of the set at its path
<CID>/video/720p.m3u8
<CID>/master.m3u8
<CID>/thumb.jpg
<CID>/ed3k/set                  the manifest of the set
<CID>/ed3k/<root, base32>       the hash body of the member with that root (http-source.md §3.1)
```

- `ed3k/` is not part of the set. It is what an ed3k client needs to use the directory
  without a peer: the manifest, and the block hashes that let it verify a member while it
  arrives. A member of one block has no hash body.
- The files in the directory are byte for byte the members of the set. The site hashes each
  once more with BLAKE3; IPFS neither needs nor sees that root.
- A newer version of the set is a new directory with a new CID. Files that did not change keep
  their CIDs inside it, so nothing is stored twice. The new CID goes to the chain with the new
  `b3` (§3.2).
- The name `ed3k/` is taken: no member of an upload set has a path under it (§2.1).
- The name has no leading dot on purpose. Tools that add a directory to IPFS leave out names
  that start with a dot unless told otherwise, and say nothing about it.
- A directory without `ed3k/` plays in a web player and is of no use to an ed3k client. A
  site SHOULD check that `<CID>/ed3k/set` can be fetched before it writes the CID to the
  chain.

**How the two names meet**

| Name | Where | Commits to |
|---|---|---|
| root of the manifest (`b3`) | the link on the chain | every member, by its root |
| CID of the directory (`ipfs`) | the link on the chain | every file, by IPFS's own tree |

- Both are written by the author in one transaction. Neither is derived from the other, and
  ed3k never asks for a file by a CID (`Protocol.md` §7.4).
- **To an ed3k client the CID is a place, not an identity.** It says where bytes may be
  fetched. The bytes count when they pass under the root, as from any source.
- A CID that holds other bytes, or no longer the newest version, costs failed fetches and
  nothing else: the source is dropped by the rules of `http-source.md` §6.

**Fetching**

A URL of a member is

```
<gateway>/ipfs/<CID>/<path of the member>
```

and its `hashes` URL is `<gateway>/ipfs/<CID>/ed3k/<root>`.

- **The site node hands these URLs out** as HTTP sources of each member (§5.2), with its own
  gateway. This works for every client with the `HTTP` capability and needs no `ipfs` in the
  link.
- **With `ipfs` in the link, a client can build them itself**, for gateways the user
  configured: its own IPFS node, or a gateway of its choice. It needs no site and no peer for
  that. It uses at most 3 gateways per video *(initial)*.
- No gateway is built into the protocol. A client MUST NOT send a request to a gateway that
  neither the user nor a source of the file named.
- The checks, limits and redirects of `http-source.md` §3 and §5 apply. A gateway that
  redirects to a host name of its own for the CID is followed like any redirect. A gateway on
  the user's own machine is allowed because the user configured it.
- The gateway of a site SHOULD answer `Range` requests on files. Seeking in a player and
  fetching from several sources at once both need it. A gateway that does not is read from
  the start or not used (`http-source.md` §3).
- A client that has an IPFS node of its own MAY fetch the files through it by any means that
  node offers. That is between the client and IPFS, and outside this protocol.

**A web player without ed3k** reads the link from the chain and plays
`<gateway>/ipfs/<CID>/master.m3u8`, or a rendition directly. It verifies nothing under the
root. What it gets is as true as its gateway: the site's own IPFS node checks its blocks
against the CID, and a foreign gateway is believed.

**What the site node shares.** It is a source in ed3k for the same files. It SHOULD read them
from the IPFS node's store instead of keeping a second copy, and it verifies them under their
roots before it serves a block, as always.

---

## 6. What an attacker can do

| Attack | What limits it |
|---|---|
| **A false `tube` field**: a set that claims an author it does not have | the claim is checked on the chain (§2.3). Nothing is shown before |
| **A link to someone else's set** in an own video | the set keeps its own key; the client shows the account of the chain as the author of the video. A copy is as possible as on any video site |
| **A site lists what is not on the chain** | the client reads the video from the chain before it downloads (§4.5) |
| **A site that holds a channel key, or a thief of one, changes a video** | it cannot: a reader takes the version the chain names and follows no head (§3.2) |
| **A remote RPC node lies** | two nodes of different operators (§4.3). It cannot sign for anyone |
| **A false `ipfs` in a link**, or a gateway that sends other bytes | the bytes fail under the root; the source is dropped. A web player without ed3k has no such check (§5.5) |
| **A link names a gateway inside the reader's network** | a link names a CID, never a host; gateways are the user's or come with a source and pass the address checks of `http-source.md` §5 |
| **Long strings on the chain** | limits when shown (§4.3), 512 bytes for a link (§3.1) |
| **Markup or links in a title or comment** | shown as plain text |
| **Many accounts vote or comment** | each costs gas; counting is the client's (§4.3). Nothing in ed3k depends on a count |
| **A set that is huge** | the limits of `file-set.md` §2.1 and 64 files (§2.1); a client downloads the rendition the user chose, not the set |

## 7. What it reveals

- An upload joins an account of the chain to a channel key, in public and for good.
- A node that seeds its own upload is a source of it. Whoever asks for sources learns the
  address of a node that has the video, as for any file. An uploader that does not want its
  address next to its account lets a site node seed and stops, or uses an anonymous path
  ([`hops.md`](hops.md)).
- A gateway sees the address of whoever fetches from it and which file. A client SHOULD use
  the proxy the user configured for it, as for any HTTP source.
- A node of the chain sees which videos a client reads about. A site sees the same for what it
  is asked.
- A comment and a vote are public and cannot be taken back.

## 8. What it costs

| | |
|---|---|
| ed3k wire | nothing: no message, no record kind, no capability bit |
| A manifest | about 50 B for `tube` |
| An upload | one transaction with a link of about 200 B, 260 B with a CID |
| Kad | the records of any set: a head, sources per member, and keyword records where a site node publishes them |
| A viewer | one read of the chain, one manifest, then the rendition it plays |

---

## 9. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **The site as an index node with messages for videos, comments and votes** | one program; an ed3k client needs no chain | a second catalogue beside the chain (§5.1) |
| **Comments and votes as signed Kad records too** | readable without the chain | two stores that can differ; a vote carries a payment that only the chain enforces |
| **An `ALIAS` scheme from a video ID to the root** ([`alias.md`](alias.md)) | a video is found in Kad by its name on the chain | whoever knows the ID can ask the chain, which is the judge anyway; a record that only the chain can check adds a lookup and no certainty |
| **A signature between the account and the channel key** | a set proves its author without a read of the chain | a new signed object and a prompt in the wallet; a copy of it proves nothing unless it is on the chain too |
| **The credit key as the account** | one key for both | joins the credit address of a node to a public person (§4.4) |
| **A link without `b3`** | shorter; the head alone decides | a video is lost to readers when its head expires everywhere, and the channel key alone can change it |
| **`b3` as a fallback, the head followed when it is newer** | an author that seeds alone adds a rendition without a fee | whoever holds the channel key changes a video with no trace on the chain; a rule for sites that no reader can check |
| **A `pin` flag in the link**, the head followed without it | both of the above, by the author's choice | two ways to read a link, and a viewer has to be shown which one a video uses |
| **The CID as the identity of the video**, with no root | one name; what an IPFS site has anyway | a second identity that ed3k cannot verify block by block from a peer; rule 1 of [`alias.md`](alias.md) §5 |
| **An `ALIAS` scheme from a CID to the root** | a video is found in ed3k by its CID | the claim can be checked only by building IPFS's tree with the publisher's settings for chunks and layout; the link on the chain names both already |
| **The CID in the manifest** | changes with the set, no transaction | the directory holds the manifest, so the manifest cannot hold the directory's CID |
| **A gateway URL in the link** | a client needs no setting | a host name on the chain for good; a link could point readers at any host |
| **Segments as members** (the layout of HLS) | what web players expect | thousands of members per video; ranges of one file do the same (§2.2) |
| **Renditions in the pairs of the chain** | readable without a manifest | one transaction each (§3.3) |

## 10. Open points

- **A site registry.** Sites are configured or found by links. A list on the chain would let a
  client find them, and would be a thing to moderate.
- **Live streams of a channel.** [`live.md`](live.md) has channels with a key of their own. A
  pair that names an `ed3k:live` link would join one to an account.
- **Following an account.** The profile contract logs `FollowSet` for an account or for a board
  or thread. It is a public statement and no delivery: a client still follows the logs of an
  account, or the sets of a channel key through [`pubsub.md`](pubsub.md).
- **Media of the other kinds.** A thread, a message or a comment may carry a pair `ed3k` with
  an `ed3k:set` link to its attachments or an `ed3k:file` link to one. No layout like §2.1 is
  defined for them, and §3.2 is written for videos.
- **Tips and credits.** A vote carries a payment in the chain's coin; credits
  ([`credit.md`](credit.md)) live on another chain. Nothing joins them.
- **The contracts are a draft.** No profile contract is deployed. A change of the contracts
  changes §4.1.
- A site that does not use IPFS serves its files under paths of its own. The layout of §2.1
  under one base URL would let a web front end use any site's server; no link parameter names
  such a base today.
- How fast the gateways of the IPFS nodes in common use answer `Range` on large files was not
  measured.
- **Who pays the fee** is outside this extension. The contracts take a call from the account
  itself, or a request that the account signed (EIP-712) and that a site sends through a
  forwarder contract (ERC-2771) and pays for. The author on the chain is the account in both
  cases, so §3.2 holds unchanged. The form of the request is in the backend's
  `docs/protocol.md`.
- All numbers are first values.
