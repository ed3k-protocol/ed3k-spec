# How many relays a node keeps

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §9.4 (relays) and §9.5 (hole punching). Section numbers (§) refer to that document.

A node that is not open in an address family keeps reservations at relays, and lists them in
every `Peer` record it publishes. This note records why the number is 2 in total, against one
relay and against two per address family.

**Result:** 2 relays in total. Both are listed under IPv4 and one of them also under IPv6 when
the node is not open in either family. One relay makes every published record useless when
that relay leaves; four use relay capacity twice as fast and make the record 34 bytes longer.

The record sizes below follow from the field sizes of §5.4. The share of leaves in the network
is not known; the capacity figures are bounds, not forecasts.

## 1. Why more than one

- **A relay list is published, and a published list goes stale.** The list is in the node's
  source records (lifetime 5 h, §10.3), in what other peers pass on, and in the queue entries
  at uploaders, which the node may update once per 30 min (§12.5). With one relay, the node is
  unreachable through every copy from the moment the relay leaves until each copy is renewed.
- **Repairing that at once is expensive.** The node would have to store a new source record
  for every file it shares, immediately. With a second relay still working, the new list can
  go out with the normal refresh.
- **A relay can refuse one initiator.** It may be at its forward limit (§9.4), may have banned
  the initiator's address (§4.6), or may not be reachable from there.
- **A relay can misbehave.** It can drop `Punch` messages and so hide a node without anyone
  noticing. A second relay in another site bounds that.

A third relay adds little. It helps only when two relays fail within one refresh interval.

## 2. What a relay costs

| Cost | Per relay |
|---|---|
| Bytes in every `Peer` record of the node | 22 B listed with an IPv4 endpoint, 34 B with IPv6 |
| Keep-alive | one small packet each way per 20 s |
| Relay capacity | 1 of the at least 8 reservations a router offers |

- **Record size.** A `Peer` record without relays is about 50 B. It travels in Kad source
  records, in source exchange and in `FileRequest`, so the relay list is the larger part of a
  leaf's record.
- **Relay capacity.** With 8 reservations per router, the network carries 4 leaves per router
  when each takes 2 reservations, and 2 leaves per router when each takes 4.

## 3. Why not per address family

The list a relay is in says how an initiator reaches the relay. It does not say which family
the punch uses: `Punch` carries the endpoints of both. One relay that has IPv4 and IPv6
therefore serves initiators of either family, and a count per family buys redundancy twice.

## 4. Comparison

For a node that is not open in IPv4 nor in IPv6:

| | Reservations | Added to the record | Weakness |
|---|---|---|---|
| 1 relay | 1 | 22 B | every relay loss makes all published records dead |
| **2 in total; both under IPv4, one also under IPv6** | 2 | 78 B | an initiator that has only IPv6 finds one relay, not two |
| 2 per family | 4 | 112 B | twice the relay capacity, 34 B more per record |

A node that is not open in one family only lists both relays in that family: 44 B for IPv4,
68 B for IPv6.

## 5. What others do

| | Relays kept | Source |
|---|---|---|
| eMule Kad | 1 buddy | a firewalled node has one buddy; an open node serves one at a time in stock eMule |
| go-libp2p AutoRelay | 2 by default (`desiredRelays`, set with `WithNumRelays`) | `p2p/host/autorelay/options.go`, checked 2026-10-11 |

The relay side differs more. A go-libp2p relay accepts 128 reservations by default
(`MaxReservations`, `p2p/protocol/circuitv2/relay/resources.go`) and js-libp2p 15, where §9.4
asks a router for at least 8. A libp2p relay is a service a node chooses to run; in ed3k every
reachable node is a relay, so the minimum is set low and the number of relays is large.

## 6. Open points

- A node that is open in IPv4 but not in IPv6 needs its relays only for initiators without
  IPv4. One relay would do there, at the price of a fourth case in the listing rule.
- Whether a node whose relays are often at the forward limit should keep a third.
- The numbers should be checked against the measured share of leaves once a network exists.
