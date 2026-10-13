# Extension: IP diversity limits (`ip-diversity`)

**Status:** draft 0.2 (2026-10-13). Not implemented. **Required in version 1.**
**Extends:** [`Protocol.md`](../Protocol.md) §6.3, §10.2 and §10.4. Section numbers (§) refer to that document.
**Capability:** none. Each node applies the limits to what it accepts; nothing on the wire changes.

The routing ID of §6.3 ties a node's position to its address, but an address block still buys
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
  (§10.2); each is counted in its own family.
- Private, loopback, link-local and unique-local addresses are **exempt** from every limit, as
  they are from the routing ID check (§6.3). LAN and test networks work unchanged.
- Addresses that §8.2 rejects (Teredo, 6to4, documentation, multicast) never become contacts, so
  they need no group.

## 2. The limits

All numbers are *(initial)* and are repeated in Appendix A of the main document.

| Where | host | site | block |
|---|---|---|---|
| One bucket (§2.1) | 1 | 2 | 4 |
| Whole routing table (§2.1) | 1 | 10 | 20 |
| One `KadAnswer` (§2.2) | 1 | 2 | 4 |
| One lookup, nodes queried (§2.2) | 1 | 2 | 4 |
| Closest set (§2.3) | as the lookup | as the lookup | as the lookup |
| Failed or pending source dials per file and hour (§2.5) | 2 | 4 | 8 |

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
- **Identity churn.** A host group is admitted with a new node ID at most once per hour. A
  contact whose host group was in the table under a different node ID less than an hour ago is
  not inserted. This slows an address from walking through the 8 positions that the choice of
  key gives it (§6.3). The contact is refused, not banned (§5).
- These limits raise the price of an attack on the **responsibility rule** (§10.3) but do not
  close it. A node is responsible while it knows fewer than K closer verified contacts. The
  contacts closer to a key than the node span several buckets, and the table holds up to 20
  per block, so one block with enough sites can still place K contacts next to a key in an
  honest node's table and make it refuse the payload; without the limits one site could.

### 2.2 Lookup

A requester applies two filters while it runs a lookup (§10.4).

1. **Per answer.** From the contacts in one `KadAnswer`, in the order sent (closest first), it
   keeps a contact only while the answer's limits hold, and drops the rest. A contact in the
   answering node's own host group is dropped too. One answer can therefore add at most 4
   contacts of one block to the candidates.
2. **Per lookup.** It does not query a candidate when the nodes already queried in this lookup
   reach a limit for one of the candidate's groups. The candidate stays in the set and may be
   queried later if an earlier node of that group failed to connect.

A candidate that was skipped does not count towards termination. It is neither "answered" nor
"closer and unasked".

### 2.3 Closest set

The closest set is the R verified, responsible nodes that a lookup ends with: the nodes a
`store` is sent to and whose `search` results are merged (R = 4 and 8, §10.4).

Its members were queried, so the per-lookup limits of §2.2 already bound it: at most 1 per
host, 2 per site and 4 per block. No further rule applies, and the requester takes the R closest
of the nodes it queried. On a small network the honest nodes near a key may well share a
provider; the 4 per block of §2.2 leave room for that, and a lookup that cannot reach R nodes
within the limits ends with fewer (`Protocol.md` §10.4).

### 2.4 Stored records

A node that stores records limits publishers per target. This refines "flood limits are per
source address" in §10.3.

| Record | host | site | block |
|---|---|---|---|
| `SOURCE` records per file | 8 | 32 | — |
| `KEYWORD` and `NOTE`: publishers counted per entry | 1 | 1 | — |

- `SOURCE`: several nodes behind one carrier-grade NAT address are distinct, honest sources, so
  the host limit is above 1. Each record still belongs to one authenticated node ID (§10.3). A
  record beyond a limit is answered `FULL`.
- `KEYWORD` and `NOTE`: records are accepted from any publisher within the general flood limits,
  but when an entry's publishers are counted (popularity, ranking, or a rating average), one
  site counts once. A network that publishes many entries gains nothing per entry by doing so.

### 2.5 Dialling sources

Source records are not signed (§10.3). A storing node, or an index node, can return records that
point at an address which never published them, and every searcher of that file would then send
a connection attempt there. The handshake exposes the false record, but only after the attempt
has been sent. A node therefore limits the attempts that fail.

For the sources of one file, a node allows at most the following number of connection attempts
that **failed or are still pending**, per hour:

| | host | site | block |
|---|---|---|---|
| Failed or pending dials per file and hour | 2 | 4 | 8 |

- An attempt that completes its handshake with the expected node ID does not count. A group of
  honest sources behind one address or in one network is therefore dialled without limit.
- Sources that exceed a limit are kept and tried in a later hour.
- The limit is per file, so one answer cannot use up a node's attempts for other downloads.

A victim address then receives at most 2 attempts per hour from each searcher, whatever the
number of false records.

## 3. What a requester may still do

- Connect to, download from and upload to any number of peers in one group. The limits concern
  Kad contacts, Kad records and failed source dials only.
- Answer requests from any address. A node that is over a limit is not inserted or queried; its
  own requests are served as usual.
- Query bootstrap contacts beyond the lookup limits for the first lookup, when the table is
  empty. They are inserted into the table under the limits like any contact.

## 4. Comparison

### 4.1 BitTorrent DHT

The BEP index has no proposal for IP limits. BEP 42 restricts node IDs (the model for §6.3),
and BEP 45 only advises a multi-homed node to use one address per /64 because "other nodes might
treat IPv6 /64 subnets as a single entity". The limits in use are an implementation matter;
libtorrent's are the de-facto reference.

| | libtorrent | This extension |
|---|---|---|
| One address in the table | 1 (`dht_restrict_routing_ips`) | 1 per host group |
| Same bucket | 1 per IPv4 /24, 1 per IPv6 /64 | 2 per site, 4 per block |
| Whole table, per subnet | no limit | 10 per site, 20 per block |
| One lookup | 1 per /24, 1 per /64 (`dht_restrict_search_ips`) | 2 per site, 4 per block |
| Larger IPv6 groups | none: a /56 is 256 groups, a /48 is 65,536 | /48 and /32 |
| Larger IPv4 groups | none | /16 |
| Storage targets | no rule beyond the lookup filter | one per block preferred |
| ID tied to address | BEP 42, off by default (`dht_enforce_node_id`) | §6.3, always on |

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
| Storage targets | none | closest-set preference |
| Keyword trust | 10 points per /24, shared among all entries that /24 published | one site counts once per entry |
| One node per address (SafeKad) | a second node on a tracked address is rejected | host limit |
| ID change (SafeKad) | a verified node changing ID within 1 h bans its address for up to 24 h | refused for 1 h, no ban |
| Unverified ID change against a verified record (SafeKad) | rejected | cannot occur: every contact is verified by its handshake (§6.3) |
| Unresponsive nodes (SafeKad) | remembered 5 min | contact dropped after two failed connections (§10.2) |
| LAN addresses | exempt | exempt |

The numbers for address and site are taken over unchanged, because they have years of use behind
them. The differences come from what ed3k has that eMule Kad lacks: the node ID is proven by the
handshake, so the ID-spoofing cases SafeKad guards against do not arise, and IPv6 needs tiers of
its own.

## 5. Alternatives considered

- **Ban instead of refuse.** Rejected. A ban by address also hits the other users of a
  carrier-grade NAT address or of a /64, and a forged-source trigger would make it a tool for
  denial of service. Refusal has the same effect on the routing table with no victim.
  Connection floods are a different matter and do carry a ban, counted on completed
  handshakes only (§4.6).
- **One IPv6 tier (/64).** Rejected, see §4.1.
- **/56 instead of /48 as the IPv6 site.** A /48 customer would count as 256 sites. /48 is the
  largest prefix a single customer normally holds.
- **Hard limit in the closest set.** Rejected, see §2.3.
- **Limits by autonomous system.** The best grouping in principle, since it follows ownership
  and not prefix length. It needs a routing database that every implementation ships and
  updates, and nodes with different database versions would disagree.
- **Looser limits while the table is small.** Fills slots on a young network that strict
  limits leave empty. Left out for simplicity (§8). Done without a re-check it is harmful:
  whatever was let in early keeps its slots for as long as it answers, so the loose limits of
  the first minutes become the limits for good.
- **Disjoint lookup paths** (S/Kademlia). Complements these limits: d independent paths survive
  when fewer than d are captured. At about 2.5 KB per queried node over QUIC (§10.5) it
  multiplies the cost of every lookup by d. Left for the datagram binding.

## 6. Small networks

On a young network the honest nodes cluster in a few providers, and hard limits can leave table
slots empty. The block limits are loose for that reason (4 of 10 slots in a bucket), and the
closest set never shrinks (§2.3). Whether the limits should be looser while a table is small
is an open point (§8).

## 7. What an attacker still needs

Costs with the limits in force, K = 10 and R = 8:

| Goal | IPv4 | IPv6 |
|---|---|---|
| Fill one bucket of a victim | addresses in 5 /24s that span 3 /16s | addresses in 5 /48s that span 3 /32s |
| Hold the whole closest set of a key | 8 addresses in 8 /16s, each with a routing prefix near the key | 8 /32s, each with a routing prefix near the key |
| More than 20 table entries on one victim | a second /16 | a second /32 |

- The IPv6 column is the weak one. A /29, the usual first allocation from a regional registry,
  contains 8 /32s. Each /32 reaches 16,384 routing prefixes, so most of the 8 will have one
  near a given key on a network of 100,000 nodes. One /29 can then take most of a closest set.
- The limits remove the cheapest cases: a home /56 or /48, a single /32, a rented /24.
- What remains is addressed by the key puzzle (§6.3) and by disjoint paths (§5), both reserved.

## 8. Open points

- All numbers are untested against a real address distribution. The block limits in particular
  should be checked against the share of the largest provider once there is a network to measure.
- Whether the IPv6 block should be /29 instead of /32, which would bind the usual registry
  allocation at the price of grouping more unrelated customers.
- Whether the limits of a bucket should be looser while a table is small (§6). It was drafted
  and left out to keep the protocol simple: it needs a boundary, a re-check when the table
  crosses it, and a band around it so that a table does not take contacts out and insert them
  again in turns. §2.1 already says what any such rule must do.
- Whether the routing-ID mask for IPv6 should end at /32 (§6.3 of the main document). It was
  left at /48; with it, a /32 would reach 8 routing prefixes instead of 16,384.
