# Dat/Hypercore and Syncthing compared with ed3k

**Status:** design note (2026-10-12). Informative.
**Relates to:** [`Protocol.md`](../Protocol.md) §7, §10 and §12, and the extensions [`file-set`](../extensions/file-set.md), [`multi-author-set`](../extensions/multi-author-set.md), [`hidden-set`](../extensions/hidden-set.md), [`pubsub`](../extensions/pubsub.md), [`live`](../extensions/live.md) and [`friend`](../extensions/friend.md). Section numbers (§) refer to `Protocol.md` unless another document is named.

**Hypercore** (the base of what was called Dat) is a log that only grows. Its blocks hang in
a hash tree, the owner signs the root, and a reader can fetch and verify any block alone. File
systems and databases are built on such logs. **Syncthing** keeps folders equal between the
devices of one user or a few. Devices tell each other what each file looks like, as a list of
block hashes, and fetch the blocks they lack.

Hypercore is described from its documentation and Syncthing from the specification of its
Block Exchange Protocol, both read on 2026-10-12. Nothing was run.

**Result:** one rule was taken over: when two versions carry the same number, every node
picks the same one (section 2.1). Most of the rest ed3k has in its own form (section 3).
Seven ideas were looked at and left (section 4).

## 1. Overview

| Area | From | ed3k | Section |
|---|---|---|---|
| Two writers, one version number | Syncthing version vectors; Hypercore fork counter | taken over: one winner everywhere | 2.1 |
| Any block verified alone; partial sources; a place in the DHT that hides the key; known devices | both | has it | 3 |
| Fetching only what changed in a file | Syncthing | looked at, not taken now | 4.1 |
| One signed tree over a whole broadcast | Hypercore | looked at, not taken now | 4.2 |
| Telling peers of new blocks every few seconds | Syncthing | looked at, not taken now | 4.3 |
| Sending only the changes of a list | Syncthing | looked at, not taken now | 4.4 |
| Proving that one knows the key before data is given | Hypercore | looked at, not taken now | 4.5 |
| Block size by file size | Syncthing | looked at, not taken now | 4.6 |
| A device that adds devices for others | Syncthing | looked at, not taken now | 4.7 |

## 2. Taken over

### 2.1 One winner when two versions carry the same number

**What the two do.** Syncthing gives every file a version vector: one counter per device that
changed it. Two versions of which neither includes the other are a conflict, and the vector
shows it. A Hypercore has one writer; when the writer cuts the log back and writes another
continuation, a fork counter in the signed root goes up, and the higher counter wins.

**What ed3k had.** A head carries one number, `seq`. A storing node kept a head with the same
`seq` and another root out. Two machines of one author, or two co-owners, could each sign a
version with the next number; every storing node then kept whichever came first, and the
followers were split until a later version.

**What ed3k takes** ([`file-set.md`](../extensions/file-set.md) §5.2): of two heads with the
same `seq`, the one with the lower root wins, on storing nodes, in the push mesh and for
followers. The same rule is in the joint head and the part record of
[`multi-author-set.md`](../extensions/multi-author-set.md), and in the hidden head, where the
sealed bytes are compared because a storing node cannot see the root.

| Choice | Pro | Con |
|---|---|---|
| **A fixed winner in place of first come** | every node ends at the same version, without talking to the others | one of the two versions loses, and its author is not told by the protocol. A client MAY tell its user |
| **The lower root** | needs no clock and no further field; no node can prefer its own | arbitrary: it is not the later or the better version |
| **No vector of counters** | a head stays one number | two versions that were made apart are not recognised as such. The losing changes have to be published again in a later version |

A version vector would tell the co-owners that they wrote at the same time, so that one of
them could merge. With at most 8 co-owners it would fit, and it stays an option.

## 3. What ed3k has already

| System | Mechanism | In ed3k |
|---|---|---|
| Hypercore | A hash tree over the blocks, so that any block is verified alone and a reader holds only parts | the BLAKE3 tree and verified ranges (§7.2, §12.4) |
| Hypercore | A discovery key derived from the public key: peers find each other without the DHT learning the key that verifies the data | the head target is a hash of key and `id` (§10.1). The key is still sent to the storing node as `signer`; a hidden set uses a key made for it alone ([`hidden-set.md`](../extensions/hidden-set.md)) |
| Hypercore | A reader is told when the log grew | a new head, by poll or push ([`pubsub.md`](../extensions/pubsub.md)); segment records of a live stream |
| Hypercore | A file system on logs, with every earlier state still readable | every version of a set is a manifest that stays a file ([`file-set.md`](../extensions/file-set.md) §5.2) |
| Hypercore | Blocks encrypted with a key that readers hold | sealed manifests and members; private channels |
| Hypercore | Several writers combined above single-writer logs | a set with co-owners or open parts ([`multi-author-set.md`](../extensions/multi-author-set.md)) |
| Hypercore | Hole punching arranged through DHT nodes | through relays (§9.5) |
| Syncthing | A device ID that is the hash of the device's certificate | the node ID is the hash of the node key (§6.1) |
| Syncthing | Servers that tell where a device ID is now | the friend record in Kad, readable by friends only ([`friend.md`](../extensions/friend.md)) |
| Syncthing | Announcements in the local network | [`lan-discovery.md`](../extensions/lan-discovery.md) |
| Syncthing | A peer serves blocks of a file it is still downloading | partial sources with `have` (§12.1) |
| Syncthing | The hash expected for a block travels in the request | a block is verified against the root by the receiver (§12.3) |
| Syncthing | A device that stores a folder in encrypted form and cannot read it | a node may hold and serve sealed files without the link (`hidden-set.md` §4.2) |
| Syncthing | Deleted files are kept as old versions if the user wants | a client never deletes a file because of a new version (`file-set.md` §5.3) |

## 4. Looked at, not taken now

### 4.1 Fetching only what changed in a file (Syncthing)

Syncthing describes a file as a list of block hashes. A device that has an older version
fetches the blocks whose hashes changed and keeps the rest. A block may even be taken from
another file that holds the same data.

In ed3k a member with a new root is a new file and is downloaded whole. The means for more
are there: the hash tree of the new file can be asked for (§12.4) and compared with the tree of
the old file at the same path. Ranges with equal hashes need no download.

| Pro | Con |
|---|---|
| no change on the wire | BLAKE3 counts the position into every hash, so data matches only at the same offset. It helps files that were appended to or patched in place, not files with bytes inserted |
| a small change to a large file would cost little | clients have to keep the old file and its tree, and only clients that do it gain |

It could be written as a recommendation for clients in
[`file-set.md`](../extensions/file-set.md) §5.3.

### 4.2 One signed tree over a whole broadcast (Hypercore)

A Hypercore owner signs the root of the tree over all blocks so far. The newest signature
proves every earlier block.

In ed3k every segment of a live stream has a signed record of its own, and a node keeps a
window of about 30 segments ([`live.md`](../extensions/live.md) §2.2, §3.3).

| Pro | Con |
|---|---|
| a recording, or a viewer far behind, could verify old segments from one signature | the window is short on purpose. Whoever keeps a recording can publish it as a set |
| | a proof per segment in place of one signature check; more to carry and to get wrong |

### 4.3 Telling peers of new blocks every few seconds (Syncthing)

Syncthing devices send what they have newly got of a file in progress every 5 seconds, as
additions to what they sent before.

In ed3k the uploader sends a changed bitmap to a downloader that holds a slot at most once per 5 min (§12.5); the downloader reports its own in a `FileRequest` when it changes.

| Pro | Con |
|---|---|
| downloaders of a new file would trade fresh pieces with each other much sooner | a message per connection every few seconds, for every file in progress |
| | a form of the bitmap that carries additions only. Syncthing folders are shared by a few devices; an ed3k file may have hundreds of peers |

### 4.4 Sending only the changes of a list (Syncthing)

Syncthing numbers every change of a folder. A device says how far it has the other's list and
receives only what came after.

In ed3k a new version of a set is a new manifest. A large set is split into inner sets so that
an update sends only the manifests on the way to the change
([`file-set.md`](../extensions/file-set.md) §2.5).

| Pro | Con |
|---|---|
| an update of a flat list of 50,000 files would be a few entries | a manifest is a file with one root, which is what makes a set verifiable and shareable by anyone. A list of changes would be a second thing to verify |

### 4.5 Proving that one knows the key before data is given (Hypercore)

Peers find each other under the discovery key. A peer then has to show that it knows the
public key itself, which the DHT never saw, before the other side sends data.

| Pro | Con |
|---|---|
| a node that only watched the DHT cannot fetch | an ed3k file is public by its root. Where it should not be, a private channel has this proof ([`live.md`](../extensions/live.md) §10.3), and a hidden set seals the data |

### 4.6 Block size by file size (Syncthing)

Syncthing uses blocks of 128 KiB to 16 MiB, the smallest size that gives a file fewer than
2,000 blocks.

ed3k has no list of blocks to keep small. The tree is asked for level by level (§12.4), and a
request names a range. Nothing to take.

### 4.7 A device that adds devices for others (Syncthing)

A Syncthing device can be marked as an introducer: the devices it knows are added
automatically.

This is the introduction weighed in [`tox-briar-comparison.md`](tox-briar-comparison.md) §4.2,
without asking the user. ed3k adds no friend without the user.

## 5. Not taken, or does not apply

| System | Subject | Why |
|---|---|---|
| Hypercore | A log that only grows as the one data structure | ed3k shares files. A file is fixed once it has a root |
| Hypercore | Databases and file systems that read single blocks from the network on demand | a matter of what is built on top |
| Syncthing | Keeping a folder equal in both directions | a followed set is one direction. Two directions need the version vector of section 2.1 |
| Syncthing | Relays that carry the data | see [`tox-briar-comparison.md`](tox-briar-comparison.md) §4.6 |
| Syncthing | Opening ports on the router | outside the protocol; a client may do it |

## 6. Open points

- A version vector for sets with co-owners (section 2.1).
- The ideas of section 4. Fetching only what changed (4.1) needs no change on the wire.
