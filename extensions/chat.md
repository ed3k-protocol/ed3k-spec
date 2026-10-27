# Extension: chat (`chat`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §13.1. Section numbers (§) refer to that document.
**Capability:** bit 4, `CHAT`. Message types 16 and 18.

A node that sets `CHAT` accepts text messages from other nodes. A message goes to one node;
rooms with many members are [`room.md`](room.md).

## 1. Messages

```protobuf
message Chat {
  string text = 1;            // at most 4,096 bytes in UTF-8
  fixed64 id = 2;             // random per message, the same again when it is repeated: a replay id, not a request id (§5.2)
}

message ChatAck { }
```

## 2. Rules

- `Chat` is a request/answer exchange (§5.2), one message per exchange. The receiver answers
  `ChatAck` once it has taken the message.
- A node sends `Chat` only to a peer that set `CHAT` in its `Hello`. A node without the
  capability answers `Error{UNSUPPORTED}`.
- The sender is the node ID of the connection (T1). A message cannot be attributed to another
  node.
- `text` MUST NOT be longer than 4,096 bytes in UTF-8. A longer message is answered with
  `Error{MALFORMED}`. A sender splits a longer text into several messages.
- Whom a node accepts messages from is local policy. It answers a refused message with
  `Error{DENIED}`, or with `ChatAck` if it does not want to reveal that the sender is blocked.
- There are no captchas. The sender is an authenticated key, which the receiver can block, and
  the connection budget of §4.6 limits how fast a node can open connections to send.
- Delivery needs a connection to the receiver. There is no storage for nodes that are offline.
  How a node finds a peer it knows after that peer's address changed is in
  [`friend.md`](friend.md).
- **Rate limit.** A receiver MAY limit the messages it takes from one sender, counted per node
  ID and per host address (IPv4 /32, IPv6 /64). A message over the limit is answered with
  `Error{BUSY}` and a `retry_after` in seconds. The limit itself is local policy; 20 messages
  per minute is a reasonable value. A sender that gets `BUSY` waits that long before it sends
  again, with the same `id`.
- A node checks in this order: `UNSUPPORTED`, `MALFORMED` (the size of `text`), its policy
  (`DENIED` or a silent `ChatAck`), a repeated (sender, `id`), then the rate limit (`BUSY`).
- Every `Error{BUSY}` of this document carries a `retry_after` of at least 1 s, in whole seconds
  rounded up.

**Delivery**

- The transport delivers reliably within a connection (T4). With QUIC a message that was sent
  either arrives or the connection fails. What the transport cannot tell the sender is whether
  the connection failed before or after the message arrived, and whether the receiving node
  took it. `ChatAck` answers both, and it does so on any binding.
- `ChatAck` means the receiving node has the message. It does not mean a person has read it.
- A sender that gets no answer because the connection was lost MAY send the message again with
  the same `id`, at most 3 *(initial)* times and only within 10 min of the first send that got
  no answer: after that the receiver may have forgotten the `id`. A send answered `BUSY` does
  not count; it was not taken. The receiver answers `ChatAck` to a repeated (sender, `id`) and shows the
  message once. It remembers the `id`s of a sender for at least 10 min *(initial)* and at
  most 1,024 *(initial)* of them, the oldest forgotten first. Only the `id`s of messages it
  took are remembered: one that was answered `BUSY` or `DENIED` is not, so the retry with the
  same `id` is shown when it is taken. A sender with 1,024 remembered `id`s younger than
  10 min is answered `Error{BUSY}` with a `retry_after` of the time until the oldest of them
  is 10 min old; the message is not taken.
- A receiver remembers the `id`s of at most 256 *(initial)* senders. A sender's entry goes
  when its newest `id` is 10 min old. A message of a further sender is answered `Error{BUSY}`
  with a `retry_after` of the time until the first entry can go, and not taken, so that no
  `id` is forgotten before its 10 min. Of the 256, at most 16 *(initial)* are of one host address (IPv4 /32, IPv6 /64); a
  further sender of that host is answered `Error{BUSY}` with a `retry_after` of the time until
  the first entry of that host can go. A sender counts at the host address
  of the message that made its entry, until its entry is gone.

## 3. Open points

- Blocking a key does not stop a sender who makes a new one; the address budget of §4.6 is what
  limits that.
