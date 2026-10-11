# Extension: browsing a peer's shared files (`browse`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §13.1. Section numbers (§) refer to that document.
**Capability:** bit 3, `BROWSE`. Message types 14 and 15.

A node that sets `BROWSE` lets other nodes list the files it shares, by directory. It replaces
the four shared-files and shared-directories opcodes of eD2K.

## 1. Messages

```protobuf
message Browse {
  string dir = 1;             // empty = top level
  uint32 offset = 2;          // skip this many entries
}

message BrowseAnswer {
  repeated string   dirs = 1;
  repeated FileInfo files = 2;
  bool   more = 3;            // further entries exist; ask again with `offset`
}
```

`FileInfo` is the type of §5.4.

## 2. Rules

- `Browse` is a request/answer exchange (§5.2).
- A node sends `Browse` only to a peer that set `BROWSE` in its `Hello`. A node without the
  capability answers `Error{UNSUPPORTED}`.
- `dir` names one directory. The empty string asks for the top level. The answer lists the
  directories directly below it in `dirs` and the files directly in it in `files`.
- Directory names are those the answering node chooses to show. They need not be paths of its
  file system.
- Whom a node answers is local policy: everyone, friends only, or nobody. A refused request is
  answered with `Error{DENIED}`.
- The answer is one frame and is compressed like any other (§5.3). The frame limit of 1 MiB
  (§5.2) applies.

**Paging**

- The entries of a directory have one order: its directories first, then its files, each group
  sorted by the bytes of the name in UTF-8.
- An answer holds as many entries as the answering node chooses, at most 1,000 *(initial)*,
  starting at entry `offset` of that order. It sets `more` when entries remain.
- The requester asks again with `offset` raised by the number of entries it received (`dirs`
  plus `files`), until an answer comes without `more`.
- Each page is an exchange of its own. The node keeps no state between pages. If the directory
  changes in between, entries can be missed or seen twice; the requester merges directories by
  name and files by root.

## 3. Privacy

A browse list shows everything a node shares to whoever asks, and ties it to the node ID.
An implementation SHOULD leave the capability off by default, or limit it to friends.

## 4. Open points

- Whether an answer should say that a listing is partial by the node's choice.
