# discv5 compared with ed3k

**Status:** design note (2026-10-11). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §9.2, §10.2, §10.3 and §10.4, and the extensions [`ip-diversity`](../extensions/ip-diversity.md), [`flood-protection`](../extensions/flood-protection.md) and [`kad-datagram`](../extensions/kad-datagram.md). Section numbers (§) refer to `Protocol.md`.

discv5 is the node discovery protocol of Ethereum: a Kademlia table over sealed UDP datagrams
that finds nodes and stores nothing. This note compares its Rust implementation
([sigp/discv5](https://github.com/sigp/discv5), commit `8dc0696`) with ed3k's Kad. Everything said
about discv5 here is from reading that source; nothing was run.

**Result:** four rules were taken over, and Kad timers were added (section 2). In flood handling
and address diversity ed3k is the stricter of the two (section 3).

## 1. Overview

| | discv5 | ed3k |
|---|---|---|
| Purpose | find nodes | find nodes, and store source, keyword, note and file set records |
| Transport | one UDP datagram each way, at most 1280 bytes | QUIC (§4); single datagrams as an extension |
| Session | challenge, then an ephemeral key exchange; session kept up to 1 day, 1,000 sessions | QUIC connection; the datagram extension keeps no state and uses static keys |
| Contact | a signed node record with a sequence number, up to 300 bytes | 22 or 34 bytes; the key is proven by the handshake |
| Request | nodes at up to 3 distances from the answering node | nodes closer to a target, with the operation in the same request |
| Bucket size, parallel queries | 16, 3 | 10, 3 |
| Liveness | ping of connected nodes every 5 min | a contact is dropped after two failed connection attempts |

## 2. Taken over

### 2.1 Checking an answer

discv5 asks for nodes at given distances and checks each returned record against them. A node
that returns others is banned for 1 h.

ed3k told the answering node to return closer contacts but did not tell the requester to check.
§10.3 now does: a contact that is not closer than the answering node is dropped, as is every
contact after the 16th, and the answer counts as a failed attempt. The check is exact, because
a contact's routing ID follows from its address and node ID, so a correct node never fails it.
ed3k does not ban for it; two failures remove the contact from the table.

### 2.2 Whose address reports count

discv5 counts a vote for its own address only from a node it dialled itself, unless it is short
of votes. It needs 10 votes, each valid for 120 s, and the second most named address must stay
below 70 % of the first. Its source names the reason: a router that changes ports gives many
votes and no agreement, and the address would flip.

ed3k asked for 3 reports from different addresses, from any connection. A party that dials in
from three addresses could set a node's view of its own address. §9.2 now counts a report only
from a connection the node opened, one per site, for 60 min, and adopts an endpoint only when it
leads 2 to 1.

| | Pro | Con |
|---|---|---|
| Only own connections | dialling in buys no votes | a new node learns its address a few connections later |
| One per site | one network gives one report | none on a public network; private addresses are exempt as elsewhere |
| A lead of 2 to 1 | one false report changes nothing | with exactly 3 reports, a fourth that disagrees is tolerated, a second is not |

ed3k keeps 3 where discv5 uses 10: discv5 publishes the result in a signed record that other
nodes store, while an ed3k node uses it to classify itself and tests it with `ReachTest`.

### 2.3 Incoming connections as evidence

discv5 has no reach test. After it announces an address it waits 5 min for 2 incoming sessions;
without them it withdraws the address for 6 h.

ed3k has an active test and keeps it. It now also takes 2 unasked connections from different
sites as evidence for `open` (§9.2). An open node receives such connections all the time, so
its class is confirmed between tests and a test may be skipped.

| | Pro | Con |
|---|---|---|
| Passive evidence | no traffic; continuous | a mapping left by an earlier exchange can let one connection in. Hence 2, from different sites, from hosts not sent to for 60 min |
| Passive evidence alone (discv5) | no test message | a quiet node cannot tell "closed" from "nobody called" |

### 2.4 Contacts that dialled in

discv5 can limit, per bucket, the nodes that entered because they connected first. The limit is
off by default. A node's direction is never changed from incoming to outgoing later.

ed3k now limits them to half a bucket (§10.2) and keeps the mark in the same way. The diversity
limits already bound what one network can place in a table; this bounds what many networks can
place by taking the initiative.

### 2.5 Timers

discv5 gives a request 1 s and one retry, a queried node 2 s, and a lookup 60 s. ed3k named no
Kad timers. §10.4 now gives a node 5 s, which has to cover a QUIC handshake, and a lookup 60 s.

## 3. Not taken over

| discv5 | Why not |
|---|---|
| Bans an address that exceeds a packet rate, before the address is proven | A forged source address would ban a victim. ed3k drops and never bans on unproven addresses (`flood-protection` §3) |
| Bans an address seen with 10 node IDs | Carrier-grade NAT puts many honest nodes behind one address. ed3k refuses a new node ID per host for an hour and does not ban (`ip-diversity` §2.1) |
| Subnet limit of 2 per bucket and 10 per table: off by default, IPv4 only | ed3k requires limits in three group sizes for both families |
| Signed node records | About ten times the size of a contact. ed3k proves key and address by connecting (§6.3) |
| Requests by distance instead of by target | Hides the target from nodes on the way, but an operation cannot ride in the request (§10.5) |
| A generic request for other protocols | ed3k has typed messages and extensions (§16) |
| Ephemeral session keys for Kad datagrams | Needs state per requester; the trade is recorded in `kad-datagram` §3.5 |

## 4. Open points

- The 5 s and 60 s timers are estimates.
- Whether 5 of 10 is the right share for dialled-in contacts on a small network.
