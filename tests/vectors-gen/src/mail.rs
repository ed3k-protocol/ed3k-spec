// Offline messages (mail.md §1 to §3). Prints the section of ../vectors-v1.md as markdown.
use super::*;
use ext::{conv, w};

fn sig2(s: &[u8]) -> String { format!("{}\n              {}", hx(&s[..32]), hx(&s[32..])) }
fn f64x(f: u32, v: u64) -> Vec<u8> { let mut o = vec![]; if v == 0 { return o } vint(((f << 3) | 1) as u64, &mut o); o.extend(v.to_le_bytes()); o }
// field present even at 0 (proto3 `optional`)
fn fvo(f: u32, v: u64) -> Vec<u8> { let mut o = vec![]; vint((f << 3) as u64, &mut o); vint(v, &mut o); o }

pub(crate) fn box_of(key: &[u8]) -> u16 { let b = h("ed3k v1 mail box", key); u16::from_be_bytes([b[0], b[1]]) }
pub(crate) fn bucket(bx: u16, b: u8) -> u16 { if b == 0 { 0 } else { bx & (0xffffu16 << (16 - b as u32)) } }
pub(crate) fn target(b: u8, bucket: u16, period: u64) -> Vec<u8> { h("ed3k v1 mail target", &cat(&[&[b], &bucket.to_be_bytes(), &period.to_be_bytes()]))[..16].to_vec() }

// sealed record (§2): e(32) ‖ nonce(12) ‖ AEAD(k_mail, plain of 964 B)
pub(crate) fn seal(x: &[u8], to_x: &[u8], to: &[u8], nonce: &[u8], part: &[u8]) -> (Vec<u8>, [u8; 32], [u8; 32], Vec<u8>) {
    assert!(part.len() <= 962);
    let e = x25519_base(x);
    let ss = x25519(x, to_x); assert_ne!(ss, [0u8; 32]);
    let k = h("ed3k v1 mail", &cat(&[&ss, &e, to]));
    let mut plain = cat(&[&(part.len() as u16).to_be_bytes(), part]); plain.resize(964, 0);
    let s = cat(&[&e, nonce, &aead(&k, nonce, &plain)]); assert_eq!(s.len(), 1024);
    (s, ss, k, plain)
}

pub fn run(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8]) {
    let sign = |seed: &[u8], m: &[u8]| { let sk = SigningKey::from_bytes(seed.try_into().unwrap()); let s = sk.sign(m); sk.verifying_key().verify_strict(m, &s).unwrap(); s.to_bytes().to_vec() };
    let period: u64 = 20739; let t: u64 = period * 86400;

    // mailbox of the sample key (§1)
    let bx = box_of(pk1);
    let rows: Vec<(u8, u16, Vec<u8>)> = [8u8, 9, 10, 0, 16].iter().map(|&b| { let bk = bucket(bx, b); (b, bk, target(b, bk, period)) }).collect();
    let next = target(8, bucket(bx, 8), period + 1);
    let bx2 = box_of(pk2);

    // payload: a Chat of 1,200 B text, type "chat", sender = second key, reply 8
    let text: Vec<u8> = (0..1200usize).map(|i| b"abcdefghijklmnopqrstuvwxyz"[i % 26]).collect();
    let chat_id: u64 = 0x0102030405060708;
    let payload = cat(&[&fb(1, &text), &f64x(2, chat_id)]);
    let phash = h("ed3k v1 mail payload", &payload);
    let msg: Vec<u8> = (0..16u8).map(|i| 0x4d ^ i).collect();
    let (count, reply, ty) = (2u64, 8u64, b"chat");
    let spre = cat(&[b"ed3k v1 mail signature", pk1, &msg, &[count as u8], &(payload.len() as u64).to_be_bytes(), &phash, &t.to_be_bytes(), &[reply as u8], &[ty.len() as u8], ty]);
    let ssig = sign(seed2, &spre);
    let head = cat(&[&fb(1, pk2), &fv(2, payload.len() as u64), &fb(3, &phash), &fv(4, t), &fb(5, ty), &fvo(6, reply), &fm(7, &sigmsg(&ssig))]);
    // part 0 takes as much as fits in 962 B, part 1 the rest
    let part = |i: u64, d: &[u8], hd: Option<&[u8]>| cat(&[&fb(1, &msg), &fv(2, i), &fv(3, count), &fb(4, d), &hd.map(|x| fm(5, x)).unwrap_or_default()]);
    let mut n0 = payload.len();
    while part(0, &payload[..n0], Some(&head)).len() > 962 { n0 -= 1 }
    let p0 = part(0, &payload[..n0], Some(&head));
    let p1 = part(1, &payload[n0..], None);
    assert!(p1.len() <= 962);

    // part 0 sealed with (x, e) and the sample nonce, part 1 with (y, f) and nonce byte i = 0x10 + i
    let (to_x, _) = conv(seed1);
    let x = unhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    let y = unhex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    let n0b = unhex("000102030405060708090a0b");
    let n1b: Vec<u8> = (0..12u8).map(|i| 0x10 + i).collect();
    let (s0, ss0, k0, _) = seal(&x, &to_x, pk1, &n0b, &p0);
    let (s1, ss1, k1, _) = seal(&y, &to_x, pk1, &n1b, &p1);
    // recipient side: open with its converted private key
    let (_, to_priv) = conv(seed1);
    assert_eq!(x25519(&to_priv, &s0[..32]), ss0); assert_eq!(x25519(&to_priv, &s1[..32]), ss1);
    let back = |s: &[u8], k: &[u8; 32]| { let p = ChaCha20Poly1305::new(k.into()).decrypt(s[32..44].into(), &s[44..]).unwrap(); let l = u16::from_be_bytes([p[0], p[1]]) as usize; p[2..2 + l].to_vec() };
    assert_eq!(back(&s0, &k0), p0); assert_eq!(back(&s1, &k1), p1);

    // acknowledgement (§3.2): MailAck of the message, its head would be signed by the sample key
    let ack = fb(1, &msg);

    println!("
## Offline messages (mail.md §1 to §3)

The recipient's mailbox key is the sample key; the sender is the second key. Period is the
sample period, `sent` the sample time. `msg` is 16 bytes, byte *i* = `0x4d XOR i`.

**Mailbox** (mail.md §1):

```
box  = H(\"ed3k v1 mail box\", key)[0..2] = {:04x}
target = H(\"ed3k v1 mail target\", b(1) ‖ bucket(2) ‖ period(8))[0..16)",
        bx);
    for (b, bk, tg) in &rows { println!("  b = {b:<2}  bucket {bk:04x}  target {}", hx(tg)); }
    println!("  b = 8, next day (period 20740)  target {}", hx(&next));
    println!("box of the second key = {bx2:04x}");
    println!("```

`b` = 9 and 10 are the spill targets of `b` = 8 (mail.md §1).

**A two-part message** (mail.md §2, §3.1). Payload = `Chat` with `text` = 1,200 ASCII bytes
`abc…z` repeated and `id` = `{chat_id:016x}`; `type` `chat`, `reply` 8. Part 0 holds as much
of the payload as fits in 962 bytes:

```
payload     = {} bytes, starts {}…
hash        = H(\"ed3k v1 mail payload\", payload) = {}
signed      = \"ed3k v1 mail signature\" ‖ to ‖ msg ‖ {:02x} ‖ {} ‖ hash ‖ {} ‖ {:02x} ‖ {:02x} ‖ \"chat\"      ({} bytes)
signature   = {}
MailHead    = {}
              ({} bytes)
part 0      = {{ msg, index 0, count 2, data = payload[0..{n0}), head }}      ({} bytes)
            = {}
part 1      = {{ msg, index 1, count 2, data = payload[{n0}..{}) }}      ({} bytes)
            = {}
```",
        payload.len(), hx(&payload[..16]), hx(&phash),
        count, hx(&(payload.len() as u64).to_be_bytes()), hx(&t.to_be_bytes()), reply, ty.len(), spre.len(),
        sig2(&ssig), w(&hx(&head), 14), head.len(), p0.len(), w(&hx(&p0), 14), payload.len(), p1.len(), w(&hx(&p1), 14));

    println!("
Part 0 is sealed with `(x, e)` and the sample nonce, part 1 with `(y, f)` and the nonce
`101112131415161718191a1b`. The recipient's X25519 key is the sample key converted
(Protocol.md §6.4), `{}`:

```
part 0
X25519      = X25519(x, recipient) = {}
k_mail      = H(\"ed3k v1 mail\", X25519 ‖ e ‖ key) = {}
plain       = {:04x} ‖ part 0 ‖ {} zero bytes      (964 bytes)
sealed      = e ‖ nonce ‖ ChaCha20-Poly1305(k_mail, nonce, plain)      (1024 bytes)
              {}

part 1
X25519      = X25519(y, recipient) = {}
k_mail      = {}
plain       = {:04x} ‖ part 1 ‖ {} zero bytes      (964 bytes)
sealed      = {}
```

**Acknowledgement** (mail.md §3.2): `MailAck` = `{}` (payload of type `ack`).",
        hx(&to_x),
        hx(&ss0), hx(&k0), p0.len(), 962 - p0.len(), w(&hx(&s0), 14),
        hx(&ss1), hx(&k1), p1.len(), 962 - p1.len(), w(&hx(&s1), 14),
        hx(&ack));
}
