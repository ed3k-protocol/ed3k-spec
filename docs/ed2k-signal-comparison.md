# Signalling ed3k to eD2K peers: the chosen signals, and the alternatives

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §14 (coexistence with eD2K). Section numbers (§) refer to that document.

A client that speaks both eD2K and ed3k wants eD2K peers to learn, before any connection, that
it can also be reached over ed3k. This note records which signals §14 uses and which others were
considered.

**Result:** the port rule (§14.1) and the derived user hash (§14.2) as the signal that needs no
connection. Explicit tags ([`ed2k-tags.md`](ed2k-tags.md)) are an optional, authoritative
addition outside the specification. No other signal passes through legacy servers and legacy
relaying clients unchanged and also carries the node ID.

## 1. What the signal has to do

- **Arrive before a connection.** A source is learned from a server, from source exchange, from
  Kad or from a link. The signal should come with it.
- **Survive legacy relaying.** Servers and clients that know nothing of ed3k pass sources on.
  Whatever they rebuild, mask or drop is lost.
- **Change nothing a legacy client or server interprets.**
- **Carry enough to dial.** The endpoint, and the first 12 bytes of the node ID for the
  obfuscation key (§4.3).
- **Cost little when wrong.** A false signal should cost one unanswered handshake.

## 2. What §14 uses

| Signal | Reaches | Carries | Weakness |
|---|---|---|---|
| Port rule (§14.1) | every source record, since each has the TCP port | the ed3k UDP port | says nothing about whether the peer speaks ed3k |
| Derived user hash (§14.2) | every channel that carries the user hash | 12 bytes of the node ID | a hint: one legacy hash in 65,536 matches by accident; the user hash changes once, which loses credits at other peers |
| Explicit tags ([`ed2k-tags.md`](ed2k-tags.md)), optional | hello, Kad source publish, extended source exchange, server login, links | the wire version or the full node ID | not relayed by legacy servers or by legacy source exchange |

## 3. Other signals considered

| Signal | Why not the primary one |
|---|---|
| Reserved bits 4–7 of the connect-options byte | Reaches Kad (`FT_ENCRYPTION`) and source exchange v4, but legacy clients rebuild that byte from their own flags when relaying, and servers mask it to 3 bits and use `0x80` for "hash follows" |
| Free bits 14–31 of `CT_EMULE_MISCOPTIONS2`, or a `CT_MOD_MISCOPTIONS` bit | Only visible after a TCP hello |
| Free bits 3–7 of `TAG_KADMISCOPTIONS`, or a higher Kad version | Tells Kad contacts, not download sources |
| Fixed marker bytes in the user hash | Same one-time credit loss, but carries no node ID |
| A Kad record mapping user hash to node ID | A second lookup per source |
| Probing the port blindly | A wasted handshake against every legacy source |
