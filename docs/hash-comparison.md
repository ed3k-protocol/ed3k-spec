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

## 6. The file root

- **Plain BLAKE3 over the raw file bytes.** The file root is unkeyed `hash` mode: no key, no
  `derive_key` context, no prefix or length added. Keyed and derive-key modes are only for the
  uses in §1 other than the file root. A root computed any other way names a different file.
- **Independent of block size.** The BLAKE3 tree is fixed: 1 KiB chunks, joined in pairs up to
  the root. The block size (§7, 256 KiB = 2⁸ chunks) only chooses which level of that tree is
  exchanged and verified; it is not a parameter of the function. So the root is the same for
  any block size, and equals `b3sum`. Another system that stores the plain BLAKE3 root of a
  file, with any block size, already holds that file's ed3k file ID, with no rehash. Only hash
  data and verified slices depend on the block size; both can be rebuilt from the file.

## 7. Block size: 256 KiB or 1 MiB

Bao calls the block size `chunk_log`: a block is 2^`chunk_log` chunks of 1 KiB. ed3k uses 8
(256 KiB); 10 would be 1 MiB. Hash data is one 32-byte chaining value per block (§7).

| | 256 KiB (`chunk_log` 8) | 1 MiB (`chunk_log` 10) |
|---|---|---|
| Hash data, share of file | 0.012 % | 0.003 % |
| Hash data, 4 GiB file | 512 KiB | 128 KiB |
| Hash data, 1 TiB shared | 128 MiB | 32 MiB |
| Lost per bad block | 256 KiB | 1 MiB |
| Largest block in memory (also after decompression) | 256 KiB | 1 MiB |
| File needs no hash exchange | ≤ 256 KiB | ≤ 1 MiB |
| Blocks per 4 MiB piece | 16 | 4 |
| `GetBlocks` entries for the same range | 4× | 1× |

**256 KiB is kept.**

- **Overhead is small.** 512 KiB on a 4 GiB download; the 1 TiB figure is spread over many
  separate transfers.
- **The block is the transfer and verification unit.** A bad sender is caught after 256 KiB,
  not 1 MiB, and a bad block costs a quarter of the bytes.
- **Tighter memory bound.** No block, compressed or not, needs more than 256 KiB.
- **Finer hash data gives coarser for free.** Four 256 KiB chaining values join into the 1 MiB
  value above them, so a node can produce `chunk_log` 10 hash data, or any larger size, without
  reading the file. 1 MiB values cannot be split back into 256 KiB ones.
