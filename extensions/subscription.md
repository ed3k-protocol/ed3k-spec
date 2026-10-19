# Extension: paid subscriptions (`subscription`)

**Status:** draft 0.1 (2026-10-16). Not implemented. Optional in wire version 1.
**Extends:** [`live.md`](live.md) §1.2, §2.2, §3 and §10, [`file-set.md`](file-set.md) §1 and §5, [`hidden-set.md`](hidden-set.md) §1 to §4, [`pubsub.md`](pubsub.md) §1 and §2, and [`Protocol.md`](../Protocol.md) §13. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 18, `PAID`. Message types 40 and 41. Fields in `Channel`, `LiveSegment`, `LiveJoin`, `LiveState`, `FileSet`, `Member`, `Sealed`, `TopicJoin`, `TopicState` and `TopicItem`; `paid` in `ed3k:live` and `ed3k:set` links; `ed3k:pass` links; a report to the seller over HTTPS.
**Required of every `ROUTER`:** nothing. The extension adds no record kind.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Paid subscriptions*.

A private channel ([`live.md`](live.md) §10) and a hidden set ([`hidden-set.md`](hidden-set.md))
have one secret for everyone. A viewer who passes the link on gives full access, and access
cannot be taken away. This extension gives each viewer access of their own, for a time they
paid for.

- A publisher, or an issuer it names, signs a **pass** for a viewer after the viewer paid. How
  the viewer pays is not part of ed3k: a card, a bank transfer, a chain, a site, a gift.
- The viewer gets a **key per period** (an **epoch**) from any node that already holds it and
  that checks the pass. Nobody has to be online for the publisher between two epochs.
- Epoch keys form **reverse hash chains**, one per **season** of about a month. The key of one
  epoch opens every earlier epoch of its season, and nothing after it. A new subscriber gets
  the tip of every earlier season from other pass holders, and so the back catalogue.
- **A leaked key opens one season at most**, up to the epoch it belongs to. The publisher can
  start a new season at the next epoch (§4.5), and keeps the seed that produces a season off
  the nodes that are online (§4.4).
- A publisher can **release** old epochs on a schedule: early access that becomes free later.
- Neighbours of a paid stream and of a paid set's topic are pass holders. A publisher can give
  **relay passes**, without keys, to nodes that only carry data.

What is sealed is fixed by a signed root or a signature, as in a hidden set. The extension
changes nothing in how blocks are verified, stored or passed on.

---

## 1. Parts

| Part | What it is | Signed by | Section |
|---|---|---|---|
| **Scope** | what one pays for: a live channel, a set, or a hidden set | (derived) | 2 |
| **Offer** | the price, the payment URL, the issuers and the current season of a scope | the publisher | 3 |
| **Epoch key** | the key of one epoch; opens it and every epoch before it in its season | (chain) | 4 |
| **Pass** | a viewer's key, the scopes and the time until which they are paid | the publisher or an issuer | 5 |
| **Key exchange** | how a pass holder gets an epoch key from another node | | 6 |
| **Sealing** | what a key opens in a channel and in a set | | 7, 8 |
| **Purchase** | how a client gets a pass after the viewer paid | the issuer | 9 |

The **publisher** is the key that owns the scope: the channel key ([`live.md`](live.md) §1.1),
the author key of a set ([`file-set.md`](file-set.md) §5.1), the owner key of a set with
several authors ([`multi-author-set.md`](multi-author-set.md)), or the key of a hidden set
([`hidden-set.md`](hidden-set.md) §1).

---

## 2. Scope

```
scope = H("ed3k v1 paid scope", kind(1) ‖ key ‖ id)[0..16)
```

| `kind` | Scope | `key` | `id` |
|---|---|---|---|
| 1 | a live channel, open or private | channel key | channel `id` |
| 2 | a set with an open manifest | author or owner key | `FileSet.id` |
| 3 | a hidden set | set key | empty |

- One scope has one offer at a time (§3) and one row of epochs.
- Anyone who has the link or the channel file computes `scope`. It is not a secret.
- Several qualities of one channel are several channels ([`live.md`](live.md) §12), and so
  several scopes. One pass can name all of them (§5).

---

## 3. Offer

```protobuf
message Offer {
  bytes  scope = 1;           // 16 B
  uint64 issued = 2;          // Unix time in seconds; a later offer replaces an earlier one; at most 1 h ahead
  uint64 t0 = 3;              // start of epoch 0, Unix time in seconds
  uint32 period = 4;          // length of one epoch in seconds, 60 to 2,678,400
  SignedChain chain = 5;      // the current season (§4.2)
  repeated bytes issuers = 6; // 32 B Ed25519 keys that may sign passes besides the publisher; at most 8
  string price = 7;           // for display, at most 128 B of UTF-8, for example "4.99 EUR / month"
  string pay_url = 8;         // https URL, at most 1,024 B (§9)
  string pass_url = 9;        // https URL, at most 1,024 B (§9)
  uint32 free_after = 10;     // an epoch is released this many epochs after it began; 0 = never (§4.3)
  string terms = 11;          // for display, at most 1,024 B of UTF-8
  string report_url = 12;     // https URL, at most 1,024 B; absent = no reports (§5.3)
  uint32 devices = 13;        // node IDs one holder key may use at once, 1 to 63; 0 = 3 (§5.3)
}

message Chain {
  bytes  scope = 1;           // 16 B
  uint64 base = 2;            // first epoch of this chain; at most the epoch of now + 1 h, plus 1 (§3)
  uint32 length = 3;          // number of epochs, 1 to 2^20
  bytes  anchor = 4;          // 32 B, H("ed3k v1 paid anchor", scope ‖ K(base))
}

message SignedChain {
  bytes body = 1;             // a serialised Chain, verified over exactly these bytes
  repeated Signature sigs = 2;    // by the publisher, over "ed3k v1 paid season" ‖ body
}

message SignedOffer {
  bytes body = 1;             // a serialised Offer, verified over exactly these bytes
  repeated Signature sigs = 2;    // by the publisher, over "ed3k v1 paid offer" ‖ body
}
```

- The signature is by the publisher. A reader knows the publisher key from the link, the
  channel file or the manifest, and refuses an offer whose signature fails or whose `scope` is
  not the one it computed.
- A `SignedOffer` MUST NOT exceed 4 KiB. The total wins: an offer whose fields each stay
  within their own limits and that is larger is refused all the same.
- The rules for `sigs` of `SignedOffer`, `SignedChain` and `SignedPass` are those of
  [`Protocol.md`](../Protocol.md) §10.7.
- `devices` is 0 to 63; a reader refuses an offer with a higher value (§5.3).
- `t0` and `period` never change for a scope. An offer that changes them is refused, and the
  one held is kept.
- **Nothing far ahead.** A reader refuses an offer whose `issued` is more than 1 h *(initial)*
  ahead of its clock, and a chain whose `base` is above the epoch of its clock plus 1 h
  *(initial)*, plus 1 (0 before `t0`). The hour is the clock tolerance of the offer; with short
  periods it keeps a reader whose clock is behind from refusing a season that has begun. An
  offer that names such a chain is refused, and the one held is kept; a key that comes with
  such a chain is dropped. A thief of the publisher key can then not sign an
  `issued` or a `base` so far ahead that no later offer or season of the publisher replaces it.
- A chain covers epochs from `base` up to `base + length − 1`. A newer offer keeps the chain
  of the one held, or names one with a higher `base`. An offer whose chain has a lower `base`,
  or the same `base` with another `length` or `anchor`, is refused.
- **No gaps.** A publisher begins a new season at most at `base + length` of the one before. A
  reader does not check this: it cannot tell a gap from a season it has not seen. A gap only
  leaves epochs without a key. A chain whose `scope` is not the offer's, or whose signature
  fails, makes the offer invalid.
- **The offer names the current season.** `chain` is the season of the epoch of `issued`, or,
  after a leak, the new season that begins at the next epoch (§4.5). A planned season (§4.4) is
  not named in an offer before it begins: it travels with its keys (§6.1), and the publisher
  signs a new offer when it begins. A planned season so never takes the `base` that a new
  season after a leak needs.
- A chain that the offer held names takes the place of a chain of the same `base` that a reader
  got with keys; the keys of that other chain are dropped.
- An epoch belongs to the chain with the highest `base` that is not above it.
- `pay_url`, `pass_url` and `report_url` are `https`, with a host and without a user name or
  password. A client refuses other forms. They are optional: a publisher that sells by hand
  leaves them out.
- **A client checks the target** of each of the three before it opens, polls or posts to it,
  as [`http-source.md`](http-source.md) §5 checks a source: the address global unicast, for the
  first URL and for every redirect, literal or resolved, through a proxy as there. It follows a
  redirect only to `https`. A URL that fails is not used.
- A reader keeps the offer with the highest `issued` it has seen for a scope. Of two with the
  same `issued` it keeps the one whose `body` is lower, compared as a byte string.
- An issuer that a newer offer leaves out signs no more valid passes for the scope, and the
  passes it signed stop counting there. A publisher that drops a seller first lets its passes
  run out, or signs replacements.

**Where an offer travels**

| Scope | The offer is in |
|---|---|
| open live channel | `Channel.offer` of the channel file ([`live.md`](live.md) §1.2) |
| private live channel | the purchase answer (§9), and a `KeyAnswer` or first `LiveState` from a pass holder, as §6.2 and §6.3 say |
| set with an open manifest | `FileSet.offer` of each manifest |
| hidden set | the purchase answer, a `KeyAnswer` as §6.2 says, and `FileSet.offer` of the sealed manifest |

A node that holds a newer offer than the one in a channel file or a manifest uses the newer
one. A changed price does not need a new channel file.

```protobuf
// in Channel (live.md §1.2):
SignedOffer offer = 11;

// in FileSet (file-set.md §1):
SignedOffer offer = 12;
EpochKey    released = 13;    // §4.3
SignedChain released_chain = 14;  // the season of `released`
```

---

## 4. Epoch keys

Epoch `e` begins at `t0 + e × period`. The epoch of a time `t` is `(t − t0) / period`, rounded
down; there is no epoch before `t0`.

### 4.1 The chain

The publisher draws a random 32-byte **seed** for a chain, derives the key of the last epoch,
`K(base + length − 1)`, from it, and computes every earlier key from that:

```
K(base + length − 1) = H("ed3k v1 paid seed", scope ‖ base(8) ‖ seed)
K(e − 1) = H("ed3k v1 paid chain", scope ‖ K(e))              for base < e ≤ base + length − 1
anchor   = H("ed3k v1 paid anchor", scope ‖ K(base))
```

```protobuf
message EpochKey {
  uint64 epoch = 1;
  bytes  key = 2;             // 32 B
}
```

- **Whoever has `K(e)` computes the key of every earlier epoch of the chain**, and nobody can
  compute a later one.
- **Every step is bound to the scope**, the last key included, and so is every key derived
  from `K(e)` (§7.1, §8.1, §8.3). The last key is also bound to `base`. A seed used for two
  seasons (another `base`) or in two scopes gives unrelated chains. The seed itself is never
  sent. A publisher SHOULD still draw a fresh seed per season.
- **Verifying a key.** A reader applies the chain step `e − base` times to `K(e)`, then the
  anchor step, and compares with `anchor`. A reader keeps the newest key it verified of each
  season (§4.2) and hashes only from there the next time. Even a chain of 2^20 epochs costs
  well under a second to check.
- **Work per message.** A reader does at most 2^20 *(initial)* chain steps for the keys of one
  message (`KeyAnswer`, `LiveState`, `TopicState`), in the order they are sent. Keys beyond that
  are dropped unchecked, which is no fault of the sender; a requester asks for them again (§6.2).
- **A key that fails** is dropped. The reader closes the exchange it came on, and does not take
  that node as a source of keys or as a neighbour of the scope for 24 h *(initial)*. On a topic
  exchange this is a content failure of [`pubsub.md`](pubsub.md) §2.2, with the scope given
  there: no neighbour of any topic for 24 h.
- The seed produces every key of the chain. Where the publisher keeps it is said in §4.4.
- The first check of a season of 720 epochs costs 720 hashes.

### 4.2 Seasons

A chain is a **season**. A publisher starts a new one when the old one is used up (§4.4), or
at once after a leak (§4.5). Seasons are not linked by keys: a key of one season opens nothing
of another.

- **Season length.** RECOMMENDED is about 30 days *(initial)*: 720 epochs of one hour for a
  channel, 30 of one day for a set. A shorter season limits a leak more, and costs one more
  tip per season in a key answer (below).
- Each season is a `SignedChain`. The publisher signs it when it starts the season, and every
  node that sends a key of the season sends its `SignedChain` with it (§6.1).
- **The tip** of a season is the key of its last epoch that was used:

```
tip = K(min(base + length − 1, base of the next season − 1))
```

- A viewer that was subscribed at the end of a season holds its tip already. Others get it from
  pass holders: **a node gives the tips of every earlier season to a viewer pass that covers an
  epoch of a later season** (§6.2). A new subscriber so gets the back catalogue, and a viewer
  whose pass ran out keeps the tips of the seasons before its last one.
- A node keeps the tips of the seasons it has, with their `SignedChain`, as long as it follows
  the scope.
- A tip is verified as any key (§4.1), against the anchor of its own season.
- **A key belongs to one season.** A key is the key of its epoch only while no later season
  that the reader knows has a `base` at or below that epoch. A key of an earlier season at or
  above such a `base` counts only as the tip of its own season, its key of the epoch
  `base − 1` (with the `base` of the next season), which the reader computes from it. It counts for nothing else, and an answerer never gives it for its epoch.
- **Keys are compared by season, then epoch.** "Newer" for a key, for `have` (§6.1), for the
  newest key verified (§4.1) and for "a key that new" (§6.3) compares the pair (`base` of its
  season, epoch). A key of the season after a leak (§4.5) is so newer than a key of the old
  season for the same epoch.

### 4.3 Released epochs

When `free_after` is above 0, epoch `e` is **released** at the start of epoch
`e + free_after`. A released key is given to anyone.

- Every node that holds a later key of the same season, or the tip of that season, computes
  the released key itself and gives it without a pass (§6.2).
- The publisher of a set with an open manifest SHOULD write the newest released key into
  `FileSet.released` of each new version, with the `SignedChain` of its season in
  `FileSet.released_chain`. A reader verifies it like any key (§4.1).
- A released key can be put anywhere, a web page included, together with the `SignedChain` of
  its season: it is checked against that anchor.
- **A publisher can release by hand** at any time by handing out a key. This opens every epoch
  up to that one, for good.
- Released epochs of a live channel open recordings of it. The live window itself
  ([`live.md`](live.md) §3.3) is long gone by then.
- A released epoch opens only its own season. Once the released epochs pass the end of a
  season, its tip is released too. A node without a pass gets the released tips of every
  ended season in `tips` of a `KeyAnswer`, with their `SignedChain`s (§6.2), and the newest
  released key as `released`. The back catalogue that is released is so reachable without a
  pass, season by season.

### 4.4 Keys on online nodes

The nodes that broadcast, publish versions or seed are online and can be broken into. They
need keys, but not the seed of a season.

- The publisher SHOULD keep the seed of each season on a machine that is not one of them: a
  device kept offline, or a key store.
- It gives an online node only `K(e + h)`, the key of the epoch `h` epochs ahead. `h` is the
  **horizon**, RECOMMENDED at most 7 days *(initial)*. From that key the node computes every key
  it needs up to `e + h`. The offline side gives it the next one before the horizon runs out.
- It prepares the next season in the same way: the `SignedChain` and the first key of the
  horizon, handed to the online node before the season begins. A planned season begins at
  `base + length` of the one before, so its `base` is known in advance.
- **A node that is broken into** gives away the keys up to the end of its horizon. The rest of
  the season and every later season stay closed, unless the node also held the publisher key
  (below). The publisher starts a new season (§4.5) and
  gives the new keys only to nodes it has made safe again.
- A planned season's `SignedChain` reaches readers no earlier than 1 h *(initial)* before the
  epoch before its `base`: a reader refuses it before then (§3). It travels with its first
  keys, not in an offer (§3).
- The publisher key that signs offers, chains, segment records and manifests is a separate
  matter. A thief who has it can sign passes, seasons and offers of its own. With a pass of its
  own it gets every key of every season, later ones included, from any pass holder, until the
  publisher moves to a new key; there is no successor key yet (§13), and a new season (§4.5)
  does not help against it. §3 keeps its seasons and offers within 1 h and one epoch ahead, so
  they compete with the publisher's own and do not lock it out for good.

### 4.5 A new season after a leak

When a key of a scope has leaked, or an online node was broken into, the publisher starts a
new season at the next epoch.

1. It draws a new seed, makes a chain with `base` = the current epoch + 1, signs the
   `SignedChain`, and signs a new offer that names it. If the offline side cannot answer in
   time, the online node draws the seed itself, and the season after that is made offline
   again (§4.4).
2. It sends the new offer at once to its neighbours of the scope (§6.3). A node that learns a
   newer offer passes it on to its neighbours of the scope at once, as it does a key.
3. It seals everything from `base` on with the new season. Keys of the new season spread in
   the last part of the current epoch as any key does (§6.2).

- **What the leak still opens:** the epochs of its season up to the leaked one. Nothing of an
  earlier season, because seasons are not linked, and nothing from the next epoch boundary on.
- A leaked key opens nothing after its own epoch, so `period` bounds what one leak gives of
  content still to come: the rest of that epoch.
- **Old content.** Members sealed under the leaked season stay readable to whoever has the
  leaked key and the files. A publisher MAY seal them again under the new season in a new
  version; they get new roots and start with the publisher as their only source.
- **A new season does not stop a leaker who still pays.** It gets the keys of the new season
  like every viewer. Against such a leaker only the end of its pass helps: §5.3 helps find a
  shared holder key, and a seller that does not renew a pass (§9.2) ends it.

---

## 5. Pass

```protobuf
enum PassRole {
  PASS_VIEWER = 0;            // gets epoch keys and joins the mesh
  PASS_RELAY = 1;             // joins the mesh and carries data; gets no keys
}

message Pass {
  repeated bytes scopes = 1;  // 16 B each, 1 to 16
  bytes    holder = 2;        // 32 B Ed25519 key of the subscription (§5.1)
  uint64   until = 3;         // Unix time in seconds
  PassRole role = 4;
  uint64   issued = 5;        // Unix time in seconds
}

message SignedPass {
  bytes body = 1;             // a serialised Pass, verified over exactly these bytes
  bytes key = 2;              // 32 B signer: the publisher of each scope, or an issuer of its offer
  repeated Signature sigs = 3;    // over "ed3k v1 paid pass" ‖ body
}
```

- **A pass covers epoch `e` of a scope** when the scope is in `scopes`, `t0 + e × period` is
  before `until`, and the signer is the publisher of that scope or one of the `issuers` of its
  current offer. Once valid, a pass opens every earlier epoch as well, through the chain and
  the tips of earlier seasons (§4.1, §4.2).
- The cut-off therefore falls on an epoch boundary: a pass that ends inside an epoch still
  gets the key of that epoch.
- **A pass is valid from `issued` on.** A reader allows 1 h *(initial)* for clocks: a pass whose
  `issued` is at most 1 h ahead of its clock is valid. One further ahead is not valid yet; it
  is treated as no pass, and it is no fault of the sender.
- A `SignedPass` MUST NOT exceed 1 KiB.
- **One purchase, several scopes.** A pass can name up to 16 scopes, for example every channel
  and set of one creator. Each scope checks the signer against its own offer.
- **A pass cannot be taken back** before `until`. An issuer that wants a cancellation to take
  effect soon issues short passes and renews them (§9.2). RECOMMENDED is at most 7 days
  *(initial)*.
- **Relay passes** go to nodes the publisher trusts to carry data: its own seed boxes, a
  hosting service. A relay pass holder never gets a key (§6).
- **Every node of the publisher** that joins the mesh of the scope or answers key requests,
  the broadcaster and the author included, MUST hold a pass of the scope signed by the
  publisher: a viewer pass, or a relay pass for one that only carries data. A joiner drops a
  peer without a valid pass (§6.3), the publisher's own nodes included.

### 5.1 The holder key

- The viewer's client makes a new Ed25519 key for each subscription. It MUST NOT be the node
  key ([`Protocol.md`](../Protocol.md) §6.1), and SHOULD NOT be the holder key of another
  subscription.
- **The holder key keeps subscriptions apart.** A neighbour sees the node ID of the connection
  and the holder key of this scope, and so does a node that asks a pass holder for a key
  (§6.1). A node shows its pass only to a node that showed a valid pass of the scope first
  (§6.1, §6.3), so a node without a pass learns no holder key that a joiner or requester holds. It cannot tell which other scopes the node pays for, and an
  issuer cannot tell which node watches.
- A pass that names several scopes shows all of them to every node it is shown to. A client
  SHOULD tell the user so before it buys one.
- A renewal (§9.2) keeps the holder key. A client MAY make a new one at each purchase.
- The client stores the private holder key like a password. Whoever has it uses the pass.

### 5.2 Showing a pass

A node shows a pass on a connection with a proof of the holder key:

```protobuf
message PassProof {
  uint64 minute = 1;          // Unix time / 60, rounded down
  bytes  sig = 2;             // 64 B
}
```

```
sig = Ed25519 signature by holder over
      "ed3k v1 paid proof" ‖ scope(16) ‖ sender's node ID(16) ‖ receiver's node ID(16) ‖ minute(8)
```

- Both node IDs are those of the authenticated connection ([`Protocol.md`](../Protocol.md)
  §6.2). A proof is of no use on another connection, and a pass without the private holder
  key is of no use at all.
- `minute` is the sender's clock. A receiver refuses a proof more than 10 min *(initial)* from
  its own clock. The minute makes a proof evidence of a connection at a time (§5.3).
- **A receiver checks**, in this order: the signature of the pass under `key`; that `issued`
  is at most 1 h ahead of its clock (§5); that `scope` is
  in `scopes`; that `key` is the publisher or an issuer of the offer it holds; that the pass
  covers the current epoch, or for a relay pass that `until` is in the future; the proof.
  Ed25519 is verified as [`Protocol.md`](../Protocol.md) §10.7 says.
- **For a key request** the epoch check is left out: a viewer pass that covers an earlier epoch
  of the scope passes, and §6.2 bounds what it gets. Joining the mesh needs a pass that covers
  the current epoch (§6.3).
- **In a private channel** the secret proof of [`live.md`](live.md) §10.3, or of §6.1 for a
  key request, is checked before any of this (§7). A node that fails it sees no pass and shows none.
- A receiver that does not hold the offer of the scope cannot check an issuer. It accepts a
  pass signed by the publisher, and asks the sender for the offer (§6.1) for one signed by an
  issuer.
- Nor can it compute an epoch without `t0` and `period`. Until the offer arrived it checks that
  `until` is in the future in place of the epoch; once it arrived, it checks the epoch, and
  refuses a neighbour as §6.3 says when the pass does not cover the current one.
- A node re-checks the passes of its neighbours at each epoch boundary. A neighbour whose pass
  no longer covers the current epoch is refused as §6.3 says.

### 5.3 Shared holder keys

A holder key that several people use shows up as one key at more node IDs than one person
has devices. A node that sees that MAY report it to the seller, with the proofs as evidence.

- **What counts.** `devices` of the offer is the number of node IDs one holder key may use at
  the same time, 1 to 63; 0 means 3 *(initial)*. A report holds at most 64 sightings, so 63 is
  the highest number that a report can exceed. A node counts, per scope and holder key, the distinct
  sender node IDs of the valid proofs it received in the last hour *(initial)*. Above
  `devices` it MAY report.
- A node reports only what it received itself. It does not pass on, collect or ask for proofs
  that other nodes received.
- **Only a pass holder reports.** A node counts only proofs it received on a connection on
  which it showed a valid pass of the scope itself: from a neighbour (§6.3), or in a key
  exchange in which the other side saw its pass (§6.1). A node without a pass of the scope
  counts and reports nothing. The seller cannot check this rule from a report; it binds the
  reporting client.
- **The report** goes to `report_url` of the offer, as `POST` with the body
  `application/x-ed3k-report`, a serialised `ShareReport`:

```protobuf
message Sighting {
  bytes     sender = 1;       // 16 B node ID that showed the pass
  PassProof proof = 2;        // as received
}

message ShareReport {
  bytes      scope = 1;       // 16 B
  SignedPass pass = 2;        // the pass that was shown
  bytes      receiver = 3;    // 16 B node ID of the reporting node
  repeated Sighting seen = 4; // at least devices + 1, at most 64
}
```

- **The seller checks every proof** against `holder`, `scope`, `sender`, `receiver` and
  `minute`, and that the `minute`s of all sightings lie within 80 min *(initial)* of each other:
  1 h plus twice the proof tolerance of §5.2. A report that fails is ignored. A proof cannot be
  made without the private holder key, so the report cannot name a holder key that did not
  connect. It can be true and still innocent: a user with more devices
  than `devices`, or node IDs that changed. The seller decides.
- **What the seller can do** is not part of ed3k: not renew the pass (§9.2), ask the buyer, or
  issue a new pass to a new holder key. A pass held stays valid until `until`.
- A report gives the seller node IDs of the holder key, and through the purchase the person
  who paid. A client that reports SHOULD let its user turn reports off, and MUST NOT report a
  holder key that stays within `devices`.
- The seller answers `204` and nothing else. A node sends at most one report per holder key and
  scope per day *(initial)*.

---

## 6. Key exchange

### 6.1 Messages

```protobuf
message KeyRequest {          // type 40
  bytes      scope = 1;       // 16 B
  SignedPass pass = 2;        // absent: asks for the offer, the released keys and the answerer's pass only
  PassProof  proof = 3;       // §5.2; with `pass`
  uint64     have = 4;        // the newest epoch whose key I hold of the season that epoch belongs to (§4.2), plus 1; 0 = none
  bool       want_offer = 5;  // send the offer I hold
  uint64     tips_have = 6;   // base of the newest earlier season whose tip I hold, plus 1; 0 = none
  uint64     offer_issued = 7;  // `issued` of the offer I hold; 0 = none
  bytes      private_proof = 8; // 16 B; scope of a private channel only (below)
}

enum KeyRefusal {
  KEY_OK = 0;
  KEY_NO_PASS = 1;            // no pass, a relay pass, or one that fails a check or covers nothing new
  KEY_NOT_HELD = 2;           // I hold no key at or above `have` that your pass covers
  KEY_UNKNOWN = 3;            // I know no offer for this scope
  KEY_BUSY = 4;               // over the limit for this holder key (§6.2)
}

message KeyAnswer {           // type 41
  EpochKey    key = 1;        // the newest key your pass covers; absent with a refusal
  EpochKey    released = 2;   // the newest released key (§4.3); absent if none
  SignedOffer offer = 3;      // if asked, or if its `issued` is above `offer_issued`
  KeyRefusal  refusal = 4;
  repeated EpochKey    tips = 5;    // tips of earlier seasons (§4.2), oldest first; at most 256
  repeated SignedChain chains = 6;  // the season of `key`, of `released` and of each tip
  SignedPass  pass = 7;       // to a request without `pass`: my own pass, when I hold a key of the scope
  PassProof   pass_proof = 8; // §5.2; with `pass`
  bytes       private_proof = 9;  // 16 B; to a valid `private_proof` only (below)
}
```

`KeyRequest` and `KeyAnswer` are a request/answer exchange ([`Protocol.md`](../Protocol.md)
§5.2). The same `EpochKey` is pushed between neighbours on a live or topic exchange (§6.3).

**The answerer shows its pass first.** A requester sends its pass only to a node that showed a
valid viewer pass of the scope on the same connection, in a `KeyAnswer`, a `LiveState` or a
`TopicState`. Otherwise it asks in two steps:

1. A `KeyRequest` without `pass`. A node that holds a key of the scope answers it with its own
   `pass` and `pass_proof`, together with what §6.2 gives without a pass.
2. The requester checks that pass as §5.2 says for a key request. When it is valid, it sends a
   second `KeyRequest` with its own pass on the same connection; else it asks another node.

A requester that wants only the offer or released keys stops after step 1.

**A private channel.** For the scope of a private channel both sides prove the secret, as
[`live.md`](live.md) §10.3 does, with `k` of §10.1 there:

```
KeyRequest.private_proof = H("ed3k v1 paid private key",    k ‖ requester's node ID ‖ answerer's node ID)[0..16)
KeyAnswer.private_proof  = H("ed3k v1 paid private answer", k ‖ requester's node ID ‖ answerer's node ID)[0..16)
```

- Both node IDs are those of the authenticated connection ([`Protocol.md`](../Protocol.md)
  §6.2).
- For the scope of a private channel a node answers a `KeyRequest` without a valid
  `private_proof` with `KEY_UNKNOWN` and nothing else: no pass, offer, released key or tip.
- Every other answer for that scope carries `private_proof`. A requester takes nothing from an
  answer without a valid one.
- Such an answer is no fault of the answerer, since a node that does not know the secret
  answers so. The requester asks another node; it does not refuse the answerer as §4.1 does
  for a key that fails.
- A `private_proof` that is not 16 B makes the message malformed ([`Protocol.md`](../Protocol.md)
  §0).

### 6.2 Who answers

- **Any node that holds a key answers**: the publisher, and every viewer that got the key. A
  relay never holds a key. No node needs a role of its own, and the publisher need not be
  online between two epochs.
- It answers with the key of the newest epoch that the pass covers, that is not above the
  current epoch, that is at or above `have`, and that it holds or can compute, of the season
  that epoch belongs to (§4.2). In the last
  10 % of an epoch, at most 10 min *(initial)*, the next epoch counts as current, so a key can
  spread before it is needed.
- An answer is never more than the pass covers. A node that holds only later keys of the same
  season computes the covered one from them (§4.1).
- **A pass that has run out.** A viewer pass that no longer covers the current epoch is still
  answered: with the newest key it covers and the tips of the seasons before that key's, never
  with a key of the current epoch or a later one. A viewer whose pass ran out so gets what
  §10 says it keeps.
- **Tips.** To a viewer pass, a node adds the tips it holds of every season that comes before
  the season of `key` and has a `base` of at least `tips_have`. A tip that a requester lacks
  comes in a further request with `have` = 0, as the next two points say.
- A node that has more than 256 tips to give, with a pass or without one, gives the oldest
  256. A requester asks for the rest, and for tips it dropped unchecked (§4.1), in a further
  request with `tips_have` = the `base` of the newest tip it verified, plus 1, and `have` = 0.
- A requester knows that it lacks the tip of a season when the next season it holds has a
  `base` above the epoch of the newest key it holds of that season (§4.2), plus 1, and that key
  is not the last of its chain, `base + length − 1`. A season it holds with no key, followed
  by a season it holds, lacks its tip too. The newest season it holds has no tip yet. A tip
  counts as lacking only when it can be had: its pass covers an epoch of a later season
  (§4.2), or the tip is released (§4.3). It then asks with `tips_have` = the `base` of that
  season, and `have` = 0, at most once per day *(initial)* per scope.
- `chains` holds the `SignedChain` of each season that a key in the answer belongs to. A
  requester verifies each key against the anchor of its season, and drops the answer when a
  chain or a key fails.
- **Without a pass**, or with one that fails, a node answers `KEY_NO_PASS` with `released`, in
  `tips` the released tips (§4.3) of every ended season with a `base` of at least `tips_have`,
  with their `chains`, and `offer` as the next point says. This is how a client that has only a
  link learns the price and gets the released back catalogue.
- `offer` is in the answer when `want_offer` is set, and when the offer the node holds has a
  higher `issued` than `offer_issued`.
- A node answers at most 6 *(initial)* requests with a key per holder key per hour, and
  `KEY_BUSY` above that. A node also limits requests per connection as for any request.
- A node does not answer with a key over a connection whose peer it has refused as a source or
  neighbour for a bad answer.

### 6.3 Neighbours

Paid scopes keep the mesh rules of [`live.md`](live.md) §3 and [`pubsub.md`](pubsub.md) §2,
with a pass on both sides.

```protobuf
// in LiveJoin (live.md §3):
reserved 7, 8;                // the joiner's pass travels in LiveState (below)
uint64 offer_issued = 9;      // `issued` of the offer I hold, from the channel file or newer; 0 = none

// in LiveState (live.md §3):
SignedPass  pass = 10;        // the peer's in its first answer; the joiner's in its first LiveState after it (below)
PassProof   pass_proof = 11;  // §5.2; with `pass`
EpochKey    key = 12;         // pushed (below)
SignedOffer offer = 13;       // in the first answer if newer than `offer_issued`, and pushed (below)
SignedChain chain = 14;       // the season of `key`; with the first key of each season sent on this exchange

// in enum Refusal (live.md §3):
LIVE_PASS = 4;                // no valid pass for this scope and epoch

// in TopicJoin (pubsub.md §2):
reserved 8, 9;                // the joiner's pass travels in TopicState (below)
uint64 offer_issued = 10;     // `issued` of the offer I hold; 0 = none

// in TopicState (pubsub.md §2):
EpochKey   key = 4;           // pushed (below)
SignedPass pass = 5;          // the peer's in its first answer; the joiner's in its first TopicState after it (below)
PassProof  pass_proof = 6;    // §5.2; with `pass`
SignedOffer offer = 7;        // in the first answer if newer than `offer_issued`, and pushed (below)
SignedChain chain = 8;        // the season of `key`; with the first key of each season sent on this exchange

// in enum TopicRefusal (pubsub.md §2):
TOPIC_PASS = 3;
```

- **The peer shows its pass first.** A joiner sends `LiveJoin` or `TopicJoin` without a pass.
  The peer's first answer carries its own `pass` and `pass_proof`, `offer` when it is newer
  than `offer_issued`, in a private channel the `proof` of [`live.md`](live.md) §10.3, and
  nothing else of the scope: no peers, no records, no item, no `refusal` other than
  `LIVE_LEFT` or `TOPIC_LEFT`. A node without a pass of the scope that covers the current epoch
  (for a relay pass: whose `until` is in the future), and one that refuses the joiner for 24 h
  (§4.1), answers with `LIVE_LEFT` or `TOPIC_LEFT` and nothing else. A node that left a paid stream with its `end` record
  sends that record with `LIVE_LEFT` ([`live.md`](live.md) §2.2); it carries no epoch and seals
  nothing.
- The joiner checks that pass (§5.2). When it is absent or fails, the joiner closes the
  exchange. Else it sends its own `pass` and `pass_proof` in its first `LiveState` or
  `TopicState`, and nothing else.
- The peer checks the joiner's pass. Without a valid one it sends `refusal = LIVE_PASS` or
  `TOPIC_PASS`, with nothing else, and closes. With one, its next message is the first answer of
  the document the exchange belongs to, `refusal` and `peers` included, and the exchange goes
  on as there.
- A node therefore shows its pass only to a node that showed a valid one first, and the
  joiner's is the second. Both are passes of the scope that cover the current epoch, or relay
  passes whose `until` is in the future.
- **Keys are pushed.** A node that learns a newer key sends it at once, in `LiveState.key` or
  `TopicState.key`, to every neighbour whose viewer pass covers that epoch and that has not
  shown a key that new, compared by season and epoch (§4.2). A relay neighbour gets no key.
  This is how the key of a new epoch gets from the publisher to every viewer within seconds.
- A new neighbour has shown no key. Once it is admitted, a node with a key sends it the newest
  key that its viewer pass covers, as §6.2 picks it, with the `chain` of its season.
- **Offers are pushed** the same way: a node that learns a newer offer sends it at once in
  `LiveState.offer` or `TopicState.offer` to every neighbour of the scope, relays included. A
  new season after a leak (§4.5) so reaches every node within seconds.
- At an epoch boundary a node refuses a neighbour whose pass no longer covers the
  current epoch: it sends `refusal = LIVE_PASS` or `TOPIC_PASS` and closes the exchange.
- `peers` in a paid exchange MUST be peers whose pass the sender checked.

---

## 7. Live channels

A channel is paid when its channel file carries `offer`, or its link carries `paid=1`.

```
ed3k:live?key=<channel key>&id=<id>&paid=1[&s=<secret>][&dn=<url-encoded title>]
```

- **Open paid channel.** The channel file is published and found as any channel file
  ([`live.md`](live.md) §1.2). `stream` and the Kad target of the `LIVE` records are those of
  [`live.md`](live.md) §1.1 and §5. Anyone can find the addresses of the peers; only pass
  holders can join them (§6.3).
- **Private paid channel.** The link carries `s` as well. Everything of [`live.md`](live.md)
  §10.1, §10.3 and §10.4 applies: `stream`, the hourly target and the secret proof. A joiner
  shows the secret proof in `LiveJoin`, and its pass after the peer showed its own (§6.3).
  **The secret proof is checked first.** A join without a valid one is answered as
  [`live.md`](live.md) §10.3 says, and neither side shows a pass: a pass holder who does not
  know the secret learns nothing of the channel's members. Segments are sealed as below, not
  with the `k_data` of §10.2 there.

### 7.1 Sealed segments

```protobuf
// in LiveSegment (live.md §2.2):
optional uint64 epoch = 10;   // the segment is sealed with the key of this epoch
optional uint64 init_epoch = 11;  // the init data is sealed with the key of this epoch; absent = `epoch`
```

```
k_seg = H("ed3k v1 paid live", scope ‖ K(epoch))
stored bytes = nonce(12) ‖ ChaCha20(k_seg, nonce, plain bytes)          RFC 8439, counter from 0
```

- Every segment and every init data of a paid channel is sealed. A record without `epoch` in a
  paid channel is invalid, except the `end` record ([`live.md`](live.md) §2.2), which seals
  nothing and carries neither `epoch` nor `init_epoch`.
- **Init data** is sealed once, with `k_seg` of the epoch in which the broadcaster first sent
  it, and keeps its root after that. A later record that names the same `init` in another
  epoch names that first epoch in `init_epoch`. `init_epoch` MUST NOT be above `epoch`; a
  record with a higher one is invalid.
- `root` and `size` in the record are those of the stored bytes. Peers verify and pass on
  blocks exactly as in an open channel, relays included.
- `nonce` is random for every segment and every init data.
- **The broadcaster** seals each segment with the key of the epoch in which it cut it. It
  pushes the key of the next epoch to its neighbours in the last 10 % of an epoch
  (§6.2), so viewers hold it before the first segment arrives.
- A record with an `epoch` more than one above the current epoch is invalid. A node drops it
  and does not forward it. It is not a miss for the neighbour that sent it: the channel key
  signed it, as for the limits of [`live.md`](live.md) §2.2.
- A viewer that cannot open a segment because it lacks the key asks a neighbour (§6.1) and
  keeps the segment until it can, or until it leaves the window.

---

## 8. File sets

A paid set has an offer, in its manifest or, for a hidden set, also from pass holders. It has
one of two forms. Both can have open members.

| Form | Who can find the set | Who can read the list | Who can read a sealed member |
|---|---|---|---|
| **Catalogue** (scope kind 2) | everyone | everyone | pass holders |
| **Hidden** (scope kind 3) | everyone with the link | pass holders | pass holders |

**Open members** stay open in both forms: a set can seal only what was added after a date
("paid updates"), and release the rest later (§4.3).

### 8.1 Sealed members

```protobuf
// in Member (file-set.md §1):
optional uint64 epoch = 12;   // the shared file is sealed with the key of this epoch; root and size are those of the sealed file
```

```
k_data = H("ed3k v1 paid data", scope ‖ K(epoch))
sealed bytes = nonce(12) ‖ ChaCha20(k_data, nonce, plain bytes)        RFC 8439, counter from 0
```

- A member with `epoch` is sealed with that key. A member without it is an ordinary file. A
  member MUST NOT have both `epoch` and the `sealed` flag of
  [`hidden-set.md`](hidden-set.md) §4.2.
- The author seals a new member with the key of the epoch in which it publishes the version.
  A member that stays in later versions keeps its `epoch` and its root; it is not sealed again.
- **What a viewer whose pass ran out keeps:** every member of every epoch its last key covers.
  It sees the later versions of a catalogue, but cannot open their new members.
- The plain bytes of a sealed member are at most 256 GiB, the limit of the cipher for one
  nonce, so its `size` is at most 256 GiB + 12. A reader refuses a manifest that lists a larger
  one.
- The rules of [`hidden-set.md`](hidden-set.md) §4.3 apply to sealed members: a `SOURCE` record
  and no `KEYWORD` or `MEMBER` record; on the wire the name is the first 16 characters of the
  root in base32.
- **Anyone may seed a sealed member.** The bytes are useless without a key, and every source
  helps. A node without a pass that has the sealed file shares it as any file.

### 8.2 Catalogue

- The manifest is open and signed (§2.4 of [`file-set.md`](file-set.md)), with an `id`, a
  `seq` and `offer`. Its head, its link and the following of new versions are those of
  [`file-set.md`](file-set.md) §5. Every reader sees the titles, sizes and the price.
- The link is the ordinary `ed3k:set?key=…&id=…`. A client learns that the set is paid from
  the manifest.

### 8.3 Hidden

A hidden paid set is a hidden set ([`hidden-set.md`](hidden-set.md)) whose keys come from the
epoch keys, not from a secret.

```
ed3k:set?key=<set key>&paid=1[&dn=<url-encoded name>][&path=<url-encoded path>]
```

- A link with `paid=1` has neither `id` nor `s`. A link with `paid=1` and `id` or `s` is
  refused.
- `base` and `target` of the head record are those of [`hidden-set.md`](hidden-set.md) §2: they
  need the set key alone. Storing nodes check and keep the record exactly as there.
- The `Sealed` record names its epoch:

```protobuf
// in Sealed (hidden-set.md §3):
optional uint64 epoch = 3;    // data and the manifest are sealed with keys of this epoch
```

```
k_head = H("ed3k v1 paid head", scope ‖ K(epoch))        seals Sealed.data
k_data = H("ed3k v1 paid data", scope ‖ K(epoch))        seals the manifest and its inner manifests (§4.1 there)
```

- Reading: [`hidden-set.md`](hidden-set.md) §3 *Reading*, with `k_head` and `k_data` of
  `Sealed.epoch`. A reader without a key that covers `epoch` gets one (§6) first.
- A sealed manifest's members are open or sealed as in §8.1. The `sealed` flag of
  [`hidden-set.md`](hidden-set.md) §4.2 is not used in a paid set.
- **A viewer whose pass ran out** still finds the head record and its `seq`, but cannot open
  a version of a later epoch.
- The offer comes from the seller with the pass (§9) and from pass holders (§6.2).

### 8.4 The topic of a paid set

Pass holders of a set find each other and keys in a topic of [`pubsub.md`](pubsub.md).

```protobuf
// in enum TopicType (pubsub.md §1):
PAID = 3;                     // the pass holders of a paid scope

// in TopicItem (pubsub.md §2):
Signed hidden = 3;            // PAID, hidden set: the HIDDEN record (hidden-set.md §3); `key` absent
```

```
target = H("ed3k v1 paid topic", scope ‖ day(8))[0..16)       day = Unix time / 86400, rounded down, big-endian
```

- `TopicJoin.topic` is the `scope`. The `SUB` records ([`pubsub.md`](pubsub.md) §5) are stored
  and looked up under `target`, which changes every day. A subscriber stores its `SUB` record
  under the target of the current day only; in the first and last hour of a day a joiner also
  looks under the target of the day before or after.
- Order number of an item: `Sealed.seq` of the `HIDDEN` record. Of two items with the same
  `seq` the one with the lower `data`, compared as a byte string, wins, as in
  [`hidden-set.md`](hidden-set.md) §3. A catalogue has no item in this topic; its heads travel
  on its `SET_HEAD` topic.

| Check | |
|---|---|
| Signature | valid under the set key, with kind `HIDDEN` and `base` in the place of the target, as the reader of [`hidden-set.md`](hidden-set.md) §3 checks it |
| `expires` | later than now minus 1 h |
| Content | `sealed` with `epoch`, and nothing else |
| Size | `data` 12 to 256 B |
| `epoch` | at most one above the current epoch |
| Opened | once the node holds a key that covers `epoch`: `data` opens with `k_head` to a `Head` whose `seq` is the one outside and whose `root` is 32 B |

A failed check is a content failure of [`pubsub.md`](pubsub.md) §2.2, except `expires` and
`epoch`, which are time checks, and `Opened`. On a failed `epoch` the node drops the item, and
it is no fault of the neighbour. On a failed `expires` or `Opened`, which only the set key's
signer can fail, the node drops the item, does not pass it on, and keeps its order number as
one held, as for a head that ran out ([`pubsub.md`](pubsub.md) §4.1). It is no fault of the
neighbour that sent it. A node MAY keep the item it held before until a newer one opened, and
then gives that one out again. A node that holds no key for `epoch` yet passes the item on
when the other checks hold, and gets a key (§6) before it opens it. A relay passes it on
unopened.

A node that holds a newer item and has opened it goes on as
[`hidden-set.md`](hidden-set.md) §3 *Reading* does from step 3: it downloads the sealed
manifest by the `root` of the opened `Head`, opens it with `k_data` of `Sealed.epoch` (§8.3),
and accepts it only when it is signed by the set key, with the `seq` and `id` of that `Head`.
A relay does not download it. A manifest that fails that check does not change the item: the
item was verified and stays the one held and passed on; the node keeps its old version of the
set.
- A pass holder of a set SHOULD join its topic while it follows the set, within the limit of
  16 topics of [`pubsub.md`](pubsub.md) §2.3. The `PAID` topic counts as one topic toward that
  limit, for either form; the `SET_HEAD` topic of a catalogue counts on its own.
- A node without a pass that covers the current epoch, or a relay pass whose `until` is in the
  future, neither joins nor publishes a `SUB` record.
- A client that has a pass but no key looks up `target`, and sends `KeyRequest` or joins. A
  client without `PUBSUB` uses `KeyRequest` alone, to the nodes it finds there and to the
  sources of the manifest.
- **The author** joins its topic when it publishes a version, and pushes the key of that epoch
  to its neighbours.

---

## 9. Purchase

ed3k does not carry payments. This section fixes how a client hands a viewer to a seller and
gets a pass back, so that any client can buy from any seller.

### 9.1 Over HTTPS

1. The client makes a holder key (§5.1) and keeps it.
2. It opens `pay_url` in the browser, with `scope` and `holder` added as query parameters,
   both base32 as [`Protocol.md`](../Protocol.md) §13.2 fixes:
   `<pay_url>?scope=<base32>&holder=<base32>` (with `&` in place of `?` when `pay_url` has a
   query already). What the page asks for and how it takes the money is up to the seller.
3. The client polls `GET <pass_url>?scope=<base32>&holder=<base32>` (with `&` in place of `?`
   when `pass_url` has a query already) every 15 s *(initial)* for
   30 min *(initial)*, and then at each start of the client until it gets an answer other than
   404 or 7 days passed.

| Answer | Meaning |
|---|---|
| `200`, body `application/x-ed3k-pass`: a serialised `SignedPass` | the pass |
| `404` | not paid yet, or unknown |
| `410` | no pass will come for this holder key: refused, ended, or refunded |

- The client checks the pass (§5.2 without the proof) and that `holder` is its key and `scope`
  is in `scopes`. It ignores a pass that fails.
- The request carries no cookie, no account and nothing else. The holder key is all the seller
  needs: a pass is of no use without the private key.
- With a pass signed by an issuer, `pass_url` MUST also send the offer, as a serialised
  `SignedOffer` in the header `Ed3k-Offer` (base64): without it the buyer cannot check the
  issuer (§5.2). With a pass signed by the publisher it MAY. For a private channel or a hidden
  set this header, or step 1 of a key request to a pass holder (§6.1, §6.2), with
  `private_proof` for a private channel, is how a buyer
  gets its first offer.

### 9.2 Renewal

- In the last 48 h *(initial)* before `until` the client polls `pass_url` every 6 h
  *(initial)* with the same holder key. A `200` with a later `until` replaces the pass held; a
  `410` ends the polling.
- A subscription that renews by itself at the seller is therefore a row of short passes. A
  cancellation takes effect at the end of the pass held.

### 9.3 By hand

```
ed3k:pass?p=<SignedPass, base32>[&k=<private holder key, 32 B Ed25519 private key (RFC 8032), base32>]
```

- **Without `k`**, the client must hold the private key of `holder`. This is the form a seller
  sends back to a buyer who gave it a holder key by hand.
- **With `k`**, the link is a **voucher**: whoever has it holds the pass. It is the form for
  gifts and sales in a shop. A client imports the key and SHOULD make the voucher used: it
  marks the pass as imported in its own store, so that it does not take the link a second
  time, and forgets the link once the pass and the key are stored. It treats the link as a
  credential, MUST NOT write it to a log, and SHOULD tell the user that a voucher used twice is
  one pass for two people.
- An `ed3k:pass` link is refused when `p` does not parse as a `SignedPass` or, with `k`, when
  the key does not give `holder`.

---

## 10. What it does and does not give

| | |
|---|---|
| Someone without a pass | can see an open channel and an open catalogue, the price, and the released epochs. Cannot join the mesh or open a sealed segment, member or hidden manifest of an epoch not yet released. Learns no holder key of a joiner or requester, but the holder key, the node ID and a proof of every pass holder it asks for a key (§6.1) or sends a `LiveJoin` or `TopicJoin` (§6.3); in an open channel that is every peer listed under `stream`. It reports nothing (§5.3) |
| A viewer whose pass ran out | keeps everything up to the last epoch its pass covered, and the keys for it. Cannot open a later epoch |
| A new subscriber | opens every earlier epoch, the whole back catalogue included, through the tips of earlier seasons (§4.2) |
| A viewer who passes the holder key on | gives the pass to others until `until`. The rate limit of §6.2 slows key requests, and a neighbour that sees the key at more node IDs than `devices` may report it to the seller (§5.3), which can refuse to renew |
| A viewer who publishes an epoch key | opens that epoch and the earlier ones of its season, for everyone and for good. Not other seasons, and nothing after that epoch; only a leaked horizon key (§4.4) needs a new season (§4.5) |
| A node of the publisher that is broken into | gives away the keys up to its horizon, at most 7 days *(initial)*, if the seed was kept off it and it holds no publisher key (§4.4). A node that broadcasts or publishes versions also holds the publisher key: the thief can then sign a pass of its own and gets every key of every season, later ones included, from any pass holder; a new season does not stop it. It can also sign offers and seasons of its own, which compete with the publisher's at every reader. Both last until the publisher moves to a new key (§13). It cannot sign an offer or a season so far ahead that no later offer or season of the publisher can replace it (§3) |
| Taking access away | at the end of the pass, on an epoch boundary. Short passes make that soon (§9.2) |
| A neighbour, or a node that asks a pass holder for a key | learns the node ID and the holder key of this scope, and that both belong together. Not which other scopes the node pays for, unless the pass names several (§5.1). A joiner or requester shows its pass only to a node that showed a valid one first (§6.1, §6.3); a pass holder that answers shows its own to every node that asks |
| A storing node | as for an open channel, an open set or a hidden set. The daily target of §8.4 limits who sees the pass holders of a set over time |
| The seller | learns the holder key and whatever it asked for when the viewer paid. Not the node ID, unless a neighbour reports the holder key as shared (§5.3) |
| The publisher | needs to be online when it starts a new epoch with content in it: a broadcast, or a new version of a set |

---

## 11. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A key bundle per epoch**: the publisher wraps the epoch key for every subscriber and shares the bundle as a file | works without any node handing out keys | grows with the subscribers, about 5 MB for 100,000 per rotation, and the publisher has to know every subscriber's key |
| **A tree of keys** (logical key hierarchy) | a viewer can be removed at once, at O(log n) messages | state at the publisher and at every viewer, messages to all on every change, and a design of its own for losses |
| **Only delegated key nodes** hand out keys | one place to count and limit | someone the publisher pays must be online all the time; every viewer holds the key anyway |
| **Independent keys per epoch** | a pass can be limited to the epochs it paid for | no back catalogue without storing and serving every old key; a key that leaks still opens its epoch |
| **One chain for all time**, or seasons linked by keys | the newest key alone opens the whole back catalogue; no tips to hand out | one leaked key opens everything ever sealed in the scope |
| **Reports gathered in Kad** under the holder key, so any node can count its node IDs | finds a shared key that no single neighbour sees twice | a record kind for every `ROUTER`, and a public list of which node IDs pay for what |
| **A forward chain**, each key computed from the one before | one key gives every future epoch | a viewer who paid once has the rest for free |
| **The pass names the node key** | no holder key and no proof; the connection authenticates | every neighbour and seller can tie a node to all it pays for |
| **A pass checked on a chain**, a contract that holds the subscriptions | proof of payment without trusting a seller | every checking node needs access to the chain. A seller can still run on a contract and sign passes from its events |
| **A record kind for offers** in Kad | a hidden scope could be priced from its link alone | a fourteenth kind that every `ROUTER` has to store; the seller has the offer anyway |
| **A mark per viewer in the media** | a leak can be traced to a viewer | each viewer would get other bytes, so segments and members could not be shared. Outside the protocol; a player MAY add one |
| **An authenticated cipher** | the usual choice | 16 bytes and a second check of what a signed root already fixes |

---

## 12. Cost

| Item | Size or count |
|---|---|
| Offer | at most 4 KiB, once per channel file or manifest |
| Pass | about 200 B, in each join and key request |
| Proof | about 75 B per join or request |
| Key push | about 40 B per neighbour per epoch |
| Key check | one hash per epoch since the last verified key; the first check of a season of 720 epochs, 720 hashes |
| Tips | about 190 B per earlier season, key and `SignedChain`, once per subscriber; about 2.3 KB for a year of monthly seasons |
| Sealing | 12 bytes per segment or member, and a stream cipher over its bytes |

---

## 13. Open points

- A pass cannot be taken back before `until`. A list of revoked holder keys in the offer was
  not taken, because an offer then grows with the cancellations.
- A shared holder key is found only when one node sees it at too many node IDs (§5.3). A key
  shared among people who never meet the same neighbour is not found.
- Tracing who leaked an epoch key: a few segments or members sealed twice, under keys that
  differ between groups of viewers, would point to a group. Keys would then have to come from
  the publisher, not from any pass holder.
- A successor for a lost or stolen publisher key, as for every set head
  ([`file-set.md`](file-set.md) §10) and channel ([`live.md`](live.md) §12).
- A profile in which a seller proves its passes to buyers by events on a chain.
- Every number is *(initial)* and none is measured.
