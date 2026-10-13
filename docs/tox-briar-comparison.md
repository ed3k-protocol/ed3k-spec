# Tox and Briar compared with ed3k

**Status:** design note (2026-10-12). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §6 and §9, and the extensions [`friend`](../extensions/friend.md), [`chat`](../extensions/chat.md), [`lan-discovery`](../extensions/lan-discovery.md) and [`bootstrap`](../extensions/bootstrap.md). Section numbers (§) refer to `Protocol.md` unless another document is named.

**Tox** is a messenger without servers: friends find each other through a DHT and then talk
directly. **Briar** is a messenger for hostile conditions: contacts sync messages over Tor,
over the local network or over Bluetooth, and nothing depends on a server.

Both are messengers, so what they can teach ed3k is about nodes that know each other. Tox is
described from the first half of its protocol specification and Briar from its rendezvous
protocol, both read on 2026-10-12; the rest of Briar is from its documentation. Nothing was
run.

**Result:** two things were taken over, both into the extension for finding a known node:
pairwise keys (Briar) and the friend request (Tox). Eight ideas were looked at and left
(section 4).

## 1. Overview

| Area | From | ed3k | Section |
|---|---|---|---|
| Friends by exchanged keys, no shared secret | Briar rendezvous; Tox | taken over: pairwise keys | 2.1 |
| Asking to be added | Tox friend request | taken over: a sealed request | 2.2 |
| Local network, first nodes, hole punching, groups with an owner | both | has it | 3 |
| Staying connected to friends | Tox | looked at, not taken now | 4.1 |
| Introduction by a common friend | Briar | looked at, not taken now | 4.2 |
| A new identity in the DHT at every start | Tox | looked at, not taken now | 4.3 |
| Hiding the publisher from the storing node | Tox | looked at, not taken now | 4.4 |
| Keys that move forward in time | Briar | looked at, not taken now | 4.5 |
| Relays that carry data | Tox | looked at, not taken now | 4.6 |
| Guessing ports of a symmetric NAT | Tox | looked at, not taken now | 4.7 |
| Sync without the internet | Briar | looked at, not taken now | 4.8 |

## 2. Taken over

### 2.1 Pairwise keys (Briar)

**What Briar does.** Two users exchange public keys, in any way they like. Each computes the
same secret from its own private key and the other's public key, hashes it with both public
keys, and derives from it where to meet: for Tor, the keys of two hidden services, one per
side. No address is exchanged. Each side tries for 48 hours. Briar's specification is plain
about the limit: the protocol does not authenticate the other side, and if the keys were
swapped on the way, it cannot tell.

**What Tox does.** A friend is added by its long-term key. Adding works without any message
when both sides do it.

**What ed3k had.** Draft 0.1 of [`friend.md`](../extensions/friend.md): a link with a secret,
and everything derived from the secret.

**What ed3k takes** (`friend.md` §1): the key agreement between the two node keys. The form
with a secret stays as the *open link*, for a node that wants many people to find it.

| | Briar | ed3k |
|---|---|---|
| Exchanged | public keys made for this purpose | the node keys |
| Derived | the keys of two hidden services | a place in Kad and a record key, per direction and day |
| Lasts | 48 hours, to make first contact | as long as both keep each other |
| Who proves what | the application, afterwards | the handshake proves the node key (T1) |

| Choice | Pro | Con |
|---|---|---|
| **No secret in a friend link** | a link can be shown anywhere; nothing leaks with it | both sides have to add each other |
| **A key per pair and direction** | one friend can be dropped alone; a friend cannot plant records for the others | one record to store per friend, at most 64 |
| **The node key itself** | no second key to manage; the handshake proves it | whoever takes the node's private key finds every place of that node |
| **Both forms kept** | one link for many people is still possible | two derivations for one record |

### 2.2 The friend request (Tox)

**What Tox does.** A Tox address is the long-term key, 4 further bytes and a checksum. A user
who has someone's address can send a request with a short message, and the other accepts or
not. The 4 bytes exist so that the owner can change them: old addresses then no longer reach
the owner, and the key and the friends stay.

**What ed3k had.** Nothing. With pairwise keys two users would have to pass links in both
directions before anything happens.

**What ed3k takes** (`friend.md` §3): a request that the asking node leaves in Kad for the
other, and a value in the link that the owner can change.

| | Tox | ed3k |
|---|---|---|
| Value that can be changed | 4 bytes, checked by the receiver | 8 bytes, part of the place where requests are left |
| How a request travels | to the receiver, through other nodes | stored in Kad until the receiver looks |
| Who can read it | the receiver | the receiver: it is sealed to the receiver's key with a key made for that one record |
| Proof of the sender | the packet is encrypted with the sender's key | a signature by the asking node's key |
| The receiver is offline | the sender repeats the request | the request waits, renewed for up to 7 days |

| Choice | Pro | Con |
|---|---|---|
| **Stored, not sent** | the asked node need not be findable, which before friendship it is not | storing nodes hold requests; 32 per place, 8 per host and 16 per site |
| **The value in the place, not only in the content** | a stranger cannot even store a request | 8 bytes that should not be public. They guard against annoyance only, and are replaced at no cost |
| **Never accepted without the user** | a request is a question | none found |

## 3. What ed3k has already

| System | Mechanism | In ed3k |
|---|---|---|
| Tox | A packet with the node's key, broadcast in the local network every 10 s; the receiver then asks that node | [`lan-discovery.md`](../extensions/lan-discovery.md): multicast every 5 min, answered once by each node |
| Tox | First nodes from a list | [`bootstrap.md`](../extensions/bootstrap.md) |
| Tox | Hole punching through nodes that know both sides, started only when the other side is seen to try as well | `Punch` through a relay (§9.5) |
| Tox | Files sent between friends, paused and continued | a file stream with `have` (§12.1) |
| Tox | Friends see each other's address | the same. A friend is a peer |
| Briar | A group whose creator signs who may join | the grant of a set with co-owners ([`multi-author-set.md`](../extensions/multi-author-set.md) §1) |
| Briar | Forums and blogs that contacts pass on | sets that change; open parts |
| Briar | Contacts in the same local network connect directly | LAN peers |
| both | A link or address exchanged outside the protocol, which the protocol cannot verify | the same; said in `friend.md` §1 |

## 4. Looked at, not taken now

### 4.1 Staying connected to friends (Tox)

Tox keeps a connection to every friend that is online and sends small packets to hold it. A
friend is seen online within seconds, and status and typing notices travel on it.

In ed3k a node looks a friend up when its user wants to reach it, or once per 30 min.

| Pro | Con |
|---|---|
| presence without polling; chat starts at once | one held connection per online friend |
| | friends always know when a node is online |

### 4.2 Introduction by a common friend (Briar)

A Briar user can introduce two contacts to each other. Both are asked, and when both accept
they become contacts without ever exchanging anything themselves.

| Pro | Con |
|---|---|
| people who share a friend need no exchange of links | a message of its own between friends |
| | both trust the introducer to pass the right keys |

A friend link with a request value, passed on in a chat, does most of this already.

### 4.3 A new identity in the DHT at every start (Tox)

Tox uses a fresh key pair in the DHT each time it starts. The long-term key is shown to
friends only, and the specification takes care that the two cannot be linked by others.

| Pro | Con |
|---|---|
| a node cannot be followed in the DHT from session to session | in ed3k the node ID is what credits, source records, authors of records and ignore lists are keyed by (§6.1) |
| | the routing ID is bound to the key (§6.3), and the key puzzle would be paid at every start |

### 4.4 Hiding the publisher from the storing node (Tox)

Tox announces where a node can be found through three other nodes in a row, so that the node
that stores the announcement does not see the announcer's address.

In ed3k the storing node of a friend record sees the node ID and address of the publisher,
though not for whom the record is ([`friend.md`](../extensions/friend.md) §5).

| Pro | Con |
|---|---|
| the storing node could not tell which node has friends | three more nodes per store, each of which can drop it |
| | a record bound to the connection is what keeps `FRIEND` records from being flooded |

### 4.5 Keys that move forward in time (Briar)

Briar derives the keys of a contact anew for every period of time and deletes the old ones. A
device that is taken later gives up no earlier traffic.

| Pro | Con |
|---|---|
| a private key taken later would open no earlier place or record | both sides have to keep the same step, also after months apart |
| | a friend record holds the current address and nothing else; hidden sets would gain more, and there every follower would have to keep step |

### 4.6 Relays that carry data (Tox)

A Tox node that cannot use UDP connects to relays over TCP and sends everything through them:
3 by choice, 6 at most. The specification notes that nothing detects a relay that lies or
drops packets.

In ed3k a relay passes signalling only (D10), and the transport is UDP alone.

| Pro | Con |
|---|---|
| nodes on networks that block UDP could take part | file transfer through relays makes every relay pay for strangers' traffic; §16 keeps a data relay reserved |
| | it needs a second binding ([`transport-bindings.md`](transport-bindings.md)) |

### 4.7 Guessing the ports of a symmetric NAT (Tox)

Against a NAT that uses a new port per destination, Tox tries ports near the ones it has seen,
48 every 3 seconds, and after 5 rounds also from port 1024 upwards.

ed3k weighed this in §9.5 and left it out: hundreds of probes per attempt trip the session
limits of routers. §16 keeps it reserved. Tox adds one detail worth keeping in mind for that
day: it starts only after the other side was seen to be trying too.

### 4.8 Sync without the internet (Briar)

Briar contacts exchange messages over Bluetooth, over a local network without an uplink, and
by files on removable media.

| Pro | Con |
|---|---|
| a set could travel from follower to follower with no network at all | a set is files already: a manifest and its members can be copied on any medium, and every file is verified against its root on import |
| | LAN peers work without an uplink once both nodes know each other ([`lan-discovery.md`](../extensions/lan-discovery.md)) |

## 5. Not taken, or does not apply

| System | Subject | Why |
|---|---|---|
| Tox | Audio and video calls, group chats | ed3k is not a messenger. Live streams are one-to-many ([`live.md`](../extensions/live.md)) |
| Briar | Tor as the transport | a possible binding; not looked at here |
| Briar | Key agreement in person, by scanning codes | a matter of the client. A friend link can be shown as a code |
| Briar | A device of one's own that takes messages while one is offline | see [`gnunet-retroshare-comparison.md`](gnunet-retroshare-comparison.md) §4.8 |

## 6. Open points

- The ideas of section 4. Presence (4.1) and introduction (4.2) are the two that
  [`friend.md`](../extensions/friend.md) §7 lists.
- Whether a node key should be the key that friends agree on, or a second key made for it.
