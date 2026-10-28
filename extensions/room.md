# Extension: rooms (`room`)

**Status:** draft 0.1 (2026-10-27). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3 and §13, [`chat.md`](chat.md), [`pubsub.md`](pubsub.md) §2 and §5, [`live.md`](live.md) §10. § = this document unless a document is named.
**Capability:** bit 19, `ROOM`. Message types 42 and 43. `ed3k:room` links.
**Required of every `ROUTER`:** nothing. Rooms use record kind `SUB` of [`pubsub.md`](pubsub.md) §5, which every `ROUTER` already stores.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Rooms*.

Chat ([`chat.md`](chat.md)) carries a message from one node to one other over a connection
between them. A **room** carries posts among many members.

- Members form a mesh, like topic subscribers ([`pubsub.md`](pubsub.md) §2); each keeps a few
  neighbours and passes on what it verified.
- Every post is **signed by its author**; a post relayed via others is still the author's and
  nobody on the way can change it.
- Each member signs with its own **member key** for that room, never its node key; its posts
  aren't tied to what its node shares, nor to its other rooms.
- Members keep posts of the **last day** and give them to a member joining or coming back. Only
  room members keep them: no Kad node and no stranger stores a post.
- Owner names **moderators**: they admit and ban members, and in a closed room hand out keys.
- A room is **public**, **private** or **closed**:

| Form | Link holds | Who joins | Posts are | Removing a member | Section |
|---|---|---|---|---|---|
| **Public** | room key, `id` | anyone with link | signed, inside transport only | a ban; member's posts dropped | 1.3 |
| **Private** | and a secret | anyone with link | sealed with a key from secret | a ban, as in public; it keeps secret | 3.1 |
| **Closed** | and a secret | members a moderator admitted | sealed with a changing key | a ban and a new key; member reads no later post | 3.2 |

Mesh and member search: [`pubsub.md`](pubsub.md). Secret, its proof and hourly moving target:
private channels ([`live.md`](live.md) §10). Showing admission follows the passes of
[`subscription.md`](subscription.md) §5.2 and §6.3.

---

## 1. The room

### 1.1 Keys

| Key | Who holds it | Signs |
|---|---|---|
| **room key** | owner | room info (§2.1); also admits, bans, keys (§2.2, §2.3, §3.2) |
| **member key** | each member, one per room | its presence and posts (§2.4, §2.5) |
| **moderator** | member whose member key room info lists | admits, bans, keys |

- All three Ed25519; signature rules of [`Protocol.md`](../Protocol.md) §10.7.
- **Room key MUST NOT be a node key** ([`Protocol.md`](../Protocol.md) §6.1). It SHOULD be
  a key of its own, like a channel key ([`live.md`](live.md) §1.1): room then outlasts node
  and doesn't name it.
- **A member key MUST NOT be a node key, nor a room key.** Client makes a new one per room joined
  and SHOULD NOT use one key in two rooms, per [`subscription.md`](subscription.md) §5.1:
  neighbours of one room would learn the others.
- Owner posts like any member, with its own member key; it may list that key as moderator. In a
  closed or `moderated` room a key not in `mods` needs an admit (§2.2), which owner signs with
  room key: when making such a room, and when a room info turns room closed or
  `moderated`, together with that room info.
- **Member ID** = `H("ed3k v1 node id", member key)[0..16)`, computed like a node ID. Names a
  member in a wrap (§3.2), the permanent ban list (§2.1) and a filter list
  ([`filter-list.md`](filter-list.md) §1, subject `AUTHOR`).
- Member key as text is base32 per [`Protocol.md`](../Protocol.md) §13.2: 52 chars, lower case,
  no padding. A closed-room newcomer sends this to a moderator, by any means, to be admitted
  (§4.2).
- Client stores a private member key like a password: whoever has it posts as that member, and
  in a closed room reads what it reads.

### 1.2 Names

A room has room key and an owner-chosen `id` of 1 to 16 bytes, so one key can have several
rooms.

```
public room:
room    = H("ed3k v1 room", key ‖ id)[0..16)

private and closed room:
k       = H("ed3k v1 room private", secret ‖ key ‖ id)                    32 bytes
room    = H("ed3k v1 room private name", k)[0..16)
target  = H("ed3k v1 room private target", k ‖ period(8))[0..16)          period = Unix time / 3600, rounded down, big-endian
k_data  = H("ed3k v1 room private data", k)                               32 bytes, private room only (§3.1)
```

- `room` names room in every message and room signature (§2). A private room's `room`
  differs from the public room's with same key and `id`: knowing only key and `id`, a node can
  neither join nor read a private room.
- `secret` = 32 random bytes.
- Kad target: public room `room`; private or closed room `target`, changing hourly (§5).

### 1.3 Links

```
ed3k:room?key=<room key, base32 lower case, no padding>&id=<id, base32>[&s=<secret, 32 bytes, base32>][&dn=<url-encoded name>]
```

- No `s` → public room; with `s` → private or closed, which one room info says (§2.1).
- `key`, `id`, `s`: base32 per [`Protocol.md`](../Protocol.md) §13.2, written lower case w/o
  padding, read either way.
- A link is invalid when `key` doesn't decode to 32 bytes, `id` to 1 to 16 bytes or `s` to 32 bytes, or
  when `key`, `id`, `s` or `dn` is missing where required or repeated. `key` and `id` required.
- **A link with `s` is a credential.** A client MUST NOT write secret to a log or show it in
  an error message, and SHOULD mark such a link private wherever shown. In a closed room the
  secret alone doesn't admit (§2.2).

---

## 2. Items

Everything a room holds is an **item**: a signed body of one of six kinds.

```protobuf
message RoomItem {
  bytes body = 1;             // a serialised RoomBody, verified over exactly these bytes
  bytes key = 2;              // 32 B signer
  repeated Signature sigs = 3;  // over "ed3k v1 room item" ‖ room(16) ‖ body
}

message RoomBody {            // exactly one field is set
  RoomInfo info = 1;          // §2.1, signed by the room key
  Admit    admit = 2;         // §2.2, signed by the room key or a moderator
  Ban      ban = 3;           // §2.3, likewise
  Presence presence = 4;      // §2.4, signed by a member key
  RoomPost post = 5;          // §2.5, likewise
  KeyDrop  key_drop = 6;      // §3.2, signed by the room key or a moderator
}
```

- `room` per §1.2, so an item is invalid in another room, and a private room's item invalid in
  the public room of same key and `id`.
- `RoomBody` with no field or >1 set fails content check (§4.4). Unknown fields ignored.
- **Size.** `RoomItem` ≤ 8 KiB *(initial)*; item of `info` or `key_drop` ≤ 72 KiB *(initial)*;
  measured like a `Signed` in [`Protocol.md`](../Protocol.md) §10.7 but with `key` counted: over
  `body`, `key` and `sigs`. Larger fails content check.
- **Time.** Every body has a time in Unix seconds (`issued` or `sent`). Time more than 1 h
  *(initial)* ahead of node's clock fails a time check (§4.4).
- **Who may sign.** `info`: room key. `admit`, `ban`, `key_drop`: room key or a moderator of the
  room info node holds (§2.1). `presence`, `post`: a member key, i.e. any key but room key. A
  `RoomItem` breaking this for `info`, `presence` or `post` fails signature check (every node
  checks it alike). An `admit`, `ban` or `key_drop` whose signer is no moderator is dropped
  without fault (§4.4), since the deciding room info may differ.
- **Fields.** Content check fails on: `member` ≠ 32 B, `key_id` ≠ 8 B, `e` ≠ 32 B, `reply_to`
  present and ≠ 16 B, `seq` or `KeyDrop.gen` of 0, `title`, `about` or `nick` over its limit. In a
  sealed post a bad `reply_to` is found only when it opens, and §3 applies.

### 2.1 Room info

```protobuf
message RoomInfo {
  uint64 seq = 1;             // 1 or higher; a higher seq replaces a lower one
  uint64 issued = 2;
  string title = 3;           // at most 128 B of UTF-8
  string about = 4;           // at most 1,024 B of UTF-8
  repeated bytes mods = 5;    // 32 B member keys of the moderators; at most 8 (initial)
  bool   closed = 6;          // only admitted members join, read and post (§2.2, §3.2)
  bool   moderated = 7;       // only admitted members post and are listed present; everyone may read
  bytes  banned = 8;          // n × 16 B member IDs banned for good, ascending, each once; at most 4,096 (initial)
}
```

- Node holds one room info per room: highest `seq`; on equal `seq`, lower `body` as byte string.
- No room info → room is public or private per its link, no moderators, neither closed nor
  moderated, no admits, bans or keys. Owner SHOULD sign a room info with `seq` 1 on creating
  room.
- `closed` valid only if link has `s`; node refuses a room info with `closed` in a public room
  as content failure.
- Content check fails on a `mods` key that is room key, not 32 bytes, or twice; and on an
  unsorted or repeated `banned`.
- **Changes take effect at once.** A key dropped from `mods` signs nothing more that counts, and
  its admits, bans, lifts and `KeyDrop`s stop counting (§2.2, §2.3, §3.2); one arriving later is
  dropped, as from any non-moderator (§4.4). Owner dropping a moderator re-admits members it
  wants kept and re-bans those it wants out. Member ID added to `banned` is banned as by §2.3,
  with no end.
- Room info can turn a room closed or back. When it turns closed, a moderator starts a
  generation (§3.2). Node taking a room info that turns room closed closes every exchange of
  room opened without the steps of §4.2. Of each, the side that opened it rejoins by §4.2, while its
  own member key is admitted.

### 2.2 Admit

```protobuf
message Admit {
  bytes  member = 1;          // 32 B member key
  uint64 issued = 2;
  uint64 until = 3;           // Unix time in seconds; 0 = no end
}
```

Admit lets a member key into a closed room, or lets it post in a moderated one.

- **A member is admitted** while node holds a valid, not-over admit for its key (`until` 0 or in
  future) and the key isn't banned (§2.3). Node keeps the admit with latest `issued` per key and
  signer; on equal `issued`, lower `body` as bytes. The one that counts is the latest among
  those whose signer counts (§2.1): when a moderator is dropped, another's admit replaces it.
  Node forgets an admit 30 days *(initial)* after its `until`; an older admit without end can
  then be shown again: admit without end ends only with a ban.
- Moderator is admitted by room info listing it; needs no admit.
- In a public or private room not `moderated`, admits are kept and passed on and change nothing.
- Node holds admits of ≤ 1,024 *(initial)* keys per room, not counting banned keys or keys whose
  admit is over. It drops and doesn't pass on a further one; a moderator wanting more members removes
  some first.
- Admit ends with `until` or a ban; no other revocation. A room wanting to recheck members uses
  ending admits and renews them.
- Member whose admit ends without a ban no longer posts but still holds the current
  generation's key; reads until the next generation (§3.2), which comes within 24 h *(initial)*
  while posts are made. Moderator wanting it out at once bans it.

### 2.3 Ban

```protobuf
message Ban {
  bytes  member = 1;          // 32 B member key
  uint64 issued = 2;
  uint64 until = 3;           // Unix time in seconds; at most issued + 30 days (initial); until ≤ issued lifts the ban
}
```

- **A member is banned** while node holds a ban for its key with `until` in the future, or
  while room info lists its member ID in `banned`. Of several bans of one key the latest `issued`
  counts, so a later ban with `until` ≤ its `issued` lifts an earlier one; on equal `issued`,
  lower `body` as bytes. A lift doesn't lift a `RoomInfo.banned` ban.
- Ban lasts ≤ 30 days *(initial)*. Moderator wanting longer re-signs; owner bans for good in
  room info. Node drops a ban whose `until` is more than 30 days after its `issued`, as content
  failure.
- **A moderator does not ban a moderator.** Ban of a key in `mods` counts only if signed by room
  key. A banned key is no moderator while the ban lasts, whatever `mods` says. Admits, bans,
  `KeyDrop`s it signed with `issued` within the ban don't count, also after the ban ends. Node
  remembers such a ban window while key is in `mods`, else 30 days *(initial)* after ban ends;
  ≤ 64 *(initial)* windows per key, oldest forgotten first. An item signed in a forgotten window
  counts again: moderator could have signed it after the ban too. A `RoomInfo.banned` listing
  leaves no window: owner that unlists a moderator and wants its items of that time void drops
  it from `mods` (§2.1).
- Node keeps a ban until its `until`, a lift until 30 days *(initial)* after its `issued`; an
  older ban arriving after that is over by then.
- **What a ban does.** Node drops every post of that key, including held ones and ones sent
  before the ban, and passes none on (§2.5). In a closed room it also closes exchange of a
  neighbour that showed that key (§4.2) and refuses it with `ROOM_BLOCKED` while the ban lasts;
  banning moderator starts a generation without it (§3.2).
- Node holds ≤ 4,096 *(initial)* bans and lifts per room; beyond, drops those ending first.

### 2.4 Presence

```protobuf
message Presence {
  uint64 issued = 1;
  uint64 expires = 2;         // at most issued + 15 min (initial)
  string nick = 3;            // at most 64 B of UTF-8
  bool   left = 4;            // the member has left
  uint64 gen = 5;             // closed room: the newest generation whose key I hold (§3.2); 0 = none
}
```

- Member signs a presence on join, every 5 min *(initial)* while in room, and one with
  `left` on leaving; each with later `issued` than the previous.
- **≥ 60 s *(initial)* between a member's presences**, as receivers pass on 1 per minute (§4.4).
  A change (nick, `gen`) within that time waits, sent once at its end with the newest state.
  `left` waits for nothing.
- Node holds the presence with latest `issued` per member key. Doesn't give out one whose
  `expires` passed; remembers its `issued` until 1 h *(initial)* after that `expires`, so an
  older one can't be shown again (on-air notice rule, [`pubsub.md`](pubsub.md) §4.2).
- Held presences = node's list of members in room. ≤ 4,096 *(initial)* per room, oldest
  dropped beyond.
- Presence of a banned key, and in a closed or `moderated` room of a non-admitted key, is
  dropped, not passed on. So a moderated room lists only those who may post.
- `gen` is 0 outside a closed room. Node ignores it there: room info that turned room
  may not have reached every node yet.
- `nick` is a claim; two members can pick the same (§6).

### 2.5 Post

```protobuf
message RoomPost {
  uint64   sent = 1;          // Unix time in seconds, by the author's clock
  RoomText text = 2;          // public room
  bytes    sealed = 3;        // private and closed room: nonce(12) ‖ ChaCha20(k_post, nonce, serialised RoomText) (§3)
  uint64   gen = 4;           // closed room: the generation of k_post; 0 otherwise
  bytes    key_id = 5;        // closed room: 8 B, the key_id of that generation (§3.2)
}

message RoomText {
  string text = 1;            // 1 to 4,096 B of UTF-8
  bytes  reply_to = 2;        // 16 B id of the post this one answers; absent = none
}
```

```
post id = H("ed3k v1 room post", room ‖ body)[0..16)          body = RoomItem.body
```

- Public: `text`, no `sealed`, `gen` or `key_id`. Private: `sealed` only. Closed: `sealed`,
  `gen` and `key_id`.
- Private-room form in a closed room, or closed-room form in a private one: dropped without
  fault, not passed on (room info that turned room may not have reached every node).
  Content check fails on a post fitting neither form (`text` in a room with a secret, `sealed`
  in a public room, both `text` and `sealed` or neither, `gen` without `key_id`, `key_id`
  without `gen`, `gen` or `key_id` in a public room), `sealed` shorter than 12 bytes, `text`
  empty or above 4,096 bytes.
- **Longer text** = several posts, as in [`chat.md`](chat.md) §2.
- `sent` orders posts for display. Author's claim; node checks only that it's not more than 1 h
  ahead (§2) and not older than history (§4.3).
- Same post id as one held or remembered = duplicate; dropping is normal (posts arrive via
  several neighbours).
- **Who may post.** Node drops a post without fault, not passing it on, when: author banned
  (§2.3); room `moderated` or `closed` and author not admitted (§2.2); in a closed room, its
  generation is behind (§3.2); over its author's rate (§4.4). The relaying neighbour may not have
  known; the item still had a valid signature.
- Node that can't open `sealed` (lacks that generation's key) still keeps and passes on the
  post, once it holds a `KeyDrop` of it (§3.2); reads it once it has the key.
- `reply_to` naming no held post is shown as answer to a post node lacks.

---

## 3. Sealed posts

Transport encrypts every exchange (T2 of [`Protocol.md`](../Protocol.md) §3), and only members
are room neighbours. Sealing covers the rest: a removed node still holding secret, and posts
on disk.

```
sealed = nonce(12) ‖ ChaCha20(k_post, nonce, serialised RoomText)      RFC 8439 cipher, counter from 0
```

- `nonce` random per post.
- No authentication tag: author's signature fixes every byte of `sealed`, and node checks it
  without the key, as a private-channel node verifies a block it can't read
  ([`live.md`](live.md) §10.2).
- `sealed` opening to bytes not parsing as `RoomText`, or to a `text` breaking §2.5, is the
  author's fault, not the neighbour's: node keeps it out of view and passes it on as is.

### 3.1 Private room

`k_post` = `k_data` (§1.2), for every post, as long as room isn't closed.

- Everyone with link reads every post, also ones from before it joined, while members hold
  them (§4.3).
- **A private room cannot take a member's reading away**, like a private channel
  ([`live.md`](live.md) §10.5). A ban drops the member's posts, nothing else: member keeps the
  secret, rejoins under a new member key and reads. A room that must remove members is closed.

### 3.2 Closed room: generations

Closed-room posts are sealed with a key a moderator draws and gives each admitted member
separately. New key = new **generation**.

```protobuf
message KeyDrop {
  uint64 gen = 1;             // 1 or higher
  bytes  key_id = 2;          // 8 B
  uint64 issued = 3;
  bytes  e = 4;               // 32 B X25519 public value, new for every KeyDrop
  bytes  wraps = 5;           // n × 64 B: member ID(16) ‖ wrap(48); at most 1,032 (initial), ascending by member ID, each once
}
```

```
g        = 32 random bytes, drawn by the moderator for the generation
key_id   = H("ed3k v1 room key id", k ‖ g)[0..8)
k_post   = H("ed3k v1 room generation", k ‖ g)

x        = the member key converted to X25519 (Protocol.md §6.4)
kw       = H("ed3k v1 room wrap", X25519(e's private key, x) ‖ e ‖ member key ‖ gen(8))
wrap     = ChaCha20-Poly1305(kw, nonce = 12 zero bytes, g)                 RFC 8439, empty associated data; 48 B
```

- Checks of [`Protocol.md`](../Protocol.md) §6.4 apply to member keys as to node keys. Moderator
  wraps for no member key whose y is 1 or −1. Member getting an all-zero X25519 output, e.g. from
  an `e` of small order, treats its wrap as tag-failed (below).
- `k` enters `key_id` and `k_post`, so `g` alone opens nothing: a wrap leaked to someone without
  secret is useless.
- Wrap nonce is zero; safe since `kw` is new per `e` and member, and a moderator MUST draw `e`
  anew for every `KeyDrop`.
- Member finding its member ID in `wraps` opens its wrap with its member key and checks `key_id`
  follows from `g`. Failed tag, or `g` giving another `key_id` = moderator's fault: node ignores
  that `KeyDrop` for itself, passes it on, MAY tell its user.
- `wraps` covers the admitted members the moderator knows, moderators and itself included.
  `KeyDrop` with a member ID twice, or `wraps` not a multiple of 64 bytes, fails content check.
  1,032 = admits of 1,024 keys + 8 moderators.
- Node doesn't check whom a moderator wrapped for (it can open only its own wrap). Moderator
  decides who reads.

**Which generation is current**

- **Current generation** = highest `gen` of a valid `KeyDrop` node holds; on equal `gen` with
  different `key_id`, lower `key_id` as byte string. A `KeyDrop` of a key no longer in `mods`,
  or banned, doesn't count: node recomputes current generation without it. A room-key
  `KeyDrop` always counts. A moderator jumping `gen` far ahead, by mistake or on purpose, so
  holds room only until owner drops it. Two moderators can start a generation at once;
  every node then takes the same one, and the other's posts stay readable to holders of its key.
  A post under the other `key_id` of the current `gen` isn't behind: kept and passed on like any
  other, once node holds a `KeyDrop` of that `key_id` (below).
- Several `KeyDrop`s can carry one generation: one starting it, later ones adding wraps for
  members admitted since; each has its own `e`. Node holds every `KeyDrop` of a generation it
  holds a post of, and of the current one, ≤ 64 *(initial)* per room, dropping oldest
  generations' first. A `KeyDrop` of a generation below current, with no held post, is dropped
  without fault, not passed on: a later wrap of an earlier generation reaches the members holding
  its posts.
- Generation **began** = earliest `issued` of its counting `KeyDrop`s node holds.
- **A member posts under current generation**, only when holding its key. A member knowing a
  current generation whose key it lacks posts nothing and waits for a wrap (below).
- **A post of an older generation** is dropped without fault when its `sent` is more than 2 min
  *(initial)* after the `issued` of the earliest held `KeyDrop` of current generation.
  `sent` is the author's claim, so this only ends late posts of honest members; a removed member
  is stopped by its ban, or by the admit it no longer has (§2.5), whatever `sent` it writes.
- **A post ahead of node.** Post of current or higher `gen` whose `gen` and `key_id` no
  counting held `KeyDrop` carries: kept 60 s *(initial)* without passing on, ≤ 64 *(initial)* per
  room, oldest dropped first. Node rechecks it when a `KeyDrop` arrives (its `KeyDrop` may still
  be on the way). After 60 s dropped without fault.

**Starting a generation.** Moderator, or owner with room key, sends a `KeyDrop` with `gen` one
above current, a new `g`, and wraps for every admitted, non-banned member it knows:

| When | |
|---|---|
| It bans a member of a closed room | MUST, at once, without that member |
| Owner removes a moderator, or turns room closed (§2.1) | MUST, at once |
| 24 h *(initial)* after current generation began, when a post of that `gen` was made | SHOULD |
| Room was made | owner MUST, with `gen` 1, before the first post |

A generation needs a moderator or owner online; with all moderators away, the room goes on
under its current generation.

**Wraps for members that lack the key.** A moderator holding the current generation's key that
learns a presence of an admitted member with `gen` below current, when no held `KeyDrop` of the
current generation has a wrap for that member, SHOULD send a `KeyDrop` of same `gen`, `key_id`
and `g` with a wrap for that member and every other known admitted member still lacking one. A
moderator MAY do the same on admitting a member, so the key waits for it in room. This is how
a member admitted, or offline, when the generation began gets the key. A moderator whose current
generation already has 32 *(initial)* counting `KeyDrop`s, of any `key_id` of that `gen`, starts
a new generation instead. A moderator MAY likewise add wraps of earlier generations it still
holds, as `KeyDrop`s of those generations: gives a new member history (§4.3). Without them a
new member reads only what's posted after it joined.

**Keys are forgotten.** Node deletes a generation's key once it holds no post under it and a
later generation began ≥ 1 h *(initial)* ago; at latest 48 h *(initial)* after a later
generation began. Whoever later takes a member's key or disk reads at most what's posted in its
current generation and the two days before.

---

## 4. Between two members

Two members exchanging a room are **neighbours**. One opened the **room exchange**; after that
both sides are equal.

```protobuf
message RoomJoin {            // type 42
  bytes  room = 1;            // 16 B (§1.2)
  uint32 want_peers = 2;      // max number of peers wanted, 0 = none
  bytes  relays4 = 3;         // n × 22 B compact nodes: my relays, if I am a leaf
  bytes  relays6 = 4;         // n × 34 B
  bytes  proof = 5;           // private and closed room: 16 B (§4.1)
  uint64 since = 6;           // my clock when I last had a neighbour of this room; 0 = never (§4.3)
  uint64 info_seq = 7;        // seq of the room info I hold; 0 = none
}

message RoomState {           // type 43
  repeated RoomItem items = 1;
  repeated Peer peers = 2;    // other members of this room
  RoomRefusal refusal = 3;    // set: I do not take you as a neighbour
  bytes  proof = 4;           // private and closed room: 16 B, in the first answer to a valid proof
  bytes  member = 5;          // closed room: 32 B, my member key (§4.2)
  RoomProof member_proof = 6; // with `member`
  bool   synced = 7;          // the last RoomState of my first answer, or of my catch-up as joiner (§4.3)
}

message RoomProof {
  uint64 minute = 1;          // Unix time / 60, rounded down
  bytes  sig = 2;             // 64 B
}

enum RoomRefusal {
  ROOM_OK = 0;
  ROOM_FULL = 1;              // no room for another neighbour
  ROOM_LEFT = 2;              // I am not in this room
  ROOM_BLOCKED = 3;           // I do not take you: refused for 24 h (§4.4), or banned (§2.3)
  ROOM_MEMBER = 4;            // closed room: you showed no valid admit
}
```

`Peer` and `Signature`: types of [`Protocol.md`](../Protocol.md) §5.4 and §10.7.

### 4.1 The room exchange

- Member opens an exchange, sends `RoomJoin`; peer answers `RoomState`. First answer holds: the
  room info if its `seq` ≥ `info_seq` (so one of same `seq` can win by §2.1 tie-break); every
  admit, ban, presence and `KeyDrop` node holds and gives out; its history (§4.3); peers if
  asked.
- Peer not taking joiner answers with `refusal` set and closes. With `ROOM_FULL` it SHOULD
  still fill `peers`. Peer not in room answers `ROOM_LEFT`.
- **Private and closed rooms: secret first.** Proofs as for a private channel
  ([`live.md`](live.md) §10.3), with other strings:

```
RoomJoin.proof   = H("ed3k v1 room join",   k ‖ joiner's node ID ‖ peer's node ID)[0..16)
RoomState.proof  = H("ed3k v1 room answer", k ‖ joiner's node ID ‖ peer's node ID)[0..16)
```

  `RoomJoin` without valid proof → `refusal = ROOM_LEFT`, nothing else. Every answer to a valid
  proof carries `proof`, whatever its `refusal`; joiner closes exchange if the first
  `RoomState` has no valid proof. Rules of [`live.md`](live.md) §10.3, with `RoomJoin`/`RoomState`
  for `LiveJoin`/`LiveState`.
- **After first answer both sides send `RoomState`** whenever they verified an item they keep
  (§4.4): to every neighbour that hasn't shown them that item, never back to its source. Nothing
  else is sent on exchange.
- Exchange stays open; connection kept alive as for a file stream
  ([`Protocol.md`](../Protocol.md) §4.1). Neighbour gone when exchange or connection closes.
  Either side leaves by closing exchange.
- Two nodes keep ≤ one room exchange per room, per [`pubsub.md`](pubsub.md) §2.1 rule for topic
  exchanges.
- `peers` MUST be peers sender itself had as neighbours of this room within the last hour
  *(initial)*, and in a closed room peers whose admit it checked (§4.2).
- **Limits.** `RoomJoin` names ≤ 2 relays per family, at addresses acceptable in the network
  ([`Protocol.md`](../Protocol.md) §8.2 (7)). `RoomState` = one frame, ≤ 1 MiB
  ([`Protocol.md`](../Protocol.md) §5.2); a larger first answer is split over several (§4.3).
  A message above these limits is malformed; node closes exchange.
- **Order of checks** for `RoomJoin`: `Error{UNSUPPORTED}` without the capability;
  `Error{MALFORMED}` for `room` ≠ 16 bytes, `proof` neither empty nor 16 bytes, or relays over the
  limits above; `ROOM_LEFT` for a room it's not in or failing proof; then `ROOM_BLOCKED` for a
  node it refuses (§4.4); in a closed room the admit (§4.2); then `ROOM_FULL`.

### 4.2 Closed room: admitted members only

Closed-room node shows its member key only to a node that showed a valid admit first, like a
pass holder ([`subscription.md`](subscription.md) §6.3). Peer's first, joiner's second.

```
RoomProof.sig = Ed25519 signature by the member key over
                "ed3k v1 room proof" ‖ room(16) ‖ sender's node ID(16) ‖ receiver's node ID(16) ‖ minute(8)
```

1. Joiner sends `RoomJoin` with its secret proof.
2. Peer's first `RoomState` carries `proof`, its `member` and `member_proof`, and in `items` the
   room info and its own admit (none for a moderator); nothing else of room: no peers, no
   history, no `refusal` other than `ROOM_LEFT`, or `ROOM_BLOCKED` for a node it refuses (§4.1),
   which carries `proof` and nothing else.
3. Joiner checks room info (valid under room key, `closed` set), peer's admit or its place in
   `mods`, that the key isn't banned by what joiner holds, and proof; closes exchange if
   any fails. Else sends one `RoomState` with its own `member`, `member_proof` and, in `items`,
   its admit if it holds one, nothing else.
4. Peer checks the same, with the admit shown or one it holds for that key. Answers
   `ROOM_MEMBER`, or `ROOM_BLOCKED` for a banned key, and closes; or sends its first answer per
   §4.1.

- **A newcomer** need only send its member key to a moderator (§1.1). Moderator's admit reaches the
  room as an item; a peer holding it takes the newcomer at step 4, and its first answer gives
  the newcomer the admit.

- Both node IDs of proof are those of the authenticated connection
  ([`Protocol.md`](../Protocol.md) §6.2). Proof useless on another connection; admit useless
  without the private member key.
- `minute` = sender's clock; receiver refuses a proof more than 10 min *(initial)* off its own.
- Node passes on an admit shown at a join as an item, so room learns of members admitted
  since.
- **While exchange lasts.** Node closes exchange of a neighbour whose key it learns is
  banned, whose admit is over, or that is no longer a moderator and has no admit. Answers a
  `RoomJoin` of that node ID with `ROOM_BLOCKED` while a ban lasts.
- Node holding secret but no admit learns from a peer ≤ one member key and that member's
  admit; no post, presence, peer or key.
- Node holding no admitted member key, or whose own admit is over, isn't in room: answers
  `ROOM_LEFT`, stores no `SUB` record (§5).
- Joiner holding a room info with `closed` that gets a first answer without the steps above takes
  that answer's room info, if any. If room is still closed, sends peer its own room info
  alone and closes exchange.

### 4.3 History

- Node keeps a room's posts of the last 24 h *(initial)* by `sent`, ≤ 1,000 *(initial)*, the
  newest, kept as they arrived, `sealed` included. Drops posts with older `sent`; remembers post ids
  dropped for age 1 h *(initial)* longer.
- **First answer carries history:** every held post with `sent` ≥ `since` minus 1 h
  *(initial)*, oldest first, after the other items. The hour = clock tolerance of §2: an
  author's clock may be that far off the joiner's. A first answer not fitting one frame (with
  its other items or history) goes in several `RoomState`s, each ≤ 1 MiB; last sets `synced`.
  Once `synced` arrived, joiner sends peer the non-post items first answer didn't show,
  and posts of the same window (`sent` ≥ `since` minus 1 h) it didn't show: its **catch-up**, in
  as many `RoomState`s as needed, last with `synced`; an empty `RoomState` with `synced` if
  nothing to send. Thereafter both sides only push (§4.1).
- `since` = joiner's own clock, not a `sent`: last time it had a neighbour of room, or 0 when
  it holds no post of room, also after a restart that kept none. A returning joiner thus
  gets what it missed plus the hour before, whatever authors' clocks, and sends back only that
  hour, not its whole history. Node with `since` 0 gets the whole history, sends no post.
- History posts pass §2.5 and §4.4 checks like any other, except the rate.
- Members hold history, only while in room; a room whose members all leave keeps
  nothing. Nobody else stores posts: no store for offline members, as for chat
  ([`chat.md`](chat.md) §2).

### 4.4 Receiving an item

Node checks an item before anything else: size, signature, content first, then time, then rules
of §2 and §3. Content failure is a fault whatever those rules say of the item; the closed or
private room form (§2.5) is a rule. Then:

| Item | Node |
|---|---|
| valid, and new to it (newer room info, admit, ban, presence or `KeyDrop`, or unknown post id) | keeps it, passes it on (§4.1) |
| valid, not new | drops it. Normal: items arrive via several neighbours |
| valid, dropped by a rule of §2 or §3 (banned author, author not admitted, generation behind, post of the other form of a room with a secret, signer not a moderator, a limit per room or per neighbour) | drops it, doesn't pass it on. No fault of neighbour, which may not have known |
| fails its signature or content check | drops it, closes exchange, refuses that node as neighbour of any room for 24 h *(initial)* |
| fails a time check (`issued` or `sent` ahead, `expires` passed, post older than history) | drops it. Clocks differ: no fault of neighbour |

- **Rate of an author.** Node passes on ≤ 20 *(initial)* posts per author key and minute, drops
  the rest without fault (value of [`chat.md`](chat.md) §2); ≤ 1 presence per member key and
  minute *(initial)*, plus one with `left`; ≤ 60 admits, bans and `KeyDrop`s per signer and minute *(initial)*. A
  neighbour is at fault only when, in any 10 s, it passes on more than 10× a limit's 10 s share,
  rounded up: > 40 posts of one author key, 10 presences of one member key, or 100 admits, bans
  and `KeyDrop`s of one signer. Resent items count again. Node then closes exchange.
- **Rate of a neighbour.** Member keys cost nothing, so author rate doesn't bound a node signing
  with many. Node takes ≤ 600 *(initial)* items per neighbour and minute, not counting first
  answer and joiner's catch-up (§4.3); drops the rest without fault. Fault rule above applies
  here too: > 1,000 items in 10 s.
- **First answer and catch-up.** No rate of this section applies to items of a first answer or
  joiner's catch-up (§4.3), up to as many of each kind as a node holds (§2.2 to §4.3): 1,024
  admits of one moderator come in one answer. Further items are rated. Node passes on what it
  took from them within author and signer rates above; keeps the rest without passing on.
  Neighbour rate bounds what node takes, not what it passes on. Room info has no rate and is
  always passed on.
- **Waiting for room info.** Node holding no room info, or one with lower `seq` than a
  neighbour's `RoomJoin.info_seq` named, keeps an admit, ban or `KeyDrop` with non-moderator
  signer for 60 s *(initial)* without passing on, and rechecks it when room info arrives; ≤ 64
  *(initial)* such items per room, oldest dropped beyond. Otherwise a non-moderator-signed item
  is dropped at once, without fault. First answer brings room info before other items (§4.1).

### 4.5 Neighbours

- Node aims at 3 neighbours per room, keeps 2 to 4 *(initial)*; below 2 looks for more (§5). If
  it can't reach 2, it stays in room with what it has.
- Neighbours subject to bucket diversity limits: ≤ 1 per host, 2 per site, 4 per block
  ([`ip-diversity.md`](ip-diversity.md) §1 and §2).
- **A node is in at most 16 rooms** *(initial)*. Own limit: the 16 topics of
  [`pubsub.md`](pubsub.md) §2.3 don't count towards it.

---

## 5. Finding members

Member publishes a `SUB` record ([`pubsub.md`](pubsub.md) §5) under the room's Kad target:
`room` for public, `target` of the current hour for private or closed (§1.2).

- It is the record of [`pubsub.md`](pubsub.md) §5 in every way: source record bound to the
  connection, unsigned, lifetime 60 min *(initial)*, re-stored every 30 min while in room,
  with that section's per-target limits.
- **A private or closed room stores under target of the hour.** Member stores under the new
  target at once when the hour changes. In an hour's first 10 min *(initial)* a member finding
  nobody also looks under the previous hour's target, in the last 10 min under the next hour's,
  as [`live.md`](live.md) §10.4 does.
- **Joining** as in [`pubsub.md`](pubsub.md) §5: member looks up `SUB` records under target,
  sends `RoomJoin` to peers from the answer and from `RoomState.peers` until it has its
  neighbours, publishes its own record. Failed dials count under
  [`ip-diversity.md`](ip-diversity.md) §2.5 limits, a room counting as a file.
- **A member under its anonymous key** ([`hops.md`](hops.md) §1) stores an `ANON` record under
  target instead of `SUB`, as [`pubsub.md`](pubsub.md) §5 says; its member key MUST NOT be
  used under its node key as well. A `Peer` naming it carries its node ID alone
  ([`hops.md`](hops.md) §3.3). Under that key a node in no room answers `ROOM_LEFT`, not
  `Error{UNSUPPORTED}`, as a hidden node does for topics and streams.
- A LAN peer ([`lan-discovery.md`](lan-discovery.md) §3) MAY be a neighbour; its address isn't
  passed on in `peers`.

---

## 6. Showing a room *(informative)*

- Client shows posts as plain text; follows no link and fetches nothing a post names until the
  user asks: a post naming a file only its author shares would tell author who reads the
  room.
- Client shows a member by nick plus part of its member ID, so same-nick members are told apart;
  marks moderators' posts.
- Posts shown in `sent` order; a post whose `sent` is far from arrival time MAY be marked.
- A client MAY hide posts of a member ID that a followed filter list names as `AUTHOR`
  ([`filter-list.md`](filter-list.md)). A list doesn't stop it passing those posts on, nor
  taking that member as neighbour ([`filter-list.md`](filter-list.md) §3).
- User muting, for themselves alone, is local; a ban is what a moderator does for room.
- One member writing another alone uses chat ([`chat.md`](chat.md)), which needs node ID; a
  room shows only member keys. A member wanting private contact posts a friend link
  ([`friend.md`](friend.md)).

---

## 7. What an attacker can do

| Attack | What limits it |
|---|---|
| **Post as someone else** | Impossible without that member key; every hop verifies |
| **Change a post on the way** | Signature covers `sealed` and `text`; every hop verifies |
| **Show an old post again** | Post id known, or older than history |
| **Flood a public or private room** with new member keys | Keys cost nothing. Author rate bounds each key, neighbour rate each node (§4.4), moderators ban; a flooded room turns `moderated`, dropping posts and presences of non-admitted keys |
| **A banned member of a private room reads on** | Possible: it holds secret. That's what a closed room is for (§3.1) |
| **A removed member of a closed room reads on** | Not what's posted from the next generation; reads what it held before. A member still relaying posts to it leaks them; no key stops that |
| **A removed member posts under its old key** | Dropped: banned or no longer admitted (§2.5). Holding the key doesn't help |
| **A member that passes nothing on** | Neighbours have others; mesh keeps 2 to 4 per node. Diversity limits raise the cost of surrounding a member |
| **Send bad items** | One per attacker node; that node is then refused as neighbour of every room for 24 h (§4.4) |
| **Fill the room's Kad target** with records of non-member nodes | Wasted dials, limited as for false sources ([`ip-diversity.md`](ip-diversity.md) §2.5) |
| **A moderator turns against room** | Can admit, ban other members and hand keys to whom it wants, until owner drops it from `mods`; its admits, bans and `KeyDrop`s then stop counting (§2.1, §3.2). Can't ban owner's other moderators. Owner key can change everything |
| **Owner key is taken** | Nothing helps; no successor key (§11) |
| **Two moderators start a generation at once** | Every node takes the same one (§3.2); nothing lost |

---

## 8. What it shows

| To whom | What |
|---|---|
| A neighbour | node ID, address, member key of this room (closed room: after both showed an admit) |
| Every member | member keys, nicks, posts of room; closed room: who admitted whom |
| A storing node of the room's target | addresses of members publishing there. Public room: anyone with link computes target; private or closed: target changes hourly, can't be tied to a room |
| A holder of a closed room's secret without an admit | one member key and its admit (§4.2) |
| Anyone with a filter list | that a member ID of a room is author it names, as for any key |
| Someone who later takes a member key or a member's disk | posts of its current generation and up to two days before (§3.2); private room: every post members still hold |

- Member key keeps rooms apart, and apart from node: neighbours see node ID, but a post
  reaches the rest of room without it.
- A member not wanting neighbours to see its address joins under its anonymous key (§5).

---

## 9. Cost

| | Size |
|---|---|
| Post | ~200 B for a short line, ≤ 8 KiB; to each neighbour once |
| Presence | ~150 B per member per 5 min, to each neighbour once |
| `KeyDrop` | 64 B per member: 64 KiB for 1,024 members, per generation, so once a day |
| History at a join | ≤ 1,000 posts: ~200 KB for short ones, at most a few MiB |
| Connections | ~3 per room, held open; ~48 for 16 rooms |
| `SUB` record | one store per 30 min, on ~8 nodes |

A room of 1,000 members, all present, sends each member ~0.5 KB/s of presences per neighbour.
Protocol doesn't limit room size, except admits of 1,024 keys in a closed room (§2.2).

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A topic of [`pubsub.md`](pubsub.md)** with a type for rooms | no own messages | topic holds one item, the newest; room holds many, plus history |
| **Chat to every member**, each post sent by its author to all | nothing new on the wire | author connects to every member; needs their node IDs |
| **One node hosts room** and passes posts on | order and history in one place | a server, which ed3k leaves to index nodes (`Protocol.md` §11); reads every post |
| **Posts in Kad**, as signed records | offline members read later | record lifetime and size; every post a store on 8 nodes; a store anyone can fill |
| **Node key as author** | friend recognised in a room | every post tied to node and all it shares; rooms tied to each other |
| **Mesh as the only gate**, no sealing | no keys to hand out | removed member still holding secret reads on whenever it slips in; on-disk history stays readable |
| **Messaging Layer Security** (RFC 9420) | forward secrecy and post-compromise recovery, any group size | ratchet tree whose changes every member must apply in same order, needing one ordering service |
| **A sender key per member**, as in Signal groups | removed member reads nothing new; no moderator for keys | every member hands its key to every other over two-party channels at every change: n² messages |
| **Sealing each post to every member** | no shared key | post grows with room |
| **A key per epoch in a hash chain**, as for paid scopes ([`subscription.md`](subscription.md) §4) | no moderator online per epoch | chain can't leave out a member: holder of a key holds every earlier one of its season |
| **Rate limits by proof of work** on new member keys | flooding costs something | honest low-power members pay too; moderation cheaper |

---

## 11. Open points

- Every number *(initial)*; none measured.
- **Succession.** No successor for a lost or stolen room key, as for channels
  ([`live.md`](live.md) §12).
- **Listing public rooms**; options of [`live.md`](live.md) §11 apply too.
- **Paid rooms**, a fourth scope kind in [`subscription.md`](subscription.md) §2.
- Editing or retracting one's own post; a moderator removing one post rather than an author.
- Closed room larger than 1,024 members.
- A generation needs a moderator online (§3.2). Whether any member may start one when moderators
  are away long.
- Recovery after a member key is taken: thief reads until the next generation without that
  member, and a moderator can't tell.
- Whether a member sharing a member key across its own devices should be allowed, as a pass
  allows `devices` ([`subscription.md`](subscription.md) §5.3).
