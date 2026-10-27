# Extension: rooms (`room`)

**Status:** draft 0.1 (2026-10-27). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3 and §13, [`chat.md`](chat.md), [`pubsub.md`](pubsub.md) §2 and §5, and [`live.md`](live.md) §10. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 19, `ROOM`. Message types 42 and 43. `ed3k:room` links.
**Required of every `ROUTER`:** nothing. A room uses the record kind `SUB` of [`pubsub.md`](pubsub.md) §5, which every `ROUTER` already stores.
**Test vectors:** [`tests/vectors-v1.md`](../tests/vectors-v1.md), *Rooms*.

Chat ([`chat.md`](chat.md)) carries a message from one node to one other, over a connection
between the two. A **room** carries posts among many members.

- The members of a room form a mesh, as the subscribers of a topic do ([`pubsub.md`](pubsub.md)
  §2). Each keeps a few neighbours and passes on what it has verified.
- Every post is **signed by its author**. A post that reaches a member through others is still
  the author's, and nobody on the way can change it.
- Each member signs with a **member key** of its own for that room, never with its node key.
  Its posts are not tied to what its node shares, nor to its other rooms.
- Members keep the posts of the **last day** and give them to a member that joins or comes
  back. Only members of the room keep them: no Kad node and no stranger stores a post.
- The owner of a room names **moderators**. They admit members, ban them, and in a closed room
  hand out the keys.
- A room can be **public**, **private** or **closed**:

| Form | Link holds | Who joins | Posts are | Removing a member | Section |
|---|---|---|---|---|---|
| **Public** | room key, `id` | anyone with the link | signed, inside the transport only | a ban; the member's posts are dropped | 1.3 |
| **Private** | and a secret | anyone with the link | sealed with a key from the secret | a ban, as in a public room; it keeps the secret | 3.1 |
| **Closed** | and a secret | members a moderator admitted | sealed with a key that changes | a ban and a new key; the member reads no later post | 3.2 |

The mesh and the search for members are those of [`pubsub.md`](pubsub.md). The secret, its
proof and the target that moves every hour are those of private channels
([`live.md`](live.md) §10). How a member shows that it was admitted follows the passes of
[`subscription.md`](subscription.md) §5.2 and §6.3.

---

## 1. The room

### 1.1 Keys

| Key | Who holds it | Signs |
|---|---|---|
| **room key** | the owner | the room info (§2.1); also admits, bans and keys (§2.2, §2.3, §3.2) |
| **member key** | each member, one per room | its presence and its posts (§2.4, §2.5) |
| **moderator** | a member whose member key the room info lists | admits, bans and keys |

- All three are Ed25519 keys. The rules for signatures are those of
  [`Protocol.md`](../Protocol.md) §10.7.
- **The room key MUST NOT be a node key** ([`Protocol.md`](../Protocol.md) §6.1). It SHOULD
  be a key of its own, as a channel key is ([`live.md`](live.md) §1.1): the room then outlasts
  the node and does not name it.
- **A member key MUST NOT be a node key, nor a room key.** A client makes a new one for each
  room it joins and SHOULD NOT use one key in two rooms, for the reason of
  [`subscription.md`](subscription.md) §5.1: neighbours of one room would learn the others.
- The owner posts as any member does, with a member key of its own. It may list that key as a
  moderator. In a closed or `moderated` room a key not in `mods` needs an admit (§2.2), which
  the owner signs with the room key: when it makes such a room, and when a room info turns the
  room closed or `moderated`, together with that room info.
- The **member ID** of a member key is `H("ed3k v1 node id", member key)[0..16)`, computed as a
  node ID is. It names a member in a wrap (§3.2), in the permanent ban list (§2.1) and in a
  filter list ([`filter-list.md`](filter-list.md) §1, subject `AUTHOR`).
- A member key is shown as text in base32, as [`Protocol.md`](../Protocol.md) §13.2 fixes:
  52 characters, lower case, without padding. This is the form a newcomer of a closed room
  sends a moderator, by any way it likes, to be admitted (§4.2).
- The client stores a private member key like a password. Whoever has it posts as that
  member, and in a closed room reads what it reads.

### 1.2 Names

A room has the room key and an `id` of 1 to 16 bytes that the owner chooses, so one key can
have several rooms.

```
public room:
room    = H("ed3k v1 room", key ‖ id)[0..16)

private and closed room:
k       = H("ed3k v1 room private", secret ‖ key ‖ id)                    32 bytes
room    = H("ed3k v1 room private name", k)[0..16)
target  = H("ed3k v1 room private target", k ‖ period(8))[0..16)          period = Unix time / 3600, rounded down, big-endian
k_data  = H("ed3k v1 room private data", k)                               32 bytes, private room only (§3.1)
```

- `room` names the room in every message and in every signature of the room (§2). The `room`
  of a private room is not the one of the public room with the same key and `id`: a node that
  knows only the key and `id` can neither join nor read a private room.
- `secret` is 32 random bytes.
- The Kad target of a public room is `room`; that of a private or closed room is `target`,
  which changes every hour (§5).

### 1.3 Links

```
ed3k:room?key=<room key, base32 lower case, no padding>&id=<id, base32>[&s=<secret, 32 bytes, base32>][&dn=<url-encoded name>]
```

- A link without `s` names a public room, a link with `s` a private or a closed one. Which of
  the two it is, the room info says (§2.1).
- `key`, `id` and `s` are base32 as [`Protocol.md`](../Protocol.md) §13.2 fixes: written lower
  case without padding, read either way.
- A link is invalid when `key` does not decode to 32 bytes, `id` to 1 to 16 bytes or `s` to 32
  bytes, or when `key`, `id`, `s` or `dn` is missing where required or repeated. `key` and
  `id` are required.
- **A link with `s` is a credential.** A client MUST NOT write the secret to a log or show it
  in an error message, and SHOULD mark such a link as private wherever it shows it. In a
  closed room the secret alone does not admit (§2.2).

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

- `room` is that of §1.2, so an item of one room is not valid in another, and an item of a
  private room is not valid in the public room of the same key and `id`.
- A `RoomBody` with no field or more than one set fails its content check (§4.4). Fields a
  node does not know are ignored.
- **Size.** A `RoomItem` is at most 8 KiB *(initial)*, and an item of `info` or `key_drop` at
  most 72 KiB *(initial)*, measured as [`Protocol.md`](../Protocol.md) §10.7 measures a
  `Signed`, but with `key` counted: over `body`, `key` and `sigs`. A larger one fails its
  content check.
- **Time.** Every body carries a time in Unix seconds (`issued` or `sent`). An item whose time
  is more than 1 h *(initial)* ahead of the node's clock fails a time check (§4.4).
- **Who may sign.** The signer of `info` is the room key. The signer of `admit`, `ban` and
  `key_drop` is the room key or a moderator of the room info the node holds (§2.1). The
  signer of `presence` and `post` is a member key: any key but the room key. A `RoomItem` that
  breaks this for `info`, `presence` or `post` fails its signature check: every node checks it
  alike. An `admit`, `ban` or `key_drop` whose signer is no moderator is dropped without fault
  (§4.4), since the room info that decides may differ.
- **Fields.** A `member` other than 32 B, a `key_id` other than 8 B, an `e` other than 32 B, a
  `reply_to` that is present and not 16 B, a `seq` or `KeyDrop.gen` of 0, and a `title`,
  `about` or `nick` over its limit fail the content check. In a sealed post a bad `reply_to`
  is found only when the post opens, and §3 applies.

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

- A node holds one room info per room: the one with the highest `seq`, and of two with the
  same `seq` the one whose `body` is lower, compared as a byte string.
- A room without a room info is public or private as its link says, has no moderators, is
  neither closed nor moderated, and has no admits, bans or keys. The owner SHOULD sign a room
  info with `seq` 1 when it makes the room.
- `closed` is valid only in a room whose link has `s`. A node refuses a room info with
  `closed` in a public room as a content failure.
- A key in `mods` that is the room key, that is not 32 bytes, or that appears twice makes the
  room info fail its content check, and so does an unsorted or repeated `banned`.
- **Changes take effect at once.** A key dropped from `mods` signs nothing more that counts,
  and its admits, bans, lifts and `KeyDrop`s stop counting (§2.2, §2.3, §3.2); a node that gets
  one later drops it, as from any signer that is no moderator (§4.4). An owner that drops a
  moderator re-admits the members it wants to keep, and bans again those it wants kept out.
  A member ID added to `banned` is banned as by §2.3, with no end.
- A room info can turn a room closed or back. When a room turns closed, a moderator starts a
  generation (§3.2). A node that takes a room info that turns the room closed closes every
  exchange of the room that was opened without the steps of §4.2. Of each such exchange, the
  side that opened it joins again by §4.2, while its own member key is admitted.

### 2.2 Admit

```protobuf
message Admit {
  bytes  member = 1;          // 32 B member key
  uint64 issued = 2;
  uint64 until = 3;           // Unix time in seconds; 0 = no end
}
```

An admit lets a member key into a closed room, or lets it post in a moderated one.

- **A member is admitted** while a node holds a valid admit for its key that is not yet over
  (`until` 0 or in the future), and the key is not banned (§2.3). A node keeps the admit with
  the latest `issued` per key and signer; of two with the same `issued`, the one whose `body`
  is lower as bytes. The one that counts is the latest of those whose signer counts (§2.1):
  when a moderator is dropped, an admit of another takes its place. A node forgets an admit
  30 days *(initial)* after its `until`. An older admit without end can then be shown again:
  an admit without end ends only with a ban.
- A moderator is admitted by the room info that lists it, and needs no admit.
- In a public or private room that is not `moderated`, admits are kept and passed on and
  change nothing.
- A node holds admits of at most 1,024 *(initial)* keys per room, those of keys that are banned or
  whose admits are over not counted. It drops and does not pass on a further one; a moderator
  that wants more members removes some first.
- An admit ends with `until`, or with a ban. There is no other way to take one back. A room
  that wants to check its members again uses admits that end, and renews them.
- A member whose admit ends without a ban no longer posts, but still holds the key of the
  current generation. It reads until the next generation (§3.2), which comes within 24 h
  *(initial)* while posts are made. A moderator that wants it out at once bans it.

### 2.3 Ban

```protobuf
message Ban {
  bytes  member = 1;          // 32 B member key
  uint64 issued = 2;
  uint64 until = 3;           // Unix time in seconds; at most issued + 30 days (initial); until ≤ issued lifts the ban
}
```

- **A member is banned** while a node holds a ban for its key whose `until` is in the future,
  or while the room info lists its member ID in `banned`. Of several bans of one key the one
  with the latest `issued` counts, so a later ban with `until` at or below its `issued` lifts
  an earlier one. Of two with the same `issued`, the one whose `body` is lower as bytes
  counts. A lift does not lift a ban of `RoomInfo.banned`.
- A ban lasts 30 days *(initial)* at most. A moderator that wants it longer signs it again;
  the owner bans for good in the room info. A node drops a ban whose `until` is more than
  30 days after its `issued`, as a content failure.
- **A moderator does not ban a moderator.** A ban of a key in `mods` counts only when the room
  key signed it. A key that is banned is no moderator while the ban lasts, whatever `mods`
  says. An admit, ban or `KeyDrop` it signed with an `issued` within the ban does not count,
  also after the ban ends. A node remembers the window of such a ban while the key is in
  `mods`, and 30 days *(initial)* after the ban ends otherwise, at most 64 *(initial)* windows
  per key, the oldest forgotten first. An item signed within a window the node forgot counts
  again: the moderator could have signed it after the ban as well. A listing in
  `RoomInfo.banned` leaves no window: an owner that unlists a moderator and wants its items of
  that time void drops it from `mods` (§2.1).
- A node keeps a ban until its `until`, and a lift until 30 days *(initial)* after its
  `issued`. An older ban that arrives after that is over by then.
- **What a ban does.** The node drops every post of that key, also those it holds and those
  sent before the ban, and passes none on (§2.5). In a closed room it also closes the
  exchange of a neighbour that showed that key (§4.2), refuses it with `ROOM_BLOCKED` while
  the ban lasts, and the moderator that bans starts a generation without it (§3.2).
- A node holds at most 4,096 *(initial)* bans and lifts per room. Beyond that it drops the ones
  that end first.

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

- A member signs a presence when it joins, every 5 min *(initial)* while it is in the room, and
  one with `left` when it leaves. Each carries a later `issued` than the one before.
- A node holds the presence with the latest `issued` per member key. It does not give out one
  whose `expires` has passed, and remembers its `issued` until 1 h *(initial)* after that
  `expires`, so that an older one cannot be shown again: the rule of the on-air notice
  ([`pubsub.md`](pubsub.md) §4.2).
- The presences a node holds are its list of the members who are in the room. It holds at most
  4,096 *(initial)* per room, and drops the oldest beyond that.
- A presence of a banned key, and in a closed or a `moderated` room of a key that is not
  admitted, is dropped and not passed on. A moderated room so lists only those who may post.
- `gen` is 0 outside a closed room. A node ignores it there: the room info that turned the
  room may not have reached every node yet.
- `nick` is a claim. Two members can choose the same one (§6).

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

- A post in a public room carries `text` and no `sealed`, `gen` or `key_id`; in a private
  room `sealed` only; in a closed room `sealed`, `gen` and `key_id`.
- A post of the form of a private room in a closed one, or of a closed room in a private one,
  is dropped without fault and not passed on: the room info that turned the room may not have
  reached every node yet. A post that fits neither form (`text` in a room with a secret,
  `sealed` in a public room, both `text` and `sealed` or neither, `gen` without `key_id`,
  `key_id` without `gen`, `gen` or `key_id` in a public room), a `sealed` shorter than 12
  bytes, and a `text` that is empty or above 4,096 bytes fail the content check.
- **Text longer than that** is several posts, as in [`chat.md`](chat.md) §2.
- `sent` orders the posts for display. It is the author's claim, and the node checks only that
  it is not more than 1 h ahead (§2), and not older than the history (§4.3).
- A post is a duplicate of one the node holds or remembers when it has the same post id.
  Dropping it is normal: a post arrives over several neighbours.
- **Who may post.** A node drops a post, without fault, and does not pass it on, when its
  author is banned (§2.3); when the room is `moderated` or `closed` and the author is not
  admitted (§2.2); in a closed room, when its generation is behind (§3.2); and when it is over
  the rate of its author (§4.4). The neighbour that passed it on may not have known; the
  item still had a valid signature.
- A node that cannot open `sealed`, because it lacks the key of that generation, keeps and
  passes on the post all the same, once it holds a `KeyDrop` of it (§3.2). It reads it once it
  has the key.
- A `reply_to` that names no post the node holds is shown as an answer to a post it does not
  have.

---

## 3. Sealed posts

The transport encrypts every exchange (T2 of [`Protocol.md`](../Protocol.md) §3), and only
members are neighbours of a room. Sealing is for what that does not cover: a node that holds
the secret but was removed, and posts kept on disk.

```
sealed = nonce(12) ‖ ChaCha20(k_post, nonce, serialised RoomText)      RFC 8439 cipher, counter from 0
```

- `nonce` is random for every post.
- There is no authentication tag. The signature of the author fixes every byte of `sealed`,
  and a node checks it without the key, as a node of a private channel verifies a block it
  cannot read ([`live.md`](live.md) §10.2).
- A post whose `sealed` opens to bytes that do not parse as a `RoomText`, or to a `text` that
  breaks §2.5, is the fault of its author, not of the neighbour: the node keeps it out of
  view and passes it on as it is.

### 3.1 Private room

`k_post` is `k_data` (§1.2), for every post, for as long as the room is not closed.

- Everyone with the link reads every post, also one written before it joined, while members
  hold it (§4.3).
- **A private room cannot take a member's reading away**, as a private channel cannot
  ([`live.md`](live.md) §10.5). A ban drops the member's posts and nothing else: the member
  keeps the secret, joins under a new member key and reads. A room that has to remove
  members is closed.

### 3.2 Closed room: generations

The posts of a closed room are sealed with a key that a moderator draws and gives to each
admitted member separately. A new key is a new **generation**.

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

- The checks of [`Protocol.md`](../Protocol.md) §6.4 apply to a member key as to a node key.
  A moderator wraps for no member key whose y is 1 or −1. A member that gets an all-zero
  X25519 output, for example from an `e` of small order, treats its wrap as one whose tag
  fails (below).
- `k` enters `key_id` and `k_post`, so `g` alone opens nothing: a wrap that leaks to someone
  without the secret is of no use.
- The nonce of a wrap is zero. That is safe because `kw` is new for every `e` and member,
  and a moderator MUST draw `e` anew for every `KeyDrop`.
- A member that finds its member ID in `wraps` opens its wrap with its member key, and checks
  that `key_id` follows from `g`. A wrap whose tag fails, or whose `g` gives another `key_id`,
  is the moderator's fault: the node ignores that `KeyDrop` for itself, passes it on, and MAY
  tell its user.
- The members of `wraps` are the admitted members that the moderator knows, moderators and
  itself included. A `KeyDrop` that holds a member ID twice, or whose `wraps` is not a
  multiple of 64 bytes, fails its content check. 1,032 is the admits of 1,024 keys and 8
  moderators.
- A node does not check whom a moderator wrapped for. It cannot: it can open only its own
  wrap. A moderator decides who reads.

**Which generation is current**

- The **current generation** is the highest `gen` of a valid `KeyDrop` the node holds, and of
  two with that `gen` and another `key_id` the one with the lower `key_id`, compared as a byte
  string. A `KeyDrop` of a key that is no longer in `mods`, or that is banned, does not
  count: the node works out the current generation again without it. A `KeyDrop` of the room
  key always counts. A moderator that jumps `gen` far ahead, by mistake or on purpose, so
  holds the room only until the owner drops it. Two moderators can start a generation at once. Each node then takes the same one,
  and posts of the other stay readable to those that hold its key. A post under the other
  `key_id` of the current `gen` is not behind: it is kept and passed on like any other, once
  the node holds a `KeyDrop` of that `key_id` (below).
- Several `KeyDrop`s can carry one generation: one that starts it, and later ones that add
  wraps for members admitted since. Each has its own `e`. A node holds every `KeyDrop` of a
  generation it holds a post of, and of the current one, at most 64 *(initial)* per room, and
  drops those of the oldest generations first. A `KeyDrop` of a generation below the current
  one, of which the node holds no post, is dropped without fault and not passed on: a later
  wrap of an earlier generation reaches the members that hold its posts.
- A generation **began** at the earliest `issued` of its `KeyDrop`s that count and that the
  node holds.
- **A member posts under the current generation**, and only when it holds its key. A member
  that knows a current generation whose key it does not hold posts nothing, and waits for a
  wrap (below).
- **A post of an older generation** is dropped, without fault, when its `sent` is more than
  2 min *(initial)* after the `issued` of the earliest `KeyDrop` of the current generation the
  node holds. `sent` is the author's claim, so this rule only ends the late posts of honest
  members. A removed member is stopped by its ban, or by the admit it no longer has (§2.5),
  whatever `sent` it writes.
- **A post ahead of the node.** A post of the current `gen` or a higher one, whose `gen` and
  `key_id` no `KeyDrop` carries that counts and that the node holds, is kept for 60 s
  *(initial)* without being passed on, at most 64 *(initial)* per room, the oldest dropped
  first. The node checks it again when a `KeyDrop` arrives: its `KeyDrop` may still be on the
  way. After 60 s it is dropped without fault.

**Starting a generation.** A moderator, or the owner with the room key, sends a `KeyDrop` with
`gen` one above the current, a new `g`, and wraps for every admitted member it knows that is
not banned:

| When | |
|---|---|
| It bans a member of a closed room | MUST, at once, without that member |
| The owner removes a moderator, or turns the room closed (§2.1) | MUST, at once |
| 24 h *(initial)* after the current generation began, when a post of that `gen` was made | SHOULD |
| The room was made | the owner MUST, with `gen` 1, before the first post |

A generation needs a moderator or the owner online. A room whose moderators are all away goes
on under the generation it has.

**Wraps for members that lack the key.** A moderator that holds the key of the current
generation and learns a presence of an admitted member whose `gen` is below the current one,
when no `KeyDrop` it holds of the current generation has a wrap for that member, SHOULD send
a `KeyDrop` of the same `gen`, `key_id` and `g` with a wrap for that member, and for every
other admitted member it knows that still lacks one. A moderator MAY do the same when it
admits a member, so that the key waits for it in the room. This is how a member that was
admitted, or offline when the generation began, gets the key. A moderator whose current
generation already has 32 *(initial)* `KeyDrop`s that count, of any `key_id` of that `gen`,
starts a new generation instead. A moderator MAY
add wraps of earlier generations it still holds in the same way, as `KeyDrop`s of those
generations: that gives a new member the history (§4.3). Without them a new member reads only
what is posted after it joined.

**Keys are forgotten.** A node deletes the key of a generation once it holds no post under it
and a later generation began at least 1 h *(initial)* ago, and at the latest 48 h *(initial)*
after a later generation began. Someone who later takes a member's key, or its disk, reads at
most what is posted in the generation it is in and the two days before.

---

## 4. Between two members

Two members that exchange a room are **neighbours**. One of them opened the **room exchange**;
after that both sides are equal.

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

`Peer` and `Signature` are the types of [`Protocol.md`](../Protocol.md) §5.4 and §10.7.

### 4.1 The room exchange

- A member opens an exchange and sends `RoomJoin`. The peer answers with `RoomState`. The
  first answer holds the room info if its `seq` is at least `info_seq`, so that one of the same
  `seq` can win by the tie-break of §2.1, every admit, ban,
  presence and `KeyDrop` the node holds and gives out, its history (§4.3), and peers if asked.
- A peer that does not take the joiner answers with `refusal` set and closes. With
  `ROOM_FULL` it SHOULD still fill `peers`. A peer that is not in the room answers
  `ROOM_LEFT`.
- **Private and closed rooms: the secret first.** The proofs are those of a private channel
  ([`live.md`](live.md) §10.3), with other strings:

```
RoomJoin.proof   = H("ed3k v1 room join",   k ‖ joiner's node ID ‖ peer's node ID)[0..16)
RoomState.proof  = H("ed3k v1 room answer", k ‖ joiner's node ID ‖ peer's node ID)[0..16)
```

  A peer answers a `RoomJoin` without a valid proof with `refusal = ROOM_LEFT` and nothing
  else. Every answer to a valid proof carries `proof`, whatever its `refusal`, and a joiner
  closes the exchange when the first `RoomState` has no valid proof. These are the rules of
  [`live.md`](live.md) §10.3, with `RoomJoin` and `RoomState` in place of `LiveJoin` and
  `LiveState`.
- **After the first answer both sides send `RoomState`** whenever they have verified an item
  they keep (§4.4). They send it to every neighbour that has not shown them that item, and
  never back to the one it came from. Nothing else is sent on the exchange.
- The exchange stays open, and the connection is kept alive as for a file stream
  ([`Protocol.md`](../Protocol.md) §4.1). A neighbour is gone when the exchange or the
  connection closes. Either side leaves by closing the exchange.
- Two nodes keep at most one room exchange per room, by the rule of
  [`pubsub.md`](pubsub.md) §2.1 for topic exchanges.
- `peers` MUST be peers the sender has itself had as neighbours of this room within the last
  hour *(initial)*, and in a closed room peers whose admit it checked (§4.2).
- **Limits.** `RoomJoin` names at most 2 relays per family, at addresses acceptable in the
  network ([`Protocol.md`](../Protocol.md) §8.2 (7)). A `RoomState` is one frame and at most
  1 MiB ([`Protocol.md`](../Protocol.md) §5.2); a first answer that is larger is split over
  several (§4.3). A message above these limits is malformed,
  and the node closes the exchange.
- **Order of checks.** A node checks a `RoomJoin` in this order: `Error{UNSUPPORTED}` without
  the capability; `Error{MALFORMED}` for a `room` that is not 16 bytes or a `proof` that is
  not empty or 16 bytes, or relays over the limits above; `ROOM_LEFT` for a room it is not in or a proof that fails; then
  `ROOM_BLOCKED` for a node it refuses (§4.4); in a closed room the admit (§4.2); then
  `ROOM_FULL`.

### 4.2 Closed room: admitted members only

A node of a closed room shows its member key only to a node that showed a valid admit first,
as a pass holder does ([`subscription.md`](subscription.md) §6.3). The peer's comes first,
the joiner's second.

```
RoomProof.sig = Ed25519 signature by the member key over
                "ed3k v1 room proof" ‖ room(16) ‖ sender's node ID(16) ‖ receiver's node ID(16) ‖ minute(8)
```

1. The joiner sends `RoomJoin` with its secret proof.
2. The peer's first `RoomState` carries `proof`, its `member` and `member_proof`, and in
   `items` the room info and its own admit (none for a moderator), and nothing else of the
   room: no peers, no history and no `refusal` other than `ROOM_LEFT`, or `ROOM_BLOCKED` for
   a node it refuses (§4.1), which carries `proof` and nothing else.
3. The joiner checks the room info (valid under the room key, `closed` set), the peer's admit
   or its place in `mods`, that the key is not banned by what the joiner holds, and the
   proof. It closes the exchange if any of this fails. Else it sends one `RoomState` with its
   own `member`, `member_proof` and, in `items`, its admit if it holds one, and nothing else.
4. The peer checks the same, with the admit shown or one it holds for that key. It answers
   `ROOM_MEMBER`, or `ROOM_BLOCKED` for a banned key, and closes; or it sends its first
   answer as §4.1 says.

- **A newcomer** needs only to send its member key to a moderator (§1.1). The moderator's
  admit reaches the room as an item; a peer that holds it takes the newcomer at step 4, and
  its first answer gives the newcomer the admit.

- Both node IDs of the proof are those of the authenticated connection
  ([`Protocol.md`](../Protocol.md) §6.2). A proof is of no use on another connection, and an
  admit without the private member key is of no use at all.
- `minute` is the sender's clock. A receiver refuses a proof more than 10 min *(initial)* from
  its own.
- A node passes on an admit it was shown at a join as an item, so that the room learns of a
  member admitted since.
- **While the exchange lasts.** A node closes the exchange of a neighbour whose key it learns
  is banned, whose admit is over, or which is no longer a moderator and has no admit. It
  answers a `RoomJoin` of that node ID with `ROOM_BLOCKED` while a ban lasts.
- A node that holds the secret but no admit learns from a peer at most one member key and
  that member's admit. It learns no post, presence, peer or key.
- A node that holds no admitted member key, or whose own admit is over, is not in the room:
  it answers `ROOM_LEFT` and stores no `SUB` record (§5).
- A joiner that holds a room info with `closed` and gets a first answer without the steps
  above takes the room info of that answer, if it carries one. When the room is still
  closed, it sends the peer its own room info alone and closes the exchange.

### 4.3 History

- A node keeps the posts of a room of the last 24 h *(initial)* by their `sent`, at most 1,000
  *(initial)*, the newest. It keeps them as they arrived, `sealed` included, and drops a post
  whose `sent` is older than that. It remembers the post ids of what it dropped for being old
  for 1 h *(initial)* longer.
- **The first answer carries the history:** every post the node holds whose `sent` is at least
  `since` minus 1 h *(initial)*, oldest first, after the other items. The hour is the clock
  tolerance of §2: an author's clock may be that far off the joiner's. A first answer that
  does not fit one frame, with its other items or its history, goes in several `RoomState`s,
  each at most 1 MiB; the last of them sets `synced`.
  Once `synced` has arrived, the joiner sends the peer the items other than posts that the
  first answer did not show, and the posts of the same window, `sent` at least `since` minus
  1 h, that it did not show. This is its **catch-up**, in as many `RoomState`s as it needs,
  the last with `synced`; an empty `RoomState` with `synced` when it has nothing to send.
  From then on both sides only push (§4.1).
- `since` is the joiner's own clock, not a `sent`: the last time it had a neighbour of the
  room, or 0 when it holds no post of the room, also after a restart that kept none. A joiner that comes back so gets what it missed and
  the hour before, whatever the clocks of the authors, and sends back only that hour, not its
  whole history. A node with `since` 0 gets the whole history and sends no post.
- A post of the history passes the checks of §2.5 and §4.4 like any other, apart from the rate.
- Members hold the history, and only while they are in the room. A room whose members all
  leave keeps nothing. Nobody else stores a post: there is no store for a member that is
  offline, as there is none for chat ([`chat.md`](chat.md) §2).

### 4.4 Receiving an item

A node checks an item before it does anything else with it: size, signature and content
first, then time, then the rules of §2 and §3. A content failure is a fault whatever those
rules would say of the item; the form of a closed or a private room (§2.5) is a rule. Then:

| Item | The node |
|---|---|
| valid, and new to it (a newer room info, admit, ban, presence or `KeyDrop`, or a post id it does not know) | keeps it and passes it on (§4.1) |
| valid, not new | drops it. This is normal: an item arrives over several neighbours |
| valid, dropped by a rule of §2 or §3 (a banned author, an author not admitted, a generation behind, a post of the other form of a room with a secret, a signer that is not a moderator, a limit per room or per neighbour) | drops it and does not pass it on. It is no fault of the neighbour, which may not have known |
| fails its signature or content check | drops it, closes the exchange and does not take that node as a neighbour of any room for 24 h *(initial)* |
| fails a time check (`issued` or `sent` ahead, `expires` passed, a post older than the history) | drops it. Clocks differ, so this is no fault of the neighbour |

- **Rate of an author.** A node passes on at most 20 *(initial)* posts per author key and
  minute, and drops the rest, without fault: the value of [`chat.md`](chat.md) §2. It passes
  on at most 1 presence per member key and minute *(initial)*, and at most 60 admits, bans and
  `KeyDrop`s per signer and minute *(initial)*. A neighbour is at fault only when, in any 10 s,
  it passes on more than 10 times a limit's share of 10 s, rounded up: more than 40 posts of
  one author key, 10 presences of one member key, or 100 admits, bans and `KeyDrop`s of one
  signer. An item sent again counts again. The node then closes the exchange.
- **Rate of a neighbour.** Member keys cost nothing, so the rate of an author does not bound a
  node that signs with many. A node takes at most 600 *(initial)* items per neighbour and
  minute, the first answer and the joiner's catch-up (§4.3) not counted, and drops the rest
  without fault. The rule of fault above applies to this limit too: more than 1,000 items in
  10 s.
- **First answer and catch-up.** No rate of this section applies to the items of a first
  answer or of a joiner's catch-up (§4.3), up to as many of each kind as a node holds (§2.2
  to §4.3): 1,024 admits of one moderator come in one answer. Further items are rated. A
  node passes on what it took from them within the rates of an author and of a signer above;
  the rest it keeps and does not pass on. The rate of a neighbour bounds what a node takes,
  not what it passes on. A room info has no rate and is always passed on.
- **Waiting for the room info.** A node that holds no room info, or holds one with a lower
  `seq` than a neighbour's `RoomJoin.info_seq` named, keeps an admit, ban or `KeyDrop` whose
  signer is not a moderator for 60 s *(initial)* without passing it on, and checks it again
  when the room info arrives. It keeps at most 64 *(initial)* such items per room, and drops
  the oldest beyond that. In any other case an item whose signer is not a moderator is
  dropped at once, without fault. The first answer brings the room info before the other
  items (§4.1).

### 4.5 Neighbours

- A node aims at 3 neighbours per room and keeps between 2 and 4 *(initial)*. When it falls
  below 2 it looks for more (§5). A node that cannot reach 2 stays in the room with what it
  has.
- Neighbours are subject to the diversity limits of a bucket: at most 1 per host, 2 per site
  and 4 per block ([`ip-diversity.md`](ip-diversity.md) §1 and §2).
- **A node is in at most 16 rooms** *(initial)*. The limit is its own: the 16 topics of
  [`pubsub.md`](pubsub.md) §2.3 do not count towards it.

---

## 5. Finding members

A member publishes a record of kind `SUB` ([`pubsub.md`](pubsub.md) §5) under the Kad target
of the room: `room` for a public room, `target` of the current hour for a private or closed
one (§1.2).

- It is the record of [`pubsub.md`](pubsub.md) §5 in every way: a source record bound to the
  connection, not signed, with a lifetime of 60 min *(initial)*, stored again every 30 min
  while the node is in the room, and with the limits per target of that section.
- **A private or closed room stores under the target of the hour.** A member stores under the
  new target at once when the hour changes. In the first 10 min *(initial)* of an hour a member
  that finds nobody also looks under the target of the hour before, and in the last 10 min
  under that of the hour after, as [`live.md`](live.md) §10.4 does.
- **Joining** is as in [`pubsub.md`](pubsub.md) §5: the member looks up `SUB` records under the
  target, sends `RoomJoin` to peers from the answer and from `RoomState.peers` until it has its
  neighbours, and publishes its own record. Failed dials count under the limits of
  [`ip-diversity.md`](ip-diversity.md) §2.5, where a room counts as a file.
- **A member under its anonymous key** ([`hops.md`](hops.md) §1) stores an `ANON` record under
  the target in place of a `SUB` record, as [`pubsub.md`](pubsub.md) §5 says, and its member key
  MUST NOT be used under its node key as well. A `Peer` that names it carries its node ID alone
  ([`hops.md`](hops.md) §3.3). Under that key a node that takes part in no room answers
  `ROOM_LEFT`, not `Error{UNSUPPORTED}`, as a hidden node answers for topics and streams.
- A peer found in the local network ([`lan-discovery.md`](lan-discovery.md) §3) MAY be a
  neighbour. Its address is not passed on in `peers`.

---

## 6. Showing a room *(informative)*

- A client shows a post as plain text. It follows no link and fetches nothing that a post
  names until its user asks: a post that names a file only its author shares would tell the
  author who reads the room.
- A client shows a member by its nick together with something of its member ID, so that two
  members with one nick are told apart. It marks the posts of moderators.
- Posts are shown in the order of `sent`. A post whose `sent` is far from the time it arrived
  MAY be marked.
- A client MAY hide the posts of a member ID that a filter list it follows names as `AUTHOR`
  ([`filter-list.md`](filter-list.md)). A list does not stop it from passing those posts on,
  nor from taking that member as a neighbour ([`filter-list.md`](filter-list.md) §3).
- What the user can mute, for themselves alone, is local. A ban is what a moderator does for
  the room.
- One member writing to another alone uses chat ([`chat.md`](chat.md)), which needs the node
  ID: a room shows only member keys. A member that wants to be reached privately posts a
  friend link ([`friend.md`](friend.md)).

---

## 7. What an attacker can do

| Attack | What limits it |
|---|---|
| **Post as someone else** | Not possible without that member key. Every hop verifies |
| **Change a post on the way** | The signature covers `sealed` and `text`; every hop verifies |
| **Show an old post again** | Its post id is known, or it is older than the history |
| **Flood a public or private room** with new member keys | Keys cost nothing. The rate per author bounds each key and the rate per neighbour each node (§4.4), and moderators ban; a room that is flooded turns `moderated`, which drops the posts and presences of keys not admitted |
| **A banned member of a private room reads on** | Possible: it holds the secret. That is what a closed room is for (§3.1) |
| **A removed member of a closed room reads on** | Not what is posted from the next generation. It reads what it held before. A member that still passes posts on to it leaks them, which no key can stop |
| **A removed member posts under its old key** | Dropped: it is banned or no longer admitted (§2.5). Holding the key does not help it |
| **A member that passes nothing on** | Its neighbours have others; the mesh keeps 2 to 4 per node. The diversity limits raise the cost of surrounding a member |
| **Send bad items** | One per attacker node; that node is then refused as a neighbour of every room for 24 h (§4.4) |
| **Fill the room's Kad target** with records of nodes that are not members | Wasted dials, limited as for false sources ([`ip-diversity.md`](ip-diversity.md) §2.5) |
| **A moderator turns against the room** | It can admit, ban other members and hand keys to whom it wants, until the owner drops it from `mods`; its admits, bans and `KeyDrop`s then stop counting (§2.1, §3.2). It cannot ban the owner's other moderators. The owner key can change everything |
| **The owner key is taken** | Nothing helps. There is no successor key (§10) |
| **Two moderators start a generation at once** | Every node takes the same one (§3.2); nothing is lost |

---

## 8. What it shows

| To whom | What |
|---|---|
| A neighbour | the node ID, the address, and the member key of this room (in a closed room after both showed an admit) |
| Every member | the member keys, nicks and posts of the room; in a closed room who admitted whom |
| A storing node of the room's target | the addresses of the members that publish there. For a public room anyone with the link computes the target; for a private or closed one the target changes every hour and cannot be tied to a room |
| A holder of the secret of a closed room without an admit | one member key and its admit (§4.2) |
| Anyone with a filter list | that a member ID of a room is the author it names, as for any key |
| Someone who later takes a member key or a member's disk | the posts of its current generation and of up to two days before (§3.2); in a private room every post members still hold |

- The member key keeps rooms apart, and apart from the node: neighbours see the node ID, but a
  post reaches the rest of the room without it.
- A member that does not want its address seen by its neighbours joins under its anonymous
  key (§5).

---

## 9. Cost

| | Size |
|---|---|
| Post | about 200 B for a short line, at most 8 KiB; to each neighbour once |
| Presence | about 150 B per member per 5 min, to each neighbour once |
| `KeyDrop` | 64 B per member: 64 KiB for 1,024 members, per generation, so once a day |
| History at a join | up to 1,000 posts: about 200 KB for short ones, at most a few MiB |
| Connections | about 3 per room, held open; about 48 for 16 rooms |
| `SUB` record | one store per 30 min, on about 8 nodes |

A room of 1,000 members that are all present sends each member about 0.5 KB/s of presences
per neighbour. A room's size is not limited by the protocol, only by the admits of 1,024 keys
of a closed room (§2.2).

---

## 10. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **A topic of [`pubsub.md`](pubsub.md)** with a type for rooms | no messages of its own | a topic holds one item, the newest; a room holds many, and history |
| **Chat to every member**, each post sent by its author to all | nothing new on the wire | the author connects to every member; it needs their node IDs |
| **One node hosts the room** and passes posts on | order and history in one place | a server, which ed3k leaves to index nodes (`Protocol.md` §11); it reads every post |
| **Posts in Kad**, as signed records | offline members read them later | a record lifetime and a record size; every post a store on 8 nodes; a store anyone can fill |
| **The node key as the author** | a friend is recognised in a room | every post tied to the node and to everything it shares, and rooms tied to each other |
| **The mesh as the only gate**, no sealing | no keys to hand out | a removed member that still holds the secret reads on whenever it slips in; history on disk stays readable |
| **Messaging Layer Security** (RFC 9420) | forward secrecy and recovery after a compromise, for groups of any size | a ratchet tree whose changes every member must take in the same order, which needs one service to order them |
| **A sender key per member**, as in Signal groups | a removed member reads nothing new; no moderator needed for keys | every member hands its key to every other, over channels of two, at every change: n² messages |
| **Sealing each post to every member** | no shared key at all | a post grows with the room |
| **A key per epoch in a hash chain**, as for paid scopes ([`subscription.md`](subscription.md) §4) | no moderator online for each epoch | a chain cannot leave out a member: whoever holds a key holds every earlier one of its season |
| **Rate limits by proof of work** on new member keys | flooding costs something | honest low-power members pay too; moderation is cheaper |

---

## 11. Open points

- Every number is *(initial)* and none is measured.
- **Succession.** A room has no successor for a lost or stolen room key, as a channel has none
  ([`live.md`](live.md) §12).
- **Listing public rooms**, for which the options of [`live.md`](live.md) §11 apply as well.
- **Paid rooms**, a scope of a fourth kind in [`subscription.md`](subscription.md) §2.
- Editing or taking back one's own post, and a moderator removing one post rather than an
  author.
- A closed room larger than 1,024 members.
- A generation needs a moderator online (§3.2). Whether any member may start one in a room
  whose moderators are away for long.
- Recovery after a member key is taken: the thief reads until the next generation without that
  member, and a moderator cannot tell.
- Whether a member that shares a member key between its own devices should be allowed, as a
  pass allows `devices` ([`subscription.md`](subscription.md) §5.3).
