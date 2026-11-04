# Tixati and DarkMX compared with ed3k

**Status:** design note (Tixati 2026-11-02, DarkMX 2026-11-03). Informative; changes nothing in
wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §6.3, §9, §10.2, §10.4, §11, §12.3 and §14.1, and
[`extensions/http-source.md`](../extensions/http-source.md),
[`extensions/ip-diversity.md`](../extensions/ip-diversity.md),
[`extensions/hops.md`](../extensions/hops.md), [`extensions/room.md`](../extensions/room.md),
[`extensions/browse.md`](../extensions/browse.md),
[`transport-bindings.md`](transport-bindings.md), [`browser-bindings.md`](browser-bindings.md).
Section numbers (§) refer to `Protocol.md`, except "§N" pointing at a heading of this note.

Tixati and DarkMX come from the same author (Kevin Hearn); DarkMX releases are signed with a key
hosted at tixati.com. Both are closed source, so statements about them are what their own pages
say, not a trace of code.

- **Tixati** (§1–§4) is a BitTorrent client with its own DHT work, I2P support and "channels"
  (chat rooms with shared files and moderation). Its change log is long and detailed, so it
  shows which problems a mature DHT client met in practice.
- **DarkMX** (§5) is a chat and file sharing app that runs only over Tor onion services, with
  I2P as an upgrade. Users host "channels": an onion service holding a searchable file index,
  chat rooms and links to other channels. Its lessons are about anonymity transports and hubs,
  not about the DHT.

**Result:**

- Tixati: one gap, a node whose own network is down drops its whole routing table (§2.1);
  closed in `Protocol.md` §10.2.
- Tixati: three small gaps, port mapping lifecycle, web sources versus peers, proxies
  (§2.2–§2.4).
- Tixati: two features considered and not adopted, multi-path lookups, random port per start
  (§3). Everything else is covered, mostly by design rather than by a later fix (§4).
- DarkMX: one larger gap, no binding over an existing anonymity network (§5.2.1), with a
  per-connection upgrade between bindings (§5.2.2). Four smaller ones: graded room roles,
  a directory of rooms and index nodes, index nodes keeping a client's records across
  reconnects, reach from a plain Tor Browser (§5.2.3–§5.2.6).
- eMuleQt already runs BitTorrent over I2P (§6); ed3k and eD2K traffic does not use it.

**Method.** Tixati: release notes at <https://tixati.com/news> read on 2026-11-01, versions 1.85
(2012-04) to 3.44 (2026-06-21). DarkMX: guides at <https://darkmx.app/support> read on
2026-11-03, version 1.42 (2025-06-14): *How does DarkMX work*, *Networks*, *Share Groups*,
*Access Levels*, *I2P Network*. Only networking, protocol and security items are taken; GUI
fixes are left out. DarkMX documents no crypto, identity model or wire format of its own.

## 1. Tixati overview

| | Tixati | ed3k |
|---|---|---|
| Transport | BitTorrent TCP/uTP, optional MSE encryption; I2P over I2CP (3.34) | QUIC with obfuscation only (§4) |
| File hash | v1 SHA-1, v2 SHA-256 (3.11), own "v3" against SHA-1 weakness (3.39) | BLAKE3, 256 KiB blocks, each verified alone (§7) |
| DHT | Mainline DHT; rewritten 3.34, 3.39, 3.43; anti-Sybil heuristics, multi-path lookups (3.43) | Kad over QUIC; IP-bound routing ID (§6.3), `ip-diversity` limits, minimum walk M (§10.4) |
| Store auth | announce tokens; replay detection added 3.41 | store rides an address-validated QUIC connection; no token |
| NAT | UPnP, NAT-PMP, PCP (3.43); `ut_holepunch` | port mapping first (§9.1), then `ReachTest`, relays, hole punching (§9.3–§9.5), required |
| Anonymity | I2P, separate DHT table and session for I2P-only transfers (3.39) | `hops` extension with anonymous keys; eMuleQt runs torrents over I2P (§6) |
| Group features | channels: chat, shared files, permissions, bans, encrypted private messages | `room`, `pubsub`, `chat`, `browse` |

## 2. Gaps found in ed3k

### 2.1 Losing the network empties the routing table

Tixati 1.99 refreshes its table after suspend; 3.39 lists "better recovery after internet
disconnects".

ed3k §10.2 drops a contact after two failed attempts in a row, whatever the cause. A node whose
own uplink is down, or that wakes from sleep with a stale table, sees every attempt fail. The
background maintenance of §10.2 reaches each contact within the hour, the second attempt 5 min
later; after an outage of that length the table is empty and the node must bootstrap again
([`extensions/bootstrap.md`](../extensions/bootstrap.md)). Losing long-lived contacts also costs
Sybil resistance: old, proven contacts are what an attacker can't fake quickly.

Until then nothing in the spec told "my network is down" from "this contact is gone".

**Applied (§10.2, "Own network down"):** a node goes offline in a family once, for 2 min
*(initial)*, no connection completed and no datagram arrived there, while dials or requests to 4
*(initial)* different sites failed since the last one. While offline it counts no failed attempt
and drops no contact; failures since that last arrival are void. When it comes back, and after an address change
or resume from sleep (a timer more than 2 min late), it runs the maintenance of §10.2 on all
buckets at once, spread over 5 min *(initial)*. Constants in Appendix A.

### 2.2 Port mapping lifecycle

Tixati 2.74 removes UPnP/NAT-PMP mappings on close, with lease TTL as fallback; 3.43 rewrote the
engine, added PCP, a fallback request when PCP refuses a port, mapping over VPN and a monitor.

§9.1 only says a node SHOULD try PCP, NAT-PMP or UPnP IGD. Lease length, renewal and removal on
exit are left out. They matter to ed3k more than to a TCP client: a dead mapping with a stale
external port makes §9.2 advertise an endpoint that no longer forwards, until the next
`ReachTest` (up to 60 min).

**Proposal:** a short informative note in §9.1: lease ≤ 2 h, renew at half lease, delete on
clean exit, run a `ReachTest` at once when a mapping is lost or its external port changes.

### 2.3 Web sources versus peers

Tixati 3.37 adds limiter presets for web seeds: delayed start, and stop when peer bandwidth
already exceeds a cutoff.

[`extensions/http-source.md`](../extensions/http-source.md) §5 limits concurrent fetches and
duplicate ranges but has no rule to prefer peers. A popular file then loads the HTTP server even
when the swarm alone is fast.

**Proposal:** a SHOULD in `http-source.md` §5: fetch a range from HTTP only when no peer can
serve it within a short wait, or while the download is below a node-set speed.

### 2.4 Proxies

Tixati 2.74 runs DHT, UDP peers and UDP trackers through SOCKS5 proxies that support UDP
ASSOCIATE; 2.68 has separate proxy settings per traffic kind.

ed3k has no proxy binding. QUIC can run over SOCKS5 UDP ASSOCIATE, but address observation
(§9.2), routing IDs (§6.3) and hole punching (§9.5) then see the proxy's address. A proxied node
can only be a client: no `ROUTER`, no source records naming its address.

**Proposal:** state in [`transport-bindings.md`](transport-bindings.md) that proxies are out of
scope and that anonymity is the job of `hops` or of a binding over Tor or I2P (§5.2.1), so
implementers don't add a half-working mode.

## 3. Considered and not adopted

### 3.1 Multi-path lookups

Tixati 3.43: "rewritten search algorithm with multi-path lookups", plus heuristics against
Sybil floods.

Already discussed in [`extensions/ip-diversity.md`](../extensions/ip-diversity.md) §5 (disjoint
paths, S/Kademlia): each extra path multiplies lookup cost by about 2.5 KB per queried node over
QUIC (§10.5). Left for the datagram binding
([`extensions/kad-datagram.md`](../extensions/kad-datagram.md)). Tixati's change confirms the
threat is real on Mainline DHT, where IDs are free; ed3k's IP-bound routing ID, diversity limits
and minimum walk M cover much of what multi-path buys there.

### 3.2 Random incoming port per start

Tixati 3.37 option. In ed3k it breaks the port rule (§14.1: ed3k UDP port = advertised eD2K TCP
port) for dual-protocol nodes. It doesn't change the routing ID (§6.3 binds the address, not the
port), but Kad contacts and source records stored by others name the old port until they expire
(up to 5 h). No gain against observers: QUIC obfuscation (§4.2) already hides the protocol, and a
fixed node ID links the node across ports anyway.

### 3.3 Peer ID rotation

Tixati 2.33 changes its BitTorrent peer ID every 6–36 h; 3.39 forces a new one when switching
between I2P-only and I2P+Internet. ed3k's node ID is a long-lived key on purpose (credits,
friends, signed records). Unlinkability where wanted comes from `hops`' anonymous keys
([`extensions/hops.md`](../extensions/hops.md) §1), which plays the role of Tixati's separate I2P
session.

## 4. Tixati changes covered in ed3k

| Tixati change | ed3k |
|---|---|
| v2 SHA-256 block hashing, per-block check (3.11); "v3" against SHA-1 (3.39) | BLAKE3 from the start, each 256 KiB block checked alone (§7.2); v1, v2 and hybrid torrents mapped in [`torrent`](../extensions/torrent.md) |
| Bad-block peers deprioritized, then ignored (2.63) | ban after 2 bad blocks in 24 h (§12.3) |
| Piece repair over multi-source permutations (2.58) | not needed: a bad block names its sender at once |
| DHT announce token replay detection, mismatched tokens (3.41) | no tokens: store runs over a QUIC connection whose address QUIC validated (§10.3) |
| Reflected port detector from DHT replies, NAT type (3.43) | `observed` in `Hello`, counted per site, the lead rule (§9.2) |
| IPv6 hole punching fixes (3.39) | one procedure for both families; stateful IPv6 firewall treated like NAT (§9) |
| DHT flood / clone protection, rate limits (1.97, 2.48) | [`flood-protection`](../extensions/flood-protection.md), per-node-ID rates (§4.6) |
| Anti-Sybil heuristics (3.43) | IP-bound routing ID (§6.3), [`ip-diversity`](../extensions/ip-diversity.md), [`key-puzzle`](../extensions/key-puzzle.md) (draft) |
| Duplicate peer-ID detection (3.11) | node ID proven by the handshake; at most one connection per peer and family (§4.6) |
| Private torrents not visible in DHT (3.39 v3.1) | [`hidden-set`](../extensions/hidden-set.md) |
| Separate DHT table for I2P-only (3.39) | `hops` paths over the one Kad; no second network |
| Channels: chat, bans, ignore lists, admin commands (2.76, 2.89) | [`room`](../extensions/room.md): owner, moderators, signed admits and bans |
| Channel private messages encrypted to recipient key (2.53) | [`chat`](../extensions/chat.md) |
| Channel shares, browse (2.89) | [`browse`](../extensions/browse.md), [`file-set`](../extensions/file-set.md), [`pubsub`](../extensions/pubsub.md) |
| Encrypted local discovery for channels (1.92) | [`lan-discovery`](../extensions/lan-discovery.md) |
| IP filter lists, per-list switch (3.35) | not an IP list by design: bans only from proven misbehaviour (§4.6, §12.3); file filtering via [`filter-list`](../extensions/filter-list.md) |
| Network-coded live streams with homomorphic signatures (1.85) | [`live`](../extensions/live.md): broadcaster signs a record per segment, peers forward it unchanged; no network coding |
| Gzip/Brotli for WebUI and trackers (3.17, 3.44) | zstd for messages (§5.3); WebUI is outside the protocol |
| Piece sizes up to 256 MB (2.86) | fixed 256 KiB blocks; tree size grows, not block size (§7) |

## 5. DarkMX

### 5.1 Overview

| | DarkMX | ed3k |
|---|---|---|
| Transport | Tor onion services only; I2P over I2CP with own streaming layer, as an upgrade (§5.2.2) | QUIC with obfuscation only (§4) |
| Reachability | no open port needed: Tor carries everything | port mapping, `ReachTest`, relays, hole punching (§9) |
| Anonymity | every connection, by the transport | optional, `hops` (0–3 nodes, own design, not yet reviewed) |
| Connections | one TCP connection per peer pair for transfers, browse, indexing | one QUIC connection, streams per task (§4) |
| Flow control | in-flight data capped by measured latency, against Tor buffering | QUIC congestion control |
| File identity | location, no hash; huge libraries load fast | BLAKE3 content hash (§7) |
| Downloads | one source, in order; source chaining only for some incomplete files | many sources, any block order, each block checked alone (§7.2) |
| Search | channel host word-indexes the compressed file lists its users upload | Kad keywords (§10) or index nodes (§11) |
| Discovery | "gather" asks connected channels for known channels; "deep gather" visits all | link, name ([`name`](../extensions/name.md)) or Kad target; no directory |
| Hub hosting | owner runs the channel on own computer, as an onion service | room carried by members and Kad ([`room`](../extensions/room.md)); index node is any node with `INDEX` |
| Roles | Owner, Manager, Op, Moderator, VIP/Star/Trusted, Normal, Basic, Silent, Banned | owner (room key), moderator, member, banned |
| Sharing scope | per-item tick per contact group; groups tied to channels | `browse`: everyone, friends only or nobody |
| Web | text site as onion service, opens in plain Tor Browser | [`browser-bindings.md`](browser-bindings.md): clearnet browsers only |

### 5.2 Gaps found in ed3k

#### 5.2.1 No binding over an existing anonymity network

DarkMX gets anonymity, NAT traversal and censorship resistance from Tor, a network with years
of review and thousands of relays. ed3k's only anonymity is `hops`, whose key agreement and
ciphers are a first design marked as needing professional review. `transport-bindings.md` §6
lists TLS over TCP, WebTransport, WebRTC and in-memory as candidates, but not Tor or I2P.

What each would bring, against the contract (T1–T5) of `transport-bindings.md` §1:

| | I2P | Tor onion service |
|---|---|---|
| Access | SAM v3 (streams and datagrams) or I2CP to a local router | control port or embedded Tor; streams only |
| Datagrams | yes (repliable and raw) | none; needs the TLS-over-TCP binding with a stream multiplexer first |
| Address | destination (Ed25519 signing key in current types) | v3 onion address = an Ed25519 public key |
| T1 key proof | destination key proves the endpoint, not the node key: node key still proved inside, as on `quic-obfs` | likewise |
| Kad | own family of endpoints; no IP-bound routing ID (§6.3) possible: needs another Sybil cost ([`key-puzzle`](../extensions/key-puzzle.md)) | likewise |
| Router at hand | eMuleQt already ships i2pd for torrents (§6) | not shipped |

The endpoint key MUST NOT be the node key: that would tie the anonymous endpoint to the node's
clearnet identity. Like `hops`' anonymous keys, a node uses a separate key per binding, or per
session.

**Proposal:** add I2P (SAM) and Tor as rows of `transport-bindings.md` §6, with the two
difficulties above (no IP-bound routing ID; Tor has no datagrams). State that `hops` and such a
binding are alternatives: a node on I2P needs no ed3k hops.

#### 5.2.2 Upgrade between bindings on a live connection

Every DarkMX connection starts over Tor. A node with I2P on says so over that connection; if
the peer has I2P too, it dials I2P while the Tor connection keeps working. On success I2P takes
over and Tor is closed; on failure it retries now and then. The peer's destination is cached, so
the next connection tries I2P first.

ed3k does the same kind of thing within one binding (relay, then hole punching, §9.4–§9.5) but
has no way to say "I am also reachable on binding X". That is the addressing gap of
`transport-bindings.md` §2.

**Proposal:** when §2 of that note is closed, the endpoint list SHOULD be sendable on a live
connection (not only in `Hello`), and a node MAY move a connection to a binding both prefer, then
cache the peer's endpoint on it. The preference order (anonymous before fast, or the reverse) is
local policy, but a node MUST NOT move from an anonymous binding to a clearnet one: that would
reveal its address to a peer that knew it only by an anonymous endpoint.

#### 5.2.3 Graded room roles

DarkMX has nine levels. A user changes only users below their own level; moderators can't change
each other; owners can change owners; an owner who reconnects with the private key is owner
again.

`room.md` has the room key (owner), moderators and members, plus admit and ban. Missing:

- **mute** (DarkMX *Silent*): member stays and reads but posts are dropped. Today only a ban
  does that, and a ban in a closed room also costs a key change.
- **read but no shares** (DarkMX *Basic*), for rooms that carry file links.
- a level between owner and moderator that may change room info but not hold the room key.

**Proposal:** a `Mute` entry next to `Ban` in room info, signed by the room key or a moderator,
checked by every member like a ban. The other two wait until a room needs them.

#### 5.2.4 No directory of rooms or index nodes

DarkMX channels list other channels; a client gathers them in under a minute after start. ed3k
finds a room, a set or an index node only by link, name or Kad target, so a new user has nothing
to browse.

**Proposal:** a signed record, kind TBD, listing up to N room links and index node endpoints a
key recommends, renewed like any signed record (§10.7) and followable via
[`pubsub`](../extensions/pubsub.md). A client merges the lists of keys it trusts; nothing is
trusted by default. Not a separate network: just a set whose members are links.

#### 5.2.5 Index node forgets a client's records with their lifetime

A DarkMX channel keeps a user's file list on disk for a set number of days after the user left;
on return only the changes go up. 100,000 files take about 5 min the first time.

At an ed3k index node (§11) records outlive the connection only until their lifetime ends, and
the client renews each one. A client offline longer than a lifetime stores its whole library
again.

Not for `SOURCE`: an offline node is a dead source. For keyword records it is worth a look.

**Open:** an optional index-node rule to keep a client's keyword records for days, marked
"publisher offline", plus a short summary (hash of the published set) the client compares on
reconnect to send only the difference. Needs a message; no proposal yet.

#### 5.2.6 Reach from a plain Tor Browser

A DarkMX text site opens in Tor Browser without DarkMX. `browser-bindings.md` covers clearnet
browsers only. Depends on §5.2.1: once a node has a Tor endpoint, an HTTP gateway on it is the
same gateway as for clearnet.

**Proposal:** one paragraph in `browser-bindings.md` naming onion services as a host for its
gateway, once a Tor binding exists.

### 5.3 Considered and not adopted

- **Files identified by location, no hash.** Makes large libraries cheap, but gives no
  integrity, no multi-source download and no dedup. ed3k hashes once and keeps the hash,
  so the cost is paid at share time only.
- **Single-source, in-order downloads.** Follows from the line above.
- **Every connection anonymous.** Tor circuits add seconds per connection and limit speed; most
  ed3k users share in the open. Anonymity stays a choice per node.
- **Hub hosted on the owner's computer.** The channel is gone while the owner is offline. ed3k
  rooms live in their members and Kad.
- **No moderation of the open network.** DarkMX users report fake files. ed3k has
  [`filter-list`](../extensions/filter-list.md) and signed records.

### 5.4 DarkMX features covered in ed3k

| DarkMX | ed3k |
|---|---|
| One TCP connection for all tasks between two peers | one QUIC connection, a stream per task (§4) |
| In-flight cap from measured latency | QUIC congestion control; `hops` paths add nothing new here |
| Compressed file list sent slowly | zstd for messages (§5.3); publish within the budget of §4.6 |
| Channel accepts listings only from approved users | index node admission policy, `Error{DENIED}` (§11) |
| Owner proven by holding the private key | room key ([`room`](../extensions/room.md)) |
| Ban blocks the user from the channel | signed bans in room info, checked by every member |
| Share groups (who sees which item) | `browse` friends-only; finer groups are local policy, no wire change |
| I2P destination cached after first success | nothing to cache: one binding (until §5.2.2) |

## 6. eMuleQt: torrents over I2P

eMuleQt (2026-11-03) runs BitTorrent over I2P, as Tixati 3.34 does, but not ed3k or eD2K:

- libtorrent talks to an I2P router over SAM: the bundled i2pd or an external router.
- Per torrent: I2P only (no DHT, LSD, clearnet trackers or peers) or mixed. Default for new
  torrents is a setting; a `.torrent` with an `.i2p` tracker becomes I2P automatically.
- A setting decides whether an I2P torrent's info hash also goes out over ed3k (torrent join,
  [`torrent`](../extensions/torrent.md)). On by default. While on, ed3k shows on the clearnet
  that this node wants that torrent, so a user who chose I2P for privacy should turn it off;
  the GUI should say so.

The bundled router is what §5.2.1 needs for an I2P binding of ed3k: the same SAM connection could
carry ed3k streams and datagrams.

## 7. Open points

- Decide on the notes of §2.2–§2.4.
- Decide whether to add I2P and Tor to `transport-bindings.md` §6 (§5.2.1) and the upgrade rule
  (§5.2.2).
- Decide on `Mute` in `room.md` (§5.2.3) and on a directory record (§5.2.4).
- Index node retention and difference sync (§5.2.5): design a message first.
- eMuleQt: warning text for the I2P-torrent ed3k join (§6).
