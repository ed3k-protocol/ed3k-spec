# Extension: market (`market`)

**Status:** draft 0.1 (2026-11-18). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §13; [`file-set.md`](file-set.md) §5; [`pubsub.md`](pubsub.md) §1; [`mail.md`](mail.md) §3; [`friend.md`](friend.md) §2; [`name.md`](name.md). § = this document unless a document is named.
**Capability:** bit 21, `MARKET`. Message types 46 and 47. Three file formats. One `TopicType`. Mail payload type `market`.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Market*.
**Required of every `ROUTER`:** nothing. Uses record kinds every `ROUTER` already stores (`HEAD`, `KEYWORD`, `SUB`, `FRIEND`, `MAIL`, `NAME`).

A peer-to-peer market after OpenBazaar: stores, listings, orders under a Ricardian contract,
escrow with a moderator, ratings. ed3k carries **everything but the money**: payment, escrow
scripts and wallets are outside, per coin; ed3k carries signed documents and the signatures a
chain needs. Mapping from OpenBazaar: [`docs/openbazaar-comparison.md`](../docs/openbazaar-comparison.md).

| Piece | ed3k element |
|---|---|
| Store (profile, listings, ratings) | changing file set under a **store key** ([`file-set.md`](file-set.md) §5) |
| Store handle | `NAME` / `CLAIM` ([`name.md`](name.md)) |
| Following a store | pubsub `SET_HEAD` on the store set |
| Finding listings | `KEYWORD` records of listing files, `Query.ext = "ed3klst"` |
| Finding moderators | pubsub topic `DIRECTORY` (§3.2) |
| Reaching a store's node | open friend link in the profile ([`friend.md`](friend.md) §2) |
| Order messages, online | `Market` / `MarketAck` (§4) |
| Order messages, offline | mail, type `market` ([`mail.md`](mail.md)) |
| Chat about an order | `Chat` with `subject` ([`chat.md`](chat.md) §1) |
| Payment, escrow | outside ed3k; opaque `chain` bytes (§5) |

---

## 1. Keys

- **Store key**: Ed25519, signs the store set, profile, listings, a vendor's contract sections.
  Not the node key: a store moves between nodes, a node runs several stores, a store key can
  live offline.
- **Buyer key**: key signing a buyer's sections: its store key if it has one, else any
  Ed25519 key it keeps for the order's lifetime (node key allowed).
- A node acts for a key when the key is its node key, or the key's profile lists the node key
  in `nodes` (§2.2). Proven by the handshake (T1) plus the signed profile.
- Rating key: one-off Ed25519 key per ordered item (§6).
- Chain keys (escrow): per coin, per order, in `chain` bytes; ed3k doesn't derive them.

## 2. Store

### 2.1 The store set

Changing file set ([`file-set.md`](file-set.md) §5.1), signed by the store key, `id` =
`store`. Layout:

| Path | Content |
|---|---|
| `profile.ed3kprf` | profile (§2.2), exactly one |
| `listings/<slug>.ed3klst` | one listing per file (§2.3) |
| `ratings/<order hex>-<n>.ed3krat` | ratings received (§6) |

Other paths allowed, ignored by this document (images, a second set for posts, …). Images
referenced by root from the documents, MAY be members of the set.

- Store node follows its own set (pubsub), republishes the head.
- A client SHOULD seed stores it follows or bought from, as any set it holds
  ([`file-set.md`](file-set.md) §3): a store stays readable while its owner is offline.

### 2.2 Common wrapper, profile

Every document file: 8-byte magic ‖ `MarketDoc`.

| File | Magic | Body | Max size |
|---|---|---|---|
| profile | `ed3kprf` 0x01 | `Profile` | 64 KiB |
| listing | `ed3klst` 0x01 | `Listing` | 1 MiB |
| rating | `ed3krat` 0x01 | `Rating` | 16 KiB |

```protobuf
message MarketDoc {
  bytes  body = 1;            // serialised Profile, Listing or Rating
  bytes  key = 2;             // 32 B signing key: store key (profile, listing), rating key (rating)
  repeated Signature sigs = 3;  // over "ed3k v1 market doc" ‖ magic(8) ‖ body
}

message Profile {
  string name = 1;            // at most 64 B
  string handle = 2;          // optional ed3k:name of the store (name.md §1.3)
  string about = 3;           // at most 16 KiB
  bytes  avatar = 4;          // optional 32 B root of an image
  bytes  header = 5;          // optional 32 B root of an image
  repeated bytes nodes = 6;   // 32 B node keys acting for this key (§1); at most 8
  string link = 7;            // ed3k:friend open link of a node in `nodes` (friend.md §2)
  MailBox mailbox = 8;        // where orders go while offline
  repeated string coins = 9;  // accepted coins, ticker strings (§5)
  Moderator moderator = 10;   // present = this store offers moderation
  string location = 11;       // free text, at most 128 B
  repeated string contact = 12;  // free-form contact lines, each at most 256 B, at most 8
  bool   nsfw = 13;
  uint64 updated = 14;        // Unix time in seconds
}

message MailBox {
  bytes  key = 1;             // 32 B mailbox key (mail.md §1); usually the store key
  uint32 b = 2;               // bucket width 0 to 16
}

message Moderator {
  string terms = 1;           // at most 16 KiB
  repeated string languages = 2;  // BCP 47 tags
  Price  fee_fixed = 3;       // optional
  uint32 fee_permille = 4;    // of the disputed amount
  repeated string coins = 5;  // coins it moderates
}
```

Reader checks: magic, size, `key` 32 B and = the set's key (profile, listing), signature per
`Protocol.md` §10.7 rules. Failing → document ignored.

### 2.3 Listing

```protobuf
message Listing {
  string slug = 1;            // 1 to 64 B of [a-z0-9-]; file name is <slug>.ed3klst
  ListingType type = 2;
  string title = 3;           // at most 140 B
  string description = 4;     // at most 64 KiB
  repeated string tags = 5;   // each at most 40 B, at most 16
  repeated string categories = 6;
  repeated Image images = 7;  // at most 32
  Price  price = 8;           // per item; for CRYPTO: per unit of `coin`
  repeated string coins = 9;  // accepted; empty = profile's
  repeated Option options = 10;
  repeated Sku skus = 11;
  repeated Shipping shipping = 12;  // PHYSICAL only
  repeated bytes moderators = 13;   // 32 B store keys of moderators the vendor accepts
  string terms = 14;
  string refund_policy = 15;
  string condition = 16;
  bool   nsfw = 17;
  uint64 expires = 18;        // Unix time; 0 = none
  uint32 escrow_hours = 19;   // moderated order: vendor may claim after; 0 = coin default
  repeated bytes coupons = 20;  // 32 B H("ed3k v1 market coupon", code); discounts in `coupon_off`
  repeated Price coupon_off = 21;  // one per coupon
  string coin = 22;           // CRYPTO: coin sold
  uint64 seq = 23;            // version; higher replaces, also across a slug rename
}

enum ListingType { LISTING_NONE = 0; PHYSICAL = 1; DIGITAL = 2; SERVICE = 3; CRYPTO = 4; }

message Price {
  string currency = 1;        // ISO 4217 or coin ticker
  uint64 amount = 2;          // in smallest unit
  uint32 divisibility = 3;    // decimal places of `amount`
}

message Image { bytes root = 1; uint64 size = 2; string name = 3; }  // 32 B root

message Option { string name = 1; repeated string values = 2; }

message Sku {
  repeated uint32 choice = 1; // index into each option's values, in option order
  int64  surcharge = 2;       // added to price.amount, may be negative
  int64  quantity = 3;        // −1 = unlimited
  string product_id = 4;
}

message Shipping {
  string name = 1;
  repeated string regions = 2;  // ISO 3166-1 alpha-2, or "ALL"
  repeated Service services = 3;
}

message Service { string name = 1; Price price = 2; Price extra_item = 3; string estimate = 4; }
```

- Listing is identified by (store key, `slug`); its **hash** = `H("ed3k v1 market listing",
  whole file)`, what an order quotes (§4.2).
- Member path keeps the slug; `KEYWORD` records carry `FileInfo.name` = `<title>.ed3klst`
  (title without `/` and control characters, cut to 255 B), so keyword search finds titles
  (§3.1).
- New version: new file, same path, higher `seq`; store set gets a new manifest.

## 3. Finding

### 3.1 Listings

- Store node publishes `KEYWORD` records for each listing file (`Protocol.md` §10.1), name as
  in §2.3. Client searches with `Query.ext = "ed3klst"`.
- `MEMBER` record of a listing file (file-set.md §6) leads to the store set: from any listing
  to its store.
- Price, category, region filters: not in `FileInfo`. An index node or crawler reads listings
  and answers richer searches by own means; not in this document (§9).

### 3.2 Moderators

```protobuf
// in enum TopicType (pubsub.md §1):
DIRECTORY = 4;                // key = 32 zero bytes, id = UTF-8 directory name; no items, SUB records only
```

- Directory `market.moderators`: every node acting for a moderator stores `SUB` under
  `H("ed3k v1 topic", 4 ‖ 0³² ‖ "market.moderators")[0..16)` (pubsub.md §5).
- Client searches `SUB` there, dials, asks `Market{who}`, gets the store keys the node acts
  for (`MarketAck.stores`), reads their profiles; keeps those with `moderator`.
- `DIRECTORY` topics have no items: `TopicJoin` for one → `Error{UNSUPPORTED}`. Other
  extensions MAY name further directories.

### 3.3 Reaching a store

Online: profile `link` → friend lookup (friend.md §4) → dial → `Market`. Node must act for
the store (§1); else the client drops it. Not reachable within 2 min *(initial)*, or no link:
mail to `mailbox`.

## 4. Messages

### 4.1 Frames

```protobuf
message Market {              // type 46, request
  fixed64 id = 1;             // replay id, as Chat.id (chat.md §2); same by mail and online
  bytes   from = 2;           // 32 B key the sender acts for (§1)
  bytes   to = 3;             // 32 B key addressed: store, buyer or moderator key
  bytes   order = 4;          // 32 B order id (§4.2); empty for `who`
  repeated Section sections = 5;  // new sections, in chain order
  bool    who = 6;            // ask which keys the node acts for; nothing else set
  bool    resend = 7;         // ask the receiver for every section of `order` it has
}

message MarketAck {           // type 47, answer
  repeated bytes stores = 1;  // answer to `who`: 32 B keys this node acts for
  uint32  have = 2;           // sections of `order` the receiver now holds
  string  error = 3;          // section refused: reason, at most 256 B; nothing appended
}
```

- `Market` is one request/answer exchange (`Protocol.md` §5.2); answered by `MarketAck` once taken.
  Without `MARKET`: `Error{UNSUPPORTED}`.
- Receiver: drops unless the node acts for `to`, else `Error{DENIED}`. Sender acts for `from`:
  checked by the handshake + `from`'s profile. By mail ([`mail.md`](mail.md) §3, type `market`,
  payload = serialised `Market`): `MailHead.from` MUST equal `Market.from`, or be listed in its
  profile's `nodes`.
- Repeated (`from`, `id`) → `MarketAck`, handled once. Rate limits: as chat.
- Frame ≤ 1 MiB (`Protocol.md` §5.2); by mail ≤ 64 parts, larger → `MailFile`
  ([`mail.md`](mail.md) §3.3).

### 4.2 The contract

A **contract** = chain of signed sections; each party appends its own and sends them. Every
party keeps the whole chain; it is the Ricardian contract.

```protobuf
message Section {
  SectionKind kind = 1;
  bytes  body = 2;            // serialised message of `kind` (§4.3)
  bytes  key = 3;             // 32 B signer
  repeated Signature sigs = 4;  // over "ed3k v1 market section" ‖ prev(32) ‖ kind(1) ‖ body
}

enum SectionKind {
  SECTION_NONE = 0;
  ORDER = 1;  CONFIRM = 2;  REJECT = 3;  CANCEL = 4;  PAYMENT = 5;
  FULFILL = 6;  COMPLETE = 7;  DISPUTE = 8;  DISPUTE_UPDATE = 9;  RESOLUTION = 10;
  ACCEPT = 11;  REFUND = 12;  FINALIZED = 13;  FAILURE = 14;
}
```

```
order id = H("ed3k v1 market order", body of the ORDER section)
prev     = order id for the section after ORDER; then H("ed3k v1 market chain", prev ‖ kind(1) ‖ body)
         ORDER's own prev = 32 zero bytes
```

- One chain per order, at most 64 *(initial)* sections. `prev` binds each section to all before.
- Receiver appends a section only when: signature valid, `key` is the party the kind allows
  (table §4.3), kind allowed after the chain's last state (table), `prev` matches. Else
  `MarketAck.error`, nothing appended. Missing earlier sections → `resend`.
- Two valid sections for the same step (a race: buyer `CANCEL`, vendor `CONFIRM`) → first one in
  chain order a party received wins; the other party learns by `resend`. Money moves only on
  chain, so a chain conflict is settled there (§5).

### 4.3 Sections

| Kind | Signer | After | Body |
|---|---|---|---|
| `ORDER` | buyer | — | `Order` |
| `CONFIRM` | vendor | `ORDER`, `PAYMENT` | `Confirm` |
| `REJECT` | vendor | `ORDER`, `PAYMENT` | `MarketNote` |
| `CANCEL` | buyer | `ORDER`, `PAYMENT` (offline-paid, not confirmed) | `MarketNote` |
| `PAYMENT` | buyer | `ORDER`, `CONFIRM` | `Payment` |
| `FULFILL` | vendor | `CONFIRM`, `PAYMENT`, `FULFILL` | `Fulfill` |
| `COMPLETE` | buyer | `FULFILL` | `Complete` |
| `DISPUTE` | buyer or vendor | `CONFIRM` to `FULFILL`; vendor also after its `escrow_hours` | `MarketNote` |
| `DISPUTE_UPDATE` | moderator | `DISPUTE` | `MarketNote`: moderator has both chains |
| `RESOLUTION` | moderator | `DISPUTE`, `DISPUTE_UPDATE` | `Resolution` |
| `ACCEPT` | buyer or vendor | `RESOLUTION` | `Payment` (release signatures) |
| `REFUND` | vendor | any state after `CONFIRM`, before `COMPLETE` | `Payment` |
| `FINALIZED` | vendor | `FULFILL` after `escrow_hours`, no `COMPLETE` or `DISPUTE` | `Payment` |
| `FAILURE` | vendor | `ORDER`, `PAYMENT` | `MarketNote`: order couldn't be processed |

Moderator receives the chain from the party opening `DISPUTE` (that party sends every section
to the moderator, `to` = moderator key) and the other party's on `resend`; both chains from
`ORDER` on must agree up to the dispute, else it decides on the signed sections both share.

```protobuf
message Order {
  bytes  vendor = 1;          // 32 B store key
  repeated bytes listings = 2;  // whole listing files ordered from, as published
  repeated Item items = 3;
  PayMethod method = 4;
  string coin = 5;
  bytes  moderator = 6;       // 32 B, MODERATED only; one of the listing's `moderators`
  bytes  chain = 7;           // coin-specific: buyer's escrow key, refund address (§5)
  string ship_to = 8;         // address text; the order travels sealed
  string contact = 9;
  MailBox mailbox = 10;       // buyer's, for offline answers
  string link = 11;           // optional open friend link of the buyer's node
  uint64 issued = 12;
  Price  total = 13;          // as the buyer computed it
}

message Item {
  bytes  listing = 1;         // 32 B listing hash (§2.3)
  uint64 quantity = 2;
  repeated uint32 choice = 3; // as Sku.choice
  string shipping = 4;        // Shipping.name ‖ "/" ‖ Service.name
  repeated string coupons = 5;  // codes, checked against Listing.coupons
  bytes  rating_key = 6;      // 32 B one-off Ed25519 key (§6)
  string memo = 7;
}

enum PayMethod { PAY_NONE = 0; DIRECT = 1; OFFLINE = 2; MODERATED = 3; }

message Confirm {
  bytes  chain = 1;           // coin-specific: vendor's escrow key, payment address/script, amount due
  repeated bytes rating_sigs = 2;  // per Item, 64 B Ed25519 by vendor over "ed3k v1 market rating key" ‖ order id ‖ index(4) ‖ rating_key
  Price  total = 3;           // vendor's figure; client shows any difference to the buyer
}

message Payment { string coin = 1; repeated string txids = 2; uint64 amount = 3; bytes chain = 4; }  // chain: signatures, scripts

message Fulfill {
  repeated Delivery delivery = 1;  // one per Item
  bytes  chain = 2;           // vendor's release signatures (OFFLINE, MODERATED), payout address
  string note = 3;
}

message Delivery {
  string carrier = 1;
  string tracking = 2;
  MailFile file = 3;          // DIGITAL: sealed file, mail.md §3.3
  string text = 4;            // SERVICE / CRYPTO: what was done or sent
}

message Complete {
  repeated bytes ratings = 1; // whole rating files (§6), one per Item the buyer rates
  bytes  chain = 2;           // buyer's release signatures
}

message Resolution {
  uint64 to_buyer = 1;        // smallest units of `coin`
  uint64 to_vendor = 2;
  uint64 to_moderator = 3;
  bytes  chain = 4;           // moderator's signatures
  string text = 5;
}

message MarketNote { string text = 1; }   // at most 4 KiB
```

## 5. Payment

Out of ed3k. Per coin a document outside this one (*informative*: OpenBazaar's escrow) fixes
what the `chain` bytes hold:

| `PayMethod` | Money | ed3k carries |
|---|---|---|
| `DIRECT` | buyer pays vendor's address in `Confirm` | address, txid |
| `OFFLINE` | buyer pays a 1-of-2 address (buyer, vendor) while vendor offline | both keys, signatures |
| `MODERATED` | 2-of-3 (buyer, vendor, moderator), vendor alone after `escrow_hours` (timelock) | keys, signatures of release, resolution |

- A node MUST NOT move money on a section alone: it checks the chain.
- Credits ([`credit.md`](credit.md)) are no payment for goods: not transferable to a person,
  burned on transfer, decaying.

## 6. Ratings

- Buyer puts one `rating_key` per item in `ORDER`; vendor signs each in `CONFIRM`
  (`rating_sigs`). Only buyers of a confirmed order hold such a key.
- Rating file: `MarketDoc` with magic `ed3krat` 0x01, `key` = rating key, body `Rating`.
- Buyer sends ratings in `COMPLETE`. Vendor adds them to `ratings/` of its store set. Buyer MAY
  also keep them in its own store set: vendor can drop bad ratings from its own set, not from
  others'.

```protobuf
message Rating {
  bytes  vendor = 1;          // 32 B store key
  bytes  order = 2;           // 32 B order id
  uint32 index = 3;           // Item index
  bytes  vendor_sig = 4;      // the rating_sigs entry
  string slug = 5;
  uint32 overall = 6;         // 1..5
  uint32 quality = 7;  uint32 description = 8;  uint32 delivery = 9;  uint32 service = 10;   // 0 = none, else 1..5
  string review = 11;         // at most 4 KiB
  uint64 issued = 12;
  bytes  buyer = 13;          // optional 32 B buyer key, not anonymous
  bytes  buyer_sig = 14;      // by `buyer` over "ed3k v1 market rating buyer" ‖ rating key(32)
}
```

Reader checks: `vendor_sig` under `vendor` for (`order`, `index`, rating key); `buyer_sig`
when `buyer` set. Counts one rating per (order, index).

## 7. What it shows

| To whom | What |
|---|---|
| Anyone | store's profile, listings, ratings; who seeds it; the store key's nodes in `nodes` |
| Vendor | buyer key, ship-to, contact |
| Moderator | the whole chain of a disputed order |
| Storing nodes | `HEAD`/`KEYWORD` of a store as of any set; mail volume in buckets ([`mail.md`](mail.md) §7) |
| Chain observers | payments; outside ed3k |

A store that wants its address hidden: nodes in `nodes` reached through paths
([`hops.md`](hops.md)), mail only.

## 8. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Mirror OpenBazaar's JSON/protobuf messages 1:1** | reuse its code | ties to its BIP 32 key layout and IPFS paths; 22 types instead of one chain |
| **Store key = node key** (OpenBazaar) | one key | store can't move, node can't hide its stores |
| **Listings as Kad records** | no file per listing | 1 KiB limit; no images; no versions |
| **Payment in credits** | no chain | credits aren't money (§5) |

## 9. Open points

- Price/category/region search: crawler or index node service, not specified.
- `chain` formats per coin: separate documents.
- Vendor blocking buyers, moderator add/remove notices (OpenBazaar 16, 17, 19): local policy.
- All numbers are first values.
