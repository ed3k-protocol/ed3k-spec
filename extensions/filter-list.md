# Extension: shared filter lists (`filter-list`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §5 and [`Protocol.md`](../Protocol.md) §10.7. § = this document unless a document is named.
**Capability:** none. One file format. Nothing changes between two nodes.

A user who finds a file is not what its name says, or a publisher fills searches with junk, can
tell only their own client. This lets users share such verdicts.

- A **filter list** is a file naming files and publishers with a verdict.
- Published as a member of a signed **set that changes** ([`file-set.md`](file-set.md) §5).
  Following the set keeps it up to date; with [`pubsub.md`](pubsub.md) a new version arrives
  within seconds.
- A list has effect only in the client of a user who **chose to follow it**.

Idea from ZeroNet: a mute/block list is a file on a site other users include ([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.2).

---

## 1. The list file

```
"ed3kflt" ‖ 0x01 ‖ FilterList
```

File starts with the 8 bytes `65 64 33 6B 66 6C 74 01`; the rest is one protobuf message:

```protobuf
message FilterList {
  string title = 1;
  string comment = 2;
  uint64 created = 3;         // Unix time in seconds; 0 = not told
  repeated Group groups = 4;
}

message Group {
  Subject subject = 1;
  Verdict verdict = 2;
  bytes   ids = 3;            // n × 16 B, in ascending order, each once
  string  reason = 4;         // optional, for the whole group
}

enum Subject {
  SUBJECT_NONE = 0;
  FILE = 1;                   // root[0..16) of a file (§7.1 of Protocol.md)
  ED2K = 2;                   // eD2K hash of a file (§7.4 there)
  AUTHOR = 3;                 // node ID of a key that signs records, sets or channels
}

enum Verdict {
  VERDICT_NONE = 0;
  FAKE = 1;                   // the content is not what the names say
  HARMFUL = 2;                // malware, or a file that damages a player or an unpacker
  SPAM = 3;                   // published in bulk to fill search results
  GOOD = 4;                   // checked by the list's author and found to be what it says
}
```

- File SHOULD be named `<title>.ed3kflt`; as a set member it MUST have that ending (§2). A
  client recognises a list by its first 8 bytes.
- `ids` is sorted, so lookup needs no index. A list where an `ids` is not a multiple of
  16 bytes, not ascending, or holds an entry twice is refused as a whole; checked only in groups
  whose `subject` and `verdict` are both known to the reader and not 0.
- A group with a `subject` or `verdict` unknown to the reader is ignored, its `ids` not looked
  at; likewise a group with `SUBJECT_NONE` or `VERDICT_NONE` (judges nothing). An ignored group
  is not kept: a client re-writing a list leaves it out.
- A list file is at most 4 MiB *(initial)* including the 8 bytes, about 260,000 entries.
- An `AUTHOR` is a key's node ID: publisher of keyword and note records
  ([`Protocol.md`](../Protocol.md) §10.7), author of a set, key of a live channel, member key of
  a room ([`room.md`](room.md) §1.1). A search result is judged by its record's publisher and,
  for a signed manifest, also by the set's author key once the manifest is known; either one
  marks it.
- **A list is complete.** An entry the next version leaves out is no longer judged.

---

## 2. Publishing and following

- A list is a member of a signed set with an `id` ([`file-set.md`](file-set.md) §5.1); the set's
  author is the author of its lists.
- A user follows a list by its `ed3k:set` link. **Which members are lists** is decided from the
  manifest, before fetching anything. A member is a list file when it
  - is at the set's top level (`dir` = 0),
  - is not marked `set`,
  - has a `name` ending in `.ed3kflt`, compared as bytes,
  - and has a `size` of at most 4 MiB *(initial)*.

  The first 64 *(initial)* such members, in `files` order, are read. Other members are ignored
  by this extension, as is a list file not starting with the 8 bytes of §1.
- A new version is found as for any set (`file-set.md` §5.3). A member with unchanged root is
  not downloaded again, so an author of a large list MAY split it into several files and change
  only some.
- A client follows at most 32 *(initial)* sets of lists and reads at most 64 *(initial)* list
  files per set, the first ones as above. List files read total at most 64 MiB *(initial)*,
  counted by manifest `size`, and a list opened by hand by its file length. Lists count in the
  order the user followed/opened them, kept across restarts; one that would go beyond is not
  read. A list opened by hand is not one of the 32. A new version that would go beyond is not
  read either: client keeps the old version. In both cases the client says so.
- A list file opened by hand, without a set, MAY be used; it has no author and no updates, and
  the client says so.
- A user's own verdicts are their own list, kept by the client. A client MAY let the user
  publish it as a set; that is how lists come into being. Its author key SHOULD be its own key,
  not the node key, so the list does not name the node.

---

## 3. What a client does with a list

- **Only the user turns a list on.** A client MUST NOT follow a list the user did not choose,
  and MUST NOT ship with one already on.
- **The default is to mark.** A search result, download or set a list judges is shown with the
  verdict and the list's title. Whether a `FAKE`, `HARMFUL` or `SPAM` result is hidden, and a
  download of it refused, is a per-list user setting.
- **The user can always see why.** A client that hides something MUST be able to show what it
  hid and which list said so; the user's own decision for an entry overrules every list: a
  verdict in their own list, or an unpublished local choice to show it.
- **Nothing is deleted.** A client MUST NOT delete a file, or stop a running download, because
  of a list; it tells the user.
- **`GOOD` replaces no check.** A file is verified against its root as always; `GOOD` says only
  that the list's author looked at it.
- **Lists that disagree** (`GOOD` vs `FAKE`, `HARMFUL` or `SPAM`) are both shown: entry marked,
  not hidden or refused, unless the user decided it. A client does not decide between them.
- **A list judges what is published, not who connects.** A client MUST NOT refuse a connection,
  Kad request or upload because of a list. A list holds no addresses.
- **A storing node does not filter.** A node MUST NOT apply a list to records it stores or
  returns for others ([`Protocol.md`](../Protocol.md) §10.3). The searching node filters, for its
  own user; else a list many nodes follow would decide what the whole network can find.

---

## 4. What it gives, and to whom

| | |
|---|---|
| A user | verdicts of people they chose to trust, kept up to date |
| The author of a list that many follow | power over what its followers see. A wrong entry hides a good file from all of them until noticed; hence §3 marks by default and shows the source |
| Someone who wants to hide a file | must get it onto a list the victim follows. Publishing one's own list does nothing |
| A publisher on a list | makes a new key and is off it. A list of files holds longer than one of authors |
| An observer | sees a node follows a list's set, as for any set (`SUB` and `SOURCE` records) |

---

## 5. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Votes in the network**: every node publishes verdicts as records, a searcher counts them | no list authors | whoever makes most keys wins; note records (§10.3 of `Protocol.md`) already carry a rating and have this weakness |
| **A list as a plain file at a URL**, as eMule's `ipfilter.dat` | simplest | one server per list, no signature, no shared format |
| **Lists applied by storing nodes** | spam never reaches a searcher | a few lists would rule the network; no user could look behind them |
| **Address ranges in a list** | the filter eMule users know | cuts nodes off instead of judging content, punishes whoever gets the address next, makes a list a weapon |
| **Deltas instead of complete lists** | small updates | a client that missed one is wrong unknowingly; splitting a list into files gives most of the saving |
| **A Bloom filter instead of ids** | 10 times smaller | false matches hide good files; user can't be shown the matching entry |

---

## 6. Open points

- Whether 16 bytes per entry should become 8 for large lists.
- Whether a verdict should be able to name the right file for a fake ("this is really …").
- Whether a client should warn when a followed list grows or changes very fast.
- All numbers are first values.
