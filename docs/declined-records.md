# Declined records: where else a node's lists could apply

**Status:** design note (2026-11-16). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §10.3 "Declining", §10.4 "Declined stores", §10.8, §11. Section numbers (§) refer to that document.

A node may answer a store `DECLINED` for records it won't hold by its own policy, e.g. legal
reasons in its jurisdiction (§10.3). The spec covers only the storing side. A node keeping
lists of roots and keywords for that can use them elsewhere too; none of below needs a wire
change, all are local policy.

## 1. What the store side covers

How one implementation matches its lists (any other choice is fine, the answer is the same):

| Kind | Declined when |
|---|---|
| `SOURCE` | target is `root[0..16)` of a listed root, also as a target of `also` (rest of the batch still stored) |
| `KEYWORD` | target is a listed keyword's target at any level, or `FileInfo.root` listed, or file name holds all words of a listed keyword |
| `NOTE` | target from a listed root |
| `MEMBER` | target from a listed root, or `SetRef.set` listed |
| `HEAD` | manifest root listed, plain or joint |
| `ALIAS` | `Alias.root` listed |

Same rules as index node (§11). Records held when the lists change are dropped.

## 2. Other places

- **Search answers.** Leave matching held records out of `FindValue` / search answers, or
  answer nothing for a listed keyword target. §10.3 already lets a node leave held records out;
  with the store side in place this matters only for records stored before the list changed
  and not purged.
- **Uploads.** Refuse to serve a listed root to peers (block requests, piece requests). Own
  node only; peers find other sources.
- **Own publishing.** Don't publish sources, keywords, notes for shared files whose root is
  listed. Avoids the node putting out records it declines to hold for others.
- **Downloads.** Warn before or refuse starting a download of a listed root; hide matching
  rows in search results.
- **Relaying.** A relay (§9.4) sees no content, so nothing to match; lists don't apply.
- **Other networks' hashes.** Public block lists usually hold eD2K MD4 hashes or links, not
  BLAKE3 roots. A node could accept those and map them to roots through `ALIAS` records, and
  decline the alias records themselves.
- **Shared lists.** Lists could be fetched from a URL or a subscription topic the operator
  trusts; refresh by the same reload as a local file. Must stay operator choice, never
  default-on by the network.

## 3. Not built yet

- A UI to edit the lists and show what's loaded.
