# Extension: publish and subscribe (`pubsub`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3 and §13, [`file-set.md`](file-set.md) §5 and [`live.md`](live.md). Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 12, `PUBSUB`. Message types 22 and 23. Record kind 7.
**Required of every `ROUTER`:** storing and returning the record kind `SUB` (§5). The rest is optional.

A set that changes is followed by polling: a follower searches the head record every few hours
([`file-set.md`](file-set.md) §5.3). This extension adds the push. The nodes that follow the
same thing find each other, keep a few connections among themselves, and pass a new signed
record on as soon as one of them has it. A new version then reaches its followers in seconds.

- **Nothing new is signed for a set.** What is pushed is the `HEAD` record that the author
  stores in Kad anyway.
- **Polling stays.** The push is an optimisation on top of it. A subscriber that the push does
  not reach learns the new version at its next poll, so the mesh may be thin and need not be
  complete.
- **A node forwards only what it has verified.** Every subscriber knows the key of the topic,
  so every hop checks the signature.

The idea is BitTorrent's BEP 50. The shape is not: BEP 50 builds a routing table per topic,
ed3k uses the neighbour mesh of its live extension. The reasons are in
[`docs/bittorrent-comparison.md`](../docs/bittorrent-comparison.md) §2.1.

---

## 1. Topics

A topic has a type, a key and an `id`. They are those of the thing that is followed.

```
topic = H("ed3k v1 topic", type(1) ‖ key ‖ id)[0..16)
```

`topic` is the Kad target under which subscribers find each other (§5) and names the topic in
`TopicJoin`.

```protobuf
enum TopicType {
  TOPIC_NONE = 0;
  SET_HEAD = 1;               // key and id of a set that changes (file-set.md §5.1)
  ON_AIR = 2;                 // key and id of a live channel (live.md §1.1)
  // 3, PAID, belongs to the optional paid subscription extension (subscription.md §8.4)
}
```

An **item** is one signed record of a topic. Every item has an **order number**, and an item is
*newer* than another of the same topic when its order number is higher. Of two set heads with
the same `seq` the one with the lower `root` is the newer ([`file-set.md`](file-set.md) §5.2).
The order number of a set head is a pair: the `number` of its grant, which is 0 for a head
without `joint`, and `Head.seq`. Pairs are compared by the grant number first
([`multi-author-set.md`](multi-author-set.md) §1.3).
A node holds at most one item per topic: the newest it has verified.

| Type | Item | Order number | Signed by |
|---|---|---|---|
| `SET_HEAD` | the `HEAD` record of the set ([`file-set.md`](file-set.md) §5.2), sole-author or joint | (`Grant.number`, `Head.seq`); the grant number is 0 without `joint` | the author's key; a joint head by one key of its grant (§4.1) |
| `ON_AIR` | an on-air notice of the channel (§4.2) | `OnAir.issued` | the channel key |
| `PAID` | the `HIDDEN` record of a paid hidden set; none for other paid scopes ([`subscription.md`](subscription.md) §8.4) | `Sealed.seq` | the set key |

- There is no topic for free data. A type is defined by a specification, with the checks that a
  forwarding node makes.
- A node takes part only in topics of a type it knows, because it could not verify the others.
- A `PAID` topic is named by its scope, not by the hash above, and its `SUB` records move to
  a new target every day. Only pass holders join it ([`subscription.md`](subscription.md) §6.3, §8.4). An item that
  opened is followed as [`subscription.md`](subscription.md) §8.4 says.

---

## 2. Between two subscribers

Two subscribers that exchange a topic are **neighbours**. One of them opened the **topic
exchange**; after that both sides are equal.

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

`Signed` and `Peer` are the types of [`Protocol.md`](../Protocol.md) §10.7 and §5.4. The key is
never sent: both sides know it, or they could not have computed `topic`.

### 2.1 The topic exchange

- A subscriber opens an exchange and sends `TopicJoin`. The peer answers with `TopicState`: its
  item if its order number is at least `held`, so that an item of the same number can win by
  the tie-break of its type, where it has one (§4), and peers if asked. Of two `ON_AIR`
  notices with the same `issued` a node keeps the first it verified.
- For `SET_HEAD` the subscriber sends both parts of the order number: `held_grant` and `held`.
  The peer sends its item when the pair (grant number, `seq`) of its item is not below
  (`held_grant`, `held`). `held_grant` is 0 for `ON_AIR` and `PAID`.
- An item whose `expires` has passed is not given out, neither in this answer nor later.
- A peer that does not take the subscriber answers with `refusal` set and closes. With
  `TOPIC_FULL` it SHOULD still fill `peers` and send its item, except to a node it refuses
  for 24 h (§2.2). A peer that is not subscribed
  to the topic, or does not know the type, answers `TOPIC_LEFT`.
- After the first answer **both sides send `TopicState`** when they have verified a newer item,
  and at once to a new neighbour whose answer showed no item or an older one.
  They send it to every neighbour that has not shown this item or a newer one. Nothing else is
  sent on the exchange, apart from the passes, keys and offers of a `PAID` topic
  ([`subscription.md`](subscription.md) §6.3).
- The exchange stays open, and the connection is kept alive as for a file stream
  ([`Protocol.md`](../Protocol.md) §4.1). A neighbour is gone when the exchange or the
  connection closes. Either side leaves by closing the exchange.
- Several topics shared by two nodes use one connection, with one exchange per topic.
- Two nodes keep at most one topic exchange per topic. When two are open, both keep the one
  opened by the node with the lower node ID, compared as bytes; of two opened by the same node,
  the newer. The node that opened the other closes it. The other node does not answer on it,
  and resets it when it is still open 10 s *(initial)* later. This is no refusal, and the two
  count as one neighbour.
- `peers` MUST be peers the sender has itself had as neighbours of this topic within the last
  hour *(initial)*.

### 2.2 Receiving an item

A node checks an item before it does anything else with it (§4). Then:

| Item | The node |
|---|---|
| valid and newer than the one held | keeps it, passes it on (§2.1) and acts on it (§4) |
| valid, not newer | drops it. This is normal: an item arrives over several neighbours |
| fails its signature or content check | drops it, closes the exchange and does not take that node as a neighbour of any topic for 24 h *(initial)* |
| fails a time check (`expires`, `issued`) | drops it. Clocks differ, so this is no fault of the neighbour |

- A node answers a `TopicJoin` from a node it refuses for 24 h with `TOPIC_FULL` and nothing
  else: no `peers` and no item. In a `PAID` topic it answers `TOPIC_LEFT`
  ([`subscription.md`](subscription.md) §6.3).
- **Size.** The `Signed` of an item is at most 1 KiB ([`Protocol.md`](../Protocol.md) §10.7).
  A larger one fails the content check.
- **At most one item per topic is passed on per 10 s** *(initial)*. An item that arrives
  earlier waits, and only the newest one that waited is sent. An author who signs a thousand
  versions in a minute then costs each subscriber six messages per neighbour.
- The rule limits passing on only. The answer to a `TopicJoin` and the item sent at once to a
  new neighbour (§2.1) carry the newest item the node holds, one that waits included, and do
  not count in the 10 s.
- A subscriber that learns a newer item in another way, by polling Kad or because it is the
  author, passes it on in the same way. This is how an item enters the mesh, and how two parts
  of a mesh that lost each other meet again.

### 2.3 Neighbours

- A node aims at 3 neighbours per topic and keeps between 2 and 4 *(initial)*. When it falls
  below 2 it looks for more (§5).
- Neighbours are subject to the diversity limits of a bucket: at most 1 per host, 2 per site
  and 4 per block ([`ip-diversity.md`](ip-diversity.md) §1 and §2).
- **A node subscribes to at most 16 topics** *(initial)* in this way. Further things it follows
  are polled only. Which ones get the push is local choice; the ones that changed last are a
  good default.

---

## 3. Publishing

There is no publisher role on the wire. The author of a set, or the broadcaster of a channel:

1. stores the record in Kad as before, where that applies (§4.1);
2. subscribes to its own topic as any node does;
3. sends the new item to its neighbours.

An author that cannot stay online hands the item to any subscriber, or only stores it in Kad:
the first subscriber that polls brings it into the mesh.

---

## 4. Item types

### 4.1 `SET_HEAD`

The item is the `Signed` record of kind `HEAD`, byte for byte as it is stored in Kad. A joint
head ([`multi-author-set.md`](multi-author-set.md) §1.3) is an item too: it is checked as that
document's searcher checks it, and ordered as there, by `Grant.number` before `Head.seq`.

| Check | |
|---|---|
| Signature | a head without `joint`: valid under the key of the topic. A joint head: valid under one key of `joint.grant.keys`; the verifier tries each, at most 8. Both computed as [`Protocol.md`](../Protocol.md) §10.7 with kind `HEAD` and the head target of the topic's key and `id` ([`file-set.md`](file-set.md) §5.1) |
| `head.id` | the `id` of the topic; in a joint head `Head.id` and `Grant.id` |
| Joint head | the grant is a valid grant ([`multi-author-set.md`](multi-author-set.md) §1.1) and `grant.key` MUST be the key of the topic; the `cosigs` pass the check of §1.3 there. A failure is a content failure (§2.2) |
| `expires` | later than now minus 1 h |
| Content | `head`, or `joint` in a joint head, and nothing else |
| Size | `id`, `root` and `name` as in [`file-set.md`](file-set.md) §5.2; in a joint head `name` empty and `id` at most 16 B |

These are the checks of a searcher in [`file-set.md`](file-set.md) §5.2. A node that accepts
the item goes on as in §5.3 there, from step 2: it downloads the manifest and accepts it only
with the same key, the same `id` and the `seq` of the head, and, for a joint head, a grant
with the same `number` as the head's ([`multi-author-set.md`](multi-author-set.md) §1.2,
§1.3). A manifest that fails that check does not change the item: the head was verified and
stays the one held and passed on; the node keeps its old version of the set.

- A head with the same grant number, the same `seq`, the same `root` and a later `expires` is
  not newer and is not passed on. The node **replaces the bytes it holds** with it, so that
  what it stores in Kad and gives to a node that joins is the copy that lives longest.
- A head has **run out** when its `expires` has passed (`expires` < now). It is not given out
  (§2.1) and not stored in Kad. The node keeps its order number, so that an older head is
  still not newer.
- A subscriber stores the head it holds in Kad as a follower does, once per 24 h on its own
  schedule; taking a new head from the push does not move that schedule. The node that signs
  a new head stores it when it publishes it (§3), and that store starts its 24 h anew
  ([`file-set.md`](file-set.md) §5.2). A head signed only to extend `expires` (the same `seq`
  and `root`) is a new head for this rule: it starts the 24 h anew. A head it takes from the push or from polling, also one
  under its own key, follows the 24 h schedule. `Store.signer` is set as file-set.md §5.2
  says, for a joint head too.

### 4.2 `ON_AIR`

A node that follows a channel wants to know when a broadcast starts, without looking up the
`LIVE` records of every channel it follows every few minutes. The broadcaster signs a notice:

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

The item is a `Signed` whose body is a `Record` with `expires` and `on_air`. It is signed with
the channel key over the bytes of [`Protocol.md`](../Protocol.md) §10.7, with kind `LIVE` and
the channel's `stream` as the target. `LIVE` serves as the domain of the signature only: no
signed record of that kind is ever stored, so the notice cannot be replayed as a Kad record.

| Check | |
|---|---|
| Signature | valid under the key of the topic, with kind `LIVE` and target `stream` |
| `on_air.id` | the `id` of the topic |
| `issued` | at most 1 h ahead of now |
| `expires` | later than now minus 1 h, and at most 15 min *(initial)* after `issued` |
| Content | `on_air` and nothing else |

The three time conditions of the table (`issued` at most 1 h ahead, `expires` later than now
minus 1 h, `expires` at most 15 min after `issued`) are time checks in the sense of §2.2: a notice that fails one is dropped, and it is no fault of the neighbour.

- The broadcaster sends a notice when a broadcast starts, every 5 min *(initial)* while it
  runs, and one with `end` when it is over. Each notice carries a strictly later `issued` than
  the one before, so that two never tie.
- A notice says that a broadcast runs. The peers are still found through the `LIVE` records
  ([`live.md`](live.md) §5), and a notice is no proof that any peer can be reached.
- A notice has **run out** when its `expires` has passed (`expires` < now). It is then no
  longer held, and not given to a node that joins. The check above still lets a notice in
  that ran out less than 1 h ago, for clocks that differ; such a notice is not passed on.
- **The highest `issued` is remembered.** A node remembers the highest `issued` it has
  verified for a topic until 1 h *(initial)* after the `expires` of that notice, also when it
  no longer holds the notice. It drops a notice whose `issued` is not later, as one that is
  not newer (§2.2). An old notice can therefore not be shown again after the one that
  followed it ran out.
- **In this version the notice is not stored in Kad.** It is built as a signed record so that
  the options for listing channels ([`live.md`](live.md) §11) can use the same bytes later.
- A private channel ([`live.md`](live.md) §10) has no topic, and neither has a hidden set
  ([`hidden-set.md`](hidden-set.md)) other than a paid one ([`subscription.md`](subscription.md) §8.4).

---

## 5. Finding subscribers

A subscriber publishes a record of kind `SUB` (7) under the target `topic`.

- It is a source record in everything but its lifetime. It is sent in `Store.source`, bound to
  the connection, not signed, and returned in `KadAnswer.sources`
  ([`Protocol.md`](../Protocol.md) §10.3).
- **Lifetime 60 min** *(initial)*; stored again every 30 min while the node is subscribed.
- The limits per target are those of `SOURCE` records: at most 8 *(initial)* per host and 32
  per site ([`ip-diversity.md`](ip-diversity.md) §2.4), and at most 10,000 *(initial)* records
  under one target; a record of a further node is `FULL`, and a held one is renewed
  ([`Protocol.md`](../Protocol.md) §10.8).
- An index node does not keep it past its lifetime ([`Protocol.md`](../Protocol.md) §11): a
  node that left the topic drops out of the list as on any other node.
- Every `ROUTER` of wire version 1 MUST store and return this kind, whether or not it sets
  `PUBSUB`. It needs to know nothing about topics for that. Rooms store the same kind for
  their members ([`room.md`](room.md) §5).
- **A node under its anonymous key** ([`hops.md`](hops.md) §1) cannot store a `SUB` record: a
  session has no observed address ([`hops.md`](hops.md) §3.3). It stores an `ANON` record
  under `topic` in its place, through a `HopLookup` ([`hops.md`](hops.md) §4), with `expires`
  at most 60 min *(initial)* ahead, and stores it again every 30 min. A subscriber that sets
  `ANON` also searches kind `ANON` under `topic` and reaches those nodes through their gates
  ([`hops.md`](hops.md) §4.3). A node without a gate stores none: it joins others and is not
  found by them.

**Joining.** The subscriber looks up `SUB` records under `topic`, sends `TopicJoin` to peers
from the answer and to peers named in `TopicState.peers` until it has its neighbours, and
publishes its own record. Failed dials count under the limits of
[`ip-diversity.md`](ip-diversity.md) §2.5. A subscriber that finds nobody publishes its record
and waits to be found.

---

## 6. What an attacker can do

| Attack | Result |
|---|---|
| **Invent an item** | Not possible without the key. Every hop verifies |
| **Show an old item again** | It is not newer than the one held and is dropped |
| **Surround a subscriber** with nodes that pass nothing on | The push does not arrive. The poll does, and so the delay is that of a node without this extension. The diversity limits (§2.3) raise the cost |
| **Fill the target** with records of nodes that are not subscribed | Wasted dials, limited as for false sources ([`ip-diversity.md`](ip-diversity.md) §2.5) |
| **Send bad items** | One per attacker node; that node is then refused for 24 h (§2.2) |
| **An author that signs without pause** | One message per neighbour and 10 s (§2.2) |

**What a subscriber shows.** Its `SUB` record tells anyone who computes the topic that this
node follows it, as a `SOURCE` record tells who shares a file. Polling shows the same only to
the storing nodes. A user who does not want to be listed follows by polling.

---

## 7. Cost

| | Size |
|---|---|
| Item | a head record of about 150 B or a notice of about 110 B, to each neighbour once |
| Connections | about 3 per topic, held open; about 48 and at most 64 for 16 topics |
| Keeping them alive | one packet of about 70 B each way per connection and 20 s |
| `SUB` record | one store per 30 min, on about 8 nodes |

A node with 16 topics spends on the order of 300 B/s on idle connections. That is the price of
the push, and the reason for the limit of 16.

---

## 8. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A Kademlia overlay per topic** (BEP 50 as written) | fan-out per node is bounded by the routing table; matches the BEP | a routing table and its upkeep per topic; with one contact per bucket a single node that leaves cuts a path; costly over connections |
| **Storing nodes notify** the nodes that registered with them | no mesh; one message type | every subscriber of a popular topic sits on the same 8 nodes; a subscriber behind a NAT must hold that connection; storing nodes change |
| **Polling only, made cheaper** | nothing new | the delay stays at minutes at best, and the load on the storing nodes grows with the followers |
| **Any signed value as an item** (BEP 44) | new uses without a new specification | nodes carry and pass on bytes they cannot judge |
| **Using the live exchange** for all of it | one mesh design | `LiveState` carries a window and segment records; a topic has neither |
| **The sources of the current manifest as the mesh** | no record kind | sources hold no lasting connections to each other, and a follower need not share the manifest |

---

## 9. Open points

- Every number is *(initial)* and none is measured.
- Whether a subscriber with neighbours may poll less often than
  [`file-set.md`](file-set.md) §5.3 asks.
- A very large topic puts as many `SUB` records on its storing nodes as a popular file puts
  `SOURCE` records. Whether subscribers that found enough peers should leave their own record
  out.
- Topics for other signed records: notes of one author, the files of one publisher.
- Whether the live extension should send its segment records over this exchange.
