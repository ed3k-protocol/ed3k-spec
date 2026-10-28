# Extension: browsing a peer's shared files (`browse`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §13.1. § = that document.
**Capability:** bit 3, `BROWSE`. Message types 14 and 15.

`BROWSE` lets others list a node's shared files by directory; replaces eD2K's four shared-files/
shared-directories opcodes.

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
- Sent only to a peer whose `Hello` set `BROWSE`; a node without it answers `Error{UNSUPPORTED}`.
- `dir` names one directory; empty = top level. Its components MUST follow
  [`file-set.md`](file-set.md) §2.2 (no `.`, `..`, empty component or `//`) and §2.1 (at most
  32 components); a `dir` that does not → `Error{MALFORMED}`; one the node does not show →
  `Error{NOT_FOUND}`. Answer: directories directly below it in `dirs`, as one-component names
  ([`file-set.md`](file-set.md) §2.2 component rules apply), files directly in it in `files`.
  Requester lists a subdirectory via `dir` ‖ `/` ‖ name (name alone at top level).
- Directory names are what the answering node chooses to show, not necessarily its FS paths.
- Whom to answer is local policy: everyone, friends only, or nobody. Nobody = capability off:
  node clears `BROWSE` and answers `Error{UNSUPPORTED}` (§2), also on a connection whose
  `Hello` still showed `BROWSE`. Friends only: refused request → `Error{DENIED}`; node MAY set
  `BROWSE` only in the `Hello` of a connection to a friend. A change reaches a connected peer
  as §2 says.
- Check order: `UNSUPPORTED` (no capability, or nobody answered), then `MALFORMED` (`dir`), then
  `DENIED` (friends only), then `NOT_FOUND`.
- Answer is one frame, compressed as usual (§5.3); 1 MiB frame limit (§5.2) applies.

**Paging**

- Entry order: directories first, then files, each group sorted by the bytes of the name in UTF-8.
- An answer holds as many entries as the answering node chooses, at most 1,000 *(initial)*,
  starting at entry `offset`; `more` set when entries remain. An answer with >1,000 entries ends
  the listing: node dropped for it.
- Requester asks again with `offset` + entries received (`dirs` plus `files`) until an answer
  lacks `more`.
- Per node, a requester asks at most 100 *(initial)* pages and takes at most 10,000
  *(initial)* entries; a node still answering `more` after that is dropped for this listing.
- Each page is its own exchange; no state kept between pages. If the directory changes in
  between, entries can be missed or seen twice; requester merges directories by name, files by
  root.

## 3. Privacy

A browse list shows all a node shares to whoever asks, tied to its node ID. An implementation SHOULD leave the capability off by default, or limit it to friends.

## 4. Open points

- Whether an answer should say that a listing is partial by the node's choice.
