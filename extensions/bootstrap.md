# Extension: first contact (`bootstrap`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.2, §10.3 and §13.2. § = that document unless this one is named.
**Capability:** none. One file format, one link; nothing changes between two nodes.

A node enters Kad with a `KadRequest` for its own routing ID, one per public address (§6.3,
§10.3), so needs ≥ 1 node address. This extension: where a node with no contact gets them.

- **Node list**: small signed file with addresses of some nodes.
- Fetched over HTTPS from **two or more sources run by different people**, only when nothing
  better exists.
- A listed node is a candidate to dial; handshake proves who it is, as for any contact.

Model: I2P reseed ([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.1).

---

## 1. Where the first nodes come from

Client tries in order, stops when its lookups work:

1. **Saved contacts.** A client SHOULD save up to 200 *(initial)* verified `ROUTER` contacts on
   stop, and at least hourly while running (local policy), so a crash doesn't lose them.
2. **Local-network nodes** ([`lan-discovery.md`](lan-discovery.md)): not put in the routing
   table, but answer a `KadRequest` with contacts.
3. **`ed3k:node` links** the user opened (§13.2).
4. **eD2K peers that also speak ed3k** (§14), for a client of both networks, and **index nodes
   advertised by eD2K servers** in ident or description
   ([`docs/ed2k-tags.md`](../docs/ed2k-tags.md) section 2).
5. **Node lists** (§2 to §4 of this document).

Client auto-fetches node lists only when < 10 *(initial)* nodes from the sources above
completed a handshake, at most once per 6 h *(initial)*. Judged once every candidate of those
sources was dialled, or 60 s *(initial)* after start, whichever first. Saved contacts count once
loaded (also if none readable); LAN discovery once its first round ended: 2 s *(initial)* after
its first announce on every interface it listens on, or at once with no such interface or
extension off. Until then only the 60 s decide. 6 h counted per first contact, not per source:
one first contact fetches from several sources (§3 of this document); next one ≥ 6 h later.

---

## 2. The node list

```
"ed3knod" ‖ 0x01 ‖ NodeFile
```

File starts with the 8 bytes `65 64 33 6B 6E 6F 64 01`; rest is one protobuf message:

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

`Signature` and `sigs` rules: §10.7.

`net_tag` names the network w/o revealing `net_id` (§4.3), the obfuscation key input and, in a
closed network, all that makes a member. Public network: `net_tag` empty. Any other network:

```
net_tag = H("ed3k v1 nodes net", net_id)[0..16)
```

A member computes it and recognises its list; nobody can recover `net_id` or name from it.

Reader refuses a list

- not starting with the 8 bytes, or larger than 32 KiB *(initial)*,
- whose `key` isn't the one configured for the source, or whose signature doesn't verify under
  it (§3 of this document),
- whose `net_tag` isn't the client's own network's (another network's list is no use; public
  network lists must not lead into a test network),
- whose `nodes4` isn't a multiple of 22 B, or `nodes6` not a multiple of 34 B,
- whose `created` is > 30 days *(initial)* past or > 1 h future.

A list naming > 200 *(initial)* nodes per family isn't refused; entries after a family's 200th
are ignored. The 200 = first 200 entries as written, usable or not. An entry with port 0, an
IPv4-mapped address, or an address not acceptable in the client's network (§8.2 (7), and (8) in
a named network) is skipped; list not refused for it.

File SHOULD be named `nodes.ed3knod`.

---

## 3. Sources

**Source** = HTTPS URL plus the key signing the lists found there.

- Client ships with its sources; it SHOULD ship at least 4 *(initial)*, run by different people
  in different networks.
- Per first contact client picks 2 *(initial)* sources at random and fetches both; picks another
  for each that fails.
- Server certificate verified as for any HTTPS request, signature on top, so a mirror or cache
  the signer doesn't run may serve a list.
- HTTPS only. At most 3 *(initial)* redirects, each to an HTTPS URL; a redirect to anything
  else, or a fourth, fails the fetch.
- Client stops reading at 32 KiB, the largest list of §2 of this document; an answer going
  beyond is a failed fetch.
- Request carries nothing naming the node: no node ID, no cookie, a plain user agent. Follows
  user's proxy settings.
- A single-source list is used when no second source answers; client then fetches a second as
  soon as one answers.

**Adding a source**

```
ed3k:nodes?url=<url-encoded HTTPS URL>&key=<signer key, base32 lower case, no padding>
```

Link invalid when `url` or `key` missing or repeated, `key` doesn't decode to 32 bytes, or `url`
isn't an absolute HTTPS URL.

A client MUST ask the user before adding a source from a link, showing the URL's host name.

**By hand.** User may open a node list file directly; doing so = decision to trust it: client checks the
signature against the key in the file, plus the other §2 checks of this document. A client
SHOULD be able to write such a file from its own verified contacts, signed with its node key, so
one user can help another cut off from every source.

---

## 4. Using a list

- Per list ≤ 50 *(initial)* random nodes. One entry per IP address, whatever port and node ID;
  further entries of an IP dropped before sampling. Sample ≤ 2 *(initial)* nodes per site, 8 per
  block ([`ip-diversity.md`](ip-diversity.md) §1); further entries of that site/block skipped
  while drawing; up to 50 taken from the rest.
- **Listed node = candidate, not contact.** Client dials it; handshake proves node ID (T1) and
  thus routing ID (§6.3). Only then inserted, under §10.2 and
  [`ip-diversity.md`](ip-diversity.md). A list gives no better place in the table.
- ≤ half the candidates of a first contact from one list, while a second list was received.
  Here and below, a list counts as received only if it gave ≥ 1 candidate. A node in two lists
  is a candidate of each, dialled once; toward the half of the 4 it counts for the one with
  fewer nodes counted so far.
- Once 4 *(initial)* nodes completed a handshake, client looks up its own routing ID (§6.3,
  §10.3), once per public address. With > 1 list received, ≤ half of the 4 from one list:
  further nodes of it don't count toward the 4, client waits for nodes from elsewhere ≤ 30 s
  *(initial)* from first handshake, then every node counts. All candidates dialled and < 4
  count → look up with those that completed a handshake; none → this first contact failed (§1
  of this document). So one list can't supply every seed of the first lookup. Afterwards table
  fills by lookups; lists not used again.
- A listed node that doesn't answer is dropped, no second attempt.
- List belongs to one network ([`Protocol.md`](../Protocol.md) §4.3); a node of another doesn't
  answer. A closed network's name or `net_id` is never in a list; `net_tag` hashes `net_id` under
  its own context (§2 of this document).

---

## 5. Making a list

For source operators:

- A list SHOULD be remade at least daily.
- Names only nodes the signer dialled itself, that set `ROUTER`, and that it saw reachable for
  24 h *(initial)* or longer.
- At most 2 nodes per site, 8 per block, as in [`ip-diversity.md`](ip-diversity.md).
- A source SHOULD give different requesters different samples of the nodes it knows; a node in
  every list would get every newcomer.
- It SHOULD give one requester address the same sample for a list's life; else a requester asking
  repeatedly collects every node the source knows.
- Signing key isn't a node key in use; signs lists only. A source keeping it off the server
  serves the samples above from a pool of pre-signed lists.

---

## 6. What an attacker can do

| Attack | What limits it |
|---|---|
| **Source names only its own nodes**, to surround a newcomer | two sources; ≤ half the candidates from one; diversity limits; after first lookup the table comes from the network |
| **False list at a real source** (server taken over) | the signature; signing key need not be on the server |
| **Old list replayed** | `created` and the 30 days |
| **Stolen signing key** | nothing in the protocol; client's next release removes the source |
| **List naming a victim's address** many times, to make newcomers send to it | one entry per IP address used; newcomer sends one handshake; 50 nodes per list |
| **Blocking the sources** | sources at several hosts; lists by hand (§3 of this document); every other way of §1 of this document |

**What a fetch shows.** Network between client and source sees an HTTPS connection to the
source host; source sees client address. So anyone knowing the sources knows the host starts an
ed3k client; §4 obfuscation doesn't cover this step. A user who cares disables auto-fetch and
uses a list received by hand, or a proxy.

---

## 7. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Left to each client**: own node file from the project web site, as eMule does with `nodes.dat` | nothing to specify | no shared lists between clients; one unsigned file from one place decides what a newcomer sees |
| **Fixed nodes in the client** | no fetch, no HTTPS | same few nodes carry every newcomer, easy to block or attack; go stale between releases |
| **DNS names resolving to nodes** | one query, cached everywhere | no signature w/o DNSSEC; no node IDs or ports in an address record; query in clear |
| **Another network as meeting point**, e.g. a record in the BitTorrent DHT | no operator | second protocol to implement, with its own first contact |
| **One source is enough** | simpler | whoever runs or takes over that source picks the newcomer's whole first view |

---

## 8. Open points

- Who runs the sources, and how a client decides which signers it ships.
- A second transport binding ([`docs/transport-bindings.md`](../docs/transport-bindings.md))
  needs a way to say which binding a listed node speaks. `NodeList` has no field yet;
  `Hello.bindings = 11` and `Peer.bindings = 6` are reserved for it (`Protocol.md` §5.4, §5.5).
- A node with a clock far off refuses every list; the HTTPS answer's date could tell it.
- All numbers are first values.
