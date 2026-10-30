# ed3k-spec

This is the specification of a modern file sharing protocol using protobuf with NAT-T and IPv6.

It is inspired by eD2K. Its reference implementation, [eMuleQt](https://github.com/ModderMule/emule-qt),
is fully compatible with the eD2K network.

See [Protocol.md](./Protocol.md) for implementation details. [See here for discussion.](https://github.com/ModderMule/emule-qt/discussions/91)

**DRAFT VERSION: This spec is alpha and will change - please do NOT publish your implementations yet without prior contact and review via GitHub issues**

## Core Protocol

ed3k keeps the eD2K/Kad model — hash-identified files, a Kademlia DHT, optional index servers and upload queues with credits — but replaces wire format, transport and identity:

- **Transport:** UDP only, over obfuscated QUIC (ALPN `ed3k/1`), so traffic is encrypted and hard to fingerprint
- **Messages:** protobuf, one message per purpose, zstd compression
- **Identity:** an Ed25519 key per node; its BLAKE3 hash is the node ID in Kad and everywhere else
- **Files:** identified by their BLAKE3 root hash; 256 KiB blocks are verified on their own against the root, so a bad block is caught at once with its sender known
- **Kad:** Kademlia DHT over QUIC for sources, keywords and signed records; IP-bound routing IDs and IP diversity limits against Sybil attacks
- **Index nodes:** an "eD2K server" is just an ordinary node with the `INDEX` capability
- **NAT traversal:** required, not an add-on: reachability tests, relays for signalling and UDP hole punching
- **IPv4 and IPv6:** one overlay, both first-class
- **eD2K coexistence:** optional; a file can carry its eD2K hash as an alias, so eMule peers serve the same download. ed3k sources travel seamlessly through eMule source exchange (XS), eD2K servers and eMule Kad: the ed3k port equals the eD2K TCP port and the eD2K user hash is derived from the ed3k node ID, so any eD2K source record already says the peer speaks ed3k and where to reach it. eD2K peers are contacted directly over ed3k where supported

## Extensions

Optional features on top of the core, each announced by a capability bit or carried in records and links. Network extensions map other networks onto the BLAKE3 identity of a file.

| Extension | Description |
|---|---|
| [`torrent`](./extensions/torrent.md) | Maps BitTorrent torrents to file sets, so torrent peers and ed3k peers serve the same file. |
| [`usenet`](./extensions/usenet.md) | Maps Usenet posts (NZB) to file sets, so articles from news servers fill the same download. |
| [`hoster`](./extensions/hoster.md) | Publishes links at file hosters as sources of a file. |
| [`http-source`](./extensions/http-source.md) | Adds plain HTTP(S) URLs as download sources. |
| [`alias`](./extensions/alias.md) | Links other names of a file (eD2K, BitTorrent, Usenet, …) to its BLAKE3 root in Kad. |
| [`file-set`](./extensions/file-set.md) | Groups files into signed sets such as folders, albums or releases. |
| [`hidden-set`](./extensions/hidden-set.md) | File sets that only holders of the link can find and read. |
| [`multi-author-set`](./extensions/multi-author-set.md) | File sets maintained by several authors. |
| [`filter-list`](./extensions/filter-list.md) | Shared lists to filter out fakes and unwanted files. |
| [`chaintube`](./extensions/chaintube.md) | Videos whose names and sets are anchored on a blockchain. |
| [`credit`](./extensions/credit.md) | Upload credits backed by an on-chain ledger to reward uploaders. |
| [`subscription`](./extensions/subscription.md) | Paid access to channels, sets and live streams. |
| [`live`](./extensions/live.md) | Live streams distributed peer to peer. |
| [`pubsub`](./extensions/pubsub.md) | Publish/subscribe topics to follow publishers and get new items pushed. |
| [`chat`](./extensions/chat.md) | Direct chat messages between nodes. |
| [`room`](./extensions/room.md) | Group chat rooms joined via `ed3k:room` links. |
| [`name`](./extensions/name.md) | Human-readable names for nodes and sets, also via DNS. |
| [`friend`](./extensions/friend.md) | Finds a known node again by its key after its address changed. |
| [`browse`](./extensions/browse.md) | Browses the shared files of a peer. |
| [`hops`](./extensions/hops.md) | Anonymous paths over several hops to hide who downloads or shares. |
| [`lan-discovery`](./extensions/lan-discovery.md) | Finds nodes in the local network via multicast for fast LAN transfers. |
| [`bootstrap`](./extensions/bootstrap.md) | File format and link for first contacts to join the network. |
| [`ip-diversity`](./extensions/ip-diversity.md) | Limits nodes per IP range in routing tables and records (required). |
| [`flood-protection`](./extensions/flood-protection.md) | Local policy against packet floods. |
| [`keyword-load`](./extensions/keyword-load.md) | Spreads popular keywords over several levels and ranks results. |
| [`kad-datagram`](./extensions/kad-datagram.md) | Draft: sessionless Kad over plain datagrams instead of QUIC. |
| [`key-puzzle`](./extensions/key-puzzle.md) | Draft: proof-of-work on node keys against Sybil attacks. |

## License

The specification is placed in the public domain.
See [LICENSE](./LICENSE).

---

[emule-qt.org](https://emule-qt.org/)
