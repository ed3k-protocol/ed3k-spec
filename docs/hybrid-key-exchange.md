# Hybrid key exchange (X25519MLKEM768) for the QUIC binding

**Status:** design note (2026-10-11). Informative; a possible later upgrade. Changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §4.1 (QUIC profile) and §10.5 (lookup cost). Section numbers (§) refer to that document.

Version 1 uses the X25519 key exchange of TLS 1.3. This note evaluates adding the hybrid group
X25519MLKEM768, which stays secure if either X25519 or ML-KEM-768 holds.

**Result:** not in version 1. An adversary who records all traffic today in order to decrypt it
with a future quantum computer is outside the threat model of version 1. The upgrade is
compatible whenever it is wanted, because TLS negotiates the group, and it roughly doubles the
cost of every new connection.

The handshake sizes below are estimates from the key-share sizes, not measurements.

## 1. What it changes on the wire

| | X25519 | X25519MLKEM768 |
|---|---|---|
| Client key share | 32 B | 1,216 B |
| Server key share | 32 B | 1,120 B |
| Client Initial | 1 datagram (1,200 B) | 2 datagrams (2,400 B) |
| Round trips | 1 | 1 |
| New connection, both directions | about 2.5 KB | about 4.8 KB |
| Kad lookup with operation (§10.5) | 15–25 KB | 30–45 KB |
| CPU per handshake | — | tens of microseconds more |

- **No extra round trip.** The ClientHello no longer fits one Initial packet, so the initiator
  sends two padded datagrams. The accepting node may then send up to 7,200 B before the address
  is validated (RFC 9000 §8.1). Its larger reply fits, because raw public keys keep the rest of
  its flight small.
- **The obfuscation layer is unaffected.** §4.4 splits each `HANDSHAKE` datagram into 2–4
  fragments; there are two datagrams to split instead of one.
- **Slightly more state before address validation.** The accepting node buffers the first
  Initial until the second arrives.

## 2. What it protects

Confidentiality of traffic that is recorded now and decrypted later. The recorder has to be on
the path: an access provider or a state.

| Traffic | Value of protecting it |
|---|---|
| Chat, browse lists | high |
| File requests and blocks (who fetched what) | medium to high |
| Kad requests | low: they go to arbitrary nodes anyway, and stored records are public |

It does **not** protect authentication. The handshake is still signed with Ed25519. Forging
that needs a quantum computer at the time of the attack, not afterwards, and is a question of
the node identity (§6.2), not of the key exchange.

## 3. Library support

- OpenSSL 3.5 and later include the group and offer it by default. A QUIC stack that takes its
  groups from the TLS library needs a configuration setting, not new code.
- BoringSSL, Go's standard TLS, rustls with aws-lc and wolfSSL support it, so independent
  implementations are not blocked.
- It is specified in `draft-ietf-tls-ecdhe-mlkem` (TLS group 0x11EC), an IETF draft at the time
  of writing, and is widely deployed by browsers and content networks.

## 4. Options for an upgrade

| | Rule | Cost | Weakness |
|---|---|---|---|
| **A. By purpose** | Every node accepts both groups. Connections for a peer session (file, chat, browse) use the hybrid group; a connection opened only for a Kad walk may use X25519 | About 2.3 KB once per peer session; Kad unchanged | The accepting node has to refuse a file, chat or browse exchange on a classical connection, or the rule is optional in practice |
| B. Always | Every connection uses the hybrid group | Kad lookups double | The datagram binding, the way to lower Kad cost, is not post-quantum either |
| C. Never | Stay with X25519 | none | Recorded traffic stays exposed |

Option A is the one to take if the upgrade is made: the traffic worth protecting is on
long-lived peer connections, where 2.3 KB is noise, and Kad carries nothing that is secret from
the network.

## 5. How it would be introduced

No new wire version and no capability bit are needed for the key exchange itself: the initiator
offers the group, and a node that does not know it selects X25519. The steps are those of a
network rule (§16):

1. Nodes start accepting and offering the hybrid group. Nothing is enforced.
2. From a date fixed in the specification, the rule of option A or B applies, and a node closes
   an exchange that needs the hybrid group on a connection without it.

Step 1 alone already protects every connection between two upgraded nodes.

## 6. Open points

- Handshake sizes are to be measured with the chosen QUIC stack.
- Whether the sessionless Kad binding ([`kad-datagram.md`](../extensions/kad-datagram.md)), which
  uses static X25519, needs a post-quantum form at all, given how little Kad traffic is secret.
