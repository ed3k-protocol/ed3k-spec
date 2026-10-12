# IPFS and Freenet compared with ed3k

**Status:** design note (2026-10-12). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §4.3, §7, §10 and §12, and the extensions [`file-set`](../extensions/file-set.md), [`hidden-set`](../extensions/hidden-set.md), [`filter-list`](../extensions/filter-list.md), [`pubsub`](../extensions/pubsub.md) and [`bootstrap`](../extensions/bootstrap.md). Section numbers (§) refer to `Protocol.md` unless another document is named.

**IPFS** addresses data by its hash. A file is a tree of blocks, each block has an identifier,
and a Kademlia DHT says which peers hold a block. **Freenet** (now Hyphanet) is an anonymous
store: a node inserts encrypted blocks into the network, other nodes keep and route them, and
nobody knows who put a block in or who asked for it.

What is said here is from the IPFS specifications, the configuration reference of Kubo (the
reference implementation), the libp2p specifications and the Hyphanet wiki, read on
2026-10-12, and for Freenet's history from published papers. No source was read and nothing
was run. Earlier notes already compare parts of IPFS:
[`file-set.md`](../extensions/file-set.md) §9 (directories, IPNS),
[`nat-traversal-comparison.md`](nat-traversal-comparison.md) and
[`relay-count.md`](relay-count.md) (libp2p).

**Result:** two things were taken over, both into the core: storing source records by region
of the keyspace (IPFS), and a network name that separates networks (IPFS). Most of the rest
ed3k has in its own form (section 3). Seven ideas were looked at and left (section 4).

## 1. Overview

| Area | From | ed3k | Section |
|---|---|---|---|
| Publishing many items | IPFS reprovide sweep | taken over: batch store by region | 2.1 |
| Separate networks | IPFS private networks | taken over: a name in the obfuscation key | 2.2 |
| Exchange of blocks, names that follow changes, push, client and server role | IPFS | has it | 3 |
| Keys that update, files in parts, key in the link | Freenet | has it | 3 |
| Hashed entries in filter lists | IPFS denylist | looked at, not taken now | 4.1 |
| Lists that name lists | Freenet web of trust | looked at, not taken now | 4.2 |
| Recovery data and healing | Freenet | looked at, not taken now | 4.3 |
| One sealed form for the same file everywhere | Freenet | looked at, not taken now | 4.4 |
| Lookups that hide what is searched | IPFS, a draft | looked at, not taken now | 4.5 |
| The hash function named in the identifier | IPFS | looked at, not taken now | 4.6 |
| Lookups done by a server over HTTP | IPFS | looked at, not taken now | 4.7 |
| Anonymous routing, a store that forgets, blocks shared between files | both | not taken | 5 |

## 2. Taken over

### 2.1 Storing source records by region (IPFS)

**What IPFS found.** A provider record says "this peer has this item". It expires, so a node
stores it again every 22 h. With one lookup per item, each taking 10 to 20 s, a node manages
fewer than 10,000 items in that time, and a node with more loses records. Since Kubo 0.39 the
default is the *sweep*: the keyspace is cut into regions of at least 20 peers, the regions
are visited evenly over the 22 h, and the peers of a region get all records that fall into it
in one batch. The reference states about 3,000 lookups in a network of 10,000 DHT servers,
whatever the number of items, and 97 % fewer lookups for a node with 100,000 items.

**What ed3k had.** A source record per file, with a lifetime of 5 h, stored by a lookup of its
own. §10.5 already told a publisher to sort its targets so that neighbouring walks share
connections. Every file still cost one walk and one request per storing node.

**What ed3k takes** (§10.3, §10.4): one request may carry further targets (`Store.also`), each
answered on its own, and a publisher walks its sorted targets once per 4 h.

| | IPFS sweep | ed3k |
|---|---|---|
| Region | a prefix of the keyspace with at least 20 peers | the span of the closest set a lookup ended with |
| What a batch holds | all records of the region | the targets inside that span, at most 256 per request |
| Who decides that a node is the right one | the publisher, from its picture of the region | the storing node, per target: it answers `FAR` when it is not responsible |
| A target the batch missed | covered by the next region | gets a lookup of its own when fewer than 4 nodes stored it |
| Period | 22 h, records live about twice that | 4 h, records live 5 h |

**What it saves.** An estimate, not a measurement. The closest set of a lookup covers about
R / N of the keyspace, with R = 8 storing nodes and N routers. A node with F files then has
about F · R / N targets per region:

| Files shared | Routers in the network | Targets per region | Lookups per pass |
|---|---|---|---|
| 10,000 | 100,000 | 0.8 | about 10,000: no gain |
| 100,000 | 100,000 | 8 | about 12,500 |
| 1,000,000 | 100,000 | 80 | about 12,500 |

The gain begins where a node shares more files than the network has routers divided by R, and
from there the number of lookups no longer grows. In a small network it begins early: with
10,000 routers a node with 10,000 files already has 8 targets per region.

| Choice | Pro | Con |
|---|---|---|
| **A batch field in the core request** | the number of lookups is bounded by the network | a new field and a new result in the one Kad message; every router has to handle it |
| **The storing node decides per target** | a publisher with a wrong picture of the region cannot put records on the wrong nodes | answers of `FAR` cost a second attempt |
| **Source records only** | they are the records a node has many of. Keyword records are batched per keyword already | a downloader of many files still searches each one |
| **An even pass instead of a burst** | no load peak every 5 h, at the publisher or at storing nodes | a record is up to 4 h old when it is renewed |

IPFS also has a client that crawls the whole DHT every hour and then stores without walking.
Its own reference calls it very resource hungry and recommends the sweep first. ed3k does not
take it.

### 2.2 Separate networks (IPFS)

**What IPFS does.** A private network is a group of nodes that share a key file. Every
connection is encrypted once more with that key, below the normal handshake. A node without
the key cannot complete a connection, and nodes of the public network and of a private one do
not mix.

**What ed3k had.** One network. A test installation would have joined the public Kad as soon
as it met a public node.

**What ed3k takes** (§4.3): a network other than the public one has a name, and the name
enters the key of the obfuscation layer. That layer sits where the IPFS key sits: below the
handshake.

| Choice | Pro | Con |
|---|---|---|
| **The name in the obfuscation key** | no new layer and no new message; a foreign datagram fails the check every datagram already gets | the obfuscation layer was made to hide a protocol, not to guard a door. It keeps out whoever lacks the name and nothing more |
| **The public network has no name** | its key is what it was | a client has to treat "no name" as a value of its own |
| **A secret name for a closed network** | a group can run a network that others cannot enter or even recognise | one secret for all members; no member can be removed without a new name; a small network has few sources and few storing nodes |

What a closed network does not give: its members see each other's addresses as in any
network, and the name is as safe as its least careful member.

## 3. What ed3k has already

| System | Mechanism | In ed3k |
|---|---|---|
| IPFS | Bitswap: a peer sends a list of blocks it wants, and asks "do you have" before "send" | `have` and `BlockRequest` (§12.1, §12.2) |
| IPFS | A file is verified block by block against a root | the BLAKE3 tree (§7.2) |
| IPFS | A whole tree fetched in one exchange | one turn for several files of a set ([`file-set.md`](../extensions/file-set.md) §4.2) |
| IPFS | Directories as lists of names and hashes; large ones split | manifests and inner sets (`file-set.md` §2.5) |
| IPFS | IPNS: a signed record with a sequence number names the newest version | the head record (`file-set.md` §5) |
| IPFS | IPNS and other topics pushed between subscribers (gossipsub) | [`pubsub.md`](../extensions/pubsub.md). Gossipsub also tells peers outside the mesh what was seen, so that a broken mesh heals; ed3k keeps the poll for that |
| IPFS | DHT client and server mode: only reachable nodes are in routing tables | leaf and `ROUTER` (§2) |
| IPFS | Large providers publish to index servers in place of the DHT | index nodes (§11) |
| IPFS | Data fetched over HTTP and verified against the hash | [`http-source.md`](../extensions/http-source.md) |
| IPFS | Reach test, relays, hole punching (libp2p) | §9; compared in [`nat-traversal-comparison.md`](nat-traversal-comparison.md) |
| IPFS | A deny list as a shared file | [`filter-list.md`](../extensions/filter-list.md) |
| IPFS | Bootstrap peers named in the configuration | [`bootstrap.md`](../extensions/bootstrap.md) |
| Freenet | A key that can be updated: signed, with edition numbers, and clients that watch for the next edition | head records with `seq`; following and push |
| Freenet | A large file as many blocks under one small top block | a file under one root; a set under one manifest |
| Freenet | The key to read a block is in the link, not in the network | sealed manifests and members ([`hidden-set.md`](../extensions/hidden-set.md)); private channels |
| Freenet | A key made from a phrase, which anyone who knows the phrase may write | keyword records, signed by each publisher and kept side by side (§10.7). Freenet's form let the first or the loudest writer own the phrase |
| Freenet | Seed nodes for the open network; node references passed by hand for the closed one | node lists from sources, and by hand ([`bootstrap.md`](../extensions/bootstrap.md) §3) |
| Freenet | A node tells its neighbours which blocks it stores, as a Bloom filter | the member bitmap; `LanQuery` |

## 4. Looked at, not taken now

### 4.1 Hashed entries in filter lists (IPFS)

The IPFS deny list format can carry the hash of an identifier in place of the identifier. A
reader hashes what it meets and compares.

| Pro | Con |
|---|---|
| a list of harmful files is not a catalogue for finding them | nobody can see what the list blocks without holding the item. The IPFS specification advises against it as the default for that reason |
| | every item has to be hashed once per list that uses the form |

It would be one more `Subject` in [`filter-list.md`](../extensions/filter-list.md).

### 4.2 Lists that name lists (Freenet)

In Freenet's web of trust every identity publishes a list of trust values for others, from
−100 to +100. A reader computes a score for each identity from the lists of those it trusts,
and a list counts less the further its author is from the reader: 40 % at the first step, 16 %
at the second, 6 % at the third. A newcomer gets onto a list by solving a puzzle that an
established identity published. A related message system keeps two values apart: trust in
what an identity writes, and trust in whom it recommends.

In ed3k a filter list judges files and authors. It says nothing about other lists.

| Pro | Con |
|---|---|
| a user finds lists through the ones already followed | trust that spreads without the user is what gets gamed: an identity earns trust with good entries and spends it on bad ones |
| the split between "writes well" and "recommends well" is cheap to keep | every reader has to fetch the lists of everyone in reach. Freenet's is known for that cost |

A recommendation that the user has to confirm, one step deep, would be the careful form.

### 4.3 Recovery data, and healing

Freenet adds check blocks to every file: a segment has up to 128 data blocks and as many check
blocks, and any half rebuilds the segment. A node that had to rebuild a missing block inserts
it again, so a download repairs the file for the next reader. Later versions add check blocks
across segments, because large files used to stop just before the end.

In ed3k a file exists on its sources and nowhere else. A set whose last source of one member
has gone is incomplete for good.

| Pro | Con |
|---|---|
| a set could mark members as recovery data for others, and stay complete when a file is lost | a recovery format has to be chosen or defined, and every client needs it to gain anything |
| a downloader that rebuilt a member becomes a source of it at once, which is healing without a new rule | more data to store and share. An author can add recovery files to a set today as ordinary members |

### 4.4 One sealed form for the same file everywhere (Freenet)

Freenet makes the key of a block from the block's own content. The same file, inserted twice
by people who never met, gives the same encrypted blocks.

A sealed member of a hidden set ([`hidden-set.md`](../extensions/hidden-set.md) §4.2) is
sealed with the key of its set and a random nonce. The same file in two hidden sets is two
unrelated files.

| Pro | Con |
|---|---|
| sealed copies of one file share their sources across sets | whoever has the plain file can compute its sealed form and learn which nodes hold it. That is the opposite of what a hidden set is for |

### 4.5 Lookups that hide what is searched (IPFS, a draft)

A proposal for the IPFS DHT (still a draft, not deployed) looks an item up under a second hash
of its identifier and sends only a prefix of that. The provider records are encrypted with a
key made from the identifier itself. A DHT server then cannot tell which item a request means
unless it knows the item already.

In ed3k a source target is 16 of the 32 bytes of the root. A node on the path cannot download
the file from that, but it can match the target against roots it knows. Keyword targets are
hashes of the word.

| Pro | Con |
|---|---|
| a storing node learns what it serves only for files it knows | it knows most of them: roots are published under keywords for everyone |
| | a second hash and an encryption for every source record |

It would pay only for files that are never published under keywords, and hidden sets cover
those.

### 4.6 The hash function named in the identifier (IPFS)

An IPFS identifier starts with a code for its hash function and the length of the hash. A new
function needs no new format.

In ed3k the root is BLAKE3, a link says so with `b3=`, and every context string names the
version. A new function is a new wire version ([`hash-comparison.md`](hash-comparison.md)).

| Pro | Con |
|---|---|
| two functions can live side by side for years | every node has to implement every function in use, or the network splits into files it can and cannot verify |
| | IPFS shows the cost: the same file has many identifiers, by function, block size and tree layout |

### 4.7 Lookups done by a server over HTTP (IPFS)

A light IPFS client can leave the DHT walk to a server and ask it over HTTP who holds an item.

In ed3k an index node (§11) answers searches for every target, over the ed3k transport.

| Pro | Con |
|---|---|
| a client in a browser, or on a network where UDP is blocked, could take part | the server sees every request of its clients. It is a second transport, which [`transport-bindings.md`](transport-bindings.md) treats |

## 5. Not taken, or does not apply

| System | Subject | Why |
|---|---|---|
| Freenet | Requests routed hop by hop so that no node knows the origin | ed3k does not hide a node's address from its peers. Research on Freenet has shown that a peer can still tell a requester from a forwarder with good odds, so a weak form of this is worse than none: it promises what it does not keep |
| Freenet | A closed mode where a node talks only to peers its user knows | a closed network (section 2.2) is a group, not a web of friends. Routing through friends only is another protocol |
| Freenet | Positions in the keyspace that nodes swap to make routing work | positions come from address and key (§6.3). The swap was open to an attack that spoiled the positions of a whole network |
| Freenet | A store that keeps what is asked for and drops the rest; nothing can be deleted | a storing node holds records until they expire. Files are held by the nodes that chose them |
| Freenet | Every node stores blocks it cannot read, for others | a node stores and serves only what its user shares |
| IPFS | Blocks shared between files; cutting files by content | measured and rejected in [`file-set.md`](../extensions/file-set.md) §8 |
| IPFS | A record per block in the DHT | a record per file. This is why section 2.1 starts to matter at 100,000 files and not at 100 |
| IPFS | A data model for linked objects of any kind | ed3k carries files |
| IPFS | Asking a service to keep an item (pinning) | outside the protocol |
| IPFS | Public gateways from the web into the network | outside the protocol. IPFS use is concentrated on few of them (`file-set.md` §9 cites the measurement) |
| IPFS | Scores for peers in the push mesh | the mesh is thin and the poll is the safety net; a bad item ends the neighbourship ([`pubsub.md`](../extensions/pubsub.md) §2.2) |

## 6. Open points

- The estimate of section 2.1 needs a measurement, and with it the 4 h and the 256.
- A batch for *searching* sources of many files, the mirror of the batch store.
- Whether a closed network should get a real gate in the handshake, in place of the
  obfuscation key.
- The ideas of section 4.
