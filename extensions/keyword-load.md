# Extension: keyword levels and ranking (`keyword-load`)

**Status:** draft 0.1 (2026-10-11). Not implemented. The wire fields and the level target are part of wire version 1. At an index node the rules of this document are part of version 1 too (`Protocol.md` §11); on Kad they are reserved for a network upgrade.
**Extends:** [`Protocol.md`](../Protocol.md) §10.1, §10.3 and §10.8. § = that document unless this one is named.
**Capability:** bit 8, `LEVELS`.

Keyword frequency very uneven: a few words occur in a large share of file names; a DHT puts
all records of one word on the same few nodes. Version 1 does as eMule: storing node holds
a fixed number of files per keyword, refuses the rest (§10.8). This extension gives a crowded
keyword more room and an order:

- **Levels.** Up to 16 targets per keyword, level 0 to 15, each on different nodes. A file
  finding one level full is published one level down.
- **Ranking.** Full node keeps the files with most publisher sites. Popular files rise to level
  0, rest sink, so a search's first lookup returns the most widely shared files.
- **No early deletion.** File losing its place isn't deleted; record stays until expiry, by
  when its publishers have stored it one level down.

A file is published under the keywords `Protocol.md` §10.1 selects, as in version 1.

## 1. What version 1 already carries

| Item | Where | Use |
|---|---|---|
| Level in the keyword target | §10.1 | `H("ed3k v1 keyword", level(1) ‖ word)`; version 1: level 0 on Kad, every level at an index node |
| `KadRequest.level` | §10.3 | level sender computed the target for; storing node checks a keyword of the name gives the target at that level (§10.7) |
| `KadAnswer.load` | §10.3 | files held under this target, % of level size |
| `KadAnswer.results` | §10.3 | one per stored record: `STORED`, `FULL`, `BUSY`, `REJECTED`, `OLDER` |
| `KadAnswer.spill` | §10.3 | node answered `FULL` for this target within last record lifetime |
| `KadAnswer.sites` | §10.3 | per returned file, publisher sites node counted; 0 = not counted |

Switching on the rules below changes nothing on the wire.

## 2. Storing node

### 2.1 Entries

One **entry** per file (root) and target, holding:

- up to 3 *(initial)* signed records of different publishers, kept by the rule of §10.8, one
  place per site. Kept record's site = site of the address of the `Store` that last stored it,
  renewal included.
  - Record of an already-kept publisher takes that publisher's place, whatever its site; kept
    record then has this `Store`'s site.
  - Else record from a site holding a place takes that site's record's place (least fresh one,
    should a renewal have left two of one site).
  - Record from another site: added while fewer than 3 kept, else replaces least fresh kept.
  - A `Store` with `signer` never takes the place of another publisher's record of another
    site; answered `REJECTED`.
- **sites** of publishers that stored the file, and on admission those of its candidates (§2.2
  of this document), each remembered until that publisher's record expires. Site = group of
  [`ip-diversity.md`](ip-diversity.md) §1: IPv4 /24, IPv6 /48. A site counts once; the count saturates at
  255 *(initial)*. Only stores answered `STORED` add a site; offer answered `FULL` adds its site
  to candidates (§2.2 of this document). `Store` with `signer` adds no site, neither to `sites`
  and rank nor to candidate sites (§2.2 of this document): its site is the last hop's, which an
  anonymous key borrows for free ([`hops.md`](hops.md) §4.2).

**Rank** = entry's number of sites. Node verifies every record sent (§10.7), also unkept ones.

Target holds at most S = 100,000 *(initial)* entries: the **level size**.

### 2.2 Storing a record

Record for file F arrives under target T. Rows tried in order; first match decides.

| Situation | Action | Result |
|---|---|---|
| F held under T with a record from same publisher not older than this one (§10.7) | nothing | `OLDER` |
| F held under T, not leaving | renew entry, add site | `STORED` |
| F held but **leaving** (§2.3 of this document) | add site to candidates | `FULL` |
| F not held, node has no room at all | nothing | `BUSY` |
| F not held, T has fewer than S entries | create entry with candidate's sites, if any | `STORED` |
| F not held, T full, F doesn't outrank weakest | add site to candidates | `FULL` |
| F not held, T full, F outranks weakest | create entry with candidate's sites; weakest becomes leaving | `STORED` |

- `Store` with `signer` refused by §2.1 of this document is answered `REJECTED` where a record
  would take a place (row 2). Leaving entry takes none, so row 3 answers `FULL` also to such a
  `Store`, which adds no candidate site.
- **Candidates.** For refused files, node keeps offering sites for one record lifetime. At most
  S/4 *(initial)* files per target; when full, forget the file with fewest sites; tie → oldest
  last offer; same second → lower root as bytes. The file at hand is among them.
- **Outranks** = more candidate sites than weakest entry's rank. Site of offer at hand counts,
  once; a `Store` with `signer`'s doesn't (§2.1 of this document). Equal rank never displaces:
  first-come files stay.
- **Weakest** = lowest-rank non-leaving entry; tie → created last.

### 2.3 Leaving entries

Displaced entry is **leaving**. Node:

- keeps it, returns it in searches until its last record expires
- doesn't renew it; publishers get `FULL`, move one level down (§3 of this document)
- doesn't count it towards S

So a file exists at its old level until its publishers have placed it at the next; they refresh at
different times, each storing below before its own record above runs out.

At most S/4 *(initial)* leaving entries per target; while that limit is reached, no file admitted by
outranking.

### 2.4 Answers

- `load` = non-leaving entries × 100 / S.
- `spill` set while node answered `FULL` for this target within last record lifetime; so it stays
  set that long after target stopped being full, covering records still living one level down.
- Results in descending rank, leaving entries last; `offset` pages along that order. `sites`
  carries each returned file's rank. Answer carries every record of a returned entry,
  consecutively, each with entry's rank in `sites`. Equal-rank entries in **Paging** order
  (`Protocol.md` §10.3) by first record; likewise records of one entry.
- **An index node** (`Protocol.md` §11) MAY follow this document already in version 1, like any
  storing node, with operator-chosen level size S. No `LEVELS` bit needed: its `FULL` and
  `spill` tell clients what to do. Each level is its own target there. Target answers no `FULL`
  until it holds S entries, spills to next level from then on. Publisher storing at one index
  node judges §3 of this document by that node's answer.
- `BUSY` = node as a whole out of room. Total storage is local policy; SHOULD have room for at
  least 2 S entries; no target takes more than S plus its leaving entries (§2.3 of this
  document).

## 3. Publisher

Publisher remembers one level L per (file, keyword), starting at 0. Outcome of a store at one
level is taken over storing nodes that answered `responsible`, ignoring `BUSY` and `REJECTED`:

- **placed:** at least half answered `STORED` or `OLDER` (both = node holds the file)
- **full:** otherwise

A refresh:

1. If L > 0, store at level L − 1 first. If placed, set L to L − 1 and stop.
2. Store at level L. If placed, stop.
3. If full, set L to L + 1 and repeat step 2, up to level 15.
4. If level 15 is full, give up until the next refresh.

All responsible nodes answered `REJECTED` → publisher stays at L, stores again next refresh.
`REJECTED` is a per-address/per-publisher limit (§10.8), not a full level: moving down on it
would give a fresh quota on every level.

- Step 1 is how a file rises: nodes one level up count sites offering it, admit it once it
  outranks their weakest file. A file rises at most one level per refresh.
- Record stored on fewer than half of the nodes stays there; publisher still moves down, step 1
  of next refresh renews it.
- Minority answering `FULL` can't push a file down.
- Cost: one lookup per refresh at level 0, two below, one per level on the way down.

## 4. Searcher

1. Search at level 0.
2. Go to next level when more results wanted and any responsible node of current level answered
   with `spill`. Stop at level 15. How many results wanted is local policy.

- Searcher MAY remember how deep a keyword went and query those levels in parallel next time.
- Results merged by root across nodes and levels. Popularity = lower median of non-zero `sites`
  values reported for the file within one level, 0 when all 0; file at several levels takes the
  highest per-level value; storing nodes' claim, not a signed fact.
- False `spill` costs one lookup, so following it on one node's word is acceptable; level limit
  bounds the total.

## 5. Deployment

On Kad: a network rule (§16), in two steps. At an index node: from version 1 (§2.4 of this
document), since one node decides alone and clients follow its answers.

1. **Announce.** Implementing nodes set `LEVELS`. Storing nodes rank and keep leaving entries;
   publishers and searchers use levels among each other.
2. **Enforce.** From a date fixed in the specification, every node follows §2 to §4 of this document.

In between:

| Old node as | Effect |
|---|---|
| Storing node | refuses when full w/o ranking (§10.8). Sets `spill` and `results` correctly, so publishers and searchers still move on |
| Publisher | never below level 0; its files missing from full keywords, as in eMule today |
| Searcher | sees level 0 only, which holds highest-ranked files |

## 6. What it costs and what it gives

| | Value |
|---|---|
| Files per keyword on one node | 100,000, plus up to 25,000 leaving |
| Files per keyword in the network | 1.6 million over 16 levels |
| Memory for one full target | ~30 MB at 300 B per entry (estimate) |
| Lookups for a search content with top results | 1 |
| Lookups to see all of a keyword | one per level in use |
| Lookups per refresh and keyword | 1 at level 0, 2 below |

- **Common word and other words**: filtered on storing node (§10.3), but file may be at any
  level; finding it for certain means walking all levels in use. Pair targets (§8 of this document,
  row 7) would answer in one lookup.
- **Deep levels searched rarely.** One-publisher file under a very common word is stored but
  seldom seen via that word; still found via its other words.
- **Junk no longer blocks real files.** Flood of one-publisher records fills levels from the
  top in arrival order, but any file with more sites displaces it.

## 7. What an attacker can still do

- **Buy rank with addresses.** Rank counts sites of direct stores; stores via a last hop add
  none (§2.1 of this document). 256 /24 networks give rank 255, the max: attacker with a /16
  can put its files atop a keyword. Honest popular files keep same rank, not displaced by equal
  rank.
- **Lie as storing node.** One of the closest can report any `sites` value, withhold records or
  answer `FULL`. Median over a level, "at least half" rule and diversity limits on the closest
  set bound what one node achieves.
- **Fill candidate table.** Offers for many files push real candidates out of a full target's
  table. Fewest-sites file dropped first, so a file with several sites survives.
- **Exhaust last level.** Beyond level 15 a file is refused, as in version 1.

## 8. Alternatives considered

Measures discussed for crowded keywords; "later" ones can be added w/o message changes.

| # | Measure | Status |
|---|---|---|
| 1 | Define `load`; publishers decide over all answers, not the average; no back-off longer than a record lifetime | in version 1 (§10.8) and here (§3 of this document) |
| 2 | Tell "this keyword is full" from "this node is full" | in version 1: `FULL` and `BUSY` |
| 3 | Publish a file only under its rarer words; searchers target a query's rarest word | not done; file published under all its words. Later if needed: publisher/searcher policy, using `load` |
| 4 | Storage budget in bytes, each keyword limited to a share | storing node's local policy (§2.4 of this document); later if needed |
| 5 | When full, displace file with fewest publisher sites instead of refusing | here (§2 of this document) |
| 6 | One entry per file with count of publisher sites, not every publisher's record | here (§2.1 of this document) |
| 7 | Pair targets: target from two keywords, used when one is crowded | later if needed: further target derivation agreed by publishers and searchers via network upgrade; no message changes |
| 8 | Spill to nodes further from the target | replaced by levels, which use other nodes w/o changing closest-set rules |

Further alternatives:

- **Load threshold instead of per-record results.** Every publisher, incl. already stored,
  would see high load and move; the level would empty within a day and fill again.
- **Repeated hash instead of level index.** Equivalent, but can't jump to a level w/o computing
  all before it.
- **Delete displaced file at once.** Missing until publishers' next refresh, up to a day.
- **Walk from level 0 every refresh.** Popular file reaches its place within a day, at up to 16
  lookups per keyword and day for every file in a deep level.
- **Random start level for searches** (spread query load). Defeats ranking, which puts best
  results at level 0.

## 9. Comparison with eMule

| | eMule Kad | ed3k version 1 | With this extension |
|---|---|---|---|
| Files per keyword on a node | 50,000 | 100,000 | 100,000 per level, 16 levels |
| All keywords on a node | 60,000 | local | local, ≥ 2 levels' worth |
| When full | refuse; above 90 % also refuse refreshes | refuse new, renew held | displace weakest, or send one level down |
| Signal to publisher | load 0–100, averaged over 3 nodes | result per record from 8 nodes | same |
| Publisher reaction | no publish for up to 7 days when average is above 20 | retry next refresh | next level at once |
| One lying node | silences keyword for days | no effect below half of nodes | no effect below half of nodes |
| Order of results | trusted entries first, at most 300 | storing node's choice | by publisher sites |

## 10. Open points

- Should a block (IPv4 /16, IPv6 /32) contribute only a few sites to a rank, so rank costs more
  than one address block? Would also compress rank of honest files popular within one large
  provider.
- S, level count, candidate/leaving table sizes set w/o measurements. Record size 300 B
  estimated from encoding rules.
- Do notes need the same treatment? File with more than a few hundred notes not expected.
