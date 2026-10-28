# Extension: HTTP sources (`http-source`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3, §12.1, §12.5 and §13.2. § = this document unless a document is named.
**Capability:** bit 9, `HTTP`. No message types; fields in existing messages.
**Required of every `ROUTER`:** keeping and returning `Store.http` (§2), whether or not it sets the bit. The rest is optional.
**Compatible with:** eMuleQt's HTTP Cache for eD2K (`docs/protocol/http-cache-spec.md` in the eMuleQt repo), below *HTTP Cache*.

A file or range can be fetched over HTTP as well as from peers; nodes tell each other where. One
description, two source kinds:

- **Plain source.** URL whose body is the file itself: web server, mirror, content network.
  Anyone knowing it can attach it to a file.
- **Sealed chunk.** File range an uploader encrypted and put on a cache server, so many
  downloaders fetch it over HTTP and the uploader spends upstream once. HTTP Cache carried to
  ed3k: same cipher, same server contract.

HTTP only transports bytes. Every byte is verified against the BLAKE3 root (`Protocol.md` §7.2)
like peer bytes, so no source can make a download accept wrong data.

## 1. Description of a source

```protobuf
message HttpSource {
  string url = 1;             // absolute http or https URL
  uint64 offset = 2;          // position in the file of the first byte this source holds
  uint64 length = 3;          // number of file bytes it holds; 0 = up to the end of the file
  uint64 expires = 4;         // Unix time after which the URL may be gone; 0 = unknown
  SealedChunk sealed = 5;     // absent = plain source
  string hashes = 6;          // optional: URL of the file's hash body (§3.1)
}

message SealedChunk {         // not `Sealed`, the record of hidden-set.md
  uint32 alg = 1;             // 1 = AES-256-CBC with PKCS#7 padding, SHA-256 over the ciphertext
  bytes  key = 2;             // 32 B
  bytes  iv = 3;              // 16 B
  bytes  digest = 4;          // 32 B: hash of the whole ciphertext
}
```

| | Plain source | Sealed chunk |
|---|---|---|
| URL body | file bytes `offset .. offset+length` | their ciphertext |
| Whole file | `offset` = 0, `length` = 0 | not used; chunk is a range |
| Body length | `length` | `(length / 16 + 1) × 16`; not sent, reader derives it |
| Readable by | anyone with the URL | anyone with URL and key |
| `expires` | usually 0 | MUST be set |

- `url`, `hashes`: each ≤ 1,024 bytes; ≤ 256 bytes in a Kad record.
- `alg` 1 = HTTP Cache scheme; others reserved. Unknown `alg` → reader doesn't fetch, reports
  `BAD_SOURCE` (§5).
- Sealed chunk: fresh random key and IV; a key never serves two chunks.
- Sealed chunk covers ≤ 16 MiB (16,777,216 bytes) of the file, i.e. 4 pieces, ciphertext 16,777,232
  bytes. A reader MUST refuse a sealed chunk with larger `length`.
- A sealed chunk SHOULD start on a piece boundary and cover whole pieces (`Protocol.md` §7.2),
  or end with the file. Default 4 pieces; fewer when the server accepts less (§4).
- A chunk made for eD2K covers one eD2K part of 9,728,000 bytes, equally valid here; edges not
  block-aligned (§6).

## 2. Where sources travel

| Carrier | Field | Limit |
|---|---|---|
| Uploader to downloader | `FileStatus.http` = 9, `repeated HttpSource` | 8 per message *(initial)* |
| Downloader to uploader | `FileRequest.http_results` = 7, `repeated HttpResult` | one per source used |
| Kad source record, stored | `Store.http` = 4, `repeated HttpSource` | 3 per publisher and file |
| Kad source search, answered | `KadAnswer.http` = 13, `repeated HttpSource` | 16 per answer *(initial)* |
| Link | `ws=<url>` in `ed3k:file` (`Protocol.md` §13.2) | plain, whole file |
| Link | `wh=<url>` in `ed3k:file` (`Protocol.md` §13.2) | hash body of the file (§3.1) |
| Link | `ed3k:cache` | a cache server and its upload key (§4.1) |

Fields sent on a file stream only to peers that set `HTTP`. Storing node keeps and returns
`Store.http` whether or not it sets the bit, checking limits only, as for every extension record
(`Protocol.md` §10.3). Nodes without the extension ignore the fields elsewhere (`Protocol.md`
§16); nothing else changes for them.

### 2.1 In the file stream

- Uploader MAY put sources in any `FileStatus`: first answer, ping, or answer to a returning
  downloader (`Protocol.md` §12.5). Queued downloader thus gets bytes w/o a slot.
- Uploader giving a downloader a source covering its request MAY reassign that slot to a
  downloader the source doesn't help.
- Downloader reports outcome in its next `FileRequest`:

```protobuf
message HttpResult {
  bytes  url_id = 1;          // H("ed3k v1 http", url)[0..8)
  Code   code = 2;
  uint64 bytes = 3;           // bytes fetched

  enum Code {
    OK = 0;                   // fetched and verified
    DISABLED = 1;             // downloads over HTTP are switched off
    BUSY = 2;                 // at the limit of concurrent fetches
    NOT_WANTED = 3;           // range not needed (any more)
    BAD_SOURCE = 4;           // failed a check of §5 on the description itself: scheme, form of the URL, range, sealed fields
    HTTP_FAILED = 5;          // transport failure, a status that is not 200 or 206, or a target address refused by §5
    SIZE_MISMATCH = 6;        // the server sent another length than described
    CORRUPT = 7;              // digest, padding or BLAKE3 verification failed
  }
}
```

  Codes and numbers = HTTP Cache's. Codes 8 to 10: file-hoster links extension
  ([`hoster.md`](hoster.md) §2). After 3 *(initial)* results of 5, 6 or 7 from downloaders in 3
  different sites ([`ip-diversity.md`](ip-diversity.md) §1), uploader stops handing out that
  source. `bytes` reported with `OK` only: on failure it would tell the URL's namer how much
  exists.

### 2.2 In Kad

- A node publishing itself as file source (`Protocol.md` §10.3) MAY attach up to 3 HTTP sources
  to that `Store`; they live and expire with its source record.
- Only ones it vouches for: own published sealed chunks, or plain sources it verified ≥ 1 block
  from.
- Storing node returns them in `KadAnswer.http` with their source record, when that is in the
  answer (`Protocol.md` §10.3, **Paging**). Checks this document's limits only; never fetches.
  `Store` with more than 3 sources in `http`, or `url` or `hashes` above 256 bytes → `REJECTED`, source
  record not kept.
- Searcher can then download without contacting any peer.

### 2.3 Passing a source on

A node that fetched a source and verified every block under the root MAY hand the same
`HttpSource` to its own downloaders; one that verified only some blocks of a sealed chunk MUST
NOT, as it would be blamed for the rest (§6). No marker for this, and none to be added:
passed-on is identical to first-hand, so it doesn't reveal who published a chunk.

Passed-on sealed chunk keeps its publisher's expiry; nobody but its publisher attaches it to a
Kad record.

## 3. Fetching

- `GET`, ≤ 1 `Range` per request, one request outstanding per connection.
- Plain source SHOULD support `Range`. One answering 200 to a range request is read from body
  start, or not used.
- Sealed chunk read as in HTTP Cache: SHA-256 of whole ciphertext checked against `digest`,
  padding checked, plaintext then to BLAKE3 verification. CBC allows resuming at any multiple of
  16 bytes, preceding block as chaining value.
- Response refused when its length or `Content-Range` disagrees with the description.
- Requests carry `Accept-Encoding: identity`; response with `Content-Encoding` other than
  `identity` refused (`HTTP_FAILED`).
- ≤ 3 redirects followed *(initial)*; every target passes §5 checks.
- A node SHOULD send a `User-Agent` naming the client, so operators can let it pass.

**Verification.** Blocks verified with chaining values of `Protocol.md` §12.4, themselves
verified against the root; from a peer or a hash URL (§3.1). With neither, a downloader can still fetch a
whole file from a plain source and verify by computing the root at the end; a failure then
can't be located.

### 3.1 Block hashes over HTTP

`hashes`, or `wh` in a link, names a URL whose body is the file's hash tree above its blocks.
Reader fetches needed parts by `Range` and can verify each on arrival.

**The tree** is that of `Protocol.md` §12.4. n = ceil(size / 256 KiB) = block count. Level 0 = block
chaining values (what `HashAnswer.cvs` carries). Each level above halves the count:

```
count(0) = n                    count(k) = ceil(count(k−1) / 2)
node(k, j) = parent( node(k−1, 2j), node(k−1, 2j+1) )     if both exist
           = node(k−1, 2j)                                 if it is the last and has no partner
```

`parent` = BLAKE3's parent node function. The one-node level is the root, level L, finalised
with `ROOT` flag; equals the file's root. This is the tree BLAKE3 itself builds (`Protocol.md`
§7.2); a partnerless node is carried up unchanged.

**The body.** Levels L−1 down to 0, in that order, nodes in order, 32 bytes each. Root not
stored.

```
level L−1 ‖ level L−2 ‖ … ‖ level 1 ‖ level 0

offset of level k  =  32 × ( count(L−1) + … + count(k+1) )
length of the body =  32 × ( count(L−1) + … + count(0) )        about 64 bytes per block
```

| File size | Blocks | Body |
|---|---|---|
| 1 GiB | 4,096 | 256 KiB |
| 100 GiB | 409,600 | 25 MiB |

- One-block file: no hash body; root is its only hash.
- Body of another length than the one computed from file size → refused.

**Reading.** Level-k node **verified** when it and its partner give the verified node above, or
it has no partner and equals that node. Start: the root being downloaded; the two level L−1
nodes must give it.

- Fetch top down: body starts with the top levels, so one prefix request verifies many.
- Below, fetch a level's range covering blocks about to be downloaded; ranges start at an even
  node index so partners arrive together.
- Level 0 = last n × 32 bytes of the body. A reader wanting every block hash can fetch just that
  and build the tree itself.
- A hash URL needs no trust: wrong body fails verification, URL dropped.
- Server MUST support `Range`. URL passes §5 checks and counts towards fetch limits like any
  other.

**Use.**

- Hashes belong to the file, not one source. Any `HttpSource` of the file, plain or sealed, may
  carry the field; values verify blocks from every source, peers included.
- A node SHOULD take block hashes from an ed3k source (`Protocol.md` §12.4) if it has one: peer
  returns a range's values with proof in one exchange, on an already open connection.
- A node with no ed3k source SHOULD NOT fetch the hash body up front: it downloads from the
  plain source and checks the root on completion (no request); fetches from the hash URL only
  if that fails, to find the wrong blocks.
- A node MAY fetch hashes earlier when it needs verified blocks before completion, e.g. to be a
  source for parts it has: unverified blocks MUST NOT be served. A `hashes` field doesn't change
  this. W/o ed3k source, a node fetches hashes before completion only for blocks it is about to
  offer: once it stores a `SOURCE` record for the partial file (`Protocol.md` §10.3), or a
  downloader asks it for a range.
- Field optional on both sides.

## 4. Cache servers

Sealed chunks are stored on a server speaking the HTTP Cache contract: `POST` stores a blob
with TTL, `GET` with `Range` reads, `DELETE` removes. This extension adds nothing to it and
requires nothing else of a server.

- No size limit in the contract; each server states its own as `maxChunkSize` in `/v1/info`,
  answers 413 above it. Publisher reads it and seals the largest whole number of pieces, up to 4, whose
  ciphertext fits. Server sized for exactly one eD2K part (9,728,016 bytes) takes 2 pieces.
- Server gets ciphertext only; never root, offset or key.
- One server serves eD2K and ed3k, as does one blob cut as an eD2K part: HTTP Cache offer tags
  map to `HttpSource` as below. Chunk cut in pieces: read by ed3k nodes only, as an
  eD2K reader accepts nothing but one part of 9,728,000 bytes.
- A node on both networks SHOULD cut a chunk the way serving more waiting downloaders: counts
  queued downloaders lacking the range, per network. More on eD2K → publish eD2K part, describe
  in both networks; else publish pieces, describe in ed3k only. Tie → eD2K part (both read it).

| HTTP Cache offer | `HttpSource` |
|---|---|
| file hash, part index | the file of the stream or record; `offset` = part index × 9,728,000 |
| plain length | `length` |
| URL | `url` |
| key and IV | `sealed.key`, `sealed.iv` |
| cipher length | derived from `length` |
| SHA-256 of the ciphertext | `sealed.digest` |
| expiry | `expires` |

When an uploader publishes a chunk: local policy. HTTP Cache publishes a range once ≥ 2 queued
downloaders lack it and one of them asks for it.

**Copies of one range.** A range already on a cache server needs no second upload, except as
spare for a failing server.

- A node SHOULD NOT publish a sealed chunk for a range while it knows 2 *(initial)* live sealed
  sources covering it. Live until `expires`; with `expires` = 0, until a fetch from it fails.
- Knowing 1, it SHOULD publish a second only on another server: different host name (of the
  sources' `url`) and, where both known, different IP address. For a copy it posted itself, also
  the host it posted to; a second copy goes to a server differing from both.
- Counts sources of every origin: `FileStatus`, source exchange, Kad records, own uploads.

### 4.1 Links a server shows

A server SHOULD show ed3k links, so client setup is one click or paste.

**A cache server** shows a link with its address and an upload key:

```
ed3k:cache?url=<url-encoded base URL>&key=<upload key>[&dn=<url-encoded name>][&kid=<key name>]
```

| Parameter | Meaning |
|---|---|
| `url` | cache server base URL, `http` or `https`, absolute, no user name, query or fragment |
| `key` | secret authorising uploads. Not empty, no white space |
| `dn` | display name |
| `kid` | key name, for display |

- **The link is the credential**: whoever holds it can upload under that key. A client MUST ask
  the user before applying it, and MUST NOT write the key to a log or show it in an error
  message.
- Link longer than 4,096 bytes, or `url`/`key` breaking the rules above, or `url` or `key` repeated →
  refused whole. Unknown parameters ignored.
- Same three things as HTTP Cache's own eD2K link (name, base URL, secret), so a server can show
  both for one account.

**A server hosting a file** shows an `ed3k:file` link (`Protocol.md` §13.2) with itself as source:

```
ed3k:file?b3=<root>&sz=<size>&dn=<name>&ws=<URL of the file>[&wh=<URL of the hash body>]
```

No key: plain source is public; the link's root makes the download verifiable.

## 5. Checks before fetching

A source is a URL someone else chose. Fetch only when all hold:

| Check | Rule |
|---|---|
| Scheme | `http` or `https`, with host; no user name or password in the URL |
| Target address | global unicast, for first URL and every redirect (below) |
| Range | `offset` and `length` inside the file, covering bytes still wanted. `offset` is below file size, so a size-0 file's source never passes |
| Sealed | `length` 1 to 16 MiB; `alg` known; key 32 B, IV 16 B, digest 32 B; `expires` set, ≥ 120 s *(initial)* ahead |
| Origin | from a peer: peer is a source of that file on an open or recent file stream. From Kad or link: node itself chose the file |
| Load | below node's concurrent-fetch limit; no other fetch running for the same range |

**Target address.** The address a fetch connects to MUST be global unicast as `Protocol.md`
§8.2 (7) lists it.

- Literal address checked at once, name after resolving, again per redirect. Connect to the checked
  address, never a re-resolution. Address refused (address check or fetch limits) → fetch
  ends; no other address of the name tried.
- `Alt-Svc` and similar hints ignored; only the checked address used.
- IPv4-mapped (`::ffff:0:0/96`) normalised to IPv4 first, checked as IPv4.
- NAT64 (`64:ff9b::/96`) or 6to4 (`2002::/16`): judged by embedded IPv4 — last 32 bits of the
  first, bits 16 to 47 of the second; acceptable only if that IPv4 is. A name resolving to an
  inner address via such a prefix is refused like the inner address.
- **The one exception is a named network** (`net_id` not empty, `Protocol.md` §4.3): private
  ranges of `Protocol.md` §8.2 allowed too. Never in the public network.
- **Through a proxy**: node resolves and checks itself, passes the literal to the proxy, original
  name in `Host` and, for `https`, as TLS server name (SNI) and in the certificate check.
  Follows redirects itself so every target is checked alike. Node unable to resolve itself, or whose
  proxy takes names only → source not used.

**What is reported.** A peer that named a URL MUST NOT learn from the result how a name resolves
at the downloader. So the result (§2.1) depends on the check:

| Failed | `HttpResult.code` |
|---|---|
| Scheme, or URL form: no host, user name or password, longer than 1,024 bytes | `BAD_SOURCE` |
| Range: `offset` and `length` not inside the file, `offset` = file size, or size-0 file | `BAD_SOURCE` |
| Range: inside the file, but not wanted | `NOT_WANTED` |
| Sealed: size, `alg`, key/IV/digest lengths, `expires` | `BAD_SOURCE` |
| Target address, first URL or redirect, literal or resolved | `HTTP_FAILED` |
| Scheme or form of a redirect target | `HTTP_FAILED` |
| Load | `BUSY` |
| Fetch limits: failed or pending fetches per file, per namer or per host | `BUSY` |

`BAD_SOURCE` = what every node decides alike from the description alone; says nothing of the
downloader's network. Refused address reported like an unreachable server.

**Fetch limits.** A false source costs a request to a host that didn't ask. So failed or pending
fetches per file and hour are limited to the node's values for failed source dials
([`ip-diversity.md`](ip-diversity.md) §2.5), counted by HTTP server address: 2 per host, 4 per
site, 8 per block *(initial)*.

Same values bound failed or pending fetches **over all files** per hour, per pair (URL namer,
server host/site/block). So one URL under many files costs its host no more than one file; a
peer naming dead URLs at a cache server or content network spends its own budget there, not
other sources'.

Over all files and namers together: ≤ 8 *(initial)* failed or pending fetches per host and hour.
Node IDs cost nothing, so per-namer counts alone don't bound a host's load.

- **Who named the URL**: sending peer, by node ID. Kad-answer source: pair (target, answering
  node).
- **Pending**: from request until response header arrives; then failure or nothing.
- Redirected fetch counts at every chain address, first included; each checked against these
  limits before its request, pending from it, literal or resolved. A chain
  ended by a later address (refused by these limits or the address check) is a failure at every
  address before it.
- **Not counted over all files:** fetch at a host that delivered a block passing verification
  within the last hour, and fetch at a user-configured or pasted gateway, cache server or link.
  Same for the all-namers limit. Per-file limit holds for these too.

## 6. Who is blamed

| Case | Blamed |
|---|---|
| Plain source, a block fails BLAKE3 | the URL: dropped. Namer told `CORRUPT`, not banned: may have passed it on in good faith |
| Sealed chunk, ciphertext does not match `digest`, or bad padding | nobody: broken server and wrong description look alike. `CORRUPT` reported |
| Sealed chunk, ciphertext matches `digest`, a block fails BLAKE3 | peer that gave the description: vouched for exactly these bytes. Treated like a peer sending a corrupt block |
| Sealed chunk, a block fails BLAKE3 before the whole ciphertext was checked against `digest` | nobody, unless downloader fetches the rest of the chunk and a row above applies |
| Sealed chunk, a block that lies partly outside the chunk fails | nobody: describer vouched for chunk bytes only; rest came from elsewhere. Block re-fetched from an ed3k source |
| Source from Kad or a link | nobody; URL dropped |

Cache server never blamed for a sealed chunk: holds bytes it can't read; digest proves they are
what the publisher stored.

## 7. What it reveals

- **To the HTTP server:** every fetching node's address. Plain source: also which file; sealed:
  only that some chunk was fetched.
- **To anyone with a sealed chunk's description:** its content; key travels with the URL. The
  only claim: the *server* never gets the key.
- **In Kad:** a source-record description is public; for a sealed chunk that publishes the key.
  Chunk = one range of a file shared anyway, and expires. A node wanting keys kept between it
  and its downloaders doesn't attach sealed chunks to Kad records.
- **Over `http`:** request and plain body readable on the path. `https` preferred; certificates
  checked as a browser does.

## 8. Comparison

| | HTTP Cache (eD2K) | This extension | BitTorrent web seeds |
|---|---|---|---|
| Sources | sealed chunks only | sealed chunks and plain URLs | plain URLs |
| Unit | one part of 9,728,000 bytes | sealed: range ≤ 16 MiB, 4 pieces by default; plain: any range or whole file | whole torrent |
| Verified by | MD4 part hash, later | BLAKE3 per 256 KiB block | piece hashes |
| Announced | opcode with tags; tags on Kad source record | fields in file stream, Kad source record, links | in metadata file |
| Result report | to offering peer | same | none |
| Peers without it | never sent the opcode | ignore the fields | ignore the keys |

## 9. Open points

- Hash body test vectors, files of 2, 3, 5 and 4,096 blocks:
  [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- Cipher w/o padding, readable at any offset (`alg` 2): would drop derived length and chaining rule,
  but eD2K nodes can't read such chunks.
- Whether a storing node should refuse a URL whose host is a literal private address. It can't resolve
  names without doing the publisher's work.
- Limits of §2 and §5 are first values.
