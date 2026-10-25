# I2P, iMule and ZeroNet compared with ed3k

**Status:** design note (2026-10-12; iMule and `hops` added 2026-10-26). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §6, §9, §10 and §13, and the extensions [`hops`](../extensions/hops.md), [`bootstrap`](../extensions/bootstrap.md), [`filter-list`](../extensions/filter-list.md), [`hidden-set`](../extensions/hidden-set.md), [`multi-author-set`](../extensions/multi-author-set.md), [`file-set`](../extensions/file-set.md), [`pubsub`](../extensions/pubsub.md) and [`live`](../extensions/live.md). Section numbers (§) refer to `Protocol.md` unless another document is named.

**I2P** is an anonymity network: every message travels through tunnels of several routers, and
a distributed database (the netDb) holds the records that lead to routers and services.
**iMule** is aMule moved onto I2P: the eD2K and Kad protocols, with an I2P destination in
place of every address.
**ZeroNet** distributes web sites: a site is a folder of files, listed with their hashes in a
file that the site's key signs, and every visitor serves it to the next.

What is said here is from reading the source of the Java I2P router, of ZeroNet and of
iMule 2.3.3.3. Nothing was run. Neither tree contains its protocol specification, so wire details of I2P that the
source does not show were not checked.

**Result:** four things were taken over as optional extensions: first contact through signed
node lists (I2P), shared filter lists (ZeroNet), hidden sets (I2P) and sets with several
authors (ZeroNet), each in another shape (section 2). Much of the rest ed3k has in its own
form (section 3). Five ideas were looked at and left (section 4). ZeroNet is also a list of things that went wrong, and section 6 checks
ed3k against it. Anonymity, left out when this note was first written, is now the extension
`hops`; section 7 compares it with I2P's tunnels and with what iMule built on them.

## 1. Overview

| Area | From | ed3k | Section |
|---|---|---|---|
| First contact | I2P reseed | taken over: signed node lists | 2.1 |
| Shared filter lists | ZeroNet mutes and block lists | taken over: a list file in a followed set | 2.2 |
| Records only link holders can read | I2P encrypted lease sets | taken over: hidden sets | 2.3 |
| NAT traversal, address check, load, Sybil limits | I2P SSU2, router info, Sybil analysis | has it | 3 |
| Signed content with versions, optional files, large files, peer exchange, local network | ZeroNet | has it | 3 |
| A signing key apart from the author key; a successor key | I2P offline signatures | left open | 4.1 |
| Checking a store, clock check, positions that change daily | I2P | looked at, not taken now | 4.2 to 4.4 |
| Several authors in one set | ZeroNet co-signers, includes, user content | taken over: co-owners, mounts, open parts | 2.4 |
| Web sites | ZeroNet | the distribution is there, the viewer is not protocol | 4.5 |
| Names, code from the network, a database | I2P, ZeroNet | not taken | 5 |
| Anonymity: tunnels, records that lead to a hidden node, Kad through others | I2P, iMule | in `hops`, in another shape; rotation and one-way tunnels not taken | 7 |

## 2. Taken over

### 2.1 First contact (I2P reseed)

**What I2P does.** A router that knows too few others fetches a bundle of router records over
HTTPS. The bundle is one signed file. The router ships with the list of servers and with the
certificates of their signers. It asks at least 2 servers, wants 100 records, and refuses a
bundle older than 30 days. A user can also load a bundle from a file, and a router can write
one for another user.

**What ed3k had.** `ed3k:node` links and the rule that a node enters Kad by looking up its own
ID. Where the first address comes from was not said.

**What ed3k takes** ([`bootstrap.md`](../extensions/bootstrap.md)): the signed list, two
sources, the age limit, and lists passed on by hand.

| | I2P | ed3k |
|---|---|---|
| Content | whole signed router records | node ID and address; the handshake proves the rest |
| Size | up to 1 MiB | up to 32 KiB |
| Used | when fewer routers are known than a threshold | after saved contacts, the local network, links and eD2K peers have failed |
| After the fetch | records go into the netDb | nodes are candidates to dial; the table is filled by lookups |
| Signer | certificate shipped with the router | key shipped with the client, or given in an `ed3k:nodes` link |

| Choice | Pro | Con |
|---|---|---|
| **A common list format** | every client and every operator of a source can share lists | list signers become parties that clients trust |
| **Two sources, half of the candidates each** | one bad source cannot choose a newcomer's whole first view | a newcomer needs two reachable hosts |
| **HTTPS** | works everywhere, mirrors and caches can serve | the fetch is visible to the network operator, which the transport otherwise avoids |

ZeroNet shows the other way: it finds peers through BitTorrent trackers and trackers of its
own, with no DHT behind them. A site whose trackers are gone is found only through peers that
a node already knows.

### 2.2 Shared filter lists (ZeroNet)

**What ZeroNet does.** A user mutes other users and blocks sites. Both lists are kept in one
local file. A user can also *include* a list that is a file on a site: its entries are merged
with the user's own, and they change when the site's owner changes the file. A blocked site
shows a warning with the reason, and the user can go on.

**What ed3k had.** A client can ignore or trust the author of a record
([`signed-records.md`](signed-records.md)). There was no way to share that.

**What ed3k takes** ([`filter-list.md`](../extensions/filter-list.md)): a list is a file that a
user follows, and it only acts in that user's client.

| | ZeroNet | ed3k |
|---|---|---|
| Judged | users (by key) and sites | files (by root) and authors (by key) |
| Verdicts | muted, blocked, with a reason | fake, harmful, spam, good |
| Carried in | a JSON file on a site | a binary file in a signed set that changes |
| Update | when the site is updated | head record, and push with `pubsub` |
| Default effect | a muted user's content is hidden | a result is marked; hiding is the user's setting |

| Choice | Pro | Con |
|---|---|---|
| **Lists act only at the searcher** | no list can decide what the network stores | spam still costs storing nodes room |
| **Mark by default** | a wrong entry is seen, not suffered | less effect for a user who never changes the default |
| **No addresses in a list** | a list cannot cut nodes off | no successor to the address filter of eMule |

### 2.3 Hidden sets (I2P encrypted lease sets)

**What I2P does.** The record that leads to a service can be stored in encrypted form. It is
signed with a key that is derived from the service's key and the date, so the storing router
can verify it and tell a newer from an older one, but learns neither the service's key nor the
content. Whoever knows the service's key can derive the same key and find the record. A
further secret can restrict who reads it.

**What ed3k had.** The construction for one case: a private live channel
([`live.md`](../extensions/live.md) §10), with a secret in the link, a target that changes
every hour, and encrypted segments.

**What ed3k takes** ([`hidden-set.md`](../extensions/hidden-set.md)): a head record that a
storing node verifies but cannot read, at a place that changes with the date.

| | I2P | ed3k |
|---|---|---|
| Key the storing node sees | a derived key, new every day | the key of the set, which is made for this set alone |
| Who signs, and when | the service, for every new record | the author, once per version |
| Place in the keyspace | follows the derived key | computed from the set key and the day |
| What opens the content | the service's key, and a secret if one is used | a secret in the link |
| What is hidden | the tunnels that lead to the service | the manifest, and members that the author seals |

| Choice | Pro | Con |
|---|---|---|
| **A key per set instead of a derived key** | standard signatures; the author can be offline and followers store the record | a node that stored the record once can find it on every later day |
| **A secret apart from the key** | storing nodes hold the key and still cannot read | two values in the link |
| **Members sealed by choice** | a public file keeps its sources | the author has to understand the difference |

### 2.4 Several authors in one set (ZeroNet)

**What ZeroNet does.** A site has three ways to let others write.

- The owner's file lists further signers and how many are needed; the site key signs that
  list.
- A directory is handed to other keys (*includes*), with a size limit.
- Visitors write into a directory named after their own address. The owner names no visitor.
  It names certificate issuers, and rules by pattern on the certified name: size limits and
  permitted file names. Single visitors can be given other limits or be banned, and content
  older than a date can be removed.

**What ed3k had.** One author per set. An inner set may be signed by another author
([`file-set.md`](../extensions/file-set.md) §2.5), but the outer author has to publish a new
version to take a change in.

**What ed3k takes** ([`multi-author-set.md`](../extensions/multi-author-set.md)): all three
forms.

| | ZeroNet | ed3k |
|---|---|---|
| Co-owners | signer list signed by the site key | a numbered grant signed by the owner key; storing nodes check it too |
| Handed-over directory | keys and a size limit in the parent file | a member that follows another set by key and `id`, with limits |
| Visitors are found | every peer of the site holds every visitor's directory | a Kad record per part, at a target made from set and issuer |
| Who keeps junk out | each peer, when it verifies | the storing node for sets with an issuer; the reader in any case |
| Rules | patterns on the name | limits per issuer, and changes per author |
| Removal | ban; everything before a date | ban; versions of one author below a number |
| What a follower downloads | everything, up to the site's limit | nothing of a part unless the user chose it |

| Choice | Pro | Con |
|---|---|---|
| **The issuer's key in the target** | a storing node can refuse keys without a certificate, without knowing the set's rules | one search per issuer |
| **A part is the author's own small set** | signed, versioned and transferred like any set | at most 1,024 parts per target in this version; no push |
| **Parts are never downloaded unasked** | a follower holds nothing from authors it did not choose | an open collection fills more slowly than a ZeroNet site |
| **The owner key above every grant** | a group cannot lock the owner out | the co-owner threshold does not hold against the owner |

## 3. What ed3k has already

| System | Mechanism | In ed3k |
|---|---|---|
| I2P | Peer test with three parties: a second router asks a third to send to the first | `ReachTest` (§9.3) |
| I2P | Introducers: a reachable router passes on a request to a firewalled one, then both send | relays and `Punch` (§9.4, §9.5) |
| I2P | A token before a handshake is answered; path challenge when an address changes | QUIC has both (§4.1) |
| I2P | Limits per address on handshakes, peer tests and introductions | [`flood-protection.md`](../extensions/flood-protection.md), §4.6 |
| I2P | A router publishes that it is congested (three levels) and others pass it by | `Error{BUSY}` with `retry_after`; a relay without slots clears `RELAY` (§9.4). The answer comes on asking, not in advance |
| I2P | A router takes the storing role by itself when it is reachable, has run for 2 h and has bandwidth | `ROUTER` is set by every node that is reachable (§2). There is no test for uptime |
| I2P | Sybil analysis: points for routers in the same /24 or /16, for IDs close to each other or to the own one; above a threshold a router is blocked | prevented at insertion instead: the routing ID is bound to the address (§6.3), and [`ip-diversity.md`](../extensions/ip-diversity.md) limits each network |
| I2P | Padding blocks in handshake and data packets | §4.4 |
| I2P | Comments and ratings on a torrent, exchanged between peers | `NOTE` records (§10.3) |
| I2P | Peers of a torrent from a DHT and from peer exchange | Kad and `FileStatus.sources` (§12.1) |
| ZeroNet | A signed list of files with hashes; the address is the key | a signed manifest and its head ([`file-set.md`](../extensions/file-set.md) §2.4, §5) |
| ZeroNet | Optional files: a peer tells which it holds by 2 bytes of each hash, and is asked for peers of others | the member bitmap (`file-set.md` §4.1); every member is a file with sources of its own |
| ZeroNet | Large files: a hash per piece, a field of the pieces a peer holds, written as run lengths | the BLAKE3 tree (§7.2) and `have` (§12.1), which zstd shortens (§5.3) |
| ZeroNet | A peer names files changed since a given time | a new manifest; members with the same root are not fetched again (`file-set.md` §5.3) |
| ZeroNet | An update is pushed from peer to peer | [`pubsub.md`](../extensions/pubsub.md) |
| ZeroNet | Peer exchange; trackers shared between peers | `FileStatus.sources`; Kad |
| ZeroNet | Broadcast in the local network, with the list of sites | [`lan-discovery.md`](../extensions/lan-discovery.md), which announces no content |
| ZeroNet | A limit on the size of a site, raised by the user | limits of a manifest (`file-set.md` §2.1); what a follower downloads is the user's choice per set |
| ZeroNet | A news feed of followed sites | following sets and channels |

## 4. Looked at, not taken now

### 4.1 A signing key apart from the author key, and a successor

I2P lets a service keep its long-term key off the machine. That key signs a second key with an
expiry time, and the second key signs the records. ZeroNet has nothing of the kind: the key of
a site is its address, and a site whose key is lost or stolen cannot be continued.

In ed3k the key of a set or channel signs every head and every notice, and
[`file-set.md`](../extensions/file-set.md) §10 and [`live.md`](../extensions/live.md) list the
successor as an open point. It stays open.

| Variant | Pro | Con |
|---|---|---|
| **Signing key with expiry, and a successor statement by the author key** | the author key can stay offline; a stolen signing key ends at its expiry; a lost one is replaced | one more signature and key per record, about 110 bytes of the 1 KiB; a stolen author key is still the end |
| **Successor only**: a head names the key that continues the set | small | the key in daily use is the only key, so a thief names its own successor first |

### 4.2 Checking a store

About 20 s after it stored a record, an I2P router asks another storing router for it, one
that is not in the same /24 as the first. If the record is missing it stores again elsewhere.

In ed3k a store is judged by the answers of the storing nodes (§10.8).

| Pro | Con |
|---|---|
| finds nodes that answer `STORED` and keep nothing | one more lookup per record that is checked |
| cheap for the few records that matter most, heads above all | I2P asks through a tunnel, so the node cannot tell who asks. In ed3k a node that lies only to others stays hidden from its publisher |

### 4.3 A clock check

I2P puts the time into every handshake, refuses a peer whose clock is more than 2 min off, and
corrects its own clock from its peers. ZeroNet orders versions by a time stamp and accepts one
up to a day in the future.

In ed3k `expires` is a clock time with 1 h of tolerance (§10.7). A node whose clock is wrong
publishes records that every storing node refuses, and nothing tells it why.

| Pro | Con |
|---|---|
| a field in `Hello` would let a node notice and correct | peers learn each other's clock, which the live extension avoids on purpose |
| | `REJECTED` on a store, from several nodes, is already a hint a client can act on |

### 4.4 Positions that change every day

I2P does not store a record at the hash of its key but at the hash of that key and the date.
Routers that placed themselves around a record are in the wrong place the next day.

| Pro | Con |
|---|---|
| a position taken around a target is worth one day | every record moves at midnight; publishers and searchers work with two days around it |
| | in ed3k a position cannot be chosen freely: its top bits come from the address (§6.3), and the key puzzle ([`key-puzzle.md`](../extensions/key-puzzle.md)) is reserved for the rest |

ed3k uses the idea where it gives privacy and not only cost: for private channels and hidden
sets.

### 4.5 Web sites

A ZeroNet site is a folder of static files, signed and distributed between its visitors. An
ed3k set that changes is the same thing: a signed list of files under a key, with a link that
always leads to the newest version, and followers who serve it.

What ZeroNet adds is outside the transfer:

- **A viewer.** The client serves the files to a browser from a local port and keeps sites
  apart from each other. In ed3k this would be a function of a client. It needs nothing on the
  wire.
- **A database.** A site that ships a schema file gets an SQLite database on every visitor's
  machine. The client fills it from the site's JSON data files, and the site's scripts query
  it through the client. Only `SELECT` is accepted. The database is an index built locally; it
  is never transferred.
- **Content from visitors**, which is section 2.4.

## 5. Not taken, or does not apply

| System | Subject | Why |
|---|---|---|
| I2P | Choosing peers by measured speed and capacity, kept in profiles | `hops` measures whole paths and replaces the slowest ([`hops.md`](../extensions/hops.md) §2.1); it keeps no profile per hop. Which source or relay a node prefers is its own policy |
| I2P | Routers of one operator declare a family, so that no tunnel uses two of them | `hops` keeps the hops of a path in different sites and blocks (`hops.md` §2.1). A family is declared only by operators who are honest anyway |
| I2P | Names for services: a local address book, fed by lists the user subscribes to | a name service is not part of ed3k. A link carries the key, and a client may let its user name a key |
| I2P | Updates of the router, signed and fetched through the network | outside the protocol. A client could follow a signed set for its releases |
| ZeroNet | Names through a blockchain | as above; and a second network that every node would depend on |
| ZeroNet | Site code and client updates delivered as sites | ed3k carries files. Nothing received from the network is run |
| ZeroNet | Identity through certificate issuers, for every use | a key is an identity (§6.1). An issuer appears only where the owner of a set asks for one (section 2.4) |
| ZeroNet | Sites merged into one view; cloning a site | a function of a client |
| ZeroNet | Messages encrypted to a user's key | chat is on an encrypted connection ([`chat.md`](../extensions/chat.md)); there are no stored messages |
| both | Reaching peers through Tor or I2P | a possible transport binding ([`transport-bindings.md`](transport-bindings.md)); what iMule teaches about it is in section 7.5 |

## 6. What went wrong in ZeroNet, and where ed3k stands

| In ZeroNet | In ed3k |
|---|---|
| A version is a time stamp. A publisher with a fast clock, or one who sets a date ahead, wins against later versions | a version is a counter (`seq`). The clock decides only when a record expires |
| The signature is made over the JSON text as the reader writes it again, sorted. Two implementations must format every value alike; the source carries a correction for one platform's numbers | a signature covers the bytes as they were sent (`body`, §10.7) |
| An update can delete files at the visitor | a client never deletes or overwrites a file because of a new version ([`file-set.md`](../extensions/file-set.md) §5.3) |
| One key per site, used for every signature, with no successor | the same today; open (section 4.1) |
| Peers are found through trackers only | Kad; source exchange; the local network; node lists only for the first contact |
| Sites bring scripts that run in the visitor's browser with access to the client | no code is transferred |
| Every visitor holds and serves the whole site, up to a size limit | a follower chooses what it downloads; every file is shared on its own |

## 7. Anonymity: I2P tunnels, iMule and `hops`

### 7.1 How iMule works

iMule 2.3.3.3 is a fork of aMule 2.3.1. It keeps eD2K and Kad and replaces the IP network with
I2P.

- It talks to an I2P router through SAM version 3 on port 7656. It can start a router of its
  own: the Java router, compiled to native code with GCJ and linked into the client. Or it
  uses a router that runs apart.
- It opens a stream session for what was TCP and a datagram session for Kad, each with its own
  destination, and one more datagram destination for eD2K servers. A Kad contact carries both
  the stream and the datagram destination.
- A destination is 387 bytes, 516 characters in base64. It stands wherever an IP address and a
  port stood: in Kad contacts, source lists, server lists and links.
- The Kad ID is a random number that the client keeps. It is not tied to the destination.
- The tunnels: 2 hops inwards and 2 outwards, 2 tunnels of each kind per session. The router
  forwards for others with 80 % of its bandwidth. A random variance of the length is not set.
- Every destination can be reached, so every node is a full Kad node. Firewalls, LowID and
  callbacks are gone.
- It is a network apart. An iMule node reaches iMule nodes only, never an eMule or aMule source
  of the same file.

### 7.2 What `hops` has of it

| iMule, through I2P | ed3k, through [`hops`](../extensions/hops.md) |
|---|---|
| Hops inwards and outwards set apart (`inbound.length`, `outbound.length`) | 0 to 3 hops for each direction, the node's own choice (`hops.md` §1) |
| A peer is reached by its destination; a lease set in the netDb says where its tunnels end | an `ANON` record names a source by node ID, a `GATE` record names its gates (`hops.md` §4.3). The same split as destination and lease set |
| 2 tunnels per direction and session | up to 4 paths outwards and 4 gates (`hops.md` §2.1) |
| Kad lookups through tunnels | `HopLookup`: the last hop sees what is searched, not by whom (`hops.md` §4.1) |
| The router forwards for others, 80 % of its bandwidth | `HOP`, with a share the user sets, a quarter at first, and give and take between neighbours (`hops.md` §5) |
| Peers that cannot connect directly reach each other through tunnels | a path of one hop is the data relay that the core leaves out (§9.4, D10) |
| A network apart | named and closed networks, for those who want one (§4.3) |
| Seed list of Kad nodes shipped with the client | signed node lists ([`bootstrap.md`](../extensions/bootstrap.md)) |

### 7.3 What went wrong in iMule, and where ed3k stands

| In iMule | In ed3k |
|---|---|
| A network apart: a file has the sources that run iMule, and no others | one network. A node that hides downloads from open sources, and its upload counts for them as any other |
| A 387-byte destination in every contact, source and link | a node ID of 16 bytes in contacts and `ANON` records. Gates are looked up once per source and node, with the `GATE` record (`hops.md` §4.3) |
| Two destinations per node, three with servers, which an observer can link to each other | one anonymous key per node, apart from its node key, and always a leaf (`hops.md` §1) |
| The Kad ID is chosen freely, so a node can place itself next to any target | the node ID comes from the key, the routing ID's top bits from the address (§6.3). Anonymous keys are in no routing table |
| The router inside the client was built with GCJ, which GCC dropped in version 7; the client was then tied to an old toolchain | nothing outside the protocol is needed. A binding over I2P would use a router that runs apart (section 7.5) |
| Every node reachable, every node a Kad node | a hidden node is a leaf. Kad is kept up by the nodes that do not hide |

### 7.4 Looked at, not taken

**A maximum lifetime of a path.** I2P replaces every tunnel after 10 minutes. In `hops` a path
lasts until it breaks or is far slower than the others, and a gate lasts as long as its path.
A timer was looked at and is not taken.

- **Each new path is a new draw.** The first hop stays for 30 days (entry hops, `hops.md`
  §2.1). The last hop and the gate are picked again with every path. If an entry hop is
  hostile, each rebuild is one more chance that the last hop or the gate is hostile too, and
  the two can then match the traffic by its timing. At 10 minutes that is 144 draws per path
  and day in place of a few a week. This is the predecessor attack, and Tor keeps its guards
  for months for this reason: a client that picks new relays often will sooner or later pick
  hostile ones at both ends. A timer would spread exposure over many short spans. It would
  not make it rarer, and it would make it certain over time.
- **It would hide nothing in ed3k.** Tor and I2P rotate so that later activity does not share
  a circuit with earlier activity. In `hops` the session and the anonymous key stay the same
  over every path (`hops.md` §3.2). The peer at the other end sees the same key whatever path
  is used, and a hostile last hop learns no more from a short path than from a long one.
- **It costs.** Each rebuild is a handshake through every hop. Streams on the old path end and
  start again on the next, and a gate that moves is a new `GATE` record.

The same draw is made when a path breaks or is replaced for being slow. A hostile hop can slow
a path to get it replaced. A client SHOULD limit how often it replaces paths that did not
break. How often is local policy.

| Choice | Pro | Con |
|---|---|---|
| **No timer; replace a path when it breaks or is slow** | few draws of last hops and gates; no cost while a path works | a hostile pair that holds a path keeps it until the path ends |
| **A timer, as in I2P** | a hostile pair loses a path after its lifetime | many more draws; nothing hidden that the session and the key do not show anyway |

**Tunnels in one direction.** In I2P a tunnel carries one direction. A request leaves through
the sender's outbound tunnel and the reply comes back through its inbound one, so no hop sees
both. A path of `hops` carries both directions: each link is a reliable exchange, and its loss
repair and flow control need traffic back on the same link (`hops.md` §2.4, §7).

| Pro | Con |
|---|---|
| no hop sees request and reply, nor their timing together | packets without delivery on every link and loss repair from end to end, which `hops.md` §7 decided against for throughput |
| | twice the paths for the same number of hops |

**Random variance of the path length.** I2P can add or remove hops at random, so that a
tunnel does not show the setting. A hop of `hops` does not learn from the protocol how long its
path is, so there is little to hide. A client MAY add a hop now and then; this needs nothing
on the wire. It SHOULD NOT remove one, because that gives a user less than they set.

**Hiding by default.** In I2P everyone is hidden, and the crowd is every user. In ed3k it is
each node's choice, and the crowd is the nodes that hide. A larger crowd helps each of them. A
default costs every user throughput, since each hop uploads every byte again. This is the
policy of a client, and it stays an open point (section 8).

### 7.5 A binding over I2P

[`hops.md`](../extensions/hops.md) §7 lists a binding over Tor or I2P as an alternative to
`hops`. iMule shows what such a binding would meet.

- **T1, the node key.** A destination is a key of I2P, not the node's Ed25519 key. The node key
  needs a proof inside the stream, as with TLS.
- **T3 and the routing ID.** A node with no IP address has no routing ID bound to one (§6.3).
  It would be a leaf, as a hidden node of `hops` is, unless the routing ID gets a rule for
  destinations.
- **T4, exchanges.** A SAM stream is one ordered stream, and each new one is a handshake through
  tunnels, which takes seconds. A binding needs a multiplexer inside one stream, with the
  head-of-line blocking that QUIC avoids.
- **T5** is met by I2P itself, and no second layer is added.
- **Addresses.** In place of the 387 bytes, a `Binding` entry in `Peer` (field 6, reserved
  for the endpoints of other bindings, [`transport-bindings.md`](transport-bindings.md)) would
  carry the 32-byte hash of the destination, and the peer looks the rest up in the netDb.
- **One destination.** SAM 3.3 lets streams and datagrams share one destination. iMule's split
  over two shows why that matters.
- **A router that runs apart.** A router in the client ties the client to the router's code and
  toolchain, as GCJ did iMule.
- **A network apart, again.** A node that speaks only I2P reaches only nodes that speak I2P,
  which is iMule's split. Nodes that speak both would be its only bridge, and they would see
  both sides.

| Pro | Con |
|---|---|
| a large crowd, and years of study | everything above; and the speed of I2P, which is that of its slowest tunnel hop |
| every node reachable | a split swarm, unless many nodes speak both |

## 8. Open points

- The ideas of section 4, above all the signing key and the successor.
- Whether `ROUTER` should need a time of proven reachability, as I2P asks for its storing
  routers.
- Whether a node should say that it is busy before it is asked, as I2P's congestion levels do.
- A transport binding over an anonymity network (section 7.5).
- Whether a client should hide by default, so that hiding nodes are a larger crowd
  (section 7.4).
