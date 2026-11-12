# Signalling ed3k to eD2K peers: signals, explicit tags, alternatives

**Status:** design note (2026-10-11, merged 2026-11-13 from `ed2k-signal-comparison.md` and
`ed2k-tags.md`). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §14 (coexistence with eD2K). § = that document;
"section N" = this one.

A client that speaks both eD2K and ed3k wants eD2K peers to learn that it can also be reached
over ed3k, best before any connection. This note records the signals §14 uses (section 2), the
explicit tags proposed on top (sections 3 to 5), a capability bit kept as an idea (section 6),
the signals that were turned down (section 7), eD2K callbacks as a hole-punch signal
(section 8), how an eD2K server with an index node lists ed3k nodes (section 9), and built-in
seed node links for a network without routers yet (section 10).

**Result:**
- The port rule (§14.1) and the derived user hash (§14.2) are the signal that needs no
  connection. No other signal passes legacy servers and legacy relaying clients unchanged and
  also carries the node ID.
- Explicit tags are an optional, authoritative addition outside the specification: tag ID
  `0xE3` for the hello, server login, server ident, server description and extended source
  exchange; string name `"e3"` in Kad; token `b3=` in links. All unused in the eMule source
  trees checked.
- A capability bit (bit 11 of `CT_MOD_MISCOPTIONS`) is an idea only: it says "ed3k" after a
  hello, carries no node ID, and could gate the hello tag (section 6).
- The rendezvous of an eD2K server can serve as the signal for an ed3k hole punch; the Kad
  buddy callback cannot (section 8).

## 1. What the signal has to do

- **Arrive before a connection.** A source is learned from a server, from source exchange, from
  Kad or from a link. The signal should come with it.
- **Survive legacy relaying.** Servers and clients that know nothing of ed3k pass sources on.
  Whatever they rebuild, mask or drop is lost.
- **Change nothing a legacy client or server interprets.**
- **Carry enough to dial.** The endpoint, and the first 12 bytes of the node ID for the
  obfuscation key (`obf_id`, §4.3). Without them no `PROBE` or handshake can be built.
- **Cost little when wrong.** A false signal should cost one unanswered handshake.

## 2. What §14 uses, and what a tag adds

| Signal | Reaches | Carries | Weakness |
|---|---|---|---|
| Port rule (§14.1) | every source record, since each has the TCP port | the ed3k UDP port | says nothing about whether the peer speaks ed3k |
| Derived user hash (§14.2) | every channel that carries the user hash, also a server (`OP_LOGINREQUEST`) | 12 bytes of the node ID | a hint: one legacy hash in 65,536 matches by accident; the user hash changes once, which loses credits at other peers |
| Explicit tags (section 3), optional | hello, Kad source publish, extended source exchange, server login, links | the wire version or the full node ID; authoritative | not relayed by legacy servers or by legacy source exchange |
| Capability bit (section 6), idea | hello only | one bit | after a TCP hello only; no node ID |

The user hash test needs the 2 check bytes: bytes 5 = 14 and 14 = 111 are set on **every**
classic eMule user hash, not only on derived ones. A client of both networks that keeps its
legacy hash is invisible to §14.2. It can still dial a peer whose hash is derived, and that
peer asks it for contacts while its own table is small (`extensions/bootstrap.md` §1 source 4).

## 3. The explicit tags

A tag from the list below means "ed3k version `n`, port rule of §14.1".

| Channel | Signal | Note |
|---|---|---|
| Client hello (`OP_HELLO`, `OP_HELLOANSWER`) | tag `CT_ED3K` = `0xE3`, HASH type, value = the full 16-byte node ID | see section 5 for a client that penalises unknown hello tags |
| Kad source publish | string-named tag `"e3"`, UINT8 = wire version | stored and returned verbatim by legacy nodes, like `"ip6"` |
| Extended source exchange record | tag `CT_ED3K` = `0xE3`, UINT8 = wire version | only to peers that skip unknown tags |
| Server login (`OP_LOGINREQUEST`) | tag `CT_ED3K` = `0xE3`, HASH type, value = the node ID | lets an ed3k-aware server mark sources, pair them with the node's ed3k connection (section 9) and advertise an index node |
| Server ident (`OP_SERVERIDENT`) | tag `ST_ED3K` = `0xE3`, string, value = the server's `ed3k:node?…` link (§13.2) | how a client learns of a server's index node (§11) |
| Server description (UDP `OP_SERVER_DESC_RES`, the extended form with challenge) | tag `ST_ED3K` = `0xE3`, string, the same link | reaches a client for every server in its list without a login, and other servers that read the reply |
| eD2K link | token `b3=<root, base32 lower case, no padding>`, the parameter name of `ed3k:file` links (§13.2) | MUST be placed before the `sources` block, where legacy parsers stop reading |

**Binding the identities.** The hello tag ties the eD2K identity to the ed3k one after an eD2K
connection: if secure identification passed and the tag's node ID yields the peer's user hash
under §14.2, a client MAY carry the peer's eD2K credits over to the node ID.

## 4. Check against existing clients

The numbers were checked against the source of stock eMule 0.50a and 0.51d, eMuleQt and four
eMule mods (2026-10-11). Every `CT_`, `ET_` and `ST_` definition was collected, including the
lists of tags that some clients treat as a mark of a leecher mod.

| Signal | Result |
|---|---|
| `0xA1`, the number first proposed | **taken.** One mod sends `0xA1` in the hello as a HASH tag with its 16-byte Kad ID. The type and length are the same as `CT_ED3K`, so a node ID would be read as a Kad ID, and the other way round |
| `0xA0`–`0xAF` | not usable: a family of mod tags, partly assigned and partly reserved by their owner |
| `0xE3` | free in all trees checked, as a client tag, an eMule info tag and a server tag. Not on any leecher-tag list. Also free in the server-tag and login-tag space of the server implementations checked (2026-10-21) |
| `"e3"` as a Kad tag name | unused; the string names in use are `"ip"`, `"port"`, `"ip6"`, `"bi6"` and the five HTTP Cache names |
| `b3=` as a link token | unused; the tokens in use are `p=`, `h=`, `s=` and `s6=` |

Other numbers free in every tree, should `0xE3` turn out to be taken elsewhere: `0xC0`–`0xC3`,
`0xC5`, `0xC6`, `0xDB`–`0xE2`, `0xE4`, `0xE5`, `0xE7`–`0xEB`.

`0xE3` is also the eD2K protocol byte. That is a different field and no parser confuses the
two; the number was chosen because it is easy to recognise in a packet dump.

## 5. Unknown hello tags are not always harmless

Stock eMule ignores a hello tag it does not know. Two of the checked mods do more:

- One keeps a list of tag numbers used by known leecher mods and bans their senders. `0xE3` is
  not on it.
- One treats **any** unknown hello tag as a sign of a hidden mod when the sender announces no
  readable mod name, and penalises the sender.

A client that sends `CT_ED3K` in the hello SHOULD therefore also send a mod version string
(`CT_MOD_VERSION`, `0x55`), or send it only to a peer that set the bit of section 6. The Kad,
source exchange, login and link signals are not affected.

## 6. Idea: an ed3k bit in the hello option word

**Status:** idea, not implemented, no number taken. Recorded so the bit is not used for
something else.

The hello option word `CT_MOD_MISCOPTIONS` (tag `0xAA`, uint32) is a capability bitfield
already sent in every hello by three clients. A bit there could say "this client runs an ed3k
node, at the port of §14.1".

**Where `0xAA` stands** (eleven client source trees read 2026-11-13):

| Clients | `0xAA` in the hello | Bits |
|---|---|---|
| one mod, the owner of the word | sent and read | 0 extended source exchange, 1 uTP NAT traversal, 2 IPv6, 3 serving-buddy pull, 4 QUIC NAT traversal; 5–31 reserved. A read word is kept as is |
| one client (current development tree) | sent and read | bits 0–4 as the owner; a read word is masked to `0x1F`, so any higher bit reads as absent |
| one client of both networks | sent in every hello, read | 0, 2, 4 as the owner; 5 extended source exchange skips unknown tags; 10 HTTP Cache. 6–9 left free on purpose, so the owner's run can grow |
| stock eMule 0.50a and 0.51d, and eight other trees | unknown | tag not defined; ignored by stock code (section 5 for the two mods that punish) |

**Proposed:** bit **11** (`0x800`), the next free bit after the HTTP Cache bit, outside the
range kept for the owner.

**What it does and does not give:**
- No new tag: clients that send `0xAA` now send one more bit in it. No client checked rejects
  a set reserved bit; one keeps it, one masks it off.
- One bit, no node ID. It cannot be dialled on: the first ed3k datagram needs the peer's
  `obf_id` (§4.3), which only a derived hash (§14.2) or `CT_ED3K` (section 3) gives.
- Known only after a TCP hello, as every hello option (section 7). Not relayed by source
  exchange, Kad or servers.

**Uses:**
1. **Gate for `CT_ED3K`.** A client answers a hello that has the bit with `CT_ED3K` in
   `OP_HELLOANSWER`, and sends `CT_ED3K` in `OP_HELLO` only to a peer seen with the bit before.
   No peer without ed3k ever sees an unknown hello tag (section 5). The side that learns the
   node ID dials; the handshake gives the other side the dialler's ID (§4.3). One direction
   is enough.
2. **A legacy user hash stays reachable.** A client of both networks that kept its legacy
   user hash is found through the bit and section 3, where §14.2 sees nothing.
3. **Counting.** A client can show how many of its eD2K peers run ed3k.

Before first use: confirm bit 11 with the authors of the two other clients that send the word.

## 7. Other signals considered

| Signal | Why not the primary one |
|---|---|
| Reserved bits 4–7 of the connect-options byte | Reaches Kad (`FT_ENCRYPTION`) and source exchange v4, but legacy clients rebuild that byte from their own flags when relaying, and servers mask it to 3 bits and use `0x80` for "hash follows" |
| Free bits 14–31 of `CT_EMULE_MISCOPTIONS2`, or a `CT_MOD_MISCOPTIONS` bit | Only visible after a TCP hello, and no node ID. The second is kept as an idea for other uses (section 6) |
| Free bits 3–7 of `TAG_KADMISCOPTIONS`, or a higher Kad version | Tells Kad contacts, not download sources |
| Fixed marker bytes in the user hash | Same one-time credit loss, but carries no node ID |
| A Kad record mapping user hash to node ID | A second lookup per source |
| Probing the port blindly | A wasted handshake against every legacy source |

## 8. eD2K callbacks as a signal for an ed3k hole punch

Two firewalled ed3k nodes meet through a relay (§9.4, §9.5). A dual-protocol client has a
second way to be told about a peer: the callbacks of eD2K. The eD2K rendezvous mechanisms hand
both sides a UDP endpoint and then need a stream transport of their own. Between two
dual-protocol clients, ed3k is that transport.

Described from the eMule sources, from a capture against a stock eD2K server (eserver 17.15,
2026-10-06) and from the published descriptions of the two newer mechanisms, read on
2026-10-12. Nothing was run against ed3k.

### 8.1 What a hole punch needs

| Need | In ed3k (`Punch`, §9.5) |
|---|---|
| The peer's mapped UDP endpoint, as someone outside saw it | `ep4`, `ep6` |
| The first 12 bytes of the peer's node ID, to key the first datagram (§4.3) | `peer_id` |
| Both sides are told | the relay forwards in both directions |
| The endpoint belongs to the socket ed3k uses | always |

### 8.2 The Kad buddy callback: not usable

| Packet | What the firewalled node learns | For ed3k |
|---|---|---|
| `KADEMLIA_CALLBACK_REQ` → buddy → `OP_CALLBACK` | the requester's address and TCP port | the node connects by TCP as today. The hello then shows the derived user hash, and both move to ed3k (§14.4). Nothing to add |
| `OP_REASKCALLBACKUDP` → buddy → `OP_REASKCALLBACKTCP` | the requester's address and observed UDP port | answered only for a client that is already in the queue. It cannot start a first contact |

- Neither packet carries the requester's user hash. Without it there is no `obf_id`, and no
  `PROBE` or handshake can be built.
- The first packet needs a requester that is open on TCP. Nobody observes a UDP port for it.
- Buddies are few: a probe of 1,299 Kad nodes got 3 offers.
- A dual-protocol node that is firewalled already holds 2 ed3k relays and lists them in its
  `SOURCE` records.

### 8.3 Server rendezvous: usable

Three mechanisms exist. They are named here by their opcodes.

| | UDP callback `0xA6` / `0xA7` | Rendezvous by user hash, protocol byte `0xF1` | Hole-punch request `0x60`–`0x62` |
|---|---|---|---|
| Found on | the stock eD2K server | one newer server | another newer server |
| Request | UDP `0xA6` to the server's obfuscated port | UDP: register, then a request naming a user hash | TCP `0x60` on the server session; UDP keepalive `0x9F` |
| Mapped UDP endpoint comes from | the source of the `0xA6` datagram | the source of each register and keepalive, per address family | the source of the `0x9F` keepalive |
| Both sides are told | the target only. The requester learns the target's endpoint when the target sends `0xA6` back | yes, in one step | yes, `0x61` goes to both |
| User hash of the peer | only when both sides support obfuscation and one requests it | always: the pair is named by it | always, in `0x61` |
| Who dials | not said | not said | a role byte in `0x61` |
| Address families | IPv4. The server has no IPv6 | IPv4 and IPv6, IPv6 preferred when both have it | the server accepts clients over IPv6, but `0x60` names the target by its 4-byte client ID and `0x61` has a 4-byte address field. The punch is IPv4 |
| Peers | on the same server | any two registered hashes, also on different servers | on the same server |

Two things stand in the way of building any of them as they are. The protocol byte `0xF1` is
not in the list of first bytes an ed3k salt avoids (§4.3); a dual-protocol client that adopts
that rendezvous has to add it there, which is a sender-side rule and needs no wire change.
And the opcodes `0xA6`/`0xA7`, `0x9F` and `0x60`–`0x62` collide with stock eD2K opcodes, so
their packets need layouts of their own before anything is built (section 11).

All three fill the needs of 8.1 when two conditions hold:

1. **The request leaves from the ed3k socket.** The server hands out the source port of that
   datagram. Where ed3k shares the eMule UDP socket (§14.1) this is the case already. Where it
   does not, the client sends the eD2K datagram from its ed3k socket, and the answer is told
   apart by the keyed check of §14.1.
2. **The user hash arrives and matches §14.2.** It gives the `obf_id`. Without a hash, or with
   one that does not match, the pair continues on eD2K.

**Procedure.** For two dual-protocol clients I (requester) and T (target):

1. I asks the server for T, as for any firewalled eD2K source.
2. Each side that receives the peer's endpoint and user hash checks the hash (§14.2).
3. T sends `PROBE` datagrams to I's endpoint and I dials QUIC, as in §9.5 steps 3 and 4. I is
   the QUIC client. Where the server names roles, the side it names as initiator is I.
4. The node ID after the handshake MUST start with the 12 bytes from the hash; otherwise the
   connection is closed with `WRONG_PEER` (§14.2).
5. I tries up to 3 times, as in §9.5.

| Pro | Con |
|---|---|
| reaches a firewalled source that is known only from an eD2K server, where §14.2 gives a hash and no address | works only between two dual-protocol clients |
| eD2K servers act as relays while ed3k has few `ROUTER` nodes | the notice has no NAT class (§9.2), so pairs that cannot work are tried anyway |
| a dual-protocol client needs no second stream transport for these pairs | with the stock server about 13 s on first use, because the first notice goes over TCP |
| no change to any server | the stock server knows the requester by its address alone: clients behind one address can ask in each other's name. The ed3k handshake still fails for a wrong peer |
| | IPv6 only with the rendezvous by user hash |

An index node (§11) is a relay and needs none of this. An eD2K server that runs one says so in
its ident and its description (section 3).

## 9. A server that runs an index node

A server that runs an index node (§11) MAY list, in `OP_FOUNDSOURCES` for an eD2K hash, a node
that stored a `SOURCE` record on its index node, when all of these hold:

- the node's `Hello` has `ED2K` (§2);
- the node is `open` in IPv4 (§9.2);
- a `KEYWORD` record signed by the same key as the connection gives `FileInfo.ed2k` for that
  root, or an `ALIAS` record of scheme `ed2k` by that key does
  ([`extensions/alias.md`](../extensions/alias.md));
- the server's own TCP test of the observed address at that port succeeds.

By the port rule (§14.1) the eD2K TCP port is the ed3k UDP port. But `open` in ed3k is a UDP
result, hence the TCP test. The pair of eD2K hash and root is a claim (§7.4); the server
checks no bytes.

With the login tag `CT_ED3K` (section 3) the server pairs the user hash with the node ID. It
can then send the source in `OP_FOUNDSOURCES_OBFU` with the user hash, so that a
dual-protocol peer goes to ed3k (§14.2). The login tag is not authenticated, so the server
pairs them only when both hold, and otherwise lists the source without a user hash:

- the login's user hash is the one derived from the tag's node ID (§14.2);
- the login comes from the IPv4 address observed on that node's ed3k connection (§8.2 (5)).

Without the login tag it does not use a user hash.

A node that does not want to be listed on eD2K leaves `ED2K` unset in its `Hello`.

The other direction is not possible: eD2K records carry no BLAKE3 root, so a server cannot
store an eD2K source as an ed3k `SOURCE` record.

## 10. Built-in seed node links

A new network has no `ROUTER` to export a node list from (bootstrap.md §5): the first
permanent node meets nobody, stays out of `ROUTER`, and a list made from its table is empty.
A client MAY therefore ship one or more `ed3k:node` links (§13.2) to nodes its makers run, and
feed them as source 3 of [`extensions/bootstrap.md`](../extensions/bootstrap.md) §1, as if the
user had opened them:

- The link carries the node ID and the node's public endpoints, one per family. The handshake
  proves the ID, so a stale or forged built-in link costs one unanswered dial.
- The seed keeps its node key across restarts and updates, else every shipped link breaks.
- `ep` takes IP literals only, so a seed that moves needs a new client release. Shipping links
  to seeds on different hosts and networks keeps one move or outage from mattering.
- Used after saved contacts and before the shipped node list. Once a node list from a `ROUTER`
  ships with the client, the links stay as a fallback.
- The seed runs eD2K too, with a derived user hash (§14.2). A dual-protocol client that meets
  it on eD2K (source 4 of bootstrap.md §1) needs no built-in link.
- A built-in seed sees every fresh install's first contact. It learns no more than any
  `ROUTER` a new client meets first, but running several seeds spreads that.

## 11. Open points

- The check covers the source trees at hand. `0xE3`, `"e3"` and bit 11 of `0xAA` should be
  confirmed with the authors of other clients and of server software before first use.
- Whether a server relays the login tag to other clients, for example as one more bit in the
  per-source option byte of `OP_FOUNDSOURCES_OBFU`. That byte has no free bit that legacy
  clients preserve (section 7).
- Section 8 is a design only. Open are the time a client waits for ed3k before it falls back
  to the eD2K stream of the mechanism, and whether the request may leave from the ed3k socket
  when that is not the eMule UDP socket: the keepalives then have to come from it too.
- A hole punch over IPv6 through the `0x60`–`0x62` request would need an address field of
  16 bytes there.
