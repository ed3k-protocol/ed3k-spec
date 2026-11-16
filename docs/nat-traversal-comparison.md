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

## 6. The two QUIC NAT traversal drafts

Read 2026-11-17:
[draft-seemann-quic-nat-traversal-04](https://datatracker.ietf.org/doc/html/draft-seemann-quic-nat-traversal-04)
and [draft-bruynooghe-n0-quic-nat-traversal-00](https://datatracker.ietf.org/doc/draft-bruynooghe-n0-quic-nat-traversal/)
(n0's dialect, as built in its `noq` QUIC stack for iroh, based on Seemann -02 with other
codepoints; depends on [QUIC Address Discovery](https://datatracker.ietf.org/doc/draft-ietf-quic-address-discovery/)
and [QUIC Multipath](https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/)). Both are
individual drafts, no WG status.

**Main difference.** Both drafts run inside a QUIC connection the two peers already hold over a
data-carrying relay path (MASQUE proxy, iroh relay); on success that connection migrates to the
direct path. ed3k has no such connection: relays carry signalling only (§9.4), and a punch ends
in a new handshake. So the draft frames cannot replace `Punch` without a data relay; only §9.2's
`observed` maps 1:1 to `OBSERVED_ADDRESS`. §5's "replaceable by the standard frames" holds for
address observation only.

| Draft idea | ed3k (§9) | Fit |
|---|---|---|
| Signalling inside an end-to-end relayed QUIC connection, then migration | `Punch` through the relay on separate connections, then a new dial | different model, follows from signalling-only relays |
| Asymmetric roles (Seemann: server advertises candidates, client picks pairs and timing) | I = QUIC client, T = server; T sends `PROBE`s, I the Initials | same split |
| Probes = `PATH_CHALLENGE`/`PATH_RESPONSE` on the connection | `PROBE{nonce}`, never answered; I's Initial is the answer | ours needs no connection, is cheaper |
| n0: answer an off-path challenge with a new challenge (endpoint-dependent mapping) | §9.5 step 4: a `PROBE` from an unknown source → I dials it at once | covered |
| Seemann `PUNCH_GRANT` / `MAX_PUNCH_GRANTS` credits | T's limits (30/min, 3/min per host), `Error{BUSY}` | same effect by local policy |
| Candidate sequence numbers, `REMOVE_ADDRESS`, `STALE_ADVERTISEMENT` | endpoints sent fresh in every `Punch` try | not needed |
| Many candidates (local addresses, interfaces, sending bases) | one observed endpoint per family, one socket | by choice (§1); same-NAT pairs → `lan-discovery` |
| Amplification / spoofing | TODO in both drafts | ed3k specifies it: `seen` bits, ≤ 3 probes to a claimed endpoint, 50 per punch, 100/min per host |
| n0 rounds, 30 s timeout, PTO back-off | 5 s probing, 8 s handshake wait, 3 tries, 30 min back-off | equivalent |
| Re-punch a live connection after the server side's NAT rebinds (multipath) | server-side address change loses the connection; new punch (§4.5) | gain of the drafts; needs multipath |
| Path probes from new addresses on the connection | §4.5 demux knows live connection addresses and client-side migration only; a server-side packet from a new address is dropped | adopting the drafts needs obfuscation demux changes too |

**Libraries.** §2's "not available in the candidate QUIC libraries" is no longer quite true for
Rust: `noq` (a quinn fork) has n0's dialect, and packmule uses sans-IO `quinn-proto`, so the
socket stays ours. n0's codepoints are not Seemann's, though; nothing to converge on yet.

**Discarded (2026-11-17).** Considered and not taken; wire version 1 unchanged:

- ~~Reword §5 and `Protocol.md` §9.6: draft frames replace `Punch` only with a data-carrying
  relayed QUIC path (data relay capability or `hops`)~~ — discarded.
- ~~Add the n0 draft to `Protocol.md` references and pin the Seemann entry to -04~~ — discarded.
- ~~Move packmule from `quinn-proto` to `noq` for the n0 frames~~ — discarded: individual drafts,
  dialect codepoints, demux changes (above).
