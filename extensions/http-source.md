# Extension: HTTP sources (`http-source`)

**Status:** draft 0.1 (2026-10-11). Not implemented. Optional in wire version 1.
**Extends:** [`Protocol.md`](../Protocol.md) §10.3, §12.1, §12.5 and §13.2. Section numbers (§) refer to this document unless a document is named.
**Capability:** bit 9, `HTTP`. No message types; fields in existing messages.
**Compatible with:** eMuleQt's HTTP Cache for eD2K (`docs/protocol/http-cache-spec.md` in the eMuleQt repository), referred to below as *HTTP Cache*.

A file, or a range of one, can be fetched from an HTTP server as well as from peers. This
extension lets nodes tell each other where. It covers two kinds of source with one description:

- **Plain source.** A URL whose body is the file itself: a web server, a mirror, a content
  network. Anyone who knows such a URL can attach it to a file.
- **Sealed chunk.** A range of the file that an uploader encrypted and placed on a cache server,
  so that many downloaders fetch it over HTTP and the uploader spends its upstream once. This is
  HTTP Cache, carried over to ed3k with the same cipher and the same server contract.

HTTP is only a transport for bytes. Every byte is verified against the BLAKE3 root (`Protocol.md` §7.2) like
bytes from a peer, so a source cannot make a download accept wrong data.

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
| Body of the URL | the file bytes `offset .. offset+length` | the ciphertext of those bytes |
| Whole file | `offset` = 0, `length` = 0 | not used; a chunk is a range |
| Body length | `length` | `(length / 16 + 1) × 16`; not sent, the reader derives it |
| Who can read it | anyone with the URL | anyone with the URL and the key |
| `expires` | usually 0 | MUST be set |

- `url` and `hashes` are each at most 1,024 bytes, and at most 256 bytes in a Kad record.
- `alg` 1 is the scheme of HTTP Cache. Other values are reserved; a reader does not fetch a
  source with an `alg` it does not know, and reports it as `BAD_SOURCE` (§5).
- A sealed chunk uses a fresh random key and IV. A key is never used for two chunks.
- A sealed chunk covers at most 16 MiB (16,777,216 bytes) of the file, which is 4 pieces and a
  ciphertext of 16,777,232 bytes. A reader MUST refuse a sealed chunk with a larger `length`.
- A sealed chunk SHOULD start on a piece boundary and cover whole pieces (`Protocol.md` §7.2), or end with
  the file. The default is 4 pieces; a publisher uses fewer when the server accepts less (§4).
- A chunk made for eD2K covers one eD2K part of 9,728,000 bytes and is equally valid here; its
  edges are not block-aligned, which §6 covers.

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

A node sends these fields on a file stream only to a peer that set `HTTP`. A storing node keeps
and returns `Store.http` whether or not it sets the bit, checking the limits and nothing else,
as it does for every record of an extension (`Protocol.md` §10.3). A node without the extension ignores the
fields elsewhere (`Protocol.md` §16), so nothing else changes for it.

### 2.1 In the file stream

- An uploader MAY put sources into any `FileStatus`: the first answer, a ping, or the answer to
  a returning downloader (`Protocol.md` §12.5). A downloader waiting in the queue thereby gets bytes without a
  slot.
- An uploader that gives a downloader a source covering what it asked for MAY take its slot
  back and give it to a downloader the source does not help.
- The downloader reports the outcome in its next `FileRequest`:

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

  The codes and their numbers are those of HTTP Cache. Codes 8 to 10 belong to the extension
  for links at file hosters ([`hoster.md`](hoster.md) §2). After 3 *(initial)* results of 5, 6 or 7
  from downloaders in 3 different sites ([`ip-diversity.md`](ip-diversity.md) §1), an uploader
  stops handing out that source. `bytes` is reported with `OK` only: a count on a failure
  would tell the peer that named the URL how much of it exists.

### 2.2 In Kad

- A node that publishes itself as a source of a file (`Protocol.md` §10.3) MAY attach up to 3 HTTP sources to
  that `Store`. They live and expire with its source record.
- It attaches only sources it vouches for: sealed chunks it published itself, or plain sources
  from which it has verified at least one block.
- A storing node returns them in `KadAnswer.http` with their source record, when that record
  is in the answer (`Protocol.md` §10.3, **Paging**). It checks the limits of
  this document and nothing else; it does not fetch. A `Store` whose `http` holds more than 3
  sources, or a `url` or `hashes` above 256 bytes, is answered `REJECTED`, and its source
  record is not kept.
- A searcher can then download without contacting any peer.

### 2.3 Passing a source on

A node that fetched a source and verified every block of it under the root MAY hand the same
`HttpSource` to its own downloaders; one that verified only some blocks of a sealed chunk MUST
NOT, since it would be blamed for the rest (§6). There is no marker for this, and none is to be added: a passed-on description
is identical to a first-hand one, so it does not reveal who published a chunk.

A sealed chunk that is passed on keeps the expiry its publisher set, and is not attached to a
Kad record by anyone but its publisher.

## 3. Fetching

- Requests are `GET` with at most one `Range` per request and one request outstanding per
  connection.
- A plain source SHOULD support `Range`. One that answers 200 to a range request is read from
  the start of its body, or not used.
- A sealed chunk is read as in HTTP Cache: the SHA-256 of the whole ciphertext is checked
  against `digest`, the padding is checked, and the plaintext then goes to BLAKE3 verification.
  CBC allows resuming at any multiple of 16 bytes, with the preceding block as chaining value.
- A response is refused when its length or `Content-Range` disagrees with the description.
- Requests carry `Accept-Encoding: identity`. A response with a `Content-Encoding` other than
  `identity` is refused (`HTTP_FAILED`).
- Redirects are followed at most 3 times *(initial)*, and every target passes the checks of §5.
- A node SHOULD send a `User-Agent` that names the client, so that operators can let it pass.

**Verification.** Blocks are verified with the chaining values of `Protocol.md` §12.4, which are themselves
verified against the root. They come from a peer, or from a hash URL (§3.1). A downloader that
has neither can still fetch a whole file from a plain source and verify it by computing the
root at the end; a failure then cannot be located.

### 3.1 Block hashes over HTTP

`hashes`, or `wh` in a link, names a URL whose body is the hash tree of the file above its
blocks. A reader fetches the parts it needs with `Range` requests and can verify each part as
it arrives.

**The tree** is the one of `Protocol.md` §12.4. Let n = ceil(size / 256 KiB) be the number of blocks. Level 0
holds the chaining values of the blocks, the values `HashAnswer.cvs` carries. Each level above
halves the count:

```
count(0) = n                    count(k) = ceil(count(k−1) / 2)
node(k, j) = parent( node(k−1, 2j), node(k−1, 2j+1) )     if both exist
           = node(k−1, 2j)                                 if it is the last and has no partner
```

`parent` is BLAKE3's parent node function. The level with one node is the root, level L. It is
finalised with the `ROOT` flag and equals the file's root. This is the tree BLAKE3 itself builds
(`Protocol.md` §7.2); a node without a partner is carried up unchanged.

**The body.** Levels L−1 down to 0, in that order, each as its nodes in order, 32 bytes per
node. The root is not stored.

```
level L−1 ‖ level L−2 ‖ … ‖ level 1 ‖ level 0

offset of level k  =  32 × ( count(L−1) + … + count(k+1) )
length of the body =  32 × ( count(L−1) + … + count(0) )        about 64 bytes per block
```

| File size | Blocks | Body |
|---|---|---|
| 1 GiB | 4,096 | 256 KiB |
| 100 GiB | 409,600 | 25 MiB |

- A file of one block has no hash body; its root is its only hash.
- A body of another length than the one computed from the file size is refused.

**Reading.** A node of level k is **verified** when it and its partner give the verified node
above them, or when it has no partner and equals that node. The root the reader is downloading
is the starting point: the two nodes of level L−1 must give it.

- The reader fetches from the top down. The first bytes of the body are the top levels, so one
  request for a prefix verifies many levels at once.
- Below that, it fetches the range of a level that covers the blocks it is about to download.
  Ranges start at an even node index, so that partners arrive together.
- Level 0 is the last n × 32 bytes of the body. A reader that wants every block hash can fetch
  that range alone and build the tree itself.
- A hash URL needs no trust: a wrong body fails verification and the URL is dropped.
- The server MUST support `Range`. The URL passes the checks of §5 and counts towards the fetch
  limits like any other.

**Use.**

- The hashes belong to the file, not to one source. Any `HttpSource` of the file may carry the
  field, plain or sealed, and the values verify blocks from every source, peers included.
- A node SHOULD take block hashes from an ed3k source (`Protocol.md` §12.4) when it has one. A peer returns
  the values for a range together with their proof in one exchange, on a connection that is
  open anyway.
- A node that has no ed3k source SHOULD NOT fetch the hash body up front. It downloads from the
  plain source and checks the root when the file is complete, which costs no request. It
  fetches from the hash URL only if that check fails, to find the blocks that are wrong.
- A node MAY fetch hashes earlier when it needs verified blocks before the file is complete,
  for example to act as a source for the parts it has: unverified blocks MUST NOT be served.
  A `hashes` field does not change this. A node without an ed3k source fetches hashes before
  the file is complete only for blocks it is about to offer: once it stores a `SOURCE` record
  for the file in part (`Protocol.md` §10.3), or a downloader asks it for a range.
- The field is optional on both sides.

## 4. Cache servers

A sealed chunk is stored on a server that speaks the HTTP Cache contract: `POST` to store a
blob with a time to live, `GET` with `Range` to read it, `DELETE` to remove it. This extension
adds nothing to that contract and requires nothing else of a server.

- The contract sets no size limit. Each server states its own as `maxChunkSize` in `/v1/info`
  and answers 413 above it. A publisher reads that value and seals the largest whole number of
  pieces, up to 4, whose ciphertext fits. A server sized for exactly one eD2K part
  (9,728,016 bytes) takes 2 pieces.
- A server receives ciphertext only. It is given neither the root, the offset, nor the key.
- The same server serves eD2K and ed3k, and so does the same blob when it is cut as an eD2K
  part: the tags of an HTTP Cache offer map to `HttpSource` as below. A chunk cut in pieces is
  read by ed3k nodes only, since an eD2K reader accepts nothing but one part of 9,728,000 bytes.
- A node on both networks SHOULD cut a chunk the way that serves more of its waiting
  downloaders. It counts the queued downloaders that lack the range, per network. With more on
  eD2K it publishes the eD2K part and describes it in both networks; otherwise it publishes
  pieces and describes them in ed3k only. On a tie it publishes the eD2K part, which both can
  read.

| HTTP Cache offer | `HttpSource` |
|---|---|
| file hash, part index | the file of the stream or record; `offset` = part index × 9,728,000 |
| plain length | `length` |
| URL | `url` |
| key and IV | `sealed.key`, `sealed.iv` |
| cipher length | derived from `length` |
| SHA-256 of the ciphertext | `sealed.digest` |
| expiry | `expires` |

When an uploader publishes a chunk is local policy. HTTP Cache publishes a range once two or
more queued downloaders lack it and one of them is asking for it.

**Copies of one range.** A range that is already on a cache server needs no second upload,
except as a spare for a server that fails.

- A node SHOULD NOT publish a sealed chunk for a range while it knows 2 *(initial)* live
  sealed sources that cover it. A source is live until its `expires`; with `expires` = 0,
  until a fetch from it fails.
- When it knows 1, it SHOULD publish a second only on another server: one whose URL has a
  different host name and, where both are known, a different IP address. The host names
  compared are those of the sources' `url`. For a copy it posted itself, a node also takes
  the host of the server it posted to; a second copy goes to a server that differs from
  both.
- The rule counts sources from every origin: `FileStatus`, source exchange, Kad records and
  the node's own uploads.

### 4.1 Links a server shows

A server SHOULD show ed3k links, so that setting it up in a client is one click or one paste.

**A cache server** shows a link with its address and an upload key:

```
ed3k:cache?url=<url-encoded base URL>&key=<upload key>[&dn=<url-encoded name>][&kid=<key name>]
```

| Parameter | Meaning |
|---|---|
| `url` | base URL of the cache server, `http` or `https`, absolute, without user name, query or fragment |
| `key` | the secret that authorises uploads. Not empty, no white space |
| `dn` | a name for display |
| `kid` | a name for the key, for display |

- **The link is the credential.** Anyone who holds it can upload under that key. A client MUST
  ask the user before it applies such a link, and MUST NOT write the key to a log or show it in
  an error message.
- A link longer than 4,096 bytes, or with a `url` or `key` that breaks the rules above, or
  with `url` or `key` repeated, is refused as a whole. Unknown parameters are ignored.
- The link carries the same three things as HTTP Cache's own eD2K link (name, base URL,
  secret), so a server can show both for one account.

**A server that hosts a file** shows an `ed3k:file` link (`Protocol.md` §13.2) with itself as source:

```
ed3k:file?b3=<root>&sz=<size>&dn=<name>&ws=<URL of the file>[&wh=<URL of the hash body>]
```

No key is involved: a plain source is public, and the root in the link is what makes the
download verifiable.

## 5. Checks before fetching

A source is a URL chosen by someone else. A node fetches only when all of this holds:

| Check | Rule |
|---|---|
| Scheme | `http` or `https`, with a host; no user name or password in the URL |
| Target address | global unicast, for the first URL and for every redirect (below) |
| Range | `offset` and `length` lie inside the file and cover bytes still wanted. `offset` is below the file size, so a source of a file of size 0 never passes |
| Sealed | `length` from 1 to 16 MiB; `alg` known; key 32 B, IV 16 B, digest 32 B; `expires` set and at least 120 s *(initial)* ahead |
| Origin | from a peer: the peer is a source for that file on an open or recent file stream. From Kad or a link: the node itself chose the file |
| Load | below the node's limit of concurrent fetches; no other fetch running for the same range |

**Target address.** The address a fetch connects to MUST be global unicast as `Protocol.md`
§8.2 (7) lists it.

- A literal address is checked at once, a name after it is resolved, and again for every
  redirect. The fetch connects to the address that was checked, not to a second resolution of
  the name.
- `Alt-Svc` and similar hints are ignored. The fetch uses only the checked address.
- An IPv4-mapped address (`::ffff:0:0/96`) is normalised to IPv4 first and checked as IPv4.
- An address of NAT64 (`64:ff9b::/96`) or of 6to4 (`2002::/16`) is judged by the IPv4 address
  embedded in it: the last 32 bits of the first, bits 16 to 47 of the second. It is acceptable
  only when that IPv4 address is. A name that resolves to an inner address through such a
  prefix is refused like the inner address itself.
- **The one exception is a named network** (`net_id` not empty, `Protocol.md` §4.3): there the
  private ranges of `Protocol.md` §8.2 are allowed as well. In the public network they never
  are.
- **Through a proxy** the node resolves the name itself, checks the address, and passes the
  literal address to the proxy, with the original name in `Host` and, for `https`, as the
  server name of TLS (SNI) and in the check of the certificate. It follows redirects itself, so
  that every target is checked the same way. A node that cannot resolve the name itself, or
  whose proxy takes names only, does not use the source.

**What is reported.** A peer that named a URL MUST NOT learn from the result how a name
resolves where the downloader is. The result (§2.1) therefore depends on the check:

| Failed | `HttpResult.code` |
|---|---|
| Scheme, or the form of the URL: no host, a user name or password, longer than 1,024 bytes | `BAD_SOURCE` |
| Range: `offset` and `length` not inside the file, `offset` equal to the file size, or a file of size 0 | `BAD_SOURCE` |
| Range: inside the file, but not wanted | `NOT_WANTED` |
| Sealed: size, `alg`, the lengths of key, IV and digest, `expires` | `BAD_SOURCE` |
| Target address, of the first URL or of a redirect, literal or resolved | `HTTP_FAILED` |
| Scheme or form of a redirect target | `HTTP_FAILED` |
| Load | `BUSY` |
| Fetch limits: failed or pending fetches per file, per namer or per host | `BUSY` |

`BAD_SOURCE` is for what every node decides alike from the description alone. It says nothing
of the downloader's network. A refused address is reported like a server that could not be
reached.

**Fetch limits.** A false source costs a request to a host that did not ask for it. A node
therefore limits failed or pending fetches per file and hour to the values it uses for failed
source dials ([`ip-diversity.md`](ip-diversity.md) §2.5), counted by the address of the HTTP
server: 2 per host, 4 per site, 8 per block *(initial)*.

The same values bound failed or pending fetches **over all files** per hour. There they are
counted per pair of who named the URL and the host, site or block of the server. One URL
published under many files then costs its host no more than one file would, and a peer that
names dead URLs at a cache server or a content network uses up its own budget there, not that
of other sources.

Over all files and all namers together, a node allows at most 8 *(initial)* failed or pending
fetches per host and hour. Node IDs cost nothing, so the count per namer alone does not bound
what a host receives.

- **Who named the URL** is the peer that sent it, by node ID. For a source from a Kad answer,
  who named it is the pair of the target and the node that answered.
- A fetch is **pending** from the request until the response header arrived. After that it is
  a failure or it counts nothing.
- A fetch that was redirected counts at every address of the chain, the first included. Each
  address of the chain is checked against these limits before its request, and counts as
  pending from it, whether the target is a literal address or a resolved name.
- **Not counted over all files:** a fetch at a host that delivered a block which passed
  verification within the last hour, and a fetch at a gateway, a cache server or a link that
  the user configured or pasted. The same holds for the limit over all namers. The limit per
  file holds for these too.

## 6. Who is blamed

| Case | Blamed |
|---|---|
| Plain source, a block fails BLAKE3 | the URL: it is dropped. The node that named it is told `CORRUPT` and not banned, since it may have passed the URL on in good faith |
| Sealed chunk, ciphertext does not match `digest`, or bad padding | nobody: a broken server and a wrong description look the same. `CORRUPT` is reported |
| Sealed chunk, ciphertext matches `digest`, a block fails BLAKE3 | the peer that gave the description: it vouched for exactly these bytes. Treated like a peer that sent a corrupt block |
| Sealed chunk, a block fails BLAKE3 before the whole ciphertext was checked against `digest` | nobody, unless the downloader fetches the rest of the chunk and a row above applies |
| Sealed chunk, a block that lies partly outside the chunk fails | nobody: the describer vouched for the chunk's bytes only, and the rest of the block came from elsewhere. The block is fetched again from an ed3k source |
| Source from Kad or a link | nobody; the URL is dropped |

The cache server is never blamed for a sealed chunk. It holds bytes it cannot read, and the
digest proves they are the ones the publisher stored.

## 7. What it reveals

- **To the HTTP server:** the address of every node that fetches. For a plain source also which
  file; for a sealed chunk only that some chunk was fetched.
- **To anyone who has a sealed chunk's description:** the content of that chunk. The key travels
  with the URL, so the claim is only that the *server* is never given the key.
- **In Kad:** a description in a source record is public. For a sealed chunk that publishes the
  key; the chunk is one range of a file that is being shared anyway, and it expires. A node
  that wants keys to stay between it and its downloaders does not attach sealed chunks to Kad
  records.
- **Over `http`:** the request and a plain body are readable on the path. `https` is to be
  preferred; certificates are checked as a browser does.

## 8. Comparison

| | HTTP Cache (eD2K) | This extension | BitTorrent web seeds |
|---|---|---|---|
| Sources | sealed chunks only | sealed chunks and plain URLs | plain URLs |
| Unit | one part of 9,728,000 bytes | sealed: a range of up to 16 MiB, 4 pieces by default; plain: any range or the whole file | whole torrent |
| Verified by | MD4 part hash, later | BLAKE3 per 256 KiB block | piece hashes |
| Announced | opcode with tags; tags on the Kad source record | fields in the file stream, the Kad source record and links | in the metadata file |
| Result report | to the offering peer | the same | none |
| Peers without it | never sent the opcode | ignore the fields | ignore the keys |

## 9. Open points

- Test vectors for the hash body, for files of 2, 3, 5 and 4,096 blocks, are in
  [`tests/vectors-v1.md`](../tests/vectors-v1.md).
- A cipher that needs no padding and can be read at any offset (`alg` 2). It would remove the
  derived length and the chaining rule, at the cost of chunks that eD2K nodes cannot read.
- Whether a storing node should refuse a URL whose host is a literal private address. It cannot
  resolve names without doing the publisher's work.
- The limits of §2 and §5 are first values.
