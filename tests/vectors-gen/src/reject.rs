// Third batch: signatures that Protocol.md §10.7 rejects although the equation
// [S]B = R + [k]A holds when one check is skipped, and Signed forms that are invalid or
// dropped. Every crafted signature is checked here against a lax verifier (accepts) and
// against ed25519-dalek's verify_strict (rejects). Prints the section of ../vectors-v1.md.
use super::*;
use curve25519_dalek::{constants::{ED25519_BASEPOINT_POINT, EIGHT_TORSION}, edwards::{CompressedEdwardsY, EdwardsPoint}, scalar::Scalar, traits::IsIdentity};
use ed25519_dalek::{Signature, VerifyingKey};

// group order L and field prime p = 2^255 - 19, little-endian
const L_LE: [u8; 32] = [0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10];
fn p_le() -> [u8; 32] { let mut p = [0xffu8; 32]; p[0] = 0xed; p[31] = 0x7f; p }
fn add_le(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] { let mut o = [0u8; 32]; let mut c = 0u16; for i in 0..32 { let s = a[i] as u16 + b[i] as u16 + c; o[i] = s as u8; c = s >> 8; } assert_eq!(c, 0); o }
fn ge_le(a: &[u8; 32], b: &[u8; 32]) -> bool { for i in (0..32).rev() { if a[i] != b[i] { return a[i] > b[i] } } true }
fn sub_le(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] { let mut o = [0u8; 32]; let mut br = 0i16; for i in 0..32 { let d = a[i] as i16 - b[i] as i16 - br; o[i] = (d & 0xff) as u8; br = if d < 0 { 1 } else { 0 }; } assert_eq!(br, 0); o }

// what a lax implementation does: reduce the y coordinate mod p, keep the sign bit, no order check
fn decode_lax(b: &[u8; 32]) -> Option<EdwardsPoint> {
    let sign = b[31] & 0x80; let mut y = *b; y[31] &= 0x7f;
    if ge_le(&y, &p_le()) { y = sub_le(&y, &p_le()) }
    y[31] |= sign; CompressedEdwardsY(y).decompress()
}
fn k_of(r: &[u8], a: &[u8], m: &[u8]) -> Scalar { let h: [u8; 64] = sha2::Sha512::new().chain_update(r).chain_update(a).chain_update(m).finalize().into(); Scalar::from_bytes_mod_order_wide(&h) }
fn lax_ok(a: &[u8; 32], m: &[u8], sig: &[u8; 64]) -> bool {
    let (Some(pa), Some(pr)) = (decode_lax(a), decode_lax(sig[..32].try_into().unwrap())) else { return false };
    let s = Scalar::from_bytes_mod_order(sig[32..].try_into().unwrap());
    s * ED25519_BASEPOINT_POINT == pr + k_of(&sig[..32], a, m) * pa
}
fn strict_ok(a: &[u8; 32], m: &[u8], sig: &[u8; 64]) -> bool { VerifyingKey::from_bytes(a).map(|v| v.verify_strict(m, &Signature::from_bytes(sig)).is_ok()).unwrap_or(false) }
fn nc(canonical: &[u8; 32], y: u8) -> [u8; 32] { let mut v = add_le(&p_le(), &{ let mut z = [0u8; 32]; z[0] = y; z }); v[31] |= canonical[31] & 0x80; v }
// smallest S from 1 up such that, with R = [S]B, [k]A is the identity: then [S]B = R + [k]A
fn craft(a_bytes: &[u8; 32], a: &EdwardsPoint, m: &[u8]) -> ([u8; 64], u64) {
    for n in 1u64.. {
        let s = Scalar::from(n); let r = (s * ED25519_BASEPOINT_POINT).compress().to_bytes();
        if (k_of(&r, a_bytes, m) * a).is_identity() { let mut sig = [0u8; 64]; sig[..32].copy_from_slice(&r); sig[32..].copy_from_slice(s.as_bytes()); return (sig, n) }
    }
    unreachable!()
}
fn two(sig: &[u8; 64]) -> String { format!("{}\n              {}", hx(&sig[..32]), hx(&sig[32..])) }
fn wrapn(hex: &str, ind: usize) -> String { hex.as_bytes().chunks(96).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(&format!("\n{}", " ".repeat(ind))) }

pub fn run(pk1: &[u8], seed1: &[u8], root: &[u8]) {
    let a_bytes: [u8; 32] = pk1.try_into().unwrap();
    let mut a_scalar = sha2::Sha512::digest(seed1)[..32].to_vec(); a_scalar[0] &= 248; a_scalar[31] &= 127; a_scalar[31] |= 64;
    let a = Scalar::from_bytes_mod_order(a_scalar.as_slice().try_into().unwrap());
    assert_eq!((a * ED25519_BASEPOINT_POINT).compress().to_bytes(), a_bytes);
    // the signed KEYWORD record of the file
    let file = cat(&[&fb(1, root), &fv(2, 16), &fb(3, b"The.Big_Sleep (1946) [1080p].mkv")]);
    let body = cat(&[&fv(1, 1791936000), &fb(2, &file)]);
    let target = h("ed3k v1 keyword", &cat(&[&[0u8], b"sleep"]))[..16].to_vec();
    let m = cat(&[b"ed3k v1 record", &[2u8], &target, &body]); assert_eq!(m.len(), 109);
    let sk = SigningKey::from_bytes(seed1.try_into().unwrap());
    let good: [u8; 64] = sk.sign(&m).to_bytes(); assert_eq!(hx(&good[..8]), "b1c5bcac6211b9e5");
    assert!(strict_ok(&a_bytes, &m, &good) && lax_ok(&a_bytes, &m, &good));

    // 1. S not below L
    let mut s_big = good; let s2 = add_le(good[32..].try_into().unwrap(), &L_LE); s_big[32..].copy_from_slice(&s2);
    // 2. R the identity, canonical; S = k·a
    let id = EIGHT_TORSION[0].compress().to_bytes(); assert_eq!(hx(&id), format!("01{}", "00".repeat(31)));
    let mut r_id = [0u8; 64]; r_id[..32].copy_from_slice(&id); r_id[32..].copy_from_slice((k_of(&id, &a_bytes, &m) * a).as_bytes());
    // 3. R the identity, written as y = 1 + p
    let id_nc = nc(&id, 1);
    let mut r_nc = [0u8; 64]; r_nc[..32].copy_from_slice(&id_nc); r_nc[32..].copy_from_slice((k_of(&id_nc, &a_bytes, &m) * a).as_bytes());
    // 4. A of order 8, canonical
    let t8 = EIGHT_TORSION[1]; let a8 = t8.compress().to_bytes(); let (sig8, n8) = craft(&a8, &t8, &m);
    // 5. A of order 4 (y = 0), written as y = 0 + p
    let t4 = EIGHT_TORSION[2]; let a4 = t4.compress().to_bytes(); assert_eq!(&a4[..31], &[0u8; 31]); let a4_nc = nc(&a4, 0); let (sig4, n4) = craft(&a4_nc, &t4, &m);
    for (name, key, sig) in [("S >= L", &a_bytes, &s_big), ("R identity", &a_bytes, &r_id), ("R non-canonical", &a_bytes, &r_nc), ("A order 8", &a8, &sig8), ("A non-canonical", &a4_nc, &sig4)] {
        assert!(lax_ok(key, &m, sig), "lax verifier should accept: {name}");
        assert!(!strict_ok(key, &m, sig), "verify_strict should reject: {name}");
    }
    // Signed forms
    let signed = |key: u64, sigs: &[Vec<u8>]| { let mut o = cat(&[&fb(1, &body), &fv(2, key)]); for s in sigs { o.extend(fm(3, s)) } o };
    let s_ok = sigmsg(&good); let s_alg0 = fb(2, &good);
    let (f_ok, f_alg0, f_two, f_key) = (signed(0, &[s_ok.clone()]), signed(0, &[s_alg0]), signed(0, &[s_ok.clone(), s_ok.clone()]), signed(1, &[s_ok]));
    if let Some(d) = std::env::args().nth(2) { for (n, f) in [("signed_ok.bin", &f_ok), ("signed_alg0.bin", &f_alg0), ("signed_two.bin", &f_two), ("signed_key1.bin", &f_key)] { std::fs::write(format!("{d}/{n}"), f).unwrap() } }
    let tail = |f: &[u8]| wrapn(&hx(&f[2 + body.len()..]), 20);

    println!("\n### Signatures and forms that are rejected

The same `signed` bytes, 109 of them, under the sample key unless a key is given. For each of
the five signatures the equation `[S]B = R + [k]A` holds, with `k` taken over the bytes as
they are sent, once the check named is skipped: a verifier that skips it accepts. §10.7
rejects every one, and so does `verify_strict` of ed25519-dalek 2.2.0. L is the group order,
p = 2²⁵⁵ − 19.

```
S not below L: the signature above with S + L
  R ‖ S     = {}
R of small order: the identity, S = k · a (a = the secret scalar of the sample key)
  R ‖ S     = {}
R not canonical: the identity written as y = 1 + p, S = k · a with k over these bytes
  R ‖ S     = {}
A of small order: the point of order 8, S = {}, R = [S]B; k · A is the identity
  A         = {}
  R ‖ S     = {}
A not canonical: the point of order 4 (y = 0) written as y = p, S = {}, R = [S]B
  A         = {}
  R ‖ S     = {}
```

A verifier that skips the check in question computes the same `k` and the same point on each
side; it is the rule of §10.7 alone that rejects. The `Signed` forms below carry the valid
signature; the `body` is the 78 bytes above, written as `0a4e ‖ body`.

```
valid               Signed{{ body, sigs = [ {{ alg = 1, sig }} ] }}
  after the body    = {}
alg = 0             Signed{{ body, sigs = [ {{ sig }} ] }}: invalid, `alg` 0
  after the body    = {}
two Ed25519         Signed{{ body, sigs = [ {{ alg = 1, sig }}, {{ alg = 1, sig }} ] }}: invalid, one per algorithm
  after the body    = {}
key out of range    Signed{{ body, key = 1, sigs = [ {{ alg = 1, sig }} ] }} in a KadAnswer with one key: dropped
  after the body    = {}
```",
        two(&s_big), two(&r_id), two(&r_nc), n8, hx(&a8), two(&sig8), n4, hx(&a4_nc), two(&sig4),
        tail(&f_ok), tail(&f_alg0), tail(&f_two), tail(&f_key));
}
