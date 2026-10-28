# Extension: paid subscriptions (`subscription`)

**Status:** draft 0.1 (2026-10-16). Not implemented. Optional in wire version 1.
**Extends:** [`live.md`](live.md) §1.2, §2.2, §3 and §10, [`file-set.md`](file-set.md) §1 and §5, [`hidden-set.md`](hidden-set.md) §1 to §4, [`pubsub.md`](pubsub.md) §1 and §2, and [`Protocol.md`](../Protocol.md) §13. § = this document unless a document is named.
**Capability:** bit 18, `PAID`. Message types 40 and 41. Fields in `Channel`, `LiveSegment`, `LiveJoin`, `LiveState`, `FileSet`, `Member`, `Sealed`, `TopicJoin`, `TopicState` and `TopicItem`; `paid` in `ed3k:live` and `ed3k:set` links; `ed3k:pass` links; a report to the seller over HTTPS.
**Required of every `ROUTER`:** nothing. The extension adds no record kind.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Paid subscriptions*.

A private channel ([`live.md`](live.md) §10) and a hidden set ([`hidden-set.md`](hidden-set.md))
share one secret among all: passing the link on gives full access, which can't be revoked. Here
each viewer gets own access, for a paid time.

- Publisher, or an issuer it names, signs a **pass** for a viewer after payment. Payment method isn't ed3k: card, bank transfer, chain, site, gift.
- Viewer gets a **key per period** (**epoch**) from any node holding it that checks the pass.
  Nobody need be online for the publisher between epochs.
- Epoch keys form **reverse hash chains**, one per **season** (~a month). An epoch key opens
  every earlier epoch of its season, nothing later. New subscriber gets each earlier season's
  tip from other pass holders → back catalogue.
- **A leaked key opens ≤ one season**, up to its epoch. Publisher can start a new season at the
  next epoch (§4.5), and keeps the season seed off online nodes (§4.4).
- Publisher can **release** old epochs on a schedule: early access, free later.
- Neighbours of a paid stream and of a paid set's topic are pass holders. Publisher can give
  **relay passes**, without keys, to nodes that only carry data.

What is sealed is fixed by a signed root or signature, as in a hidden set. Block verification,
storage and forwarding unchanged.

---

## 1. Parts

| Part | What it is | Signed by | Section |
|---|---|---|---|
| **Scope** | what one pays for: live channel, set, or hidden set | (derived) | 2 |
| **Offer** | price, payment URL, issuers, current season of a scope | the publisher | 3 |
| **Epoch key** | key of one epoch; opens it and every earlier epoch of its season | (chain) | 4 |
| **Pass** | viewer's key, scopes, paid-until time | the publisher or an issuer | 5 |
| **Key exchange** | how a pass holder gets an epoch key from another node | | 6 |
| **Sealing** | what a key opens in a channel and in a set | | 7, 8 |
| **Purchase** | how a client gets a pass after payment | the issuer | 9 |

**Publisher** = key owning the scope: channel key ([`live.md`](live.md) §1.1), set author key
([`file-set.md`](file-set.md) §5.1), owner key of a multi-author set
([`multi-author-set.md`](multi-author-set.md)), or hidden-set key
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

- One scope: one offer at a time (§3), one row of epochs.
- Anyone with link or channel file computes `scope`; not secret.
- Several qualities of a channel = several channels ([`live.md`](live.md) §12) → several scopes;
  one pass can name all (§5).

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

- Publisher signs. Reader knows publisher key from link, channel file or manifest; refuses an
  offer whose signature fails or whose `scope` ≠ the one it computed.
- A `SignedOffer` MUST NOT exceed 4 KiB. Total wins: larger is refused even if every field is
  within its own limit.
- `sigs` rules of `SignedOffer`, `SignedChain`, `SignedPass`: [`Protocol.md`](../Protocol.md)
  §10.7.
- `devices` is 0 to 63; reader refuses higher (§5.3).
- `t0` and `period` never change for a scope; an offer changing them is refused, held one kept.
- **Nothing far ahead.** Reader refuses an offer with `issued` > 1 h *(initial)* ahead of its
  clock, and a chain with `base` above the epoch of its clock + 1 h *(initial)*, plus 1 (0
  before `t0`). The hour = offer clock tolerance; with short periods it keeps a reader whose clock
  lags from refusing a begun season. Offer naming such a chain: refused, held one kept; key
  arriving with such a chain: dropped. So a publisher-key thief can't sign an `issued` or `base`
  so far ahead that no later publisher offer or season replaces it.
- Chain covers epochs `base` to `base + length − 1`. A newer offer keeps the held chain or names one with higher `base`; an offer whose chain has lower `base`, or same `base` with other `length` or `anchor`, is refused.
- **No gaps.** Publisher begins a new season at most at previous `base + length`. Reader doesn't
  check (can't tell a gap from an unseen season); a gap only leaves epochs keyless. A chain whose
  `scope` isn't the offer's, or whose signature fails, invalidates the offer.
- **Offer names the current season.** `chain` = season of the epoch of `issued`, or after a leak
  the new season starting next epoch (§4.5). A planned season (§4.4) isn't named in an offer
  before it begins: travels with its keys (§6.1); publisher signs a new offer when it begins. So a
  planned season never takes the `base` a post-leak season needs.
- A chain named by the held offer replaces a same-`base` chain a reader got with keys; that
  chain's keys are dropped.
- An epoch belongs to the chain with highest `base` not above it.
- `pay_url`, `pass_url`, `report_url`: `https`, with a host, no user name or password; client
  refuses other forms. Optional: a publisher selling by hand omits them.
- **Client checks the target** of each before opening, polling or posting, as
  [`http-source.md`](http-source.md) §5 checks a source: global unicast address, for first URL and
  every redirect, literal or resolved, through a proxy as there. Follows redirects only to `https`. Failing URL unused.
- Reader keeps the offer with highest `issued` per scope; tie → lower `body`, compared as a byte
  string.
- An issuer omitted from a newer offer signs no more valid passes for the scope; its passes stop
  counting there. A publisher dropping a seller first lets its passes run out, or signs
  replacements.

**Where an offer travels**

| Scope | The offer is in |
|---|---|
| open live channel | `Channel.offer` of the channel file ([`live.md`](live.md) §1.2) |
| private live channel | purchase answer (§9), and a `KeyAnswer` or first `LiveState` from a pass holder, as §6.2 and §6.3 say |
| set with an open manifest | `FileSet.offer` of each manifest |
| hidden set | purchase answer, a `KeyAnswer` as §6.2 says, and `FileSet.offer` of the sealed manifest |

A node holding a newer offer than a channel file's or manifest's uses the newer; a price change
needs no new channel file.

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

Epoch `e` begins at `t0 + e × period`. Epoch of time `t` = `(t − t0) / period`, rounded down; no
epoch before `t0`.

### 4.1 The chain

Publisher draws random 32-byte **seed** per chain, derives the last key `K(base + length − 1)`
from it, and every earlier key from that:

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

- **Holder of `K(e)` computes every earlier key of the chain**; nobody can compute a later one.
- **Every step bound to the scope**, last key included, as is every key derived from `K(e)`
  (§7.1, §8.1, §8.3); last key also bound to `base`. Seed reused for two seasons (another `base`)
  or two scopes → unrelated chains. Seed is never sent. A publisher SHOULD still draw a fresh seed
  per season.
- **Verifying a key.** Apply chain step `e − base` times to `K(e)`, then anchor step, compare with
  `anchor`. Reader keeps newest verified key per season (§4.2), next time hashes only from there.
  Even a 2^20-epoch chain checks in well under a second.
- **Work per message.** Reader does ≤ 2^20 *(initial)* chain steps for the keys of one message (`KeyAnswer`,
  `LiveState`, `TopicState`), in sending order. Keys beyond that dropped unchecked, no fault of sender;
  requester asks again (§6.2).
- **A key that fails** is dropped; reader closes the exchange it came on and doesn't take that
  node as key source or neighbour of the scope for 24 h *(initial)*. On a topic exchange: content
  failure of [`pubsub.md`](pubsub.md) §2.2, scope as there: no neighbour of any topic for 24 h.
- Seed produces every chain key; where publisher keeps it: §4.4.
- First check of a 720-epoch season: 720 hashes.

### 4.2 Seasons

A chain is a **season**. Publisher starts a new one when the old is used up (§4.4), or at once after a leak (§4.5).
Seasons not linked by keys: one season's key opens nothing of another.

- **Season length.** RECOMMENDED ~30 days *(initial)*: 720 one-hour epochs for a channel, 30
  one-day for a set. Shorter limits a leak more, costs one more tip per season in a key answer
  (below).
- Each season is a `SignedChain`, signed when the publisher starts it; any node sending a key of
  the season sends its `SignedChain` along (§6.1).
- **Tip** = key of the season's last used epoch:

```
tip = K(min(base + length − 1, base of the next season − 1))
```

- A viewer subscribed at season end already holds its tip. Others get it from pass holders: **a
  node gives the tips of every earlier season to a viewer pass covering an epoch of a later
  season** (§6.2). So new subscribers get the back catalogue; a viewer whose pass ran out keeps
  tips of seasons before its last.
- A node keeps its seasons' tips, with their `SignedChain`, while it follows the scope.
- Tip verified like any key (§4.1), against its own season's anchor.
- **A key belongs to one season.** It is its epoch's key only while no later season known to the
  reader has `base` ≤ that epoch. A key of an earlier season at or above such a `base` counts only
  as its own season's tip, its key of epoch `base − 1` (next season's `base`), computed from it. It counts for nothing else, and an answerer never gives it for its epoch.
- **Keys compared by season, then epoch.** "Newer" for a key, for `have` (§6.1), for newest key
  verified (§4.1) and for "a key that new" (§6.3) compares the pair (`base` of its season, epoch).
  So a post-leak season key (§4.5) is newer than an old-season key of the same epoch.

### 4.3 Released epochs

With `free_after` above 0, epoch `e` is **released** at start of epoch `e + free_after`; a
released key is given to anyone.

- Any node holding a later key of the same season, or its tip, computes the released key and
  gives it without a pass (§6.2).
- Publisher of an open-manifest set SHOULD write the newest released key into `FileSet.released`
  of each new version, its season's `SignedChain` in `FileSet.released_chain`. Reader verifies it
  like any key (§4.1).
- A released key may go anywhere, a web page included, with its season's `SignedChain`; checked
  against that anchor.
- **Release by hand:** publisher hands out a key any time; opens every epoch up to it, for good.
- Released epochs of a live channel open its recordings; the live window
  ([`live.md`](live.md) §3.3) is long gone by then.
- A released epoch opens only its own season. Once released epochs pass a season's end, its tip
  is released too. A passless node gets released tips of every ended season in `tips` of a
  `KeyAnswer`, with their `SignedChain`s (§6.2), and newest released key as `released`. So the
  released back catalogue is reachable passless, season by season.

### 4.4 Keys on online nodes

Nodes that broadcast, publish versions or seed are online and can be broken into; they need keys, not a
season seed.

- Publisher SHOULD keep each season seed on a machine not among them: offline device or key
  store.
- Online node gets only `K(e + h)`, key `h` epochs ahead; `h` = **horizon**, RECOMMENDED at most
  7 days *(initial)*. Node computes every key it needs up to `e + h`. Offline side supplies the
  next before the horizon runs out.
- Next season prepared likewise: `SignedChain` and first horizon key, handed over before it
  begins. A planned season begins at previous `base + length`, so its `base` is known ahead.
- **Broken-into node** leaks keys up to its horizon's end. Rest of the season and all later
  seasons stay closed, unless it also held the publisher key (below). Publisher starts a new
  season (§4.5), gives new keys only to nodes made safe again.
- A planned season's `SignedChain` reaches readers no earlier than 1 h *(initial)* before the
  epoch before its `base`: reader refuses it before then (§3). Travels with first keys, not in an
  offer (§3).
- Publisher key (signs offers, chains, segment records, manifests) is separate. A thief with it
  can sign own passes, seasons, offers. With an own pass it gets every key of every season, later
  ones included, from any pass holder, until publisher moves to a new key; no successor key yet
  (§13); a new season (§4.5) doesn't help. §3 keeps its seasons and offers within 1 h and one
  epoch ahead, so they compete with the publisher's and don't lock it out for good.

### 4.5 A new season after a leak

When a scope key leaked or an online node was broken into, publisher starts a new season at the
next epoch.

1. Draws a new seed, makes a chain with `base` = current epoch + 1, signs the `SignedChain` and a
   new offer naming it. If the offline side can't answer in time, the online node draws the seed itself, and the season after is made offline again (§4.4).
2. Sends the new offer at once to its neighbours of the scope (§6.3). A node learning a newer
   offer forwards it at once to its neighbours of the scope, as with a key.
3. Seals everything from `base` on with the new season. New-season keys spread in the last part
   of the current epoch like any key (§6.2).

- **Leak still opens:** epochs of its season up to the leaked one. Nothing of an earlier season
  (not linked), nothing from the next epoch boundary on.
- Leaked key opens nothing after its epoch, so `period` bounds what one leak gives of future
  content: rest of that epoch.
- **Old content.** Members sealed under the leaked season stay readable to whoever has the leaked
  key and files. A publisher MAY reseal them under the new season in a new version; new roots,
  publisher as only initial source.
- **New season doesn't stop a still-paying leaker**: it gets new keys like any viewer. Only its
  pass ending helps: §5.3 helps find a shared holder key; a seller not renewing (§9.2) ends it.

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

- **Pass covers epoch `e` of a scope** when scope ∈ `scopes`, `t0 + e × period` < `until`, and
  signer = scope's publisher or one of `issuers` of its current offer. Once valid it opens every
  earlier epoch too, via chain and earlier seasons' tips (§4.1, §4.2).
- So cut-off is on an epoch boundary: a pass ending mid-epoch still gets that epoch's key.
- **Valid from `issued` on.** Reader allows 1 h *(initial)* for clocks: `issued` ≤ 1 h ahead of its clock is valid; further ahead → not valid yet, treated as no pass, no fault of sender.
- A `SignedPass` MUST NOT exceed 1 KiB.
- **One purchase, several scopes:** ≤ 16 per pass, e.g. all channels and sets of one creator.
  Each scope checks signer against its own offer.
- **No revocation** before `until`. An issuer wanting cancellation effective soon issues short
  passes and renews (§9.2); RECOMMENDED at most 7 days *(initial)*.
- **Relay passes** go to nodes the publisher trusts to carry data: own seed boxes, a hosting
  service. A relay pass holder never gets a key (§6).
- **Every publisher node** joining the scope's mesh or answering key requests, broadcaster and
  author included, MUST hold a pass of the scope signed by the publisher: viewer pass, or relay
  pass if it only carries data. A joiner drops a peer without valid pass (§6.3), publisher's own
  nodes included.

### 5.1 The holder key

- Viewer's client makes a new Ed25519 key per subscription. It MUST NOT be the node key
  ([`Protocol.md`](../Protocol.md) §6.1), and SHOULD NOT be another subscription's holder key.
- **Holder key separates subscriptions.** A neighbour sees the connection node ID and this
  scope's holder key, as does a node asking a pass holder for a key (§6.1). A node shows its pass
  only to a node that showed a valid pass of the scope first (§6.1, §6.3), so a passless node
  learns no joiner's or requester's holder key. It can't tell which other scopes the node pays for, and an
  issuer can't tell which node watches.
- A multi-scope pass shows all its scopes to every node it's shown to; a client SHOULD tell the
  user before it first shows one. The offer names one scope, so a client can't know before the
  pass arrives; a user declining keeps the pass unused.
- Renewal (§9.2) keeps the holder key. A client MAY make a new one per purchase.
- A node MAY hold a pass under its anonymous key ([`hops.md`](hops.md) §1); holder key then shown
  under that key only, never the node key. Buying and polling `pass_url` (§9.1) aren't ed3k and
  show an address to the seller, who can tie it to the holder key; a client MUST tell its user so
  before buying for the anonymous key, once per holder key: §9.1 step 3 and §9.2 polls for that
  key need no new warning. Offering it is local policy. The scope's `SUB` or `LIVE` record is
  then an `ANON` record at its target ([`pubsub.md`](pubsub.md) §5, [`live.md`](live.md) §5.1).
- Client stores the private holder key like a password; whoever has it uses the pass.

### 5.2 Showing a pass

Shown on a connection with a holder-key proof:

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

- Both node IDs from the authenticated connection ([`Protocol.md`](../Protocol.md) §6.2). Proof
  useless on another connection; pass useless without the private holder key.
- `minute` = sender's clock. Receiver refuses a proof > 10 min *(initial)* off its clock. The
  minute makes a proof evidence of a connection at a time (§5.3).
- **Receiver checks**, in order: pass signature under `key`; `issued` ≤ 1 h ahead of its clock
  (§5); `scope` ∈ `scopes`; `key` = publisher or issuer of its held offer; pass covers the current
  epoch (relay pass: `until` in the future); the proof. Ed25519 verified as
  [`Protocol.md`](../Protocol.md) §10.7 says.
- **For a key request** no epoch check: a viewer pass covering an earlier epoch of the scope
  passes; §6.2 bounds what it gets. Joining the mesh needs a pass covering the current epoch
  (§6.3).
- **In a private channel** the secret proof of [`live.md`](live.md) §10.3, or of §6.1 for a key
  request, is checked before all this (§7). A node failing it sees no pass, shows none.
- Without the scope's offer a receiver can't check an issuer: accepts a publisher-signed pass;
  for an issuer-signed one asks the sender for the offer (§6.1).
- Nor can it compute an epoch without `t0` and `period`: until the offer arrives it checks
  `until` is in the future instead; after, it checks the epoch and refuses a neighbour per §6.3
  if the pass doesn't cover the current one.
- Neighbours' passes re-checked at each epoch boundary; one no longer covering the current epoch
  is refused per §6.3.

### 5.3 Shared holder keys

A holder key shared by several people shows as one key at more node IDs than one person has
devices. A node seeing that MAY report it to the seller, proofs as evidence.

- **What counts.** Offer `devices` = number of node IDs one holder key may use at once, 1 to 63; 0 means 3
  *(initial)*. A report holds ≤ 64 sightings, so 63 is the highest number a report can exceed.
  Node counts, per scope and holder key, distinct sender node IDs of valid viewer-pass proofs
  received in the last hour *(initial)*; relay passes get no key, not counted. Above `devices` it
  MAY report.
- A node reports only what it received itself; doesn't forward, collect or request others'
  proofs.
- **Only a pass holder reports.** Counts only proofs received on a connection where it showed a
  valid pass of the scope itself: from a neighbour (§6.3), or in a key exchange where the other
  side saw its pass (§6.1). Node without a scope pass counts and reports nothing. Seller can't check this from
  a report; it binds the reporting client.
- **Report** goes to offer's `report_url` as `POST`, body `application/x-ed3k-report`, a
  serialised `ShareReport`:

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

- **Seller checks every proof** against `holder`, `scope`, `sender`, `receiver`, `minute`, and
  that all sightings' `minute`s lie within 80 min *(initial)* of each other: 1 h plus twice the
  proof tolerance of §5.2. Failing report ignored. A proof needs the private holder key, so a
  report can't name a holder key that didn't connect. It can be true yet innocent: a user with more
  devices than `devices`, or changed node IDs. Seller decides.
- **Seller's options** aren't ed3k: not renew (§9.2), ask the buyer, or issue a new pass to a new
  holder key. A held pass stays valid until `until`.
- A report gives the seller node IDs of the holder key, and via the purchase the payer. A
  reporting client SHOULD let its user turn reports off, and MUST NOT report a holder key staying
  within `devices`.
- Seller answers `204` only. Node sends ≤ one report per holder key and scope per day *(initial)*. A node
  under its anonymous key ([`hops.md`](hops.md) §1) MUST NOT report, except through a proxy the
  user set up for that: the `POST` shows its address with its anonymous node ID.

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

`KeyRequest`/`KeyAnswer` are a request/answer exchange ([`Protocol.md`](../Protocol.md) §5.2).
The same `EpochKey` is pushed between neighbours on a live or topic exchange (§6.3).

**Answerer shows its pass first.** Requester sends its pass only to a node that showed a valid
viewer pass of the scope on the same connection (in a `KeyAnswer`, `LiveState` or `TopicState`).
Else two steps:

1. `KeyRequest` w/o `pass`. A node holding a scope key answers with own `pass` and `pass_proof`,
   plus what §6.2 gives without a pass.
2. Requester checks that pass per §5.2 for a key request. Valid → second `KeyRequest` with own pass on the same connection; else ask another node.

Wanting only offer or released keys → stop after step 1.

**A private channel.** For a private channel's scope both sides prove the secret as
[`live.md`](live.md) §10.3 does, with `k` of §10.1 there:

```
KeyRequest.private_proof = H("ed3k v1 paid private key",    k ‖ requester's node ID ‖ answerer's node ID)[0..16)
KeyAnswer.private_proof  = H("ed3k v1 paid private answer", k ‖ requester's node ID ‖ answerer's node ID)[0..16)
```

- Both node IDs from the authenticated connection ([`Protocol.md`](../Protocol.md) §6.2).
- For a private channel's scope a `KeyRequest` without valid `private_proof` gets `KEY_UNKNOWN` and nothing
  else: no pass, offer, released key or tip.
- Every other answer for that scope carries `private_proof`; requester takes nothing from an
  answer without a valid one.
- Such an answer is no fault of the answerer (a node not knowing the secret answers so).
  Requester asks another node; doesn't refuse the answerer as §4.1 does for a failing key.
- `private_proof` not 16 B → message malformed ([`Protocol.md`](../Protocol.md) §0).

### 6.2 Who answers

- **Any key holder answers**: publisher and every viewer that got the key. Relays never hold
  keys. No special role needed; publisher need not be online between epochs.
- Answer = key of the newest epoch that the pass covers, not above the current epoch, ≥ `have`,
  and held or computable, of that epoch's season (§4.2). In an epoch's last 10 %, at most 10 min
  *(initial)*, the next epoch counts as current, so keys spread before needed.
- Never more than the pass covers. A node holding only later keys of the season computes the
  covered one (§4.1).
- **Expired pass.** A viewer pass no longer covering the current epoch is still answered: newest
  key it covers plus tips of seasons before that key's, never a key of the current epoch or later.
  So it gets what §10 says it keeps.
- **Tips.** To a viewer pass, node adds tips it holds of every season before the season of `key`
  with `base` ≥ `tips_have`. A tip the requester lacks comes in a further request with `have` =
  0, as the next two points say.
- More than 256 tips to give (with or without pass) → oldest 256. Requester asks for the rest, and for
  tips it dropped unchecked (§4.1), in a further request with `tips_have` = `base` of newest
  verified tip + 1, `have` = 0.
- Requester lacks a season's tip when the next held season has `base` above the epoch of its
  newest held key of that season (§4.2), plus 1, and that key isn't its chain's last,
  `base + length − 1`. A held season with no key, followed by a held season, lacks its tip too.
  Newest held season has no tip yet. A tip counts as lacking only if obtainable: pass covers an
  epoch of a later season not above the current epoch (counting the last-10 % lead above; seasons: §4.2), or the
  tip is released (§4.3). Then asks with `tips_have` = that season's `base`, `have` = 0, at most
  once per day *(initial)* per scope.
- `chains` holds the `SignedChain` of each season a key in the answer belongs to. Requester
  verifies each key against its season's anchor; drops the answer if a chain or key fails.
- **No pass** or failing one → `KEY_NO_PASS` with `released`, in `tips` the released tips (§4.3)
  of every ended season with `base` ≥ `tips_have`, their `chains`, and `offer` per next point. So
  a client with only a link learns the price and gets the released back catalogue.
- `offer` included when `want_offer` set, and when held offer's `issued` > `offer_issued`.
- Node answers ≤ 6 *(initial)* requests with a key per holder key per hour, `KEY_BUSY` above; also limits requests per connection as for any request.
- No key answer over a connection whose peer it refused as source or neighbour for a bad answer.

### 6.3 Neighbours

Paid scopes keep mesh rules of [`live.md`](live.md) §3 and [`pubsub.md`](pubsub.md) §2, with a
pass on both sides.

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

- **Peer shows its pass first.** Joiner sends `LiveJoin`/`TopicJoin` w/o pass. Peer's first
  answer carries own `pass` and `pass_proof`, `offer` if newer than `offer_issued`, in a private
  channel the `proof` of [`live.md`](live.md) §10.3, and nothing else of the scope: no peers,
  records, item, or `refusal` other than `LIVE_LEFT` or `TOPIC_LEFT`. A node without a scope pass
  covering the current epoch (relay pass: `until` in the future), or refusing the joiner for 24 h
  (§4.1), answers `LIVE_LEFT` or `TOPIC_LEFT` and nothing else. A node that left a paid stream with its `end` record
  sends that record with `LIVE_LEFT` ([`live.md`](live.md) §2.2); it carries no epoch and seals
  nothing.
- Joiner checks that pass (§5.2); absent/failing → closes exchange. Else sends own `pass` and
  `pass_proof` in its first `LiveState` or `TopicState`, nothing else.
- Peer checks joiner's pass. Invalid → `refusal = LIVE_PASS` or `TOPIC_PASS`, nothing else, close. Valid → next message is the first answer of the exchange's own document, `refusal` and
  `peers` included; exchange continues as there.
- So a node shows its pass only to one that showed a valid one first; joiner's is second. Both
  are scope passes covering the current epoch, or relay passes with `until` in the future.
- **Keys pushed.** A node learning a newer key sends it at once in `LiveState.key` or
  `TopicState.key` to every neighbour whose viewer pass covers that epoch and that hasn't shown a
  key that new, by season and epoch (§4.2). Relay neighbours get none. So a new epoch's key gets from publisher to all viewers within seconds.
- New neighbour has shown no key; once admitted, a key-holding node sends it the newest key its
  viewer pass covers, picked per §6.2, with its season's `chain`.
- **Offers pushed** likewise: a newer offer goes at once in `LiveState.offer` or
  `TopicState.offer` to every neighbour of the scope, relays included. So a post-leak season
  (§4.5) reaches every node within seconds.
- At an epoch boundary, a neighbour whose pass no longer covers the current epoch is refused:
  `refusal = LIVE_PASS` or `TOPIC_PASS`, exchange closed.
- `peers` in a paid exchange MUST be peers whose pass the sender checked.

---

## 7. Live channels

A channel is paid when its channel file carries `offer`, or its link carries `paid=1`.

```
ed3k:live?key=<channel key>&id=<id>&paid=1[&s=<secret>][&dn=<url-encoded title>]
```

- **Open paid channel.** Channel file published and found as any other ([`live.md`](live.md) §1.2);
  `stream` and Kad target of `LIVE` records per [`live.md`](live.md) §1.1 and §5. Anyone can find
  peer addresses; only pass holders can join (§6.3).
- **Private paid channel.** Link also carries `s`. All of [`live.md`](live.md) §10.1, §10.3,
  §10.4 applies: `stream`, hourly target, secret proof. Joiner shows the secret proof in
  `LiveJoin`, its pass after the peer showed its own (§6.3). **Secret proof checked first.** A join
  without a valid one is answered per [`live.md`](live.md) §10.3; neither side shows a pass: a pass
  holder not knowing the secret learns nothing of members. Segments sealed as below, not with
  `k_data` of §10.2 there.

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

- Every segment and init data of a paid channel is sealed. A paid-channel record without `epoch`
  is invalid, except the `end` record ([`live.md`](live.md) §2.2): seals nothing, carries neither
  `epoch` nor `init_epoch`.
- **Init data** sealed once, with `k_seg` of the epoch the broadcaster first sent it in; keeps its
  root after. A later record naming the same `init` in another epoch names that first epoch in
  `init_epoch`. `init_epoch` MUST NOT be above `epoch`; a record with higher is invalid.
- Record `root` and `size` are of the stored bytes. Peers verify and forward blocks exactly as in
  an open channel, relays included.
- `nonce` random per segment and init data.
- **Broadcaster** seals each segment with the key of the epoch it cut it in; pushes the next
  epoch's key to neighbours in an epoch's last 10 % (§6.2), so viewers have it before the first
  segment.
- Record with `epoch` > current epoch + 1 is invalid: dropped, not forwarded. Not a miss for the
  sending neighbour: the channel key signed it, as for the limits of [`live.md`](live.md) §2.2.
- Viewer lacking a segment's key asks a neighbour (§6.1), keeps the segment until it can open it
  or it leaves the window.

---

## 8. File sets

A paid set has an offer, in its manifest or, for a hidden set, also from pass holders. Two forms,
both may have open members.

| Form | Who can find the set | Who can read the list | Who can read a sealed member |
|---|---|---|---|
| **Catalogue** (scope kind 2) | everyone | everyone | pass holders |
| **Hidden** (scope kind 3) | everyone with the link | pass holders | pass holders |

**Open members** stay open in both forms: a set may seal only what was added after a date ("paid
updates") and release the rest later (§4.3).

### 8.1 Sealed members

```protobuf
// in Member (file-set.md §1):
optional uint64 epoch = 12;   // the shared file is sealed with the key of this epoch; root and size are those of the sealed file
```

```
k_data = H("ed3k v1 paid data", scope ‖ K(epoch))
sealed bytes = nonce(12) ‖ ChaCha20(k_data, nonce, plain bytes)        RFC 8439, counter from 0
```

- Member with `epoch`: sealed with that key; without: ordinary file. A member MUST NOT have both
  `epoch` and the `sealed` flag of [`hidden-set.md`](hidden-set.md) §4.2.
- Author seals a new member with the key of the epoch it publishes the version in. A member kept
  in later versions keeps `epoch` and root; not resealed.
- **Expired viewer keeps:** every member of every epoch its last key covers. Sees later catalogue
  versions, can't open their new members.
- Sealed member plain bytes ≤ 256 GiB (cipher limit per nonce), so `size` ≤ 256 GiB + 12. Reader
  refuses a manifest listing a larger one.
- [`hidden-set.md`](hidden-set.md) §4.3 rules apply: a `SOURCE` record, no `KEYWORD` or `MEMBER`
  record; wire name = first 16 characters of the root in base32.
- **Anyone may seed a sealed member**: bytes useless without key; every source helps. A passless
  node holding the sealed file shares it as any file.

### 8.2 Catalogue

- Manifest open and signed (§2.4 of [`file-set.md`](file-set.md)), with `id`, `seq`, `offer`.
  Head, link, following of new versions per [`file-set.md`](file-set.md) §5. Every reader sees
  titles, sizes, price.
- Link is the ordinary `ed3k:set?key=…&id=…`; client learns it's paid from the manifest.

### 8.3 Hidden

A hidden paid set = hidden set ([`hidden-set.md`](hidden-set.md)) with keys from epoch keys, not
a secret.

```
ed3k:set?key=<set key>&paid=1[&dn=<url-encoded name>][&path=<url-encoded path>]
```

- `paid=1` link has neither `id` nor `s`; `paid=1` with `id` or `s` → refused.
- Head record `base` and `target` per [`hidden-set.md`](hidden-set.md) §2: they need the set key alone. Storing
  nodes check and keep the record exactly as there.
- `Sealed` names its epoch:

```protobuf
// in Sealed (hidden-set.md §3):
optional uint64 epoch = 3;    // data and the manifest are sealed with keys of this epoch
```

```
k_head = H("ed3k v1 paid head", scope ‖ K(epoch))        seals Sealed.data
k_data = H("ed3k v1 paid data", scope ‖ K(epoch))        seals the manifest and its inner manifests (§4.1 there)
```

- Reading: [`hidden-set.md`](hidden-set.md) §3 *Reading*, with `k_head`, `k_data` of
  `Sealed.epoch`. Reader lacking a key covering `epoch` gets one first (§6).
- Sealed manifest's members open or sealed per §8.1. The `sealed` flag of
  [`hidden-set.md`](hidden-set.md) §4.2 isn't used in a paid set.
- **Expired viewer** still finds head record and `seq`, can't open a later-epoch version.
- Offer comes from the seller with the pass (§9) and from pass holders (§6.2).

### 8.4 The topic of a paid set

Pass holders of a set find each other and keys in a [`pubsub.md`](pubsub.md) topic.

```protobuf
// in enum TopicType (pubsub.md §1):
PAID = 3;                     // the pass holders of a paid scope

// in TopicItem (pubsub.md §2):
Signed hidden = 3;            // PAID, hidden set: the HIDDEN record (hidden-set.md §3); `key` absent
```

```
target = H("ed3k v1 paid topic", scope ‖ day(8))[0..16)       day = Unix time / 86400, rounded down, big-endian
```

- `TopicJoin.topic` = `scope`. `SUB` records ([`pubsub.md`](pubsub.md) §5) stored and looked up
  under `target`, changing daily. Subscriber stores its `SUB` record under the current day's
  target only; in a day's first and last hour a joiner also looks under the previous or next
  day's target.
- Item order number: `Sealed.seq` of the `HIDDEN` record. Same `seq` → lower `data`, compared as a
  byte string, wins, as in [`hidden-set.md`](hidden-set.md) §3. Same `seq` and `data` with later
  `expires`: not newer, not forwarded; node replaces its held bytes with it, as [`pubsub.md`](pubsub.md) §4.1 says for a head. A catalogue has no item in this topic; its heads travel
  on its `SET_HEAD` topic.

| Check | |
|---|---|
| Signature | valid under the set key, with kind `HIDDEN` and `base` in the place of the target, as the reader of [`hidden-set.md`](hidden-set.md) §3 checks it |
| `expires` | later than now minus 1 h |
| Content | `sealed` with `epoch`, and nothing else |
| Size | `data` 12 to 256 B |
| `epoch` | at most one above the current epoch |
| Opened | once the node holds a key that covers `epoch`: `data` opens with `k_head` to a `Head` whose `seq` is the one outside and whose `root` is 32 B |

Failed check = content failure of [`pubsub.md`](pubsub.md) §2.2, except `expires`, `epoch` (time
checks) and `Opened`. Failed `epoch`: drop item, no fault of neighbour. Failed `expires` or
`Opened` (only the set key's signer can fail these): drop item, don't forward, keep its order
number as one held, as for an expired head ([`pubsub.md`](pubsub.md) §4.1); no fault of the
sending neighbour. A node MAY keep its previously held item until a newer one opened, and then
gives that one out again. Node without a key for `epoch` yet forwards the item when other checks
hold, and gets a key (§6) before opening. A relay forwards it unopened.

A node holding a newer, opened item continues as [`hidden-set.md`](hidden-set.md) §3 *Reading*
from step 3: downloads sealed manifest by opened `Head`'s `root`, opens with `k_data` of
`Sealed.epoch` (§8.3), accepts only if signed by the set key with that `Head`'s `seq` and `id`.
Relays don't download. A manifest failing that check doesn't change the item: it was verified,
stays held and forwarded; node keeps its old set version.
- A pass holder SHOULD join the set's topic while following it, within the 16-topic limit of
  [`pubsub.md`](pubsub.md) §2.3. The `PAID` topic counts as one, either form; a catalogue's
  `SET_HEAD` topic counts separately.
- A node without a pass covering the current epoch, or relay pass with `until` in the future,
  neither joins nor publishes a `SUB` record.
- Client with pass but no key looks up `target`, sends `KeyRequest` or joins. Client without
  `PUBSUB` looks up `SUB` records too, stores none, uses `KeyRequest` alone, to nodes found there
  and to sources of a manifest it can name: a catalogue's, or a hidden set's version opened
  before.
- **Author** joins its topic on publishing a version and pushes that epoch's key to neighbours.

---

## 9. Purchase

ed3k carries no payments. Fixed here: how a client hands a viewer to a seller and gets a pass
back, so any client can buy from any seller.

### 9.1 Over HTTPS

1. Client makes a holder key (§5.1), keeps it.
2. Opens `pay_url` in the browser with `scope` and `holder` query parameters, both base32 per
   [`Protocol.md`](../Protocol.md) §13.2: `<pay_url>?scope=<base32>&holder=<base32>` (`&` instead of
   `?` if `pay_url` already has a query). Page content and money handling: up to the seller.
3. Polls `GET <pass_url>?scope=<base32>&holder=<base32>` (`&` instead of `?` if `pass_url` already
   has a query) every 15 s *(initial)* for 30 min *(initial)*, then at each client start until an answer
   other than 404, or 7 days *(initial)* passed since opening `pay_url`.

| Answer | Meaning |
|---|---|
| `200`, body `application/x-ed3k-pass`: a serialised `SignedPass` | the pass |
| `404` | not paid yet, or unknown |
| `410` | no pass will come for this holder key: refused, ended, or refunded |

- Client checks pass (§5.2 without proof), `holder` = its key, `scope` ∈ `scopes`; ignores a
  failing pass.
- Request carries no cookie, account or anything else; holder key is all the seller needs (pass
  useless without the private key).
- With an issuer-signed pass, `pass_url` MUST also send the offer, serialised `SignedOffer` in
  header `Ed3k-Offer` (base64): else buyer can't check the issuer (§5.2). With a publisher-signed
  pass it MAY. For a private channel or hidden set, this header, or step 1 of a key request to a
  pass holder (§6.1, §6.2), with `private_proof` for a private channel, is how a buyer gets its
  first offer.

### 9.2 Renewal

- Last 48 h *(initial)* before `until`: client polls `pass_url` every 6 h *(initial)*, same holder
  key. `200` with later `until` replaces held pass; `410` ends polling.
- So seller-side auto-renewal = row of short passes; cancellation takes effect at the held pass's
  end.

### 9.3 By hand

```
ed3k:pass?p=<SignedPass, base32>[&k=<private holder key, 32 B Ed25519 private key (RFC 8032), base32>]
```

- **Without `k`**: client must hold `holder`'s private key. Form a seller returns to a buyer who
  gave it a holder key by hand.
- **With `k`**: link is a **voucher**; whoever has it holds the pass. For gifts and shop sales.
  Client imports the key and SHOULD make the voucher used: marks the pass imported in its own
  store so it won't take the link twice, forgets the link once pass and key are stored. Treats the
  link as a credential, MUST NOT write it to a log, and SHOULD tell the user a voucher used twice
  is one pass for two people.
- `ed3k:pass` link refused when `p` doesn't parse as a `SignedPass` or, with `k`, the key doesn't
  give `holder`.

---

## 10. What it does and does not give

| | |
|---|---|
| Someone without a pass | sees open channel and catalogue, price, released epochs. Can't join the mesh or open a sealed segment, member or hidden manifest of an unreleased epoch. Learns no joiner's or requester's holder key, but holder key, node ID and a proof of every pass holder it asks for a key (§6.1) or sends a `LiveJoin` or `TopicJoin` (§6.3); in an open channel = every peer listed under `stream`. Reports nothing (§5.3) |
| A viewer whose pass ran out | keeps everything up to the last epoch its pass covered, and its keys. Can't open a later epoch |
| A new subscriber | opens every earlier epoch, whole back catalogue included, via earlier seasons' tips (§4.2) |
| A viewer who passes the holder key on | gives the pass away until `until`. §6.2 rate limit slows key requests; a neighbour seeing the key at more node IDs than `devices` may report to the seller (§5.3), which can refuse renewal |
| A viewer who publishes an epoch key | opens that epoch and earlier ones of its season, for all, for good. Not other seasons, nothing after; only a leaked horizon key (§4.4) needs a new season (§4.5) |
| A node of the publisher that is broken into | leaks keys up to its horizon, ≤ 7 days *(initial)*, if the seed was kept off it and it holds no publisher key (§4.4). A broadcasting or version-publishing node also holds the publisher key: thief can sign an own pass, get every key of every season, later ones included, from any pass holder; new season doesn't stop it. Can also sign own offers and seasons, competing with the publisher's at every reader. Both last until publisher moves to a new key (§13). Can't sign an offer or season so far ahead that no later publisher one replaces it (§3) |
| Taking access away | at pass end, on an epoch boundary; short passes make that soon (§9.2) |
| A neighbour, or a node that asks a pass holder for a key | learns node ID and this scope's holder key, and that they belong together. Not other scopes paid for, unless the pass names several (§5.1). Joiner or requester shows its pass only to a node that showed a valid one first (§6.1, §6.3); an answering pass holder shows its own to every asker |
| A storing node | as for an open channel, open set or hidden set. §8.4 daily target limits who sees a set's pass holders over time |
| The seller | learns holder key and whatever it asked at payment. Not node ID, unless a neighbour reports the holder key as shared (§5.3) |
| The publisher | must be online to start a new epoch with content: a broadcast, or a new set version |

---

## 11. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Key bundle per epoch**: publisher wraps the epoch key for every subscriber, shares bundle as a file | no node needs to hand out keys | grows with subscribers, ~5 MB for 100,000 per rotation; publisher must know every subscriber's key |
| **Key tree** (logical key hierarchy) | remove a viewer at once, O(log n) messages | state at publisher and every viewer, messages to all on each change, own design for losses
 |
| **Only delegated key nodes** hand out keys | one place to count and limit | someone the publisher pays must be always online; every viewer holds the key anyway |
| **Independent keys per epoch** | pass limitable to paid epochs | no back catalogue without storing and serving every old key; a leaked key still opens its epoch |
| **One chain for all time**, or key-linked seasons | newest key alone opens whole back catalogue; no tips | one leaked key opens everything ever sealed in the scope |
| **Reports gathered in Kad** under holder key, so any node can count its node IDs | finds a shared key no single neighbour sees twice | a record kind for every `ROUTER`, and a public list of which node IDs pay for what |
| **Forward chain**, each key from the previous | one key gives every future epoch | pay once, get the rest free |
| **Pass names the node key** | no holder key or proof; connection authenticates | every neighbour and seller can tie a node to all it pays for |
| **Pass checked on a chain**, a contract holding subscriptions | payment proof without trusting a seller | every checking node needs chain access. A seller can still run on a contract and sign passes from its events |
| **A record kind for offers** in Kad | hidden scope priceable from its link alone | a fourteenth kind every `ROUTER` must store; seller has the offer anyway |
| **Per-viewer media mark** | leak traceable to a viewer | each viewer gets other bytes, so segments and members can't be shared. Outside the protocol; a player MAY add one |
| **An authenticated cipher** | the usual choice | 16 bytes and a second check of what a signed root already fixes |

---

## 12. Cost

| Item | Size or count |
|---|---|
| Offer | ≤ 4 KiB, once per channel file or manifest |
| Pass | ~200 B, per join and key request |
| Proof | ~75 B per join or request |
| Key push | ~40 B per neighbour per epoch |
| Key check | one hash per epoch since last verified key; first check of a 720-epoch season, 720 hashes |
| Tips | ~190 B per earlier season (key and `SignedChain`),
 once per subscriber; ~2.3 KB for a year of monthly seasons |
| Sealing | 12 bytes per segment or member, plus a stream cipher over its bytes |

---

## 13. Open points

- No revocation before `until`. A revoked-holder-key list in the offer was rejected: offer would
  grow with cancellations.
- A shared holder key is found only when one node sees it at too many node IDs (§5.3); sharers who
  never meet the same neighbour go unfound.
- Tracing who leaked an epoch key: a few segments or members sealed twice, under keys differing
  between viewer groups, would point to a group. Keys would then have to come from the publisher,
  not any pass holder.
- Successor for a lost or stolen publisher key, as for every set head
  ([`file-set.md`](file-set.md) §10) and channel ([`live.md`](live.md) §12).
- A profile where a seller proves its passes to buyers by chain events.
- Every number is *(initial)*; none measured.
