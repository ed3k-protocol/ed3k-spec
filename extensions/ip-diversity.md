# Extension: IP diversity limits (`ip-diversity`)

**Status:** draft 0.2 (2026-10-13). Not implemented. **Required in version 1.**
**Extends:** [`Protocol.md`](../Protocol.md) §6.3, §10.2 and §10.4. Section numbers (§) refer to this document unless a document is named.
**Capability:** none. Each node applies the limits to what it accepts; nothing on the wire changes.

The routing ID of `Protocol.md` §6.3 ties a node's position to its address, but an address block still buys
positions: an IPv4 /16 or an IPv6 /29 reaches 131,072 of the 2²¹ prefixes, enough to sit next to
a chosen key. This extension limits how many contacts from one network a node accepts, wherever
a set of contacts is built: in the routing table, in a lookup, in the set of nodes a payload is
stored on, and among the publishers of a record.

The limits do not make an attack impossible. They turn "one allocation" into "several
unrelated allocations" (§7).

## 1. Address groups

Every public address belongs to three groups. A limit counts contacts per group.

| Group | IPv4 | IPv6 | Stands for |
|---|---|---|---|
| **host** | /32 (the address) | /64 | one machine or one home link |
| **site** | /24 | /48 | one customer or one small network |
| **block** | /16 | /32 | one provider allocation |

- Groups are counted **per address family**. A dual-stack node has two routing-table entries
  (`Protocol.md` §10.2); each is counted in its own family.
- **Private** addresses are those `Protocol.md` §8.2 (8) lists. In a named network (`net_id` not
  empty, `Protocol.md` §4.3) they are **exempt** from every limit, as they are from the routing
  ID check (`Protocol.md` §6.3). On the public network they are not exempt; they never become
  contacts there, so no limit meets them. `100.64.0.0/10` is neither private nor global unicast
  and is exempt nowhere.
- A LAN peer ([`lan-discovery.md`](lan-discovery.md)) is not a Kad contact and stores no
  record. No limit of this document counts it, in any network.
- Addresses that `Protocol.md` §8.2 rejects (Teredo, 6to4, documentation, multicast) never become contacts, so
  they need no group.

## 2. The limits

All numbers are *(initial)* and are repeated in Appendix A of the main document.

| Where | host | site | block |
|---|---|---|---|
| One bucket (§2.1) | 1 | 2 | 4 |
| Whole routing table (§2.1) | 1 | 10 | 20 |
| One `KadAnswer` (§2.2) | 1 | 2 | 4 |
| One lookup, counted over its closest set (§2.2) | 1 | 2 | R/2: 2 for a search, 4 for a store |
| Closest set (§2.3) | as the lookup | as the lookup | as the lookup |
| Failed or pending source dials per file and hour (§2.5) | 2 | 4 | 8 |
| Failed or pending source dials per hour, over all files (§2.5) | 8 | — | — |

### 2.1 Routing table

A node MUST NOT insert a contact that would exceed a bucket or table limit. When a contact
replaces another, the contact being replaced is taken out of the counts first.

- The table limits exist because the bucket limits alone allow 2 per site in *every* bucket.
- **The limits hold for the contacts that are in the table, not only for the one that
  arrives.** While the limits stay as they are, checking each insertion is enough. Whenever
  they become stricter (new numbers by a network upgrade, or by any later rule that changes
  them with the size of the table), the node checks the whole table again and takes out what
  no longer fits. Insertion then uses the stricter of the limits now and the limits the table
  was last checked against, so that what was taken out does not come back.
- A working contact is never taken out to make room for a new one. It is taken out only when
  the table itself is over a limit.
- **Identity churn.** A host group whose contact was removed from the table is not admitted
  under another node ID for 1 h after the removal. The same node ID is admitted again at once.
  A node remembers each removed host group, with its node ID, for 1 h. While a contact is in
  the table, the host limit already keeps a second node ID of its host group out. This slows
  an address from walking through the 8 positions that the choice of key gives it
  (`Protocol.md` §6.3). The contact is refused, not banned (§5).
- These limits raise the price of an attack on the **responsibility rule** (`Protocol.md` §10.3) but do not
  close it. A node is responsible while it knows fewer than K closer verified contacts. The
  contacts closer to a key than the node span several buckets, and the table holds up to 20
  per block, so one block with enough sites can still place K contacts next to a key in an
  honest node's table and make it refuse the payload; without the limits one site could.

### 2.2 Lookup

A requester applies two filters while it runs a lookup (`Protocol.md` §10.4).

1. **Per answer.** From the contacts in one `KadAnswer`, in the order sent (closest first), it
   keeps a contact only while the answer's limits hold, and drops the rest. A contact in the
   answering node's own host group is dropped too. One answer can therefore add at most 4
   contacts of one block to the candidates.
2. **Per lookup.** The limits are 1 per host, 2 per site and R/2 per block: 2 per block for a
   search (R = 4) and 4 for a store (R = 8). A lookup without an operation uses those of a
   search. They are counted over the closest set as it stands (§2.3), not over every node
   queried: the requester does not query a candidate when the members of that set that are
   closer to the target than the candidate already reach a limit for one of the candidate's
   groups. A node that was queried and has not answered yet counts as a member for this test.
   The candidate stays among the candidates and is queried later if such a node fails.

A waypoint far from the target therefore uses up nothing. It leaves the closest set as closer
nodes answer, and the closest nodes of its groups are still queried.

A candidate that was skipped does not count towards termination. It is neither "answered" nor
"closer and unasked", and it does not count towards the minimum of answers M (`Protocol.md`
§10.4).

### 2.3 Closest set

The closest set is the R closest nodes that the lookup queried and that answered, whatever
their `responsible`, within the per-lookup limits of §2.2. R = 4 for a search and 8 for a store
(`Protocol.md` §10.4). This is the one definition; the set exists from the first answer and
changes while the lookup runs.

- The nodes that answered are taken closest first. A node is left out when the nodes taken
  before it reach a limit for one of its groups. The set therefore holds at most 1 node per
  host, 2 per site and R/2 per block, and always the closest ones of a group.
- No further rule applies. There is no preference for one node per block.
- A `search` merges the results of the members that said `responsible`. A `store` is judged
  over the members that said `responsible` (`Protocol.md` §10.3, §10.4).
- A lookup that cannot reach R nodes within the limits ends with fewer.

A full closest set spans at least 2 blocks: 2 sites for a search, 4 sites for a store. On a
small network the honest nodes near a key may well share a provider; half of the set per block
leaves room for that.

### 2.4 Stored records

A node that stores records limits publishers per target. This refines the request limits per
source address of `Protocol.md` §10.3.

| Record | host | site | block |
|---|---|---|---|
| `SOURCE` records per file | 8 | 32 | — |
| `ANON` records per target | 4 | 8 | — |
| `KEYWORD` and `NOTE`: publishers counted per entry | 1 | 1 | — |
| `FRIEND` records per target ([`friend.md`](friend.md) §4) | 8 | 16 | — |
| `MEMBER` sets per file ([`file-set.md`](file-set.md) §6) | — | 16 | — |
| `ALIAS` roots per target ([`alias.md`](alias.md) §3) | — | 8 | — |
| `PART` records per target ([`multi-author-set.md`](multi-author-set.md) §3.4) | 8 | 32 | — |

- `SOURCE`: several nodes behind one carrier-grade NAT address are distinct, honest sources, so
  the host limit is above 1. Each record still belongs to one authenticated node ID (`Protocol.md` §10.3).
- `ANON`: the record names no address, so it is counted on the address the `Store` came from
  ([`hops.md`](hops.md) §4.2).
- A record beyond a host or site limit is answered `REJECTED`, not `FULL`. `FULL` says that the
  target has no room (`Protocol.md` §10.8). An address limit says nothing about the target: the
  storing node does not set `spill` for it, and the publisher does not start a lookup of its
  own because of it (`Protocol.md` §10.4).
- **Address limits come first.** A storing node checks the limits of this table, and those per
  publisher key, before the limits of the target: a record over one is `REJECTED`, whether or
  not the target has room.
- Limits per publisher key, not per address, are in the documents of the kinds: a `KEYWORD`
  target takes at most 1,000 *(initial)* files of one publisher key and 2,000 of one publisher
  site (`Protocol.md` §10.8); a `MEMBER` target at most 4 sets of one publisher key
  ([`file-set.md`](file-set.md) §6); an `ALIAS` target at most 2 roots of one publisher key
  ([`alias.md`](alias.md) §3); a `PART` target one record per author key
  ([`multi-author-set.md`](multi-author-set.md) §3.4).
- `KEYWORD` and `NOTE`: beyond the limits per key, host and site of `Protocol.md` §10.8, records
  are accepted within the general flood limits; when an entry's publishers are counted (popularity, ranking, or a rating average), one
  site counts once. A network that publishes many entries gains nothing per entry by doing so.

### 2.5 Dialling sources

Source records are not signed (`Protocol.md` §10.3). A storing node, or an index node, can return records that
point at an address which never published them, and every searcher of that file would then send
a connection attempt there. The handshake exposes the false record, but only after the attempt
has been sent. A node therefore limits the attempts that fail.

For the sources of one file, a node allows at most the following number of connection attempts
that **failed or are still pending**, per hour:

| | host | site | block |
|---|---|---|---|
| Failed or pending dials per file and hour | 2 | 4 | 8 |
| Failed or pending dials per hour, over all files | 8 | — | — |

- An attempt that completes its handshake with the expected node ID does not count. A group of
  honest sources behind one address or in one network is therefore dialled without limit.
- The limit counts every address dialled on behalf of the file, each under its own groups: the
  endpoints of a source in both families, every relay taken from a source record or from a
  source exchange, and every gate taken from a `GATE` record ([`hops.md`](hops.md) §4.3), also
  when a last hop dials it.
- Sources that exceed a limit are kept and tried in a later hour.
- The limit is per file, so one answer cannot use up a node's attempts for other downloads.
- **Over all files** the host limit is 8 per hour. A source beyond it is kept and tried in a
  later hour, as above.

A victim address then receives at most 2 attempts per hour from each searcher and file, and 8
from each searcher over all files, whatever the number of false records and whether they name
it as an endpoint, as a relay or as a gate.

## 3. What a requester may still do

- Connect to, download from and upload to any number of peers in one group. The limits concern
  Kad contacts, Kad records and failed source dials only.
- Answer requests from any address. A node that is over a limit is not inserted or queried; its
  own requests are served as usual.
- Query candidates beyond the lookup limits of §2.2 in the first lookup
  ([`bootstrap.md`](bootstrap.md) §4), and only while fewer than α = 3 candidates remain that
  the limits allow it to query. They are inserted into the table under the limits like any
  contact.

## 4. Comparison

### 4.1 BitTorrent DHT

The BEP index has no proposal for IP limits. BEP 42 restricts node IDs (the model for `Protocol.md` §6.3),
and BEP 45 only advises a multi-homed node to use one address per /64 because "other nodes might
treat IPv6 /64 subnets as a single entity". The limits in use are an implementation matter;
libtorrent's are the de-facto reference.

| | libtorrent | This extension |
|---|---|---|
| One address in the table | 1 (`dht_restrict_routing_ips`) | 1 per host group |
| Same bucket | 1 per IPv4 /24, 1 per IPv6 /64 | 2 per site, 4 per block |
| Whole table, per subnet | no limit | 10 per site, 20 per block |
| One lookup | 1 per /24, 1 per /64 (`dht_restrict_search_ips`) | 2 per site; 2 per block for a search, 4 for a store |
| Larger IPv6 groups | none: a /56 is 256 groups, a /48 is 65,536 | /48 and /32 |
| Larger IPv4 groups | none | /16 |
| Storage targets | no rule beyond the lookup filter | at most half of them, 4 of 8, in one block (§2.3) |
| ID tied to address | BEP 42, off by default (`dht_enforce_node_id`) | `Protocol.md` §6.3, always on |

libtorrent is stricter inside a bucket and has nothing above /24 and /64. For IPv6 that leaves
the limit almost without effect, since a home delegation already spans hundreds of /64s.

### 4.2 eMule Kad and eMuleQt

eMuleQt enforces the eMule limits in its routing bins and adds the SafeKad node tracker. Both
are IPv4 only.

| Rule | eMule Kad / eMuleQt | This extension |
|---|---|---|
| Per address in the table | 1 | 1 per host group |
| Per /24 in a bucket | 2 | 2 per site |
| Per /24 in the table | 10 | 10 per site |
| Per /16, and all of IPv6 | none | block tier; /64, /48, /32 |
| Contacts in one answer | no duplicate address, 2 per /24 | 1 per host, 2 per site, 4 per block |
| Lookup as a whole | none | per-lookup limit |
| Storage targets | none | at most 2 per site and 4 of 8 per block (§2.3) |
| Keyword trust | 10 points per /24, shared among all entries that /24 published | one site counts once per entry |
| One node per address (SafeKad) | a second node on a tracked address is rejected | host limit |
| ID change (SafeKad) | a verified node changing ID within 1 h bans its address for up to 24 h | refused for 1 h, no ban |
| Unverified ID change against a verified record (SafeKad) | rejected | cannot occur: every contact is verified by its handshake (`Protocol.md` §6.3) |
| Unresponsive nodes (SafeKad) | remembered 5 min | contact dropped after two failed connections (`Protocol.md` §10.2) |
| LAN addresses | exempt | exempt in a named network only (§1) |

The numbers for address and site are taken over unchanged, because they have years of use behind
them. The differences come from what ed3k has that eMule Kad lacks: the node ID is proven by the
handshake, so the ID-spoofing cases SafeKad guards against do not arise, and IPv6 needs tiers of
its own.

## 5. Alternatives considered

- **Ban instead of refuse.** Rejected. A ban by address also hits the other users of a
  carrier-grade NAT address or of a /64, and a forged-source trigger would make it a tool for
  denial of service. Refusal has the same effect on the routing table with no victim.
  Connection floods are a different matter: `Protocol.md` §4.6 bans the node ID, never the
  address, counted on completed handshakes only.
- **One IPv6 tier (/64).** Rejected, see §4.1.
- **/56 instead of /48 as the IPv6 site.** A /48 customer would count as 256 sites. /48 is the
  largest prefix a single customer normally holds.
- **One node per block first in the closest set, more only when the blocks run out.** Not
  taken. It is stricter where blocks are plenty, but it is a preference with a fall-back: where
  blocks are few it allows what the count allows, and which nodes it picks depends on the
  blocks each requester happened to meet. The closest set has one hard count instead, R/2 per
  block, and a lookup that cannot meet it ends with fewer nodes (§2.3).
- **Limits by autonomous system.** The best grouping in principle, since it follows ownership
  and not prefix length. It needs a routing database that every implementation ships and
  updates, and nodes with different database versions would disagree.
- **Looser limits while the table is small.** Fills slots on a young network that strict
  limits leave empty. Left out for simplicity (§8). Done without a re-check it is harmful:
  whatever was let in early keeps its slots for as long as it answers, so the loose limits of
  the first minutes become the limits for good.
- **Disjoint lookup paths** (S/Kademlia). Complements these limits: d independent paths survive
  when fewer than d are captured. At about 2.5 KB per queried node over QUIC (`Protocol.md` §10.5) it
  multiplies the cost of every lookup by d. Left for the datagram binding.

## 6. Small networks

On a young network the honest nodes cluster in a few providers, and hard limits can leave table
slots empty. The block limits are loose for that reason: 4 of 10 slots in a bucket, and half
of a closest set. The closest set can still shrink: where all nodes near a key are in one
block, a search ends with 2 nodes and a store with 4 (§2.3). Whether the limits should be
looser while a table is small is an open point (§8).

## 7. What an attacker still needs

Costs with the limits in force, K = 10, R = 4 for a search and R = 8 for a store:

| Goal | IPv4 | IPv6 |
|---|---|---|
| Fill one bucket of a victim | addresses in 5 /24s that span 3 /16s | addresses in 5 /48s that span 3 /32s |
| Hold the whole closest set of a search (R = 4) | 4 addresses in 2 /24s, one /24 in each of 2 /16s, each address with a routing prefix near the key | 4 /64s in 2 /48s, one /48 in each of 2 /32s, each with a routing prefix near the key |
| Hold the whole closest set of a store (R = 8) | 8 addresses in 4 /24s, two /24s in each of 2 /16s, each address with a routing prefix near the key | 8 /64s in 4 /48s, two /48s in each of 2 /32s, each with a routing prefix near the key |
| More than 20 table entries on one victim | a second /16 | a second /32 |

- The IPv6 column is the weak one. A /29, the usual first allocation from a regional registry,
  contains 8 /32s. Each /32 reaches 16,384 routing prefixes, so most of the 8 will have one
  near a given key on a network of 100,000 nodes. A closest set needs 2 of the 8, so one /29
  can hold a whole one, of a search and of a store.
- Two blocks are enough in both families, because one block may hold half of a set. A stricter
  count would shut out honest nodes that share a provider (§6). One block alone holds at most
  half of a set, and the other half answers.
- The limits remove the cheapest cases: a home /56 or /48, a single /32, a rented /24.
- What remains is addressed by the key puzzle (`Protocol.md` §6.3) and by disjoint paths (§5), both reserved.

## 8. Open points

- All numbers are untested against a real address distribution. The block limits in particular
  should be checked against the share of the largest provider once there is a network to measure.
- Whether the IPv6 block should be /29 instead of /32, which would bind the usual registry
  allocation at the price of grouping more unrelated customers.
- Whether the limits of a bucket should be looser while a table is small (§6). It was drafted
  and left out to keep the protocol simple: it needs a boundary, a re-check when the table
  crosses it, and a band around it so that a table does not take contacts out and insert them
  again in turns. §2.1 already says what any such rule must do.
- Whether the routing-ID mask for IPv6 should end at /32 (`Protocol.md` §6.3). It was
  left at /48; with it, a /32 would reach 8 routing prefixes instead of 16,384.
