# Extension: packet flood protection (`flood-protection`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional; not part of what a version 1 node must do.
**Extends:** [`Protocol.md`](../Protocol.md) §4.1, §4.4 and §4.6, and [`kad-datagram.md`](kad-datagram.md) §5. § = `Protocol.md` unless a document is named.
**Capability:** none. Local policy, using only what version 1 already requires.

§4.6 connection budget counts completed handshakes, not packets that never get that far:
Initials from forged addresses, or from real ones abandoning the handshake. This extension
limits a node's work for those. Two tools, no new wire element:

- **QUIC Retry.** Under load, node makes an unknown address prove itself before any
  cryptographic work for it.
- **Address budgets.** Packets from unvalidated addresses counted per host, site and block.
  Above a limit node answers with Retry; drops only what one host sends beyond its share of
  Retry packets.

A node implementing none of this still interoperates; every version 1 node accepts a Retry
(§4.1).

## 1. What a packet costs the receiver

| Step | Work | Who can cause it |
|---|---|---|
| Unmask 5 bytes, check `magic` (§4.5 steps 1 and 2) | one BLAKE3 block | anyone knowing the node ID |
| Reassemble fragments (§4.4) | memory for ≤ 4 messages per source address and port, 3 s | anyone knowing the node ID: pieces pass `magic` check first |
| Answer with Retry | one keyed hash, one small datagram | anyone knowing the node ID |
| Start a handshake | key exchange, a signature and a verification; connection state | anyone knowing the node ID, unless Retry is demanded |
| Complete a handshake | counted by §4.6 | a node at a real address |

Expensive step: the fourth, ~0.1 to 0.2 ms of one core per handshake (estimate), plus memory
until done or timed out. Aim: under load only validated addresses reach it.

## 2. Retry

Node is **guarded** while its own load measure exceeds a threshold it chooses, e.g. handshakes in
progress or Initials per second. While guarded:

- Initial w/o valid token → Retry packet, otherwise ignored; no state kept.
- Initial with valid token starts a handshake.

**Tokens**

Opaque to the initiator; layout is the node's own, usually the QUIC library's. It MUST let the
node check statelessly: that this node issued it within validity time (time stamp and keyed hash
under `secret`); that the Initial comes from the host address it was issued to (Retry token:
also the port); and, for a Retry token, which destination connection ID the first Initial
carried: handshake must prove it in `original_destination_connection_id` (RFC 9000 §7.3), so a
Retry token carries that ID, encrypted or alongside its hash. Retry token ~40 bytes, return
token ~16.

| Token | Issued in | Valid for *(initial)* | Bound to |
|---|---|---|---|
| Retry token | a Retry packet | 10 s | host address (IPv4 /32, IPv6 /64) and port |
| Return token | a `NEW_TOKEN` frame after a handshake | 24 h | host address |

- **One binding.** Bound at least to the host address (IPv4 address or IPv6 /64,
  [`ip-diversity.md`](ip-diversity.md) §1); Retry token also the port. A node MAY bind to the
  exact address: a peer whose address changed within its /64 then answers one more Retry.
- `secret` random, replaced every 24 h; previous stays valid one period.
- A node SHOULD send a return token on every connection, guarded or not; a peer returning from
  the same address presents it and isn't asked to retry.
- A node SHOULD accept a return token once (RFC 9000 §8.1.4); used-token log is the only state
  needed, entry kept until token validity ends; log full → token not valid. Initiator uses a
  return token once, keeps newest per node. Validated-start limit (§3 of this document) stays for nodes accepting
  a token more than once. Usual case: downloaders returning to a queue (§12.5), routing-table
  contacts.
- Retry costs initiator one round trip and one more padded Initial; 0-RTT data resent with the
  second Initial.
- Retry packet is smaller than its Initial → no amplification. Sent as a `HANDSHAKE` datagram
  like any long-header packet.

A node MAY demand Retry from every unknown address always, trading a round trip on every first contact
for never doing handshake work for a forged address.

## 3. Address budgets

Packets that would start a new flow from an address without valid token are counted in the
groups of [`ip-diversity.md`](ip-diversity.md) §1.

| Per second *(initial)* | host | site | block |
|---|---|---|---|
| Handshakes started without a token (node not guarded) | 4 | 16 | 64 |
| Retry packets sent | 4 | — | — |

- **Above a handshake budget node answers with Retry** (§2 of this document), starts no
  handshake, as if guarded for that group; doesn't drop the Initial. Forged Initials from one
  site/block then cost the group's other addresses one round trip, not access: an honest node
  there answers the Retry and is served.
- **Only the Retry budget drops.** Tokenless Initial from a host that already got 4 Retry
  packets this second is dropped unanswered. Bounds what a forged source makes the node send to
  one victim host. Victim is cut off only for a first contact w/o token; with a return token it
  isn't counted here at all.
- **No Retry budget per site or block.** Retry = one keyed hash, no state, smaller than its
  Initial. Such a budget would reintroduce the fault removed here: a forger of /16 addresses
  could use it up, leaving the whole /16 unanswered.
- **Nothing here bans.** Source address is unproven; a ban would let an attacker cut off a victim by forging its address. Bans come only from §4.6 (node IDs) and §12.3 (bad blocks), after a completed handshake.
- A node SHOULD also limit total handshakes in progress, dropping oldest unvalidated first.
- While guarded, validated handshake starts are limited per host, site and block: 30, 120 and
  480 per minute *(initial)*, one, four and sixteen times the §4.6 host budget. Counts starts,
  not completed handshakes: a token passes for its whole host for 24 h, and a never-completed
  handshake escapes §4.6. Above them handshake is refused, not banned: a site or block holds many
  unrelated users.

## 4. Kad datagrams

Sessionless Kad binding ([`kad-datagram.md`](kad-datagram.md)) already has a Retry equivalent:
`INIT` answered statelessly with a `CHALLENGE` carrying a cookie; `REQUEST` processed only with
a valid cookie. This extension adds group budgets:

| Per second *(initial)* | host | site | block |
|---|---|---|---|
| `CHALLENGE`s sent | 4 (as in `kad-datagram.md` §5) | — | — |
| New key derivations for `REQUEST`s | 8 (as there) | 32 | 128 |

Valid-cookie `REQUEST` = validated address, proven key. Not counted by §4.6 connection budget
(would need per-requester state the responder doesn't keep,
[`kad-datagram.md`](kad-datagram.md) §1); the `REQUEST` rate of
[`kad-datagram.md`](kad-datagram.md) §5 bounds it: 16 / 64 / 256 `REQUEST`s per second per
host / site / block *(initial)*, dropped above. Table bounds only what precedes a validated
`REQUEST`: `CHALLENGE`s and key derivations. No site/block budget for `CHALLENGE`s: `INIT` can
carry a forged source; such a budget would let forged addresses keep a whole network from
getting a cookie (as §3 of this document says of Retry). `CHALLENGE` never longer than its
`INIT` → per-host budget suffices. Key derivations follow a valid cookie, which a forged source
can't have.

## 5. What it does not do

- **Flood from many real addresses** passes Retry, stays under each group budget; each
  connection costs the attacker as little as an honest node. §4.6 and diversity limits bound
  what each address achieves; nothing here raises price per connection.
- **Bandwidth.** Node can't stop packets arriving, only keep work per packet small.
- **Knowing the node ID** is needed to pass `magic` check, but node IDs are public.

## 6. Alternatives considered

- **Hash puzzle on first packet**, difficulty raised by load: real addresses pay per
  connection, for QUIC and Kad datagrams alike. Not taken: needs a new datagram kind; honest
  low-power nodes pay too during an attack. Next step if floods from real addresses become a
  problem.
- **Puzzle only.** Does by computation what Retry does for free against forged addresses.
- **Retry always, as protocol rule.** One more round trip and second padded Initial

 on every first
  contact, ~1.2 KB, for all nodes always.
- **Bans for unvalidated packets.** Rejected, see §3 of this document.

## 7. Open points

- Budgets are first values. A router that many nodes behind one carrier NAT bootstrap from may
  need a wider host budget.
- Whether to accept a return token also from another address in the same site, for nodes whose
  address changes within their network.
- Interaction with §4.4 fragment limits under load: 4 incomplete messages per source address
  and port may be too many with forged sources.
