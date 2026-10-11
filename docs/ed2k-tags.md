# Explicit ed3k tags in eD2K: a proposal

**Status:** design note (2026-10-11). Informative; not part of wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §14 (coexistence with eD2K). Section numbers (§) refer to that document.

A client that speaks both eD2K and ed3k is recognised by eD2K peers through the port rule
(§14.1) and the derived user hash (§14.2). Both work without any new tag. This note proposes
tags that state ed3k support outright, for the places where a hint is not enough.

**Result:** tag ID `0xE3` for the hello, the server login, the server ident and extended source
exchange; the string name `"e3"` in Kad; the token `e3=` in links. All are unused in the seven
eMule source trees that were checked. The tags stay outside the specification, because a
dual-protocol client is complete without them.

## 1. What a tag adds over the user hash

| | Derived user hash (§14.2) | Explicit tag |
|---|---|---|
| Certainty | a hint: one legacy hash in 65,536 matches | authoritative |
| Carries | the first 12 bytes of the node ID | the full node ID, or the wire version |
| Passes legacy servers and relaying clients | yes | no |
| Reaches a server | no | yes, in the login |

A tag from the list below means "ed3k version `n`, port rule of §14.1".

## 2. The tags

| Channel | Signal | Note |
|---|---|---|
| Client hello (`OP_HELLO`, `OP_HELLOANSWER`) | tag `CT_ED3K` = `0xE3`, HASH type, value = the full 16-byte node ID | see §4 for a client that penalises unknown hello tags |
| Kad source publish | string-named tag `"e3"`, UINT8 = wire version | stored and returned verbatim by legacy nodes, like `"ip6"` |
| Extended source exchange record | tag `CT_ED3K` = `0xE3`, UINT8 = wire version | only to peers that skip unknown tags |
| Server login (`OP_LOGINREQUEST`) | tag `CT_ED3K` = `0xE3`, HASH type, value = the node ID | lets an ed3k-aware server mark sources and advertise an index node |
| Server ident (`OP_SERVERIDENT`) | tag `ST_ED3K` = `0xE3`, string, value = the server's `ed3k:node?…` link (§13.2) | how a client learns of a server's index node (§11) |
| eD2K link | token `e3=<base32 root>` | MUST be placed before the `sources` block, where legacy parsers stop reading |

**Binding the identities.** The hello tag ties the eD2K identity to the ed3k one after an eD2K
connection: if secure identification passed and the tag's node ID yields the peer's user hash
under §14.2, a client MAY carry the peer's eD2K credits over to the node ID.

## 3. Check against existing clients

The numbers were checked against the source of stock eMule 0.50a and 0.51d, eMuleQt and four
eMule mods (2026-10-11). Every `CT_`, `ET_` and `ST_` definition was collected, including the
lists of tags that some clients treat as a mark of a leecher mod.

| Signal | Result |
|---|---|
| `0xA1`, the number first proposed | **taken.** One mod sends `0xA1` in the hello as a HASH tag with its 16-byte Kad ID. The type and length are the same as `CT_ED3K`, so a node ID would be read as a Kad ID, and the other way round |
| `0xA0`–`0xAF` | not usable: a family of mod tags, partly assigned and partly reserved by their owner |
| `0xE3` | free in all seven trees, as a client tag, an eMule info tag and a server tag. Not on any leecher-tag list |
| `"e3"` as a Kad tag name | unused; the string names in use are `"ip"`, `"port"`, `"ip6"`, `"bi6"` and the five HTTP Cache names |
| `e3=` as a link token | unused; the tokens in use are `p=`, `h=`, `s=` and `s6=` |

Other numbers free in every tree, should `0xE3` turn out to be taken elsewhere: `0xC0`–`0xC3`,
`0xC5`, `0xC6`, `0xDB`–`0xE2`, `0xE4`, `0xE5`, `0xE7`–`0xEB`.

`0xE3` is also the eD2K protocol byte. That is a different field and no parser confuses the
two; the number was chosen because it is easy to recognise in a packet dump.

## 4. Unknown hello tags are not always harmless

Stock eMule ignores a hello tag it does not know. Two of the checked mods do more:

- One keeps a list of tag numbers used by known leecher mods and bans their senders. `0xE3` is
  not on it.
- One treats **any** unknown hello tag as a sign of a hidden mod when the sender announces no
  readable mod name, and penalises the sender.

A client that sends `CT_ED3K` in the hello SHOULD therefore also send a mod version string
(`CT_MOD_VERSION`, `0x55`). The Kad, source exchange, login and link signals are not affected.

## 5. Open points

- The check covers the source trees at hand. `0xE3` and `"e3"` should be confirmed with the
  authors of other clients and of server software before first use.
- Whether a server relays the login tag to other clients, for example as one more bit in the
  per-source option byte of `OP_FOUNDSOURCES_OBFU`. That byte has no free bit that legacy
  clients preserve (see [`ed2k-signal-comparison.md`](ed2k-signal-comparison.md) §3).
