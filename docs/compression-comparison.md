# Compression: zstd with a dictionary, and the alternatives

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §5.3 (compression). Section numbers (§) refer to that document.

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
- `Block` data is excluded; shared files are overwhelmingly compressed media and archives already.

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
  new ID, and every node must ship both for as long as old senders exist.
- **A corpus is needed first.** The dictionary has to be trained on real messages before the format
  is frozen, so it depends on a working implementation.

## 4. Required support, optional use

Making *support* mandatory removes negotiation and every "does the peer do compression" variant.
Making *use* mandatory was rejected: it would enlarge the many messages that are mostly hashes and
keys. A sender compresses only when the result is smaller and does not try under 64 bytes.
