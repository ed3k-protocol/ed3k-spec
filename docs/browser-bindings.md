# Reaching web browsers: options for later

**Status:** design note (2026-10-27). Informative; changes nothing in wire version 1. Nothing
here is planned; it collects what was looked at, so that the question need not be worked out
again.
**Relates to:** [`Protocol.md`](../Protocol.md) §3 (transport contract), §4 (`quic-obfs`), §9 (NAT
traversal), [`transport-bindings.md`](transport-bindings.md),
[`extensions/http-source.md`](../extensions/http-source.md),
[`extensions/name.md`](../extensions/name.md) §5.4 and §6,
[`extensions/chaintube.md`](../extensions/chaintube.md) §5. Section numbers (§) refer to
`Protocol.md` unless a document is named.

A web browser cannot be an ed3k node. Version 1 has one binding, `quic-obfs`: QUIC over raw UDP,
TLS with raw Ed25519 keys (RFC 7250), no SNI, masked datagrams (§4.1, §4.3). A page has no raw
UDP, no raw-key TLS and no way to send datagrams of its own format. A browser extension has none
of these either.

This note sorts the ways to serve browsers into two groups:

- **Without a new binding** (§1). Browsers fetch over HTTP; nodes do the peer-to-peer part.
  Most of it exists already or needs only a role description.
- **With a browser binding** (§2–§5). A page becomes a node, a leaf that can only dial. This
  is the larger step 3 of [`transport-bindings.md`](transport-bindings.md) §3.

**Result:** the first group covers watching, reading and downloading at little cost. The second
pays off only if ordinary nodes accept the browser binding as well, not only a few gateways
(§5.1). It is left for later.

## 1. Without a new binding

| | Way | State | What the browser gets |
|---|---|---|---|
| 1 | **Local server of a client** | specified, [`name.md`](../extensions/name.md) §6 | websites and files from ed3k, on `*.localhost`, when the user runs a client |
| 2 | **Hand-off** from a page or a browser extension | specified, [`name.md`](../extensions/name.md) §5.4 | an `ed3k:` link opened in the client; the extension is not a node |
| 3 | **Site gateway** | specified, [`chaintube.md`](../extensions/chaintube.md) §5.2 | a site's own files over HTTP; its node lists the gateway as an HTTP source |
| 4 | **Public gateway** | not specified (§1.1) | any file or set over HTTPS, by root, from a node operator |
| 5 | **Lookups over HTTPS** | not specified (§1.2) | sources of a root, without a Kad walk |
| 6 | **Live streams as HLS or DASH** | segment form fits, [`live.md`](../extensions/live.md) §2 | a stream played by the browser's own player, from a gateway |
| 7 | **IPFS** | named in links, [`chaintube.md`](../extensions/chaintube.md) §5.5 | files that also have a content ID, through IPFS tooling |

### 1.1 A public gateway

A node that also serves files to browsers by root:

```
https://<gateway>/ed3k/<root>                 the file
https://<gateway>/ed3k/<root>/hashes          its hash body (http-source.md §3.1)
https://<gateway>/ed3k/set/<key>/<id>/<path>  a member of a set
```

- It downloads from ed3k on demand and answers with `Range`, like a plain HTTP source
  ([`http-source.md`](../extensions/http-source.md) §1). Its URLs MAY be handed out as HTTP
  sources too.
- **The gateway need not be trusted for content.** A page, or a service worker in front of it,
  fetches the hash body, checks it against the root, and checks every 256 KiB block as it
  arrives, as an ed3k node does. BLAKE3 runs in WebAssembly. A page that skips the check trusts
  the gateway, as with any web server.
- The service worker can fetch blocks from several gateways and HTTP sources at once, and drop
  one that sends a wrong block. That gives a browser a swarm of servers, not of peers.
- The gateway sees what each visitor asks for, and carries the upload cost alone. It decides
  what it serves, as any HTTP server does. Abuse rules are its operator's.

What would need specifying: the URL layout, how a gateway is named (a capability bit, or a
link), and whether it serves sets as websites under one origin per set, as
[`name.md`](../extensions/name.md) §6.2 does on the loopback address.

### 1.2 Lookups over HTTPS

An HTTPS front end of an index node (§11) that answers a `SOURCE` lookup for a root with the
HTTP sources it knows, and with gateways. This is the delegated lookup of IPFS
([`ipfs-freenet-comparison.md`](ipfs-freenet-comparison.md) §4.7). The server sees every
lookup of its clients. Together with §1.1 it is a light client with no peer-to-peer at all.

## 2. A browser binding: what a browser offers

| Transport | Browser → node | Browser → browser | Certificates | Underneath |
|---|---|---|---|---|
| **WebRTC data channels** | yes, also without a signalling server when the node's certificate hash is known (§3.1) | yes, with signalling (§3.2) | self-signed; checked by fingerprint | SCTP over DTLS over UDP, with ICE |
| **WebTransport** | yes | no | `serverCertificateHashes`: self-signed is accepted, but browsers require ECDSA P-256 and at most 14 days of validity | HTTP/3 over QUIC |
| **WebSocket** (`wss`) | yes | no | a certificate of a public CA, so a domain name | TLS over TCP |

Precedents:

- **WebTorrent.** Browsers are peers over WebRTC; trackers over WebSocket carry the signalling.
  Desktop clients that also speak WebRTC join both swarms. Most desktop clients never added it,
  and browser and desktop swarms stayed largely apart.
- **libp2p.** It has all three: WebRTC to a server with the certificate hash in the address,
  WebRTC between browsers with signalling over a relay, and WebTransport with certificate
  hashes. It is the closest model for ed3k, because it also keeps one key identity across
  transports.

## 3. A WebRTC binding

The best fit of the three: it reaches browsers and native nodes alike, does NAT traversal of
its own, and needs no domain and no CA.

### 3.1 Browser to node

A node that accepts the binding listens on a UDP port for it, with a DTLS certificate that it
keeps. The `Binding` endpoint ([`transport-bindings.md`](transport-bindings.md) §3, step 1)
carries the port and the SHA-256 fingerprint of the certificate. The browser builds the remote
description from that address, so no signalling server takes part: the node answers ICE on
that port and derives the ICE credentials from what the browser sends.

How the contract would be met:

| | How |
|---|---|
| T1 | DTLS gives a channel bound to two certificate fingerprints, not to Ed25519 keys. On the first data channel each side sends its Ed25519 key and a signature over both fingerprints and a context string. WebCrypto signs Ed25519 in current browsers. The dialler checks the node ID as §4.1 does |
| T2 | DTLS |
| T3 | ICE connectivity checks prove the address before data flows |
| T4 | one data channel per exchange; ordered, reliable. Flow control is coarser than QUIC's: the sender watches `bufferedAmount` |
| T5 | WebRTC is recognisable as WebRTC but not as ed3k. Whether that counts as a transport that hides itself (§3, T5) needs a ruling. Masking DTLS is not possible from a page |

A node keeps the binding's port apart from its `quic-obfs` port: the datagram format of §4.3
and STUN/DTLS cannot share one socket without a demultiplexer that would make the `quic-obfs`
port recognisable.

### 3.2 Browser to browser

Two pages exchange an offer and an answer through a node that both reach. That is the role of
a relay (§9.4) with a different payload: `Punch` carries endpoints and a nonce, not a session
description. It would need a message for the description, or a field in `Punch`, and relays
that forward it to browser leaves over the binding of §3.1. The browsers then connect directly
by ICE, or not at all; ed3k has no data relay (§9.4).

### 3.3 What a page can and cannot do as a node

- It is always a **leaf**: it cannot listen, so it never sets `ROUTER` and stays out of routing
  tables. It looks up through the nodes it dialled, as any leaf does.
- It can download, verify, and upload to peers while the tab is open. It stores in the origin
  private file system; large files are bounded by the browser's storage quota.
- It needs contacts it can dial. Kad answers would carry compact contacts per binding
  ([`transport-bindings.md`](transport-bindings.md) §3, step 3); the 22- and 34-byte layouts
  have no room for a fingerprint, so these contacts are larger.
- Entry points: `ed3k:node` links and bootstrap lists that name the binding and the
  fingerprint.
- protobuf, zstd and BLAKE3 run in WebAssembly. None of the format is in the way.

## 4. WebTransport and WebSocket as second choices

**WebTransport.** Simpler than WebRTC for browser to node: HTTP/3 streams map to exchanges one
to one, with QUIC flow control. The node keeps an ECDSA P-256 certificate, renews it before 14
days pass, and publishes the new hash, signed by its Ed25519 key, in its `Binding` endpoint.
T1 needs the same signature exchange as §3.1, because a page cannot read TLS exporter values.
No browser-to-browser connections. Support differs between browsers.

**WebSocket.** TCP, one stream, so exchanges need a multiplexer. It needs a CA certificate,
so only nodes with a domain name can offer it. Its one advantage: on port 443 it looks like any
HTTPS connection, so the same listener could serve as the TLS-over-TCP binding for native nodes
on networks that block UDP ([`transport-bindings.md`](transport-bindings.md) §6). If that
binding is ever added, it is worth defining over WebSocket for this reason.

## 5. When it would be worth it

### 5.1 The condition

A browser binding is useful only if browsers reach many nodes, not a few gateways. If only
gateways accept it, it does what §1.1 does with more code on both sides. So the question is
whether ordinary implementations accept the binding:

| | Few gateways accept it | Most nodes accept it |
|---|---|---|
| Browsers can | download from gateways | download from and upload to the swarm |
| Compared with §1.1 | the same, with a WebRTC stack | browsers add upload capacity while tabs are open |
| Cost to every implementation | none | a WebRTC stack, a second port, signalling at relays |

### 5.2 Cost

- A WebRTC stack in every implementation that joins. Mature libraries exist for C++ and Rust.
- A second UDP port per node, with its own reachability, and the address announcement of
  [`transport-bindings.md`](transport-bindings.md) §3: `Hello.bindings` (field 11) and
  `Peer.bindings` (field 6), both reserved in version 1.
- Per-binding Kad contacts, and a ruling on whether an address validated over WebRTC counts for
  the routing ID check (§6.3).
- A message or field to carry session descriptions through relays (§3.2).
- A capability bit; bit 21 is the next one free at the time of writing.

### 5.3 Suggested order

1. **§1.1 and §1.2 first.** They serve watching and downloading in a browser, need no change
   to the protocol, and reuse the hash body of
   [`http-source.md`](../extensions/http-source.md) §3.1.
2. **A WebRTC binding** (§3) once two implementations are willing to accept it on ordinary
   nodes, as an optional bit, possibly on by default.
3. **WebTransport** as a simpler browser-to-node path, if browser support is wide enough.
4. **WebSocket** only together with a TCP binding for networks that block UDP (§4).

## 6. Open points

- Whether a WebRTC binding meets T5, or needs an exception for transports that look like a
  common web protocol.
- Whether a node that accepts only browser bindings, and not `quic-obfs`, is allowed at all.
  [`transport-bindings.md`](transport-bindings.md) §3 keeps `quic-obfs` mandatory; a page
  cannot meet that.
- How a gateway (§1.1) is announced: a capability bit, a record kind, or links only.
- Whether a gateway serves sets as websites, and under which origins.
- Upload from browsers: whether a page that holds a file counts as a source in `SOURCE`
  records, given that it disappears when the tab closes.
