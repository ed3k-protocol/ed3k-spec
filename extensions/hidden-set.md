# Extension: hidden sets (`hidden-set`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §3 and §5, and [`Protocol.md`](../Protocol.md) §10.1, §10.3 and §10.7. § = this document unless a document is named.
**Capability:** none of its own; a node using it sets `SETS`. Record kind 8. One field in `Record`, one in `Member`, one parameter in `ed3k:set` links.
**Required of every `ROUTER`:** storing and returning the record kind `HIDDEN` (§3). The rest is optional.

Anyone can find and read a changing set ([`file-set.md`](file-set.md) §5): head record names
the manifest, manifest lists files. A **hidden set** is findable, readable and followable only
with its link.

- Link carries a **secret**; head record and manifest are encrypted with keys derived from it.
- A storing node can **check** a head record and tell newer from older, but can't read it.
- Head record's Kad location **changes every day**.
- Members MAY be encrypted too (§4.2).

I2P does this for a hidden service's record
([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.3); private live
channels ([`live.md`](live.md) §10) use the same construction for a stream.

---

## 1. The link

```
ed3k:set?key=<set key, base32 lower case, no padding>&s=<secret, 32 bytes, base32>[&dn=<url-encoded name>][&path=<url-encoded path>]
```

- Link with `s` = hidden set, no `id` parameter; both `id` and `s` → refused.
- **`key` is made for this set.** The author MUST NOT use its node key or another set's key. One
  key = one hidden set.
- `secret` = 32 random bytes.
- `key` and `s` are base32 per `Protocol.md` §13.2: written lower case without padding, read
  either way.
- Link invalid when `s` is repeated or doesn't decode to 32 bytes.

**Link = credential.** A client MUST NOT write the secret to a log or show it in an error
message, and SHOULD mark such a link private wherever shown.

---

## 2. Keys

```
k       = H("ed3k v1 hidden set", secret ‖ key)                           32 bytes
k_head  = H("ed3k v1 hidden set head", k)                                 32 bytes
k_data  = H("ed3k v1 hidden set data", k)                                 32 bytes

base    = H("ed3k v1 hidden head", key)[0..16)
target  = H("ed3k v1 hidden head target", base ‖ period(8))[0..16)        period = Unix time / 86400, rounded down, big-endian
```

`base`, `target` need only the key (storing node computes them); `k` and its derivatives need
the secret.

Sealed data:

```
sealed bytes = nonce(12) ‖ ChaCha20(key, nonce, plain bytes)              RFC 8439 cipher, counter from 0
```

with a random `nonce` per use, except the sealed `Head` of a head record (§3): nonce derived,
so a re-stored version has the same bytes, also after a restart:

```
head nonce = H("ed3k v1 hidden head nonce", k_head ‖ seq(8) ‖ root)[0..12)     seq and root as in the Head
```

Plain `Head` follows from `seq` and `root` → one nonce never seals two different texts. No
authentication tag: everything sealed is fixed by a signature or signed root.

---

## 3. The head record

```protobuf
// in enum Kind (§10.3 of Protocol.md):
HIDDEN = 8;

// in Record (§10.7 there):
Sealed sealed = 7;

message Sealed {
  uint64 seq = 1;             // FileSet.seq of the manifest named inside
  bytes  data = 2;            // sealed with k_head: a serialised Head (file-set.md §5.2)
  // field 3 belongs to the optional paid subscription extension (subscription.md §8.3)
}
```

- Record = `Signed` of kind `HIDDEN`, signed with the set key; in the signature **`base`**
  (day-independent) replaces `target`. So author signs once; others can store it on later days
  until its `expires`.
- Travels in `HEAD` record fields: `Store.head` with `Store.signer`, and `KadAnswer.sets`.
  `signer` always present; `Store` of kind `HIDDEN` without it → `REJECTED`.
- In the sealed `Head`: `id` and `seq` are the manifest's, `root` and `size` the sealed
  manifest's (§4.1), `name` empty.
- `data` is 12 to 256 bytes: nonce + sealed `Head`.

| | Storing node, on `Store` | Reader, on `KadAnswer` |
|---|---|---|
| Signature | valid under `signer`, with `base` computed from `signer` | valid under the link's key |
| Target | request target is `signer`'s for current day, day before or day after | the one it searched, and the link key's for current day, day before or day after |
| `expires` | later than now, ≤ 7 days *(initial)* plus 1 h ahead | later than now minus 1 h |
| `seq` | any; a record not replacing the held one → `OLDER` | highest of all answers wins |
| Count | one record per target, replaced by: higher `seq`; same `seq` and lower `data` (byte-string compare); same `seq` and `data` with later `expires`. A valid record not replacing it → `OLDER` | among equal `seq` the lower `data`, as on the storing node |
| Content | `sealed` only; else `REJECTED` | `data` opens to a `Head` with the outer `seq` and a 32 B `root` |
| Size | `data` 12 to 256 B; else `REJECTED` | the same |

- Storing node deletes the record at `expires`, at the latest 48 h after its target's day
  began.
- **Only the author can make a record.** Target follows from the signing key; nobody can occupy
  the place first under an own key.
- An author MUST store the same sealed bytes each time it re-stores one version: a fresh nonce
  would change `data` and thus the tie-break between two records of one `seq`. §2's derived
  nonce provides that.

**Storing**

- Author stores the record on publishing a version, re-stores like a follower, and before
  `expires` signs the same `data` with a later `expires`.
- Follower stores its held record under the current day's target, daily: at a random time
  others can't predict in the day's first 6 h *(initial)*, or at once when coming online later.
  Author need not be online.
- One day's storing node is, as a rule, not one of the next.

**Reading**

1. Search `HIDDEN` at today's and yesterday's targets, merge answers: a follower's new-day
   store of an old version must not hide a newer one still under yesterday's target.
2. Take highest-`seq` record, open `data`. One not opening per the Content row isn't taken:
   reader keeps its held version and stores the record it held.
3. Download the sealed manifest by its `root`, open it (§4.1), accept per
   [`file-set.md`](file-set.md) §5.3: signed by the set key, with the `seq` and `id` of the opened
   `Head`, and with the `id` of the previously held version, if any.

A follower checks as often as for any set (`file-set.md` §5.3). No [`pubsub.md`](pubsub.md) topic
for a hidden set in this version, except for a paid one ([`subscription.md`](subscription.md)
§8.4).

---

## 4. What is sealed

### 4.1 Manifests

- Manifest written per [`file-set.md`](file-set.md) §1, signed (§2.4 there), with `id` and
  `seq`, then sealed with `k_data`; the sealed file is shared.
- Signed by the set key alone. No co-owners for a hidden set
  ([`multi-author-set.md`](multi-author-set.md) §7): reader refuses a hidden manifest with a
  `grant`. No parts or mounts either: a hidden manifest with `open`, or a member with `mount`
  (`multi-author-set.md` §2, §3.1), is refused too.
- An inner manifest (`file-set.md` §2.5) of a hidden set is sealed the same way; its `root` and
  `size` in the outer manifest are the sealed file's.
- Sealed manifest lacks a manifest's 8 leading bytes; client knows it from head record or
  outer manifest.

### 4.2 Members

```protobuf
// in Member (file-set.md §1):
bool sealed = 9;              // the shared file is sealed with k_data; root and size are those of the sealed file
```

- Without `sealed`: ordinary file, downloadable by anyone knowing its root; same file as any
  other copy in the network.
- With `sealed`: stored and shared sealed. Client opens it after download, or while reading
  (cipher allows reading from any position).
- A sealed member holds ≤ 256 GiB plain bytes, the cipher's one-nonce limit: `size` at most
  2^38 + 12 bytes. Reader refuses a manifest listing a larger one.
- Sealed data doesn't compress; the rule of [`Protocol.md`](../Protocol.md) §12.3 then sends it
  as is.

| | Open member | Sealed member |
|---|---|---|
| Who can read it | everyone with the root | only who has the link |
| Sources | all sources of that file in the network | only nodes holding this set |
| What others see | a known file at a node | nameless data |
| Cost | none | 12 bytes, and client opens it before use |

### 4.3 Rules for a node that holds a hidden set

- Publishes a `SOURCE` record for the sealed manifest and each sealed member it shares; MUST NOT
  publish `KEYWORD` or `MEMBER` records for them; doesn't list them in `Browse`.
- Open member: nothing published because of the hidden set; shared for another reason → usual
  rules.
- A sealed file is named on the wire, in `FileStatus.name` (`Protocol.md` §12.1), by the first 16
  characters of its root in base32, lower case (`Protocol.md` §13.2). Manifest names stay inside.
- Member bitmap ([`file-set.md`](file-set.md) §4.1) works on the sealed manifest's root as on any
  manifest: knowing that root reveals which members a node holds, nothing about what they are.
- A reader asks the manifest's sources for the members first (`file-set.md` §3).

---

## 5. What it does and does not give

| | |
|---|---|
| Someone without the link | can't find the set, read its list, or read a sealed member |
| A node that stores the head record | learns set key, version number, addresses of those storing it there. Can't open it |
| A node that stored it once | can compute its place on any later day and search there; still can't open it |
| A node that stores `SOURCE` records | learns that some nodes share a nameless file |
| A follower who passes the link on | gives full access. Followers aren't told apart |
| Taking access away | impossible. A new secret = a new set; every follower needs the new link. A paid hidden set gives each follower an own expiring pass ([`subscription.md`](subscription.md) §8.3) |
| A secret that leaks later | opens every version someone kept. No forward secrecy |
| Open members | as visible as any shared file; only that they belong together is hidden |
| The author | not named by the `HIDDEN` record when storing through `signer` like any follower. Its `SOURCE` records for the sealed manifest (§4.3) are bound to its node ID like any, so a storing node sees who first offers a new version |
| An author who is away | can stay offline only until `expires` of the record its followers hold, ≤ 7 days *(initial)* plus 1 h. Only the author can sign a later one (§3); after that set isn't found until it returns |

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Key blinding** (I2P, Tor): record signed with a key derived from set key and day | storing node never learns set key, can't compute another day's target | author must sign daily, so be online; derived-key signing is outside most libraries' standard Ed25519 interface |
| **A fixed target** | simpler; no daily re-store | same few nodes see every follower for the set's life; anyone can place nodes there |
| **Target from the secret**, uncheckable by a storing node | storing node learns nothing | anyone knowing the target can put an own record there first; real one finds no room |
| **Sealing the head only** | manifests stay ordinary files | whoever finds a manifest root reads the whole list |
| **Always sealing members** | no choice to explain | a file public anyway loses its sources in the rest of the network |
| **A key per follower**, to remove one follower | revocable access | author wraps a key per follower, changes it on every removal. Per-follower access with epoch keys = paid subscription extension ([`subscription.md`](subscription.md)) |
| **User-typed password** instead of a random secret | easier to pass on | guessable offline by anyone holding a sealed file |
| **An authenticated cipher** | the usual choice | 16 bytes and a second check of what a signature or a signed root already fixes |

---

## 7. Open points

- Push for hidden sets: a [`pubsub.md`](pubsub.md) topic subscribers enter with a proof of the
  secret, as private-channel neighbours do ([`live.md`](live.md) §10.3). A paid hidden set has
  one: the `PAID` topic of [`subscription.md`](subscription.md) §8.4.
- Whether a day is the right period: shorter moves the record more often, costs followers more
  stores.
- A hidden set shares every set's open point: no successor for a lost key
  ([`file-set.md`](file-set.md) §10).
- All numbers are first values.
