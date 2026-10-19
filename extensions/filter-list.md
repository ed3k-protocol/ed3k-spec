# Extension: shared filter lists (`filter-list`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §5 and [`Protocol.md`](../Protocol.md) §10.7. Section numbers (§) refer to this document unless a document is named.
**Capability:** none. One file format. Nothing changes between two nodes.

A user who finds that a file is not what its name says, or that a publisher fills searches with
junk, can tell only its own client. This extension lets users share such verdicts.

- A **filter list** is a file that names files and publishers together with a verdict.
- It is published as a member of a signed **set that changes**
  ([`file-set.md`](file-set.md) §5). Following the set keeps the list up to date, and with
  [`pubsub.md`](pubsub.md) a new version arrives within seconds.
- A list has an effect only in the client of a user who **chose to follow it**.

The idea is ZeroNet's, where a mute list or block list is a file on a site that other users
include ([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.2).

---

## 1. The list file

```
"ed3kflt" ‖ 0x01 ‖ FilterList
```

The file starts with the 8 bytes `65 64 33 6B 66 6C 74 01`. The rest is one protobuf message:

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

- The file SHOULD be named `<title>.ed3kflt`; as a member of a set it MUST have that ending
  (§2). A client recognises a list by its first 8 bytes.
- `ids` is sorted, so a client looks an entry up without building an index. A list in which an
  `ids` is not a multiple of 16 bytes, not in ascending order, or holds an entry twice is
  refused as a whole. This is checked only in groups whose `subject` and `verdict` are both
  known to the reader and not 0.
- A group with a `subject` or `verdict` the reader does not know is ignored; its `ids` is not
  looked at. So is a group with `SUBJECT_NONE` or `VERDICT_NONE`, which judges nothing. An
  ignored group is not kept: a client that reads a list and writes it again leaves it out.
- A list file is at most 4 MiB *(initial)*, the 8 bytes included, which is about 260,000
  entries.
- An `AUTHOR` is the node ID of a key: the publisher of keyword and note records
  ([`Protocol.md`](../Protocol.md) §10.7), the author of a set, the key of a live channel.
- **A list is complete.** An entry that the next version leaves out is no longer judged.

---

## 2. Publishing and following

- A list is a member of a set that is signed and has an `id`
  ([`file-set.md`](file-set.md) §5.1). The author of the set is the author of its lists.
- A user follows a list by its `ed3k:set` link. **Which members are lists** is decided from
  the manifest, before anything is fetched. A member is a list file when it
  - is at the top level of the set (`dir` = 0),
  - is not marked `set`,
  - has a `name` that ends in `.ed3kflt`, compared as bytes,
  - and has a `size` of at most 4 MiB *(initial)*.

  The first 64 *(initial)* such members, in the order of `files`, are read. Other members are
  ignored by this extension, and so is a list file that does not start with the 8 bytes of
  §1.
- A new version is found as for any set (`file-set.md` §5.3). A member whose root did not
  change is not downloaded again, so an author of a large list MAY split it into several files
  and change only some.
- A client follows at most 32 *(initial)* sets of lists and reads at most 64 *(initial)* list
  files per set, the first ones as above. The list files it reads are at most 64 MiB
  *(initial)* in all, counted by their `size` in the manifest, and a list opened by hand by
  its file length. Lists count in the order the user followed or opened them; one that would
  go beyond is not read. A list opened by hand is not one of the 32. A new version that
  would go beyond is not read either: the client keeps the old version. In both cases the
  client says so.
- A list file that was opened by hand, without a set, MAY be used. It has no author and no
  updates, and the client says so.
- A user's own verdicts are a list of their own, kept by the client. A client MAY let the user
  publish it as a set. That is how lists come into being.

---

## 3. What a client does with a list

- **Only the user turns a list on.** A client MUST NOT follow a list the user did not choose,
  and MUST NOT ship with one that is already on.
- **The default is to mark.** A search result, a download or a set that a list judges is shown
  with the verdict and the title of the list. Whether a `FAKE`, `HARMFUL` or `SPAM` result is
  hidden, and whether a download of it is refused, is a setting the user makes per list.
- **The user can always see why.** A client that hides something MUST be able to show what it
  hid and which list said so, and the user's own decision for an entry overrules every list.
- **Nothing is deleted.** A client MUST NOT delete a file, or stop a download that is under
  way, because of a list. It tells the user.
- **`GOOD` replaces no check.** A file is verified against its root as always. `GOOD` says only
  that the author of the list looked at it.
- **Lists that disagree** are both shown. A client does not decide between them.
- **A list judges what is published, not who connects.** A client MUST NOT refuse a
  connection, a Kad request or an upload because of a list. A list has no addresses in it.
- **A storing node does not filter.** A node MUST NOT apply a list to the records it stores or
  returns for others ([`Protocol.md`](../Protocol.md) §10.3). Filtering is done by the node
  that searches, for its own user. Otherwise a list that many nodes follow would decide what
  the whole network can find.

---

## 4. What it gives, and to whom

| | |
|---|---|
| A user | the verdicts of people they chose to trust, kept up to date |
| The author of a list that many follow | power over what its followers see. A wrong entry hides a good file from all of them until someone notices. This is why §3 marks by default and shows the source |
| Someone who wants to hide a file | has to get it onto a list that the victim follows. Publishing a list of one's own does nothing |
| A publisher on a list | makes a new key and is off the list. A list of files holds longer than a list of authors |
| An observer | sees that a node follows the set of a list, as for any set (`SUB` and `SOURCE` records) |

---

## 5. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Votes in the network**: every node publishes its verdicts as records, and a searcher counts them | no list authors | whoever makes the most keys wins the count; note records (§10.3 of `Protocol.md`) already carry a rating and have this weakness |
| **A list as a plain file at a URL**, as eMule's `ipfilter.dat` | simplest | one server per list, no signature, no shared format |
| **Lists applied by storing nodes** | spam never reaches a searcher | a few lists would rule the network, and no user could look behind them |
| **Address ranges in a list** | the filter eMule users know | cuts nodes off instead of judging content, punishes whoever gets the address next, and is what makes a list a weapon |
| **Deltas instead of complete lists** | small updates | a client that missed one is wrong without knowing it; splitting a list into files gives most of the saving |
| **A Bloom filter instead of ids** | 10 times smaller | false matches hide good files, and a user cannot be shown the entry that matched |

---

## 6. Open points

- Whether 16 bytes per entry should become 8 for large lists.
- Whether a verdict should be able to name the right file for a fake one ("this is really …").
- Whether a client should warn when a list it follows grows or changes very fast.
- All numbers are first values.
