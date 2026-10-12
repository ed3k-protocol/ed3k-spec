# The BitTorrent enhancement proposals compared with ed3k

**Status:** design note (2026-10-12). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §6.3, §9, §10, §12 and §13, and the extensions [`pubsub`](../extensions/pubsub.md), [`lan-discovery`](../extensions/lan-discovery.md), [`file-set`](../extensions/file-set.md), [`live`](../extensions/live.md), [`http-source`](../extensions/http-source.md) and [`ip-diversity`](../extensions/ip-diversity.md). Section numbers (§) refer to `Protocol.md` unless another document is named.

BitTorrent is specified in numbered proposals (BEPs). This note goes through them and says for
each what ed3k does. Everything said about a BEP is from reading its text at
[bittorrent.org/beps](https://www.bittorrent.org/beps/bep_0000.html); nothing was run, and how
far clients implement a BEP was not checked.

**Result:** two things were taken over as optional extensions, publish and subscribe (BEP 50)
and finding nodes in the local network (BEP 14), both in another shape (section 2). Most of
the rest ed3k has already in its own form (section 3). Three ideas were looked at and left for
later (section 4).

## 1. Overview

| Area | BEPs | ed3k | Section |
|---|---|---|---|
| Publish and subscribe | 50 | taken over, as a mesh | 2.1 |
| Local network | 14, 26 | taken over, presence only | 2.2 |
| Mutable data in the DHT, feeds | 44, 46, 49, 36, 39 | has it: head records of file sets | 3 |
| DHT | 5, 32, 42, 43, 45 | has it: Kad | 3 |
| File format and hashing | 30, 52, 47, 35, 53 | has it: BLAKE3 roots, file sets | 3 |
| Peer protocol | 6, 9, 10, 11, 21, 54, 55 | has it, mostly inside two messages | 3 |
| HTTP | 17, 19, 22, 25 | has it: HTTP sources and cache servers | 3 |
| Republishing less, congestion control, peer priority | 44, 29, 40 | looked at, not taken now | 4 |
| Trackers, indexing, private torrents, others | 3, 7, 8, 12, 15, 16, 18, 20, 23, 24, 27, 28, 31, 33, 34, 37, 38, 41, 48, 51 | not taken or does not apply | 5 |

## 2. Taken over

### 2.1 Publish and subscribe (BEP 50)

**What BEP 50 does.** A topic is a mutable item of the DHT (BEP 44): a value signed by a key,
with a sequence number. The nodes that follow a topic form a Kademlia overlay of their own for
it, entered through an ordinary peer announce under the item's target. In that overlay a
bucket holds one node; only the two buckets nearest the node's own ID hold 8. A node that
receives a valid item with a higher sequence number sends it to every node in its table for
the topic. The BEP is a draft.

**What ed3k has already.** The head record of a file set
([`file-set.md`](../extensions/file-set.md) §5) is BEP 44's mutable item with BEP 46's use: a
signed pointer to the newest version, with a sequence number, under a target made from key and
`id`, stored again by its followers. A follower polls it every 6 h. Only the push was missing.

**What ed3k takes.** The idea that followers tell each other. The item that is pushed is the
head record itself, so nothing new is signed.

**What it changes.**

| | BEP 50 | ed3k ([`pubsub.md`](../extensions/pubsub.md)) |
|---|---|---|
| Overlay per topic | a Kademlia table, one node per bucket | 2 to 4 neighbours, found through a `SUB` record |
| Who a node sends to | every node in the topic table, about log₂ n + 16 | its neighbours that have not shown the item |
| Transport | one datagram per message | a held connection per neighbour |
| Missed item | the node asks with `get` and its sequence number | the poll of the head record, which stays |
| Content | any bencoded value up to 1,000 bytes | typed records only: a set head, an on-air notice |
| Flood limit | none stated | one item per topic and 10 s |
| Number of topics | not limited | 16 with push, the rest by polling |

| Choice | Pro | Con |
|---|---|---|
| **Mesh instead of a table per topic** | no routing table to keep per topic; the live extension has the same mesh; a neighbour that leaves is replaced, not a path that breaks | no bound on the path an item takes; about 3 held connections per topic |
| **Polling kept under the push** | the mesh may be thin and need not be complete; an attacker who surrounds a node gains only delay | two mechanisms for one purpose |
| **Typed records only** | every hop can check what it passes on; no general store | each new use needs a specification |
| **A limit of 16 topics** | bounds connections and keep-alive traffic | a node that follows more gets the push only for some |

The other shapes that were considered (storing nodes that notify, polling only) are in
[`pubsub.md`](../extensions/pubsub.md) §8.

### 2.2 Local network (BEP 14)

**What BEP 14 does.** A client sends a multicast datagram in the form of an HTTP request every
5 minutes, with its port and the infohashes of the torrents it is active in. A client that
hears one of its own infohashes connects.

**What ed3k takes.** Multicast to find nodes on the same link, the 5 min interval and the
limit of one announce per minute.

**What it changes.**

| | BEP 14 | ed3k ([`lan-discovery.md`](../extensions/lan-discovery.md)) |
|---|---|---|
| Announced | port and infohashes | node ID and port |
| Files in common | read from the announce | asked inside the connection, 16 bytes of each root |
| Form | text, in clear | protobuf under the mask of §4.3 with a public key |
| Reach | groups of organisation and site scope | one link |
| Accepted from | any source | local addresses in a network of the receiving interface |
| Newcomer | waits for the next announces | is answered by each node once |
| Default | not stated | on in private networks, off in public ones |

| Choice | Pro | Con |
|---|---|---|
| **Presence only** | hosts in the network do not read what a node downloads | one more exchange before a transfer starts; a message pair of its own |
| **A mask with a public key** | no fixed bytes on the wire, as everywhere else in the transport | hides nothing from someone who knows the specification |
| **Off in public networks** | an announce gives away that the host runs ed3k | no local transfers in a guest network unless the user turns it on |

BEP 26 (Zeroconf) does the same job through DNS service discovery. ed3k does not use it, for
the reason given in [`lan-discovery.md`](../extensions/lan-discovery.md) §6.

## 3. What ed3k has already

| BEP | Subject | In ed3k |
|---|---|---|
| 5 | DHT | Kad (§10), with one request instead of four and with records beyond peers |
| 32 | IPv6 DHT | one overlay for both families (§8.1) instead of two |
| 42 | Node ID bound to the address | the routing ID (§6.3): the same hash and IPv4 mask, enforced from the start, and the rest of the ID bound to the key |
| 43 | Read-only nodes | a leaf (§2): a node without `ROUTER` is in no routing table and stores nothing |
| 45 | Several addresses per node | one node ID, one routing ID per address (§6.3, §10.2) |
| 44 | Mutable and immutable items | head records ([`file-set.md`](../extensions/file-set.md) §5.2); an immutable item is a small file. There is no store for free values |
| 46 | Following a publisher by key | `ed3k:set` links (`file-set.md` §5.4) |
| 49, 36, 39 | Feeds | a set that changes; a set may contain sets (`file-set.md` §2.5) |
| 30, 52 | Hash trees, one per file | a BLAKE3 root per file (§7); a set is a list of roots, so a file is the same thing in every set |
| 47 | Padding files, file attributes | padding is not needed, since no piece spans two files. Attributes (executable, symbolic link) are not carried |
| 35 | Signed torrents | a signed manifest (`file-set.md` §2.4) |
| 53 | Choosing files in a link | `path` in a link (`file-set.md` §5.4) |
| 9 | Metadata from peers | there is no metadata file; name and size come in `FileStatus`, a manifest is a file |
| 10 | Extension negotiation | capability bits in `Hello`, unknown fields ignored (§16) |
| 6 | Have-all, have-none, reject | an empty `have` (§12.1); `Error` |
| 21, 54 | Upload only; a piece no longer held | a node that only uploads sends no request; a changed `have` is sent again in `FileStatus` |
| 11 | Peer exchange | `FileStatus.sources`, limited to peers the sender has verified itself (§12.1) |
| 55 | Hole punching through a common peer | relays and `Punch` (§9.4, §9.5), with a reach test and NAT classes that BEP 55 does not have |
| 17, 19 | HTTP seeds | [`http-source.md`](../extensions/http-source.md) |
| 22, 25 | Finding a cache | `ed3k:cache` links (`http-source.md`) |
| 24 | The tracker reports the client's address | `Hello.observed` from every peer, counted by the rules of §9.2 |
| 33 | Size of a swarm from the DHT | in part: `FileInfo.sources` and the publisher sites of a keyword answer. There is no estimate over all storing nodes |

## 4. Looked at, not taken now

These would fit. They were left out of this round and are kept here so that they are not
looked up again.

### 4.1 Storing again only when needed (BEP 44)

BEP 44 lets a node skip the store when its lookup found the item on more than 8 nodes and the
8 closest all hold the newest sequence number. A `get` may also name the sequence number the
asker holds, and the answer then leaves the value out.

In ed3k a follower stores a head once per 24 h whatever it found
([`file-set.md`](../extensions/file-set.md) §5.2), and a search always returns the record.

| Pro | Con |
|---|---|
| a set with many followers is stored thousands of times a day for no gain | a head is stored once per day and follower, which is small against source records |
| answers to polls shrink to nothing when nothing is new | a field in `KadRequest` for one record kind; a rule that is easy to get wrong, so that everyone skips and the record dies |

### 4.2 Congestion control that gives way (BEP 29)

BitTorrent's own transport measures delay and slows down before the user's line is full, so a
transfer does not disturb other traffic. ed3k leaves congestion control to the sender and
recommends BBR (§4.1).

| Pro | Con |
|---|---|
| no wire change: it is the sender's choice already | not every QUIC library has a controller of this kind |
| a client that stays out of the way is left running | it loses against every other flow, also where the user wants speed; a recommendation without measurement |

### 4.3 The same peer priority at both ends (BEP 40)

Both ends of a connection compute one number from their two masked addresses. A client that
must choose keeps the connections with the highest number. Which peers a node keeps is then
not decided by who dialled first or most often, and an attacker cannot fill a node's
connections from a few addresses.

In ed3k this would apply to the neighbours of a live stream and of a topic. Today they are
chosen freely within the diversity limits ([`ip-diversity.md`](../extensions/ip-diversity.md)).

| Pro | Con |
|---|---|
| neighbour sets are hard to steer; the graph is close to random | the live extension picks neighbours by how well they deliver, which this would work against |
| nothing on the wire | the diversity limits cover the cheapest form of the attack already |

## 5. Not taken, or does not apply

| BEP | Subject | Why |
|---|---|---|
| 3, 7, 8, 12, 15, 23, 28, 31, 34, 41, 48 | Trackers | ed3k has no tracker. An index node (§11) is the nearest thing and speaks the Kad messages |
| 27 | Private torrents | there is no closed swarm of files. A private live channel is the one case, and uses a secret in the link ([`live.md`](../extensions/live.md) §10) |
| 51 | Sampling the infohashes a DHT node stores | makes it cheap to crawl what the network holds. ed3k has a keyword search for finding files, and a node does not list its records |
| 16 | Superseeding | needs nothing on the wire: an uploader may show fewer pieces in `have` than it holds. Left to the implementation |
| 38 | Finding local data for a new torrent | a member has the same root in every set, and `MEMBER` records lead from a file to its sets (`file-set.md` §6) |
| 18, 20, 37 | Search plug-ins, peer ID conventions, proxies | outside the protocol; the agent string is free text in `Hello` |
| 29 | A transport of its own | QUIC. The congestion control idea is in section 4.2 |

## 6. Open points

- The three ideas of section 4.
- An estimate of the number of sources over all storing nodes, as BEP 33 gives with Bloom
  filters, if `FileInfo.sources` turns out too rough.
- Whether file attributes belong in a manifest.
