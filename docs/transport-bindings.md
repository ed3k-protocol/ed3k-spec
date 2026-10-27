# Adding a second transport binding: the gap and a path

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §3 (transport contract), §4 (`quic-obfs`), §5.5 (`Hello`) and §16 (versioning). Section numbers (§) refer to that document.

The messages of ed3k are written against a transport contract (T1–T5), so that another binding
can carry them unchanged. Version 1 specifies one binding, `quic-obfs`, and says nothing about
how a node would announce or choose a second one. This note lists what is missing and describes
a way to add a binding later without a new wire version.

**Result:** the messages are ready for a second binding; the addressing is not. The gap can be
closed additively, as long as `quic-obfs` stays the binding every node has.

## 1. What a new binding has to bring

| Contract | What the binding must provide |
|---|---|
| T1 | proof of the peer's Ed25519 key, in both directions, so that the node ID is the same on every binding |
| T2 | encryption and integrity |
| T3 | a validated remote address before more than a bounded reply is sent |
| T4 | many independent, reliable, flow-controlled exchanges on one connection |
| T5 | obfuscation of its own, unless the transport already hides itself |

Framing, compression, messages, node IDs, Kad records and signatures are above the contract and
do not change.

## 2. The gap in version 1

| | What version 1 has | What is missing |
|---|---|---|
| Endpoints | `Peer`, `Hello`, compact nodes and `ed3k:node` links carry one UDP endpoint per family | no field for an endpoint of another transport (for example a TCP port) |
| Which bindings | every endpoint is `quic-obfs` by definition | no list of the bindings a node accepts |
| Choice | one binding, nothing to choose | no rule for which binding to dial first, and when to fall back |
| Kad contacts | compact nodes of 22 and 34 bytes, fixed layout | no room for a binding or a second port |
| NAT traversal | `ReachTest`, relays and `Punch` (§9) are built on UDP datagrams | nothing for a transport that cannot be hole-punched the same way |
| Address checks | the routing ID (§6.3) is verified against the address of the QUIC connection | no statement that an address validated over another binding counts |
| Reachability class | `Hello.nat` describes UDP reachability | no class per binding |
| eD2K coexistence | the port rule (§14.1) names a UDP port | no signal for another transport |

None of this prevents a second binding. It means that the first one to be added has to define
these points, and this note proposes how.

## 3. A path

The path rests on one rule: **`quic-obfs` stays mandatory while a second binding is introduced.**
Every node can then still be found and reached as today, and the new binding is an addition
that two nodes use when both have it.

### Step 1: announce

- A registry of binding IDs. `quic-obfs` is 1.
- `Hello` gains a list of the other bindings the node accepts, each with its endpoints:

  ```protobuf
  message Binding {
    uint32 id = 1;            // registry number
    bytes  ep4 = 2;           // endpoint for this binding
    bytes  ep6 = 3;
  }
  // in Hello:  repeated Binding bindings = 11;
  // in Peer:   repeated Binding bindings = 6;
  ```

  Both field numbers are reserved in version 1 (`Protocol.md` §5.4, §5.5). An old node ignores
  unknown fields (§16), so the addition is compatible.
- Kad contacts stay as they are. A node learns a peer's other bindings from the `Hello` of a
  `quic-obfs` connection, or from the `Peer` record of a source.

At this step a node discovers over `quic-obfs` and may move a peer session to another binding.

### Step 2: dial rule

- A node dials the binding it prefers among those both sides have, and falls back to
  `quic-obfs` after a short delay, in the manner of the IPv6-first rule of §8.3.
- An address validated by any binding that meets T3 counts for the routing ID check.
- The node ID is the same on every binding, so credits, source records and friends carry over.

### Step 3: nodes that cannot use `quic-obfs` at all

The main reason for a second binding is a network that blocks or throttles UDP. Such a node
cannot bootstrap over `quic-obfs`, so steps 1 and 2 do not help it. It needs:

- **Entry points.** `ed3k:node` links and bootstrap lists that name a binding, for example
  `&b=<id>,<ip:port>`.
- **Contacts it can dial.** `KadRequest.want` gains bits for further bindings, and `KadAnswer`
  gains a compact contact list per binding, next to `nodes4` and `nodes6`.
- **A way to be reached.** It is a leaf. The relays of §9.4 forward signalling only, and
  hole punching is specific to UDP, so such a node needs either peers that it dials itself or
  the data relay that §9.4 leaves for later.

This step is the larger one. It changes Kad answers and depends on enough routers offering the
second binding.

### Step 4: making a binding mandatory, or retiring one

Both are network rules (§16): a capability bit announces support, and a date fixed in the
specification switches the rule. Retiring `quic-obfs` would also mean new compact contact
formats, because their layout assumes it. That is the point at which a new wire version
(`ed3k/2`) is warranted, not before.

## 4. What each binding would need to define for itself

- Its identifier inside the transport (an ALPN string or the equivalent) and, if it adds
  obfuscation, a `magic` value of its own.
- How T1 is met. With TLS, raw public keys as in §4.1.
- How exchanges map to the transport: QUIC has streams; a TCP binding needs a multiplexer.
- Its reachability classes and how they are tested.
- Whether 0-RTT or an equivalent exists, and for which messages.

## 5. Why this is larger than other upgrades

| | New zstd dictionary | New signature algorithm | New binding |
|---|---|---|---|
| What changes | a data file and its ID | one more entry in a list | handshake, encryption, address proof, multiplexing, obfuscation |
| Announced by | `Hello.dict` | a network upgrade | new endpoint fields, later new Kad contact lists |
| If the peer lacks it | larger messages | the record is still valid until the algorithm is required | no connection, unless both still share a binding |
| NAT traversal and address checks | untouched | untouched | redone for the new transport |

## 6. Candidates

| Binding | Why it might be wanted | Main difficulty |
|---|---|---|
| TLS over TCP with a stream multiplexer | networks that block or throttle UDP | head-of-line blocking; no hole punching; a second listening port |
| WebTransport | browser clients | needs HTTP/3 and certificates that browsers accept, which conflicts with raw public keys |
| WebRTC data channels | browser clients, also browser to browser | a second UDP port, signalling through relays; see [`browser-bindings.md`](browser-bindings.md) |
| In-memory | tests | none; it needs no advertisement |

Ways to serve browsers, with and without a binding, are compared in
[`browser-bindings.md`](browser-bindings.md).

## 7. Open points

- Whether to reserve the two field numbers of §3 in version 1, so that no other extension takes
  them.
- Whether a node that is reachable only over a second binding may set `ROUTER`.
- How the eD2K coexistence signals (§14) would name a second binding.
