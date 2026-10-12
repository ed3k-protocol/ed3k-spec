# Extension: finding a known node (`friend`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §9, §10.1, §10.3 and §13.2. Section numbers (§) refer to that document unless this one is named.
**Capability:** none. Record kind 10. One field in `Store`, one in `KadAnswer`, one link.
**Required of every `ROUTER`:** storing and returning the record kind `FRIEND` (§3). The rest is optional.

A node is found through what it shares: its source records. A node that a user *knows*, a
friend, cannot be found at all once its address has changed. Chat
([`chat.md`](chat.md)) and browsing ([`browse.md`](browse.md)) need a connection, and a node
behind a NAT is in no routing table.

- A node that wants to be found gives a **friend link** to those who may find it.
- It stores a small **record** of its addresses and relays in Kad, encrypted with a key from
  the link, at a place that follows from the link and **changes every day**.
- Whoever has the link finds the record, reads it and dials. Nobody else can find or read it.

RetroShare uses a DHT for exactly this, and GNUnet stores an address record per peer
([`docs/gnunet-retroshare-comparison.md`](../docs/gnunet-retroshare-comparison.md) §2.1).

---

## 1. The link

```
ed3k:friend?key=<node key, base32 lower case, no padding>&s=<secret, 32 bytes, base32>[&dn=<url-encoded name>]
```

- `key` is the Ed25519 key of the node (§6.1). Its node ID follows from it.
- `secret` is 32 random bytes.
- A node MAY make one link for all its friends, or a link per friend. A link per friend can be
  taken back alone (§4). A node has at most 32 *(initial)* links in use.
- A link works in one direction. For two nodes to find each other, each gives the other a
  link.

**The link is the credential.** A client MUST NOT write the secret to a log or show it in an
error message, and SHOULD mark such a link as private wherever it shows it.

---

## 2. Keys

```
k       = H("ed3k v1 friend", secret ‖ key)                               32 bytes
k_rec   = H("ed3k v1 friend record", k)                                   32 bytes
target  = H("ed3k v1 friend target", k ‖ period(8))[0..16)                period = Unix time / 86400, rounded down, big-endian
```

Everything follows from the link. A storing node can compute none of it.

---

## 3. The record

```protobuf
// in enum Kind (§10.3):
FRIEND = 10;

// in Store (§10.3):
bytes sealed = 9;             // FRIEND: at most 256 B

// in KadAnswer (§10.3):
repeated bytes friends = 15;  // answer to a search of kind FRIEND

message FriendRecord {
  Peer   peer = 1;            // my endpoints, and relays if I am a leaf; `id` empty
  uint64 issued = 2;          // Unix time in seconds
}
```

```
sealed = nonce(12) ‖ ChaCha20(k_rec, nonce, serialised FriendRecord)      RFC 8439 cipher, counter from 0
```

**The storing node** treats the record like a source record (§10.3):

- It is bound to the connection: one record per publishing node under a target, and a new one
  replaces the old.
- It holds at most 8 *(initial)* records under one target and answers `FULL` after that.
- The lifetime is 5 h *(initial)*.
- It checks the size and nothing else. It cannot read the record, and it returns the records
  of a target without saying who stored them.

**The node that wants to be found**

- stores a record for each link in use, under the target of the current day: when it starts,
  when its addresses or relays change, and every 4 h *(initial)*;
- names in `peer` what it would put into a source record: its endpoints, or its relays when it
  is a leaf (§9.4).

**The friend**

1. Searches `FRIEND` at the target of the current day. If nothing is found, it searches the
   day before.
2. Opens every record with `k_rec`. One that does not parse is dropped.
3. Takes the record with the latest `issued` that is not more than 1 h in the future, and
   dials: directly, or through the relays as in §9.4 and §9.5.
4. **The handshake is the proof.** The node at that address has to hold `key` (T1). A record
   is not signed for the same reason a source record is not: no signature can prove an
   address, and dialling does.

- A friend searches when its user wants to reach the node, and otherwise at most once per
  30 min *(initial)*.
- Once connected, the two nodes are ordinary peers. What a friend is allowed, in chat, in
  browsing or in the upload queue, is local policy.

---

## 4. Taking a link back

- A node stops storing under a link. After 5 h the record is gone, and the holder of the link
  finds nothing.
- With one link for everyone, taking it back from one friend means a new link for all others.
- A friend that was dropped still knows the node's key and its last address. It can dial that
  address as long as it is valid; whether the node answers it is local policy.

---

## 5. What it shows

| To whom | What |
|---|---|
| A holder of the link | the node's addresses and relays whenever it is online, and so when it is online |
| A node that stores the record | the node ID and address of the publisher, as for any record, and that it publishes a friend record. Not for whom |
| The same node | the addresses of those who search that target. It can conclude that they know the publisher. The target moves every day, so no node sees this for long |
| Anyone else | nothing. The target cannot be computed without the link |
| Someone who gets the link later | the node's current address. There is no forward secrecy |

**What a bad friend can do.** A holder of the link can store false records under the target.
Each costs the other friends one failed dial, and 8 records per target bound it. With a link
per friend there are no other friends at that target.

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **An open record under the node ID** | any node ID seen anywhere can be reached; no link | anyone can follow a node from address to address for as long as it exists |
| **A target a storing node can check**, made from the node key as for a hidden set ([`hidden-set.md`](hidden-set.md) §2) | one record per target, no false records | every peer the node ever connected to knows its key, so each could watch the place |
| **A signed record** | a false record is seen without dialling | the handshake proves more than a signature could, and a signature in a sealed record guards only against friends |
| **A fixed target** | nothing to store again at midnight | the same nodes see who looks for whom, for years |
| **Asking common friends** for the address | no Kad record | needs a friend that is online and knows both; this is how a network of friends works, and ed3k is not one |
| **Being in the routing table is enough** | nothing to add | only for a `ROUTER` whose address did not change |

---

## 7. Open points

- A list of friends is not part of the protocol: there is no message to ask for friendship
  and none to tell that one is online.
- Whether a node should say in `Hello` that the peer may ask it for a link.
- A friend that is a leaf behind a symmetric NAT can be reached only as far as §9.5 reaches
  it.
- All numbers are first values.
