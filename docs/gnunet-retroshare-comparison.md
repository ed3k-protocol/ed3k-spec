# GNUnet and RetroShare compared with ed3k

**Status:** design note (2026-10-12). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §9, §10 and §12, and the extensions [`friend`](../extensions/friend.md), [`bootstrap`](../extensions/bootstrap.md), [`hidden-set`](../extensions/hidden-set.md), [`multi-author-set`](../extensions/multi-author-set.md), [`filter-list`](../extensions/filter-list.md) and [`chat`](../extensions/chat.md). Section numbers (§) refer to `Protocol.md` unless another document is named.

**GNUnet** is a framework for networks without servers. Its parts used here are the DHT
(R5N), the name system (GNS) and the anonymous file sharing. **RetroShare** is a network of
friends: a node connects only to nodes whose certificates its user accepted, and everything
else (files, forums, channels, mail) travels along those connections.

GNUnet is described from its specifications of GNS and R5N, read on 2026-10-12, and its file
sharing from the published papers. For RetroShare only the source of its friend server was at
hand; the core library was not, and what is said about it is from its documentation. Nothing
was run.

**Result:** one thing was taken over as an optional extension: finding a known node
(section 2.1), and one rule was added to the first-contact extension (section 2.2). Eight
ideas were looked at and left (section 4).

## 1. Overview

| Area | From | ed3k | Section |
|---|---|---|---|
| Finding a known node | RetroShare's DHT, GNUnet's address records | taken over: a sealed record found with a friend link | 2.1 |
| The same answer for the same asker | RetroShare's friend server | taken over: a rule for list sources | 2.2 |
| Collections, channels, a key for posting apart from the owner's, reputations | RetroShare | has it | 3 |
| Records a storing node verifies but cannot read; checks on every storing node | GNUnet | has it | 3 |
| Key revocation | GNUnet | looked at, not taken now | 4.1 |
| Followers that sync what they hold | RetroShare, GNUnet | looked at, not taken now | 4.2 |
| Members of a closed group | RetroShare circles | looked at, not taken now | 4.3 |
| Keyword records the storing node cannot read | GNUnet | looked at, not taken now | 4.4 |
| Routing with a random phase | GNUnet R5N | looked at, not taken now | 4.5 |
| An estimate of the network's size | GNUnet | looked at, not taken now | 4.6 |
| Names | GNUnet GNS | looked at, not taken now | 4.7 |
| Messages for nodes that are offline | RetroShare | looked at, not taken now | 4.8 |
| A network of friends, anonymous transfer, an economy of trust | both | not taken | 5 |

## 2. Taken over

### 2.1 Finding a known node

**What RetroShare does.** Friends know each other's keys. To find where a friend is now, a
node looks the friend up in a DHT (the one of BitTorrent), which answers with an address. The
DHT carries nothing else. A node behind a NAT can be reached through a relay found the same
way.

**What GNUnet does.** Every peer signs a record of its addresses with an expiry time and puts
it into the DHT under its own identity. GNS, its name system, shows how to hide a record:
the place and the encryption key are derived from a public key and a label, so a storing node
verifies the record without learning either.

**What ed3k had.** Nothing. A node is found through its source records. A `ROUTER` can be
looked up by its ID while its address stays the same; a leaf cannot.

**What ed3k takes** ([`friend.md`](../extensions/friend.md)): the address record, but one that
only holders of a link can find and read. This is the *open link* of that extension. The form
for two friends, with keys agreed between their node keys, was added afterwards
([`tox-briar-comparison.md`](tox-briar-comparison.md) §2.1).

| | RetroShare | GNUnet | ed3k |
|---|---|---|---|
| Who can look a node up | anyone who knows its ID | anyone who knows its identity | holders of a friend link |
| What the DHT node reads | ID and address | the signed record | nothing |
| Place in the keyspace | fixed | fixed | changes every day |
| Proof | the connection, by certificate | a signature in the record | the handshake |
| A node behind NAT | relays found in the DHT | addresses of several transports | relays in the record |

| Choice | Pro | Con |
|---|---|---|
| **Only with a link** | a node ID cannot be followed from address to address by strangers | no way to reach a node of which only the ID is known |
| **Place and key from a secret in the link** | storing nodes learn nothing about whom the record is for | a storing node cannot check the record, so a holder of the link can add false ones |
| **Not signed** | the handshake proves the key; a false record costs one dial | none found |
| **A link per friend possible** | one friend can be dropped | one record to store per link |

### 2.2 The same answer for the same asker

RetroShare's friend server introduces a new user to a few others. Its source says why it
gives a profile the same peers every time it asks: a user who asks again and again would
otherwise collect everyone the server knows. It picks by a distance that is mixed with a
secret random value, so the choice cannot be predicted from outside.

A source of node lists ([`bootstrap.md`](../extensions/bootstrap.md) §5) has the same
exposure. The rule was added there: one requester gets the same sample for a time.

## 3. What ed3k has already

| System | Mechanism | In ed3k |
|---|---|---|
| RetroShare | Collections: a file that lists files with their hashes | file sets ([`file-set.md`](../extensions/file-set.md)) |
| RetroShare | Channels: posts with attached files, followed by subscribers, downloaded on arrival if the user wants | sets that change, with push ([`pubsub.md`](../extensions/pubsub.md)); live channels |
| RetroShare | A group has an admin key and a publish key; the owner can hand the publish key to others | the owner key and the keys of a grant ([`multi-author-set.md`](../extensions/multi-author-set.md) §1) |
| RetroShare | Forums where everyone posts under an identity of their own | open parts (`multi-author-set.md` §3) |
| RetroShare | Identities apart from the node, several per user | the key of a set or channel is not the node key |
| RetroShare | An opinion on an identity, combined with the opinions of one's friends; below a threshold its posts are dropped | filter lists ([`filter-list.md`](../extensions/filter-list.md)), which the user picks one by one |
| RetroShare | Comments and votes on posts | `NOTE` records on a root (§10.3) |
| RetroShare | Browsing what a friend shares, by permission | `Browse`, off or limited to friends by default ([`browse.md`](../extensions/browse.md)) |
| RetroShare | Finding nodes in the local network by broadcast | [`lan-discovery.md`](../extensions/lan-discovery.md) |
| RetroShare | A server that introduces newcomers | node lists ([`bootstrap.md`](../extensions/bootstrap.md)) |
| RetroShare | A map of the chunks a peer holds, and a check per chunk | `have` and the hash tree (§12.1, §7.2) |
| GNUnet | A record signed with a derived key, at a derived place, that a storing node verifies but cannot read | the hidden head ([`hidden-set.md`](../extensions/hidden-set.md)), with a key per set in place of a derived one; the alternative is weighed there |
| GNUnet | Every node on the way checks a block before it stores or passes it on | the checks of a storing node (§10.7), and of every hop in the push mesh |
| GNUnet | Address records signed with an expiry time | endpoints in `Hello`, proven by the connection; source records with a lifetime |
| GNUnet | Files by content hash; name spaces of a key | BLAKE3 roots; sets under an author key |
| GNUnet | A peer serves freely while it has room and prefers peers that served it when it has none | upload queue and credits, which are local policy (§12.5) |

## 4. Looked at, not taken now

### 4.1 Key revocation (GNUnet)

In GNS the owner of a key can declare it dead. The statement is signed with the key and
carries a proof of work, 32 values of Argon2 with 22 leading zero bits on average, which takes
long to compute and makes the statement valid for about a year; more work makes it valid
longer. The owner can compute it when the key is made and keep it. Nodes that have seen a
revocation keep it and pass it on.

In ed3k a stolen key of a set or channel cannot be stopped
([`i2p-zeronet-comparison.md`](i2p-zeronet-comparison.md) §4.1).

| Pro | Con |
|---|---|
| ends a stolen key for everyone who saw the statement | a node that never saw it accepts the thief's records. It has to reach every storing node of every day and every follower |
| can be made in advance and kept off line | whoever holds the key can revoke, the thief too. That only ends the set, which is what the owner wanted |
| | does not continue the set. A successor is still needed, and is the harder half |

It belongs with the successor key, which is open.

### 4.2 Followers that sync what they hold

RetroShare friends tell each other which groups they have and when each last changed, and
fetch what is newer. GNUnet has a service that finds the difference of two sets by sending a
summary much smaller than the sets.

In ed3k the parts of a set with open parts are found through Kad records, at most 1,024 per
target and without push ([`multi-author-set.md`](../extensions/multi-author-set.md) §7).

| Pro | Con |
|---|---|
| push for parts; no limit per target; every follower checks the full rules before it passes a part on | followers hold lists of all parts and compare them with every neighbour |
| | the topic exchange of [`pubsub.md`](../extensions/pubsub.md) holds one item per topic today. This would be a second kind of topic |

### 4.3 Members of a closed group (RetroShare circles)

A RetroShare circle is a list of identities. A group can be limited to a circle, and
membership needs both sides: an invitation and a request.

A hidden set has one secret for all followers
([`hidden-set.md`](../extensions/hidden-set.md) §5).

| Pro | Con |
|---|---|
| the owner could wrap the key of a version for each member, and leave a member out from the next version on | the head or the manifest grows with every member |
| | a member that was removed keeps what it had, and can pass it on |

### 4.4 Keyword records the storing node cannot read (GNUnet)

GNUnet encrypts the record that leads from a keyword to a file with a key made from the
keyword. A storing node sees a hash of the keyword. It can read the record only by guessing
the word.

In ed3k a keyword record carries the file name in clear, and the storing node uses it.

| Pro | Con |
|---|---|
| a storing node cannot choose what to drop by what a record says | it can no longer match a search against further words, sizes and the extension (§10.3), so every searcher downloads everything under a common word |
| | it cannot check that the record belongs under the keyword (§10.7), or count files per target (§10.8) |

### 4.5 Routing with a random phase (GNUnet R5N)

R5N forwards a request from peer to peer. For the first hops, as many as the logarithm of the
network's size, the next peer is chosen at random; after that it is the one closest to the
key. A filter in the request names the peers it has passed. The answer returns along the same
path.

In ed3k the requester asks each node itself and walks towards the target (§10.4).

| Pro | Con |
|---|---|
| reaches the right peers where many nodes cannot connect to each other | every peer on the path keeps state for requests of others, and a request can be dropped unseen |
| a lookup starts from a random place, so a requester with bad neighbours is harder to mislead | the requester cannot check each step, which §10.3 does; with QUIC a requester can reach any `ROUTER` itself |

### 4.6 An estimate of the network's size (GNUnet)

GNUnet peers estimate how many peers there are, and R5N uses the estimate.

| Pro | Con |
|---|---|
| would tell a publisher when storing by region pays ([`ipfs-freenet-comparison.md`](ipfs-freenet-comparison.md) §2.1) | a node can estimate it from how full its own buckets are, without a protocol |

### 4.7 Names (GNUnet GNS)

In GNS every user has a zone of names that the user assigns. A name can point to another
user's zone, so names are paths through people one knows. There is no global name.

| Pro | Con |
|---|---|
| readable names for sets, channels and friends, with no registry | a second system of records beside the file sharing; a mount ([`multi-author-set.md`](../extensions/multi-author-set.md) §2) already lets one set lead to another by key |

Taken since in [`name.md`](../extensions/name.md): zones under a key with names handed on to other
zones, as in GNS, without the key blinding (§10 there).

### 4.8 Messages for nodes that are offline (RetroShare)

RetroShare passes mail from friend to friend and keeps it until the receiver appears.

In ed3k a chat message needs a connection ([`chat.md`](../extensions/chat.md)).

| Pro | Con |
|---|---|
| a friend that is found later ([`friend.md`](../extensions/friend.md)) would get what was written before | someone has to store messages for others: a new service, and a new thing to flood |

## 5. Not taken, or does not apply

| System | Subject | Why |
|---|---|---|
| RetroShare | Connections to friends only | ed3k is an open network. A closed network ([`ipfs-freenet-comparison.md`](ipfs-freenet-comparison.md) §2.2) is the nearest thing |
| RetroShare | Tunnels through friends of friends, so that a file comes from someone unknown | ed3k does not hide a node's address from its peers |
| RetroShare | Certificates in the format of PGP | a key is an identity (§6.1) |
| RetroShare | One exchange system under forums, channels, identities and circles | ed3k has files, sets and records. Each use is specified on its own |
| GNUnet | Anonymous file sharing: requests that look the same whether a peer asks or forwards, cover traffic, content that moves between peers | as above |
| GNUnet | Trust earned by serving, as part of the protocol | credits are local policy (§12.1); credits that count everywhere are an optional extension with a ledger outside ed3k ([`extensions/credit.md`](../extensions/credit.md)) |
| GNUnet | Several transports under one peer identity | one binding ([`transport-bindings.md`](transport-bindings.md)) |

## 6. Open points

- The ideas of section 4. Revocation (4.1) belongs to the open question of the successor key.
- Whether friends should be able to tell each other that they are online without a search.
