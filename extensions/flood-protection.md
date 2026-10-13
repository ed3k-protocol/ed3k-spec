# Extension: packet flood protection (`flood-protection`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional; not part of what a version 1 node must do.
**Extends:** [`Protocol.md`](../Protocol.md) §4.1, §4.4 and §4.6, and [`kad-datagram.md`](kad-datagram.md) §5. Section numbers (§) refer to the main document unless a file is named.
**Capability:** none. The rules are local policy and use only what version 1 already requires.

The connection budget of §4.6 counts completed handshakes. It does nothing against packets that
never get that far: Initial packets from forged addresses, or from real addresses that abandon
the handshake. This extension describes how a node limits the work it does for such packets.

It has two tools and no new wire element:

- **QUIC Retry.** A node under load makes an unknown address prove itself before it does any
  cryptographic work for it.
- **Address budgets.** Packets from unvalidated addresses are counted per host, site and block.
  Above a limit the node answers with a Retry, and drops only what one host sends beyond its
  share of Retry packets.

A node that implements none of this still interoperates. Every version 1 node accepts a Retry
(§4.1).

## 1. What a packet costs the receiver

| Step | Work | Who can cause it |
|---|---|---|
| Unmask 5 bytes, check `magic` (§4.5 steps 1 and 2) | one BLAKE3 block | anyone who knows the node ID |
| Reassemble fragments (§4.4) | memory for at most 4 messages per source address, 3 s | anyone who knows the node ID: the pieces pass the `magic` check first |
| Answer with Retry | one keyed hash, one small datagram | anyone who knows the node ID |
| Start a handshake | key exchange, a signature and a verification; state for the connection | anyone who knows the node ID, unless Retry is demanded |
| Complete a handshake | counted by §4.6 | a node at a real address |

The expensive step is the fourth, starting a handshake: on the order of 0.1 to 0.2 ms of one core per handshake
(estimate), and memory until it completes or times out. The aim is that only validated
addresses reach it while the node is under load.

## 2. Retry

A node is **guarded** while its own measure of load is above a threshold it chooses, for
example the number of handshakes in progress or of Initials per second. While guarded:

- An Initial without a valid token is answered with a Retry packet and otherwise ignored. No
  state is kept.
- An Initial with a valid token starts a handshake.

**Tokens**

A token is opaque to the initiator; its layout is the node's own, and usually the QUIC
library's. It MUST let the node check without state that the token was issued by this node
within the validity time (a time stamp and a keyed hash under `secret`), that the Initial
comes from the address the token was issued to (and, for a Retry token, the port), and, for a
Retry token, which destination connection ID the first Initial carried: the handshake has to
prove it in `original_destination_connection_id` (RFC 9000 §7.3), so a Retry token carries
that ID, encrypted or alongside its hash. A Retry token is about 40 bytes, a return token
about 16.

| Token | Issued in | Valid for *(initial)* | Bound to |
|---|---|---|---|
| Retry token | a Retry packet | 10 s | the host address (IPv4 /32, IPv6 /64) and port |
| Return token | a `NEW_TOKEN` frame after a handshake | 24 h | the host address |

- `secret` is random and replaced every 24 h; the previous one stays valid for one period.
- A node SHOULD send a return token on every connection, guarded or not. A peer that comes back
  from the same address presents it and is not asked to retry. Downloaders returning to a queue
  (§12.5) and routing-table contacts are the usual case.
- A Retry costs the initiator one round trip and one more padded Initial. 0-RTT data is sent
  again with the second Initial.
- A Retry packet is smaller than the Initial that caused it, so it cannot be used for
  amplification. It is sent as a `HANDSHAKE` datagram like any long-header packet.

A node MAY also demand Retry from every unknown address at all times. That trades a round trip
on every first contact for never doing handshake work for a forged address.

## 3. Address budgets

Packets that would start a new flow from an address without a valid token are counted in the
groups of [`ip-diversity.md`](ip-diversity.md) §1.

| Per second *(initial)* | host | site | block |
|---|---|---|---|
| Handshakes started without a token (node not guarded) | 4 | 16 | 64 |
| Retry packets sent | 4 | — | — |

- **Above a handshake budget the node answers with a Retry** (§2 of this document) and starts
  no handshake, as if it were guarded for that group. It does not drop the Initial. Forged
  Initials from one site or block then cost every other address of that group one round trip,
  not its access: an honest node there answers the Retry and is served.
- **Only the Retry budget drops.** An Initial without a token from a host that already got 4
  Retry packets in this second is dropped without an answer. This bounds what a forged source
  address makes the node send to one victim host. The victim itself is cut off only for a first
  contact without a token; with a return token it is not counted here at all.
- **There is no Retry budget per site or block.** A Retry costs one keyed hash and keeps no
  state, and it is smaller than the Initial that caused it. A site or block budget for it would
  bring back the fault this rule removes: whoever forges addresses of a /16 could use it up, and
  the whole /16 would get no answer.
- **Nothing here bans.** The source address of these packets is not proven; a ban would let an
  attacker cut off a victim by forging its address. Bans come only from §4.6.
- A node SHOULD also limit handshakes in progress in total, and drop the oldest unvalidated
  ones first.
- Validated handshake starts are further limited per site and block while guarded: 120 and 480
  per minute *(initial)*, four and sixteen times the host budget of §4.6. Above them the
  handshake is refused, not banned, since a site or a block holds many unrelated users.

## 4. Kad datagrams

The sessionless Kad binding ([`kad-datagram.md`](kad-datagram.md)) already has the equivalent
of Retry: an `INIT` is answered with a `CHALLENGE` that carries a cookie, statelessly, and a
`REQUEST` is processed only with a valid cookie.

This extension adds the group budgets to it:

| Per second *(initial)* | host | site | block |
|---|---|---|---|
| `CHALLENGE`s sent | 4 (as in `kad-datagram.md` §5) | 16 | 64 |
| New key derivations for `REQUEST`s | 8 (as there) | 32 | 128 |

A `REQUEST` with a valid cookie comes from a validated address and proves its key. It is not
counted by the connection budget of §4.6, which would need state per requester that the
responder does not keep ([`kad-datagram.md`](kad-datagram.md) §1). The `REQUEST` rate of
[`kad-datagram.md`](kad-datagram.md) §5 bounds it instead: 16 / 64 / 256 `REQUEST`s per second
per host / site / block *(initial)*, dropped above. The table above bounds only what comes
before a validated `REQUEST`: `CHALLENGE`s and key derivations.

## 5. What it does not do

- **A flood from many real addresses** passes Retry and stays under each group's budget. Every
  such connection costs the attacker as little as it costs an honest node. The limits of §4.6
  and the diversity limits bound what each address achieves; nothing here raises the price per
  connection.
- **Bandwidth.** A node cannot stop packets from arriving. It can only keep the work per packet
  small.
- **Knowing the node ID** is needed to pass the `magic` check, but node IDs are public.

## 6. Alternatives considered

- **A hash puzzle on the first packet**, with difficulty raised by load. It would make real
  addresses pay per connection, for QUIC and for Kad datagrams alike. Not taken: it needs a new
  datagram kind, and honest low-power nodes pay it too while a node is attacked. It remains the
  next step if floods from real addresses become a problem.
- **A puzzle only.** It would do by computation what Retry does for free against forged
  addresses.
- **Retry always, as a rule of the protocol.** One more round trip and a second padded Initial
  on every first contact, about 1.2 KB, for all nodes at all times.
- **Bans for unvalidated packets.** Rejected, see §3.

## 7. Open points

- The budgets are first values. A router that many nodes behind one carrier NAT bootstrap from
  may need a wider host budget.
- Whether a return token should also be accepted from another address in the same site, for
  nodes whose address changes within their network.
- The interaction with the fragment limits of §4.4 under load: 4 incomplete messages per source
  address may be too many when sources are forged.
