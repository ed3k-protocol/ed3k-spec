# NAT traversal: a self-contained design, and the alternatives

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §9 (NAT traversal) and §4 (`quic-obfs`). Section numbers (§) refer to that document.

ed3k defines its own NAT traversal: address observation, `ReachTest`, signalling relays and
`Punch`. This note records why it does not take an existing stack instead.

**Result:** a self-contained design of four messages. No existing stack leaves room for the
obfuscation of §4, and each brings its own socket, runtime or servers. The procedure itself is
borrowed, and is kept close to the QUIC drafts that should replace it.

## 1. What the design needs

- **The application owns the UDP socket.** Every datagram passes through the obfuscation layer
  (§4.3), and punch packets leave from the same socket as the QUIC connection.
- **No plaintext and no fixed byte patterns** on the wire (§4.2).
- **No dedicated servers.** Address observation and relaying are done by ordinary `ROUTER` nodes.
- **Two candidate types only:** the observed public endpoint, and a relay for signalling.
- **Usable from C++**, without a second runtime.

## 2. Comparison

| Option | Verdict |
|---|---|
| **libp2p** (DCUtR, AutoNAT, Circuit Relay v2) | The procedure in §9.5 is DCUtR's. The stack is not usable as a dependency here: the C++ implementation has no QUIC, relay or hole punching; the Go and Rust ones bring their own runtime, socket, peer-ID format, stream negotiation and the ALPN `libp2p`, and offer no place for §4's obfuscation except a socket wrapper in Go |
| **iroh** | Closest in spirit (dial by key, relays, QUIC with NAT traversal). Rust with an immature C interface; its relays carry data; it owns the socket |
| **ICE/STUN/TURN** (libjuice, libnice, libdatachannel) | STUN is plaintext with a fixed magic cookie, which is exactly the kind of tell §4 removes, and it needs STUN/TURN servers. ICE's candidate machinery is far more than two candidate types need |
| **QUIC drafts** (address discovery, NAT traversal) | The right long-term home: `OBSERVED_ADDRESS` is §9.2 and the punch frames are §9.5. Both are still drafts and not available in the candidate QUIC libraries |

## 3. What is borrowed

- **The punch procedure** (§9.5) is DCUtR's: the two sides exchange their observed endpoints
  through a relay and then send to each other at once. The success rate quoted in §9.5 was
  measured on DCUtR.
- **Address observation** (§9.2) does what the `OBSERVED_ADDRESS` frame of the QUIC draft does.
- **Reachability tests** (§9.3) and **limited relays** (§9.4) have counterparts in libp2p's
  AutoNAT v2 and Circuit Relay v2 (Appendix C). Unlike a libp2p relay, an ed3k relay carries
  signalling only.

## 4. Costs of the choice

- **More to specify and test.** §9 is part of this specification instead of a reference to
  another one, and its interoperability has to be proven by ed3k implementations alone.
- **No data relay.** A pair that cannot be punched (symmetric NAT on both sides) does not
  connect. §9.4 leaves a data relay for later.

## 5. Path to the standard

§9 is kept close to the QUIC drafts so that it can be replaced by the standard frames in a later
wire version, once they are final and present in QUIC libraries. The messages of §9 would then
go, and the rules for classes, relays and when to punch would stay.
