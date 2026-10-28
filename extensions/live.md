# Extension: live streams (`live`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3, §12 and §13. A section number (§) that exists in this document refers to this document; `Protocol.md` is named where its section of the same number is meant. §7.2 is a section of `Protocol.md`.
**Capability:** bit 11, `LIVE`. Message types 19 to 21. Record kind 6.
**Required of every `ROUTER`:** storing and returning the record kind `LIVE` (§5). The rest is optional.

Live stream = media produced while watched. Broadcaster cuts it into **segments** of a few
seconds; viewers fetch each from one another shortly after it exists, keep only the newest.
Little is new:

- Segment = ordinary file with a BLAKE3 root, verified block by block (§7.2).
- Broadcaster signs one small **segment record** per segment; peers forward it unchanged.
- Channel described by a small **channel file**, shared and searched like any file.
- Stream peers found via one new record kind.
- NAT traversal (`Protocol.md` §9), compression and the bad-block rule (both `Protocol.md` §12.3) apply
  unchanged.

Media is opaque to the protocol; encoding/playing is the application's business.

---

## 1. Channel

### 1.1 Naming

A channel has a **channel key** (Ed25519, public key 32 bytes) and an owner-chosen `id` of 1 to
16 bytes, so one key can have several channels.

```
stream = H("ed3k v1 live", key ‖ id)[0..16)
```

`stream` is the channel's Kad target (§5) and names it in every message.

- Channel key SHOULD be its own key, not the node's identity key ([`Protocol.md`](../Protocol.md) §6.1): channel then
  outlasts the node, can be broadcast from another node, link doesn't name broadcaster's node ID.
- At most one **broadcast** per channel at a time, named by its `start`: Unix time in seconds it
  began. A later broadcast has a later `start`.

### 1.2 The channel file

```
"ed3klive" ‖ 0x01 ‖ LiveFile        magic: 65 64 33 6B 6C 69 76 65 01
```

```protobuf
message LiveFile {
  bytes body = 1;             // a serialised Channel, verified over exactly these bytes
  bytes key = 2;              // 32 B channel key
  repeated Signature sigs = 3;
}

message Channel {
  bytes  id = 1;
  string title = 2;
  string comment = 3;
  string lang = 4;            // BCP 47 tag, for example "en"
  string media = 5;           // media type of a segment, for example "video/mp4"
  string codecs = 6;          // RFC 6381 codecs string, for example "avc1.64001f,mp4a.40.2"
  uint32 kbps = 7;            // typical bit rate
  uint32 segment_ms = 8;      // duration a segment aims at
  uint32 window = 9;          // segments a viewer keeps, 8 to 240; 0 or absent = 30
  uint64 created = 10;
  // field 11 belongs to the optional paid subscription extension (subscription.md §3)
}
```

- Signature over `"ed3k v1 live channel" ‖ body`, REQUIRED. Reader checks it against `key`,
  refuses the file on failure. `sigs` rules: `Protocol.md` §10.7.
- A channel file MUST NOT exceed 64 KiB. `window` is 8 to 240 *(initial)*; 0 or absent still
  = 30. Reader refuses a larger file or other `window`, so §3.1's first `LiveState` carries
  ≤ 240 records, a viewer keeps ≤ that many segments, and a window is never too short to join 3
  segments behind (§3.3).
- Shared, published under its keywords and downloaded like any file ([`Protocol.md`](../Protocol.md) §10 and §12). Search
  finds channels as files; suggested name ends in `.ed3klive`.
- Describes the channel, not a broadcast; changes only with the description. Of two files for
  one `stream`, reader uses later `created`.

### 1.3 Links

```
ed3k:live?key=<channel key, base32 lower case, no padding>&id=<id, base32>[&dn=<url-encoded title>]
```

Link suffices to watch: gives `stream` and the key segment records are checked against; `media` and
`codecs` come from the first peer (§3.1). Clients w/o this extension don't know the link. `key`, `id` and §10's `s` are base32 as [`Protocol.md`](../Protocol.md) §13.2 fixes:
written lower case w/o padding, read either way. Link invalid when `key` doesn't decode to 32
bytes, `id` to 1 to 16 bytes or `s` to 32 bytes, or when `key`, `id`, `s` or `dn` is repeated.

---

## 2. Segments

### 2.1 Segment

- Byte string ≤ 4 MiB, one piece (§7.2). Identity = BLAKE3 root; blocks verified as a file's.
- A broadcast's segments numbered by `seq` from 0, no gaps.
- Each segment MUST be playable on its own once init data is known, so a viewer can start at
  any segment.
- **Init data**: what a container needs before its first segment, e.g. fragmented MP4's
  initialisation segment. Byte string ≤ 256 KiB, named by its root; none if container needs
  none.

### 2.2 Segment record

```protobuf
message LiveRecord {
  bytes body = 1;             // a serialised LiveSegment, verified over exactly these bytes
  repeated Signature sigs = 2;
}

message LiveSegment {
  uint64 start = 1;           // the broadcast (§1.1)
  uint64 seq = 2;
  bytes  root = 3;            // 32 B
  uint32 size = 4;
  uint32 ms = 5;              // duration
  bytes  init = 6;            // root of the init data; absent = none
  uint32 init_size = 7;
  bool   gap = 8;             // not continuous with the segment before: the player resets
  bool   end = 9;             // the broadcast is over; no segment, `root` absent
  // fields 10 and 11 belong to the optional paid subscription extension (subscription.md §7.1)
}
```

Signature, made with the channel key, over

```
"ed3k v1 live segment" ‖ stream(16) ‖ body
```

The string is no prefix of the channel-file one (§1.2), nor vice versa.

- Record ~150 bytes. A `LiveRecord` MUST NOT exceed 1 KiB; larger one dropped unread, not
  forwarded, no miss for the sender.
- `sigs` rules: `Protocol.md` §10.7.
- Record accepted only with valid signature, else dropped, one miss for the sending neighbour
  (§6), taking one of the 20 places as a request outcome does.
- **Limits.** A validly signed record is still invalid when `size` > 4 MiB, `init_size` >
  256 KiB (§2.1), or `root` absent without `end`: dropped, not forwarded. No miss for sender:
  channel key signed it, neighbour only passed it on.
- **Order.** By `start`, then `seq`. Node ignores a record with `start` earlier than newest
  seen, or > 1 h *(initial)* ahead of its clock, so an old broadcast can't be played back into a
  running one.
- **Behind the window.** Node drops, w/o forwarding, a record whose `seq` is ≤ newest held `seq`
  of that broadcast minus `window`, i.e. outside the newest `window` segments (§3.3; nothing
  while newest `seq` < `window`), so a running broadcast's old records don't travel the mesh
  again. Not invalid, no miss.
- **Two records for one `seq`** of one broadcast with different content prove the channel key
  signed both. Lower `root` (byte compare) wins: kept, passed on, other dropped, even if that
  segment was already fetched. So all nodes hold the same bytes per `seq`; `have` bits mean the
  same everywhere.
- `end` closes the broadcast. Nodes forward it, keep it 30 min *(initial)*, leave the stream:
  stop renewing own `LIVE` records at once, and for those 30 min answer a `LiveJoin` with the
  `end` record and `refusal = LIVE_LEFT`, so late viewers learn it's over. In a private channel only a `LiveJoin` with valid proof is answered so (§10.3). The only
  ways to end a broadcast: this record or silence (§3.4). An `end` record has the `seq` after
  the last segment's, and `root`, `size`, `ms`, `init` and `init_size` absent.
- For one `seq`, a record w/o `root` loses against one with a `root`; of two without, the first
  verified is kept: the rule above can't compare absent roots.
- Record carries no wall-clock time of the segment: `seq` gives order, a time stamp would only
  expose the broadcaster's clock.

### 2.3 Formats

Any format is carried, but two compliant clients could still fail to play each other's
channels, hence a baseline.

| | Baseline |
|---|---|
| Container | fragmented MP4 (CMAF, ISO/IEC 23000-19), `media` = `video/mp4` or `audio/mp4` |
| Video | H.264 (AVC), High profile, up to level 4.1 (1080p at 30 frames per second) |
| Audio | AAC-LC or Opus |
| Init data | the initialisation segment (`ftyp` and `moov`) |
| Segment | one or more fragments (`moof` and `mdat`), starting with a key frame |

- Viewer SHOULD be able to play the baseline. Broadcaster SHOULD use it unless it knows its
  audience can play something else.
- **Audio only** is a channel like any other, `media` = `audio/mp4`, no video track.
- Other formats allowed, named in `media` and `codecs`: MPEG transport stream segments
  (`video/mp2t`, no init data), WebM, or newer codecs e.g. H.265 and AV1 inside MP4.
- Viewer that can't play what `media` and `codecs` name says so, doesn't join.
- Producing/playing segments isn't protocol; no particular encoder or tool required.

Baseline = what browsers, phones, common players decode w/o added software; the segment form
HLS and DASH share.

---

## 3. Between two peers

Two peers exchanging a stream are **neighbours**. One opened the **live exchange**; after that
both sides are equal.

```protobuf
message LiveJoin {
  bytes  stream = 1;          // 16 B
  uint32 want_peers = 2;      // max number of peers wanted, 0 = none
  bool   want_channel = 3;    // send `media` and `codecs`
  bytes  relays4 = 4;         // n × 22 B compact nodes: my relays, if I am a leaf
  bytes  relays6 = 5;         // n × 34 B
  bytes  proof = 6;           // private channel: 16 B (§10.3)
  // fields 7 to 9 belong to the optional paid subscription extension (subscription.md §6.3); 7 and 8 are reserved
}

message LiveState {
  optional uint64 start = 1;  // broadcast this state is about; absent = unchanged
  optional uint64 first = 2;  // seq of bit 0 of `have`; absent = unchanged
  optional bytes have = 3;    // segments I hold completely, LSB first; absent = unchanged
  repeated LiveRecord records = 4;
  repeated Peer peers = 5;    // other peers of this stream
  string media = 6;           // answer to want_channel
  string codecs = 7;
  Refusal refusal = 8;        // set: I do not take you as a neighbour
  bytes  proof = 9;           // private channel: 16 B, in the first answer to a valid proof, whatever `refusal` (§10.3)
  // fields 10 to 14 belong to the optional paid subscription extension (subscription.md §6.3)
}

enum Refusal { LIVE_NONE = 0; LIVE_FULL = 1; LIVE_LEFT = 2; LIVE_BLOCKED = 3; }   // prefixed: enum values share one scope per package
// 4, LIVE_PASS, belongs to the optional paid subscription extension (subscription.md §6.3)

message LiveGet {
  bytes  stream = 1;
  bytes  root = 2;            // a segment or init data
}
```

### 3.1 The live exchange

- Viewer opens exchange, sends `LiveJoin`; peer answers `LiveState`: its window, records of every
  segment in it, peers if asked.
- Peer not taking the viewer answers with `refusal` set, closes. With `LIVE_FULL` it SHOULD still
  fill `peers` so viewer can go elsewhere. Peer not in the stream answers `LIVE_LEFT`.
- After the first answer **both sides send `LiveState`** on the exchange whenever:
  - It learns a new valid record: sends it at once to every neighbour that hasn't sent it that
    record. How records travel; the only push, apart from a paid channel's keys and offers
    ([`subscription.md`](subscription.md) §6.3).
  - It completes a segment: `first` and `have` sent, at most once per second *(initial)*.
  - 10 s *(initial)* passed without one: empty `LiveState` as sign of life.
- Neighbour silent for 30 s *(initial)* is gone. Either side leaves by closing the exchange.
- `peers` MUST be peers the sender itself had as verified neighbours of this stream within the
  last 10 min *(initial)*.
- At most one live exchange per stream between two nodes. If two are open, both keep the one
  opened by the lower node ID (byte compare); of two by the same node, the newer. The node that
  opened the other one closes it; the peer doesn't answer on it, resets it if still open 10 s
  *(initial)* later. No refusal, no miss, no loss (§6.2); the two count as one neighbour.
- **Limits.** `LiveJoin` names ≤ 2 relays per family, at addresses acceptable in the network
  (`Protocol.md` §8.2 (7)). `LiveState` carries ≤ 240 records and a `have` of ≤ 240 bits (30
  bytes), the largest window. Above these: malformed, node closes the exchange.
- `first` is the sender's choice; SHOULD be `seq` of the oldest record it holds within its
  window, so `have` stays within `window` bits. Receiver accepts any `first`, doesn't ask for a
  segment behind the window (§2.2).

### 3.2 Fetching a segment

Data is never pushed. Node asks a neighbour whose `have` shows the segment.

- Node opens a **new exchange per segment**, sends `LiveGet` (no answer of its own: names this
  exchange's segment), then `BlockRequest`s and `HashRequest`s on it w/o waiting, as in
  `Protocol.md` §12.2 to §12.4, offsets inside the segment, ≤ 16 ranges outstanding; neighbour
  answers `Block`s and `HashAnswer`s. No queue position
  (`FileStatus.rank`, `Protocol.md` §12.5): neighbour serves at once, or answers `Error{BUSY}`,
  `Error{NOT_FOUND}` or `Error{DENIED}` and closes the exchange.
- `LiveGet` accepted only from a neighbour, i.e. while a live exchange with that node is open.
  Else `Error{DENIED}`.
- Multi-block segment may be fetched from several neighbours at once, by block; how many, how
  split, which exchange asks for hash values: local policy.
- **Init data** asked of a neighbour whose `have` shows a segment whose record names it.
  `Error{NOT_FOUND}` from such a neighbour counts as for a segment (below), unless every such
  segment has left its window since.
- Node no longer wanting a segment resets the exchange. Own exchange per segment → a lost packet
  delays only its segment.
- `Error{NOT_FOUND}` for a segment the neighbour's last `have` showed = a miss (§6), unless the
  segment has left its window since.
- Bad block = that of `Protocol.md` §12.3, stream in place of file: node sends `BadBlock` on the
  segment's exchange, closes the live exchange, answers a `LiveJoin` of that node ID with
  `LIVE_BLOCKED` for 4 h *(initial)*, this stream only. Bans of `Protocol.md` §12.3 apply.
- Node gives up on a request after 4 s *(initial)* plus 1 s per MiB asked, asks another
  neighbour.

### 3.3 The window

- Node keeps the newest `window` segments of the running broadcast (§1.2; 30 when it doesn't
  know the channel file) and the init data they name. Older ones deleted.
- Joining viewer starts 3 segments *(initial)* behind the newest record given; doesn't fetch
  older. Node falling out of the window (couldn't keep up) restarts the same way.
- Record with `gap`, or naming other init data than the one before → player reset at that
  segment.

### 3.4 Neighbours and silence

- Node aims at 6 neighbours, keeps 3 to 8 *(initial)*. Broadcaster takes as many as upload
  allows (§4), also more than 8.
- Neighbours subject to a bucket's diversity limits: at most 1 per host, 2 per site, 4 per block
  ([`ip-diversity.md`](ip-diversity.md) §1 and §2).
- No new record for 60 s *(initial)* → broadcast counts as interrupted. Viewer keeps its
  neighbours, looks for others (§5). After 10 min *(initial)* without a record it leaves.

---

## 4. Broadcasting

- Broadcaster = node holding the channel key, or receiving signed records from whoever does.
  Otherwise a peer like any other: answers `LiveJoin` and `LiveGet`, publishes itself (§5).
- SHOULD take only as many neighbours as its upload can serve at stream rate, at least 3
  *(initial)*, and serve longest-standing ones first, so each segment leaves it several times
  before late joiners are served. Rate measurement: local policy.
- Segment SHOULD be 1 to 4 s long. Shorter = less delay, more records and exchanges.
- Broadcast ends with an `end` record (§2.2). Broadcaster cut off and returning within 10 min
  *(initial)*, before viewers leave (§3.4), MAY continue with the next `seq` of the same
  `start`, and sets `gap`.

---

## 5. Finding peers

### 5.1 The `LIVE` record

Node in a stream, broadcaster included, publishes a record of kind `LIVE` (6) under target
`stream`.

- A source record in everything but lifetime: sent in `Store.source`, bound to the connection,
  not signed, returned in `KadAnswer.sources` ([`Protocol.md`](../Protocol.md) §10.3).
- **Lifetime 10 min** *(initial)*; stored again every 5 min while in the stream. Leaving node
  lets it run out.
- Per-target limits as for `SOURCE` records: at most 8 *(initial)* per host and 32 per site
  ([`ip-diversity.md`](ip-diversity.md) §2.4), at most 10,000 *(initial)* records under one
  target; a further node's record is `FULL`, a held one renewed
  ([`Protocol.md`](../Protocol.md) §10.8).
- Index node doesn't keep it past its lifetime ([`Protocol.md`](../Protocol.md) §11): a viewer
  that left drops out of the list as on any node.
- Every `ROUTER` of wire version 1 MUST store and return this kind, whether or not it sets `LIVE`; needs no
  stream knowledge for that.
- **A node under its anonymous key** ([`hops.md`](hops.md) §1) stores an `ANON` record under
  `stream` (or §10.4's target) instead of `LIVE`, via a `HopLookup` ([`hops.md`](hops.md) §4),
  `expires` ≤ 10 min *(initial)* ahead, every 5 min, once it holds a segment. Viewer setting
  `ANON` also searches kind `ANON` there, reaches those nodes via their gates. Broadcaster under its anonymous key is found only by viewers setting `ANON`;
  its `ANON` records count for "on air" below.

Stream on air when its target holds `LIVE` or `ANON` records, off when none, per a node's own
lookups; node not setting `ANON` sees only `LIVE`. `LIVE` is a kind of its own because of the short lifetime: a
`SOURCE` record lives 5 h, and most viewers listed that long would have left.

### 5.2 Joining

1. From a link or channel file the viewer knows `key`, `id`, hence `stream`.
2. Looks up `LIVE` records under `stream`.
3. Sends `LiveJoin` to peers from the answer and to peers in `LiveState.peers`, until it has its
   neighbours. Failed dials count under the limits of
   [`ip-diversity.md`](ip-diversity.md) §2.5.
4. Publishes own `LIVE` record once it holds a segment.

After the first neighbours a viewer needs Kad again only when short of peers.

---

## 6. Fairness

Viewer taking segments, giving none back, uses up the stream for others. So every node rates
neighbours and treats them by rank. Rating is **local**: per neighbour node ID, only own
measurements, told to nobody.

### 6.1 What is measured

Per neighbour, node keeps the outcome of its own last 20 *(initial)* requests to it.

| Outcome | Counts as |
|---|---|
| Whole segment, or every block asked for, arrived within §3.2's time (4 s + 1 s per MiB asked) | 1 hit |
| `Error{BUSY}` (with §6.2's rule), no answer in time, a bad block, a record whose signature fails (§2.2), or `NOT_FOUND` for a segment shown in `have` | 1 miss |
| Same for an **urgent** request | 1 outcome counting twice in the rate (2 hits or 2 misses), in one of the 20 places |
| A request the node reset itself | nothing |

```
rate = hits / (hits + misses)
```

- Request is **urgent** when its segment is within 3 segments *(initial)* of what node is
  playing. **What the node is playing** = oldest segment from its join point (§3.3) not yet
  handed to the player; urgent when `seq` < that one plus 3. Only the asking node knows that;
  only its own count uses it. Neighbour serving old segments promptly but letting the urgent one
  wait loses rank fast, which a byte count wouldn't show.
- **A neighbour never asked has no rate and isn't blamed.** Giving nothing because nobody asked
  isn't taking.
- **Probe.** No request to a neighbour for 60 s *(initial)* → ask it for one block of a segment
  its `have` shows, count the outcome. Only request for data the node doesn't need.

### 6.2 Ranks

| Rank | Reached when | How its requests are treated |
|---|---|---|
| **new** | on becoming neighbour | served while upload free; 1 request at a time |
| **steady** | neighbour 2 min, ≥ 5 outcomes, rate ≥ 80 % | served before **new**; 2 requests at a time |
| **core** | neighbour 5 min, ≥ 10 outcomes, rate ≥ 90 % | served first; 4 requests at a time; last dropped when node has too many neighbours; first in `peers` |

All numbers are *(initial)*.

- **Down.** Rate < 50 % with ≥ 5 outcomes → neighbour back to **new**; its time as neighbour
  restarts; kept outcomes cleared. Applies at every rank, **new** included.
- **Out.** Second Down for same neighbour within 10 min *(initial)* of the previous ends it.
  Out checked first: when rate falls < 50 % with ≥ 5 outcomes, node looks for an earlier Down in
  those 10 min; only without one is it a Down. On Out: close live exchange, answer a `LiveJoin`
  of that node ID with `LIVE_BLOCKED` for 30 min, this stream only. Down time kept per node ID,
  also across leave and return.
- Request above rank's allowance → `Error{BUSY}`. `BUSY` counts as miss only when node had at
  most one request outstanding there (within every rank's allowance); `BUSY` for a further request is
  its own doing, counts nothing.
- Request counts against allowance from its `LiveGet` until node handed every block asked on its
  exchange to the transport, or exchange closed.
- Limited upload: **core** request blocks go to transport before **steady**'s, those before
  **new**'s. Max wait for upload: local policy; counts against allowance while waiting.
- **Core limits.** At most 1 **core** neighbour per site, 2 per block; further ones stay
  **steady**. So one network can't hold a node's whole inner circle.
- **Sudden loss.** Node with ≥ 3 **core** neighbours losing more than a third within 2 min
  neither lowers a rank nor ends a neighbour for the next 2 min. Core neighbour lost = live
  exchange closed not by the node, or silent 30 s (§3.1); demotion isn't loss. A network fault
  looks like many peers failing at once; punishing through it would tear the mesh apart; with
  1–2 core neighbours one failure says nothing of the kind.
- **Returning neighbour** within 30 min starts at former rank less one. Ranks kept per node ID,
  which a peer can't change with its address ([`Protocol.md`](../Protocol.md) §6.1).
- Broadcaster ranks the same way; nothing to ask for, so rates by probes alone.

### 6.3 What this does and does not do

- Rewards answering peers, by reliability.
- Can't see whether a neighbour serves *others*. A peer could serve only nodes rating it; costs
  it as much upload as being fair to that many nodes.
- New node ID starts at **new**: discarding an identity gains only a clean record at lowest
  rank.
- Being served by a neighbour doesn't require uploading to it. A rarely asked viewer at the mesh
  edge still rises by answering probes.

---

## 7. Cost

| | Size |
|---|---|
| Segment record | ~150 B per segment, to each neighbour once |
| `LiveState` with a window update | ~20 B, at most once per second per neighbour |
| `LIVE` record | one store per 5 min, on ~8 nodes |
| Probe | ≤ one block per neighbour and minute, only to neighbours not otherwise asked |
| Per segment fetched | one exchange; `LiveGet` ~55 B, block requests and, above one block, a hash answer of 32 B per block |

At 2 s segments, 6 neighbours, 30-segment window: ~0.5 KB/s signalling vs a stream of some
hundred KB/s.

**Delay.** Segment reaches a viewer one round trip + one transfer after reaching the neighbour
it's fetched from. Starting 3 segments back, viewer is 3 segments + mesh depth behind the
broadcaster. Not for conversation.

---

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| **Pushing data to neighbours as well** | Saves a round trip per hop. Node with several neighbours gets a segment several times unless pushes cancelled in time; round trip small vs a segment of seconds |
| **Push trees** | Lowest delay and overhead. Leaving parent cuts off all below; repair needs own rules |
| **One signature over the whole segment** | Segment must arrive fully before anything is known; bad sender among several not found. With a root, each block checked on arrival |
| **A fresh key for every broadcast** | No key to keep. But new link per broadcast; a lasting channel needs a second mechanism on top |
| **Using the node's identity key** | No extra key. Channel then tied to one node and names it |
| **`SOURCE` records for peers** | Nothing new in Kad. They live 5 h |
| **A head record for the newest segment** (as for file sets) | Kad far too slow for a record changing every 2 s. Records travel between neighbours |
| **Fairness as local policy only** | Shortest. Peer withholding the urgent segment while serving old ones would meet no common answer |
| **Counting bytes given against bytes taken** | Simple. Blames a peer nobody asked, doesn't see late delivery |
| **Onion paths between viewer and broadcaster** | A project of its own. Viewer's address visible to neighbours, as a downloader's to sources |
| **Error-correcting codes across segments** | QUIC retransmits; a late segment is fetched elsewhere |

---

## 9. Comparison with a live extension of eD2K

One eMule-based client has a live TV subsystem. Compared from its source and own description;
nothing was run.

| | That client | This extension |
|---|---|---|
| Stream name | 16 B of hash of a fresh key per broadcast; lasting channels a separate, experimental layer | channel key and `id`; broadcast = a `start` under it |
| Segment | 2 s HLS transport-stream file from an external encoder | any container; ≤ 4 MiB |
| Integrity | SHA-256 and a signature over whole segment, checked after arrival; second, unsigned packet form for forwarding | signed record with BLAKE3 root; each block checked on arrival; one form |
| Large segments | fragments above 1.8 MB, as a packet is limited to 2 MB | blocks, as for files |
| Window | 16 segments, fixed by a 16-bit map | `window` from channel file; map of any length |
| State between peers | map every second, announcement every 2 s | on change, else every 10 s |
| Data | pulled once per second, pushed to subscribers | pulled; only records pushed |
| Transport | eD2K connection, one byte stream | one QUIC exchange per segment |
| Finding a stream | keyword entries with own tags under a common word, the title and a hash; stored among file entries | channel file is a file; `LIVE` records under stream's target |
| Finding peers | from broadcaster, peer exchange on the map packet, disk cache | `LIVE` records, `peers` from neighbours |
| End | signed end packet, remembered 30 min | `end` record, remembered 30 min |
| Fairness | three levels by response rate, probes, punctuality near play position, per-subnet limits, emergency mode | same ideas, rated locally per node ID (§6) |
| Peer identity | user hash, after pointer-keyed state proved unstable | node ID, proven by handshake |
| Firewalled broadcaster | own hole punching and relay | `Protocol.md` §9, unchanged |
| Privacy | onion paths with fixed-size cells, cover traffic, Reed-Solomon striping | none beyond transport |

**Taken from it:** rating by response rate, not volume; the probe; double weight near the play
position; starting 3 segments behind the newest; a refusal naming other peers; the signed end;
no clock time in a segment.

**Not taken:** its own relay and hole punching, the onion layer, the fixed 16-segment window,
pushing data, discovery via tagged keyword entries.

---

## 10. Private channels

Watchable only by those given its link, which carries a **secret**; everything setting a private
channel apart derives from it.

```
ed3k:live?key=<channel key>&id=<id>&s=<secret, 32 bytes, base32>[&dn=<url-encoded title>]
```

No channel file; never published under keywords. `media` and `codecs` come from the first
neighbour (§3.1), title from the link.

**The link is the credential.** A client MUST NOT write the secret to a log or show it in an
error message, and SHOULD mark such a link as private wherever it shows it.

### 10.1 Keys

```
k       = H("ed3k v1 live private", secret ‖ key ‖ id)                    32 bytes
stream  = H("ed3k v1 live private stream", k)[0..16)
k_data  = H("ed3k v1 live private data", k)                               32 bytes
target  = H("ed3k v1 live private target", k ‖ period(8))[0..16)          period = Unix time / 3600, rounded down, big-endian
```

`stream` replaces §1.1's value in every message and in the segment-record signature (§2.2).
Records signed with the channel key as before, so a viewer can't forge a segment.

### 10.2 What is encrypted

Every segment and init data is encrypted before hashing:

```
stored bytes = nonce(12) ‖ ChaCha20(k_data, nonce, plain bytes)          RFC 8439 cipher, counter from 0
```

- `nonce` random for every segment and init data.
- `root`, `size` in a segment record are of the stored bytes. Peers verify, keep, pass on blocks
  exactly as in an open channel, needing no key.
- No authentication tag: the signed root already fixes every byte.
- §2.1's limits apply to stored bytes.
- Encrypted data doesn't compress; the rule of [`Protocol.md`](../Protocol.md) §12.3 then sends it as is.

Segment record itself isn't encrypted: it travels only between neighbours, who have shown they
hold the secret (§10.3).

### 10.3 Neighbours prove the secret

A node knowing only `stream`, e.g. one sent a `LiveJoin` for the channel, must not be able to
join. (A storing node sees only `target`, §10.4.)

```
LiveJoin.proof   = H("ed3k v1 live private join",   k ‖ joiner's node ID ‖ peer's node ID)[0..16)
LiveState.proof  = H("ed3k v1 live private answer", k ‖ joiner's node ID ‖ peer's node ID)[0..16)
```

- Both node IDs are those of the authenticated connection ([`Protocol.md`](../Protocol.md) §6.2), so a proof is useless to
  a third node and can't be replayed on another connection.
- `LiveJoin` w/o valid proof → `refusal = LIVE_LEFT` (answer of a node not in the stream), nothing else: no
  `proof`, no `peers`, no record, §2.2's `end` record included.
- **Every answer to a valid proof carries `proof`, whatever its `refusal`:** an answer taking
  the joiner, `LIVE_FULL` with `peers` as §3.1 asks, `LIVE_BLOCKED`, and `LIVE_LEFT` with the
  `end` record after the broadcast is over. `end` record sent only to a valid proof.
- Joiner closes the exchange when the first `LiveState` has no valid proof, and reads `peers`
  from one that has, whatever its `refusal`.
- `peers`, records and segments go only to neighbours that passed this.

### 10.4 Finding peers

Peers publish and look up `LIVE` records under `target` (§10.1), not `stream`.

- Target changes hourly; storing nodes differ per hour, so no node set sees who watches over a
  long time.
- Peer stores under current hour's target. Viewer finding nothing also looks under the previous
  hour's in an hour's first 10 min *(initial)*, and under the next hour's in the last 10 min
  (peers with clocks ahead).
- Storing node sees a target it can't tie to a channel, and publishing peers' addresses; can't
  join (§10.3).

### 10.5 What it does and does not give

| | |
|---|---|
| Someone without the link | can't find the channel, join it, or read a segment |
| A node that stores the `LIVE` records | learns some peers share something for an hour, and their addresses |
| A viewer | sees its neighbours' addresses, as in any stream |
| A viewer who passes the link on | gives full access. Viewers aren't told apart |
| Taking access away | impossible. New secret = new channel; every viewer needs the new link. A paid channel gives each viewer an own pass that ends ([`subscription.md`](subscription.md)) |
| A secret that leaks later | opens every recorded segment of that channel. No forward secrecy |
| The broadcaster | known to neighbours as node with newest segments, not by name |

### 10.6 Alternatives considered

| Alternative | Why not |
|---|---|
| **Encrypting the transport only** | Already encrypted. Keeps observers out, not peers: anyone finding the stream could join |
| **A key per viewer**, so one viewer can be removed | Broadcaster would wrap a content key per viewer, change it on every removal. Private channel keeps one secret; per-viewer access with epoch keys = paid subscription extension ([`subscription.md`](subscription.md)) |
| **Hash first, then encrypt per neighbour** | Peers couldn't verify or pass on blocks w/o decrypting; every hop would encrypt again |
| **An authenticated cipher** | Adds 16 bytes and a second check of what the signed root already fixes |
| **A fixed Kad target** | Simpler. Same few nodes would see every viewer as long as the channel exists |
| **A password typed by the user** in place of a random secret | Guessable off line by anyone who recorded a segment |

---

## 11. Under consideration: listing channels that are on air

Not part of this draft. A `LIVE` record shows a channel on air only to someone knowing the
channel. Today a search with `Query.ext` = `ed3klive` finds channel files; whether each runs is
a separate lookup. The name ending is the name's extension
([`Protocol.md`](../Protocol.md) §10.1), not a keyword, so channel files share no keyword target.

Four combinable ways to list running channels are considered for this extension's next version.

Options 1, 3, 4 rest on an **on-air statement**: channel key signs `stream`, `start` and a
valid-until time ≤ 15 min ahead, renewed every 5 min; ~110 bytes. Without it anyone could mark
any channel on air. Exists already: on-air notice of [`pubsub.md`](pubsub.md) §4.2, pushed to
a channel's followers. Open here: storing and searching it.

| | Option | Pro | Con |
|---|---|---|---|
| 1 | **Statement in keyword records.** Channel file's keyword records carry it while the channel runs; `Query` gets an "on air only" flag. Storing node drops the statement on expiry, keeps the record | uses existing keyword search; claim checked by storing node and searcher; no new hot spot | finds channels by a word, not all; a field in the signed record and a filter every `ROUTER` must apply |
| 2 | **One well-known target**, or one per language. Every running channel stores a record there | full list in one lookup | few nodes nearest the target carry every channel, the place to attack or flood; per-target limit ([`Protocol.md`](../Protocol.md) §10.8) caps the list |
| 3 | **Index nodes** ([`Protocol.md`](../Protocol.md) §11). Holds records for every target, can answer "all channels on air", sorted, paged | complete, fast; fits what a server is | as complete as that index node; optional, central |
| 4 | **Passing statements on.** Neighbours send each other statements of other known channels, in `LiveState` | no Kad load | reaches only nodes already watching a channel; never complete |

Current preference: 1 with 3. Option 1 gives every node a search for running channels on a
subject; the statement keeps an index node from inventing channels. Option 2 recreates a
central point inside Kad w/o a server's capacity.

---

## 12. Open points

- Every number is *(initial)*, none measured; the ranks in §6.2 most of all.
- No list of channels on air (§11).
- Several qualities (bit rates) of one channel not described; can be several channels today.
- A successor for a lost or stolen channel key, as for a set head
  ([`file-set.md`](file-set.md) §10).
- Whether a viewer that can't upload at all (no free upstream) should be able to say so.
- A private channel can't take a viewer's access away (§10.5). A paid channel can, at the end
  of a viewer's pass ([`subscription.md`](subscription.md) §5).
