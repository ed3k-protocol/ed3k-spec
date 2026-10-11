# Extension: live streams (`live`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3, §12 and §13. Section numbers (§) refer to that document.
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
- NAT traversal (§9), compression (§12.3) and the bad-block rule (§12.3) apply unchanged.

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

- The channel key SHOULD be a key of its own, not the node's identity key (§6.1). The channel
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
  uint32 window = 9;          // segments a viewer keeps; absent = 30
  uint64 created = 10;
}
```

- The signature is over `"ed3k v1 live channel" ‖ body` and is REQUIRED. A reader checks it
  against `key` and refuses the file when it fails.
- A channel file MUST NOT exceed 64 KiB.
- It is shared, published under its keywords and downloaded like any file (§10, §12). A search
  finds channels as it finds files; the suggested name ends in `.ed3klive`.
- It describes the channel, not a broadcast. It changes only when the description does.
  A reader that holds two files for one `stream` uses the one with the later `created`.

### 1.3 Links

```
ed3k:live?key=<channel key, base32 lower case, no padding>&id=<id, base32>[&dn=<url-encoded title>]
```

The link is enough to watch: it gives `stream` and the key that segment records are checked
against. `media` and `codecs` then come from the first peer (§3.1). A client without this
extension does not know the link.

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
}
```

The signature is over

```
"ed3k v1 live" ‖ stream(16) ‖ body
```

and is made with the channel key.

- A record is about 150 bytes. A `LiveRecord` MUST NOT exceed 1 KiB.
- A node accepts a record only when the signature is valid, and drops it otherwise. A neighbour
  that sent it counts one miss (§6).
- **Order.** Records are ordered by `start`, then `seq`. A node ignores a record of an earlier
  `start` than the newest it has seen, and a `start` more than 1 h *(initial)* ahead of its
  clock. An old broadcast can therefore not be played back into a running one.
- **Two records for one `seq`** of one broadcast with different content are proof that the
  channel key signed both. A node keeps the first and ignores the other.
- `end` closes the broadcast. Nodes forward it, keep it for 30 min *(initial)*, and leave the
  stream. There is no other way to end a broadcast than this record or silence (§3.4).
- A record carries no wall-clock time of the segment. `seq` gives the order, and a time stamp
  would only expose the broadcaster's clock.

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
}

message LiveState {
  uint64 start = 1;           // broadcast this state is about
  uint64 first = 2;           // seq of bit 0 of `have`
  bytes  have = 3;            // segments I hold completely, LSB first
  repeated LiveRecord records = 4;
  repeated Peer peers = 5;    // other peers of this stream
  string media = 6;           // answer to want_channel
  string codecs = 7;
  Refusal refusal = 8;        // set: I do not take you as a neighbour
}

enum Refusal { NONE = 0; FULL = 1; LEFT = 2; BLOCKED = 3; }

message LiveGet {
  bytes  stream = 1;
  bytes  root = 2;            // a segment or init data
}
```

### 3.1 The live exchange

- A viewer opens an exchange and sends `LiveJoin`. The peer answers with `LiveState`: its
  window, the records of every segment in it, and peers if asked.
- A peer that does not take the viewer answers with `refusal` set and closes. With `FULL` it
  SHOULD still fill `peers`, so the viewer has somewhere else to go. A peer that is not in the
  stream answers `LEFT`.
- After the first answer **both sides send `LiveState`** on the exchange whenever one of these
  happens:
  - It learns a new valid record: it sends the record at once to every neighbour that has not
    shown it. This is how records travel, and it is the only thing that is pushed.
  - It completes a segment: `first` and `have` are sent, at most once per second *(initial)*.
  - 10 s *(initial)* have passed without one: an empty `LiveState` as a sign of life.
- A neighbour that sent nothing for 30 s *(initial)* is gone.
- Either side leaves by closing the exchange.
- `peers` MUST be peers the sender has itself had as verified neighbours of this stream within
  the last 10 min *(initial)*.

### 3.2 Fetching a segment

Data is never pushed. A node asks a neighbour whose `have` shows the segment.

- The node opens a **new exchange per segment** and sends `LiveGet`. It then uses
  `BlockRequest`, `Block`, `HashRequest` and `HashAnswer` exactly as §12.2 to §12.4, with
  offsets inside the segment. There is no queue and no `rank`: a neighbour serves at once or
  answers `Error{BUSY}`.
- `LiveGet` is accepted only from a neighbour, that is while a live exchange with that node is
  open. Anything else is answered `Error{DENIED}`.
- A segment of more than one block may be fetched from several neighbours at once, by block.
- A node that no longer wants a segment resets the exchange. One lost packet delays only the
  segment it belongs to, because each segment has an exchange of its own.
- `Error{NOT_FOUND}` for a segment that the neighbour's last `have` showed counts as a miss
  (§6), unless the segment has left its window since.
- A node gives up on a request after 4 s *(initial)* and asks another neighbour.

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
  many as its upload allows.
- Neighbours are subject to the diversity limits of a bucket: at most 2 per site and 4 per
  block ([`ip-diversity.md`](ip-diversity.md) §1 and §2).
- When no new record has arrived for 60 s *(initial)*, the broadcast counts as interrupted. A
  viewer keeps its neighbours and looks for others (§5). After 10 min *(initial)* without a
  record it leaves.

---

## 4. Broadcasting

- The broadcaster is the node that holds the channel key, or receives signed records from
  whoever does. Apart from that it is a peer like any other: it answers `LiveJoin` and
  `LiveGet` and publishes itself (§5).
- It SHOULD hand the first neighbours a full share of its upload, so that each segment leaves
  it several times before it serves late joiners.
- A segment SHOULD be 1 to 4 s long. Shorter segments lower the delay and raise the number of
  records and exchanges.
- A broadcast ends with an `end` record (§2.2). A broadcaster that was cut off and returns
  within 10 min MAY continue with the next `seq` of the same `start`, and sets `gap`.

---

## 5. Finding peers

### 5.1 The `LIVE` record

A node that is in a stream, the broadcaster included, publishes a record of kind `LIVE` (6)
under the target `stream`.

- It is a source record in everything but its lifetime. It is sent in `Store.source`, bound to
  the connection, not signed, and returned in `KadAnswer.sources` (§10.3).
- **Lifetime 10 min** *(initial)*; stored again every 5 min while the node is in the stream. A
  node that leaves lets it run out.
- The limits per target are those of `SOURCE` records
  ([`ip-diversity.md`](ip-diversity.md) §2.4).
- Every `ROUTER` of wire version 1 MUST store and return this kind, whether or not it sets
  `LIVE`. It needs to know nothing about streams for that.

A stream is on air when its target holds records, and off when it holds none. The short
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
| The whole segment, or every block asked for, arrived within 4 s | 1 hit |
| `Error{BUSY}`, no answer in time, a bad block, an invalid record, or `NOT_FOUND` for a segment shown in `have` | 1 miss |
| The same for an **urgent** request | 2 hits or 2 misses |
| A request the node reset itself | nothing |

```
rate = hits / (hits + misses)
```

- A request is **urgent** when its segment is within 3 segments *(initial)* of what the node is
  playing. Only the asking node knows that, and only its own count uses it. A neighbour that
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
  its time as a neighbour starts again.
- **Out.** A rate under 20 % with at least 10 outcomes ends it. The node closes the live
  exchange and refuses that node ID with `BLOCKED` for 30 min, for this stream only.
- A request above the number its rank allows is answered `Error{BUSY}`. That answer does count
  as a miss for the node that sent too many, which is its own doing.
- **Core limits.** At most 1 **core** neighbour per site and 2 per block. A further one stays
  **steady**. One network can then not hold a node's whole inner circle.
- **Sudden loss.** A node that loses more than a third of its **core** neighbours within 2 min
  neither lowers a rank nor ends a neighbour for the next 2 min. A network fault looks like
  many peers failing at once, and punishing through it would take the mesh apart.
- **A returning neighbour** within 30 min starts at the rank it had, less one. Ranks are kept
  per node ID, which a peer cannot change with its address (§6.1).
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
| Firewalled broadcaster | hole punching and a relay of its own | §9, unchanged |
| Privacy | onion paths with fixed-size cells, cover traffic, striping with Reed-Solomon codes | none beyond the transport |

**Taken from it:** rating by response rate, not by volume; the probe; double weight near the
play position; starting 3 segments behind the newest; a refusal that names other peers; the
signed end; no clock time in a segment.

**Not taken:** its own relay and hole punching, the onion layer, the fixed 16-segment window,
pushing data, and discovery through tagged keyword entries.

---

## 10. Open points

- Every number is *(initial)* and none is measured. The ranks in §6.2 most of all.
- There is no list of channels that are on air. A search finds channel files; whether a channel
  runs is one lookup each.
- Several qualities of one channel (bit rates) are not described. They can be several channels
  today.
- A successor for a lost or stolen channel key, as for a set head
  ([`file-set.md`](file-set.md) §10).
- Whether a viewer that cannot upload at all (no free upstream) should be able to say so.
- Private channels: segments encrypted to a key from the link.
