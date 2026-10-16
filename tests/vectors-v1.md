# Test vectors, wire version 1

Every value below was computed with the reference BLAKE3 implementation (the `blake3` crate,
1.8.7: `derive_key` for `H`, keyed XOF for the mask stream, the `hazmat` chaining values for
the tree) and a bitwise CRC32C. An implementation that reproduces them has the derivations of
`Protocol.md` §4.3, §6.1, §6.3, §6.4, §10.1, §10.7, §12.4 and §14.2 right, and those of the
extensions named in the headings. Signatures were made with `ed25519-dalek` 2.2.0 and pass its
`verify_strict`, except the five of the section on rejected signatures, which it rejects; X25519 and the key conversion are `curve25519-dalek` 4.1.3, ChaCha20 and
ChaCha20-Poly1305 the `chacha20` 0.9 and `chacha20poly1305` 0.10 crates, SHA-1, SHA-256 and
SHA-512 the `sha1` and `sha2` crates, Argon2id the `argon2` crate 0.5.3 (the two outputs also
with OpenSSL 3.6), UTS 46 the `idna` crate 1.1.0. The words of the tokeniser cases were made with the Unicode 16.0.0
tables (`unicodedata` of Python 3.14). Every serialised message below is in the canonical
serialisation of §0 and was read back with `protoc`. Hex is lower case; `‖` is
concatenation; integers in a layout are big-endian (§0).

Random values (salts, padding, nonces, X25519 private values) are fixed here so that the
results can be reproduced. An implementation draws them at random.

The values from the second routing ID on, the edge cases of the tokeniser, the rejected
signatures and forms, the whole datagrams and everything after the first tree are printed by the generator in
[`vectors-gen/`](vectors-gen/) (`tok.py`, then `cargo run --release -- words.txt`). The
sections from the version 2 torrent on, and those after the live segment record, are its
output as it stands.

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
| alias `id` | scheme `ed2k` (alias.md §2): a 16-byte hash, byte *i* = `0x5a XOR i`, ‖ the size of the sample file as 8 bytes: `5a5b58595e5f5c5d52535051565754550000000000000010` |
| period | day 20739 (2026-10-13 UTC) as 8 bytes: `0000000000005103` |
| salt | `0102030405060708` |
| network name | the 9 ASCII bytes `ed3k test` |
| second key | `3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c` (RFC 8032 test 2; its seed is `4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb`) |
| X25519 pair `(x, e)` | `77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a`, `8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a` (RFC 7748 §6.1, Alice) |
| X25519 pair `(y, f)` | `5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb`, `de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f` (RFC 7748 §6.1, Bob) |
| nonce (12 bytes) | `000102030405060708090a0b` |
| time | 1791849600 (2026-10-13 00:00 UTC, the start of day 20739) |

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

With the same node ID at `21.75.31.124` the third byte differs from the CRC's and from the
node ID's: its top 5 bits come from the CRC (`e9` → `e8`), its low 3 bits from the node ID
(`7f` → `07`).

```
masked    = c1 0b 1f 7c
CRC32C    = 5a3ce9b0                   top 21 bits = 0b479d
RoutingID = 5a3cef4479e97358c9d31741900a90ae
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
| `ALIAS` | `H("ed3k v1 alias", 04 ‖ "ed2k" ‖ alias id)` | `9f8d46b06d714c56e4648bf6aa932c32` |
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
| `final` | `7c1c05bfcc0e8de2e77021cf8408423e` |
| `tokyo` | `fa5db173fc4ae2f9bf101849de1f8ed9` |
| `story` | `4fb0dea3e88250d48db1ab4145d6009d` |
| `東京` | `988b051341ac734878ffb07e7c97c1d9` |
| `京物` | `c5ad4b45ff9cc3d17e09986f98e6711a` |
| `物語` | `bbcb1d1e2730b4a844019e9d9d67e705` |

### Edge cases of the tokeniser

`<U+…>` stands for that code point.

| Name | Words | Keywords | What it shows |
|---|---|---|---|
| `Avi.Love.avi` | `avi` `love` | `love` | the word of the extension is no keyword, wherever it occurs |
| `AC/DC` | `ac` `dc` | `ac` `dc` | no `.`, so no extension; keywords of 2 bytes |
| `a.b` | `a` `b` | none | words of 1 byte |
| `notes.` | `notes` | `notes` | nothing after the last `.`: no extension |
| `archive.tar.backup123` | `archive` `tar` `backup123` | all three | 9 code points after the last `.`: no extension |
| `co<U+00AD>op.TXT` | `coop` `txt` | `coop` | a code point of category Cf is removed |
| `e<U+0301>te<U+0301> à la carte` | `été` `à` `la` `carte` | all four | NFKC composes; `à` is 2 bytes and a keyword |
| `ＦＵＬＬ<U+3000>Ｗｉｄｔｈ` | `full` `width` | both | NFKC of the full-width forms |
| `İstanbul` | `i<U+0307>stanbul` | that word | full case folding of U+0130; the mark stays in the word |
| `x² ½` | `x2` `1` `2` | `x2` | NFKC makes `½` into `1`, U+2044, `2`; U+2044 separates |
| `don't` | `don` `t` | `don` | |
| `abc東京物def.mkv` | `abc` `東京` `京物` `def` `mkv` | `abc` `東京` `京物` `def` | an unspaced run inside a word |
| `x東y` | `x` `東` `y` | `東` | a run of 1 |
| `ｶﾞｰﾄﾞ` (U+FF76 U+FF9E U+FF70 U+FF84 U+FF9E) | `ガー` `ード` | both | NFKC gives U+30AC U+30FC U+30C9 |
| `映画.日本語` | `映画` `日本` `本語` | `映画` | the pairs of an unspaced extension are no keywords |
| `ab.x日本` | `ab` `x` `日本` | `ab` | the extension `x日本` gives a word and a pair; neither is a keyword |
| `น้ำ` (U+0E19 U+0E49 U+0E33) | U+0E19 U+0E49, U+0E49 U+0E4D, U+0E4D U+0E32 | all three | NFKC splits U+0E33 into U+0E4D U+0E32; a pair may begin with a mark |

Targets of the keywords above that the first table does not have:

| Keyword | UTF-8 | Target |
|---|---|---|
| `love` | `6c6f7665` | `941e0056ab2573fb531870cd5030f4a6` |
| `ac` | `6163` | `e9440af0a794dce72ecbb72d520fe685` |
| `dc` | `6463` | `e94991eaabb37d934b0279a77a6708f7` |
| `notes` | `6e6f746573` | `cf5757ddcc76ae0dd9d3de7d489f5b3d` |
| `archive` | `61726368697665` | `f01af38391144eda688455db29fab626` |
| `tar` | `746172` | `92bfac102106e33584b03feebe2e7d75` |
| `backup123` | `6261636b7570313233` | `3a43e6811853b730f36173cda25e0142` |
| `coop` | `636f6f70` | `2b4e85e3303a56834fb0381eb9e6a990` |
| `à` | `c3a0` | `cea931f9a697dde28c881129ad1edcae` |
| `la` | `6c61` | `0123a91f34f98c78ae22df2db0b3e2a5` |
| `carte` | `6361727465` | `7c495f05e9a35ea309897967321bc95b` |
| `full` | `66756c6c` | `d6a2b7096c83f17c45b06cb67eea8f04` |
| `width` | `7769647468` | `23fa46598f82bec3ff3c816f45b26cac` |
| `i<U+0307>stanbul` | `69cc877374616e62756c` | `e3ee3b59a82510462b54b9831ed8fdc4` |
| `x2` | `7832` | `f69557aa1f9e282cd7d2a2376e77b967` |
| `don` | `646f6e` | `57280ed7fa13c24471dc97654e527482` |
| `abc` | `616263` | `6f18af9272b3006e77704c60012c3e95` |
| `def` | `646566` | `2c6d713c8a3654d8390de95b4e833821` |
| `東` | `e69db1` | `a278d22aab3582777ae987fd45329504` |
| `ガー` | `e382ace383bc` | `e032996bbf74f08c39b8f940a356e148` |
| `ード` | `e383bce38389` | `8bcf978742dbf241af20edb47b71fc0d` |
| `映画` | `e698a0e794bb` | `bf766cfad784a465d50b9f8b947200e9` |
| `ab` | `6162` | `93c14af517fc93a1f297b673dbfcdf02` |
| U+0E19 U+0E49 | `e0b899e0b989` | `a0be856da3f5177812406bec4c03e87a` |
| U+0E49 U+0E4D | `e0b989e0b98d` | `e101155a875aba1c1123760f7c8b0f9f` |
| U+0E4D U+0E32 | `e0b98de0b8b2` | `ca09fb4602c9a001b290474ccb41deac` |

## Signed record (§10.7)

A `KEYWORD` record for the sample file under the keyword `sleep`, signed with the sample key
(RFC 8032 test 1; its seed is `9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60`).
Ed25519 signatures are deterministic, so the signature is reproducible.

```
Record    = { expires = 1791936000, file = { root, size = 16, name = "The.Big_Sleep (1946) [1080p].mkv" } }
body      = 08808cbbd60612460a20453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48fc4a0e9bc3e394
            10101a205468652e4269675f536c65657020283139343629205b31303830705d2e6d6b76      (78 bytes)
kind      = 02 (KEYWORD)
target    = f797b920611f6bebcb89cad846086177                                    (`sleep`, level 0)
signed    = "ed3k v1 record" ‖ kind ‖ target ‖ body                             (109 bytes)
          = 6564336b207631207265636f7264 02 f797b920611f6bebcb89cad846086177 ‖ body
signature = b1c5bcac6211b9e588444546096a6b4a817476d8103a32084162df8641ce5f8b
            7960c6196eec9e53ef5430e4ba978024b30bec48f02861507ce71c8d67b25102
```

### Signatures and forms that are rejected

The same `signed` bytes, 109 of them, under the sample key unless a key is given. For each of
the five signatures the equation `[S]B = R + [k]A` holds, with `k` taken over the bytes as
they are sent, once the check named is skipped: a verifier that skips it accepts. §10.7
rejects every one, and so does `verify_strict` of ed25519-dalek 2.2.0. L is the group order,
p = 2²⁵⁵ − 19.

```
S not below L: the signature above with S + L
  R ‖ S     = b1c5bcac6211b9e588444546096a6b4a817476d8103a32084162df8641ce5f8b
              6634bc76884fb1abc5f1278799915f39b30bec48f02861507ce71c8d67b25112
R of small order: the identity, S = k · a (a = the secret scalar of the sample key)
  R ‖ S     = 0100000000000000000000000000000000000000000000000000000000000000
              a466c59a42f6107cb6af91b61f753b1f0637a80cace2234de45f5254eb8da90d
R not canonical: the identity written as y = 1 + p, S = k · a with k over these bytes
  R ‖ S     = eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f
              ef4e5f0ad5f8dcc6e01eceeca12eab1b0621a0be115e2a07239f73b87403a90c
A of small order: the point of order 8, S = 6, R = [S]B; k · A is the identity
  A         = c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a
  R ‖ S     = f47e49f9d07ad2c1606b4d94067c41f9777d4ffda709b71da1d88628fce34d85
              0600000000000000000000000000000000000000000000000000000000000000
A not canonical: the point of order 4 (y = 0) written as y = p, S = 4, R = [S]B
  A         = edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
  R ‖ S     = 2f1132ca61ab38dff00f2fea3228f24c6c71d58085b80e47e19515cb27e8d047
              0400000000000000000000000000000000000000000000000000000000000000
```

A verifier that skips the check in question computes the same `k` and the same point on each
side; it is the rule of §10.7 alone that rejects. The `Signed` forms below carry the valid
signature; the `body` is the 78 bytes above, written as `0a4e ‖ body`.

```
valid               Signed{ body, sigs = [ { alg = 1, sig } ] }
  after the body    = 1a4408011240b1c5bcac6211b9e588444546096a6b4a817476d8103a32084162df8641ce5f8b7960c6196eec9e53ef54
                    30e4ba978024b30bec48f02861507ce71c8d67b25102
alg = 0             Signed{ body, sigs = [ { sig } ] }: invalid, `alg` 0
  after the body    = 1a421240b1c5bcac6211b9e588444546096a6b4a817476d8103a32084162df8641ce5f8b7960c6196eec9e53ef5430e4
                    ba978024b30bec48f02861507ce71c8d67b25102
two Ed25519         Signed{ body, sigs = [ { alg = 1, sig }, { alg = 1, sig } ] }: invalid, one per algorithm
  after the body    = 1a4408011240b1c5bcac6211b9e588444546096a6b4a817476d8103a32084162df8641ce5f8b7960c6196eec9e53ef54
                    30e4ba978024b30bec48f02861507ce71c8d67b251021a4408011240b1c5bcac6211b9e588444546096a6b4a817476d8
                    103a32084162df8641ce5f8b7960c6196eec9e53ef5430e4ba978024b30bec48f02861507ce71c8d67b25102
key out of range    Signed{ body, key = 1, sigs = [ { alg = 1, sig } ] } in a KadAnswer with one key: dropped
  after the body    = 10011a4408011240b1c5bcac6211b9e588444546096a6b4a817476d8103a32084162df8641ce5f8b7960c6196eec9e53
                    ef5430e4ba978024b30bec48f02861507ce71c8d67b25102
```

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

### Whole datagrams

Public network, `obf_id` of the sample node. `plain` is `magic ‖ kp ‖ pad ‖ body` before the
mask; `datagram` is what goes on the wire, the salt first. The QUIC bytes are stand-ins: a first
byte with the long-header bit (`c3`) or without it (`40`), then a counter.

```
HANDSHAKE, dir 0x00, salt 0102030405060708, pad_len 3 (aa bb cc), body 40 bytes
  key      = 015bc0e543423a9ede489325944e161ff19848369026217d2944ecd547e08588
  plain    = 65336b0143aabbccc30102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f2021222324252627
  datagram = 0102030405060708c7cf7179594074fb6e1a0f2d1517dd0fc6ac011ab50118d11530787ba50a25e1c3b67728c5e44ded
             1e3831c44199cce8

FRAGMENT, piece 0 of 2 of that body (its first 15 bytes), msg_id 5c, frag 02,
dir 0x00, salt 1112131415161718, pad_len 2 (dd ee)
  key      = 506b7aec409126d72815e85f19cfcaef22efe41cc2ce719987597b4cff65d647
  plain    = 65336b0182ddee5c02c30102030405060708090a0b0c0d0e
  datagram = 11121314151617187465c88fdaad03a9b0d3143a5d81e2e0c07e666be20cccf3

FRAGMENT, piece 1 of 2 (the other 25 bytes), msg_id 5c, frag 12,
dir 0x00, salt 2122232425262728, pad_len 0
  key      = 450d789930daca47a27dd8307195e287efc7a9ec5cf77305a8f7bcb2a8c675ae
  plain    = 65336b01805c120f101112131415161718191a1b1c1d1e1f2021222324252627
  datagram = 2122232425262728f46df8f16fb1babe6be0094bff77f58100c67e8956f37dc665d51fade09f5560

DATA, dir 0x01, salt 3132333435363738, pad_len 2 (f0 0f), packet of 80 bytes
  key      = 7559c75c7150e28bad66ad0a61adf1d9123006828c6d0fb19936386ba4ad61ce
  plain    = 65336b0102f00f400102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728
             292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f
  datagram = 313233343536373859688f3efab84209604dd4fd78a27e225d9ee2a2a657f09908f07d24c828bba0d4f507ad8a602368
             2e43353f3168ba6744fbaf11f4265723f45897db07a553529bb86c05f5e723404142434445464748494a4b4c4d4e4f

PROBE, dir 0x00, salt 4142434445464748, pad_len 1 (99), nonce a1a2a3a4a5a6a7a8
  key      = e816031163738ff21eb55341389114518e74e1dc849a3a0f3c4e6d51ecc89666
  plain    = 65336b01c199a1a2a3a4a5a6a7a8
  datagram = 414243444546474860859be56816b55f2f5d527512f3
```

The two pieces, put together in the order of `idx`, are the body of the `HANDSHAKE` above. Of
the `DATA` datagram 71 bytes are masked: the 5 of the header, the 2 of the padding and the
first 64 of the packet. Its last 16 bytes are the packet's own (`40` … `4f`).

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

### Other sizes

The same byte pattern. Blocks 0 to 4 have the chaining values above wherever they are full
blocks of the file; `node(k, j)` is the node of §12.4.

**n = 1**, 100,000 bytes. No hash exchange; the root is the hash of the file:

```
root = f3eadfdb1a885faeaf4de1e409a3bc50ad12c2f6f9264014b208d4ed87233318
```

**n = 2, last block of 1 byte**, 262,145 bytes:

```
root       = fad1c29152879b677f8ca0b2f7cae5a934bc0ebfb5ee642095723f09e5caa752
block 1 cv = 008be304b1866b26902531c83fedb08abe9562846894bdabd10b65dc1e7d4092
```

`HashRequest{first = 0, count = 1}` has `proof` = block 1 cv; `{first = 1, count = 1}` has
`proof` = block 0 cv. The root is `parent_root(b0, b1)`.

**n = 6, last block of 1,000 bytes**, 1,311,720 bytes:

```
root       = 366dcda67caf63f9543a3f491264b7d5940cffbda5cae4545e3c8b60b4e1f07c
block 5 cv = 3deff90b9cee8a4d628340dfef60012d2eb4b3612151f7d669d414b26e75ed03
node(1, 0) = b995408e0a6ca938a3edc97104bed5fe6ad5874b7258c60c1b528cbd156f9b46      parent(b0, b1)
node(1, 1) = cda2d0b3b269d459ef7ab19fd910369ade8c6069619d9d37c34e75ff5e549c3b      parent(b2, b3)
node(1, 2) = 0dc1265adedc6f1edb4ecc06a4573c4f22fc823eb4e86c863d356532907275cc      parent(b4, b5); carried up as node(2, 1)
node(2, 0) = 4fc7bcf5a5e19a22ea31568aedf995deb92e6958fccbbe9a321a5bab2ab3229c      parent(node(1, 0), node(1, 1))
```

| Request | `proof` |
|---|---|
| `first = 4, count = 1` | block 5 cv ‖ node(2, 0). Level 1 gives nothing: b(1) = 2 is even, but node(1, 3) does not exist |
| `first = 5, count = 1` | block 4 cv ‖ node(2, 0) |
| `first = 2, count = 2` | node(1, 0) ‖ node(2, 1). Level 0 gives nothing |

**n = 7**, 1,835,008 bytes, all blocks full:

```
root       = 9d4d65ac18d289c0d8e9ef167f9b617c597e4b5ecf33c8694c66eb18ff716850
block 5 cv = a9f6c52110a54d213b4b3d27f5cb4c1b505ebcfbaa1ee76a1bdf30dc471131b4
block 6 cv = 7baa8a204d9b5e9dbfe9126412dce614161d2e2f177d421f9864d33243aebbd6      carried up as node(1, 3)
node(1, 2) = 355174190d08ecb96bc7fc429f2cea030a7a5b317916fa22edf322e1a2026726      parent(b4, b5)
node(2, 1) = f8115cfd612632625f36304c774e3f6721b1bca575fe9db87595bbdd88122598      parent(node(1, 2), b6)
```

`node(1, 0)`, `node(1, 1)` and `node(2, 0)` are those of n = 6. Block 5 is full here and has
another value than the short block 5 there.

| Request | `proof` |
|---|---|
| `first = 6, count = 1` | node(1, 2) ‖ node(2, 0). Level 0 gives nothing: block 7 does not exist |
| `first = 5, count = 1` | block 4 cv ‖ block 6 cv ‖ node(2, 0) |
| `first = 0, count = 7` | empty |

## Key conversion (§6.4)

The sample key and its seed, converted for X25519:

```
x_public  = d85e07ec22b0ad881537c2f44d662d1a143cf830c57aca4305d85c7a90f6b62e
x_private = 307c83864f2833cb427a2ef1c00a013cfdff2768d980c0a3a520f006904de94f
X25519(x, x_public) = X25519(x_private, e)
          = 90f395580ca33f3c54390ac1d7210220b6a336de2c47a61ac90ff56e6be11f18
```

## Hop keys (hops.md §2.3)

The owner's pair is `(x, e)`, the hop's pair is `(y, f)`, and the hop's node key is the sample
key.

```
ss1   = 90f395580ca33f3c54390ac1d7210220b6a336de2c47a61ac90ff56e6be11f18      the value of §6.4 above
ss2   = 4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742
k     = H("ed3k v1 hop", ss1 ‖ ss2 ‖ e ‖ f ‖ node key)
      = 9903bea8a1e5f2dfc053bf0fd268d18ed9b2b222c28a8f884c659ac77ea09620
proof = d0e7dbc97d709a814991a9520790ea4c81adb273e5d484f4cc36b4f92f3d9980
k_out = 8426c27cc34b7c52cdaa8e8a710d0948c2684f0dd662b6788a609dffe96a0680
k_in  = 098de99f8d9fc72089113d45daabf5c5f61fb1d35f6fad9ebc46bccdf5c687ac
```

First 16 bytes of a layer's key stream (ChaCha20, nonce `00000000` ‖ stream number, counter 0):

```
k_out, stream 0 (nonce 000000000000000000000000) = b781a9ebd2b6f21b72ad91eb9016350c
k_in,  stream 0                                  = e19180f27614e216058b93c5fd2b6c2b
k_out, stream 5 (nonce 000000000000000000000005) = c2a45231d8cd935606c3a0e036879def
```

## Manifest and signatures (file-set.md §1 and §2.4, multi-author-set.md §1)

```
FileSet = { title = "vectors", files = [ { root, size = 16, name = "sample.txt" } ], id = "sid1", seq = 1 }
body    = 0a07766563746f72731a300a20453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48fc4a0e9bc3e39410101a
          0a73616d706c652e7478743204736964313801      (67 bytes)
```

**One author.** Signed with the sample key over `"ed3k v1 set manifest" ‖ body`:

```
signature = 5d4e80e6e2b206a768a4eaee864adf527c0ae9d62714887fb5f5f42c8a5023d6
            bb0573fa82234a740e739c80d4a43ebc26fb774101f69c50ad72067031d0ee0f
file      = "ed3kset" ‖ 01 ‖ SetFile{ body, key, sigs }      (181 bytes)
          = 6564336b736574010a430a07766563746f72731a300a20453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48
            fc4a0e9bc3e39410101a0a73616d706c652e74787432047369643138011220d75a980182b10ab7d54bfed3c964073a0e
            e172f3daa62325af021a68f707511a1a44080112405d4e80e6e2b206a768a4eaee864adf527c0ae9d62714887fb5f5f4
            2c8a5023d6bb0573fa82234a740e739c80d4a43ebc26fb774101f69c50ad72067031d0ee0f
root      = 75bda0e39c1bc81d7b13381e17606ce7f793a06d2469248e9f5ad3f0a5d2ee3d
```

**Grant.** Owner = the sample key; signers = the sample key (index 0) and the second key
(index 1):

```
Grant     = { id = "sid1", number = 1, keys = [sample key, second key], required = 2 }
          = 0a047369643110011a20d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a1a203d4017c3
            e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c2002      (78 bytes)
signature = by the sample key, over "ed3k v1 set grant" ‖ Grant
          = 97479f5ed38efb3beddd796db271a70578d9ef9dbb4ba7844afec30b407983f8
            7d8a68dc8cec622e3db2409d9cfb18abd80ae7de66be93db7923c5c2e5548e0c
```

**Cosignatures of the manifest**, each over `"ed3k v1 set cosig" ‖ body`:

```
signer 0  = aaa350798c2363b85390335a0bf82c577d87b9a0c9527b25e45e890d3d09b940
            b033a163fb06793440689cda2e86f6b1bdd13c9d122977b366f11cb0c3895b03
signer 1  = ed001dd11ee415b19bbdabcb5180598a7808394ede6cce1423541dec9d9a50bb
            28c731724e49d6f1970af299a96e751b3099951916ef61e84c73b9948ec63204
file      = "ed3kset" ‖ 01 ‖ SetFile{ body, key, grant, cosigs }      (444 bytes)
          = 6564336b736574010a430a07766563746f72731a300a20453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48
            fc4a0e9bc3e39410101a0a73616d706c652e74787432047369643138011220d75a980182b10ab7d54bfed3c964073a0e
            e172f3daa62325af021a68f707511a22b8010a4e0a047369643110011a20d75a980182b10ab7d54bfed3c964073a0ee1
            72f3daa62325af021a68f707511a1a203d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c
            20021220d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a1a440801124097479f5ed38e
            fb3beddd796db271a70578d9ef9dbb4ba7844afec30b407983f87d8a68dc8cec622e3db2409d9cfb18abd80ae7de66be
            93db7923c5c2e5548e0c2a46124408011240aaa350798c2363b85390335a0bf82c577d87b9a0c9527b25e45e890d3d09
            b940b033a163fb06793440689cda2e86f6b1bdd13c9d122977b366f11cb0c3895b032a480801124408011240ed001dd1
            1ee415b19bbdabcb5180598a7808394ede6cce1423541dec9d9a50bb28c731724e49d6f1970af299a96e751b30999519
            16ef61e84c73b9948ec63204
root      = cc00b477856526c4719b767a8293c756169349fe438121d64ba9bc472378cf8a
```

`signer` = 0 is the default value and is left out of the first `CoSig`.

**Joint head** (§1.3) for that manifest:

```
Head      = { id = "sid1", seq = 1, root = the root above, size = 444 }
          = 0a047369643110011a20cc00b477856526c4719b767a8293c756169349fe438121d64ba9bc472378cf8a20bc03      (45 bytes)
target    = db35cab4ac16050fa47f13fbcc4fe600                     (`HEAD`, above)
signed    = "ed3k v1 joint head" ‖ target ‖ Grant.number(8) ‖ Head      (87 bytes)
          = 6564336b207631206a6f696e742068656164db35cab4ac16050fa47f13fbcc4fe60000000000000000010a0473696431
            10011a20cc00b477856526c4719b767a8293c756169349fe438121d64ba9bc472378cf8a20bc03
signer 0  = 4a4e9b0c443aa2f03e1daed4208c68f73d94e92d2576ec7eb5203050da29db22
            5436554507b59457a0ccb3435d7c9f27b0d8ff98f014c295e910b161d90dd00b
signer 1  = 7c0c91a92a83ac987efe487611e2ab36d48a4b44b383cb454a745121ce4536c3
            4d40ae67b167a7f22bf54c0c8787000d1405ec002c3a12f1c7a84f3e6e5f7f0e
```

## Canonical manifest of a torrent (torrent.md §2)

A version 1 torrent of two files: `a.txt`, the sample file, and `sub/b.bin`, the 5 ASCII bytes
`hello`. Its name is `ed3k vec`, its piece length 16,384, so it has one piece.

```
pieces    = SHA-1(sample file ‖ "hello") = 6baaebb2aa53d8167df0b11aa619a984d604d89b
info      = d5:filesld6:lengthi16e4:pathl5:a.txteed6:lengthi5e4:pathl3:sub5:b.bineee
            4:name8:ed3k vec12:piece lengthi16384e6:pieces20:<pieces>e      (142 bytes)
btih      = SHA-1(info)  = c83efbfc519022f307890901503501749e6164a5
info root = BLAKE3(info) = ee7d5ae5dc7e83b8d7c1288b9c7b8ec9c3dd54d1ac9f61afe053b4d37cca1517
root of `b.bin`          = ea8f163db38682925e4491c5e58d4bb3506ef8c14eb78a86e908c5624a67200f

FileSet   = { title = "ed3k vec", dirs = ["sub"],
              files = [ { root, size = 16, name = "a.txt" }, { root, size = 5, name = "b.bin", dir = 1 } ],
              torrent = { btih, info = info root, info_size = 142 } }
file      = "ed3kset" ‖ 01 ‖ SetFile{ body }      (179 bytes)
          = 6564336b736574010aa8010a086564336b2076656312037375621a2b0a20453b9482e0d4f14e084be18f10b4b16b0f61
            bfa2ffae0f6e48fc4a0e9bc3e39410101a05612e7478741a2d0a20ea8f163db38682925e4491c5e58d4bb3506ef8c14e
            b78a86e908c5624a67200f10051a05622e62696e20014a3b0a14c83efbfc519022f307890901503501749e6164a51a20
            ee7d5ae5dc7e83b8d7c1288b9c7b8ec9c3dd54d1ac9f61afe053b4d37cca1517208e01
root      = df88bce194ec658e1b8272c3badd2dd0e7d3bfff452b1a41022e2dc61756bea3
```

### Version 2 and hybrid

The same name and piece length, with three files: `a.txt` (the sample file), `empty` (no
bytes) and `sub/b.bin` (`hello`). A file of one block has the SHA-256 of its bytes as its
`pieces root`.

```
pieces root of a.txt = 0886c695dae60df00b3f7fea5c42c2622359996e666a3ea16dac09c4a448b9af
pieces root of b.bin = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
file tree = d5:a.txtd0:d6:lengthi16e11:pieces root32:<root of a.txt>ee
            5:emptyd0:d6:lengthi0eee
            3:subd5:b.bind0:d6:lengthi5e11:pieces root32:<root of b.bin>eeee      (180 bytes)
root of `empty` = af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262      (BLAKE3 of the empty input)
```

**Version 2:**

```
info      = d9:file tree<file tree>12:meta versioni2e4:name8:ed3k vec12:piece lengthi16384ee      (249 bytes)
btmh      = SHA-256(info) = 3d4ab798dfbcec5f58d27288da7146c916cb2f519a2e8ce2a8a5106b7d284ff4
info root = BLAKE3(info)  = 56ad98ea5369bb6b9d65915fdf6a2c4b0741425ddd2b6e43bfc0eb0042662cbe
FileSet   = { title = "ed3k vec", dirs = ["sub"],
              files = [ { root, size = 16, name = "a.txt" }, { root, name = "empty" }, { root, size = 5, name = "b.bin", dir = 1 } ],
              torrent = { btmh, info = info root, info_size = 249 } }
file      = "ed3kset" ‖ 01 ‖ SetFile{ body }      (234 bytes)
          = 6564336b736574010adf010a086564336b2076656312037375621a2b0a20453b9482e0d4f14e084be18f10b4b16b0f61
            bfa2ffae0f6e48fc4a0e9bc3e39410101a05612e7478741a290a20af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9ad
            c112b7cc9a93cae41f32621a05656d7074791a2d0a20ea8f163db38682925e4491c5e58d4bb3506ef8c14eb78a86e908
            c5624a67200f10051a05622e62696e20014a4712203d4ab798dfbcec5f58d27288da7146c916cb2f519a2e8ce2a8a510
            6b7d284ff41a2056ad98ea5369bb6b9d65915fdf6a2c4b0741425ddd2b6e43bfc0eb0042662cbe20f901
root      = 8b43b66fde37c05d035e5a18fccf8c61aa2bd642ce57f76ef220e2fbc56b5b8b
```

**Hybrid.** The version 1 list has a padding file after `a.txt`, so that the next file starts
a piece. It is no member. The two pieces are `a.txt` ‖ 16,368 zero bytes, and `hello`.

```
pieces    = 30955fe0c6691fb99c41a7757708f73f12480367
            aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d
info      = d9:file tree<file tree>
            5:filesl d6:lengthi16e4:pathl5:a.txtee
                     d4:attr1:p6:lengthi16368e4:pathl4:.pad5:16368ee
                     d6:lengthi0e4:pathl5:emptyee
                     d6:lengthi5e4:pathl3:sub5:b.binee e
            12:meta versioni2e4:name8:ed3k vec12:piece lengthi16384e6:pieces40:<pieces>e      (446 bytes; shown with spaces and line breaks that the dictionary does not have)
btih      = SHA-1(info)   = 891e05c57896f770335e33948a508027b1523dd5
btmh      = SHA-256(info) = 4dd6ef3a18cc15b7987a9af960cc8eba7d4a0a1a66eac41dc6aa96985cd28256
info root = BLAKE3(info)  = 376c50bd8a69de439dcc8a0be9b55d9c556b9cdc34ce8ae14e52eb1c6fe98deb
FileSet   = as for version 2, with torrent = { btih, btmh, info = info root, info_size = 446 }
file      = "ed3kset" ‖ 01 ‖ SetFile{ body }      (256 bytes)
          = 6564336b736574010af5010a086564336b2076656312037375621a2b0a20453b9482e0d4f14e084be18f10b4b16b0f61
            bfa2ffae0f6e48fc4a0e9bc3e39410101a05612e7478741a290a20af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9ad
            c112b7cc9a93cae41f32621a05656d7074791a2d0a20ea8f163db38682925e4491c5e58d4bb3506ef8c14eb78a86e908
            c5624a67200f10051a05622e62696e20014a5d0a14891e05c57896f770335e33948a508027b1523dd512204dd6ef3a18
            cc15b7987a9af960cc8eba7d4a0a1a66eac41dc6aa96985cd282561a20376c50bd8a69de439dcc8a0be9b55d9c556b9c
            dc34ce8ae14e52eb1c6fe98deb20be03
root      = c7fdd17e8a7066ab72ee3feb85b093b55979c51766c2a3fe01569b7665b0c100
```

`size` = 0 is the default value and is left out of the member `empty`.

## Hidden set (hidden-set.md §2 and §3)

Secret and key are the samples.

```
k      = H("ed3k v1 hidden set", secret ‖ key) = 75b6d42747dffa6ae895ac559d3d26c370f22232a47fce582dcffebbe0c347b3
k_head = H("ed3k v1 hidden set head", k)       = 2875f8fc31f3563b982468ef40edcf12e42f88e4d2192ccda94e74eb58fc7ad4
k_data = H("ed3k v1 hidden set data", k)       = 697f5c6d1a00f8b6b41ea8d1a5796c509fc4124e5d7690fcb0e890b0e9251639
```

A sealed head. The root and size of the sample file stand in for those of a sealed manifest:

```
Head      = { id = "sid1", seq = 1, root, size = 16 }
          = 0a047369643110011a20453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48fc4a0e9bc3e3942010      (44 bytes)
data      = nonce ‖ ChaCha20(k_head, nonce, Head)      (56 bytes)
          = 000102030405060708090a0b6e3273c6a981662a7e630759015dc4c9be2e0576b31a1c22464db19002aadaa2866ddb98
            c49f9ca44aad69d6
Record    = { expires = 1791936000, sealed = { seq = 1, data } }
body      = 08808cbbd6063a3c08011238000102030405060708090a0b6e3273c6a981662a7e630759015dc4c9be2e0576b31a1c22
            464db19002aadaa2866ddb98c49f9ca44aad69d6      (68 bytes)
signed    = "ed3k v1 record" ‖ 08 (HIDDEN) ‖ base ‖ body      (99 bytes; base = 08299d3a4b291d676e0ecd0d39db534a, above)
signature = e60c764f765a3e642804265178fb1de104edbbcd641285cba8781293add3e12f
            89331ce926caa0fa6aa6ecf08b58642c9d219e120ef390b687b0218e063fb20b
```

## Friend records (friend.md §3 and §4)

**Record of the open link** (`k_rec` above), sealed with the sample nonce:

```
FriendRecord = { peer = { ep4 = 124.31.75.21 port 4672 }, issued = 1791849600 }
             = 0a0812067c1f4b1512401080e9b5d606      (16 bytes)
plain        = 0010 ‖ FriendRecord ‖ 338 zero bytes      (356 bytes)
sealed       = nonce ‖ ChaCha20-Poly1305(k_rec, nonce, plain)      (384 bytes)
             = 000102030405060708090a0b871a3a4e82fa95e1e469cbf9b46c5912d30dd649fee2d157e3c093d629304ef36e48a60b
               1d93407d536587f8f802acb188edfcb31a9135f0cbf8af9840e2786522bf2543cfd55fe5aba95e4e10a2411e390aca76
               a114f03e27c8811cb87eec14b8f2fbfb4a9fed34e0dd69bdb9f80dbbb4c02f7212af54406e7c06e5b48de36853c3bc15
               6dc1e46af90b3a7e8fb2a3952f0e92a5c02c28c04f332bd73db5c11764deadfc0c80b49ec32b8aa9e289f2967b3f5429
               87114a6cfcccce10faac034a4d34f7df66b71e00e289c75d2efb3f4be0dfc76a61108b3cea32285d068e85245cfbe9ad
               92f47ade24a80f18f9cd88f637e3668303841c1659e06408c784ba6bb93fc6a55c06f52cc7de33b7f35c164f191f0194
               1472d5026e92b48272be94d974ba5f1161e868a4897f6ea03f7544a84fd3f9592c4609e14fff029dd49f3f86baddcfba
               1de305cbf5d27e7e6fe6b73278d73a522e09df26a49bd78d4636e1cf94ebed84c51e0ac00cac3f2954e3aaef98e76409
```

**Request.** The asked node has the sample key and `r` of the samples (its target is above).
The asking node has the second key and uses `(x, e)` as the pair of this one record.

```
signed        = "ed3k v1 friend request signature" ‖ sample key ‖ 000000006acd7480 ‖ "hi from the vectors"      (91 bytes)
signature     = e15f08a0a1116de5040934920cae86927a949a0c9aa052ebfb639a5b6eb1c094
                b5aaf9b7e7b287e03383743b4cf7740f4d358ad0e4f097ceadf68c981f3ade07
FriendRequest = { key = second key, text = "hi from the vectors", issued = 1791849600, sigs }
              = 0a203d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c121368692066726f6d2074686520
                766563746f72731880e9b5d606224408011240e15f08a0a1116de5040934920cae86927a949a0c9aa052ebfb639a5b6e
                b1c094b5aaf9b7e7b287e03383743b4cf7740f4d358ad0e4f097ceadf68c981f3ade07      (131 bytes)
k_req         = H("ed3k v1 friend request", X25519(x, x_public of §6.4 above) ‖ e ‖ sample key)
              = 3c2abfb18086184dc37e4c5802f970fa60ef33a037883f24a92c05c70e830cc6
plain         = 0083 ‖ FriendRequest ‖ 191 zero bytes      (324 bytes)
sealed        = e ‖ nonce ‖ ChaCha20-Poly1305(k_req, nonce, plain)      (384 bytes)
              = 8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a000102030405060708090a0b6610ec50
                f344819096b87628a0a3671c885f8b0d0efae459ebaa7ebebf5b6a360f8d1f2d1ebc091a16e1e57f8cac474fbc9c0a01
                1b66802f0c5b56f983586a850647779a6528dfc34d38914474f068954cb3922ff631301b0b333cdc7b1db2028d99efd8
                931e23c30bcad3d9bcc386f5b7e21696c0d9d38691b6a4715f49b5be1a0c21736ac87d96e2c9a253bc4293d8cf29b459
                bc4d08e44a49ae0516e2aaf0cf603887f2b6962f57e05c6271a687c84a2648b53740e37d3c529d7ffe4358f242440bb3
                6d154b551c9a5b946a5aa5ad4f4936bbff5f3270769d48baa02784057ba42e25ced8d337669797e4f67ef9e2e939ae53
                9c08362b31b71ae278de7a06803dae2b4293716e029eb793eb7cdbcf35a5f690fef3dcdcd1f94bd632ccdc2bb60b528f
                2fac1cb5f8231056fec85697732037a91a3dc73ee2dcc6164d983b953419045d2ebb8d91ddd2fbd3fb0a34a1b14f23da
```

## LAN announce (lan-discovery.md §1)

```
LanAnnounce = { id = NodeID, port = 4672 } = 0a10cf617f4479e97358c9d31741900a90ae10c024
plain       = 65336c01 ‖ LanAnnounce      (25 bytes)

public network, salt 0102030405060708:
  key       = H("ed3k v1 lan", salt)
            = 634ccde6786e6587bb378e19e50028d7a36d717d8ec1793bd5592a48a18a5801
  datagram  = 01020304050607080725bca2b354e2cb47d2aff768760f257c6873d61c8902e22d
test network "ed3k test":
  key       = H("ed3k v1 lan", net_id ‖ salt)
            = c6acd147e1aee83928b01737128af785ceb6501161554d73b4c44aecb05d4d6f
  datagram  = 0102030405060708dc1004d30ddabf7ca75c7628929f113b6b89bd8ca4dc1d7e14
```

## Live segment record (live.md §2.2)

The channel has the sample key and the `id` `sid1`; `stream` is the `LIVE` target above.

```
LiveSegment = { start = 1791849600, seq = 7, root, size = 16, ms = 4000 }
body        = 0880e9b5d60610071a20453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48fc4a0e9bc3e394201028a01f      (47 bytes)
signed      = "ed3k v1 live segment" ‖ d6c1deaa5cf816f5f5430d7dd6c0e59a ‖ body      (83 bytes)
signature   = 7061b642997bc4e26f71b1b4af9f3c06d0012b3da81134ee5f7c8cf6c001344e
              a984ebded051fd914d3575ad927402c04d300c4cda29e81f0ec5fb0c1c983207
```

## Node list (bootstrap.md §2)

```
net_tag("ed3k test") = H("ed3k v1 nodes net", net_id)[0..16) = 81f17b149485f8a972430b537f981072
```

## Kad datagrams (kad-datagram.md §2 and §3)

The responder has the sample key. The requester has the second key and sends from
`21.75.31.124`, port 4672. `txid` is `0a0b0c0d`. All four datagrams are masked with the
responder's `obf_id`, in the public network.

```
second key, converted (§6.4): x_public  = 25c704c594b88afc00a76b69d1ed2b984d7e22550f3ed0802d04fbcd07d38d47
                              x_private = 68bd9ed75882d52815a97585caf4790a7f6c6b3b7f821c5e259a24b02e502e51
ss            = 5166f24a6918368e2af831a4affadd97af0ac326bdf143596c045967cc00230e
k_req ‖ k_ans = H("ed3k v1 kad dgram", ss ‖ second key ‖ sample key), 64 bytes
k_req         = aaa0820b1803ae1b58914452430685dbe933211ed53b80e2cd5f0357c0f3d633
k_ans         = e0b1517900a7ab7095e291500406a11c0c6d6423e02c851e50559f4ee3ee8907

secret_e      = 32 bytes, byte i = 0xc3 XOR i
cookie        = BLAKE3_keyed(secret_e, 154b1f7c ‖ 1240)[0..8) = 437575e0ef5186ac

INIT, dir 0x00, salt 5152535455565758, pad_len 52 (pad byte i = i)      (70 bytes)
  datagram    = 51525354555657584351db368cc4a2c478f438c0f9e6688fe0ae800ffbbb146af8606e468871fe8a5d249403185edaed
                9463432dcc178607cbcf1f97256675b067cc50abf271

CHALLENGE, dir 0x01, salt 6162636465666768, pad_len 0, payload = cookie ‖ sample key      (58 bytes)
  datagram    = 61626364656667685b1b8ee037947c8e49201a1697834a9b88dd1795c4ca53e2f7d05b461aaa76a77622735ad9ed221b
                4adf8a893c8bd8b3b1b0

REQUEST, dir 0x00, salt 7172737475767778, pad_len 0
  KadRequest  = { target = root[0..16) of the sample file, want = 1 } = 0a10453b9482e0d4f14e084be18f10b4b16b1001
  plaintext   = nf 00 ‖ tag 06 ‖ KadRequest      (22 bytes)
  nonce       = 71727374757677780a0b0c0d
  aad         = 32 ‖ txid ‖ second key ‖ cookie      (45 bytes)
  sealed      = 4ab4fb803855f673e873e03a8c57489d9f61bd494902eb32a5a9ae0128e76176d6c24090ad4e
  payload     = second key ‖ cookie ‖ sealed      (78 bytes)
  req_hash    = H("ed3k v1 kad req", 32 ‖ txid ‖ payload)[0..8) = 4616e05bbe6f7f1b
  datagram    = 717273747576777834e58cd25b51d00b984cb4f70f3bd6a38868e780018bc0221d89bfc5bf5c719816a60c7b2a986c28
                9f9edafff279289eefbaeef3dc218f671d10cc4060ed2380ee31da6669e8ca20bb68dc790435037082037790464d2f5d

ANSWER, dir 0x01, salt 8182838485868788, pad_len 0
  KadAnswer   = { nodes4 = node ID of the second key ‖ 65.23.51.170 port 4672, responsible = true }
              = 0a16017da93e9fc9e3079a462d8b461dfc9b411733aa12401801
  plaintext   = nf 01 ‖ tag 08 ‖ KadAnswer      (28 bytes)
  nonce       = 81828384858687880a0b0c0d
  aad         = 33 ‖ txid ‖ req_hash      (13 bytes)
  sealed      = 016ddf8db465ccb0fa584dcad8f0b8d2bc668b7d372abf6c2b65c96a4e65582421902b741024c948c1ca4779
  datagram    = 8182838485868788678444984e63c893590fafd126f868b901fc951534744cb84905a3c01b60b5879e469b77a0ef1e91
                b2020e975b5bc19d0d9d9297c1fb
```

## Key puzzle (key-puzzle.md §2 and §3)

The sample key at `124.31.75.21`, on day 20739: epoch 691.

```
password = public key ‖ 04 ‖ 7c1f4b000000 ‖ 000002b3 ‖ nonce      (51 bytes)
salt     = "ed3k v1 puzzle", t = 1, m = 65536 KiB, p = 1, 32 bytes out, Argon2id version 0x13

nonce 0000000000000000: out = 425043cad580924beee55026a2eb93e523793e0bb2fe42e1c8c8594784e62e75      1 zero bits, not valid
nonce 0000000000000532: out = 00056d47e5ab341a93ebbb37ec1295f29ba9743b574de52b9bbf666d66c1adc3      13 zero bits, valid
Hello.puzzle = 000002b30000000000000532
```

The second nonce is the lowest one, counted as a big-endian number from 0, that gives 13 zero
bits.

## Usenet post (usenet.md §1 to §3)

A post of two files. The articles of `sample.txt` (the sample file, 16 bytes) have the message
IDs `p1.sample@vectors.invalid` and `p2.sample@vectors.invalid`, parts 1 and 2; the one
article of the second file (the 5 ASCII bytes `hello`) has `p1.hello@vectors.invalid`. Its yEnc
header gives the name as the bytes `68 e9 6c 6c 6f 2e 62 69 6e`, which are not UTF-8: they are
read as ISO 8859-1 and written as `héllo.bin` (`68 c3 a9 6c 6c 6f 2e 62 69 6e`).

```
file id of sample.txt = H("ed3k v1 nzb file", m₁ ‖ 00 ‖ m₂ ‖ 00) = 44a6f4e05a5e79d302673c02ac19fa6f20cfd2a13e20c9665ce003bcd5eb7b57
file id of héllo.bin  = H("ed3k v1 nzb file", m₁ ‖ 00)          = 2785bbf8e4b6d424e24346e1323454861a266b4a0dc592c56d5d86104c915604
order of the files    = héllo.bin, then sample.txt
post id               = H("ed3k v1 nzb", f₁ ‖ f₂) = 7c37dc64de7b1233ac765c68f55322a6ee24a4840abf80ebe5d338c0343e1608

article list = "ed3knzb" ‖ 01 ‖ ArticleList      (120 bytes)
             = 6564336b6e7a62010a280a0a68c3a96c6c6f2e62696e10051a1870312e68656c6c6f40766563746f72732e696e76616c
               69640a440a0a73616d706c652e74787410101a1970312e73616d706c6540766563746f72732e696e76616c69641a1970
               322e73616d706c6540766563746f72732e696e76616c6964
its root     = 31da26308dda2e12cdda54a45a7ad347374d378d78c9287465862e31d9a13e45

manifest     = "ed3kset" ‖ 01 ‖ SetFile{ body }, FileSet = { files, usenet = { id, list, list_size } }      (183 bytes)
             = 6564336b736574010aac011a300a20ea8f163db38682925e4491c5e58d4bb3506ef8c14eb78a86e908c5624a67200f10
               051a0a68c3a96c6c6f2e62696e1a300a20453b9482e0d4f14e084be18f10b4b16b0f61bfa2ffae0f6e48fc4a0e9bc3e3
               9410101a0a73616d706c652e74787452460a207c37dc64de7b1233ac765c68f55322a6ee24a4840abf80ebe5d338c034
               3e1608122031da26308dda2e12cdda54a45a7ad347374d378d78c9287465862e31d9a13e451878
its root     = 3184070839b8235c2812381efd8564172321bf1c4e5621cfdc6232056f55f3e9
```

## Hoster links (hoster.md §4)

| Link | `canonical` | `id` = H("ed3k v1 url", canonical) |
|---|---|---|
| `http://example.com` | `example.com/` | `5bb3c229423d62708c114b8b86b0c1936764a01050874ecb08e3f0f9c25b539d` |
| `https://example.com/?#` | `example.com/` | `5bb3c229423d62708c114b8b86b0c1936764a01050874ecb08e3f0f9c25b539d` |
| `HTTPS://User:p@ss@WWW.Example.COM:8443/a/../B%7e%2f?x=1#Frag` | `example.com/a/../B~%2F?x=1#Frag` | `05ea36eb68d685c3e292aee998b004dc37406a96251f193d9a6b7852ba115c5b` |
| `https://www.www.example.com:/` | `example.com/` | `5bb3c229423d62708c114b8b86b0c1936764a01050874ecb08e3f0f9c25b539d` |
| `http://www.www/` | `www/` | `a02d447a438ab4d8e6ee578157519eb452641e483339dfdde739152cb7ee31f7` |
| `https://example.com/100%25/%2541/%41?a+b=c d` | `example.com/100%25/%2541/A?a+b=c%20d` | `7faedf95933a273bb4481ba8ab1978f5f2bdf335fb75ff82410f398fd0ab5666` |
| `https://example.com/a b"<é>{}` | `example.com/a%20b%22%3C%C3%A9%3E%7B%7D` | `56ba782066df690f981146df5ab7b1f570ee59d17f78073d02901ac17e9b3ad5` |
| `https://example.com/p?a?b#c#d` | `example.com/p?a?b#c#d` | `f768e00638f5e9ab284b27efa08df2e221cb1ea926709164d1c6740a6070a0a9` |
| `https://faß.example./ü` | `xn--fa-hia.example/%C3%BC` | `8915e93cea7c04e931ef1f321308683c3a83bd483f4c31e0f982eec9fc94d4fb` |
| `https://Bücher.example/` | `xn--bcher-kva.example/` | `1f4ffbec2e2f0bdd5555cb002f4bab3bfdb986eb1b41bfeb8d92d09d52204a44` |
| `https://r3---sn-abc.example/v` | `r3---sn-abc.example/v` | `1ee608141372b3c4b99d1bb253f7c293e36d92a08cc493646245d77992d8261a` |
| `http://[2001:DB8:0:0:0:0:0:1]:8080/x` | `[2001:db8::1]/x` | `a17275fd2613b162fca61e73e5a79af9e91c7edada0fcf3b803232e0c69576b4` |
| `http://[::ffff:1.2.3.4]/` | `[::ffff:102:304]/` | `9b69ad5dfc5324b0c121cf5f910a5022cd44dbe365fde85299464e59ae79e4a3` |
| `http://[2001:db8:0:1:0:0:0:1]/` | `[2001:db8:0:1::1]/` | `62fb2b3bc95354f92ea0bdeab756172a03df7b37313fc8f16bb98a393654750d` |
| `http://192.168.1.1:8080/f` | `192.168.1.1/f` | `23f497b76e378d5bfaf967ba330c3561e35f76ce5d821d256e78b5cb707ed256` |
| `http://192.168.01.1/` | invalid | none |
| `http://192.168.1/` | invalid | none |
| `http://0x7f.0.0.1/` | invalid | none |
| `http://example.com/%zz` | invalid | none |
| `http://example.com/50%` | invalid | none |
| `http://ex%61mple.com/` | invalid | none |
| `http://[fe80::1%25eth0]/` | invalid | none |
| `http:///path` | invalid | none |
| `ftp://example.com/f` | invalid | none |
| `http://exa_mple.com/` | invalid | none |

## Hash body (http-source.md §3.1)

Files of the byte pattern of the tree section. `BLAKE3(body)` is the plain hash of the body
and serves as a check here; the protocol does not use it.

| Size | Blocks | Body | Length | Root of the file | BLAKE3(body) |
|---|---|---|---|---|---|
| 262145 | 2 | block 0 cv ‖ block 1 cv | 64 | `fad1c29152879b677f8ca0b2f7cae5a934bc0ebfb5ee642095723f09e5caa752` | `a9133b11a9b7501080c9f103f468746ad37fafd2aafef59247e8626b2ae2b2bb` |
| 525288 | 3 | node(1, 0) ‖ node(1, 1) ‖ block 0 to 2 cv; node(1, 1) is block 2 cv, carried up | 160 | `96a14a72c5e25de2dc3e81186f0d1d62791351491f41477d17f288b4ba97cc96` | `183573acb52747e76ed44ae952b82f4b9af5685c7dc79027d08f670bbf607ea1` |
| 1310720 | 5 | node(2, 0) ‖ node(2, 1) ‖ node(1, 0) to node(1, 2) ‖ block 0 to 4 cv; the values of the tree section | 320 | `103f4e723c6f36531b9d0be6e48c8c9187dfd7a6f98917a9ca878b43bc52859b` | `e30f4b6b49800f6f5fab779640a5c4ea0c720503fcdddca73f3b607f8c17e26a` |
| 1073741824 | 4096 | levels 11 down to 0 | 262080 | `d7dd364a0856680d3bcc08ce10b9b1a4da2bc6b9cc86baf253f1a7f05a981025` | `59f5234a413cf114a466050379cf836e395169605ebba9f8698acf1b8a91dd47` |

## Paid subscriptions (subscription.md §2 to §8)

The publisher has the sample key; the channel and the set have the `id` `sid1`. `t0` is
1767225600 (2026-01-01 00:00 UTC) and `period` 86400, so the sample time lies in epoch 285.
The holder key is made from the seed byte *i* = `0x6b XOR i`; the viewer's node is the node of
the second key, the receiver the node of the sample key. Messages are canonical (§0).

**Scopes** (§2)

```
scope(1, key, "sid1") = 06d88de43163c009c0f83944f914165d      live channel
scope(2, key, "sid1") = d47b2479f4900c25a6780ab1f052d520      set with an open manifest
scope(3, key, "")     = 3575cab2145043e979c679fb1a42e9f2      hidden set
```

**Season and keys** (§4.1, §4.2). Every chain step and the anchor are bound to `scope(1)`.
Season A covers epochs 270 to 299; its seed `K(299)` is 32 bytes, byte *i* = `0x1f XOR i`. A
leak in epoch 284 starts season B at 285, that epoch + 1 (§4.5); its seed `K(314)` is byte
*i* = `0x4e XOR i`. Season A is so abandoned after 284, and its tip is `K(284)`. At the sample
time, epoch 285, B is the current season and A has ended.

```
K(299) of A = 1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100
K(285) of A = 8d044e8bbbef436d6ea3c544aec44a92a892a705d0b5dc263441faa39909e78e
K(284) of A = H("ed3k v1 paid chain", scope(1) ‖ K(285) of A) = 55cd7672b3d1c58b82eeddc0f4b8561bc344d245bc798c9e5aac58a88f99e456
K(270) of A = 10cb47ae397f6dbb7ccbc0c9685141352245ca60760674724e81cac43badfea3
anchor A    = H("ed3k v1 paid anchor", scope(1) ‖ K(270)) = 2898c66d68c15eb5f6279270d6fb8362b263e5439842c2e1d577cc5d567a0ba1
tip of A    = K(284) of A
anchor B    = c7a4ac7771593c2dc3c2320f3723a8c3184e0b9df93cec5ced37fd72ad374a19
K(285) of B = 3ed36ecb23e78867e31662d45d2882acd6071013e9654709bc10ba408bd009ec
```

```
Chain A     = { scope = scope(1), base = 270, length = 30, anchor A }
            = 0a1006d88de43163c009c0f83944f914165d108e02181e22202898c66d68c15eb5f6279270d6fb8362b263e5439842c2
              e1d577cc5d567a0ba1
              (57 bytes)
signed      = "ed3k v1 paid season" ‖ Chain A      (76 bytes)
signature   = 41d6b204034f398770aad87ec1e2277b1186b28a5677c0826216108c4624faf3
              9a2d2b35cec6c519677d1cdb4228b722d2e7b61dd9f55d7b4ec9b1c90eb0590e
SignedChain = 0a390a1006d88de43163c009c0f83944f914165d108e02181e22202898c66d68c15eb5f6279270d6fb8362b263e54398
              42c2e1d577cc5d567a0ba112440801124041d6b204034f398770aad87ec1e2277b1186b28a5677c0826216108c4624fa
              f39a2d2b35cec6c519677d1cdb4228b722d2e7b61dd9f55d7b4ec9b1c90eb0590e
              (129 bytes)
```

**Offer** (§3). `price` "4.99 EUR / month", `pay_url` `https://example.com/pay`,
`pass_url` `https://example.com/pass`, `report_url` `https://example.com/report`,
`devices` 1, `issued` the sample time, `chain` the SignedChain of B, the current season.

```
Offer       = 0a1006d88de43163c009c0f83944f914165d1080e9b5d6061880f2d6ca062080a3052a81010a390a1006d88de43163c0
              09c0f83944f914165d109d02181e2220c7a4ac7771593c2dc3c2320f3723a8c3184e0b9df93cec5ced37fd72ad374a19
              124408011240bb9a200927dc8b514a732d84316dba05fa6de7dc8889a8bd7eb4f0ed4868b008541970be750f3bc8e3b3
              e4ed9ee0f12051076707bc62c3d7e6cd2ac1e6717d013a10342e393920455552202f206d6f6e7468421768747470733a
              2f2f6578616d706c652e636f6d2f7061794a1868747470733a2f2f6578616d706c652e636f6d2f70617373621a687474
              70733a2f2f6578616d706c652e636f6d2f7265706f72746801
              (265 bytes)
signature   = e73ea6dc8fe65a516cc91b34465b2be77e52989c611f8c3eadf364152f30d808
              6e8edd5e084f14bac083869113b1eea6a1dbcb00a030781a10bab5bd35e2c40c
SignedOffer = 338 bytes
```

**Pass** (§5) for `scope(1)`, signed by the publisher, `until` = 1794441600 (30 days later):

```
holder key  = 8a84b06e1255ac4f98b95041a13cd11f81113da1ac477d4265a8beea51377df0
Pass        = 0a1006d88de43163c009c0f83944f914165d12208a84b06e1255ac4f98b95041a13cd11f81113da1ac477d4265a8beea
              51377df0188083d4d7062880e9b5d606
              (64 bytes)
signature   = ecef872796b6535d1695d0297bd1223c5bc85b18a06a455585b0c0e2cd8bb8cd
              db5003cd5dfcaf73ca2da724c6edb8b182e933a1811dc21f7ad4d0bd430c7d02
SignedPass  = 0a400a1006d88de43163c009c0f83944f914165d12208a84b06e1255ac4f98b95041a13cd11f81113da1ac477d4265a8
              beea51377df0188083d4d7062880e9b5d6061220d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68
              f707511a1a4408011240ecef872796b6535d1695d0297bd1223c5bc85b18a06a455585b0c0e2cd8bb8cddb5003cd5dfc
              af73ca2da724c6edb8b182e933a1811dc21f7ad4d0bd430c7d02
              (170 bytes)
```

**Proof** (§5.2), minute 29864160:

```
viewer node = 017da93e9fc9e3079a462d8b461dfc9b
signed      = "ed3k v1 paid proof" ‖ scope(1) ‖ viewer node ‖ cf617f4479e97358c9d31741900a90ae ‖ 0000000001c7b0e0      (74 bytes)
signature   = 5c9bb5632cc0e1b5329d03e95e3c0a25f86b6319b1135b10fb1ddec574afa586
              9e20cd368822c3f6b2ad34a7bd744bcb1073f01b8ebc543477c6b582d9edc203
PassProof   = 08e0e19e0e12405c9bb5632cc0e1b5329d03e95e3c0a25f86b6319b1135b10fb1ddec574afa5869e20cd368822c3f6b2
              ad34a7bd744bcb1073f01b8ebc543477c6b582d9edc203
              (71 bytes)
```

**Key exchange** (§6.1). The request shows the pass, holds no key and asks for the offer. The
answer gives `K(285)` of season B, the key of the current epoch, the offer, the tip of the
ended season A, and both `SignedChain`s:

```
KeyRequest  = 266 bytes
KeyAnswer   = 0a25089d0212203ed36ecb23e78867e31662d45d2882acd6071013e9654709bc10ba408bd009ec1ad2020a89020a1006
              d88de43163c009c0f83944f914165d1080e9b5d6061880f2d6ca062080a3052a81010a390a1006d88de43163c009c0f8
              3944f914165d109d02181e2220c7a4ac7771593c2dc3c2320f3723a8c3184e0b9df93cec5ced37fd72ad374a19124408
              011240bb9a200927dc8b514a732d84316dba05fa6de7dc8889a8bd7eb4f0ed4868b008541970be750f3bc8e3b3e4ed9e
              e0f12051076707bc62c3d7e6cd2ac1e6717d013a10342e393920455552202f206d6f6e7468421768747470733a2f2f65
              78616d706c652e636f6d2f7061794a1868747470733a2f2f6578616d706c652e636f6d2f70617373621a68747470733a
              2f2f6578616d706c652e636f6d2f7265706f72746801124408011240e73ea6dc8fe65a516cc91b34465b2be77e52989c
              611f8c3eadf364152f30d8086e8edd5e084f14bac083869113b1eea6a1dbcb00a030781a10bab5bd35e2c40c2a25089c
              02122055cd7672b3d1c58b82eeddc0f4b8561bc344d245bc798c9e5aac58a88f99e4563281010a390a1006d88de43163
              c009c0f83944f914165d109d02181e2220c7a4ac7771593c2dc3c2320f3723a8c3184e0b9df93cec5ced37fd72ad374a
              19124408011240bb9a200927dc8b514a732d84316dba05fa6de7dc8889a8bd7eb4f0ed4868b008541970be750f3bc8e3
              b3e4ed9ee0f12051076707bc62c3d7e6cd2ac1e6717d013281010a390a1006d88de43163c009c0f83944f914165d108e
              02181e22202898c66d68c15eb5f6279270d6fb8362b263e5439842c2e1d577cc5d567a0ba112440801124041d6b20403
              4f398770aad87ec1e2277b1186b28a5677c0826216108c4624faf39a2d2b35cec6c519677d1cdb4228b722d2e7b61dd9
              f55d7b4ec9b1c90eb0590e
              (683 bytes)
```

**Sealed segment** (§7.1). The sample file, sealed with the sample nonce under `K(285)` of B:

```
k_seg       = H("ed3k v1 paid live", scope(1) ‖ K(285)) = 23560eb99d40f7c85da7088d457628578912ec732ef33a21a325939e9c393bd5
stored      = 000102030405060708090a0b86bc4e50c9206e05bd167a6d18bdb483      (28 bytes)
root        = ffeed7901652d4643f849828b5c8b38fd6482fea7f48515035cf893868737b6b
LiveSegment = { start = 1791849600, seq = 7, root, size = 28, ms = 4000, epoch = 285 }
            = 0880e9b5d60610071a20ffeed7901652d4643f849828b5c8b38fd6482fea7f48515035cf893868737b6b201c28a01f50
              9d02
              (50 bytes)
signed      = "ed3k v1 live segment" ‖ stream ‖ LiveSegment      (86 bytes)
signature   = 591833107d2a4596a0b52b179149d7cc2c2038220dddd026d7885c82ef8a5c8e
              b7d4019d092253fb83e326c2d785479dfea8073869fb81848aae1d3732540700
```

**Sealed member and hidden head** (§8.1, §8.3). The head is that of the hidden set section.
The hidden set, `scope(3)`, has a season with the seed and the epochs of B; its chain is bound
to `scope(3)`, so its keys are not those of `scope(1)`:

```
K(285)      = a9d32bd1418df908d4346912e44dcdda9fb081c1221fc7518d86f3e49fd7eb26      season B of scope(3)
k_data      = H("ed3k v1 paid data", scope(3) ‖ K(285)) = 5b55e3b45f151f419d70304ae5261fd1c44896b23496a223cd3b6b1b1036b31f
sealed file = 000102030405060708090a0b496011187114c888779feeaccfb6c729      (28 bytes)
k_head      = H("ed3k v1 paid head", scope(3) ‖ K(285)) = 9c71b0fc66b2e8637cee1cc0894c0d0bd15c25c81c99f13aeaa92aa1832ca0ae
Sealed      = { seq = 1, data = nonce ‖ ChaCha20(k_head, nonce, Head), epoch = 285 }
            = 08011238000102030405060708090a0b5341a28f32846402694c4586ac9c739ba2cb236a6f2d091a47cebbdc290517ef
              4f79ccb388becd17cdfebbd6189d02
              (63 bytes)
```

**Topic of the set** (§8.4), sample day:

```
target      = H("ed3k v1 paid topic", scope(2) ‖ 0000000000005103)[0..16) = e60266fd65e7e95d73d9e85b364b9178
```

**Share report** (§5.3). The holder key shows the pass from the viewer's node at the sample
minute and from a node with ID byte *i* = `0x2c XOR i` three minutes later; `devices` is 1:

```
ShareReport = 0a1006d88de43163c009c0f83944f914165d12aa010a400a1006d88de43163c009c0f83944f914165d12208a84b06e12
              55ac4f98b95041a13cd11f81113da1ac477d4265a8beea51377df0188083d4d7062880e9b5d6061220d75a980182b10a
              b7d54bfed3c964073a0ee172f3daa62325af021a68f707511a1a4408011240ecef872796b6535d1695d0297bd1223c5b
              c85b18a06a455585b0c0e2cd8bb8cddb5003cd5dfcaf73ca2da724c6edb8b182e933a1811dc21f7ad4d0bd430c7d021a
              10cf617f4479e97358c9d31741900a90ae225b0a10017da93e9fc9e3079a462d8b461dfc9b124708e0e19e0e12405c9b
              b5632cc0e1b5329d03e95e3c0a25f86b6319b1135b10fb1ddec574afa5869e20cd368822c3f6b2ad34a7bd744bcb1073
              f01b8ebc543477c6b582d9edc203225b0a102c2d2e2f28292a2b2425262720212223124708e3e19e0e1240dec38650be
              6528859ad8069bb343f0ccb2e601af761a22f22188ac1732a33f771ac3d6df40bb3eed37533240946078131db4c38308
              80788103142e66992a9100
              (395 bytes)
```
