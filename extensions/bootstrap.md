# Extension: first contact (`bootstrap`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.2, §10.3 and §13.2. Section numbers (§) refer to that document unless this one is named.
**Capability:** none. One file format and one link. Nothing changes between two nodes.

A node enters Kad with a `KadRequest` for its own routing ID, one per public address (§6.3,
§10.3). For that it needs the address of
at least one node. This extension says where a node without any contact gets addresses from.

- A **node list** is a small signed file with the addresses of some nodes.
- A client fetches lists over HTTPS from **two or more sources run by different people**, and
  only when it has nothing better.
- A node from a list is a candidate to dial. The handshake proves who it is, as for any contact.

The model is the reseed of I2P ([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.1).

---

## 1. Where the first nodes come from

A client tries these in order and stops when its lookups work:

1. **The contacts it saved.** A client SHOULD save up to 200 *(initial)* verified `ROUTER`
   contacts when it stops, and from time to time while it runs.
2. **Nodes in the local network** ([`lan-discovery.md`](lan-discovery.md)). A LAN peer is not
   put into the routing table, but it answers a `KadRequest` with contacts.
3. **`ed3k:node` links** the user opened (§13.2).
4. **eD2K peers that also speak ed3k** (§14), for a client of both networks.
5. **Node lists** (§2 to §4 of this document).

A client fetches node lists by itself only when fewer than 10 *(initial)* nodes from the
sources above completed a handshake, and at most once per 6 h *(initial)*. The 6 h are counted
per first contact, not per source: one first contact fetches from several sources (§3 of this
document), and the next one starts 6 h later at the earliest.

---

## 2. The node list

```
"ed3knod" ‖ 0x01 ‖ NodeFile
```

The file starts with the 8 bytes `65 64 33 6B 6E 6F 64 01`. The rest is one protobuf message:

```protobuf
message NodeFile {
  bytes  body = 1;            // a serialised NodeList
  bytes  key = 2;             // 32 B Ed25519 key of the signer
  repeated Signature sigs = 3;  // over body
}

message NodeList {
  uint64 created = 1;         // Unix time in seconds
  bytes  nodes4 = 2;          // n × 22 B compact nodes (§5.4)
  bytes  nodes6 = 3;          // n × 34 B compact nodes
  string comment = 4;
  bytes  net_tag = 5;         // 16 B: H("ed3k v1 nodes net", net_id)[0..16) of the network the list is for; empty = the public network
}
```

Each signature is computed over

```
"ed3k v1 nodes" ‖ body
```

`Signature` and the rules for `sigs` are those of §10.7.

`net_tag` names the network without giving away `net_id` (§4.3), which is the key input of the
obfuscation and, in a closed network, all that makes a member. A list of the public network
leaves `net_tag` empty. A list of any other network carries

```
net_tag = H("ed3k v1 nodes net", net_id)[0..16)
```

A member computes the same value and recognises its list. Nobody can get `net_id` or the name
back from it.

A reader refuses a list

- that does not start with the 8 bytes, or is larger than 32 KiB *(initial)*,
- whose `key` is not the one configured for the source, or whose signature does not verify
  under it (§3 of this document),
- whose `net_tag` is not that of the client's own network: a list of another network is
  no use, and the public network's lists must not lead into a test network,
- whose `nodes4` is not a multiple of 22 B, or whose `nodes6` is not a multiple of 34 B,
- whose `created` is more than 30 days *(initial)* in the past or more than 1 h in the future.

A list that names more than 200 *(initial)* nodes per family is not refused; the entries after
the 200th of a family are ignored. The 200 are the first 200 entries as written, usable or not.
An entry with port 0, an IPv4-mapped address or an address that is not acceptable in the
client's network (§8.2 (7)) is skipped; the list is not refused for it.

The file SHOULD be named `nodes.ed3knod`.

---

## 3. Sources

A **source** is an HTTPS URL together with the key that signs the lists found there.

- A client ships with its sources. It SHOULD ship at least 4 *(initial)*, run by different
  people in different networks.
- For one first contact a client picks 2 *(initial)* of its sources at random and fetches both.
  It picks another one for each that fails.
- The server certificate is verified as for any HTTPS request. The signature is verified on
  top of that, so a list may be served by a mirror or a cache that the signer does not run.
- A fetch is HTTPS only. The client follows at most 3 *(initial)* redirects, each to an HTTPS
  URL; a redirect to anything else, or a fourth one, ends the fetch as failed.
- The client stops reading at 32 KiB, the largest list of §2 of this document. An answer that
  goes on beyond that is a failed fetch.
- The request carries nothing that names the node: no node ID, no cookie, a plain user agent.
  It follows the user's proxy settings.
- A list from a single source is used when no second source answers. The client then fetches
  from a second source as soon as one answers.

**Adding a source**

```
ed3k:nodes?url=<url-encoded HTTPS URL>&key=<signer key, base32 lower case, no padding>
```

The link is invalid when `url` or `key` is missing or repeated, `key` does not decode to 32
bytes, or `url` is not an absolute HTTPS URL.

A client MUST ask the user before it adds a source from a link, and shows the host name of the
URL when it asks.

**By hand.** A user can open a node list file directly. Opening it is the user's decision to
trust it: the client checks the signature against the key in the file and applies the other
checks of §2 of this document. A client SHOULD be able to write such a file from its own verified contacts,
signed with its node key, so that one user can help another who is cut off from every source.

---

## 4. Using a list

- From each list a client takes at most 50 *(initial)* nodes, chosen at random. One entry per
  IP address is used, whatever its port and node ID; further entries of an IP address are
  dropped before the sample is taken. The sample holds at most 2 *(initial)* nodes of one site
  and 8 of one block ([`ip-diversity.md`](ip-diversity.md) §1); further entries of that site
  or block are skipped while the sample is drawn, and it takes up to 50 from the entries that
  remain.
- **A node from a list is a candidate, not a contact.** The client dials it. The handshake
  proves the node ID (T1) and with it the routing ID (§6.3). Only then is it inserted, under
  the rules of §10.2 and [`ip-diversity.md`](ip-diversity.md). A list gives a node no better
  place in the table.
- At most half of the candidates of a first contact come from one list, as long as a second
  list was received. Here and below, a list counts as received only if it gave at least one
  candidate. A node named by two lists is a candidate of each; it is dialled once.
- When 4 *(initial)* nodes have completed a handshake, the client looks up its own routing ID
  (§6.3, §10.3), once per public address. When more than one list was received, at most half
  of the 4 come from one list: further nodes of that list do not count towards the 4, and the
  client waits for nodes from elsewhere, for at most 30 s *(initial)* from the first
  handshake; after that every node counts. One list then cannot supply every seed of the first
  lookup. From then on
  its table is filled by lookups, and the lists are not used again.
- A node from a list that does not answer is dropped without a second attempt.
- A list belongs to one network ([`Protocol.md`](../Protocol.md) §4.3). A node of another
  network does not answer. Neither the name of a closed network nor its `net_id` is ever part
  of a list; `net_tag` is a hash of `net_id` under a context of its own (§2 of this document).

---

## 5. Making a list

For whoever runs a source:

- A list SHOULD be made again at least once per day.
- It names only nodes that the signer has dialled itself, that set `ROUTER`, and that it has
  seen reachable for 24 h *(initial)* or longer.
- It holds at most 2 nodes of one site and 8 of one block, in the sense of
  [`ip-diversity.md`](ip-diversity.md).
- A source SHOULD give different requesters different samples of the nodes it knows. A node
  named in every list would receive every newcomer.
- It SHOULD give one requester address the same sample for the life of a list. A requester
  that asks again and again would otherwise collect every node the source knows.
- The signing key is not a node key in use. It signs lists and nothing else. A source that
  keeps the key off the server serves the samples above from a pool of lists signed in
  advance.

---

## 6. What an attacker can do

| Attack | What limits it |
|---|---|
| **A source names only its own nodes**, to surround a newcomer | two sources; at most half of the candidates from one; the diversity limits; after the first lookup the table comes from the network |
| **A false list at a real source** (server taken over) | the signature. The signing key need not be on the server |
| **An old list shown again** | `created` and the 30 days |
| **A stolen signing key** | nothing in the protocol. The client's next release removes the source |
| **A list that names a victim's address** many times, to make newcomers send to it | one entry per IP address is used; a newcomer sends one handshake; 50 nodes per list |
| **Blocking the sources** | sources at several hosts; lists by hand (§3 of this document); every other way of §1 of this document |

**What a fetch shows.** The network between client and source sees an HTTPS connection to the
host of the source, and the source sees the client's address. Anyone who knows the sources
therefore knows that the host starts an ed3k client. The obfuscation of §4.2 does not cover
this first step. A user for whom that matters turns the automatic fetch off and uses a list
received by hand, or a proxy.

---

## 7. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Left to each client**: a node file of its own, fetched from the project's web site, as eMule does with `nodes.dat` | nothing to specify | no shared lists between clients; one unsigned file from one place decides what a newcomer sees |
| **Fixed nodes in the client** | no fetch, no HTTPS | the same few nodes carry every newcomer and are easy to block or attack; they go stale between releases |
| **DNS names that resolve to nodes** | one query, cached everywhere | no signature without DNSSEC; no node IDs or ports in an address record; the query is in clear |
| **Another network as meeting point**, for example a record in the BitTorrent DHT | no operator | a second protocol to implement, and its own first contact |
| **One source is enough** | simpler | whoever runs or takes over that source chooses the newcomer's whole first view |

---

## 8. Open points

- Who runs the sources, and how a client decides which signers it ships.
- A second transport binding ([`docs/transport-bindings.md`](../docs/transport-bindings.md))
  needs a way to say which binding a listed node speaks. `NodeList` has no field for it yet;
  `Hello.bindings = 11` and `Peer.bindings = 6` are reserved for it (`Protocol.md` §5.4, §5.5).
- A node whose clock is far off refuses every list. The date of the HTTPS answer could tell it.
- All numbers are first values.
