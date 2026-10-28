# Extension: publish and subscribe (`pubsub`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3, §13, [`file-set.md`](file-set.md) §5; [`live.md`](live.md). § = this document unless a document is named.
**Capability:** bit 12, `PUBSUB`. Message types 22 and 23. Record kind 7.
**Required of every `ROUTER`:** storing and returning the record kind `SUB` (§5). The rest is optional.

Changing sets are followed by polling the head record every few hours
([`file-set.md`](file-set.md) §5.3). This adds push: nodes following one thing find each other,
keep a few connections, pass a new signed record on once one has it → followers get a new
version in seconds.

- **Nothing new is signed for a set.** Pushed item = the `HEAD` record the author stores in Kad
  anyway.
- **Polling stays;** push is an optimisation on top. A subscriber the push misses learns
  of it at its next poll, so the mesh may be thin and incomplete.
- **A node forwards only what it has verified.** Every subscriber knows the topic key → every hop
  checks the signature.

Idea: BitTorrent's BEP 50. Not its shape: BEP 50 builds a per-topic routing table; ed3k uses its
live extension's neighbour mesh. Reasons:
[`docs/bittorrent-comparison.md`](../docs/bittorrent-comparison.md) §2.1.

---

## 1. Topics

Topic = type, key, `id` of the followed thing.

```
topic = H("ed3k v1 topic", type(1) ‖ key ‖ id)[0..16)
```

`topic` = Kad target where subscribers meet (§5); names the topic in `TopicJoin`.

```protobuf
enum TopicType {
  TOPIC_NONE = 0;
  SET_HEAD = 1;               // key and id of a set that changes (file-set.md §5.1)
  ON_AIR = 2;                 // key and id of a live channel (live.md §1.1)
  // 3, PAID, belongs to the optional paid subscription extension (subscription.md §8.4)
}
```

**Item** = one signed record of a topic, with an **order number**; higher = *newer* (same
topic). Two set heads with equal `seq`: lower `root` newer ([`file-set.md`](file-set.md) §5.2).
Set head order number = pair (`number` of its grant, 0 without `joint`; `Head.seq`), grant number
compared first ([`multi-author-set.md`](multi-author-set.md) §1.3). A node holds at most one item
per topic: newest verified.

| Type | Item | Order number | Signed by |
|---|---|---|---|
| `SET_HEAD` | set's `HEAD` record ([`file-set.md`](file-set.md) §5.2), sole-author or joint | (`Grant.number`, `Head.seq`); grant number 0 without `joint` | author's key; joint head: one key of its grant (§4.1) |
| `ON_AIR` | channel's on-air notice (§4.2) | `OnAir.issued` | channel key |
| `PAID` | `HIDDEN` record of a paid hidden set; none for other paid scopes ([`subscription.md`](subscription.md) §8.4) | `Sealed.seq` | set key |

- No topic for free data. A type is defined by a specification, with a forwarding node's checks.
- Nodes join only topics of known types; others can't be verified.
- `PAID` topic: named by its scope, not the hash above; its `SUB` records move to a new target
  daily. Only pass holders join ([`subscription.md`](subscription.md) §6.3, §8.4). Opened item
  followed per [`subscription.md`](subscription.md) §8.4.

---

## 2. Between two subscribers

Subscribers exchanging a topic = **neighbours**. One opened the **topic exchange**; then both
equal.

```protobuf
message TopicJoin {
  bytes     topic = 1;        // 16 B
  TopicType type = 2;
  uint64    held = 3;         // order number of the item I hold (SET_HEAD: Head.seq, ON_AIR: issued, PAID: Sealed.seq), or the highest remembered (§4.1, §4.2) when it ran out; 0 = none
  uint32    want_peers = 4;   // max number of peers wanted, 0 = none
  bytes     relays4 = 5;      // n × 22 B compact nodes: my relays, if I am a leaf
  bytes     relays6 = 6;      // n × 34 B
  uint64    held_grant = 7;   // Grant.number of the joint head I hold, 0 = none or a sole-author head
  // fields 8 to 10 belong to the optional paid subscription extension (subscription.md §6.3); 8 and 9 are reserved
}

message TopicState {
  TopicItem item = 1;
  repeated Peer peers = 2;    // other subscribers of this topic
  TopicRefusal refusal = 3;   // set: I do not take you as a neighbour
  // fields 4 to 8 belong to the optional paid subscription extension (subscription.md §6.3)
}

message TopicItem {
  Signed head = 1;            // SET_HEAD: `key` absent
  Signed on_air = 2;          // ON_AIR: `key` absent
  // field 3 belongs to the optional paid subscription extension (subscription.md §8.4)
}

enum TopicRefusal { TOPIC_OK = 0; TOPIC_FULL = 1; TOPIC_LEFT = 2; }
// 3, TOPIC_PASS, belongs to the optional paid subscription extension (subscription.md §6.3)
```

`Signed`, `Peer`: types of [`Protocol.md`](../Protocol.md) §10.7 and §5.4. Key never sent: both
sides know it, else no `topic`.

### 2.1 The topic exchange

- Subscriber opens exchange, sends `TopicJoin`; peer answers `TopicState`: its item if order
  number ≥ `held` (so an equal-numbered item can win by its type's tie-break, where one exists,
  §4), and peers if asked. Of two `ON_AIR` notices with equal `issued`, node keeps the first
  verified.
- `SET_HEAD`: subscriber sends both parts, `held_grant` and `held`; peer sends its item when its
  (grant number, `seq`) is not below (`held_grant`, `held`). `held_grant` = 0 for `ON_AIR` and
  `PAID`.
- Item past `expires`: never given out, now or later.
- Peer not taking the subscriber: answers with `refusal` set, closes. With `TOPIC_FULL` it SHOULD
  still fill `peers` and send its item, except to a node it refuses for 24 h (§2.2). Peer not
  subscribed, or not knowing the type: `TOPIC_LEFT`.
- After the first answer **both sides send `TopicState`** on verifying a newer item, and at once
  to a new neighbour whose answer showed no or an older item; to every neighbour not having shown
  this item or newer. Nothing else on the exchange, except a `PAID` topic's passes, keys, offers
  ([`subscription.md`](subscription.md) §6.3).
- Exchange stays open, connection kept alive as for a file stream
  ([`Protocol.md`](../Protocol.md) §4.1). Exchange or connection closes → neighbour gone. Either
  side leaves by closing the exchange.
- Several shared topics: one connection, one exchange per topic.
- At most one exchange per topic per node pair. Two open → both keep the one opened by the lower
  node ID (bytewise); both by the same node → the newer. The other's opener closes it; the other
  node doesn't answer on it and resets it if still open 10 s *(initial)* later. No refusal; they
  count as one neighbour.
- `peers` MUST be peers the sender itself had as neighbours of this topic within the last hour
  *(initial)*.

### 2.2 Receiving an item

Item checked before anything else (§4). Then:

| Item | The node |
|---|---|
| valid and newer than the one held | keeps, passes on (§2.1), acts on it (§4) |
| valid, not newer | drops it. Normal: items arrive via several neighbours |
| fails its signature or content check | drops it, closes the exchange, refuses that node as neighbour of any topic for 24 h *(initial)* |
| fails a time check (`expires`, `issued`) | drops it. Clocks differ: no fault of the neighbour |

- `TopicJoin` from a node refused for 24 h → `TOPIC_FULL` only: no `peers`, no item. In a `PAID`
  topic `TOPIC_LEFT` ([`subscription.md`](subscription.md) §6.3).
- **Size.** Item's `Signed` ≤ 1 KiB ([`Protocol.md`](../Protocol.md) §10.7); larger fails the
  content check.
- **At most one item per topic passed on per 10 s** *(initial)*. Earlier arrivals wait; only the
  newest waiting one is sent. So an author signing a thousand versions a minute costs each
  subscriber six messages per neighbour.
- The 10 s rule limits passing on only: `TopicJoin` answer and the item sent at once to a new neighbour (§2.1)
  carry the newest held item, waiting one included; not counted in the 10 s.
- Newer item learned otherwise (polling Kad, or as author) is passed on likewise: so items enter
  the mesh, and two mesh parts that lost each other reunite.

### 2.3 Neighbours

- Aim 3 neighbours per topic, keep 2 to 4 *(initial)*; below 2, look for more (§5).
- Bucket diversity limits apply: at most 1 per host, 2 per site, 4 per block
  ([`ip-diversity.md`](ip-diversity.md) §1 and §2).
- **A node subscribes to at most 16 topics** *(initial)* this way; others it follows are only
  polled. Which get push: local choice; last-changed ones a good default.

---

## 3. Publishing

No publisher role on the wire. Set author or channel broadcaster:

1. stores the record in Kad as before, where that applies (§4.1);
2. subscribes to its own topic like any node;
3. sends the new item to its neighbours.

Author unable to stay online hands the item to any subscriber, or only stores it in Kad; first
polling subscriber brings it in.

---

## 4. Item types

### 4.1 `SET_HEAD`

Item = `Signed` record of kind `HEAD`, byte for byte as stored in Kad. A joint head
([`multi-author-set.md`](multi-author-set.md) §1.3) is an item too: checked as that document's
searcher does, ordered as there (`Grant.number` before `Head.seq`).

| Check | |
|---|---|
| Signature | without `joint`: valid under the topic key. Joint head: under one key of `joint.grant.keys`; verifier tries each, at most 8. Both computed per [`Protocol.md`](../Protocol.md) §10.7, kind `HEAD`, head target of the topic's key and `id` ([`file-set.md`](file-set.md) §5.1) |
| `head.id` | topic's `id`; joint head: `Head.id` and `Grant.id` |
| Joint head | valid grant ([`multi-author-set.md`](multi-author-set.md) §1.1), `grant.key` MUST be the topic key; `cosigs` pass §1.3 there. Failure = content failure (§2.2) |
| `expires` | later than now minus 1 h |
| Content | `head`, or `joint` in a joint head, nothing else |
| Size | `id`, `root`, `name` per [`file-set.md`](file-set.md) §5.2; joint head: `name` empty, `id` at most 16 B |

Same as a searcher's checks in [`file-set.md`](file-set.md) §5.2. Accepting node continues as §5.3
there from step 2: downloads the manifest, accepts it only with same key, same `id`, the head's
`seq` and, for a joint head, a grant with the head's `number` ([`multi-author-set.md`](multi-author-set.md) §1.2, §1.3). Manifest failing it doesn't
change the item: verified head stays held and passed on; node keeps its old set version.

- Head with same grant number, `seq`, `root` and later `expires`: not newer, not passed on; node
  **replaces the bytes it holds** with it, so it stores in Kad and gives joiners the
  longest-living copy.
- Head **run out** when `expires` passed (`expires` < now): not given out (§2.1) or stored in Kad;
  its order number kept, so an older head is still not newer.
- Subscriber stores its held head in Kad as a follower does, once per 24 h on its own schedule;
  a new head from push doesn't move that schedule. Signer of a new head stores it on publishing (§3),
  restarting its 24 h ([`file-set.md`](file-set.md) §5.2); a head signed only to extend `expires`
  (same `seq`, `root`) counts as new here: restarts the 24 h. A head taken from push or polling,
  also one under its own key, follows the 24 h schedule. `Store.signer` set as file-set.md §5.2 says, for a joint head too.

### 4.2 `ON_AIR`

A follower wants to know when a broadcast starts without looking up every followed channel's
`LIVE` records every few minutes. Broadcaster signs a notice:

```protobuf
// in Record (Protocol.md §10.7):
OnAir on_air = 6;

message OnAir {
  bytes  id = 1;              // channel id
  uint64 start = 2;           // the broadcast (live.md §1.1)
  uint64 issued = 3;          // Unix time in seconds when this notice was signed
  bool   end = 4;             // the broadcast is over
}
```

Item = `Signed` with body `Record` holding `expires` and `on_air`, signed with the channel key
over the bytes of [`Protocol.md`](../Protocol.md) §10.7, kind `LIVE`, target = channel's
`stream`. `LIVE` is only the signature domain: no signed record of it is ever stored, so the
notice can't be replayed as a Kad record.

| Check | |
|---|---|
| Signature | under the topic key, kind `LIVE`, target `stream` |
| `on_air.id` | topic's `id` |
| `issued` | at most 1 h ahead of now |
| `expires` | later than now minus 1 h, and at most 15 min *(initial)* after `issued` |
| Content | `on_air` and nothing else |

The table's three time conditions (`issued`, both `expires` ones) are time checks per §2.2:
failing notice dropped, no neighbour fault.

- Broadcaster sends a notice at start, every 5 min *(initial)* while running, one with `end` when
  over. Each has strictly later `issued` than the previous, so none tie.
- Notice says a broadcast runs; peers still found via `LIVE` ([`live.md`](live.md) §5); no proof
  any peer is reachable.
- Notice **run out** when `expires` passed (`expires` < now): no longer held, not given to
  joiners. Check above still admits one run out under 1 h ago (clock skew); such a notice is not
  passed on.
- **Highest `issued` remembered** per topic (verified) until 1 h *(initial)* after that notice's
  `expires`, also when no longer held; a notice with `issued` not later is dropped as not newer
  (§2.2). So an old notice can't reappear after its successor ran out.
- **In this version the notice is not stored in Kad.** Signed-record form so channel listing
  options ([`live.md`](live.md) §11) can reuse the bytes later.
- No topic for a private channel ([`live.md`](live.md) §10), nor for a hidden set
  ([`hidden-set.md`](hidden-set.md)) other than a paid one ([`subscription.md`](subscription.md)
  §8.4).

---

## 5. Finding subscribers

Subscriber publishes a `SUB` (7) record under target `topic`.

- Same as a source record but for lifetime: sent in `Store.source`, bound to the connection, unsigned,
  returned in `KadAnswer.sources` ([`Protocol.md`](../Protocol.md) §10.3).
- **Lifetime 60 min** *(initial)*; re-stored every 30 min while subscribed.
- Per-target limits as `SOURCE`: ≤ 8 *(initial)* per host, 32 per site
  ([`ip-diversity.md`](ip-diversity.md) §2.4), ≤ 10,000 *(initial)* records per target; a further
  node's record → `FULL`, a held one renewed ([`Protocol.md`](../Protocol.md)
  §10.8).
- Index node doesn't keep it past lifetime ([`Protocol.md`](../Protocol.md) §11): a node that
  left drops out of the list as on any other node.
- Every `ROUTER` of wire version 1 MUST store and return this kind, whether or not it sets
  `PUBSUB`; needs no topic knowledge. Rooms store it for members ([`room.md`](room.md) §5).
- **Node under its anonymous key** ([`hops.md`](hops.md) §1) can't store `SUB`: a session has no
  observed address ([`hops.md`](hops.md) §3.3). Instead stores an `ANON` record under `topic`
  via a `HopLookup` ([`hops.md`](hops.md) §4), `expires` at most 60 min
  *(initial)* ahead, re-stored every 30 min. Subscriber setting `ANON` also searches `ANON`
  under `topic`, reaching those nodes via their gates ([`hops.md`](hops.md) §4.3). Node without
  a gate stores none: joins others, isn't found.

**Joining.** Subscriber looks up `SUB` under `topic`, sends `TopicJoin` to peers from the answer and to
`TopicState.peers` until it has its neighbours, publishes its own record. Failed dials count under the limits of [`ip-diversity.md`](ip-diversity.md) §2.5.
Finding nobody: publish record, wait to be found.

---

## 6. What an attacker can do

| Attack | Result |
|---|---|
| **Invent an item** | Impossible without the key. Every hop verifies |
| **Show an old item again** | Not newer than the one held; dropped |
| **Surround a subscriber** with nodes that pass nothing on | Push fails; poll arrives, so delay = a node without this extension's. Diversity limits (§2.3) raise the cost |
| **Fill the target** with records of nodes that are not subscribed | Wasted dials, limited as false sources ([`ip-diversity.md`](ip-diversity.md) §2.5) |
| **Send bad items** | One per attacker node, then refused for 24 h (§2.2) |
| **An author that signs without pause** | One message per neighbour and 10 s (§2.2) |

**What a subscriber shows.** Its `SUB` record tells anyone computing the topic that it follows
it, as `SOURCE` shows who shares a file; polling shows this only to storing nodes. Users not
wanting to be listed follow by polling.

---

## 7. Cost

| | Size |
|---|---|
| Item | head ~150 B or notice ~110 B, once per neighbour |
| Connections | ~3 per topic, held open; ~48 and at most 64 for 16 topics |
| Keeping them alive | one ~70 B packet each way per connection per 20 s |
| `SUB` record | one store per 30 min, on ~8 nodes |

16 topics ≈ 300 B/s on idle connections: price of push, reason for the limit of 16.

---

## 8. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A Kademlia overlay per topic** (BEP 50 as written) | per-node fan-out bounded by routing table; matches the BEP | routing table and upkeep per topic; one contact per bucket → one leaving node cuts a path; costly over connections |
| **Storing nodes notify** the nodes that registered with them | no mesh; one message type | all subscribers of a popular topic on the same 8 nodes; NATed subscriber must hold that connection; storing nodes change |
| **Polling only, made cheaper** | nothing new | delay minutes at best; storing-node load grows with followers |
| **Any signed value as an item** (BEP 44) | new uses without a new specification | nodes carry and pass on bytes they can't judge |
| **Using the live exchange** for all of it | one mesh design | `LiveState` carries a window and segment records; a topic has neither |
| **The sources of the current manifest as the mesh** | no record kind | sources hold no lasting mutual connections; a follower need not share the manifest |

---

## 9. Open points

- Every number is *(initial)*, none measured.
- May a subscriber with neighbours poll less than [`file-set.md`](file-set.md) §5.3 asks?
- A very large topic puts as many `SUB` records on its storing nodes as a popular file `SOURCE`
  records. Should subscribers with enough peers leave their own record out?
- Topics for other signed records: notes of one author, files of one publisher.
- Should the live extension send its segment records over this exchange?
