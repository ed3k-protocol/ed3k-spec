# Test vectors, wire version 1

Every value below was computed with the reference BLAKE3 implementation (the `blake3` crate,
1.8.7: `derive_key` for `H`, keyed XOF for the mask stream, the `hazmat` chaining values for
the tree) and a bitwise CRC32C. An implementation that reproduces them has the derivations of
`Protocol.md` §4.3, §6.1, §6.3, §10.1, §12.4 and §14.2 right. Hex is lower case; `‖` is
concatenation; integers in a layout are big-endian (§0).

## Sample inputs

| Name | Value |
|---|---|
| node public key | `d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a` (RFC 8032 test 1) |
| address | `124.31.75.21` |
| sample file | the 16 ASCII bytes `ed3k test vector` |
| set `id` | the 4 ASCII bytes `sid1` |
| issuer key | 32 bytes, byte *i* = `0xa5 XOR i`: `a5a4a7a6a1a0a3a2adacafaea9a8abaab5b4b7b6b1b0b3b2bdbcbfbeb9b8bbba` |
| secret | 32 bytes, byte *i* = `0x37 XOR i`: `37363534333231303f3e3d3c3b3a393827262524232221202f2e2d2c2b2a2928` |
| request value `r` | `1122334455667788` |
| alias `id` | `5a5b58595e5f5c5d5253505156575455` |
| period | day 20739 (2026-10-13 UTC) as 8 bytes: `0000000000005103` |
| salt | `0102030405060708` |
| network name | the 9 ASCII bytes `ed3k test` |

The one sample key serves as the node key and as the key of a set, a channel or a topic: `key`
in the preimages below is it.

## Identity (§6.1, §4.3)

```
NodeID = H("ed3k v1 node id", public key)[0..16)  = cf617f4479e97358c9d31741900a90ae
obf_id = NodeID[0..12)                            = cf617f4479e97358c9d31741
```

## Routing ID (§6.3)

`r = NodeID[15] & 7 = 6`. For the sample address:

```
masked    = c0 0f 0b 15                (ip AND 03 0F 3F FF; byte 0 OR r << 5)
CRC32C    = 719c3fe1                   top 21 bits = 0e3387
RoutingID = 719c3f4479e97358c9d31741900a90ae
```

The four rows of §6.3 (address, `r`, CRC32C, top 21 bits):

| Address | `r` | masked | CRC32C | top 21 bits |
|---|---|---|---|---|
| 124.31.75.21 | 1 | `20 0f 0b 15` | `5fbfbdb2` | `0bf7f7` |
| 21.75.31.124 | 6 | `c1 0b 1f 7c` | `5a3ce9b0` | `0b479d` |
| 65.23.51.170 | 6 | `c1 07 33 aa` | `a5d4344a` | `14ba86` |
| 2001:db8:1234:5678::1 | 3 | `60 01 05 08 12 34 00 00` | `32b2d72e` | `06565a` |

## Record targets (§10.1)

The sample file's root is `453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48fc4a0e9bc3e394`.

| Kind | Preimage | Target |
|---|---|---|
| `SOURCE`, `NOTE`, `MEMBER` | `root[0..16)` | `453b9482e0d4f14e084be18f10b4b16b` |
| `HEAD` | `H("ed3k v1 set head", key ‖ id)` | `db35cab4ac16050fa47f13fbcc4fe600` |
| `LIVE` | `H("ed3k v1 live", key ‖ id)` | `d6c1deaa5cf816f5f5430d7dd6c0e59a` |
| `PART` | `H("ed3k v1 parts", key ‖ id ‖ issuer key)` | `804fa4f5ecc00a2337fc07d81ac0510c` |
| `GATE` | `H("ed3k v1 gate", NodeID)` | `65c28351371c965dc864a4e0f14ef72d` |
| `SUB` | `H("ed3k v1 topic", 01 ‖ key ‖ id)` (type 1) | `f48c084b69dab97e1a4beaa74d48bb1f` |
| `ALIAS` | `H("ed3k v1 alias", 04 ‖ "ed2k" ‖ alias id)` | `309bddd5e0f4d3a1b37f963d12a3613a` |
| `HIDDEN` base | `H("ed3k v1 hidden head", key)` | `08299d3a4b291d676e0ecd0d39db534a` |
| `HIDDEN` | `H("ed3k v1 hidden head target", base ‖ period)` | `3566278da07fab366afdbc5f4d76265c` |

`FRIEND`, open link (friend.md §2 and §4):

```
k      = H("ed3k v1 friend", secret ‖ key)            = 4ca05ced9e1dc7091e0b2677e874d201c4d1fb3dac1934ebe8fef2f80b5e69da
k_rec  = H("ed3k v1 friend record", k)                = 2bcb004f2be3afb67b17062739acc35606912f4bc2eef1a2337dd3ea99ad16ce
target = H("ed3k v1 friend target", k ‖ period)[0..16) = 7974cd2b5ffa64e851517666b7a460ec
```

`FRIEND` request (friend.md §3), `r` as its 8 raw bytes:

```
H("ed3k v1 friend request target", key ‖ r ‖ period)[0..16) = 482ed5f410de42fb81f27a6ceb39c827
```

## Keyword targets (§10.1)

`H("ed3k v1 keyword", 00 ‖ word)[0..16)`, level 0, for the words of the examples in §10.1:

| Word | Target |
|---|---|
| `the` | `ebac9c8ff16567f5ea601eb083446933` |
| `big` | `4f39e4225918bc8b814d4a22f5b293ac` |
| `sleep` | `f797b920611f6bebcb89cad846086177` |
| `1946` | `44ea18a56cdfdd6ea2cd49ecc0ace001` |
| `1080p` | `3f2df8c6648f2a1c8ba36b380a91d5c1` |
| `ça` | `d5f217031899040bcd0304b1c67536b2` |
| `va` | `5ce5a7ac339dee564ffbc48b031bfc35` |
| `été` | `aa5038d6eb60080247c35b751a004695` |
| `strasse` | `6fbd6277e369c70ebb8f1ed7f00c02af` |
| `tokyo` | `fa5db173fc4ae2f9bf101849de1f8ed9` |
| `story` | `4fb0dea3e88250d48db1ab4145d6009d` |
| `東京` | `988b051341ac734878ffb07e7c97c1d9` |
| `京物` | `c5ad4b45ff9cc3d17e09986f98e6711a` |
| `物語` | `bbcb1d1e2730b4a844019e9d9d67e705` |

## Obfuscation (§4.3)

```
net_id("ed3k test") = H("ed3k v1 net", name)[0..16) = 19f07e3be153aed603e15b7c7c50ee13

public network, dir 0x00 (to the node that accepted):
  key           = H("ed3k v1 obfs", obf_id ‖ 00 ‖ salt)
                = 015bc0e543423a9ede489325944e161ff19848369026217d2944ecd547e08588
  stream[0..16) = a2fc1a781aeacf37ad1b0d2e1112db08
public network, dir 0x01:
  key           = 044d7656afac36bcaf6015a915045f7c3ce8f1da93d2cde4edfcde8285b62fe2
  stream[0..16) = 483efeb0ed02110f46e2e17ffb58c1ee
test network "ed3k test", dir 0x00:
  key           = H("ed3k v1 obfs", net_id ‖ obf_id ‖ 00 ‖ salt)
                = cf09d12875ae26aa9753d30b2841f2114549a29eb7e82ea736bc60f223c5009a
  stream[0..16) = 465a4533e297e694061ded909b6cc21a
```

A `HANDSHAKE` header `65 33 6b 01 40` (magic, `kp` = kind 1 with `pad_len` 0) masked with
the public-network dir 0x00 stream is `c7 cf 71 79 5a`.

## Derived eD2K user hash (§14.2)

```
(c0, c1) = H("ed3k v1 ed2k user hash", NodeID[0..12))[0..2) = (c1, 2e)
user hash = cf617f4479 0e e97358c9d31741 c1 6f 2e
          = cf617f44790ee97358c9d31741c16f2e        byte 5 = 14, byte 14 = 111
```

## Hash tree and proof (§12.4)

File: 5 blocks of 256 KiB (1,310,720 bytes), byte *i* = `(i / 256) mod 251`.

```
root       = 103f4e723c6f36531b9d0be6e48c8c9187dfd7a6f98917a9ca878b43bc52859b
block 0 cv = 55fa1e24fe29ec733b4284907b56f6edbd29953e05ddeda395c1884a29c4f629
block 1 cv = c2327d70dd6e9a16c8051575c71c3adae1af7d9b0736f4c570dcaffe1f843217
block 2 cv = c72ee4be107d937ea63c084d0096fc27e88db20f1227969b81d5e78df4e26106
block 3 cv = 4fdce0b1b798a557dc132967756a01110658514a0318049cf95a782e67723c5b
block 4 cv = ef30e2d4dcfd7ba4a87bd65cb623dec592128ef71fe9f249c8e6c9d9fa362454
```

A block's chaining value is computed with the chunk counter of its first chunk in the whole
file (block *b* starts at chunk 256 *b*), not from 0.

`HashRequest{first = 1, count = 3}` is answered with `cvs` = block 1, 2, 3 and

```
proof = block 0 cv        level 0: a(0) = 1 is odd, left sibling
      ‖ block 4 cv        level 2: b(2) = 0 is even and node(2, 1) exists; it is block 4 carried up
```

Level 1 contributes nothing: a(1) = 0, b(1) = 1. The receiver rebuilds
`parent(parent(b0, b1), parent(b2, b3))`, then `parent_root(that, b4)` = root.
