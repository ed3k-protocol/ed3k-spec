# Scuttlebutt and Nostr compared with ed3k

**Status:** design note (2026-10-12). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §4.3, §10 and §13.2, and the extensions [`file-set`](../extensions/file-set.md), [`pubsub`](../extensions/pubsub.md), [`filter-list`](../extensions/filter-list.md), [`friend`](../extensions/friend.md) and [`multi-author-set`](../extensions/multi-author-set.md). Section numbers (§) refer to `Protocol.md` unless another document is named.

**Secure Scuttlebutt** is a social network without servers. Every user has a *feed*: a log of
signed messages, each naming the one before. Peers copy the feeds of the users they follow,
and of those users' friends. **Nostr** is a social network with simple servers. A user signs
*events* and sends them to *relays*; readers ask relays for the events of the keys they
follow. Extensions are numbered documents (NIPs).

Scuttlebutt is described from its protocol guide and Nostr from the index of its NIPs, both
read on 2026-10-12. What is said about single NIPs beyond their title and status is from
memory of their text and was not checked again. Nothing was run.

**Result:** nothing was taken over. Most of what the two do, ed3k has by now in its own form
(section 2). Nine ideas were looked at and left (section 3). Both systems also show a problem
that ed3k has left open as well: neither can replace a key (section 4).

## 1. Overview

| Area | From | ed3k | Section |
|---|---|---|---|
| Signed things with versions, followed by key; push; lists; requests; networks | both | has it | 2 |
| A hint in a link where the thing is found | Nostr | looked at, not taken now | 3.1 |
| Each version names the one before | Scuttlebutt | looked at, not taken now | 3.2 |
| Names through a web domain | Nostr | taken later, in [`name.md`](../extensions/name.md) §5 | 3.3 |
| A notice that withdraws something | Nostr | looked at, not taken now | 3.4 |
| A key that stays on another device and signs on request | Nostr | looked at, not taken now | 3.5 |
| Asking friends of friends for a file | Scuttlebutt | looked at, not taken now | 3.6 |
| Following what the people one follows follow | Scuttlebutt | looked at, not taken now | 3.7 |
| Finding the difference of two collections | both | looked at, not taken now | 3.8 |
| An end date set by the author | Nostr | looked at, not taken now | 3.9 |

## 2. What ed3k has already

| System | Mechanism | In ed3k |
|---|---|---|
| Scuttlebutt | A feed: messages signed by one key, numbered from 1 | versions of a set, signed and numbered by `seq` ([`file-set.md`](../extensions/file-set.md) §5) |
| Scuttlebutt | Files (blobs) named by their hash, at most 5 MB by default, asked for with want and have | files by BLAKE3 root, of any size (§7, §12) |
| Scuttlebutt | A network identifier in the handshake: another value makes another network | the network name in the obfuscation key (§4.3) |
| Scuttlebutt | An invite code for a server that then follows the newcomer | the friend request ([`friend.md`](../extensions/friend.md) §3); node lists for the first contact |
| Scuttlebutt | A broadcast every second in the local network, with address and key | [`lan-discovery.md`](../extensions/lan-discovery.md): every 5 min, node ID and port |
| Scuttlebutt | Servers that are always on and hold feeds for others | index nodes (§11); any node that follows a set stores its head again |
| Nostr | An event that replaces the older one of the same author and kind | the head record: a higher `seq` replaces it. Nostr orders by the author's clock, ed3k by a counter |
| Nostr | A subscription that a relay answers as events arrive | [`pubsub.md`](../extensions/pubsub.md), between followers and with no server |
| Nostr | Lists that a user publishes: who is muted, what is followed | filter lists ([`filter-list.md`](../extensions/filter-list.md)); a set of sets |
| Nostr | Reports on an event or a key, with a reason such as spam or malware | verdicts in a filter list |
| Nostr | Where a user's events are found, published by the user | `SOURCE` and `SUB` records, kept by Kad |
| Nostr | Proof of work on an event | reserved for node keys ([`key-puzzle.md`](../extensions/key-puzzle.md)) |
| Nostr | A message wrapped for one receiver under a key made for that message, so that others cannot see who wrote to whom | the sealed friend request (`friend.md` §3) |
| Nostr | Files on HTTP servers, named by their hash | HTTP sources and cache servers ([`http-source.md`](../extensions/http-source.md)) |
| Nostr | A listing of torrents with comments | keyword and `NOTE` records |
| Nostr | Groups that a moderator keeps | co-owners and open parts ([`multi-author-set.md`](../extensions/multi-author-set.md)) |

## 3. Looked at, not taken now

### 3.1 A hint in a link where the thing is found (Nostr)

A Nostr link to a profile or an event can carry the addresses of relays that have it.

An `ed3k:file`, `ed3k:set` or `ed3k:live` link names what to find and leaves the where to Kad.
Only HTTP sources can be named in a link (§13.2).

| Pro | Con |
|---|---|
| a link works at once, without a lookup, and when the records have expired | the link shows a node's address to everyone who sees it |
| a new or rare file is found by the first reader | a hint is stale as soon as the address changes; clients then wait for a dead node before they search |

A leaf could not be named by an address at all; the hint would have to carry relays.

### 3.2 Each version names the one before (Scuttlebutt)

A Scuttlebutt message carries the hash of the message before it. A feed is therefore one
chain, and two messages with the same number show that something is wrong: the key is in use
on two devices, or someone else has it. Clients stop following such a feed.

[`file-set.md`](../extensions/file-set.md) §8 weighed a pointer to the previous version as "a
history, not a way forward". A storing node already refuses a second head with the same `seq`
and another root (§5.2 there).

| Pro | Con |
|---|---|
| a follower that holds version 7 and receives a version 8 that does not name it knows that the chain was broken, and can warn | an author who publishes from two machines, or restores a backup, breaks the chain without any attack |
| a taken key is noticed instead of followed in silence | noticing is all. Scuttlebutt shows the price: a feed that split once is dead, and its owner starts again under a new key |

### 3.3 Names through a web domain (Nostr)

A Nostr user can claim `name@domain`. A client fetches a small file from that domain's web
server, which maps names to keys, and shows the name when the key matches.

| Pro | Con |
|---|---|
| readable names for authors and friends, checked by anyone, with no registry | a fetch over HTTPS outside ed3k, which a network operator sees |
| the domain is something a user may already trust | whoever controls the domain or its server can put another key behind a name |

Taken later in [`name.md`](../extensions/name.md) §5, with a DNS record beside the file at the
web server, and pins that warn when a domain's key changes.

A certificate issuer of a set with open parts
([`multi-author-set.md`](../extensions/multi-author-set.md) §3.2) is a narrower form: it
vouches for names inside one set.

### 3.4 A notice that withdraws something (Nostr)

A Nostr author can publish a request to delete earlier events. Relays and clients that honour
it stop showing them.

In ed3k a new version of a set can leave a member out, and followers keep the file until their
user decides ([`file-set.md`](../extensions/file-set.md) §5.3).

| Pro | Con |
|---|---|
| an author could take back a mistake, and followers would stop offering the file | nobody can be made to follow it; a copy that left stays out |
| | it is an instruction from outside to stop sharing, which the rule that nothing is deleted or changed without the user was written against |

### 3.5 A key that stays on another device (Nostr)

A Nostr client can leave the key on a second device and send it what is to be signed.

In ed3k the key of a set signs every version on the machine that publishes
([`i2p-zeronet-comparison.md`](i2p-zeronet-comparison.md) §4.1).

| Pro | Con |
|---|---|
| the key is not on the machine that is online all day | a protocol between a client and its signer, which is outside what two ed3k nodes say to each other |
| nothing changes for readers or storing nodes | the signer signs what it is sent; a taken client still gets signatures while the signer answers |

A grant that names another key ([`multi-author-set.md`](../extensions/multi-author-set.md)
§1.1) already lets an owner keep its own key away from the publishing machine.

### 3.6 Asking friends of friends for a file (Scuttlebutt)

A Scuttlebutt peer that wants a blob tells its peers. A peer that does not have it passes the
want on, marked as one step further away; the reference implementation stops after two steps.

| Pro | Con |
|---|---|
| a file is found without any global lookup | it serves small networks of people who know each other. Kad finds a source in any size of network |

### 3.7 Following what the people one follows follow (Scuttlebutt)

Scuttlebutt feeds say in public whom their owner follows. A client copies feeds up to three
steps away and shows those up to two.

| Pro | Con |
|---|---|
| a new user sees content at once, and finds people through people | every user stores and passes on feeds it never chose. Scuttlebutt is known for the size of a first sync |
| | the list of what a node follows is public |

ed3k has the careful form: a user may publish a set that names other sets, and others may
follow it.

### 3.8 Finding the difference of two collections (both)

Scuttlebutt peers exchange how far they have each feed and send what the other lacks. Nostr
has an extension that finds the events one side has and the other does not, with a summary
far smaller than the events.

This is what push for the parts of a set would need. It was weighed and left in
[`gnunet-retroshare-comparison.md`](gnunet-retroshare-comparison.md) §4.2.

### 3.9 An end date set by the author (Nostr)

A Nostr event can carry a time after which relays drop it.

Every ed3k record has `expires` (§10.7). A manifest has none.

| Pro | Con |
|---|---|
| an author could say that a version is good for a month | a manifest is a file. Its sources keep it as long as they like, and its head record already ends at a time the author chose |

## 4. A problem both leave open: replacing a key

- **Scuttlebutt.** A feed is its key. A key that is lost ends the feed. A key used on two
  devices splits the feed, and a split feed is no longer followed.
- **Nostr.** The index of NIPs has no entry for moving to a new key. The extension that let a
  key delegate signing to another is marked as not recommended, with the remark that it "adds
  unnecessary burden for little gain".

ed3k is in the same place for the key of a set or channel
([`file-set.md`](../extensions/file-set.md) §10). What it has is partial: a grant lets an owner
publish with other keys and replace them
([`multi-author-set.md`](../extensions/multi-author-set.md) §1), but the owner key itself
cannot be replaced. Nostr's remark is a warning for whoever closes this point: delegation was
specified there, hardly used, and withdrawn.

## 5. Not taken, or does not apply

| System | Subject | Why |
|---|---|---|
| Scuttlebutt | A log that only grows and is copied whole | a follower of a set needs the newest version, not all of them |
| Scuttlebutt | Copying everything from everyone within some steps | a node stores and serves what its user chose |
| Nostr | Relays as the place where everything is stored | Kad stores the records, and files are with the nodes that share them. An index node is optional |
| Nostr | The author's clock as the order of versions | a counter (§10.7 and [`i2p-zeronet-comparison.md`](i2p-zeronet-comparison.md) §6) |
| Nostr | Payments attached to events | outside the protocol |
| both | A social network: posts, replies, reactions | ed3k shares files. A note on a file is as far as it goes |

## 6. Open points

- The ideas of section 3. The hint in a link (3.1) and the version chain (3.2) are the two that
  would cost least.
- Replacing a key (section 4).
