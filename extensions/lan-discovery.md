# Extension: finding nodes in the local network (`lan-discovery`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §9.2, §12.1 and §13.1. § = that document unless this one is named.
**Capability:** bit 13, `LAN`. Message types 24 and 25. One multicast datagram outside any connection.

Two nodes in one home/office network usually can't find each other: same NAT, same public
address in Kad. This lets them meet directly, so a file moves at LAN speed, using none of the
user's line.

- A node **announces its presence** by multicast: node ID and port only.
- Files in common are asked **inside the connection**, encrypted and authenticated as usual.

Idea: BitTorrent's Local Service Discovery (BEP 14), which announces the files themselves to the
whole network; ed3k does not
([`docs/bittorrent-comparison.md`](../docs/bittorrent-comparison.md) §2.2).

---

## 1. The announce

```
salt(8) ‖ mask( magic(4) ‖ body )

magic  = 0x65 0x33 0x6C 0x01          ("e3l", version 1)
key    = H("ed3k v1 lan", net_id ‖ salt)        net_id as in §4.3: empty in the public network
mask   = as in §4.3, with this key
body   = a serialised LanAnnounce
```

`salt` is fresh per datagram; `salt[0]` follows the rule of §4.3. A datagram is at most
512 *(initial)* bytes; larger → dropped unread.

```protobuf
message LanAnnounce {
  bytes  id = 1;              // 16 B node ID
  uint32 port = 2;            // UDP port the node listens on, on this interface
  bool   reply = 3;           // sent in answer to an announce; is not answered
}
```

| | IPv4 | IPv6 |
|---|---|---|
| Group *(initial)* | `239.255.101.51` | `ff02::6533:6b` |
| Port *(initial)* | 6533 | 6533 |
| Reach | TTL 1 | hop limit 1; group is link-local |

Neither group nor port is registered.

- Announcing node's address = datagram source address; body carries none.
- A `LanAnnounce` with `port` 0 or above 65,535 is dropped.
- Several nodes on one host share the port; each binds it for shared use.
- Anyone can compute the mask key; it only keeps the datagram free of fixed bytes. Anyone in the
  network who knows this document can read it (§5 of this document).

**Sending**

- Announce on each listening interface: at start, when the interface gets a new address, and every 5 min
  *(initial)*. An interface with both families announces to both groups; the two datagrams =
  one announce for the limits below.
- Announce from an unknown node → answer with one announce to the group, `reply` set, after a
  random wait of up to 1 s. At most once per 5 min per given node, at most 10 *(initial)*
  replies per minute; replies don't count against the limit below. So a newcomer knows the
  others at once, as does everyone else on the link, as in BEP 14.
- At most 1 multicast announce per minute and interface. The 10 replies/minute are per node,
  over all its interfaces.

**Receiving**

- An announce is accepted only when its source address is a private address of `Protocol.md`
  §8.2 (8) other than loopback (`127.0.0.0/8`, `::1`), **and** lies in a network of the
  arrival interface. All else dropped.
- Node remembers (node ID, address, port, interface) as a **LAN peer** for 15 min *(initial)*
  after the last announce.
- An announce is no reason to connect; a node dials a LAN peer when it has something to ask
  (§2 of this document).
- A node without Kad contacts also dials a LAN peer for first contact
  ([`bootstrap.md`](bootstrap.md) §1). Every node answers `KadRequest` with contacts it knows
  (§10.3), so a leaf LAN peer serves too. Rule §3 of this document holds: the LAN peer
  itself does not enter the routing table.
- An announce proves nothing; the handshake does (T1): a LAN peer whose key does not give the
  announced node ID is dropped. A false announce costs one LAN dial.
- At most 256 *(initial)* LAN peers kept, at most 10 new ones taken per minute, both per node
  over all interfaces. A node not taken due to these limits is still answered as unknown.

---

## 2. Which files a LAN peer has

```protobuf
message LanQuery {
  bytes roots = 1;            // n × 16 B: root[0..16) of files I look for; at most 256
}

message LanAnswer {
  bytes have = 1;             // one bit per entry, LSB first: I share that file, completely or in part
}
```

- `LanQuery`: request/answer exchange (§5.2), only to a LAN peer that set `LAN`. >256 roots, or
  `roots` not a multiple of 16 bytes → `Error{MALFORMED}`.
- Answered only on a connection whose remote address passes §1 of this document (Receiving),
  else `Error{DENIED}`; checked before the `MALFORMED` test.
- `have`: one bit per query root, in order. Shorter `have` → missing bits read as 0; bits beyond
  the last root ignored.
- Per set bit, asker adds the LAN peer as source of that file and opens a file stream as usual
  (§12.1).
- Ask a LAN peer at most once per 10 min *(initial)*, plus once more when starting a new
  download, at most once per minute.
- 16 root bytes suffice to pick a source; the file stream names the whole root and every block
  is verified against it.

A node that only shares never needs to ask; downloading nodes find it.

---

## 3. Rules for a LAN peer

- **A LAN address is never passed on.** A node MUST NOT name a LAN peer by its local address in
  a Kad answer, in `FileStatus.sources`, in a `Peer` of any other message, or as a relay. Not put in
  the routing table.
- **Its address reports do not count.** A LAN peer's `Hello.observed` is a local address: no
  report in the §9.2 sense, and a connection from it is no unasked connection there.
- The limits of [`ip-diversity.md`](ip-diversity.md) do not count a LAN peer (§1 there); the
  connection budget (§4.6) exempts local addresses.
- A LAN peer waits in the upload queue like any downloader. A node MAY serve LAN peers outside
  its upload limit and slot count, since that data does not use the user's line; local policy.
- A bad file from a LAN peer is handled as from any source (§12.3).

---

## 4. When it is on

- A user MUST be able to turn the extension off, as a whole and per network.
- It SHOULD be on in a network the OS marks private/home, and off in one marked public, unless
  the user decides otherwise.
- Off: no announcing, no answering announces; `LanQuery` → `Error{UNSUPPORTED}`.

---

## 5. What it shows

| To whom | What |
|---|---|
| Everyone in the local network | that this host runs ed3k, its node ID and port. §4.2 obfuscation hides this from network operators; an announce gives it away |
| A LAN peer that asks | which named files this node shares. Any internet peer learns the same via a file stream |
| A LAN peer that is asked | 16 bytes of the root of each file the asking node looks for |
| Nobody outside the network | anything. The datagrams do not leave the link |

Hence the announce names no files, and §4 of this document turns it off in a public network.

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Announcing the files** (BEP 14) | no query; dial only peers that have something | every host in the network reads what each node downloads, w/o connecting |
| **Asking with a file stream per file** | no new message | files × peers exchanges, most answered `NOT_FOUND` |
| **mDNS / DNS-SD** | registered, tools exist | service name in clear; a 2nd protocol

 to implement for 30 bytes of content |
| **Browsing the LAN peer** ([`browse.md`](browse.md)) | exists | optional, lists everything, many nodes turn it off |
| **A plain datagram without the mask** | simplest | fixed bytes at a fixed place, avoided everywhere else in the transport |
| **Finding the peer through Kad and hairpin NAT** | nothing new | many routers do not hairpin, and data would pass the router |

---

## 7. Open points

- Group and port are placeholders.
- Whether to prefer a LAN source so strongly that others are no longer asked for the same
  blocks.
- Networks blocking multicast between clients (many guest networks) are not served.
