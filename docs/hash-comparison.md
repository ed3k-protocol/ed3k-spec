# Hash function: BLAKE3, and the alternatives

**Status:** design note (2026-10-11). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §7 (files), §4.3 (obfuscation keystream) and §6 (identity). Section numbers (§) refer to that document.

ed3k uses one hash function for files, node IDs, Kad targets, key derivation and the obfuscation
keystream. This note records why that function is BLAKE3, against the others that were
considered.

**Result:** BLAKE3, because the hash tree is part of the function. SHA-256 with a tree defined
on top is the fallback if BLAKE3 is ever weakened.

The speeds below are orders of magnitude from published benchmarks, not measurements made for
this document.

## 1. What the function is used for

| Use | What it needs |
|---|---|
| File root and block verification (§7) | a tree, so that a part of a file can be verified against the root |
| Node ID, routing ID, Kad targets (§6, §10.1) | domain separation by context string |
| Obfuscation key and keystream (§4.3) | keyed hashing and an output of any length |
| Derived eD2K user hash (§14.2) | domain separation |

## 2. Comparison

| | Tree / partial verification | Speed (one core) | Security margin | Availability |
|---|---|---|---|---|
| **BLAKE3** | built in; verified streaming is specified (Bao) | several GB/s with SIMD, scales across cores | 256-bit output, 128-bit collision security; 7 rounds, the thinnest margin in this table | small C library (CC0/Apache-2.0); not in Qt, OpenSSL or libsodium |
| SHA-256 | none; a tree has to be defined on top (as BitTorrent v2 does) | ~0.5 GB/s, ~2 GB/s with SHA extensions | very well studied | everywhere |
| SHA3-256 | none | ~0.5 GB/s | largest margin | everywhere |
| KangarooTwelve | tree mode with 8 KiB leaves, but no standard for verifying a part | ~2–3 GB/s with SIMD | reduced-round Keccak, well analysed | few libraries |
| BLAKE2b | none by default | ~1 GB/s | well studied | Qt, OpenSSL, libsodium |
| SHA-1 (AICH), MD4 (eD2K) | AICH has a tree; MD4 only per 9.28 MB part | fast | both broken for collisions; MD4 collisions take milliseconds | — |

## 3. Why BLAKE3

- **The tree is part of the function.** With any other entry the specification would have to
  define leaf size, node encoding, domain separation and odd-node handling itself. That is where
  hash-tree designs usually go wrong.
- **A standard tool computes the root.** The file identity is what `b3sum` prints.
- **No second primitive.** Keyed hashing, key derivation with context strings and an extendable
  output cover §4 and §6.

## 4. Costs of the choice

- **The thinnest round margin** of the candidates.
- **One more dependency.** BLAKE3 is not in Qt, OpenSSL or libsodium.

## 5. Fallback

SHA-256 with a BitTorrent-v2-style tree. The root field is 32 bytes either way and the hash is
named in every context string, so the function can be replaced with a wire version bump.
