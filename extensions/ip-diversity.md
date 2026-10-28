# Extension: IP diversity limits (`ip-diversity`)

**Status:** draft 0.2 (2026-10-13). Not implemented. **Required in version 1.**
**Extends:** [`Protocol.md`](../Protocol.md) §6.3, §10.2, §10.3 and §10.4. § = this document unless a document is named.
**Capability:** none. Each node applies the limits to what it accepts; nothing on the wire changes.

The `Protocol.md` §6.3 routing ID ties a node's position to its address, but an address block
still buys positions: an IPv4 /16 or IPv6 /29 reaches 131,072 of the 2²¹ prefixes, enough to sit
by a chosen key. So a node limits contacts accepted from one network wherever a contact set is
built: routing table, lookup, nodes a payload is stored on, a record's publishers.

Attacks stay possible but need "several unrelated allocations" instead of "one allocation" (§7).

## 1. Address groups

Every public address belongs to three groups; a limit counts contacts per group.

| Group | IPv4 | IPv6 | Stands for |
|---|---|---|---|
| **host** | /32 (the address) | /64 | one machine or home link |
| **site** | /24 | /48 | one customer or small network |
| **block** | /16 | /32 | one provider allocation |

- Counted **per address family**: a dual-stack node's two routing-table entries
  (`Protocol.md` §10.2) count each in its own family.
- **Private** addresses = those `Protocol.md` §8.2 (8) lists. In a named network (`net_id` not
  empty, `Protocol.md` §4.3) **exempt** from every limit, as from the routing ID check
  (`Protocol.md` §6.3). Public network: not exempt, but never contacts there, so no limit meets
  them. `100.64.0.0/10` is neither private nor global unicast; exempt nowhere.
- Where a private address is counted per host address in a named network, the host is the IPv4
  /32 or IPv6 /128, not the /64 (`Protocol.md` §9.2).
- LAN peer ([`lan-discovery.md`](lan-discovery.md)): no Kad contact, stores no record; no limit
  here counts it, in any network.
- Addresses `Protocol.md` §8.2 rejects (Teredo, 6to4, documentation, multicast): never contacts,
  no group needed.

## 2. The limits

All numbers *(initial)*, repeated in Appendix A of the main document.

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

A node MUST NOT insert a contact that would exceed a bucket or table limit. A replaced contact
leaves the counts first.

- Table limits exist because bucket limits alone allow 2 per site in *every* bucket.
- **Limits hold for the contacts in the table, not only the arriving one.** With unchanged
  limits, checking each insertion suffices. When they get stricter (network upgrade numbers, or
  any later rule varying them with table size), node re-checks the whole table, removes what no
  longer fits. Insertion then uses the stricter of current limits and those last checked
  against, so removed contacts don't come back.
- A working contact is removed only when the table is over a limit, never to make room.
- **Identity churn.** A host group whose contact was removed from the table for whatever reason
  (failed attempts, a `Hello` without `ROUTER` and `nat = 1`, `Protocol.md` §10.2, or a stricter
  limit) is not admitted under another node ID for 1 h after removal. The same node ID is admitted again at once.
  Removed host groups remembered with node ID for 1 h. While a contact is in the table,
  the host limit already keeps out a second node ID of its group. Slows an address walking the
  8 positions its key choice gives (`Protocol.md` §6.3). Refused, not banned (§5).
- Limits raise the price of attacking the **responsibility rule** (`Protocol.md` §10.3), don't close it. A node is responsible while it knows < K closer verified contacts. Contacts closer
  to a key than the node span several buckets, and the table holds up to 20 per block, so one
  block with enough sites can still put K contacts by a key in an honest node's table, making it
  refuse the payload; without limits one site could.

### 2.2 Lookup

Two filters during a lookup (`Protocol.md` §10.4):

1. **Per answer.** Of one `KadAnswer`'s contacts, in sent order (closest first), keep each only
   while the answer's limits hold; drop the rest, and any in the answerer's own host group. So
   one answer adds ≤ 4 contacts of one block to the candidates.
2. **Per lookup.** Limits: 1 per host, 2 per site, R/2 per block: 2 per block for a search
   (R = 4), 4 for a store (R = 8). A lookup without an operation uses a search's. Counted over
   the current closest set (§2.3), not every node queried: a candidate isn't queried when set
   members closer to the target than it already reach a limit for one of its groups. A queried,
   not-yet-answered node counts as a member for this test. The candidate stays, queried later if
   such a node fails.

So a waypoint far from the target uses up nothing: it leaves the closest set as closer nodes
answer; its groups' closest nodes are still queried.

A skipped candidate doesn't count towards termination: neither "answered" nor "closer and
unasked", and not towards the minimum of answers M (`Protocol.md` §10.4).

### 2.3 Closest set

Closest set = R closest queried nodes that answered, whatever their `responsible`, within the
per-lookup limits of §2.2. R = 4 for a search, 8 for a store
(`Protocol.md` §10.4). The one definition; exists from the first answer, changes as the lookup runs.

- Answered nodes taken closest first; a node is left out when those taken before reach a limit
  for one of its groups. So ≤ 1 per host, 2 per site, R/2 per block, always a group's closest.
- No further rule; no preference for one node per block.
- A `search` merges results of members saying `responsible`; a `store` is judged over them
  (`Protocol.md` §10.3, §10.4).
- A lookup unable to reach R nodes within the limits ends with fewer.

A full closest set spans ≥ 2 blocks: 2 sites for a search, 4 for a store. On a small network
honest nodes near a key may share a provider; half the set per block allows that.

### 2.4 Stored records

Storing nodes limit publishers per target, refining the per-source-address request limits of
`Protocol.md` §10.3.

| Record | host | site | block |
|---|---|---|---|
| `SOURCE` records per file, `LIVE` and `SUB` records per target | 8 | 32 | — |
| `ANON` records per target | 4 | 8 | — |
| `NOTE` records per file target (`Protocol.md` §10.8) | 4 | 8 | — |
| `KEYWORD` and `NOTE`: publishers counted per entry | 1 | 1 | — |
| `FRIEND` records per target ([`friend.md`](friend.md) §4) | 8 | 16 | — |
| `MEMBER` sets per file ([`file-set.md`](file-set.md) §6) | — | 16 | — |
| `ALIAS` roots per target ([`alias.md`](alias.md) §3) | — | 8 | — |
| `CLAIM` zones per target ([`name.md`](name.md) §4.2) | — | 8 | — |
| `PART` records per target ([`multi-author-set.md`](multi-author-set.md) §3.4) | 8 | 32 | — |
| `HEAD` and `HIDDEN` records held, over all targets ([`file-set.md`](file-set.md) §5.2, [`hidden-set.md`](hidden-set.md) §3) | 1,024 | 4,096 | — |
| `GATE` records held, over all targets ([`hops.md`](hops.md) §4.3) | 64 | 256 | — |
| `NAME` records held, over all targets ([`name.md`](name.md) §3.2) | 1,024 | 4,096 | — |

- `SOURCE`: nodes behind one carrier-grade NAT address are distinct honest sources, hence host
  limit > 1. Each record still belongs to one authenticated node ID (`Protocol.md` §10.3).
- `ANON`: record names no address, so counted on the address the `Store` came from
  ([`hops.md`](hops.md) §4.2).
- `HEAD`/`HIDDEN`, `GATE`, `NAME` rows count over the node, not per target: each such kind holds
  one record per target, target derived from a free key. A record counts at the
  host and site of the `Store` that last stored it, renewal included, also one with `signer`. A
  record replacing one held under the same target from the same host adds none. Beyond the
  limit → `REJECTED`. An index node, holding every target, bounds the other kinds over all
  targets too (`Protocol.md` §11).
- A record beyond a host or site limit of this table (the publishers-per-entry row is a count,
  not a limit) is answered `REJECTED`, not `FULL`. `FULL` says the target has no room
  (`Protocol.md` §10.8). An address limit says nothing about the target: no `spill` set for it, and
  the publisher starts no own lookup because of it (`Protocol.md` §10.4).
- **Address limits come first.** A storing node checks this table's limits, and per-publisher-key
  ones, before the target's limits: a record over one is `REJECTED`, whether or not the target
  has room. A valid record not newer than the held one is `OLDER` (`Protocol.md` §10.7) before
  these limits are consulted; changes nothing. A limit over all targets of an index node
  (`Protocol.md` §11) is an address limit too, `REJECTED` before the target. A record renewing
  one held for the same publisher counts as new at a host or site holding no record of that
  publisher there. Over a limit → `REJECTED`; held record stays as is until it expires.
- Per-publisher-key (not per-address) limits are in the kinds' documents: a `KEYWORD` target
  takes at most 1,000 *(initial)* files of one publisher key and 2,000 of one publisher site
  (`Protocol.md` §10.8); a `MEMBER` target at most 4 sets of one publisher key
  ([`file-set.md`](file-set.md) §6); an `ALIAS` target at most 2 roots of one publisher key
  ([`alias.md`](alias.md) §3); a `CLAIM` target at most 2 zones of one publisher key
  ([`name.md`](name.md) §4.2); a `PART` target one record per author key
  ([`multi-author-set.md`](multi-author-set.md) §3.4).
- `KEYWORD` and `NOTE`: beyond the per-key, host and site limits of `Protocol.md` §10.8, records
  are accepted within the general flood limits; when an entry's publishers are counted
  (popularity, ranking, rating average), one site counts once. A network publishing many entries
  gains nothing per entry.

### 2.5 Dialling sources

Source records are unsigned (`Protocol.md` §10.3). A storing or index node can return records
pointing at an address that never published them, and every searcher of that file would dial
it. The handshake exposes the false record only after the attempt. So failing attempts are
limited.

Per file's sources, at most this many connection attempts **failed or still pending** per hour:

| | host | site | block |
|---|---|---|---|
| Failed or pending dials per file and hour | 2 | 4 | 8 |
| Failed or pending dials per hour, over all files | 8 | — | — |

- Attempt completing its handshake with the expected node ID: doesn't count, so honest sources
  behind one address or in one network are dialled without limit. A relay counts as dialled
  until its handshake completes with its node ID; one already connected when the dial starts
  doesn't count. A dial through a relay completes only when the relay
  forwards: a `Punch` answered `Error{DENIED}`, `Error{NOT_FOUND}`, or `Error{UNSUPPORTED}` from
  a node that is no relay counts as one failed attempt at the relay's address (`Protocol.md`
  §10.3), also on a connection open before. `Error{BUSY}` and every other relay answer don't
  count. An attempt made for several files counts under each.
- An attempt given up before its handshake completes (e.g. download closed) counts as failed.
- Limit counts every address dialled for the file, each under its own groups: a source's endpoints in
  both families, every relay from a source record or source exchange, every gate from a `GATE`
  record ([`hops.md`](hops.md) §4.3), also when a last hop dials it.
- An address that would exceed a limit is not dialled; kept and tried in a later hour. Other
  addresses of that source MAY be dialled within their own limits.
- A live stream ([`live.md`](live.md) §5.2), topic ([`pubsub.md`](pubsub.md) §5) and room
  ([`room.md`](room.md) §5) each count as a file here; the over-all-files limit counts them with
  files.
- Gates and addresses dialled for a node rather than a file (uploader's slot offer, chat,
  browse, friends) count in the over-all-files row. Only failed or pending attempts count, so a
  reached node is dialled without limit, also for a long chat.
- Limit is per file, so one answer can't use up a node's attempts for other downloads.
- **Over all files** host limit is 8 per hour; a source beyond it is kept, tried in a later hour.

So a victim address gets ≤ 2 attempts per hour per searcher and file, and 8 per searcher over all
files, however many false records, and whether they name it as endpoint, relay or gate.

## 3. What a requester may still do

- Connect to, download from, upload to any number of peers in one group. Limits concern only Kad
  contacts, Kad records and failed source dials.
- Answer requests from any address. A node over a limit isn't inserted or queried; its requests
  are served as usual.
- Query candidates beyond the §2.2 lookup limits in the first lookup
  ([`bootstrap.md`](bootstrap.md) §4), only while fewer than α = 3 limit-allowed candidates
  remain. Inserted into the table under the limits like any contact.

## 4. Comparison

### 4.1 BitTorrent DHT

The BEP index has no IP-limit proposal. BEP 42 restricts node IDs (model for `Protocol.md` §6.3);
BEP 45 only advises a multi-homed node to use one address per /64 because "other nodes might
treat IPv6 /64 subnets as a single entity". Limits in use are an implementation matter;
libtorrent's are the de-facto reference.

| | libtorrent | This extension |
|---|---|---|
| One address in the table | 1 (`dht_restrict_routing_ips`) | 1 per host group |
| Same bucket | 1 per IPv4 /24, 1 per IPv6 /64 | 2 per site, 4 per block |
| Whole table, per subnet | no limit | 10 per site, 20 per block |
| One lookup | 1 per /24, 1 per /64 (`dht_restrict_search_ips`) | 2 per site; 2 per block for a search, 4 for a store |
| Larger IPv6 groups | none: a /56 is 256 groups, a /48 is 65,536 | /48 and /32 |
| Larger IPv4 groups | none | /16 |
| Storage targets | no rule beyond the lookup filter | at most half, 4 of 8, in one block (§2.3) |
| ID tied to address | BEP 42, off by default (`dht_enforce_node_id`) | `Protocol.md` §6.3, always on |

libtorrent: stricter inside a bucket, nothing above /24 and /64. For IPv6 nearly ineffective, as
a home delegation already spans hundreds of /64s.

### 4.2 eMule Kad and eMuleQt

eMuleQt enforces the eMule limits in its routing bins and adds the SafeKad node tracker; both
IPv4 only.

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
| One node per address (SafeKad) | 2nd node on a tracked address rejected | host limit |
| ID change (SafeKad) | verified node changing ID within 1 h bans its address for up to 24 h | refused for 1 h, no ban |
| Unverified ID change against a verified record (SafeKad) | rejected | can't occur: every contact verified by its handshake (`Protocol.md` §6.3) |
| Unresponsive nodes (SafeKad) | remembered 5 min | contact dropped after two failed connections (`Protocol.md` §10.2) |
| LAN addresses | exempt | exempt in a named network only (§1) |

Address and site numbers taken over unchanged (years of use). Differences come from what ed3k
has and eMule Kad lacks: handshake-proven node ID, so SafeKad's ID-spoofing cases don't arise;
and IPv6 needs own tiers.

## 5. Alternatives considered

- **Ban instead of refuse.** Rejected: an address ban also hits other users of a carrier-grade
  NAT address or /64, and a forged-source trigger makes it a DoS tool. Refusal: same
  routing-table effect, no victim. Connection floods differ: `Protocol.md` §4.6 bans
  the node ID, never the address, counted on completed handshakes only.
- **One IPv6 tier (/64).** Rejected, see §4.1.
- **/56 instead of /48 as the IPv6 site.** A /48 customer would count as 256 sites. /48 is the
  largest prefix a single customer normally holds.
- **One node per block first in the closest set, more only when blocks run out.** Not taken.
  Stricter where blocks are plenty, but a preference with fall-back: with few blocks it allows
  what the count allows, and its picks depend on the blocks each requester happened to meet. The closest set has one hard count instead, R/2 per block; a lookup that
  can't meet it ends with fewer nodes (§2.3).
- **Limits by autonomous system.** Best grouping in principle (ownership, not prefix length).
  Needs a routing database every implementation ships and updates; differing versions disagree.
- **Looser limits while the table is small.** Fills young-network slots strict limits leave
  empty. Left out for simplicity (§8). Without re-check harmful: what got in early keeps its
  slots while it answers, so loose first-minute limits become permanent.
- **Disjoint lookup paths** (S/Kademlia). Complements these limits: d independent paths survive
  if fewer than d are captured. At about 2.5 KB per queried node over QUIC (`Protocol.md` §10.5) it
  multiplies every lookup's cost by d. Left for the datagram binding.

## 6. Small networks

Young network: honest nodes cluster in few providers; hard limits can leave slots empty. Hence
loose block limits: 4 of 10 slots in a bucket, half a closest set. The closest set
can still shrink: where all nodes near a key are in one block, a search ends with 2 nodes, a
store with 4 (§2.3). Whether limits should be looser while a table is small: open point (§8).

## 7. What an attacker still needs

Costs with limits in force, K = 10, R = 4 for a search, R = 8 for a store ("each near" = each address, or each /64, with a routing prefix near the key):

| Goal | IPv4 | IPv6 |
|---|---|---|
| Fill one bucket of a victim | addresses in 5 /24s spanning 3 /16s | addresses in 5 /48s spanning 3 /32s |
| Hold the whole closest set of a search (R = 4) | 4 addresses in 2 /24s, one /24 in each of 2 /16s, each near | 4 /64s in 2 /48s, one /48 in each of 2 /32s, each near |
| Hold the whole closest set of a store (R = 8) | 8 addresses in 4 /24s, two /24s in each of 2 /16s, each near | 8 /64s in 4 /48s, two /48s in each of 2 /32s, each near |
| More than 20 table entries on one victim | a second /16 | a second /32 |

- IPv6 is the weak column. A /29, usual first regional-registry allocation, holds 8 /32s, each
  reaching 16,384 routing prefixes, so most of the 8 have one near a given key on a 100,000-node
  network. A closest set needs 2 of the 8, so one /29 can hold a whole one, search or store.
- Two blocks suffice in both families, as one block may hold half a set; stricter would shut out
  honest nodes sharing a provider (§6). One block alone holds ≤ half; the other half answers.
- The limits remove the cheapest cases: a home /56 or /48, a single /32, a rented /24.
- What remains is addressed by the key puzzle (`Protocol.md` §6.3), reserved, and disjoint paths
  (§5), unspecified.

## 8. Open points

- All numbers untested against a real address distribution; block limits especially to be
  checked against the largest provider's share once a network can be measured.
- Whether the IPv6 block should be /29 instead of /32: would bind the usual registry allocation, at
  the price of grouping more unrelated customers.
- Whether bucket limits should be looser while a table is small (§6). Drafted, left out for
  simplicity: needs a boundary, a re-check on crossing it, and a band around it so a table
  doesn't remove and re-insert contacts in turns. §2.1 already says
  what any such rule must do.
- Whether the IPv6 routing-ID mask should end at /32 (`Protocol.md` §6.3). Left at /48; with
  /32, a /32 would reach 8 routing prefixes instead of 16,384.
