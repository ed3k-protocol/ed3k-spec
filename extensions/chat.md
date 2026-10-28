# Extension: chat (`chat`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §13.1. § = that document.
**Capability:** bit 4, `CHAT`. Message types 16 and 18.

`CHAT` = node accepts text messages from other nodes. A message goes to one node; rooms with many
members: [`room.md`](room.md).

## 1. Messages

```protobuf
message Chat {
  string text = 1;            // at most 4,096 bytes in UTF-8
  fixed64 id = 2;             // random per message, the same again when it is repeated: a replay id, not a request id (§5.2)
}

message ChatAck { }
```

## 2. Rules

- `Chat` is a request/answer exchange (§5.2), one message each; receiver answers `ChatAck` once
  it has taken the message.
- Sent only to a peer whose `Hello` set `CHAT`; a node without it answers `Error{UNSUPPORTED}`.
- Sender = node ID of the connection (T1); a message cannot be attributed to another node.
- `text` MUST NOT be longer than 4,096 bytes in UTF-8. Longer → `Error{MALFORMED}`; sender splits
  longer text into several messages.
- Whom to accept messages from is local policy. Refused message → `Error{DENIED}`, or `ChatAck`
  if the node does not want to reveal that the sender is blocked.
- No captchas: the sender is an authenticated key the receiver can block, and the connection
  budget of §4.6 limits how fast a node can open connections to send.
- Delivery needs a connection to the receiver; no storage for offline nodes. Finding a known
  peer after its address changed: [`friend.md`](friend.md).
- **Rate limit.** A receiver MAY limit messages taken from one sender, counted per node ID and
  per host address (IPv4 /32, IPv6 /64). Over the limit → `Error{BUSY}` with `retry_after` in
  seconds. Limit is local policy; 20 messages per minute is reasonable. A sender getting
  `BUSY` waits that long before sending again, with the same `id`.
- Check order: `UNSUPPORTED`, `MALFORMED` (size of `text`), its policy (`DENIED` or silent
  `ChatAck`), a repeated (sender, `id`), then rate limit (`BUSY`).
- Every `Error{BUSY}` here carries a `retry_after` of at least 1 s, whole seconds
  rounded up.

**Delivery**

- Transport delivers reliably within a connection (T4): with QUIC a sent message either arrives
  or the connection fails. It cannot tell the sender whether the connection failed before or
  after arrival, nor whether the receiving node took it. `ChatAck` answers both, on any binding.
- `ChatAck` = receiving node has the message, not that a person has read it.
- A sender getting no answer because the connection was lost MAY resend with the same `id`, at
  most 3 *(initial)* times and only within 10 min of the first unanswered send: after that the
  receiver may have forgotten the `id`. A send answered `BUSY` does not count; it was not taken.
  Receiver answers `ChatAck` to a repeated (sender, `id`) and shows the message once. It
  remembers a sender's `id`s for at least 10 min *(initial)*, at most 1,024 *(initial)* of
  them, oldest forgotten first. Only `id`s of taken messages are remembered: one answered `BUSY`
  or `DENIED` is not, so the retry with the same `id` is shown when taken. A sender with 1,024
  remembered `id`s younger than 10 min → `Error{BUSY}` with `retry_after` = time until the
  oldest is 10 min old; message not taken.
- A receiver remembers `id`s of at most 256 *(initial)* senders. A sender's entry goes when its
  newest `id` is 10 min old. A further sender's message → `Error{BUSY}` with `retry_after` =
  time until the first entry can go, not taken, so no `id` is forgotten before its 10 min. Of
  the 256, at most 16 *(initial)* are of one host address (IPv4 /32, IPv6 /64); a further
  sender of that host → `Error{BUSY}` with `retry_after` = time until that host's first entry
  can go. A sender counts at the host address of the message that made its entry, until the
  entry is gone.

## 3. Open points

- Blocking a key does not stop a sender who makes a new one; the address budget of §4.6 limits
  that.
