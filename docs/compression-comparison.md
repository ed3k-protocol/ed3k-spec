# Compression: zstd for messages and file blocks, and the alternatives

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §5.3 (messages) and §12.3 (file blocks). Section numbers (§) refer to that document.

ed3k compresses frame bodies with zstd and a shared dictionary. This note records why, against the
other codecs that were considered.

**Result:** zstd with a trained dictionary. It is the only candidate that gains on the bodies ed3k
actually sends, which are mostly under 1 KiB.

The ratios below are expectations from the codecs' published behaviour on small inputs, not
measurements on ed3k traffic. The dictionary does not exist yet (Appendix B); the comparison
should be repeated on the message corpus that the dictionary is trained on.

## 1. What is being compressed

- Most frames are small: a `KadRequest`, a `FileRequest`, a `Hello` are tens to a few hundred bytes.
- Much of a body is incompressible: hashes, node IDs, public keys.
- The compressible part is repetitive across messages, not within one: field tags, common file
  extensions and name fragments, tag names, address prefixes.
- The large bodies are lists: search results, source lists, shared-file lists.
- File data is not a frame body. It is treated in section 5.

A codec that only finds repetition inside one body therefore gains little. One that starts with
knowledge of typical messages gains a lot. That is what a dictionary provides.

## 2. Comparison

| | Small-message ratio | Speed | Notes |
|---|---|---|---|
| **zstd + dictionary** | best: a trained dictionary gives ×2–×5 on bodies under 1 KiB | very fast both ways | chosen; a hard dependency, dictionary frozen per ID |
| zstd without dictionary | poor under ~500 B | very fast | header cost eats the gain on typical messages |
| deflate (zlib) | poor without, fair with a preset dictionary (32 KiB limit) | slower, larger header | what eD2K uses; no reason to keep it in a new format |
| Brotli | good on text thanks to its built-in web dictionary, which does not match this traffic | slow to compress | larger library |
| LZ4 | weak | fastest | wins nothing on bodies this small |
| none | — | — | search results, source lists and shared-file lists stay 2–4 times larger |

## 3. Costs of the choice

- **A hard dependency.** Every implementation needs a zstd decoder with dictionary and magicless
  frame support.
- **A frozen dictionary.** `ed3k-v1` (ID 1) can never change once deployed. A better one needs a
  new ID, and every node must ship both for as long as old senders exist. Nodes announce the
  highest ID they hold in `Hello` (§5.3), so a new dictionary needs no flag day.
- **A corpus is needed first.** The dictionary has to be trained on real messages before the format
  is frozen, so it depends on a working implementation.

## 4. Required support, optional use

Making *support* mandatory removes negotiation and every "does the peer do compression" variant.
Making *use* mandatory was rejected: it would enlarge the many messages that are mostly hashes and
keys. A sender compresses only when the result is smaller and does not try under 64 bytes.

## 5. File blocks

A block of file data may be sent as one zstd frame (§12.3). An earlier state of this note
excluded file data; it was added on 2026-10-11. The figures in this section are from the codecs' published
behaviour, not from measurements on shared files.

### 5.1 Why at all

- Most of the volume is video, audio and archives and gains nothing.
- Some files gain a factor of 2 to 5: text, subtitles, source trees in uncompressed containers,
  database dumps, disk images, uncompressed executables, and file set manifests.
- The decoder is required anyway (§5.3), so the cost is one field in `Block`.
- A relay (§9.4) carries less for a compressed block.

### 5.2 Constraints on the codec

- **Blocks stand alone.** A downloader asks for blocks in any order and from several sources, so
  no context can be carried from one block to the next. No codec sees more than 256 KiB.
- **Compression happens during the upload.** The sender cannot spend seconds per block.
- **One more decoder is a cost for every implementation**, as in section 3.

| | Ratio at 256 KiB | Speed | Verdict |
|---|---|---|---|
| **zstd, no dictionary** | good; the level is the sender's choice | compresses and decompresses far faster than any upload link | chosen; already required |
| LZ4 | clearly weaker | fastest | speed is not the limit, so it only loses ratio |
| deflate | weaker than zstd at equal speed | slower | what eD2K uses for parts |
| Brotli | close to zstd; better on text at its highest levels | highest levels far too slow for an upload | a second decoder for a small gain |
| xz (LZMA) | best | slow both ways | its advantage comes from large windows, which a 256 KiB block does not have |
| Compressing whole pieces or files | better, more context | — | breaks asking for any block from any source |
| A dictionary per file type | better on small blocks | — | blocks are 256 KiB, where a dictionary adds little |

The message dictionary of §5.3 is not used for blocks: it is trained on protobuf messages.

### 5.3 Which blocks

| Rule | Pro | Con |
|---|---|---|
| **Measured saving** (chosen): compressed only when it saves 1/8; after a miss the next 15 blocks are not tried | works for any content; a wrong name cannot mislead it | a little work on the blocks that are tried. zstd gives up early on data that does not compress |
| A list of file types in the specification | no work on listed types | a name is not part of a file's identity; lists age; containers such as disk images vary inside |
| Local policy only | shortest text | a simple implementation would try every block of every video |

15 blocks after a miss is one piece less the block tried, so a file that does not compress costs
one attempt per piece. An implementation may still keep a list of types as a shortcut; it is not
part of the protocol.

### 5.4 What compressed sizes show

Encryption hides the bytes of a block, not their number. Blocks sent as they are all have the
same size, so every file looks alike. The sizes of compressed blocks depend on the content, and
their sequence identifies a file to an observer who has a copy and compresses it the same way.

- The observer has to match the sender's zstd version and level. There are few in use.
- Several ranges in flight and several sources blur the block boundaries; an observer next to
  one uploader still sees that uploader's blocks.
- Size shaping (§4.4) pads datagrams, not the total of a block.
- It confirms a file the observer already has. It does not reveal unknown content.

This differs from the CRIME/BREACH case of §5.3, where a secret shares a context with chosen
text. Here nothing secret is recovered; a known file is recognised. Either side can turn it off:
a sender may be set never to compress, and a downloader sets `BlockRequest.raw`, which the sender
must honour. The flag is per request and needs no state on the stream.

### 5.5 Open

- The gain has not been measured on a sample of shared files.
