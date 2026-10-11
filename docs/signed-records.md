# Signed Kad records: what signing gives, and what it does not

**Status:** design note (2026-10-11). Informative; the rules are in the main document.
**Relates to:** [`Protocol.md`](../Protocol.md) §10.3 and §10.7. Section numbers (§) refer to that document.

ed3k signs keyword and note records with the publisher's identity key and leaves source records
unsigned. This note records why the line is drawn there, what the signature is good for in spam
filtering, and why a record carries a *list* of signatures.

**Result:** sign claims about content (keywords, notes); do not sign claims about reachability
(sources), because a handshake proves those and a signature cannot.

## 1. What a signature proves

A signature proves that the holder of a key made a statement. It does not prove the statement.

| Record | Statement | Can a signature settle it? |
|---|---|---|
| `NOTE` | "I rate this file 4 and say this about it" | yes: the statement *is* the author's opinion |
| `KEYWORD` | "a file with this name, size and root exists" | partly: it fixes who claims it; the download verifies the rest |
| `SOURCE` | "the node with this ID is at this address and has the file" | no: a key can sign any address |

## 2. Sources: verified by dialling

A source record carries a node ID and an endpoint. When a searcher dials the endpoint, the
transport handshake proves that the node there holds the key behind that node ID (T1). A record
that is false in either part fails at that point and the source is dropped. This check happens
anyway, costs nothing extra, and is stronger than a signature over the record.

What signing sources would add:

- A storing node could not attribute an invented record to an *existing* node ID. The handshake
  already catches that.
- It would **not** stop a storing node from inventing sources. It generates a key, signs a
  record that points at a victim's address, and returns it. Every searcher then sends a
  connection attempt to the victim.

That reflection attack is real, and it is limited on the searcher's side, by counting failed
dials per host, site and block ([`ip-diversity.md`](../extensions/ip-diversity.md) §2.5). The
price of signing would have been about 85 bytes per source: a 32-byte key in place of the
16-byte ID, a 64-byte signature and an expiry. A 50-source answer would have grown from about
2 KB to about 6 KB.

## 3. Keywords and notes: what a client can do with an author

Every keyword and note record has an author: the node ID derived from the signing key.

| Use | How |
|---|---|
| One voice per author | Records from several storing nodes are merged by author and file. A storing node cannot multiply a rating or a file name by repeating it |
| Ignore an author | A user who meets one fake from an author can hide everything else that author published, now and later |
| Prefer known authors | Friends, and publishers whose earlier files verified, can be ranked first or shown as such |
| Integrity in transit | A storing node, an index node or a cache cannot change a name, a size or a rating. It can only withhold records |
| Pass records on | A record stays checkable when it is handed on by a third party |

**Limits**

- **Keys are free.** A spammer can use a new key for every record. Blocking authors is therefore
  weak; preferring known authors is what works. The key puzzle prices router identities only,
  and publishers are not routers.
- **A count of authors is not a count of people.** Popularity by number of signing keys can be
  inflated at no cost. The counting by site in the diversity limits (§2.4 there) is what makes
  inflation cost addresses, and only the storing node can do it, since only it sees the
  publishers' addresses.
- **A storing node can still lie by its own hand.** It can publish its own signed records under
  keys it made. The signature stops it from altering other people's records, not from adding
  its own.
- **Authorship is public and permanent.** Everything a node publishes is linkable to its node
  ID. In eD2K and Kad this is already so through the user hash and the address.

## 4. Cost

| | Size |
|---|---|
| Ed25519 signature | 64 B |
| Expiry, key index, wrappers | about 16 B |
| Publisher key | 34 B, once per publisher in an answer |
| Verification | on the order of 0.1 ms per record on one core |

A keyword result of 100–150 B grows by about 80 B. Signatures do not compress. `kind` and
`target` are signed but taken from the request, which saves 18 B per record.

## 5. Why a list of signatures

The record format holds a list of `{algorithm, signature}` pairs from the first version. A
second algorithm can then be added, required and, later, the first one retired, each by a
network upgrade, with no change to the record format and no second kind of record.

Validity is defined so that adding a signature never weakens a record: every signature the
verifier understands must verify, and the algorithms that are *required* must be present. An
attacker who strips an added signature gains nothing before that algorithm is required, and
produces an invalid record afterwards.

**What is deliberately left open**

- **Key binding.** Version 1 does not say how a key of another algorithm is tied to a node
  identity. The node ID stays the hash of the Ed25519 key. Whatever is chosen later (a
  certificate signed by the identity key, or a new node-ID derivation) belongs to the
  specification that registers the algorithm.
- **Size.** The signature schemes standardised against quantum computers are large: ML-DSA-44
  has 2,420-byte signatures and 1,312-byte keys, against 64 and 32 for Ed25519. One per record
  is not affordable, and the 1 KiB limit on a signed record does not admit it. Field 3 of
  `Signature` is reserved for a batch form, in which one signature covers a hash tree over many
  records and each record carries its path.

**Why this is not urgent for signatures.** A signature can only be forged once a capable quantum
computer exists; nothing recorded today is exposed by it. Encrypted traffic is different, since
it can be recorded now and decrypted later. That concerns the key exchange of the transport, not
this document.

## 6. Alternatives considered

- **Sign nothing** (the first draft). Records are claims relayed by storing nodes that may
  alter them, and a client has no notion of an author.
- **Sign sources too.** See §2.
- **Accept unsigned records and rank them lower.** Spam would be published unsigned, and every
  filter would have to handle two classes for good. With no installed base there is no need.
- **One signature field and a version number.** Adding an algorithm would then mean a second
  record format and a period in which storing nodes hold both.
- **A batch form from the first version.** One layout for all algorithms, at about 32 B per tree
  level on every record today and more code, for no gain while the only algorithm is Ed25519.

## 7. Open points

- A publisher whose clock is more than an hour ahead has its records rejected. Whether an error
  code should tell it so is undecided.
- Sizes in §4 are estimates from the encoding rules.
