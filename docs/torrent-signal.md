# Signalling between ed3k and BitTorrent: no capability bit, and what a BEP would give

**Status:** design note (2026-10-12). Informative; changes nothing in wire version 1.
**Relates to:** [`extensions/torrent.md`](../extensions/torrent.md) and [`Protocol.md`](../Protocol.md) §2, §5.5 and §14. Section numbers (§) refer to `Protocol.md` unless another document is named.

A client that downloads a torrent from BitTorrent peers and ed3k nodes at once
([`torrent.md`](../extensions/torrent.md)) wants to learn two things about the peers it meets:

- of a BitTorrent peer: does it also speak ed3k, and under which node ID?
- of an ed3k node: which BitTorrent peers of this torrent does it know?

**Result:**

- **No capability bit** for torrents in `Hello`. It would switch nothing on (section 1).
- Signalling **in both directions** makes most sense with a BitTorrent enhancement proposal
  (BEP) by which a BitTorrent peer says that it speaks ed3k (section 2). It does not exist; this
  note sketches it.
- The other direction, BitTorrent peers passed on over ed3k, is a field in the file stream. A
  capability bit would come with that field, not before (section 3).

## 1. Decided: no capability bit

A bit `BT` in `Hello.caps` was weighed for the torrent extension and left out.

| What the bit might be for | Why it does not do it |
|---|---|
| Knowing that a node can serve the manifest, the info dictionary or a member | all three are ordinary files, asked for by root. Every node serves them, with or without the extension |
| Using a peer over one network only | that needs to know that the BitTorrent peer at an address *is* ed3k node X. A bit in an ed3k `Hello` does not say it; an identity in the BitTorrent handshake does (section 2) |
| Finding more sources | `caps` travels in `Hello` only (§2). The bit is seen after the connection is made |
| Information, as `ED2K` (bit 6) | `ED2K` goes with the port rule of §14.1, which gives it a use. Nothing would read `BT` |

| | Pro | Con |
|---|---|---|
| **No bit** | nothing in `Hello` that tells every ed3k peer the node also runs BitTorrent; one bit less to explain | none while no message depends on it |
| **A bit** | free on the wire: bit 16 fits the 3 bytes that bits 14 and 15 already cost | says something about the user and gives nothing back |

Between two ed3k nodes the torrent extension changes nothing on the wire: one field in a
manifest, one record kind in Kad, one parameter in a link. There is no behaviour for a bit to
announce.

## 2. From BitTorrent to ed3k: a BEP

### 2.1 What is missing

Two clients that both speak ed3k can meet in a swarm and never learn it. They then trade the
torrent over BitTorrent, with pieces that span files, verification by piece and no way through
a NAT that BitTorrent does not offer. Each would be a better source for the other over ed3k.

ed3k has this problem with eD2K too, and solves it without changing eD2K: the port rule and
the derived user hash (§14.1, §14.2). BitTorrent has no field that could carry such a hidden
sign. Its peer ID is 20 bytes chosen per torrent and its port is free, but nothing ties either
to a second protocol. An open sign is needed, and BitTorrent has a place for one.

### 2.2 The sketch

The extension protocol of BitTorrent (BEP 10) starts with a handshake: a bencoded dictionary
that both sides send once per connection. Keys a client does not know are ignored.

| Key | Value | Meaning |
|---|---|---|
| `ed3k` | 18 bytes: node ID (16) ‖ UDP port (2, big-endian) | I speak ed3k under this node ID, on this port of the address you see me at |
| `ed3k_root` | 32 bytes, optional | the torrent root of this torrent ([`torrent.md`](../extensions/torrent.md) §2), if I have computed or verified it |

- **The address** is the one of the BitTorrent connection. A node behind a NAT is dialled
  through the punch of §9.5 if the port does not answer, by its node ID.
- **The node ID is enough to dial.** Its first 12 bytes are the obfuscation key of §4.3, as
  with the user hash of §14.2.
- **The sign is a claim.** It is proven by the ed3k connection: QUIC authenticates the node
  ID (§6.2). A peer that names a node ID it does not hold gains nothing.
- **`ed3k_root` is a claim too,** of the same weight as a `TORRENT` record
  (`torrent.md` §4.2). It saves the Kad lookup, and it comes from a peer that provably has
  the torrent.
- **Private torrents:** a client MUST NOT send either key on a connection for a torrent that
  is marked private (`torrent.md` §6.3).

No new message type is needed. A proposal would be one page.

### 2.3 What it gives

| | Without the BEP | With it |
|---|---|---|
| Two ed3k clients in one swarm | trade over BitTorrent | move to ed3k and drop the BitTorrent connection (`torrent.md` §6.1) |
| A start from a magnet link | finds the torrent root through Kad, if someone published it | learns it from the first peer that has it |
| A start from a torrent root | finds ed3k sources through Kad only | also among the BitTorrent peers of the swarm |
| A file of the torrent alone | needs the pieces that reach into its neighbours | asked for by its own root |
| Counting a peer once | not possible: the same node may serve over both | the node ID joins the two |

### 2.4 What it costs

| Pro | Con |
|---|---|
| one key in a handshake that exists; legacy clients ignore it | **it is an open sign.** The handshake is readable unless the connection uses BitTorrent's stream encryption, and every peer of the swarm receives it. Anyone who joins a swarm can list the ed3k nodes in it, with address, port and node ID |
| needs no change to ed3k | ed3k hides that a node speaks it (§4.2). This sign gives that up for every swarm the node is in |
| the claim is proven by the ed3k connection | the node ID ties the user's torrents to the user's ed3k identity |
| | a BEP that only ed3k clients implement signals between them only. That is the purpose, but it is not a gain for BitTorrent as such |

A client that implements it SHOULD let the user switch it off, and MUST NOT send it in a closed
network (§4.3).

### 2.5 Alternatives to an open key

| Alternative | Pro | Con |
|---|---|---|
| **A sign in the peer ID**: some of its 20 bytes derived from the node ID | needs no extension; seen before the handshake, also from a tracker | the peer ID carries the client's name and version by convention, and trackers and peers read it. 12 bytes of it would have to be taken |
| **The port rule of §14.1**, with the BitTorrent port in place of the eD2K port | no byte on the wire | says nothing about the node ID, so nothing can be dialled obfuscated; BitTorrent ports are often random per session |
| **An extension message** after the handshake, sent only to peers that named the extension | the node ID goes only to peers that know ed3k | the name of the extension in the handshake is the same open sign |
| **Nothing in BitTorrent**; the `TORRENT` record and `SOURCE` records in Kad only | ed3k stays hidden from the swarm | two ed3k clients in a swarm meet only if both published, and then by a detour |

## 3. From ed3k to BitTorrent: peers in the file stream

The opposite direction needs no BEP. An ed3k node that shares a torrent set knows BitTorrent
peers of that torrent and could pass them on.

- A download that started from a torrent root has no tracker addresses
  (`torrent.md` §6.2) and finds its BitTorrent peers through the DHT of BitTorrent alone.
- A field in `FileRequest` and `FileStatus` on the stream of the manifest would carry them:
  compact endpoints, as BitTorrent's own peer exchange does.

| Pro | Con |
|---|---|
| the first BitTorrent peers come from a node that provably has the torrent | a node hands out addresses of third parties who did not ask for it. Only peers the node itself was connected to may be named, and few of them |
| closes the gap that the missing trackers leave | an address list from a stranger is a way to make clients dial a victim. The limits of failed dials apply ([`ip-diversity.md`](../extensions/ip-diversity.md) §2.5) |

**This is where a capability bit would belong.** It would say whom to ask and to whom such a
list is worth sending. If the field is added, the bit (16, `BT`) is added in the same step.

## 4. Why the BEP is the better half

| | BEP (section 2) | Peers over ed3k (section 3) |
|---|---|---|
| Who gains | every pair of ed3k clients that meet in a swarm | a client that starts from a torrent root |
| What moves | traffic from BitTorrent to ed3k, with verification by block and single files | nothing; BitTorrent gets more peers |
| Joins the two identities of a node | yes | no |
| Carries the torrent root | yes | not needed, the client has it |
| Needs agreement outside ed3k | a BEP number at best; two clients that implement the key suffice | no |

With the BEP, the second direction is partly covered as well: a client that learned through
BitTorrent that a peer speaks ed3k has an ed3k source, and ed3k's own source exchange
(§12.1) then passes ed3k sources on.

## 5. Open points

- Whether the open sign is acceptable at all, given §4.2. A client could send it only on
  connections that use BitTorrent's stream encryption; that hides it from an observer on the
  path, not from the peers of the swarm.
- The same key in the BitTorrent DHT or in a tracker announce was not looked at.
- The field of section 3 is not specified: its place, the number of peers, and how a node
  proves that it was connected to a peer it names.
