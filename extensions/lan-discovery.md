# Extension: finding nodes in the local network (`lan-discovery`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §9.2, §12.1 and §13.1. Section numbers (§) refer to that document unless this one is named.
**Capability:** bit 13, `LAN`. Message types 24 and 25. One multicast datagram outside any connection.

Two nodes in the same home or office network usually cannot find each other: both sit behind
the same NAT, and Kad knows them by the same public address. This extension lets them meet
directly, so that a file one of them has moves at the speed of the local network and uses none
of the user's line.

- A node **announces its presence** by multicast: node ID and port, nothing else.
- Which files two nodes have in common is asked **inside the connection**, which is encrypted
  and authenticated like any other.

The idea is BitTorrent's Local Service Discovery (BEP 14). BEP 14 announces the files
themselves to the whole network; ed3k does not
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
| Reach | TTL 1 | hop limit 1; the group is link-local |

Neither group nor port is registered.

- The address of the announcing node is the source address of the datagram. The body carries
  none.
- Several nodes on one host share the port; each binds it for shared use.
- The mask has a key that everyone can compute. It only keeps the datagram free of fixed
  bytes. Anyone in the network who knows this document can read it (§5).

**Sending**

- A node announces on each interface it listens on: when it starts, when the interface gets a
  new address, and every 5 min *(initial)*.
- A node that hears an announce from a node it did not know answers with one announce sent to
  the group, with `reply` set, after a random wait of up to 1 s. It answers a given node at most
  once per 5 min and sends at most 10 *(initial)* replies per minute; replies do not count
  against the limit below. A newcomer then knows the others at once, and so does everyone else
  on the link, as in BEP 14.
- A node sends at most 1 multicast announce per minute and interface.

**Receiving**

- A node accepts an announce only when its source address is private, link-local or
  unique-local, **and** lies in a network of the interface it arrived on. Everything else is
  dropped.
- It remembers (node ID, address, port) as a **LAN peer** for 15 min *(initial)* after the last
  announce.
- An announce is not a reason to connect. A node dials a LAN peer when it has something to ask
  (§2 of this document).
- An announce proves nothing. The handshake does (T1): a LAN peer whose key does not give the
  announced node ID is dropped. A false announce costs one dial inside the local network.
- A node keeps at most 256 *(initial)* LAN peers and takes at most 10 new ones per minute.

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

- `LanQuery` is a request/answer exchange (§5.2), sent only to a LAN peer that set `LAN`.
- A node answers it only on a connection whose remote address passes the test of §1 of this
  document (Receiving), and with `Error{DENIED}` otherwise.
- For every set bit the asking node adds the LAN peer as a source of that file and opens a file
  stream as usual (§12.1).
- A node asks a LAN peer at most once per 10 min *(initial)*, and once more when it starts a new
  download, at most once per minute.
- 16 bytes of the root are enough to pick a source. The file stream names the whole root, and
  every block is verified against it.

A node that only shares never needs to ask. It is found by the nodes that download.

---

## 3. Rules for a LAN peer

- **A LAN address is never passed on.** A node MUST NOT name a LAN peer by its local address in
  a Kad answer, in `FileStatus.sources`, in a `Peer` of any other message, or as a relay. It is
  not put into the routing table.
- **Its address reports do not count.** What a LAN peer says in `Hello.observed` is a local
  address. It is not a report in the sense of §9.2, and a connection from it is not an unasked
  connection there.
- The limits of [`ip-diversity.md`](ip-diversity.md) and the connection budget (§4.6) already
  exempt local addresses.
- A LAN peer waits in the upload queue like any other downloader. A node MAY serve LAN peers
  outside its upload limit and outside its number of slots, because that data does not use the
  user's line. This is local policy.
- A file that is bad when it comes from a LAN peer is handled as from any source (§12.3).

---

## 4. When it is on

- A user MUST be able to turn the extension off, as a whole and per network.
- It SHOULD be on in a network that the operating system marks as private or home, and off in
  one marked as public, unless the user decides otherwise.
- With the extension off a node neither announces nor answers announces, and answers `LanQuery`
  with `Error{UNSUPPORTED}`.

---

## 5. What it shows

| To whom | What |
|---|---|
| Everyone in the local network | that this host runs ed3k, its node ID and its port. The obfuscation of §4.2 hides this from a network operator; an announce gives it away |
| A LAN peer that asks | which of the files it names this node shares. Any peer on the internet learns the same by opening a file stream |
| A LAN peer that is asked | 16 bytes of the root of each file the asking node looks for |
| Nobody outside the network | anything. The datagrams do not leave the link |

This is why the announce names no files, and why §4 of this document turns the extension off
in a public network.

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Announcing the files** (BEP 14) | no query; a node dials only peers that have something | every host in the network reads what each node downloads, without connecting |
| **Asking with a file stream per file** | no new message | files × peers exchanges, most of them answered `NOT_FOUND` |
| **mDNS / DNS-SD** | registered, tools exist | a service name in clear, and a second protocol to implement for 30 bytes of content |
| **Browsing the LAN peer** ([`browse.md`](browse.md)) | exists | optional, lists everything, and many nodes turn it off |
| **A plain datagram without the mask** | simplest | fixed bytes at a fixed place, which the transport avoids everywhere else |
| **Finding the peer through Kad and hairpin NAT** | nothing new | many routers do not hairpin, and the data would pass the router |

---

## 7. Open points

- Group and port are placeholders.
- Whether a node should prefer a LAN source so strongly that it stops asking others for the
  same blocks.
- Networks that block multicast between clients (many guest networks) are not served.
