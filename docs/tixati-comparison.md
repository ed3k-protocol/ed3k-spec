# Tixati compared with ed3k

**Status:** design note (2026-11-02). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §6.3, §9, §10.2, §10.4, §12.3 and §14.1, and
[`extensions/http-source.md`](../extensions/http-source.md),
[`extensions/ip-diversity.md`](../extensions/ip-diversity.md),
[`extensions/hops.md`](../extensions/hops.md). Section numbers (§) refer to `Protocol.md`.

Tixati is a closed-source BitTorrent client with its own DHT work, I2P support and "channels"
(chat rooms with shared files and moderation). Its change log is long and detailed, so it shows
which problems a mature DHT client met in practice. This note checks each networking change
against ed3k.

**Result:**

- One gap: a node whose own network is down drops its whole routing table (§2.1); closed in
  `Protocol.md` §10.2.
- Three small gaps: port mapping lifecycle, web sources versus peers, proxies (§2.2–§2.4).
- Two Tixati features considered and not adopted: multi-path lookups, random port per start (§3).
- Everything else is covered, mostly by design rather than by a later fix (§4).

**Method.** Release notes at <https://tixati.com/news> read on 2026-11-01, versions 1.85
(2012-04) to 3.44 (2026-06-21). Only networking, protocol and security items are taken; GUI
fixes are left out. Tixati's source is closed, so statements about it are what its notes say,
not a trace of code.

## 1. Overview

| | Tixati | ed3k |
|---|---|---|
| Transport | BitTorrent TCP/uTP, optional MSE encryption; I2P over I2CP (3.34) | QUIC with obfuscation only (§4) |
| File hash | v1 SHA-1, v2 SHA-256 (3.11), own "v3" against SHA-1 weakness (3.39) | BLAKE3, 256 KiB blocks, each verified alone (§7) |
| DHT | Mainline DHT; rewritten 3.34, 3.39, 3.43; anti-Sybil heuristics, multi-path lookups (3.43) | Kad over QUIC; IP-bound routing ID (§6.3), `ip-diversity` limits, minimum walk M (§10.4) |
| Store auth | announce tokens; replay detection added 3.41 | store rides an address-validated QUIC connection; no token |
| NAT | UPnP, NAT-PMP, PCP (3.43); `ut_holepunch` | port mapping first (§9.1), then `ReachTest`, relays, hole punching (§9.3–§9.5), required |
| Anonymity | I2P, separate DHT table and session for I2P-only transfers (3.39) | `hops` extension with anonymous keys |
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
scope and that anonymity is `hops`' job, so implementers don't add a half-working mode.

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

## 4. Covered in ed3k

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

## 5. Open points

- Decide on the notes of §2.2–§2.4.
