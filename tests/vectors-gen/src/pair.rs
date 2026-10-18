// Pairwise keys: hop session (hops.md §3.2, §3.3) and friend pair (friend.md §1).
// Prints both parts of ../vectors-v1.md as markdown.
use super::*;
use ext::{conv, w};

fn hn(ctx: &str, d: &[u8], n: usize) -> Vec<u8> { let mut o = vec![0u8; n]; blake3::Hasher::new_derive_key(ctx).update(d).finalize_xof().fill(&mut o); o }
fn nonce(n: u64) -> Vec<u8> { cat(&[&[0u8; 4], &n.to_be_bytes()]) }
fn sig2(s: &[u8]) -> String { format!("{}\n                  {}", hx(&s[..32]), hx(&s[32..])) }

pub fn run(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8]) {
    hop_session(pk1, seed1, pk2, seed2);
    friend_pair(pk1, seed1, pk2, seed2);
}

// A = second key with (x, e) as (a, e_A); B = sample key with (y, f) as (b, e_B)
fn hop_session(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8]) {
    let sign = |seed: &[u8], m: &[u8]| { let sk = SigningKey::from_bytes(seed.try_into().unwrap()); let s = sk.sign(m); sk.verifying_key().verify_strict(m, &s).unwrap(); s.to_bytes().to_vec() };
    let x = unhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    let y = unhex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    let (e, f) = (x25519_base(&x), x25519_base(&y));
    let ss = x25519(&x, &f); assert_eq!(ss, x25519(&y, &e)); assert_ne!(ss, [0u8; 32]);
    assert_eq!(hx(&ss), "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");
    let okm = hn("ed3k v1 hop session", &cat(&[&ss, &e, &f]), 96);
    assert_eq!(okm[..32], h("ed3k v1 hop session", &cat(&[&ss, &e, &f])));
    let k_b: [u8; 32] = okm[0..32].try_into().unwrap();
    let k_a: [u8; 32] = okm[32..64].try_into().unwrap();
    let k_s: [u8; 32] = okm[64..96].try_into().unwrap();
    let sid = h("ed3k v1 hop session id", &k_s)[..16].to_vec();
    let pre_b = cat(&[b"ed3k v1 hop session sig", &[0x02], &e, &f]); let sig_b = sign(seed1, &pre_b);
    let pre_a = cat(&[b"ed3k v1 hop session sig", &[0x01], &e, &f, pk1]); let sig_a = sign(seed2, &pre_a);
    let n0 = nonce(0);
    let msg1 = cat(&[&[0x01], &e]);
    let msg2 = cat(&[&f, &aead(&k_b, &n0, &cat(&[pk1, &sig_b]))]);
    let msg3 = aead(&k_a, &n0, &cat(&[pk2, &sig_a]));
    assert_eq!((msg1.len(), msg2.len(), msg3.len()), (33, 144, 112));
    let skey = |num: u64, dir: u8| h("ed3k v1 hop session stream", &cat(&[&k_s, &num.to_be_bytes(), &[dir]]));
    let (ka0, kb0) = (skey(0, 0x00), skey(0, 0x01));
    let record = |key: &[u8; 32], n: u64, plain: &[u8]| { let c = aead(key, &nonce(n), plain); cat(&[&(c.len() as u16).to_be_bytes(), &c]) };
    let r0 = record(&ka0, 0, b"ed3k"); let r1 = record(&ka0, 1, b"");
    assert_eq!((r0.len(), r1.len()), (22, 18));
    let further = cat(&[&[0x02], &sid, &2u64.to_be_bytes()]);

    println!("
## Hop session (hops.md §3.2 and §3.3)

A opens. It has the second key and the pair `(x, e)` as `(a, e_A)`; B has the sample key and
the pair `(y, f)` as `(b, e_B)`. Neither hides, so the keys are their node keys. `seal` has
the nonce `000000000000000000000000`.

```
ss              = X25519(x, f) = {}      ss2 of the hop keys above
k_b ‖ k_a ‖ k_s = H(\"ed3k v1 hop session\", ss ‖ e ‖ f), 96 bytes
k_b             = {}
k_a             = {}
k_s             = {}
sid             = H(\"ed3k v1 hop session id\", k_s)[0..16) = {}
signed by B     = \"ed3k v1 hop session sig\" ‖ 02 ‖ e ‖ f      ({} bytes)
sig_B           = {}
signed by A     = \"ed3k v1 hop session sig\" ‖ 01 ‖ e ‖ f ‖ sample key      ({} bytes)
sig_A           = {}
```

The three messages that open the first stream:

```
A → B  = 01 ‖ e      ({} bytes)
B → A  = f ‖ seal(k_b, sample key ‖ sig_B)      ({} bytes)
       = {}
A → B  = seal(k_a, second key ‖ sig_A)      ({} bytes)
       = {}
```

The first stream has the number 0. A sends one record with the plaintext `ed3k` and then ends
its direction:

```
key(0, 00) = H(\"ed3k v1 hop session stream\", k_s ‖ 0000000000000000 ‖ 00)
           = {}
key(0, 01) = H(\"ed3k v1 hop session stream\", k_s ‖ 0000000000000000 ‖ 01)
           = {}
record 0   = len ‖ ChaCha20-Poly1305(key(0, 00), nonce 000000000000000000000000, \"ed3k\")
           = {}      ({} bytes)
record 1   = len ‖ ChaCha20-Poly1305(key(0, 00), nonce 000000000000000000000001, empty)
           = {}      ({} bytes, the end of A's direction)
```

A further stream that A opens with the number 2 starts with

```
02 ‖ sid ‖ 0000000000000002 = {}      ({} bytes)
```",
        hx(&ss), hx(&k_b), hx(&k_a), hx(&k_s), hx(&sid), pre_b.len(), sig2(&sig_b), pre_a.len(), sig2(&sig_a),
        msg1.len(), msg2.len(), w(&hx(&msg2), 9), msg3.len(), w(&hx(&msg3), 9),
        hx(&ka0), hx(&kb0), hx(&r0), r0.len(), hx(&r1), r1.len(), hx(&further), further.len());
}

// the sample key and the second key are friends; ss as in the Kad datagrams
fn friend_pair(pk1: &[u8], seed1: &[u8], pk2: &[u8], seed2: &[u8]) {
    let (p1, s1) = conv(seed1); let (p2, s2) = conv(seed2);
    let ss = x25519(&s1, &p2); assert_eq!(ss, x25519(&s2, &p1)); assert_ne!(ss, [0u8; 32]);
    assert_eq!(hx(&ss), "5166f24a6918368e2af831a4affadd97af0ac326bdf143596c045967cc00230e");
    let period = 20739u64.to_be_bytes();
    let dir = |publ: &[u8], read: &[u8]| {
        let k = h("ed3k v1 friend pair", &cat(&[&ss, publ, read]));
        let k_rec = h("ed3k v1 friend record", &k);
        let target = h("ed3k v1 friend target", &cat(&[&k, &period]))[..16].to_vec();
        (k, k_rec, target)
    };
    let (k12, r12, t12) = dir(pk1, pk2); let (k21, r21, t21) = dir(pk2, pk1);

    println!("
**Friend pair** (§1). The sample key and the second key are friends. `ss` is the one of the
Kad datagrams below, from the two keys converted as in §6.4; `period` is the sample period.

```
ss     = X25519(x_private of one key, x_public of the other)
       = {}

the sample key publishes, the second key reads:
k      = H(\"ed3k v1 friend pair\", ss ‖ sample key ‖ second key)
       = {}
k_rec  = H(\"ed3k v1 friend record\", k)                = {}
target = H(\"ed3k v1 friend target\", k ‖ period)[0..16) = {}

the second key publishes, the sample key reads:
k      = H(\"ed3k v1 friend pair\", ss ‖ second key ‖ sample key)
       = {}
k_rec  = H(\"ed3k v1 friend record\", k)                = {}
target = H(\"ed3k v1 friend target\", k ‖ period)[0..16) = {}
```",
        hx(&ss), hx(&k12), hx(&r12), hx(&t12), hx(&k21), hx(&r21), hx(&t21));
}
