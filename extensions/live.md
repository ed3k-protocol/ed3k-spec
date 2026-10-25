# Extension: live streams (`live`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3, §12 and §13. A section number (§) that exists in this document refers to this document; `Protocol.md` is named where its section of the same number is meant. §7.2 is a section of `Protocol.md`.
**Capability:** bit 11, `LIVE`. Message types 19 to 21. Record kind 6.
**Required of every `ROUTER`:** storing and returning the record kind `LIVE` (§5). The rest is optional.

A live stream is media that is produced while it is watched. A broadcaster cuts it into
**segments** of a few seconds. Viewers fetch each segment from one another shortly after it
exists, and keep only the newest ones.

The extension adds little of its own:

- A segment is an ordinary file with a BLAKE3 root. It is verified block by block as any file
  is (§7.2).
- The broadcaster signs one small **segment record** per segment. Peers forward it unchanged.
- A channel is described by a small **channel file**, shared and searched like any file.
- Peers of a stream are found through one new record kind.
- NAT traversal (`Protocol.md` §9), compression and the bad-block rule (both `Protocol.md` §12.3) apply
  unchanged.

The media is opaque to the protocol. Encoding and playing are the application's business.

---

## 1. Channel

### 1.1 Naming

A channel has a **channel key** (Ed25519, public key 32 bytes) and an `id` of 1 to 16 bytes that
the owner chooses, so one key can have several channels.

```
stream = H("ed3k v1 live", key ‖ id)[0..16)
```

`stream` is the Kad target of the channel (§5) and names it in every message.

- The channel key SHOULD be a key of its own, not the node's identity key ([`Protocol.md`](../Protocol.md) §6.1). The channel
  then outlasts the node, can be broadcast from another node, and does not name the
  broadcaster's node ID in its link.
- A channel has at most one **broadcast** at a time. A broadcast is named by its `start`: the
  Unix time in seconds at which it began. A later broadcast has a later `start`.

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

- The signature is over `"ed3k v1 live channel" ‖ body` and is REQUIRED. A reader checks it
  against `key` and refuses the file when it fails. The rules for `sigs` are those of
  `Protocol.md` §10.7.
- A channel file MUST NOT exceed 64 KiB. `window` is 8 to 240 *(initial)*; 0 or absent still
  means 30. A reader refuses a channel file that is larger or has another `window`, so the
  first `LiveState` of §3.1 carries at most 240 records, a viewer keeps at most that many
  segments, and a window is never too short to join 3 segments behind (§3.3).
- It is shared, published under its keywords and downloaded like any file ([`Protocol.md`](../Protocol.md) §10 and §12). A search
  finds channels as it finds files; the suggested name ends in `.ed3klive`.
- It describes the channel, not a broadcast. It changes only when the description does.
  A reader that holds two files for one `stream` uses the one with the later `created`.

### 1.3 Links

```
ed3k:live?key=<channel key, base32 lower case, no padding>&id=<id, base32>[&dn=<url-encoded title>]
```

The link is enough to watch: it gives `stream` and the key that segment records are checked
against. `media` and `codecs` then come from the first peer (§3.1). A client without this
extension does not know the link. `key`, `id` and the `s` of §10 are base32 as
[`Protocol.md`](../Protocol.md) §13.2 fixes: written lower case without padding, read either way.
A link is invalid when `key` does not decode to 32 bytes, `id` to 1 to 16 bytes or `s` to 32
bytes, or when `key`, `id`, `s` or `dn` is repeated.

---

## 2. Segments

### 2.1 Segment

- A segment is a byte string of at most 4 MiB, one piece (§7.2). Its identity is its BLAKE3
  root, and its blocks are verified as those of a file.
- Segments of a broadcast are numbered by `seq` from 0, without gaps.
- Each segment MUST be playable on its own once the init data is known, so a viewer can start
  at any segment.
- **Init data** is what a container needs before its first segment, such as the initialisation
  segment of fragmented MP4. It is a byte string of at most 256 KiB, named by its root. A
  container that needs none has none.

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

The signature is over

```
"ed3k v1 live segment" ‖ stream(16) ‖ body
```

and is made with the channel key. The string is not a prefix of the one that signs the
channel file (§1.2), nor is that one a prefix of this one.

- A record is about 150 bytes. A `LiveRecord` MUST NOT exceed 1 KiB. A node drops a larger one
  unread and does not forward it; it is not a miss for the neighbour that sent it.
- The rules for `sigs` are those of `Protocol.md` §10.7.
- A node accepts a record only when the signature is valid, and drops it otherwise. A neighbour
  that sent it counts one miss (§6), which takes one of the 20 places as a request outcome
  does.
- **Limits.** A record with a valid signature is still invalid when its `size` is above 4 MiB,
  its `init_size` is above 256 KiB (§2.1), or `root` is absent without `end`. A node drops it
  and does not forward it. It is not a miss for the neighbour that sent it: the channel key
  signed it, and the neighbour only passed it on.
- **Order.** Records are ordered by `start`, then `seq`. A node ignores a record of an earlier
  `start` than the newest it has seen, and a `start` more than 1 h *(initial)* ahead of its
  clock. An old broadcast can therefore not be played back into a running one.
- **Behind the window.** A node drops, without forwarding, a record whose `seq` is at or below
  the newest `seq` it holds of that broadcast minus `window`, that is outside the newest
  `window` segments (§3.3; nothing while the newest `seq` is below `window`). Old records of a running broadcast then do not travel through the mesh
  again. Such a record is not invalid and counts no miss.
- **Two records for one `seq`** of one broadcast with different content are proof that the
  channel key signed both. The one with the lower `root`, compared as a byte string, wins: a
  node keeps it, passes it on and drops the other, also when it had already fetched that
  segment. Every node then holds the same bytes under a `seq`, and `have` bits mean the same
  everywhere.
- `end` closes the broadcast. Nodes forward it, keep it for 30 min *(initial)*, and leave the
  stream: they stop renewing their own `LIVE` records at once, and for those 30 min answer a `LiveJoin`
  with the `end` record and `refusal = LIVE_LEFT`, so that a late viewer learns that the
  broadcast is over. In a private channel only a `LiveJoin` with a valid proof is answered so
  (§10.3). There is no other way to end a broadcast than this record or silence (§3.4). An `end`
  record has the `seq` after the last segment's, and `root`, `size`, `ms`, `init` and
  `init_size` absent.
- For one `seq`, a record without `root` loses against one with a `root`, and of two without
  the first verified is kept: the rule above cannot compare absent roots.
- A record carries no wall-clock time of the segment. `seq` gives the order, and a time stamp
  would only expose the broadcaster's clock.

### 2.3 Formats

The protocol carries any format. Two clients can still follow it and fail to play each other's
channels, so there is a baseline.

| | Baseline |
|---|---|
| Container | fragmented MP4 (CMAF, ISO/IEC 23000-19), `media` = `video/mp4` or `audio/mp4` |
| Video | H.264 (AVC), High profile, up to level 4.1 (1080p at 30 frames per second) |
| Audio | AAC-LC or Opus |
| Init data | the initialisation segment (`ftyp` and `moov`) |
| Segment | one or more fragments (`moof` and `mdat`), starting with a key frame |

- A viewer SHOULD be able to play the baseline. A broadcaster SHOULD use it unless it knows its
  audience can play something else.
- **Audio only** is a channel like any other, with `media` = `audio/mp4` and no video track.
- Other formats are allowed and named in `media` and `codecs`: MPEG transport stream segments
  (`video/mp2t`, no init data), WebM, or newer codecs such as H.265 and AV1 inside MP4.
- A viewer that cannot play what `media` and `codecs` name says so, and does not join.
- How segments are produced and played is not part of the protocol. No particular encoder or
  tool is required.

The baseline is what browsers, phones and common players decode without added software, and it
is the segment form that HLS and DASH share.

---

## 3. Between two peers

Two peers that exchange a stream are **neighbours**. One of them opened the **live exchange**;
after that both sides are equal.

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

- A viewer opens an exchange and sends `LiveJoin`. The peer answers with `LiveState`: its
  window, the records of every segment in it, and peers if asked.
- A peer that does not take the viewer answers with `refusal` set and closes. With `LIVE_FULL` it
  SHOULD still fill `peers`, so the viewer has somewhere else to go. A peer that is not in the
  stream answers `LIVE_LEFT`.
- After the first answer **both sides send `LiveState`** on the exchange whenever one of these
  happens:
  - It learns a new valid record: it sends the record at once to every neighbour that has not
    sent it that record. This is how records travel, and it is the only thing that is pushed, apart from the keys
    and offers of a paid channel ([`subscription.md`](subscription.md) §6.3).
  - It completes a segment: `first` and `have` are sent, at most once per second *(initial)*.
  - 10 s *(initial)* have passed without one: an empty `LiveState` as a sign of life.
- A neighbour that sent nothing for 30 s *(initial)* is gone.
- Either side leaves by closing the exchange.
- `peers` MUST be peers the sender has itself had as verified neighbours of this stream within
  the last 10 min *(initial)*.
- Two nodes keep at most one live exchange per stream. When two are open, both keep the one
  opened by the node with the lower node ID, compared as bytes; of two opened by the same node,
  the newer. The node that opened the other closes it. The other node does not answer on it,
  and resets it when it is still open 10 s *(initial)* later. This is no refusal, no miss and
  no loss (§6.2), and the two count as one neighbour.
- **Limits.** `LiveJoin` names at most 2 relays per family, at addresses acceptable in the
  network (`Protocol.md` §8.2 (7)). `LiveState` carries at most 240 records and a `have` of at
  most 240 bits (30 bytes), the largest window. A message above these limits is malformed,
  and the node closes the exchange.
- `first` is the sender's choice. It SHOULD be the `seq` of the oldest record it holds within
  its window, so that `have` stays within `window` bits. A receiver accepts any `first`, and
  does not ask for a segment that is behind the window (§2.2).

### 3.2 Fetching a segment

Data is never pushed. A node asks a neighbour whose `have` shows the segment.

- The node opens a **new exchange per segment** and sends `LiveGet`, which has no answer of
  its own: it names the segment of this exchange. The node then sends `BlockRequest`s and
  `HashRequest`s on the same exchange without waiting, as in `Protocol.md` §12.2 to §12.4,
  with offsets inside the segment and at most 16 ranges outstanding; the neighbour answers
  with `Block`s and `HashAnswer`s. There is no queue position (`FileStatus.rank`,
  `Protocol.md` §12.5): a neighbour serves at
  once, or answers `Error{BUSY}`, `Error{NOT_FOUND}` or `Error{DENIED}` and closes the
  exchange.
- `LiveGet` is accepted only from a neighbour, that is while a live exchange with that node is
  open. Anything else is answered `Error{DENIED}`.
- A segment of more than one block may be fetched from several neighbours at once, by block.
- **Init data** is asked of a neighbour whose `have` shows a segment whose record names it.
  `Error{NOT_FOUND}` from such a neighbour counts as for a segment (below), unless every such
  segment has left its window since.
- A node that no longer wants a segment resets the exchange. One lost packet delays only the
  segment it belongs to, because each segment has an exchange of its own.
- `Error{NOT_FOUND}` for a segment that the neighbour's last `have` showed counts as a miss
  (§6), unless the segment has left its window since.
- A node gives up on a request after 4 s *(initial)* plus 1 s per MiB it asked for, and asks
  another neighbour.

### 3.3 The window

- A node keeps the newest `window` segments of the running broadcast (§1.2; 30 when it does
  not know the channel file) and the init data they name. Older ones are deleted.
- A viewer that joins starts 3 segments *(initial)* behind the newest record it was given. It
  does not fetch older ones.
- A node that falls out of the window, because it could not keep up, starts again in the same
  way.
- When a record has `gap`, or names other init data than the one before, the player is reset
  at that segment.

### 3.4 Neighbours and silence

- A node aims at 6 neighbours and keeps between 3 and 8 *(initial)*. A broadcaster takes as
  many as its upload allows (§4), also more than 8.
- Neighbours are subject to the diversity limits of a bucket: at most 1 per host, 2 per site
  and 4 per block ([`ip-diversity.md`](ip-diversity.md) §1 and §2).
- When no new record has arrived for 60 s *(initial)*, the broadcast counts as interrupted. A
  viewer keeps its neighbours and looks for others (§5). After 10 min *(initial)* without a
  record it leaves.

---

## 4. Broadcasting

- The broadcaster is the node that holds the channel key, or receives signed records from
  whoever does. Apart from that it is a peer like any other: it answers `LiveJoin` and
  `LiveGet` and publishes itself (§5).
- It SHOULD take only as many neighbours as its upload can serve at the stream's rate, at
  least 3 *(initial)*, and serve its longest-standing neighbours first, so that each segment
  leaves it several times before it serves late joiners. How it measures the rate is local
  policy.
- A segment SHOULD be 1 to 4 s long. Shorter segments lower the delay and raise the number of
  records and exchanges.
- A broadcast ends with an `end` record (§2.2). A broadcaster that was cut off and returns
  within 10 min *(initial)*, before viewers leave (§3.4), MAY continue with the next `seq` of the same `start`, and sets `gap`.

---

## 5. Finding peers

### 5.1 The `LIVE` record

A node that is in a stream, the broadcaster included, publishes a record of kind `LIVE` (6)
under the target `stream`.

- It is a source record in everything but its lifetime. It is sent in `Store.source`, bound to
  the connection, not signed, and returned in `KadAnswer.sources` ([`Protocol.md`](../Protocol.md) §10.3).
- **Lifetime 10 min** *(initial)*; stored again every 5 min while the node is in the stream. A
  node that leaves lets it run out.
- The limits per target are those of `SOURCE` records: at most 8 *(initial)* per host and 32
  per site ([`ip-diversity.md`](ip-diversity.md) §2.4), and at most 10,000 *(initial)* records
  under one target; a record of a further node is `FULL`, and a held one is renewed
  ([`Protocol.md`](../Protocol.md) §10.8).
- An index node does not keep it past its lifetime ([`Protocol.md`](../Protocol.md) §11): a
  viewer that left drops out of the list as on any other node.
- Every `ROUTER` of wire version 1 MUST store and return this kind, whether or not it sets
  `LIVE`. It needs to know nothing about streams for that.
- **A node under its anonymous key** ([`hops.md`](hops.md) §1) stores an `ANON` record under
  `stream` (or the target of §10.4) in place of a `LIVE` record, through a `HopLookup`
  ([`hops.md`](hops.md) §4), with `expires` at most 10 min *(initial)* ahead, every 5 min,
  once it holds a segment. A viewer that sets `ANON` also searches kind `ANON` there and
  reaches those nodes through their gates. A broadcaster under its anonymous key is found
  only by viewers that set `ANON`; its `ANON` records count for "on air" below.

A stream is on air when its target holds `LIVE` or `ANON` records, and off when it holds none. The short
lifetime is the reason for a kind of its own: a `SOURCE` record lives 5 h, and most viewers
listed that long would have left.

### 5.2 Joining

1. From a link or a channel file the viewer knows `key`, `id` and so `stream`.
2. It looks up `LIVE` records under `stream`.
3. It sends `LiveJoin` to peers from the answer, and to peers named in `LiveState.peers`,
   until it has its neighbours. Failed dials count under the limits of
   [`ip-diversity.md`](ip-diversity.md) §2.5.
4. It publishes its own `LIVE` record once it holds a segment.

After the first neighbours a viewer needs Kad again only when it runs short of peers.

---

## 6. Fairness

A viewer that takes segments and gives none back uses up the stream for others. Every node
therefore rates its neighbours and treats them by rank. The rating is **local**: it is kept per
neighbour node ID, uses only what the node measured itself, and is told to nobody.

### 6.1 What is measured

For each neighbour a node keeps the outcome of its own last 20 *(initial)* requests to it.

| Outcome | Counts as |
|---|---|
| The whole segment, or every block asked for, arrived within the time of §3.2 (4 s plus 1 s per MiB asked for) | 1 hit |
| `Error{BUSY}` (with the rule of §6.2), no answer in time, a bad block, a record whose signature fails (§2.2), or `NOT_FOUND` for a segment shown in `have` | 1 miss |
| The same for an **urgent** request | 1 outcome that counts twice in the rate: 2 hits or 2 misses, in one of the 20 places |
| A request the node reset itself | nothing |

```
rate = hits / (hits + misses)
```

- A request is **urgent** when its segment is within 3 segments *(initial)* of what the node is
  playing. **What the node is playing** is the oldest segment from its join point (§3.3) that
  it has not yet handed to the player; a request is urgent when its `seq` is below that one
  plus 3. Only the asking node knows that, and only its own count uses it. A neighbour that
  serves old segments promptly and lets the urgent one wait therefore loses rank fast, which a
  count of bytes would not show.
- **A neighbour that was never asked has no rate and is not blamed.** Giving nothing because
  nobody asked is not taking.
- **Probe.** A node that has sent a neighbour no request for 60 s *(initial)* asks it for one
  block of a segment its `have` shows, and counts the outcome. A probe is the only request a
  node makes for data it does not need.

### 6.2 Ranks

| Rank | Reached when | How its requests are treated |
|---|---|---|
| **new** | on becoming a neighbour | served while upload is free; 1 request at a time |
| **steady** | neighbour for 2 min, at least 5 outcomes, rate ≥ 80 % | served before **new**; 2 requests at a time |
| **core** | neighbour for 5 min, at least 10 outcomes, rate ≥ 90 % | served first; 4 requests at a time; the last to be dropped when the node has too many neighbours; listed first in `peers` |

All numbers are *(initial)*.

- **Down.** A rate under 50 % with at least 5 outcomes puts a neighbour back to **new**, and
  its time as a neighbour starts again; the outcomes kept are cleared with it. Down applies at
  every rank, **new** included.
- **Out.** A second Down for the same neighbour within 10 min *(initial)* of the one before
  ends it. Out is checked first: when a rate falls under 50 % with at least 5 outcomes, the
  node looks for an earlier Down in those 10 min, and only without one is it a Down. On Out
  the node closes the live exchange and answers a `LiveJoin` of that node ID with
  `LIVE_BLOCKED` for 30 min, for this stream only. The time of a Down is kept per node ID,
  also when the neighbour leaves and returns.
- A request above the number its rank allows is answered `Error{BUSY}`. A node counts a `BUSY`
  as a miss only when it had at most one request outstanding at that neighbour, the allowance
  of every rank; a `BUSY` for a further request is its own doing and counts nothing.
- A request counts against the allowance from its `LiveGet` until the node has handed every
  block asked for on its exchange to the transport, or the exchange closed.
- With a limited upload, blocks of **core** requests are handed to the transport before those
  of **steady**, and those before **new**. How long a request may wait for upload is local
  policy; it counts against the allowance while it waits.
- **Core limits.** At most 1 **core** neighbour per site and 2 per block. A further one stays
  **steady**. One network can then not hold a node's whole inner circle.
- **Sudden loss.** A node that had at least 3 **core** neighbours and lost more than a third
  of them within 2 min neither lowers a rank nor ends a neighbour for the next 2 min. A core
  neighbour is lost when its live exchange closed without the node closing it, or when it sent
  nothing for 30 s (§3.1). A demotion is not a loss. A network fault looks like many peers
  failing at once, and punishing through it would take the mesh apart; with 1 or 2 core
  neighbours one failure says nothing of the kind.
- **A returning neighbour** within 30 min starts at the rank it had, less one. Ranks are kept
  per node ID, which a peer cannot change with its address ([`Protocol.md`](../Protocol.md) §6.1).
- The broadcaster ranks its neighbours in the same way. It has nothing to ask them for, so it
  rates them by probes alone.

### 6.3 What this does and does not do

- It rewards peers that answer, in the order of how reliably they do.
- It cannot see whether a neighbour serves *others*. A peer could serve the nodes that rate it
  and nobody else. That costs it as much upload as being fair to that many nodes.
- A new node ID starts at **new**, so discarding an identity gains nothing but a clean record
  at the lowest rank.
- Uploading to a neighbour is not a condition for being served by it. A viewer at the edge of
  the mesh that is rarely asked still rises by answering probes.

---

## 7. Cost

| | Size |
|---|---|
| Segment record | about 150 B per segment, to each neighbour once |
| `LiveState` with a window update | about 20 B, at most once per second per neighbour |
| `LIVE` record | one store per 5 min, on about 8 nodes |
| Probe | at most one block per neighbour and minute, only towards neighbours not otherwise asked |
| Per segment fetched | one exchange; a `LiveGet` of about 55 B, block requests and, above one block, a hash answer of 32 B per block |

At 2 s segments, 6 neighbours and 30 segments of window: about 0.5 KB/s of signalling, against
a stream of some hundred KB/s.

**Delay.** A segment reaches a viewer one round trip and one transfer after it reached the
neighbour it is fetched from. With the start 3 segments back, a viewer is 3 segments plus the
depth of the mesh behind the broadcaster. This is not a design for conversation.

---

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| **Pushing data to neighbours as well** | Saves one round trip per hop. A node with several neighbours then receives a segment several times unless pushes are cancelled in time, and the round trip is small against a segment of seconds |
| **Push trees** | Lowest delay and overhead. A parent that leaves cuts off everything below it, and the repair needs rules of its own |
| **One signature over the whole segment** | The segment must arrive completely before anything is known about it, and a bad sender among several is not found. With a root, each block is checked on arrival |
| **A fresh key for every broadcast** | Needs no key to keep. Every broadcast then needs a new link, and a channel that lasts needs a second mechanism on top |
| **Using the node's identity key** | No extra key. The channel is then tied to one node and names it |
| **`SOURCE` records for peers** | Nothing new in Kad. They live 5 h |
| **A head record for the newest segment** (as for file sets) | Kad is far too slow for a record that changes every 2 s. Records travel between neighbours |
| **Fairness as local policy only** | Shortest. A peer that withholds the urgent segment while serving old ones would meet no common answer |
| **Counting bytes given against bytes taken** | Simple. It blames a peer nobody asked, and it does not see late delivery |
| **Onion paths between viewer and broadcaster** | A project of its own. A viewer's address is visible to its neighbours, as a downloader's is to its sources |
| **Error-correcting codes across segments** | QUIC retransmits; a segment that is late is fetched elsewhere |

---

## 9. Comparison with a live extension of eD2K

One eMule-based client carries a live television subsystem. The comparison is from reading its
source and its own description; nothing was run.

| | That client | This extension |
|---|---|---|
| Stream name | 16 bytes of a hash of a fresh key per broadcast; lasting channels are a separate, experimental layer | channel key and `id`; a broadcast is a `start` under it |
| Segment | an HLS transport-stream file of 2 s, made by an external encoder | any container; at most 4 MiB |
| Integrity | SHA-256 and a signature over the whole segment, checked after it arrived; a second, unsigned packet form exists for forwarding | signed record with a BLAKE3 root; every block checked on arrival; one form |
| Large segments | split into fragments above 1.8 MB because a packet is limited to 2 MB | blocks, as for files |
| Window | 16 segments, fixed by a 16-bit map | `window` from the channel file; a map of any length |
| State between peers | a map every second, an announcement every 2 s | sent when something changes, and every 10 s otherwise |
| Data | pulled once per second, and pushed to subscribers | pulled; only records are pushed |
| Transport | the eD2K connection, one byte stream | one QUIC exchange per segment |
| Finding a stream | keyword entries with tags of their own, under a common word, the title and a hash; stored among file entries | the channel file is a file; `LIVE` records under the stream's target |
| Finding peers | from the broadcaster, peer exchange on the map packet, a cache on disk | `LIVE` records, and `peers` from neighbours |
| End | signed end packet, remembered 30 min | `end` record, remembered 30 min |
| Fairness | three levels by response rate, probes, punctuality near the play position, limits per subnet, an emergency mode | the same ideas, rated locally per node ID (§6) |
| Peer identity | the user hash, after pointer-keyed state proved unstable | the node ID, proven by the handshake |
| Firewalled broadcaster | hole punching and a relay of its own | `Protocol.md` §9, unchanged |
| Privacy | onion paths with fixed-size cells, cover traffic, striping with Reed-Solomon codes | none beyond the transport |

**Taken from it:** rating by response rate, not by volume; the probe; double weight near the
play position; starting 3 segments behind the newest; a refusal that names other peers; the
signed end; no clock time in a segment.

**Not taken:** its own relay and hole punching, the onion layer, the fixed 16-segment window,
pushing data, and discovery through tagged keyword entries.

---

## 10. Private channels

A private channel can be watched only by those who were given its link. The link carries a
**secret**; everything that sets a private channel apart is derived from it.

```
ed3k:live?key=<channel key>&id=<id>&s=<secret, 32 bytes, base32>[&dn=<url-encoded title>]
```

A private channel has no channel file and is never published under keywords. `media` and
`codecs` come from the first neighbour (§3.1), the title from the link.

**The link is the credential.** A client MUST NOT write the secret to a log or show it in an
error message, and SHOULD mark such a link as private wherever it shows it.

### 10.1 Keys

```
k       = H("ed3k v1 live private", secret ‖ key ‖ id)                    32 bytes
stream  = H("ed3k v1 live private stream", k)[0..16)
k_data  = H("ed3k v1 live private data", k)                               32 bytes
target  = H("ed3k v1 live private target", k ‖ period(8))[0..16)          period = Unix time / 3600, rounded down, big-endian
```

`stream` replaces the value of §1.1 in every message and in the signature of a segment record
(§2.2). Records are signed with the channel key as before, so a viewer cannot forge a segment.

### 10.2 What is encrypted

Every segment and every init data is encrypted before it is hashed:

```
stored bytes = nonce(12) ‖ ChaCha20(k_data, nonce, plain bytes)          RFC 8439 cipher, counter from 0
```

- `nonce` is random for every segment and every init data.
- `root` and `size` in a segment record are those of the stored bytes. Peers verify, keep and
  pass on blocks exactly as in an open channel, and need no key to do so.
- There is no authentication tag. The signed root already fixes every byte.
- The limits of §2.1 apply to the stored bytes.
- Encrypted data does not compress; the rule of [`Protocol.md`](../Protocol.md) §12.3 then sends it as it is.

A segment record itself is not encrypted. It travels only between neighbours, and those have
shown that they hold the secret (§10.3).

### 10.3 Neighbours prove the secret

A node that only knows `stream`, for example one that was sent a `LiveJoin` for the channel,
must not be able to join. (A storing node sees only `target`, §10.4.)

```
LiveJoin.proof   = H("ed3k v1 live private join",   k ‖ joiner's node ID ‖ peer's node ID)[0..16)
LiveState.proof  = H("ed3k v1 live private answer", k ‖ joiner's node ID ‖ peer's node ID)[0..16)
```

- Both node IDs are those of the authenticated connection ([`Protocol.md`](../Protocol.md) §6.2), so a proof is of no use to
  a third node and cannot be replayed on another connection.
- A peer answers a `LiveJoin` without a valid proof with `refusal = LIVE_LEFT`, the answer of a
  node that is not in the stream. It sends nothing else: no `proof`, no `peers` and no record,
  the `end` record of §2.2 included.
- **Every answer to a valid proof carries `proof`, whatever its `refusal`:** an answer that
  takes the joiner, `LIVE_FULL` with `peers` as §3.1 asks, `LIVE_BLOCKED`, and `LIVE_LEFT`
  with the `end` record after the broadcast is over. The `end` record is sent only to a valid
  proof.
- A joiner closes the exchange when the first `LiveState` has no valid proof, and reads `peers`
  from one that has, whatever its `refusal`.
- `peers`, records and segments go only to neighbours that passed this.

### 10.4 Finding peers

Peers publish and look up their `LIVE` records under `target` (§10.1), not under `stream`.

- The target changes every hour. The nodes that store the records of one hour are not those of
  the next, so no set of nodes sees who watches over a long time.
- A peer stores under the target of the current hour. In the first 10 min *(initial)* of an
  hour a viewer that finds nothing also looks under the target of the hour before, and in the
  last 10 min under the target of the hour after, for peers whose clocks are ahead.
- A storing node sees a target it cannot tie to a channel, and the addresses of the peers that
  publish there. It cannot join (§10.3).

### 10.5 What it does and does not give

| | |
|---|---|
| Someone without the link | cannot find the channel, join it, or read a segment |
| A node that stores the `LIVE` records | learns that some peers share something for an hour, and their addresses |
| A viewer | sees the addresses of its neighbours, as in any stream |
| A viewer who passes the link on | gives full access. Viewers are not told apart |
| Taking access away | not possible. A new secret is a new channel, and every viewer needs the new link. A paid channel gives each viewer a pass of their own that ends ([`subscription.md`](subscription.md)) |
| A secret that leaks later | opens every recorded segment of that channel. There is no forward secrecy |
| The broadcaster | is known to its neighbours as the node with the newest segments, not by name |

### 10.6 Alternatives considered

| Alternative | Why not |
|---|---|
| **Encrypting the transport only** | It is encrypted already. That keeps observers out, not peers: anyone who finds the stream could join |
| **A key per viewer**, so one viewer can be removed | The broadcaster would wrap a content key for every viewer and change it on every removal. A private channel keeps one secret; per-viewer access with epoch keys is the paid subscription extension ([`subscription.md`](subscription.md)) |
| **Hash first, then encrypt per neighbour** | Peers could not verify or pass on blocks without decrypting; every hop would encrypt again |
| **An authenticated cipher** | Adds 16 bytes and a second check of what the signed root already fixes |
| **A fixed Kad target** | Simpler. The same few nodes would then see every viewer for as long as the channel exists |
| **A password typed by the user** in place of a random secret | Can be guessed off line by anyone who recorded a segment |

---

## 11. Under consideration: listing channels that are on air

Not part of this draft. A `LIVE` record shows that a channel is on air, but only to someone who
knows the channel. Today a search with `Query.ext` = `ed3klive` finds channel files, and
whether each one runs is a lookup of its own. The name ending is the extension of the name
([`Protocol.md`](../Protocol.md) §10.1), not a keyword, so channel files do not share a keyword target.

Four ways to list running channels are considered for the next version of this extension. They
can be combined.

Options 1, 3 and 4 rest on an **on-air statement**: the channel key signs `stream`, `start` and
a time until which it holds, at most 15 min ahead, renewed every 5 min. It is about 110 bytes.
Without it anyone could mark any channel as on air. The statement exists already: it is the
on-air notice of [`pubsub.md`](pubsub.md) §4.2, which is pushed to the nodes that follow a
channel. What is open here is storing and searching it.

| | Option | Pro | Con |
|---|---|---|---|
| 1 | **Statement in keyword records.** The keyword records of the channel file carry the statement while the channel runs. `Query` gets a flag for "on air only". A storing node drops the statement when it runs out and keeps the record | uses the keyword search that exists; the claim is checked by the storing node and by the searcher; no new hot spot | finds channels by a word, not all channels; a field in the signed record and a filter that every `ROUTER` must apply |
| 2 | **One well-known target**, or one per language. Every running channel stores a record there | a full list in one lookup | the few nodes nearest the target carry every channel and are the place to attack or flood; the limit per target ([`Protocol.md`](../Protocol.md) §10.8) caps the list |
| 3 | **Index nodes** ([`Protocol.md`](../Protocol.md) §11). An index node holds records for every target and can answer "all channels on air", sorted and in pages | complete and fast; fits what a server is | as complete as that index node; optional and central |
| 4 | **Passing statements on.** Neighbours send each other statements of other channels they know, in `LiveState` | no Kad load | reaches only nodes that already watch a channel; never complete |

Current preference: 1 together with 3. Option 1 gives every node a search for running channels
about a subject, and the statement keeps an index node from inventing channels. Option 2
recreates a central point inside Kad without the capacity of a server.

---

## 12. Open points

- Every number is *(initial)* and none is measured. The ranks in §6.2 most of all.
- There is no list of channels that are on air (§11).
- Several qualities of one channel (bit rates) are not described. They can be several channels
  today.
- A successor for a lost or stolen channel key, as for a set head
  ([`file-set.md`](file-set.md) §10).
- Whether a viewer that cannot upload at all (no free upstream) should be able to say so.
- A private channel cannot take a viewer's access away (§10.5). A paid channel can, at the end of a
  viewer's pass ([`subscription.md`](subscription.md) §5).
