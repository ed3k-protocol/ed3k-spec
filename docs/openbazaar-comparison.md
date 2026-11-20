# OpenBazaar on ed3k

*Informative.* OpenBazaar (openbazaar-go, protocol of 2017–2021) is a peer-to-peer market on
libp2p and IPFS: stores as IPNS-named directories, orders as Ricardian contracts, escrow on
Bitcoin-like chains. This document maps it onto ed3k and lists what ed3k needed.

Result: two new extensions, [`mail.md`](../extensions/mail.md) (offline messages, record kind
16) and [`market.md`](../extensions/market.md) (bit 21, types 46/47), plus small fields in
[`chat.md`](../extensions/chat.md), [`pubsub.md`](../extensions/pubsub.md),
[`friend.md`](../extensions/friend.md). Money stays outside ed3k, as in OpenBazaar.

## 1. Mapping

| OpenBazaar | How | ed3k |
|---|---|---|
| Peer ID (libp2p Ed25519) | node identity | node key (`Protocol.md` §6.1) |
| Same key signs store | identity = store | separate store key ([`market.md`](../extensions/market.md) §1) |
| secp256k1 BIP 32 master key, `bitcoinSig` in `ID` | escrow keys derived per order | per-order chain keys in opaque `chain` bytes; no derivation in ed3k |
| IPNS root: `profile.json`, `listings.json`, `listings/`, `ratings/`, `posts/` | mutable directory | store file set, `HEAD` record ([`file-set.md`](../extensions/file-set.md) §5) |
| Handle (`@name`, blockchain/DNS) | readable name | `NAME`/`CLAIM`, DNS TXT ([`name.md`](../extensions/name.md)) |
| `SignedListing` JSON | one file per listing | `.ed3klst` file, signed by store key |
| Follow / unfollow messages, followers list | push + lists | pubsub `SET_HEAD` on the store set; no follower lists |
| Posts | IPNS directory | second set or a room ([`room.md`](../extensions/room.md)) |
| Chat (type 1) | online + offline | `Chat` + `subject`; offline by mail |
| Order flow types 4–13, 20, 21, 501 | 22 message types, contract grows | one `Market` message carrying signed contract sections (§4 there) |
| `RicardianContract` | listing + order + each party's signed parts | chain of `Section`s, each signed, linked by hash |
| Moderators: pointer at `sha256("moderators")` | DHT provider list | pubsub `DIRECTORY` topic per coin, `SUB` records |
| Ratings with vendor-signed rating keys | anonymous but buyer-only | same scheme, rating key signed at fulfilment, not confirmation ([`market.md`](../extensions/market.md) §6) |
| Offline messages: NaCl box, pointer in 14-bit prefix bucket, push nodes | store-and-forward | `MAIL` records in a bucket target, data in the records (§3) |
| Tor transport option | hide address | paths ([`hops.md`](../extensions/hops.md)) |
| search.ob1.io crawler | central index | `KEYWORD` with `ext = ed3klst`; richer search by an index node or crawler |
| Escrow: direct, 1-of-2 offline, 2-of-3 + timelock | on chain | unchanged, outside ed3k ([`market.md`](../extensions/market.md) §5) |
| HTTP API (~105 routes) | client UI | local, not protocol |

## 2. What fits without change

- Store data = signed changing set. IPNS record ≈ `HEAD`; republish ≈ head renewal; seeding by
  followers is what OpenBazaar's IPFS pinning did, with ed3k's normal file transfer.
- Images, digital goods: ed3k files, found by root; digital goods sealed with hidden-set data
  sealing and handed over by key.
- Address changes of a store's node: open friend link in the profile
  ([`friend.md`](../extensions/friend.md) §2).

## 3. Offline messages

OpenBazaar: sender encrypts to the recipient (NaCl box), stores the ciphertext at push nodes
or its own IPFS node, puts a **pointer** in the DHT under the first 14 bits of the recipient's
hash. Pointer lives 7 days, republished every 12 h up to 30 days. Recipient fetches every
pointer of its prefix, downloads, tries to open.

ed3k ([`mail.md`](../extensions/mail.md)):

| | OpenBazaar | ed3k mail |
|---|---|---|
| Place | DHT key = recipient hash prefix, 14 bits fixed | daily target from prefix of `b` bits, recipient-chosen 0–16 |
| What's stored | pointer to data elsewhere | sealed data itself, 1,024 B per record, ≤ 64 per message |
| Large payloads | any size at push node | sealed file, root + key by mail |
| Third party needed | push nodes (or sender online) | no; K storing nodes |
| Sender visible to storing node | peer ID in pointer | connection only; a mail-only leaf key recommended |
| Signature | inside envelope | inside, binding recipient key |
| Acknowledgement | `OFFLINE_ACK` | `MailAck` by mail, or online answer |
| Republish | every 12 h, 30 days | same bytes every 12 h, ≤ 8 days |

Why buckets: a per-recipient target tells storing nodes who gets mail. Why data in records:
no always-on hosts, no second fetch protocol; cost is 1 KiB granularity and a 57 KiB cap,
enough for most orders; larger ones go as a sealed file (`MailFile`).

## 4. Changes made to ed3k

| Where | Change |
|---|---|
| `Protocol.md` §2 | bit 21 `MARKET` |
| `Protocol.md` §5.1 | types 46 `Market`, 47 `MarketAck` |
| `Protocol.md` §10.1, §10.3 | kind 16 `MAIL`: target, enum, `Store` field 15, `KadAnswer` 24/25, lifetime, every `ROUTER` stores it |
| `Protocol.md` §10.7 | `signer` ignored for `MAIL` |
| `Protocol.md` §13.1, §13.2, App. A, B | tables, `mb` link parameter, constants, list of vectors |
| [`ip-diversity.md`](../extensions/ip-diversity.md) §2.4 | `MAIL` per-host/site limits |
| [`pubsub.md`](../extensions/pubsub.md) §1 | `TopicType` 4 `DIRECTORY` |
| [`chat.md`](../extensions/chat.md) | `subject` field 3; offline by mail |
| [`friend.md`](../extensions/friend.md) §1 | `mb` parameter |
| `proto/ed3k.proto` | all of the above |

`MAIL` must be in version 1: kinds added later are unknown to v1 `ROUTER`s (`Protocol.md`
§10.3), so a later kind would be stored by nobody until the network upgraded.

## 5. Not changed

- 16 push topics, 16 rooms per node: a buyer follows its stores by polling heads too.
- `FileInfo` price/category fields: listings are files; filters belong to an index.
- Credits ([`credit.md`](../extensions/credit.md)): not money (no transfer to a person, burn,
  decay); goods paid on chain.
