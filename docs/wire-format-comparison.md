# Wire format: protobuf or a self-describing variant

**Status:** design note (2026-10-10). Informative; changes nothing in wire version 1.
**Relates to:** [`Protocol.md`](../Protocol.md) §5.1 (frame) and §16 (versioning). Section numbers (§) refer to that document.

ed3k encodes every message as proto3. This note records why, measured against the most plausible
alternative for a peer-to-peer protocol: a self-describing tagged variant. The variant examined is
`CVariant` from
[TaskExplorer](https://github.com/DavidXanatos/TaskExplorer/blob/master/MiscHelpers/Common/Variant.h),
read from `Variant.h`, `VariantDefs.h` and the encoder in `Variant.cpp` as of 2026-10-10.

**Result:** protobuf stays. It is roughly 35–45% smaller on small messages and is the stronger base for a
protocol with several implementations. The variant has one property protobuf lacks out of the
box, extension without a number registry; §5 below shows how to get it in protobuf.

---

## 1. The two encodings

### 1.1 proto3

```
field   = key  value
key     = varint  (field_number << 3) | wire_type      1 byte for fields 1–15, 2 for 16–2047
value   = varint                                       integers, bools, enums: 1–10 bytes by value
        | 4 or 8 fixed bytes                           fixed32/64, float, double
        | varint length, then that many bytes          bytes, string, nested message, packed list
```

A field that holds its default value is not written. The reader needs the schema to know what a
field means; without it, it still knows how long the field is.

### 1.2 CVariant

```
variant = head [length] payload
head    = 1 byte: bits 0–4 type, bit 5 "length field follows", bits 6–7 size code
          bit 5 clear: size code is the payload size, 1, 2, 4 or 8 bytes; no length field
          bit 5 set:   size code is the width of the length field, 1, 2 or 4 bytes
          type empty:  the head alone, no length and no payload
payload = raw value, or the concatenated entries of a container

map   entry = 1 byte name length, name, variant        "map":   keyed by name
index entry = 4 byte key, variant                      "index": keyed by uint32, used as four characters
list  entry = variant
```

Types: empty, map, list, index, bytes, ASCII, UTF-8, UTF-16, unsigned and signed integer, double.
Every value states its own type and length, so a packet can be decoded with no schema at all.
Integers are written as the raw 1, 2, 4 or 8 bytes of the writer's C++ type. A packet is a name
(1 byte length + text) followed by one variant.

---

## 2. Byte overhead

### 2.1 Per item

| Item | proto3 | CVariant index | CVariant map |
|---|---|---|---|
| Field key | 1 B (fields 1–15), 2 B (16–2047) | 4 B | 1 B + name |
| Type and length | in the key; +1 B length for bytes, strings, messages under 128 B (2 B up to 16 KB) | 1 B head; +1 B length up to 255 B, +2 B up to 64 KB, +4 B beyond; no length when the payload is exactly 1, 2, 4 or 8 B | same |
| `uint32` holding 5 | 1 B | 4 B | 4 B |
| `uint64` holding 700 MB | 5 B | 8 B | 8 B |
| Field at its default | 0 B, omitted | written in full | written in full |
| 16 B hash as a field | 18 B | 22 B | 19 B + name |
| List of *n* `uint32` as a field | 2 B + 1–5 B each | 6 B + 5 B each | 3 B + name + 5 B each |
| Nested message under 128 B | 2 B | 6 B | 3 B + name |
| Message envelope | 2 B frame (§5.1); 3 B from a 128 B body | 1 B + name; the variant's own head follows | same |
| Maximum size of a message or of one field | 2 GiB − 1 B (sizes are a signed 32-bit integer) | 4 GiB − 1 B (length field of at most 4 B) | same; a name is at most 254 B |

Both formats parse a message whole into memory, and neither ceiling is reached in ed3k: a frame
body is capped at 1 MiB (§5.2) and file data travels as raw bytes outside the frame (§12.3).

### 2.2 Worked example

A hello-sized message: a 16 B ID, port 4662, version 1, a 700 MB size and a 20-character name.
The values alone are 50 B.

| Field | proto3 | CVariant index | CVariant map, 6-character names |
|---|---|---|---|
| ID, 16 B | 18 | 22 | 25 |
| port, `uint16` | 3 | 7 | 10 |
| version, `uint32` | 2 | 9 | 12 |
| size, `uint64` | 6 | 13 | 16 |
| name, 20 B | 22 | 26 | 29 |
| container head | 0 | 2 | 2 |
| **total** | **51** | **79** | **94** |

Protobuf lands at the size of the raw values because varints win back what the keys cost. The
variant is 55% larger keyed by index and 84% larger keyed by name.

### 2.3 Where it matters

- **Kad.** Requests are about 20 B and answers a few hundred, sent in large numbers. In the
  datagram binding ([`kad-dgram`](../extensions/kad-datagram.md)) an answer has to fit one
  datagram, so per-field overhead decides how many contacts or records it carries.
- **Source exchange and search results.** Many small repeated records; the per-entry key cost of
  the variant repeats with every record.
- **Not for bulk data.** Hash sets and `Block` payloads (§12.3) are opaque bytes in both formats.
  The difference is a few bytes per megabyte.

zstd (§5.3) removes most of the repeated keys from a large variant message, but small messages
are sent uncompressed, and those are the ones where the overhead is largest.

---

## 3. Upgradability

Both formats let an old reader skip a field it does not know, which is all §16 asks of the
encoding. They differ in everything around that.

| Aspect | proto3 | CVariant |
|---|---|---|
| Unknown fields | skipped by wire type | skipped by length |
| What defines a message | the `.proto` file: normative, machine-checked, diffable | prose and each implementation's code; there is no schema language |
| Second implementation | generated code for every common language | one C++ class; the wire format is defined by that code and has to be ported by hand |
| Retiring a field | `reserved` stops the number being reused | by convention |
| Widening an integer | `uint32` → `uint64` is wire-compatible | reader zero-extends or truncates when the dropped bytes are zero, so it works too |
| Changing a field's type | undetected unless the wire type differs | detected: the type travels with the value |
| Presence | `optional`, `oneof` | a key is there or it is not; no `oneof` |
| Adding a field without coordination | needs a number nobody else uses | a four-character key or a name is unlikely to collide |
| Decoding without the schema | lengths and numbers only (`protoc --decode_raw`) | full: keys, types, values |
| Portability | fully specified | integers are host-order memory with no byte swap; the UTF-16 type is written as `wchar_t`, 2 B on Windows and 4 B elsewhere |
| Duplicate keys | last one wins, specified | unspecified; a lookup on an unparsed map returns the first match |
| Tooling | Wireshark, linters, breaking-change checkers, fuzzed parsers | none outside the source |

The variant's real strengths are the two rows it wins: a packet explains itself, and a third
party can add a field without asking anyone. The first is a debugging convenience. The second is
a property a peer-to-peer network with independent clients can make use of, and §5 covers it.

The portability rows are not flaws of the idea, only of this implementation as a wire format
between different machines; a specification could fix byte order and drop the UTF-16 type. That
specification, and a second implementation of it, would have to be written first.

---

## 4. Canonical form and signatures

Neither format is canonical. Protobuf allows any field order and non-minimal varints. The variant
writes map entries sorted when built in memory but in call order when streamed, and the same
number can be 1, 2, 4 or 8 bytes wide.

The rule of §16 therefore holds for either choice: anything signed travels as `bytes` and is
verified over exactly those bytes. Switching the format would not remove it.

---

## 5. Extension without a registry

The one thing worth taking from the variant. Two ways to give protobuf the same property:

**A. A private field-number range.** Every message reserves, say, fields 1000–1999 for private
use. A client may put anything there; such a field is never standardised in place but gets a
regular number when adopted. Costs nothing on the wire for anyone who does not use it, and a
private field costs a 2 B key.

**B. A keyed extension field.**

```protobuf
message Ext {
  fixed32 key   = 1;   // four characters
  bytes   value = 2;
}
// in a message that third parties are likely to extend:
repeated Ext ext = 15;
```

This is the variant's index map inside protobuf: about 9 B of overhead per entry, paid only by
those who use it, with the same low collision risk.

**Recommendation: A.** It adds no second mechanism and no bytes. A range of 1,000 numbers does not
rule out collisions between two independent clients, so a private field has to be treated as
meaningful only together with the sender's `agent` string from `Hello` (§5.5). B is the
better choice only if collisions between many independent clients turn out to be a real problem.

Neither is part of wire version 1.

---

## 6. Decision

| Criterion | Better |
|---|---|
| Bytes on the wire, small messages | protobuf, by roughly 35–45% |
| Bytes on the wire, bulk data | no difference |
| Old readers skipping new fields | no difference |
| Schema, tooling, other languages | protobuf |
| Extension without coordination | variant; protobuf with §5 |
| Decoding without a schema | variant |
| Canonical form | neither |

ed3k keeps proto3 for every message.
