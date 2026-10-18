// Generator of the values in ../vectors-v1.md that were added on 2026-10-13: routing ID,
// whole datagrams, trees, tokeniser targets, sets, torrent manifest, hidden set, friend
// records, LAN announce, key conversion, hop keys, live segment; ext.rs, paid.rs, reject.rs, pair.rs
// (hop session, friend pair) and credit.rs (credit signatures) print
// the later batches as markdown. The older values of that
// file (targets, obfuscation keys, user hash, the signed KEYWORD record) are only asserted
// here where a new value builds on them.
//
//   python3 tok.py words.txt                 words and keywords of the tokeniser cases (§10.1)
//   cargo run -- words.txt [dump-dir]        prints every value; with dump-dir also writes the
//                                            serialised messages, to read back with protoc --decode
//
// Random values of the protocol (salts, padding, nonces, X25519 private values) are fixed.
use blake3::hazmat::{merge_subtrees_non_root, merge_subtrees_root, HasherExt, Mode};
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20poly1305::{aead::{Aead, KeyInit}, ChaCha20Poly1305};
use curve25519_dalek::montgomery::MontgomeryPoint;
use ed25519_dalek::{Signer, SigningKey};
use sha2::Digest;

mod credit;
mod ext;
mod paid;
mod pair;
mod reject;

fn hx(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
fn unhex(s: &str) -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() }
fn h(ctx: &str, d: &[u8]) -> [u8; 32] { blake3::derive_key(ctx, d) }
fn cat(parts: &[&[u8]]) -> Vec<u8> { parts.concat() }
fn wrap(label: &str, b: &[u8]) { let s = hx(b); println!("{label} ({} B)", b.len()); for c in s.as_bytes().chunks(96) { println!("    {}", std::str::from_utf8(c).unwrap()); } }

// protobuf, canonical: caller writes fields in ascending order, defaults left out
fn vint(mut v: u64, o: &mut Vec<u8>) { loop { let b = (v & 0x7f) as u8; v >>= 7; if v == 0 { o.push(b); break } o.push(b | 0x80) } }
fn fb(f: u32, d: &[u8]) -> Vec<u8> { let mut o = vec![]; if d.is_empty() { return o } vint(((f << 3) | 2) as u64, &mut o); vint(d.len() as u64, &mut o); o.extend(d); o }
fn fm(f: u32, d: &[u8]) -> Vec<u8> { let mut o = vec![]; vint(((f << 3) | 2) as u64, &mut o); vint(d.len() as u64, &mut o); o.extend(d); o }
fn fv(f: u32, v: u64) -> Vec<u8> { let mut o = vec![]; if v == 0 { return o } vint((f << 3) as u64, &mut o); vint(v, &mut o); o }
fn sigmsg(sig: &[u8]) -> Vec<u8> { cat(&[&fv(1, 1), &fb(2, sig)]) }

fn crc32c(d: &[u8]) -> u32 { let mut c = !0u32; for &b in d { c ^= b as u32; for _ in 0..8 { c = if c & 1 != 0 { (c >> 1) ^ 0x82F63B78 } else { c >> 1 } } } !c }
fn xof(key: &[u8; 32], n: usize) -> Vec<u8> { let mut o = vec![0u8; n]; blake3::Hasher::new_keyed(key).finalize_xof().fill(&mut o); o }
fn mask(key: &[u8; 32], x: &[u8], n: usize) -> Vec<u8> { let s = xof(key, n.min(x.len())); x.iter().enumerate().map(|(i, b)| if i < s.len() { b ^ s[i] } else { *b }).collect() }
fn x25519(k: &[u8], u: &[u8]) -> [u8; 32] { MontgomeryPoint(u.try_into().unwrap()).mul_clamped(k.try_into().unwrap()).0 }
fn x25519_base(k: &[u8]) -> [u8; 32] { MontgomeryPoint::mul_base_clamped(k.try_into().unwrap()).0 }
fn chacha(key: &[u8; 32], nonce: &[u8], data: &[u8]) -> Vec<u8> { let mut o = data.to_vec(); chacha20::ChaCha20::new(key.into(), nonce.into()).apply_keystream(&mut o); o }
fn aead(key: &[u8; 32], nonce: &[u8], plain: &[u8]) -> Vec<u8> { ChaCha20Poly1305::new(key.into()).encrypt(nonce.into(), plain).unwrap() }

const BLK: usize = 262144;
fn pattern(n: usize) -> Vec<u8> { (0..n).map(|i| ((i / 256) % 251) as u8).collect() }
fn block_cvs(f: &[u8]) -> Vec<[u8; 32]> { f.chunks(BLK).enumerate().map(|(b, d)| blake3::Hasher::new().set_input_offset((b * BLK) as u64).update(d).finalize_non_root()).collect() }
// levels below the root, as §12.4 defines them
fn levels(cvs: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
    let mut l = vec![cvs.to_vec()];
    while l.last().unwrap().len() > 2 { let p = l.last().unwrap(); l.push(p.chunks(2).map(|c| if c.len() == 2 { merge_subtrees_non_root(&c[0], &c[1], Mode::Hash) } else { c[0] }).collect()); }
    l
}
fn root_of(l: &[Vec<[u8; 32]>]) -> [u8; 32] { let t = l.last().unwrap(); *merge_subtrees_root(&t[0], &t[1], Mode::Hash).as_bytes() }
fn proof(l: &[Vec<[u8; 32]>], first: usize, count: usize) -> Vec<(usize, usize)> {
    let (mut a, mut b, mut p) = (first, first + count - 1, vec![]);
    for (k, lv) in l.iter().enumerate() { if a % 2 == 1 { p.push((k, a - 1)) } if b % 2 == 0 && b + 1 < lv.len() { p.push((k, b + 1)) } a /= 2; b /= 2; }
    p
}
// receiver: rebuild the root from cvs and proof alone, knowing n
fn rebuild(n: usize, first: usize, cvs: &[[u8; 32]], pr: &[[u8; 32]]) -> [u8; 32] {
    let (mut a, mut cnt, mut cur, mut pi) = (first, n, cvs.to_vec(), 0);
    loop {
        let b = a + cur.len() - 1;
        if a % 2 == 1 { cur.insert(0, pr[pi]); pi += 1; a -= 1 }
        if b % 2 == 0 && b + 1 < cnt { cur.push(pr[pi]); pi += 1 }
        if cnt == 2 { assert_eq!(pi, pr.len()); assert_eq!(cur.len(), 2); return *merge_subtrees_root(&cur[0], &cur[1], Mode::Hash).as_bytes() }
        cur = cur.chunks(2).map(|c| if c.len() == 2 { merge_subtrees_non_root(&c[0], &c[1], Mode::Hash) } else { c[0] }).collect();
        a /= 2; cnt = (cnt + 1) / 2;
    }
}
fn tree(size: usize, reqs: &[(usize, usize)]) {
    let f = pattern(size); let cvs = block_cvs(&f); let n = cvs.len();
    println!("\n-- tree: size {size}, n = {n}, last block {} B", size - (n - 1) * BLK);
    let root = *blake3::hash(&f).as_bytes(); println!("root {}", hx(&root));
    if n == 1 { return }
    let l = levels(&cvs); assert_eq!(root_of(&l), root);
    for (i, c) in cvs.iter().enumerate() { println!("block {i} cv {}", hx(c)); }
    for (k, lv) in l.iter().enumerate().skip(1) { for (j, c) in lv.iter().enumerate() { println!("node({k},{j}) {}", hx(c)); } }
    for &(first, count) in reqs {
        let p = proof(&l, first, count);
        let pv: Vec<[u8; 32]> = p.iter().map(|&(k, j)| l[k][j]).collect();
        assert_eq!(rebuild(n, first, &cvs[first..first + count], &pv), root);
        println!("HashRequest{{first={first}, count={count}}} proof = {:?}", p);
    }
}

fn main() {
    let dir = std::env::args().nth(2);
    let dump = |name: &str, d: &[u8]| if let Some(dir) = &dir { std::fs::write(format!("{dir}/{name}"), d).unwrap() };
    let pk1 = unhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let seed1 = unhex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
    let sk1 = SigningKey::from_bytes(seed1.as_slice().try_into().unwrap()); assert_eq!(sk1.verifying_key().to_bytes().to_vec(), pk1);
    let seed2 = unhex("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb");
    let pk2 = unhex("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");
    let sk2 = SigningKey::from_bytes(seed2.as_slice().try_into().unwrap()); assert_eq!(sk2.verifying_key().to_bytes().to_vec(), pk2);
    let sign = |sk: &SigningKey, m: &[u8]| { let s = sk.sign(m); sk.verifying_key().verify_strict(m, &s).unwrap(); s.to_bytes().to_vec() };
    let node_id = h("ed3k v1 node id", &pk1)[..16].to_vec(); assert_eq!(hx(&node_id), "cf617f4479e97358c9d31741900a90ae");
    let obf = &node_id[..12];
    let sample = b"ed3k test vector"; let root = blake3::hash(sample).as_bytes().to_vec();
    let secret: Vec<u8> = (0..32u8).map(|i| 0x37 ^ i).collect();
    let t0: u64 = 20739 * 86400; let exp: u64 = t0 + 86400;
    let salt = unhex("0102030405060708");

    println!("== routing id");
    let r = node_id[15] & 7; let ip = [21u8, 75, 31, 124]; let mut m = [ip[0] & 3, ip[1] & 0x0f, ip[2] & 0x3f, ip[3]]; m[0] |= r << 5;
    let c = crc32c(&m).to_be_bytes(); let mut rid = node_id.clone(); rid[0] = c[0]; rid[1] = c[1]; rid[2] = (c[2] & 0xf8) | (node_id[2] & 7);
    println!("r {r} masked {} crc {} routing {}", hx(&m), hx(&c), hx(&rid));
    assert_eq!(hx(&crc32c(&[0xc0, 0x0f, 0x0b, 0x15]).to_be_bytes()), "719c3fe1");

    println!("\n== datagrams");
    let key = |dir: u8, salt: &[u8]| h("ed3k v1 obfs", &cat(&[obf, &[dir], salt]));
    assert_eq!(hx(&key(0, &salt)), "015bc0e543423a9ede489325944e161ff19848369026217d2944ecd547e08588");
    let magic = [0x65u8, 0x33, 0x6b, 0x01];
    let dgram = |name: &str, dir: u8, salt: &[u8], kind: u8, pad: &[u8], body: &[u8], nmask: usize| {
        let plain = cat(&[&magic, &[(kind << 6) | pad.len() as u8], pad, body]);
        let k = key(dir, salt); let d = cat(&[salt, &mask(&k, &plain, nmask)]);
        println!("{name}: dir {dir:02x} salt {} key {}", hx(salt), hx(&k)); wrap("  plain", &plain); wrap("  datagram", &d);
    };
    let mut hs: Vec<u8> = (0..40u8).collect(); hs[0] = 0xc3;
    dgram("HANDSHAKE", 0, &salt, 1, &[0xaa, 0xbb, 0xcc], &hs, usize::MAX);
    dgram("FRAGMENT 0/2", 0, &unhex("1112131415161718"), 2, &[0xdd, 0xee], &cat(&[&[0x5c, 0x02], &hs[..15]]), usize::MAX);
    dgram("FRAGMENT 1/2", 0, &unhex("2122232425262728"), 2, &[], &cat(&[&[0x5c, 0x12], &hs[15..]]), usize::MAX);
    let mut data: Vec<u8> = (0..80u8).collect(); data[0] = 0x40;
    dgram("DATA", 1, &unhex("3132333435363738"), 0, &[0xf0, 0x0f], &data, 5 + 2 + 64);
    dgram("PROBE", 0, &unhex("4142434445464748"), 3, &[0x99], &unhex("a1a2a3a4a5a6a7a8"), usize::MAX);

    println!("\n== trees");
    tree(100_000, &[]);
    tree(BLK + 1, &[(0, 1), (1, 1)]);
    tree(5 * BLK + 1000, &[(4, 1), (2, 2), (5, 1)]);
    tree(7 * BLK, &[(6, 1), (5, 1), (0, 7)]);
    tree(5 * BLK, &[(1, 3)]);

    println!("\n== keywords");
    for w in std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap().lines() {
        println!("{w}\t{}\t{}", hx(w.as_bytes()), hx(&h("ed3k v1 keyword", &cat(&[&[0], w.as_bytes()]))[..16]));
    }

    println!("\n== sets");
    let member = cat(&[&fb(1, &root), &fv(2, 16), &fb(3, b"sample.txt")]);
    let body = cat(&[&fb(1, b"vectors"), &fb(3, &member), &fb(6, b"sid1"), &fv(7, 1)]);
    wrap("FileSet body", &body);
    let msig = sign(&sk1, &cat(&[b"ed3k v1 set manifest", &body])); println!("manifest sig {}", hx(&msig));
    let head8 = unhex("6564336b73657401");
    let sole = cat(&[&head8, &fb(1, &body), &fb(2, &pk1), &fm(3, &sigmsg(&msig))]);
    wrap("signed manifest file", &sole); println!("root {}", blake3::hash(&sole).to_hex());
    dump("sole.bin", &sole[8..]);
    let grant = cat(&[&fb(1, b"sid1"), &fv(2, 1), &fb(3, &pk1), &fb(3, &pk2), &fv(4, 2)]);
    wrap("Grant", &grant);
    let gsig = sign(&sk1, &cat(&[b"ed3k v1 set grant", &grant])); println!("grant sig {}", hx(&gsig));
    let sgrant = cat(&[&fb(1, &grant), &fb(2, &pk1), &fm(3, &sigmsg(&gsig))]);
    let cpre = cat(&[b"ed3k v1 set cosig", &body]);
    let (c0, c1) = (sign(&sk1, &cpre), sign(&sk2, &cpre)); println!("cosig 0 {}\ncosig 1 {}", hx(&c0), hx(&c1));
    let cos = |s: u64, sig: &[u8]| cat(&[&fv(1, s), &fm(2, &sigmsg(sig))]);
    let joint = cat(&[&head8, &fb(1, &body), &fb(2, &pk1), &fm(4, &sgrant), &fm(5, &cos(0, &c0)), &fm(5, &cos(1, &c1))]);
    wrap("joint manifest file", &joint); let jroot = blake3::hash(&joint); println!("root {}", jroot.to_hex());
    dump("joint.bin", &joint[8..]);
    let head = cat(&[&fb(1, b"sid1"), &fv(2, 1), &fb(3, jroot.as_bytes()), &fv(4, joint.len() as u64)]);
    wrap("Head", &head);
    let target = h("ed3k v1 set head", &cat(&[&pk1, b"sid1"]))[..16].to_vec(); assert_eq!(hx(&target), "db35cab4ac16050fa47f13fbcc4fe600");
    let jpre = cat(&[b"ed3k v1 joint head", &target, &1u64.to_be_bytes(), &head]);
    wrap("joint head preimage", &jpre);
    println!("head cosig 0 {}\nhead cosig 1 {}", hx(&sign(&sk1, &jpre)), hx(&sign(&sk2, &jpre)));

    println!("\n== torrent");
    let fa = sample.to_vec(); let fbn = b"hello".to_vec();
    let pieces = sha1::Sha1::digest(cat(&[&fa, &fbn]));
    let info = cat(&[b"d5:filesld6:lengthi16e4:pathl5:a.txteed6:lengthi5e4:pathl3:sub5:b.bineee4:name8:ed3k vec12:piece lengthi16384e6:pieces20:", &pieces, b"e"]);
    wrap("info", &info); println!("pieces {}", hx(&pieces));
    let btih = sha1::Sha1::digest(&info); println!("btih {}", hx(&btih));
    let iroot = blake3::hash(&info); println!("info root {} size {}", iroot.to_hex(), info.len());
    println!("root a {} root b {}", blake3::hash(&fa).to_hex(), blake3::hash(&fbn).to_hex());
    let m1 = cat(&[&fb(1, blake3::hash(&fa).as_bytes()), &fv(2, 16), &fb(3, b"a.txt")]);
    let m2 = cat(&[&fb(1, blake3::hash(&fbn).as_bytes()), &fv(2, 5), &fb(3, b"b.bin"), &fv(4, 1)]);
    let tor = cat(&[&fb(1, &btih), &fb(3, iroot.as_bytes()), &fv(4, info.len() as u64)]);
    let tbody = cat(&[&fb(1, b"ed3k vec"), &fb(2, b"sub"), &fb(3, &m1), &fb(3, &m2), &fm(9, &tor)]);
    let tfile = cat(&[&head8, &fb(1, &tbody)]);
    wrap("canonical manifest", &tfile); println!("root {}", blake3::hash(&tfile).to_hex());
    dump("torrent.bin", &tfile[8..]);

    println!("\n== hidden set");
    let k = h("ed3k v1 hidden set", &cat(&[&secret, &pk1])); let k_head = h("ed3k v1 hidden set head", &k); let k_data = h("ed3k v1 hidden set data", &k);
    println!("k {}\nk_head {}\nk_data {}", hx(&k), hx(&k_head), hx(&k_data));
    let base = h("ed3k v1 hidden head", &pk1)[..16].to_vec(); assert_eq!(hx(&base), "08299d3a4b291d676e0ecd0d39db534a");
    let hhead = cat(&[&fb(1, b"sid1"), &fv(2, 1), &fb(3, &root), &fv(4, 16)]);
    let nonce: Vec<u8> = (0..12u8).collect();
    let sealed = cat(&[&nonce, &chacha(&k_head, &nonce, &hhead)]);
    wrap("Head", &hhead); wrap("data", &sealed);
    let rbody = cat(&[&fv(1, exp), &fm(7, &cat(&[&fv(1, 1), &fb(2, &sealed)]))]);
    wrap("Record body", &rbody);
    let rpre = cat(&[b"ed3k v1 record", &[8], &base, &rbody]);
    println!("signed {} B, signature {}", rpre.len(), hx(&sign(&sk1, &rpre)));

    println!("\n== friend");
    let kf = h("ed3k v1 friend", &cat(&[&secret, &pk1])); let k_rec = h("ed3k v1 friend record", &kf);
    assert_eq!(hx(&k_rec), "2bcb004f2be3afb67b17062739acc35606912f4bc2eef1a2337dd3ea99ad16ce");
    let ep4 = [124u8, 31, 75, 21, 0x12, 0x40];
    let frec = cat(&[&fm(1, &fb(2, &ep4)), &fv(2, t0)]);
    let mut plain = cat(&[&(frec.len() as u16).to_be_bytes(), &frec]); plain.resize(356, 0);
    let fs = cat(&[&nonce, &aead(&k_rec, &nonce, &plain)]); assert_eq!(fs.len(), 384);
    wrap("FriendRecord", &frec); wrap("sealed record", &fs); println!("blake3 {}", blake3::hash(&fs).to_hex());
    let x = unhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    let e = x25519_base(&x); assert_eq!(hx(&e), "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
    let conv_pub = sk1.verifying_key().to_montgomery().0;
    let mut conv_priv = sha2::Sha512::digest(&seed1)[..32].to_vec(); conv_priv[0] &= 248; conv_priv[31] &= 127; conv_priv[31] |= 64;
    assert_eq!(x25519_base(&conv_priv), conv_pub);
    println!("converted public {}\nconverted private {}", hx(&conv_pub), hx(&conv_priv));
    let ss = x25519(&x, &conv_pub); assert_eq!(ss, x25519(&conv_priv, &e));
    let k_req = h("ed3k v1 friend request", &cat(&[&ss, &e, &pk1]));
    println!("ss {}\nk_req {}", hx(&ss), hx(&k_req));
    let text = b"hi from the vectors";
    let qpre = cat(&[b"ed3k v1 friend request signature", &pk1, &t0.to_be_bytes(), text]);
    let qsig = sign(&sk2, &qpre);
    let freq = cat(&[&fb(1, &pk2), &fb(2, text), &fv(3, t0), &fm(4, &sigmsg(&qsig))]);
    let mut plain = cat(&[&(freq.len() as u16).to_be_bytes(), &freq]); plain.resize(324, 0);
    let qs = cat(&[&e, &nonce, &aead(&k_req, &nonce, &plain)]); assert_eq!(qs.len(), 384);
    println!("request sig preimage {} B, sig {}", qpre.len(), hx(&qsig));
    wrap("FriendRequest", &freq); wrap("sealed request", &qs); println!("blake3 {}", blake3::hash(&qs).to_hex());
    dump("frec.bin", &frec); dump("freq.bin", &freq);

    println!("\n== lan");
    let net_id = h("ed3k v1 net", b"ed3k test")[..16].to_vec();
    let la = cat(&[&fb(1, &node_id), &fv(2, 4672)]);
    for (n, nid) in [("public", &[][..]), ("test", &net_id[..])] {
        let k = h("ed3k v1 lan", &cat(&[nid, &salt]));
        let pl = cat(&[&[0x65, 0x33, 0x6c, 0x01], &la]);
        println!("{n}: key {}", hx(&k)); wrap("  plain", &pl); wrap("  datagram", &cat(&[&salt, &mask(&k, &pl, usize::MAX)]));
    }

    println!("\n== hop");
    let y = unhex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    let f = x25519_base(&y); assert_eq!(hx(&f), "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f");
    let ss1 = x25519(&x, &conv_pub); assert_eq!(ss1, x25519(&conv_priv, &e));
    let ss2 = x25519(&x, &f); assert_eq!(ss2, x25519(&y, &e)); assert_eq!(hx(&ss2), "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");
    let kh = h("ed3k v1 hop", &cat(&[&ss1, &ss2, &e, &f, &pk1]));
    let (pr, ko, ki) = (h("ed3k v1 hop proof", &kh), h("ed3k v1 hop out", &kh), h("ed3k v1 hop in", &kh));
    println!("ss1 {}\nss2 {}\nk {}\nproof {}\nk_out {}\nk_in {}", hx(&ss1), hx(&ss2), hx(&kh), hx(&pr), hx(&ko), hx(&ki));
    for (key, sn) in [(&ko, 0u64), (&ki, 0), (&ko, 5)] { let n = cat(&[&[0u8; 4], &sn.to_be_bytes()]); println!("keystream stream {sn}: {}", hx(&chacha(key, &n, &[0u8; 16]))); }

    println!("\n== live");
    let stream = h("ed3k v1 live", &cat(&[&pk1, b"sid1"]))[..16].to_vec(); assert_eq!(hx(&stream), "d6c1deaa5cf816f5f5430d7dd6c0e59a");
    let seg = cat(&[&fv(1, t0), &fv(2, 7), &fb(3, &root), &fv(4, 16), &fv(5, 4000)]);
    let lpre = cat(&[b"ed3k v1 live segment", &stream, &seg]);
    wrap("LiveSegment", &seg); println!("signed {} B, signature {}", lpre.len(), hx(&sign(&sk1, &lpre)));
    dump("seg.bin", &seg);

    ext::run(&pk1, &seed1, &pk2, &seed2, &node_id, sample);
    paid::run(&pk1, &seed1, &node_id, &pk2, sample, &dir);
    reject::run(&pk1, &seed1, &root);
    pair::run(&pk1, &seed1, &pk2, &seed2);
    credit::run();
}
