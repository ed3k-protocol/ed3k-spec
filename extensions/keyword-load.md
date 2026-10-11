# Extension: keyword levels and ranking (`keyword-load`)

**Status:** draft 0.1 (2026-10-11). Not implemented. The wire fields and the level target are part of wire version 1; the rules of this document are reserved for a network upgrade.
**Extends:** [`Protocol.md`](../Protocol.md) §10.1, §10.3 and §10.8. Section numbers (§) refer to that document.
**Capability:** bit 8, `LEVELS`.

Keyword frequency is very uneven. A few words occur in a large share of all file names, and a
DHT puts every record of one word on the same few nodes. Version 1 handles that as eMule does: a
storing node holds a fixed number of files per keyword and refuses the rest (§10.8).

This extension gives a crowded keyword more room and an order:

- **Levels.** A keyword has up to 16 targets, level 0 to 15, each on different nodes. A file that
  finds one level full is published one level down.
- **Ranking.** A full node keeps the files with the most publisher sites. Popular files rise to
  level 0 and the rest sink, so the first lookup of a search returns the most widely shared
  files.
- **No early deletion.** A file that loses its place is not deleted. Its record stays until it
  expires, and by then its publishers have stored it one level down.

A file is published under every keyword of its name, as in version 1.

## 1. What version 1 already carries

| Item | Where | Use |
|---|---|---|
| Level in the keyword target | §10.1 | `H("ed3k v1 keyword", level(1) ‖ word)`; version 1 uses level 0 |
| `KadRequest.level` | §10.3 | the level the sender computed the target for |
| `KadAnswer.load` | §10.3 | files held under this target, in percent of the level size |
| `KadAnswer.results` | §10.3 | one result per stored record: `STORED`, `FULL`, `BUSY`, `REJECTED` |
| `KadAnswer.spill` | §10.3 | the node answered `FULL` for this target within the last record lifetime |
| `KadAnswer.sites` | §10.3 | per returned file, the publisher sites the node counted; 0 = not counted |

Nothing on the wire changes when the rules below are switched on.

## 2. Storing node

### 2.1 Entries

A storing node keeps one **entry** per file (root) and target. An entry holds:

- up to 3 *(initial)* signed records of different publishers, the freshest ones, preferring
  different names
- the **sites** of the publishers that stored or offered the file, each remembered until the
  record that publisher sent expires. A site is the group of
  [`ip-diversity.md`](ip-diversity.md) §1: IPv4 /24, IPv6 /48. One site counts once. The count
  saturates at 255 *(initial)*

The **rank** of an entry is its number of sites. The node verifies every record it is sent
(§10.7), also those it does not keep.

A target holds at most S = 100,000 *(initial)* entries: the **level size**.

### 2.2 Storing a record

A record for file F arrives under target T:

| Situation | Action | Result |
|---|---|---|
| F is held under T | renew the entry, add the site | `STORED` |
| F is held but **leaving** (§2.3) | add the site to the candidates | `FULL` |
| F is not held, T has fewer than S entries | create the entry | `STORED` |
| F is not held, T is full, F does not outrank the weakest entry | add the site to the candidates | `FULL` |
| F is not held, T is full, F outranks the weakest entry | create the entry with the candidate's sites; the weakest entry becomes leaving | `STORED` |
| The node has no room at all | nothing | `BUSY` |

- **Candidates.** For files it refused, the node keeps the sites that offered them, for one
  record lifetime. The table holds at most S/4 *(initial)* files per target; when it is full, the
  file with the fewest sites is forgotten.
- **Outranks** means: more candidate sites than the rank of the weakest entry. Equal rank never
  displaces, so files that came first stay.
- **Weakest** is the entry with the lowest rank that is not leaving; among equals, the one
  created last.

### 2.3 Leaving entries

A displaced entry is **leaving**. The node:

- keeps it and returns it in searches until its last record expires
- does not renew it; its publishers get `FULL` and move one level down (§3)
- does not count it towards S

A file therefore exists at its old level until its publishers have placed it at the next one.
Publishers of one file refresh at different times, and each stores below before its own record
above runs out.

A target holds at most S/4 *(initial)* leaving entries. While that limit is reached, no file is
admitted by outranking.

### 2.4 Answers

- `load` = entries that are not leaving × 100 / S.
- `spill` is set while the node has answered `FULL` for this target within the last record
  lifetime. It stays set for that long after the target stopped being full, which covers records
  that still live one level down.
- Search results are returned in descending rank, leaving entries last. `offset` pages along
  that order. `sites` carries the rank of each returned file.
- `BUSY` means the node as a whole is out of room. How much a node stores in total is local
  policy; it SHOULD have room for at least 2 S entries, and no target takes more than S.

## 3. Publisher

A publisher remembers one level L per (file, keyword), starting at 0.

The outcome of a store at one level is taken over the storing nodes that answered
`responsible`, ignoring `BUSY`:

- **placed:** at least half of them answered `STORED`
- **full:** otherwise

A refresh:

1. If L > 0, store at level L − 1 first. If placed, set L to L − 1 and stop.
2. Store at level L. If placed, stop.
3. If full, set L to L + 1 and repeat step 2, up to level 15.
4. If level 15 is full, give up until the next refresh.

- Step 1 is how a file rises: the nodes one level up count the sites that offer it and admit it
  once it outranks their weakest file. A file rises at most one level per refresh.
- A record that was stored on fewer than half of the nodes stays there. The publisher still
  moves down, and step 1 of the next refresh renews it.
- A minority of nodes answering `FULL` cannot push a file down.
- Cost: one lookup per refresh at level 0, two below it, and one per level on the way down.

## 4. Searcher

1. Search at level 0.
2. Go on to the next level when more results are wanted and any responsible node of the current
   level answered with `spill`. Stop at level 15.

- A searcher MAY remember how deep a keyword went and query those levels in parallel next time.
- Results are merged by root across nodes and levels. The popularity of a file is the median of
  the `sites` values reported for it within one level; it is the storing nodes' claim, not a
  signed fact.
- A false `spill` costs one lookup. Following it on the word of a single node is therefore
  acceptable, and the level limit bounds the total.

## 5. Deployment

The rules are a network rule (§16) and come in two steps.

1. **Announce.** Nodes that implement the extension set `LEVELS`. Storing nodes rank and keep
   leaving entries; publishers and searchers use levels among each other.
2. **Enforce.** From a date fixed in the specification, every node follows §2 to §4.

What happens in between:

| Old node as | Effect |
|---|---|
| Storing node | refuses when full without ranking (§10.8). It sets `spill` and `results` correctly, so publishers and searchers still move on |
| Publisher | never goes below level 0. Its files are missing from full keywords, as in eMule today |
| Searcher | sees level 0 only, which holds the highest-ranked files |

## 6. What it costs and what it gives

| | Value |
|---|---|
| Files per keyword on one node | 100,000, plus up to 25,000 leaving |
| Files per keyword in the network | 1.6 million over 16 levels |
| Memory for one full target | about 30 MB at 300 B per entry (estimate) |
| Lookups for a search that is content with the top results | 1 |
| Lookups to see all of a keyword | one per level in use |
| Lookups per refresh and keyword | 1 at level 0, 2 below |

- **A search with a common word and other words** is filtered on the storing node (§10.3), but
  the file may be at any level. Finding it for certain means walking all levels in use. Pair
  targets (§8, row 7) would answer that in one lookup.
- **Deep levels are searched rarely.** A file with one publisher under a very common word is
  stored but seldom seen through that word. It is still found through its other words.
- **Junk no longer blocks real files.** A flood of one-publisher records fills levels from the
  top in arrival order, but any file with more sites displaces it.

## 7. What an attacker can still do

- **Buy rank with addresses.** Rank counts sites, so 256 /24 networks give rank 255, the
  maximum. An attacker with a /16 can put its files at the top of a keyword. Honest popular
  files keep the same rank and are not displaced by equal rank.
- **Lie as a storing node.** A node among the closest can report any `sites` value, withhold
  records or answer `FULL`. The median over a level, the "at least half" rule and the diversity
  limits on the closest set bound what one node achieves.
- **Fill the candidate table.** Offers for many different files push real candidates out of the
  table of a full target. The table drops the file with the fewest sites first, so a file with
  several sites survives.
- **Exhaust the last level.** Beyond level 15 a file is refused, as in version 1.

## 8. Alternatives considered

Measures discussed for crowded keywords, and what became of each. The ones marked "later" can
be added without a change to the messages.

| # | Measure | Status |
|---|---|---|
| 1 | Define `load`; publishers decide over all answers, not the average; no back-off longer than a record lifetime | in version 1 (§10.8) and here (§3) |
| 2 | Tell "this keyword is full" from "this node is full" | in version 1: `FULL` and `BUSY` |
| 3 | Publish a file only under its rarer words; searchers target the rarest word of a query | not done; a file is published under all its words. Later if needed: publisher and searcher policy, using `load` |
| 4 | Storage budget in bytes, each keyword limited to a share | local policy of a storing node (§2.4); later if needed |
| 5 | When full, displace the file with the fewest publisher sites instead of refusing | here (§2) |
| 6 | One entry per file with a count of publisher sites, instead of every publisher's record | here (§2.1) |
| 7 | Pair targets: a target from two keywords, used when one of them is crowded | later if needed: a further target derivation that publishers and searchers agree on by network upgrade; no message changes |
| 8 | Spill to nodes further from the target | replaced by levels, which use other nodes without changing the closest-set rules |

Further alternatives:

- **A load threshold instead of per-record results.** Every publisher, including those already
  stored, would see the high load and move. The level would empty within a day and fill again.
- **A repeated hash instead of a level index.** Equivalent, but a node could not jump to a level
  without computing all the ones before it.
- **Deleting a displaced file at once.** The file would be missing until its publishers' next
  refresh, up to a day.
- **Walking from level 0 at every refresh.** A popular file would reach its place within a day,
  at up to 16 lookups per keyword and day for every file in a deep level.
- **A random start level for searches**, to spread query load. It would defeat the ranking,
  which puts the best results at level 0.

## 9. Comparison with eMule

| | eMule Kad | ed3k version 1 | With this extension |
|---|---|---|---|
| Files per keyword on a node | 50,000 | 100,000 | 100,000 per level, 16 levels |
| All keywords on a node | 60,000 | local | local, at least 2 levels' worth |
| When full | refuse; above 90 % also refuse refreshes | refuse new files, renew held ones | displace the weakest, or send one level down |
| Signal to the publisher | load 0–100, averaged over 3 nodes | result per record from 8 nodes | the same |
| Publisher reaction | no publish for up to 7 days when the average is above 20 | retry at the next refresh | next level at once |
| One lying node | silences the keyword for days | no effect below half of the nodes | no effect below half of the nodes |
| Order of results | trusted entries first, at most 300 | storing node's choice | by publisher sites |

## 10. Open points

- Whether a block (IPv4 /16, IPv6 /32) should contribute only a few sites to a rank, to make
  rank cost more than one address block. It would also compress the rank of honest files that
  are popular within one large provider.
- S, the number of levels and the size of the candidate and leaving tables are set without
  measurements. The record size of 300 B is an estimate from the encoding rules.
- Whether a storing node should check that a name contains a word that hashes to the target at
  the given `level`. The specification does not yet define how a name is split into words.
- How an index node (§11), which is responsible for every target, treats levels. The simplest
  rule is that it holds everything at level 0 and never sets `spill`.
- Whether notes need the same treatment. A file with more than a few hundred notes is not
  expected.
