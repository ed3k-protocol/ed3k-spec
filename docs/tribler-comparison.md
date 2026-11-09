# Tribler compared with ed3k `hops`

**Status:** design note (2026-11-09). Informative.
**Relates to:** [`extensions/hops.md`](../extensions/hops.md), [`Protocol.md`](../Protocol.md) §9 and §14. § = `hops.md` unless another document is named.

**Tribler** is a BitTorrent client with its own onion routing. It does **not** use the Tor
network: circuits run between Tribler users only, over IPv8, Tribler's own UDP overlay. The
design follows Tor (create/extend handshake, a layer per hop, `relay_early` cells, introduction
and rendezvous points), but the code and the crowd are separate.

Read in source, nothing run:

| Part | Repository | Commit |
|---|---|---|
| Client, tunnel glue, libtorrent sessions | `Tribler/tribler` | `af0f93e7` (2026-11-04) |
| Circuits, hidden services, DHT, hole punching | `Tribler/py-ipv8` | `4a294ed1` (2026-07-08), the submodule pin |
| Cell crypto, SOCKS5, exit sockets | `Tribler/ipv8-rust-tunnels` | `d18f148` (2026-07-19), unpinned requirement |

**Result:** the two designs agree on most points. Tribler's code supports four choices `hops`
already made: the owner picks hops, entry hops, no exits, and leaving 1 hop out of what is
called anonymity. Two ideas came out of it (§4). Letting NATed nodes be hops was taken
(`hops.md` §2.5). A cap on how long a path can get is an open point (`hops.md` §8). §5 answers whether eD2K over Tor would
give LowID.

## 1. Tribler in short

**Anonymous download** (default on, **1 hop**, user picks 1 to 3). Every download with hops > 0
runs in a separate libtorrent session with TCP turned off, uTP only, `anonymous_mode`, and
`force_proxy` to a local SOCKS5 UDP associate. The SOCKS5 server puts each uTP datagram into a
circuit. Two ways out:

- **Exit node.** The last hop of a data circuit sends the datagram from its own UDP socket into
  the ordinary BitTorrent swarm. The swarm sees the exit's address. An exit only passes what looks
  like uTP, a UDP tracker, DHT or IPv8, and nothing to loopback or multicast. Being an exit is
  opt-in, default off.
- **Hidden swarm.** In parallel, the client joins the swarm's hidden side. Peers inside it are
  other Tribler users that seed with hops > 0 (below). libtorrent sees each of them as a fake
  address, `circuit-id-as-IPv4:1024`.

**Hidden seeding** ("safe seeding", default on). A finished download with 0 hops is re-added with
the default hop count. Per swarm:

1. The seeder makes a new curve25519 **seeder key for this swarm alone** and builds a circuit of
   hops + 1 nodes. Its last node becomes the **introduction point**.
2. The introduction point announces `(own address, seeder key)` in the IPv8 DHT under
   `SHA-1("tribler anonymous download" ‖ hex(infohash))`, and in a PEX overlay for that infohash.
3. A downloader asks through its own circuit: the circuit's exit does the DHT lookup and answers
   with introduction points.
4. The downloader sends `CreateE2E` (an ephemeral DH value) through its circuit to the
   introduction point, which passes it down the seeder's circuit.
5. The seeder builds a circuit of `hops` nodes. Its last node becomes the **rendezvous point**,
   which gets a random 20-byte cookie. The seeder answers `CreatedE2E` with its DH half and the
   rendezvous address and cookie, encrypted to the downloader.
6. The downloader builds a circuit of hops + 1 nodes ending at that rendezvous point and sends
   `LinkE2E{cookie}`. The rendezvous point joins the two circuits. From then on an end-to-end
   layer (`hs_session_keys`) covers the data.

Between a downloader and a hidden seeder at `hops = h` there are h of the downloader's relays,
the rendezvous point, and h − 1 of the seeder's. At the default h = 1 the rendezvous point is the
seeder's only hop.

## 2. Side by side

| | Tribler | ed3k `hops` |
|---|---|---|
| Network | own onion routing between Tribler users, over IPv8/UDP | own paths between ed3k nodes, over QUIC |
| Hops | 1 to 3, one setting, **default 1** | 0 to 3 per direction, each node chooses (§1) |
| Way into the plain network | exit nodes, opt-in | none: a hop never connects outside ed3k (§5) |
| Who picks the next hop | the owner, from a list of 4 relays + 4 exits that **the previous hop** sends in its `Created`. The owner's own list is only a fallback | the owner picks every hop (§2.1) |
| First hop | random. The least-used peers among own circuits come first. No entry guards | 3 entry hops kept for 30 days (§2.1) |
| Network diversity | none in the code | hops in different sites, should be different blocks (§2.1) |
| Who may relay | any peer with the `RELAY` flag. Every Tribler sets it by default. NATed peers too, reached through IPv8 hole punching | a `ROUTER`, or a leaf behind a cone NAT, reached by a punch; never first hop or gate (§2.5) |
| Handshake per hop | ephemeral × ephemeral ‖ static × ephemeral X25519, HMAC auth | the same pattern: `ss1` static, `ss2` ephemeral, `proof` (§2.3) |
| Layer cipher | ChaCha20-Poly1305 per layer, 8-byte explicit counter + 16-byte tag: **24 B per layer per cell** | ChaCha20 without a tag, running over the stream (§2.3) |
| What a link carries | UDP cells, nothing resent along the way. uTP end to end repairs loss | one reliable QUIC exchange per link and stream (§2.4) |
| Cap on path length | `relay_early`: at most 8 extends per circuit, counted at each relay | none at a hop: the owner keeps to 3 (§4.1 below) |
| Identity of a hidden sharer | a new seeder key **per swarm** | one anonymous key per node (§1) |
| Where a hidden sharer is listed | introduction point in the IPv8 DHT + PEX, under a hash of the infohash | gate in a signed `GATE` record (§4.3) |
| Meeting point | rendezvous point picked by the seeder | none: each side builds half; gate joins them (§3.1) |
| Lookups | the circuit's exit does the DHT lookup | the last hop does `HopLookup` (§4.1) |
| Limits at a relay | 100 circuits joined; a circuit lives 1 h (introduction circuits 24 h), dies after 20 s idle or 10 GiB | per-neighbour paths, streams, dials, 256 KiB buffered per stream (§5) |
| Reward for relaying | none in the current code (earlier bandwidth-token accounting was removed) | give and take per neighbour, 16 KiB/s base rate (§5) |

## 3. Where Tribler supports choices `hops` made

**Owner picks every hop.** In Tribler, hop *n* hands the owner the candidates for hop *n + 1*. A
hostile first hop can name only colluding peers, and the owner can't tell. `hops` keeps the owner
picking every hop.

**Entry hops.** Tribler picks a new first hop for each circuit, preferring the least-used ones, so
over time a client meets many first hops. `hops` keeps 3 for 30 days, for the reason given in
§2.1.

**No exits.** Tribler has to work around a shortage of exits. It caches exit addresses on disk
between runs and keeps exits in its peer list on purpose (`GoldenRatioStrategy`, 9/16 non-exit).
Exits are opt-in because they send strangers' traffic from their own address. `hops` has no
plain network to reach, so it needs no exits.

**1 hop is not anonymity.** Tribler's default is 1 hop: the exit (for a download) or the
rendezvous point (for a seeder) sees both the user's address and what it fetches or serves. §6
says the same of 1 hop in ed3k. A client SHOULD NOT start at 1 and call it anonymous.

**A tag on every layer.** Tribler pays 24 B per layer per cell for it (72 B at 3 hops, on cells
of about 1.4 KB). This confirms the cost listed in §7 *Alternatives*.

## 4. Worth a look

### 4.1 A cap on path length

Tribler marks the cells that may carry an extend (`relay_early`). Each relay drops a circuit
after 8 of them, so nobody can build a path long enough to loop through the same hops many
times and multiply their load. In `hops`, a hop can't read the control stream and can't tell how
long a path is. A path looping through one hop many times is limited only by the per-neighbour
limits (8 paths, 16 per host). A middle hop can't see frame boundaries either: the control
stream is layered bytes. What it can count is bytes. A control stream carries only `HopExtend`
(under 200 B each) and `HopGate` away from the owner, so a cap on control-stream bytes away from
the owner, say 1 KiB *(initial)* per path, limits extends without reading them. Recorded as an
open point in `hops.md` §8.

### 4.2 NATed hops (LowID nodes as hops): taken

Tribler lets any peer relay, NATed ones too: IPv8's introduction walk punches holes. That way
peers behind a NAT add capacity instead of only using it. `hops` 0.1 let only an open `ROUTER`
set `HOP`, so a NATed node used paths, never carried them, and got only the base rate (§5).

Taken in `hops.md` §2.5 (draft 0.2):

- A leaf that is `cone` where it holds relay reservations MAY set `HOP`. Not `symmetric` or
  `unknown`.
- Relays list their `HOP` leaves in a `KadAnswer` when a `KadRequest` sets `hops`. No new record
  kind is needed, and the endpoint is the one the relay observes.
- `HopExtend` carries `relays4`/`relays6`, as `HopConnect` does. The last hop punches.
- A leaf hop is never the first hop (entry hops last 30 days, a leaf's mapping doesn't) and never
  a gate (`Gate.node` is dialled directly). It makes up at most half of the hop list, one per
  answer.

Costs: one punch per extend, a path dies with a leaf's mapping, and leaves are found through
their relays' listings. Entry hops and gates for leaves stay an open point.

### 4.3 A key per share (looked at, not taken)

Tribler's seeder key per swarm means two hidden swarms can't be linked by key. `hops` uses one
anonymous key, which keeps queue places and credit but links everything shared under it. A
node MAY already take a new key (§1). One key per file would also mean one gate record and one
set of lookups per file. Left as is.

## 5. eD2K over Tor: LowID?

**Yes, always LowID**, and with less than LowID usually gets:

- **HighID needs an inbound TCP connection.** The server connects back to the client's address
  as seen on the login. Over Tor that address is the exit's, which accepts no inbound
  connections. Result: LowID on every server. Peers also see only the exit's address, so they
  can't call back.
- **Tor carries TCP streams only.** No UDP means no Kad, no UDP source reasks, no UDP server
  queries or global search, and no UDP hole punching. eD2K over Tor is server-only.
- **Tor's default exit policy blocks the eD2K ports** (4661–4666), along with 6881–6999 for
  BitTorrent. Only servers and peers on other ports can be reached, through exits that allow
  them.
- **LowID to LowID can't work.** A LowID download needs a HighID source on a reachable port.
  Every other Tor user is LowID too.
- **The user hash links sessions.** eD2K sends the same user hash on every connection, so Tor
  hides the address but not who you are across exits and days.
- The Tor Project asks people not to use Tor for bulk file sharing, and exits are scarce.

An onion service doesn't help either: eD2K client IDs and server callbacks are IPv4 addresses,
and an onion address doesn't fit. That's why I2P-based eD2K clients run as a separate network
that only reaches each other.

For ed3k the same holds: a Tor binding gives streams, not UDP, so no hole punching and no Kad
datagrams. §7 *Alternatives* left it out for that reason. Tribler made the same choice and built
its own UDP onion layer.

**LowID and `hops`.** A hidden ed3k node needs no inbound reachability. Downloads go out over a
path it built, and shares are reached through a gate at the end of a path it also built outwards
(§3.1). A LowID node can download and share hidden. Since `hops` 0.2 it can also be a hop when its NAT
is cone (§4.2).
