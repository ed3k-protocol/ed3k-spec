# Extension: hidden sets (`hidden-set`)

**Status:** draft 0.1 (2026-10-12). Not implemented. Optional in wire version 1.
**Extends:** [`file-set.md`](file-set.md) §1, §3 and §5, and [`Protocol.md`](../Protocol.md) §10.1, §10.3 and §10.7. Section numbers (§) refer to this document unless a document is named.
**Capability:** none of its own; a node that uses it sets `SETS`. Record kind 8. One field in `Record`, one in `Member`, one parameter in `ed3k:set` links.
**Required of every `ROUTER`:** storing and returning the record kind `HIDDEN` (§3). The rest is optional.

A set that changes ([`file-set.md`](file-set.md) §5) can be found and read by everyone: its
head record names the manifest, and the manifest lists the files. A **hidden set** can be
found, read and followed only by those who were given its link.

- The link carries a **secret**. The head record and the manifest are encrypted with keys made
  from it.
- A storing node can **check** a head record and tell a newer from an older one. It cannot
  read it.
- The place of the head record in Kad **changes every day**.
- Members MAY be encrypted too (§4.2).

I2P does this for the record that leads to a hidden service
([`docs/i2p-zeronet-comparison.md`](../docs/i2p-zeronet-comparison.md) §2.3). Private live
channels ([`live.md`](live.md) §10) use the same construction for a stream.

---

## 1. The link

```
ed3k:set?key=<set key, base32 lower case, no padding>&s=<secret, 32 bytes, base32>[&dn=<url-encoded name>][&path=<url-encoded path>]
```

- A link with `s` names a hidden set. It has no `id` parameter; a link with both `id` and `s`
  is refused.
- **`key` is made for this set.** The author MUST NOT use its node key or the key of another
  set. One key is one hidden set.
- `secret` is 32 random bytes.
- `key` and `s` are base32 as `Protocol.md` §13.2 fixes: written lower case without padding,
  read either way.

**The link is the credential.** A client MUST NOT write the secret to a log or show it in an
error message, and SHOULD mark such a link as private wherever it shows it.

---

## 2. Keys

```
k       = H("ed3k v1 hidden set", secret ‖ key)                           32 bytes
k_head  = H("ed3k v1 hidden set head", k)                                 32 bytes
k_data  = H("ed3k v1 hidden set data", k)                                 32 bytes

base    = H("ed3k v1 hidden head", key)[0..16)
target  = H("ed3k v1 hidden head target", base ‖ period(8))[0..16)        period = Unix time / 86400, rounded down, big-endian
```

`base` and `target` need only the key. A storing node computes them. `k` and what follows from
it need the secret.

Sealed data is

```
sealed bytes = nonce(12) ‖ ChaCha20(key, nonce, plain bytes)              RFC 8439 cipher, counter from 0
```

with a random `nonce` for every use. There is no authentication tag: every sealed thing is
fixed by a signature or by a signed root.

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

- The record is a `Signed` of kind `HIDDEN`, signed with the set key. In the signature the
  place of `target` is taken by **`base`**, which does not change with the day. The author
  therefore signs once, and others can store the record on later days, until its `expires`.
- It travels in the fields of a `HEAD` record: `Store.head` with `Store.signer`, and
  `KadAnswer.sets`. `signer` is always present; a `Store` of kind `HIDDEN` without it is
  answered `REJECTED`.
- In the sealed `Head`, `id` and `seq` are those of the manifest, `root` and `size` those of
  the sealed manifest (§4.1), and `name` is empty.
- `data` is 12 to 256 bytes: the nonce and the sealed `Head`.

| | Storing node, on `Store` | Reader, on `KadAnswer` |
|---|---|---|
| Signature | valid under `signer`, with `base` computed from `signer` | valid under the key of the link |
| Target | the target of the request is the one of `signer` for the current day, the day before or the day after | the one it searched, and the one of the link's key for the current day, the day before or the day after |
| `expires` | later than now, at most 7 days *(initial)* plus 1 h ahead | later than now minus 1 h |
| `seq` | any; a record that does not replace the one held is answered `OLDER` | the highest of all answers wins |
| Count | one record per target. It is replaced by a record with a higher `seq`; by one with the same `seq` and a lower `data`, compared as a byte string; and by one with the same `seq` and `data` and a later `expires`; a valid record that does not replace it is answered `OLDER` | among equal `seq` the lower `data`, as on the storing node |
| Content | `sealed` and nothing else; else `REJECTED` | `data` opens to a `Head` whose `seq` is the one outside and whose `root` is 32 B |
| Size | `data` 12 to 256 B; else `REJECTED` | the same |

- A storing node deletes the record at `expires`, and at the latest 48 h after the day of its
  target began.
- **Only the author can make a record.** The target follows from the key that signs, so nobody
  can take the place first with a record under a key of their own.
- An author MUST store the same sealed bytes each time it stores one version again: a fresh
  nonce would change `data`, and with it the tie-break between two records of one `seq`.

**Storing**

- The author stores the record when it publishes a version.
- A follower stores the record it holds under the target of the current day, once per day, at
  a random time in the first 6 h *(initial)* of the day. The author need not be online.
- A storing node of one day is, as a rule, not one of the next.

**Reading**

1. Search `HIDDEN` at the targets of the current day and of the day before, and merge the
   answers: a follower's store of an old version on the new day must not hide a newer one that
   is still under yesterday's target.
2. Take the record with the highest `seq` and open `data`.
3. Download the sealed manifest by its `root`, open it (§4.1), and accept it by the rules of
   [`file-set.md`](file-set.md) §5.3: signed by the set key, with the `seq` and the `id` of
   the opened `Head`, and with the `id` of the version held before, if there is one.

A follower checks as often as for any set (`file-set.md` §5.3). There is no topic for a hidden
set in [`pubsub.md`](pubsub.md) in this version, except that of a paid one
([`subscription.md`](subscription.md) §8.4).

---

## 4. What is sealed

### 4.1 Manifests

- The manifest is written as in [`file-set.md`](file-set.md) §1, signed (§2.4 there), with an
  `id` and a `seq`. It is then sealed with `k_data`. What is shared is the sealed file.
- It is signed by the set key alone. Co-owners are not defined for a hidden set
  ([`multi-author-set.md`](multi-author-set.md) §7), so a reader refuses a hidden manifest
  that carries a `grant`. Parts and mounts are not defined for it either: a hidden manifest
  with `open`, or with a member that has `mount` (`multi-author-set.md` §2, §3.1), is refused
  too.
- An inner manifest (`file-set.md` §2.5) of a hidden set is sealed the same way. Its `root` and
  `size` in the outer manifest are those of the sealed file.
- A sealed manifest does not start with the 8 bytes of a manifest. A client knows what it is
  from the head record or from the outer manifest.

### 4.2 Members

```protobuf
// in Member (file-set.md §1):
bool sealed = 9;              // the shared file is sealed with k_data; root and size are those of the sealed file
```

- A member without `sealed` is an ordinary file. Everyone who knows its root can download it,
  and it is the same file as any other copy in the network.
- A member with `sealed` is stored and shared in sealed form. A client opens it after the
  download, or while reading: the cipher allows reading from any position.
- A sealed member is at most 256 GiB, the limit of the cipher for one nonce. A reader refuses a
  manifest that lists a larger one.
- Sealed data does not compress; the rule of [`Protocol.md`](../Protocol.md) §12.3 then sends
  it as it is.

| | Open member | Sealed member |
|---|---|---|
| Who can read it | everyone with the root | only who has the link |
| Sources | all sources of that file in the network | only nodes that hold this set |
| What others see | a known file at a node | data without a name |
| Cost | none | 12 bytes, and the client opens it before use |

### 4.3 Rules for a node that holds a hidden set

- It publishes a `SOURCE` record for the sealed manifest and for each sealed member it shares.
  It MUST NOT publish `KEYWORD` or `MEMBER` records for them, and does not list them in
  `Browse`.
- For an open member it publishes nothing because of the hidden set. If the node shares that
  file for another reason, the usual rules apply.
- A sealed file is named on the wire, in `FileStatus.name` (`Protocol.md` §12.1), by the first
  16 characters of its root in base32, lower case (`Protocol.md` §13.2). The names in the
  manifest stay inside it.
- The member bitmap ([`file-set.md`](file-set.md) §4.1) works on the root of the sealed
  manifest as on any manifest. Whoever knows that root learns which members a node holds, and
  nothing about what they are.
- A reader asks the sources of the manifest for the members first (`file-set.md` §3).

---

## 5. What it does and does not give

| | |
|---|---|
| Someone without the link | cannot find the set, read its list, or read a sealed member |
| A node that stores the head record | learns the set key, the version number, and the addresses of those who store it there. It cannot open the record |
| A node that stored it once | can compute where the record is on any later day, and search it there. It still cannot open it |
| A node that stores `SOURCE` records | learns that some nodes share a file without a name |
| A follower who passes the link on | gives full access. Followers are not told apart |
| Taking access away | not possible. A new secret is a new set, and every follower needs the new link. A paid hidden set gives each follower a pass of their own that ends ([`subscription.md`](subscription.md) §8.3) |
| A secret that leaks later | opens every version that someone kept. There is no forward secrecy |
| Open members | are as visible as any shared file. Only that they belong together is hidden |
| The author | is not named by the `HIDDEN` record when it stores through `signer` like any follower. Its `SOURCE` records for the sealed manifest (§4.3) are bound to its node ID like any source record, so a storing node sees who first offers a new version |

---

## 6. Alternatives considered

| Alternative | Pro | Con |
|---|---|---|
| **Key blinding**, as I2P and Tor use it: the record is signed with a key derived from the set key and the day | a storing node never learns the set key, so it cannot compute the target of another day | the author has to sign every day and so must be online; signing with a derived key is outside the standard Ed25519 interface of most libraries |
| **A fixed target** | simpler; nothing to store again each day | the same few nodes see every follower for as long as the set exists, and anyone can place nodes there |
| **A target from the secret**, which a storing node cannot check | a storing node learns nothing at all | anyone who knows the target can put a record of its own there first, and the real one finds no room |
| **Sealing the head only** | manifests stay ordinary files | whoever comes across the root of a manifest reads the whole list |
| **Always sealing members** | no choice to explain | a file that is public anyway loses its sources in the rest of the network |
| **A key per follower**, so one follower can be removed | access can be taken away | the author wraps a key for every follower and changes it on every removal. Per-follower access with epoch keys is the paid subscription extension ([`subscription.md`](subscription.md)) |
| **A password typed by the user** in place of a random secret | easier to pass on | can be guessed off line by anyone who holds a sealed file |
| **An authenticated cipher** | the usual choice | 16 bytes and a second check of what a signature or a signed root already fixes |

---

## 7. Open points

- Push for hidden sets: a topic in [`pubsub.md`](pubsub.md) that subscribers enter with a
  proof of the secret, as the neighbours of a private channel do ([`live.md`](live.md) §10.3).
  A paid hidden set has one: the `PAID` topic of [`subscription.md`](subscription.md) §8.4.
- Whether the day is the right period. A shorter one moves the record more often and costs
  followers more stores.
- A hidden set has the open point of every set: no successor for a lost key
  ([`file-set.md`](file-set.md) §10).
- All numbers are first values.
