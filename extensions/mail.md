# Extension: offline messages (`mail`)

**Status:** draft 0.1 (2026-11-18). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.1, §10.3, §13.1, §13.2; [`friend.md`](friend.md) §1; [`chat.md`](chat.md). § = that document unless this one is named; "§n here" = this document.
**Capability:** none. Record kind 16 `MAIL`. One field in `Store`, two in `KadAnswer`; `mb` in `ed3k:friend` links.
**Required of every `ROUTER`:** storing and returning the record kind `MAIL` (§4 here). The rest is optional.
**Test vectors:** not yet (§9 here).

Chat ([`chat.md`](chat.md)), rooms and every request/answer message need both nodes online.
A message for an offline node is stored in Kad as **sealed records** until the recipient comes
online and fetches it.

- **Recipients share a place.** Records go to a **bucket**: the target of every recipient whose
  mailbox hash starts with the same `b` bits, changing daily. A storing node, or someone
  watching the target, sees that *someone* in the bucket got mail, not who. A recipient fetches
  the whole bucket and opens what it can.
- **Large messages are split.** One record holds ≤ 962 B of message; a message spans up to 64
  records (≈ 57 KiB). Larger payloads go into a sealed file whose root and key travel by mail
  (§3.3 here).
- **Records look alike.** Every record is exactly 1,024 bytes, sealed to a one-off X25519 key:
  a storing node can't tell part from part, message from message, sender's copy from re-store.

Idea: OpenBazaar's offline messages (pointer under a 14-bit prefix of the recipient's hash,
NaCl box, push nodes holding the data). Here the data itself is the record; no third party
holds a mailbox ([`docs/openbazaar-comparison.md`](../docs/openbazaar-comparison.md) §3).

---

## 1. Mailbox

Mailbox = an Ed25519 key whose private key the recipient holds (node key, or a key of its own
like a store key, [`market.md`](market.md) §2) and a bucket width `b`, 0 to 16.

```
box    = H("ed3k v1 mail box", key)[0..2]                                   16 bits, big-endian
bucket = box with all bits after the first b set to 0
target = H("ed3k v1 mail target", b(1) ‖ bucket(2) ‖ period(8))[0..16)     period = Unix time / 86400, rounded down, big-endian
```

- `b` = 0: one bucket for every recipient. `b` = 16: about one recipient per bucket, no hiding.
- Recipient announces `key` and `b` with its contact: `mb=<b>` in its `ed3k:friend` link (§6
  here), `MailBox` in a store profile ([`market.md`](market.md) §2.2). Unknown `b` → sender
  uses 8 *(initial)*; recipient fetches bucket 8 too while it announced another `b` within the
  last 7 days.
- **Choosing `b`.** Recipient keeps its bucket around 64 to 1,024 *(initial)* records per day:
  above → announce `b` + 1, below → `b` − 1, at most one change per 7 days. Counts known from
  its own fetches (§5 here). An old `b` still in senders' hands keeps working: recipient
  fetches every `b` it announced within 7 days.
- **Spill.** Target answering `FULL` (§4 here) → sender stores under the target of `b` + 1,
  else `b` + 2 (same `box`, so the recipient's sub-bucket). Recipient fetches those only when
  the `b` target answered with `spill` set (`Protocol.md` §10.8). `b` = 16 has no spill.

## 2. The record

```
plain  = len(2) ‖ serialised MailPart ‖ zero bytes up to 964 B          len = size of the serialised message, big-endian
k_mail = H("ed3k v1 mail", X25519(e's private key, recipient's public key) ‖ e ‖ key)     key = recipient's 32-byte Ed25519 key
sealed = e(32) ‖ nonce(12) ‖ ChaCha20-Poly1305(k_mail, nonce, plain)     RFC 8439, 16-byte tag, empty associated data; 1,024 B
```

- `e`: X25519 key made for this one record; new `e` and `nonce` per record, parts included.
  Recipient's key converted per `Protocol.md` §6.4; MUST refuse an all-zero shared secret.
- Same construction as a friend request ([`friend.md`](friend.md) §3), other domain strings,
  larger size. Serialised `MailPart` ≤ 962 B.

```protobuf
message MailPart {
  bytes    msg = 1;           // 16 B random message id, same in every part
  uint32   index = 2;         // 0 .. count − 1
  uint32   count = 3;         // 1 to 64, same in every part
  bytes    data = 4;          // this part's slice of the payload, in index order
  MailHead head = 5;          // part 0 only
}

message MailHead {
  bytes    from = 1;          // 32 B Ed25519 key of the sender
  uint64   size = 2;          // payload bytes, all parts together
  bytes    hash = 3;          // 32 B H("ed3k v1 mail payload", payload)
  uint64   sent = 4;          // Unix time in seconds
  string   type = 5;          // payload type, at most 64 B (§3 here)
  optional uint32 reply = 6;  // `b` of the sender's mailbox under `from`; absent = sender takes no mail
  repeated Signature sigs = 7;  // by `from`, over the layout below
}
```

Signed layout:

```
"ed3k v1 mail signature" ‖ to(32) ‖ msg(16) ‖ count(1) ‖ size(8) ‖ hash(32) ‖ sent(8) ‖ reply(1) ‖ len(type)(1) ‖ type
```

`to` = recipient's mailbox key; `reply` = 0xFF when absent. `to` in the signature: recipient
can't reseal a signed message to a third key and pass it off as sent there.

**Recipient**, per record:

1. Drops `sealed` not 1,024 B, failing tag, `len` above 962, `MailPart` not parsing.
2. Drops part with `msg` not 16 B, `count` 0 or above 64, `index` ≥ `count`, `head` on
   `index` ≠ 0 or missing on 0.
3. Collects parts by `msg` for up to 8 days *(initial)*; first part with a (`msg`, `index`)
   wins, later copies ignored. Part 0 from another `from` than the message's first `head` →
   message dropped.
4. All `count` parts present → payload = `data` in `index` order. Drops message when length ≠
   `size`, hash fails, `from` not 32 B, signature fails under `from`, `type` above 64 B, `sent`
   more than 8 days old or 1 h *(initial)* ahead.
5. Shows / hands on each (`from`, `msg`) once. Remembers delivered (`from`, `msg`) 9 days.

Which senders it accepts: local policy, as for chat.

## 3. Payloads

`type` names the payload. Unknown type → message dropped after the checks of §2 here (still
acknowledged, §3.2 here).

| `type` | Payload | Defined in |
|---|---|---|
| `chat` | serialised `Chat` ([`chat.md`](chat.md) §1); `id` is the chat's replay id | §3.1 here |
| `ack` | serialised `MailAck` | §3.2 here |
| `file` | serialised `MailFile` | §3.3 here |
| `market` | serialised `Market` ([`market.md`](market.md) §4) | [`market.md`](market.md) |

Further types: by the extension using them; names without `.` reserved for this document,
others written `<domain or extension>.<name>`.

### 3.1 Chat

A `Chat` that can't be sent (no connection to the friend, [`friend.md`](friend.md) §4) MAY go
by mail. Receiver shows it as chat from `from`. Same `id` by chat and mail → shown once
(`chat.md` §2, remembered 9 days for mail).

### 3.2 Acknowledgement

```protobuf
message MailAck {
  repeated bytes msgs = 1;    // 16 B ids of messages delivered (§2 here step 5); at most 64
}
```

Recipient answers every delivered message whose head has `reply`: by mail to `from` with that
`b`, collecting ids for up to 1 h *(initial)*; or not at all when it reaches the sender online
first and the app protocol acknowledges there (`ChatAck`, `MarketAck`). An ack is never acked.

### 3.3 Large payloads

Payload above 64 parts' room → sender seals it as hidden-set sealed data
([`hidden-set.md`](hidden-set.md) §2: `nonce(12) ‖ ChaCha20(key, nonce, …)`) under a fresh
32-byte key, shares the sealed file, mails its root. No tag needed: root is in a signed message.

```protobuf
message MailFile {
  bytes  root = 1;            // 32 B root of the sealed file
  uint64 size = 2;            // its size
  bytes  key = 3;             // 32 B key it is sealed with
  string type = 4;            // payload type of the opened file, as MailHead.type
  string name = 5;            // optional, at most 255 B
}
```

Recipient downloads the root as any file (sources: sender, while online), opens it, handles it
as a payload of `type`. Sender shares it until acked, or 7 days.

## 4. Storing node

```protobuf
// in enum Kind (Protocol.md §10.3):
MAIL = 16;

// in Store (Protocol.md §10.3):
repeated bytes mail = 15;     // MAIL: each exactly 1,024 B; any other length: REJECTED

// in KadAnswer (Protocol.md §10.3):
repeated bytes mail = 24;     // answer to a search of kind MAIL
fixed64 mail_epoch = 25;      // MAIL search: random, fixed while this node holds the target; 0 = none held
```

Every `ROUTER` stores, checks and returns `MAIL` (`Protocol.md` §10.3). Unsigned, like
`FRIEND`; `signer` ignored.

- **Check:** size only. `sealed` ≠ 1,024 B → `REJECTED`. Can't read it.
- **Identity:** record = its bytes. Same bytes again under a target → `STORED`, nothing changes,
  not counted twice. No replacing: records only come and expire.
- **Limits** under one target, in this order: per host 256, per site 512
  ([`ip-diversity.md`](ip-diversity.md) §2.4), `REJECTED` beyond; then 4,096 *(initial)*
  records per target, `FULL` after (`spill` set, `Protocol.md` §10.8). A `Store` of 256
  records (`Protocol.md` §10.3) = 256 KiB.
- **Lifetime:** every record of a target lives until 7 days *(initial)* after the end of the
  target's day: `(period + 8) × 86400`. All of a target expire together; nothing leaves earlier,
  except by declining (`Protocol.md` §10.3) or the node losing its store.
- **Order:** records in order of arrival at this node. `mail_epoch` new (random, ≠ 0) whenever
  the node starts holding a target with no records, so a reader sees when an order was lost.
- **Search:** of a `Query` only `offset` read. Answer: up to 256 *(initial)* records from
  `offset` on in arrival order, `more` when further ones remain; `mail_epoch` always set.
  Deviates from the paging order of `Protocol.md` §10.3, as this document may.
- Index node ([`Protocol.md`](../Protocol.md) §11): keeps `MAIL` no longer than its lifetime.

**What it learns:** a sender's connection (node ID, address), a bucket's daily volume, a
reader's address. Not who the mail is for, nor whether two records belong together.

## 5. Sending and fetching

**Sender**

1. Splits payload into parts (data ≤ 962 B less `MailPart` and, part 0, `MailHead` overhead),
   seals each (§2 here), stores all under today's bucket target at the K closest nodes
   (`Protocol.md` §10.4); `FULL` → spill (§1 here).
2. Re-stores the *same* records under the *same* target every 12 h *(initial)*, so nodes newly
   responsible get them, until acked (§3.2 here) or the target expires. Never reseals: a reseal
   is a new record the recipient would open twice.
3. SHOULD store from a connection whose key is not its mailbox key nor its usual node key: a
   leaf key used for mail only, new daily *(recommended)*. Storing node then sees an address,
   no lasting identity. Sender's identity is inside, for the recipient only.

**Recipient**

1. Fetches at start and every 30 min *(initial)* while online: every bucket target of the last
   8 days (lifetime: older targets are gone), for each `b` announced in the last 7 days (§1 here),
   spill targets where flagged. From 3 *(initial)* of the K closest nodes per target.
2. Per (node, target) remembers `mail_epoch` and records read. Same epoch → asks with `offset`
   = records read; other epoch → from 0. Merges records of all nodes by their bytes.
3. Opens each new record (§2 here); one X25519 per record. 4,096 records ≈ 4 MiB, well under a
   second of key agreement.
4. Fetching a whole bucket is the hiding: MUST NOT fetch only part of a target for being
   uninteresting, and SHOULD fetch at its fixed interval whether or not it expects mail.

## 6. Links

`ed3k:friend` ([`friend.md`](friend.md) §1) gains:

```
mb=<b, decimal 0 to 16>
```

Holder of the link may mail the node under its `key`. Link invalid when `mb` repeated, not
decimal, or above 16. Absent = node announced no mailbox; a sender MAY still try `b` = 8.

## 7. What it shows

| To whom | What |
|---|---|
| Storing node of a bucket target | sender's connection; that someone among the bucket's recipients got that many records today; readers' addresses |
| Someone knowing a recipient's key | its bucket, so when its bucket gets mail; with small `b` that says little |
| A recipient | sender's key (signed), sending time, payload |
| Someone taking the recipient's private key later | all mail still stored. No forward secrecy, as for a friend request |

- Reader's address at the storing node is the weak point: daily target → different nodes per
  day. A reader wanting more fetches through a path ([`hops.md`](hops.md)).
- Traffic analysis: a sender storing 40 parts at 12:03 and a recipient fetching at 12:30 in
  a bucket of three recipients is a hint. Wider bucket, fixed fetch interval help.

## 8. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Target per recipient** (`H(key ‖ day)`) | recipient fetches its own mail only | storing node and watchers see exactly who gets mail and how much |
| **Pointer record + data at mailbox hosts** (OpenBazaar) | small record, any payload size | someone must run always-on hosts; recipient trusts them for delivery; second protocol |
| **Signed records** | storing node rejects junk | signature reveals sender to every storing node |
| **Hint per record** so recipient opens only its own | less decryption | any hint the recipient can test needs the same X25519; one fixed per recipient links its records |
| **Records of varying size** | small messages cheap | size tells parts and senders apart |
| **Delivery through friends' nodes** (Briar mailbox) | no Kad load | needs a friend online; strangers (a buyer) have none |

## 9. Open points

- Test vectors: box, bucket, target, one sealed record, a two-part message with signature.
- Spam filling a bucket to `FULL`: costs 8 sites per target per day; spill helps one level.
  A per-sender proof of work, or a deposit, would cost honest senders too.
- Mail from senders without `reply`: one-way only.
- All numbers are first values.
